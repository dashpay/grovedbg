//! GroveDB data visualizer and debugger, or GroveDBG.

#![deny(missing_docs)]

mod protocol;

use eframe::{egui, App, CreationContext};
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
    Box::new(GroveDbgApp {})
}

struct GroveDbgApp {}

impl App for GroveDbgApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("GroveDBG").show(ctx, |ui| {
            egui::widgets::global_dark_light_mode_buttons(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello World!");
        });
    }
}
