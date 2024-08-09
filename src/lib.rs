//! GroveDB data visualizer and debugger, or GroveDBG.

#![deny(missing_docs)]

mod bytes_utils;
mod path_ctx;
mod profiles;
mod proof_viewer;
mod protocol;
mod query_builder;
mod tree_view;

use std::time::Duration;

use base64::prelude::*;
use eframe::{
    egui::{self, Context, Style, Visuals},
    App, CreationContext, Storage,
};
use path_ctx::PathCtx;
use profiles::ProfilesView;
use proof_viewer::ProofViewer;
pub use protocol::start_grovedbg_protocol;
use protocol::{Command, GroveGdbUpdate};
use query_builder::QueryBuilder;
use tokio::sync::mpsc::{Receiver, Sender};
use tree_view::TreeView;

const PANEL_MARGIN: f32 = 5.;
const DARK_THEME_KEY: &'static str = "dark_theme";

type CommandsSender = Sender<Command>;
type UpdatesReceiver = Receiver<GroveGdbUpdate>;

/// Starts the GroveDBG application.
pub fn start_grovedbg_app(
    cc: &CreationContext,
    commands_sender: CommandsSender,
    updates_receiver: UpdatesReceiver,
) -> Box<dyn App> {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    cc.egui_ctx.set_fonts(fonts);

    let dark_theme = cc
        .storage
        .and_then(|s| s.get_string(DARK_THEME_KEY))
        .and_then(|param| param.parse::<bool>().ok())
        .unwrap_or_default();

    if dark_theme {
        let style = Style {
            visuals: Visuals::dark(),
            ..Style::default()
        };
        cc.egui_ctx.set_style(style);
    }

    let path_ctx = Box::leak(Box::new(PathCtx::new()));

    let _ = commands_sender
        .blocking_send(Command::FetchRoot)
        .inspect_err(|_| log::error!("Unable to reach GroveDBG protocol thread"));

    Box::new(GroveDbgApp::new(
        cc.storage,
        commands_sender,
        updates_receiver,
        path_ctx,
        dark_theme,
    ))
}

struct GroveDbgApp {
    commands_sender: CommandsSender,
    updates_receiver: UpdatesReceiver,
    path_ctx: &'static PathCtx,
    query_builder: QueryBuilder,
    proof_viewer: Option<ProofViewer>,
    tree_view: TreeView<'static>,
    show_query_builder: bool,
    show_proof_viewer: bool,
    show_profiles: bool,
    dark_theme: bool,
    profiles_view: ProfilesView,
}

const SHOW_QUERY_BUILDER_KEY: &'static str = "show_query_builder";
const SHOW_PROOF_VIEWER_KEY: &'static str = "show_proof_viewer";
const SHOW_PROFILES_KEY: &'static str = "show_profiles";
const PROFILES_KEY: &'static str = "profiles";

impl GroveDbgApp {
    fn new(
        storage: Option<&dyn Storage>,
        commands_sender: CommandsSender,
        updates_receiver: UpdatesReceiver,
        path_ctx: &'static PathCtx,
        dark_theme: bool,
    ) -> Self {
        GroveDbgApp {
            tree_view: TreeView::new(commands_sender.clone(), path_ctx),
            commands_sender,
            updates_receiver,
            path_ctx,
            query_builder: QueryBuilder::new(),
            proof_viewer: None,
            show_query_builder: storage
                .and_then(|s| s.get_string(SHOW_QUERY_BUILDER_KEY))
                .and_then(|param| param.parse::<bool>().ok())
                .unwrap_or(true),
            show_proof_viewer: storage
                .and_then(|s| s.get_string(SHOW_PROOF_VIEWER_KEY))
                .and_then(|param| param.parse::<bool>().ok())
                .unwrap_or(true),
            show_profiles: storage
                .and_then(|s| s.get_string(SHOW_PROFILES_KEY))
                .and_then(|param| param.parse::<bool>().ok())
                .unwrap_or(true),
            dark_theme,
            profiles_view: ProfilesView::restore(storage),
        }
    }

    fn draw_profiles_panel(&mut self, ctx: &Context) {
        egui::SidePanel::left("profiles")
            .default_width(10.)
            .show(ctx, |ui| {
                if self.show_profiles {
                    ui.horizontal(|line| {
                        if line
                            .button(egui_phosphor::variants::regular::ARROW_FAT_LINES_LEFT)
                            .clicked()
                        {
                            self.show_profiles = false;
                        }
                        line.label("Profiles");
                    });
                    ui.separator();
                    egui::Frame::default()
                        .outer_margin(PANEL_MARGIN)
                        .show(ui, |frame| {
                            self.profiles_view.draw(frame);
                        });
                } else {
                    if ui.button(egui_phosphor::variants::regular::BANK).clicked() {
                        self.show_profiles = true;
                    }
                }
            });
    }

    fn draw_query_builder_panel(&mut self, ctx: &Context) {
        egui::SidePanel::left("query_builder")
            .default_width(10.)
            .show(ctx, |ui| {
                if self.show_query_builder {
                    ui.horizontal(|line| {
                        if line
                            .button(egui_phosphor::variants::regular::ARROW_FAT_LINES_LEFT)
                            .clicked()
                        {
                            self.show_query_builder = false;
                        }
                        line.label("Query builder");
                    });
                    ui.separator();
                    egui::Frame::default()
                        .outer_margin(PANEL_MARGIN)
                        .show(ui, |frame| {
                            self.query_builder
                                .draw(frame, &self.path_ctx, &self.commands_sender);
                        });
                } else {
                    if ui
                        .button(egui_phosphor::variants::regular::LIST_MAGNIFYING_GLASS)
                        .clicked()
                    {
                        self.show_query_builder = true;
                    }
                }
            });
    }

    fn draw_proof_viewer_panel(&mut self, ctx: &Context) {
        egui::SidePanel::left("proof_viewer")
            .default_width(10.)
            .show(ctx, |ui| {
                if self.show_proof_viewer {
                    ui.horizontal(|line| {
                        if line
                            .button(egui_phosphor::variants::regular::ARROW_FAT_LINES_LEFT)
                            .clicked()
                        {
                            self.show_proof_viewer = false;
                        }
                        line.label("Proof viewer");
                    });
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
                } else {
                    if ui.button(egui_phosphor::variants::regular::LOCK_KEY).clicked() {
                        self.show_proof_viewer = true;
                    }
                }
            });
    }
}

impl App for GroveDbgApp {
    fn save(&mut self, storage: &mut dyn Storage) {
        storage.set_string(SHOW_QUERY_BUILDER_KEY, self.show_query_builder.to_string());
        storage.set_string(SHOW_PROOF_VIEWER_KEY, self.show_proof_viewer.to_string());
        storage.set_string(SHOW_PROFILES_KEY, self.show_profiles.to_string());
        storage.set_string(DARK_THEME_KEY, self.dark_theme.to_string());

        self.profiles_view.persist(storage);
    }

    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(5)
    }

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

        while !self.updates_receiver.is_empty() {
            if let Some(update) = self.updates_receiver.blocking_recv() {
                match update {
                    GroveGdbUpdate::Node(node_updates) => {
                        for update in node_updates.into_iter() {
                            self.tree_view.apply_node_update(update);
                        }
                    }
                    GroveGdbUpdate::Proof(proof) => {
                        self.proof_viewer = Some(ProofViewer::new(proof));
                        self.show_proof_viewer = true;
                    }
                    GroveGdbUpdate::RootUpdate(Some(root_update)) => {
                        self.tree_view.apply_root_node_update(root_update);
                    }
                    GroveGdbUpdate::RootUpdate(None) => {
                        log::warn!("Received no root node: GroveDB is empty");
                    }
                }
            } else {
                log::error!("Protocol thread was terminated, can't receive updates anymore");
            }
        }

        egui::SidePanel::right("log").show(ctx, |ui| {
            egui::Frame::default()
                .outer_margin(PANEL_MARGIN)
                .show(ui, |frame| {
                    egui_logger::logger_ui().show(frame);
                });
        });

        self.draw_profiles_panel(ctx);

        self.draw_query_builder_panel(ctx);

        self.draw_proof_viewer_panel(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.tree_view.draw(ui);
        });

        self.dark_theme = ctx.style().visuals.dark_mode;
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}
