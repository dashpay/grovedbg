use futures::TryFutureExt;
use grovedbg_types::{Key, NodeFetchRequest, NodeUpdate, Path, PathQuery, Proof, RootFetchRequest};
use reqwest::{Client, Url};
use tokio::sync::mpsc::{Receiver, Sender};

/// Starts the data exchange process between GroveDBG application and GroveDB's
/// debugger endpoint.
pub async fn start_grovedbg_protocol(
    address: Url,
    mut commands_receiver: Receiver<Command>,
    updates_sender: Sender<GroveGdbUpdate>,
) {
    let client = Client::new();

    while let Some(cmd) = commands_receiver.recv().await {
        let updates = match process_command(&address, &client, cmd).await {
            Ok(x) => x,
            Err(e) => {
                log::error!("Error processing command: {e}");
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
pub enum Command {
    FetchRoot,
    FetchNode { path: Path, key: Key },
    ProvePathQuery { path_query: PathQuery },
    FetchWithPathQuery { path_query: PathQuery },
}

/// Updates and commands' results pushed to GroveDBG application
pub enum GroveGdbUpdate {
    Node(Vec<NodeUpdate>),
    Proof(Proof),
}

impl From<Vec<NodeUpdate>> for GroveGdbUpdate {
    fn from(value: Vec<NodeUpdate>) -> Self {
        GroveGdbUpdate::Node(value)
    }
}

impl From<Proof> for GroveGdbUpdate {
    fn from(value: Proof) -> Self {
        GroveGdbUpdate::Proof(value)
    }
}

async fn process_command(
    address: &Url,
    client: &Client,
    command: Command,
) -> Result<GroveGdbUpdate, reqwest::Error> {
    match command {
        Command::FetchRoot => {
            log::info!("Fetch GroveDB root node");
            if let Some(root_node) = client
                .post(format!("{address}/fetch_root_node"))
                .json(&RootFetchRequest)
                .send()
                .and_then(|response| response.json::<Option<NodeUpdate>>())
                .await?
            {
                Ok(vec![root_node].into())
            } else {
                log::warn!("No root node returned, GroveDB is empty");
                Ok(Vec::new().into())
            }
        }
        Command::FetchNode { path, key } => {
            log::info!("Fetching a node...");
            if let Some(node_update) = client
                .post(format!("{address}/fetch_node"))
                .json(&NodeFetchRequest {
                    path: path.clone(),
                    key: key.clone(),
                })
                .send()
                .and_then(|response| response.json::<Option<NodeUpdate>>())
                .await?
            {
                Ok(vec![node_update].into())
            } else {
                log::warn!("No node returned");
                Ok(Vec::new().into())
            }
        }
        Command::ProvePathQuery { path_query } => Ok(client
            .post(format!("{address}/prove_path_query"))
            .json(&path_query)
            .send()
            .and_then(|response| response.json::<grovedbg_types::Proof>())
            .await?
            .into()),
        Command::FetchWithPathQuery { path_query } => Ok(client
            .post(format!("{address}/fetch_with_path_query"))
            .json(&path_query)
            .send()
            .and_then(|response| response.json::<Vec<grovedbg_types::NodeUpdate>>())
            .await?
            .into()),
    }
}
