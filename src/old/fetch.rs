mod proto_conversion;

use std::sync::Mutex;

use futures::TryFutureExt;
use grovedbg_types::{NodeFetchRequest, NodeUpdate, PathQuery, RootFetchRequest};
use reqwest::Client;
use tokio::sync::mpsc::Receiver;

use self::proto_conversion::{from_update, BadProtoElement};
use crate::{
    model::{path_display::PathCtx, Key, Tree},
    ui::ProofViewer,
};

type Path = Vec<Vec<u8>>;

pub(crate) enum Message {
    FetchRoot,
    FetchNode { path: Path, key: Key, show: bool },
    UnloadSubtree { path: Path },
    ProvePathQuery { path_query: PathQuery },
    FetchWithPathQuery { path_query: PathQuery },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FetchError {
    #[error(transparent)]
    DataError(#[from] BadProtoElement),
}

#[cfg(target_arch = "wasm32")]
fn base_url() -> String {
    web_sys::window().unwrap().location().origin().unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
fn base_url() -> String {
    unimplemented!()
}

async fn process_message<'c>(
    tree: &Mutex<Tree<'c>>,
    proof_viewer: &Mutex<Option<ProofViewer>>,
    path_ctx: &'c PathCtx,
    client: &Client,
    message: Message,
) -> Result<(), ProcessError> {
    match message {
        Message::FetchRoot => {
            log::info!("Fetch GroveDB root node");
            let Some(root_node) = client
                .post(format!("{}/fetch_root_node", base_url()))
                .json(&RootFetchRequest)
                .send()
                .and_then(|response| response.json::<Option<NodeUpdate>>())
                .await?
            else {
                log::warn!("No root node returned, GroveDB is empty");
                return Ok(());
            };

            let mut lock = tree.lock().unwrap();
            lock.set_root(root_node.key.clone());
            lock.insert(
                path_ctx.get_root(),
                root_node.key.clone(),
                from_update(path_ctx, root_node)?,
            );
        }
        Message::FetchNode { path, key, show } => {
            log::info!("Fetching a node...");
            let Some(node_update) = client
                .post(format!("{}/fetch_node", base_url()))
                .json(&NodeFetchRequest {
                    path: path.clone(),
                    key: key.clone(),
                })
                .send()
                .and_then(|response| response.json::<Option<NodeUpdate>>())
                .await?
            else {
                log::warn!("No node returned");
                return Ok(());
            };
            let mut lock = tree.lock().unwrap();
            let parent_subtree_path = path_ctx.add_path(path);
            lock.insert(
                parent_subtree_path,
                key.clone(),
                from_update(path_ctx, node_update)?,
            );
            if show {
                lock.get_subtree(&parent_subtree_path.child(key))
                    .into_iter()
                    .for_each(|ctx| {
                        ctx.subtree().set_visible(true);
                    });
            }
        }
        Message::UnloadSubtree { path } => {
            let mut lock = tree.lock().unwrap();
            lock.clear_subtree(path_ctx.add_path(path));
        }
        Message::ProvePathQuery { path_query } => {
            if let Ok(proof) = client
                .post(format!("{}/prove_path_query", base_url()))
                .json(&path_query)
                .send()
                .and_then(|response| response.json::<grovedbg_types::Proof>())
                .await
                .inspect_err(|e| log::error!("Error executing a path query with proof: {}", e))
            {
                let mut lock = proof_viewer.lock().unwrap();
                *lock = Some(ProofViewer::new(proof));
            }
        }
        Message::FetchWithPathQuery { path_query } => {
            if let Ok(updates) = client
                .post(format!("{}/fetch_with_path_query", base_url()))
                .json(&path_query)
                .send()
                .and_then(|response| response.json::<Vec<grovedbg_types::NodeUpdate>>())
                .await
                .inspect_err(|e| log::error!("Error fetching with a path query: {}", e))
            {
                let mut lock = tree.lock().unwrap();
                for node in updates.into_iter() {
                    lock.insert(
                        path_ctx.add_path(node.path.clone()),
                        node.key.clone(),
                        from_update(&path_ctx, node)?,
                    )
                }
            }
        }
    }
    Ok(())
}

struct ProcessError(String);

impl<E: std::error::Error> From<E> for ProcessError {
    fn from(value: E) -> Self {
        ProcessError(value.to_string())
    }
}

pub(crate) async fn process_messages<'c>(
    mut receiver: Receiver<Message>,
    tree: &Mutex<Tree<'c>>,
    proof_viewer: &Mutex<Option<ProofViewer>>,
    path_ctx: &'c PathCtx,
) {
    let client = Client::new();

    while let Some(message) = receiver.recv().await {
        if let Err(e) = process_message(tree, proof_viewer, path_ctx, &client, message).await {
            log::error!("{}", e.0);
        }
    }
}
