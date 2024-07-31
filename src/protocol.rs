use std::net::Ipv4Addr;

use futures::channel::mpsc::{Receiver, Sender};
use grovedbg_types::{Key, NodeUpdate, Path, PathQuery};
use reqwest::Url;

/// Starts the data exchange process between GroveDBG application and GroveDB's
/// debugger endpoint.
pub async fn start_grovedbg_protocol(
    address: Url,
    commands_receiver: Receiver<Command>,
    updates_sender: Sender<NodeUpdate>,
) {
}

pub enum Command {
    FetchRoot,
    FetchNode { path: Path, key: Key, show: bool },
    UnloadSubtree { path: Path },
    ProvePathQuery { path_query: PathQuery },
    FetchWithPathQuery { path_query: PathQuery },
}
