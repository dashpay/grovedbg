mod proto_conversion;

use std::{collections::VecDeque, sync::Mutex};

use futures::TryFutureExt;
use grovedbg_types::{NodeFetchRequest, NodeUpdate, RootFetchRequest};
use reqwest::Client;
use tokio::sync::mpsc::Receiver;

use self::proto_conversion::{from_update, BadProtoElement};
use crate::{
    model::{path_display::PathCtx, Key, Node, Tree},
    ui::common::bytes_as_hex,
};

type Path = Vec<Vec<u8>>;

pub(crate) enum Message {
    FetchRoot,
    FetchNode { path: Path, key: Key },
    FetchBranch { path: Path, key: Key, limit: FetchLimit },
    UnloadSubtree { path: Path },
}

pub(crate) enum FetchLimit {
    Unbounded,
    Count(usize),
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
        Message::FetchNode { path, key } => {
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
            lock.insert(path_ctx.add_path(path), key, from_update(path_ctx, node_update)?);
        }
        Message::FetchBranch { path, key, limit } => {
            log::info!("Fetching subtree branch...");
            let mut queue = VecDeque::new();
            queue.push_back(key.clone());

            let mut to_insert = Vec::new();

            while let Some(node_key) = queue.pop_front() {
                if let FetchLimit::Count(max_n) = limit {
                    if to_insert.len() >= max_n {
                        break;
                    }
                }
                let Ok(Some(node_update)) = client
                    .post(format!("{}/fetch_node", base_url()))
                    .json(&NodeFetchRequest {
                        path: path.clone(),
                        key: node_key.clone(),
                    })
                    .send()
                    .and_then(|response| response.json::<Option<NodeUpdate>>())
                    .await
                    .map_err(|e| log::error!("Branch fetching error: {}; attempting to load others...", e))
                else {
                    continue;
                };

                let node: Node = from_update(path_ctx, node_update)?;

                if let Some(left) = &node.left_child {
                    queue.push_back(left.clone());
                }

                if let Some(right) = &node.right_child {
                    queue.push_back(right.clone());
                }

                to_insert.push((node_key, node));
            }

            let mut lock = tree.lock().unwrap();
            to_insert
                .into_iter()
                .for_each(|(key, node)| lock.insert(path_ctx.add_path(path.clone()), key, node));
        }
        Message::UnloadSubtree { path } => {
            let mut lock = tree.lock().unwrap();
            lock.clear_subtree(path_ctx.add_path(path));
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
    path_ctx: &'c PathCtx,
) {
    let client = Client::new();

    while let Some(message) = receiver.recv().await {
        if let Err(e) = process_message(tree, path_ctx, &client, message).await {
            log::error!("{}", e.0);
        }
    }
}
