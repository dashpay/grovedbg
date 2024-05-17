mod proto_conversion;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use grovedbg_types::{NodeFetchRequest, NodeUpdate, RootFetchRequest};
use reqwest::Client;
use tokio::sync::mpsc::Receiver;

use self::proto_conversion::BadProtoElement;
use crate::model::{Key, Node, Path, Tree};

pub(crate) enum Message {
    FetchRoot,
    FetchNode { path: Path, key: Key },
    FetchBranch { path: Path, key: Key },
    UnloadSubtree { path: Path },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FetchError {
    #[error(transparent)]
    DataError(#[from] BadProtoElement),
    // #[error("tonic fetch error: {0}")]
    // TransportError(#[from] grovedbg_grpc::tonic::Status),
}

fn base_url() -> String {
    web_sys::window().unwrap().location().origin().unwrap()
}

pub(crate) async fn process_messages(mut receiver: Receiver<Message>, tree: Arc<Mutex<Tree>>) {
    let client = Client::new();

    while let Some(message) = receiver.recv().await {
        match message {
            Message::FetchRoot => {
                let Some(root_node) = client
                    .post(format!("{}/fetch_root_node", base_url()))
                    .json(&RootFetchRequest)
                    .send()
                    .await
                    .unwrap()
                    .json::<Option<NodeUpdate>>()
                    .await
                    .unwrap()
                else {
                    return;
                };

                let mut lock = tree.lock().unwrap();
                lock.set_root(root_node.key.clone());
                lock.insert(
                    vec![].into(),
                    root_node.key.clone(),
                    root_node.try_into().unwrap(),
                );
            }
            Message::FetchNode { path, key } => {
                let Some(node_update) = client
                    .post(format!("{}/fetch_node", base_url()))
                    .json(&NodeFetchRequest {
                        path: path.0.clone(),
                        key: key.clone(),
                    })
                    .send()
                    .await
                    .unwrap()
                    .json::<Option<NodeUpdate>>()
                    .await
                    .unwrap()
                else {
                    return;
                };
                let mut lock = tree.lock().unwrap();
                lock.insert(path, key, node_update.try_into().unwrap());
            }
            Message::FetchBranch { path, key } => {
                let mut queue = VecDeque::new();
                queue.push_back(key.clone());

                let mut to_insert = Vec::new();

                while let Some(node_key) = queue.pop_front() {
                    let Some(node_update) = client
                        .post(format!("{}/fetch_node", base_url()))
                        .json(&NodeFetchRequest {
                            path: path.0.clone(),
                            key: node_key.clone(),
                        })
                        .send()
                        .await
                        .unwrap()
                        .json::<Option<NodeUpdate>>()
                        .await
                        .unwrap()
                    else {
                        continue;
                    };

                    let node: Node = node_update.try_into().unwrap();

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
                    .for_each(|(key, node)| lock.insert(path.clone(), key, node));
            }
            Message::UnloadSubtree { path } => {
                let mut lock = tree.lock().unwrap();
                lock.clear_subtree(&path);
            }
        }
    }
}
