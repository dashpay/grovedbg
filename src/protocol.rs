mod proof_tree;

use std::collections::BTreeMap;

use grovedbg_types::{
    DropSessionRequest, Key, MerkProofNode, NewSessionResponse, NodeFetchRequest, NodeUpdate, Path,
    PathQuery, Proof, RootFetchRequest, SessionId, WithSession,
};
use proof_tree::ProofTree;
use reqwest::{Client, StatusCode, Url};
use tokio::sync::mpsc::{self, Receiver, Sender};

/// Starts the data exchange process between GroveDBG application and GroveDB's
/// debugger endpoint.
pub async fn start_grovedbg_protocol(
    address: Url,
    mut commands_receiver: Receiver<ProtocolCommand>,
    updates_sender: Sender<GroveGdbUpdate>,
) {
    let client = Client::new();

    log::info!(
        "Starting background fetch process, GroveDBG backend address is {}",
        address
    );

    let (feedback_send, mut feedback_recv) = mpsc::channel(10);

    while let Some(cmd) = tokio::select! {
        x = commands_receiver.recv() => x,
        x = feedback_recv.recv() => x,
    } {
        let updates = match process_command(&address, &client, cmd).await {
            Ok(x) => x,
            Err(e) => {
                match e.downcast_ref::<reqwest::Error>() {
                    Some(req_error) if req_error.status() == Some(StatusCode::UNAUTHORIZED) => {
                        log::warn!("Session expired");
                        feedback_send
                            .send(ProtocolCommand::NewSession { old_session: None })
                            .await
                            .ok();
                    }
                    _ => log::error!("Error processing command: {e}"),
                }
                continue;
            }
        };

        if let Err(send_error) = updates_sender.send(updates).await {
            log::error!("Unable to send update: {send_error}; terminating the protocol task");
            return;
        }
    }
}

/// Background tasks of GroveDBG application
pub enum FetchCommand {
    FetchRoot,
    FetchNode { path: Path, key: Key },
    ProvePathQuery { path_query: PathQuery },
    FetchWithPathQuery { path_query: PathQuery },
}

pub enum ProtocolCommand {
    NewSession {
        old_session: Option<SessionId>,
    },
    Fetch {
        session_id: SessionId,
        command: FetchCommand,
    },
}

/// Updates and commands' results pushed to GroveDBG application
#[derive(Debug)]
pub enum GroveGdbUpdate {
    RootUpdate(Option<NodeUpdate>),
    Node(Vec<NodeUpdate>),
    Proof(
        Proof,
        Vec<NodeUpdate>,
        BTreeMap<Vec<Vec<u8>>, BTreeMap<Key, MerkProofNode>>,
    ),
    Session(SessionId),
}

impl From<Vec<NodeUpdate>> for GroveGdbUpdate {
    fn from(value: Vec<NodeUpdate>) -> Self {
        GroveGdbUpdate::Node(value)
    }
}

async fn fetch_node(
    client: &Client,
    address: &Url,
    session_id: SessionId,
    path: Vec<Vec<u8>>,
    key: Vec<u8>,
) -> Result<Option<NodeUpdate>, reqwest::Error> {
    client
        .post(format!("{address}fetch_node"))
        .json(&WithSession {
            session_id,
            request: NodeFetchRequest { path, key },
        })
        .send()
        .await?
        .error_for_status()?
        .json::<Option<NodeUpdate>>()
        .await
}

async fn fetch_root_node(
    client: &Client,
    address: &Url,
    session_id: SessionId,
) -> Result<Option<NodeUpdate>, reqwest::Error> {
    client
        .post(format!("{address}fetch_root_node"))
        .json(&WithSession {
            session_id,
            request: RootFetchRequest,
        })
        .send()
        .await?
        .error_for_status()?
        .json::<Option<NodeUpdate>>()
        .await
}

async fn process_command(
    address: &Url,
    client: &Client,
    command: ProtocolCommand,
) -> anyhow::Result<GroveGdbUpdate> {
    match command {
        ProtocolCommand::Fetch {
            command: FetchCommand::FetchRoot,
            session_id: session,
        } => {
            log::info!("Fetch GroveDB root node");
            if let Some(root_node) = fetch_root_node(client, address, session).await? {
                Ok(GroveGdbUpdate::RootUpdate(Some(root_node)))
            } else {
                log::warn!("No root node returned, GroveDB is empty");
                Ok(GroveGdbUpdate::RootUpdate(None))
            }
        }
        ProtocolCommand::Fetch {
            command: FetchCommand::FetchNode { path, key },
            session_id: session,
        } => {
            log::info!("Fetching a node...");
            if let Some(node_update) = fetch_node(client, address, session, path, key).await? {
                Ok(vec![node_update].into())
            } else {
                log::warn!("No node returned");
                Ok(Vec::new().into())
            }
        }
        ProtocolCommand::Fetch {
            command: FetchCommand::ProvePathQuery { path_query },
            session_id,
        } => {
            log::info!("Requesting a proof for a path query...");
            let proof = client
                .post(format!("{address}prove_path_query"))
                .json(&WithSession {
                    session_id,
                    request: path_query,
                })
                .send()
                .await?
                .error_for_status()?
                .json::<grovedbg_types::Proof>()
                .await?;

            let mut proof_tree = ProofTree::new(client, address, proof.clone(), session_id).await?;
            proof_tree.fetch_additional_data().await?;

            let updates = proof_tree
                .tree
                .clone()
                .into_values()
                .flat_map(|vals| vals.tree.into_iter())
                .flat_map(|node| node.node_update)
                .collect();

            let tree_proof_data: BTreeMap<_, _> = proof_tree
                .tree
                .into_iter()
                .map(|(k, v)| (k, v.to_proof_tree_data()))
                .collect();

            Ok(GroveGdbUpdate::Proof(proof, updates, tree_proof_data))
        }
        ProtocolCommand::Fetch {
            command: FetchCommand::FetchWithPathQuery { path_query },
            session_id,
        } => {
            log::info!(
                "Fetching {} nodes of a subtree with a path query...",
                path_query
                    .query
                    .limit
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "all".to_owned())
            );
            Ok(client
                .post(format!("{address}fetch_with_path_query"))
                .json(&WithSession {
                    session_id,
                    request: path_query,
                })
                .send()
                .await?
                .error_for_status()?
                .json::<Vec<grovedbg_types::NodeUpdate>>()
                .await?
                .into())
        }
        ProtocolCommand::NewSession { old_session } => {
            if let Some(old) = old_session {
                log::info!("Terminating old session: {}", old);
                client
                    .post(format!("{address}drop_session"))
                    .json(&DropSessionRequest { session_id: old })
                    .send()
                    .await?
                    .error_for_status()?;
            }
            log::info!("Starting new session");
            let NewSessionResponse { session_id } = client
                .post(format!("{address}new_session"))
                .send()
                .await?
                .error_for_status()?
                .json::<NewSessionResponse>()
                .await?;
            Ok(GroveGdbUpdate::Session(session_id))
        }
    }
}
