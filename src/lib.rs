//! GroveDB data visualizer and debugger, or GroveDBG.

#![deny(missing_docs)]

mod bytes_utils;
mod path_ctx;
mod proof_viewer;
mod protocol;
mod query_builder;
mod tree_view;

use std::time::Duration;

use eframe::{egui, App, CreationContext};
use path_ctx::PathCtx;
use proof_viewer::ProofViewer;
pub use protocol::start_grovedbg_protocol;
use protocol::{Command, GroveGdbUpdate};
use query_builder::QueryBuilder;
use tokio::sync::mpsc::{Receiver, Sender};
use tree_view::TreeView;

const PANEL_MARGIN: f32 = 5.;

type CommandsSender = Sender<Command>;
type UpdatesReceiver = Receiver<GroveGdbUpdate>;

/// Starts the GroveDBG application.
pub fn start_grovedbg_app(
    _cc: &CreationContext,
    commands_sender: CommandsSender,
    updates_receiver: UpdatesReceiver,
) -> Box<dyn App> {
    let path_ctx = Box::leak(Box::new(PathCtx::new()));

    Box::new(GroveDbgApp {
        tree_view: TreeView::new(commands_sender.clone(), path_ctx),
        commands_sender,
        updates_receiver,
        path_ctx,
        query_builder: QueryBuilder::new(),
        proof_viewer: None,
    })
}

struct GroveDbgApp {
    commands_sender: CommandsSender,
    updates_receiver: UpdatesReceiver,
    path_ctx: &'static PathCtx,
    query_builder: QueryBuilder,
    proof_viewer: Option<ProofViewer>,
    tree_view: TreeView<'static>,
}

impl App for GroveDbgApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("GroveDBG").show(ctx, |ui| {
            ui.horizontal(|line| {
                egui::widgets::global_dark_light_mode_buttons(line);
                if !self.updates_receiver.is_empty() {
                    line.label("Processing updates...");
                    line.spinner();
                }
            });
            ui.add_space(PANEL_MARGIN);
        });

        // TODO: process updates

        egui::SidePanel::right("log").show(ctx, |ui| {
            egui::Frame::default()
                .outer_margin(PANEL_MARGIN)
                .show(ui, |frame| {
                    egui_logger::logger_ui().show(frame);
                });
        });

        egui::SidePanel::left("query_builder").show(ctx, |ui| {
            ui.label("Query builder");
            ui.separator();
            egui::Frame::default()
                .outer_margin(PANEL_MARGIN)
                .show(ui, |frame| {
                    self.query_builder
                        .draw(frame, &self.path_ctx, &self.commands_sender);
                });
        });

        egui::SidePanel::left("proof_viewer").show(ctx, |ui| {
            ui.label("Proof viewer");
            ui.separator();
            egui::Frame::default()
                .outer_margin(PANEL_MARGIN)
                .show(ui, |frame| {
                    if let Some(proof_viewer) = &mut self.proof_viewer {
                        proof_viewer.draw(frame);
                    } else {
                        frame.label("No proof to show yet");
                    }
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.tree_view.draw(ui);
        });

        ctx.request_repaint_after(Duration::from_secs(5));
    }
}
