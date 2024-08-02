//! GroveDB data visualizer and debugger, or GroveDBG.

#![deny(missing_docs)]

mod bytes_utils;
mod path_ctx;
mod proof_viewer;
mod protocol;
mod query_builder;

use eframe::{egui, App, CreationContext};
use path_ctx::PathCtx;
pub use protocol::start_grovedbg_protocol;
use protocol::{Command, GroveGdbUpdate};
use tokio::sync::mpsc::{Receiver, Sender};

type CommandsSender = Sender<Command>;
type UpdatesReceiver = Receiver<GroveGdbUpdate>;

/// Starts the GroveDBG application.
pub fn start_grovedbg_app(
    _cc: &CreationContext,
    commands_sender: CommandsSender,
    updates_receiver: UpdatesReceiver,
) -> Box<dyn App> {
    Box::new(GroveDbgApp {
        commands_sender,
        updates_receiver,
        path_ctx: Box::leak(Box::new(PathCtx::new())),
    })
}

struct GroveDbgApp {
    commands_sender: CommandsSender,
    updates_receiver: UpdatesReceiver,
    path_ctx: &'static mut PathCtx,
}

impl App for GroveDbgApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("GroveDBG").show(ctx, |ui| {
            egui::widgets::global_dark_light_mode_buttons(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello World!");
        });
    }
}
