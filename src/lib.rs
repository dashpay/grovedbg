//! GroveDB data visualizer and debugger, or GroveDBG.

#![deny(missing_docs)]

mod protocol;

use eframe::{App, CreationContext};
use futures::channel::mpsc::{Receiver, Sender};
use grovedbg_types::NodeUpdate;
pub use protocol::start_grovedbg_protocol;
use protocol::Command;

/// Starts the GroveDBG application.
pub fn start_grovedbg_app(
    cc: &CreationContext,
    commands_sender: Sender<Command>,
    updates_receiver: Receiver<NodeUpdate>,
) -> Box<dyn App> {
    todo!()
}
