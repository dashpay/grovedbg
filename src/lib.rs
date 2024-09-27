//! GroveDB data visualizer and debugger, or GroveDBG.

#![deny(missing_docs)]

mod bus;
mod bytes_utils;
mod help;
mod merk_view;
mod path_ctx;
mod profiles;
mod proof_viewer;
mod protocol;
mod query_builder;
mod theme;
mod tree_data;
mod tree_view;

use std::time::Duration;

use bus::CommandBus;
use eframe::{
    egui::{self, Context, Style, Visuals},
    App, CreationContext, Storage,
};
use grovedbg_types::Key;
use merk_view::MerkView;
use path_ctx::{Path, PathCtx};
use profiles::ProfilesView;
use proof_viewer::ProofViewer;
pub use protocol::start_grovedbg_protocol;
use protocol::{FetchCommand, GroveGdbUpdate, ProtocolCommand};
use query_builder::QueryBuilder;
use tokio::sync::mpsc::{Receiver, Sender};
use tree_data::TreeData;
use tree_view::TreeView;

const PANEL_MARGIN: f32 = 5.;
const DARK_THEME_KEY: &'static str = "dark_theme";

type ProtocolSender = Sender<ProtocolCommand>;
type UpdatesReceiver = Receiver<GroveGdbUpdate>;

/// Starts the GroveDBG application.
pub fn start_grovedbg_app(
    cc: &CreationContext,
    protocol_sender: ProtocolSender,
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
    } else {
        let style = Style {
            visuals: Visuals::light(),
            ..Style::default()
        };
        cc.egui_ctx.set_style(style);
    }

    let path_ctx = Box::leak(Box::new(PathCtx::new()));

    let bus = CommandBus::new(protocol_sender);

    bus.new_session();

    Box::new(GroveDbgApp::new(
        cc.storage,
        bus,
        updates_receiver,
        path_ctx,
        dark_theme,
    ))
}

struct GroveDbgApp {
    bus: CommandBus<'static>,
    updates_receiver: UpdatesReceiver,
    path_ctx: &'static PathCtx,
    query_builder: QueryBuilder,
    proof_viewer: Option<ProofViewer>,
    tree_view: TreeView<'static>,
    merk_view: MerkView,
    tree_data: TreeData<'static>,
    show_query_builder: bool,
    show_proof_viewer: bool,
    show_profiles: bool,
    dark_theme: bool,
    profiles_view: ProfilesView,
    show_help: bool,
    show_log: bool,
    show_merk_view: bool,
    merk_panel_width: f32,
    focused_subtree: Option<FocusedSubree<'static>>,
    blocked: bool,
}

const SHOW_QUERY_BUILDER_KEY: &'static str = "show_query_builder";
const SHOW_PROOF_VIEWER_KEY: &'static str = "show_proof_viewer";
const SHOW_PROFILES_KEY: &'static str = "show_profiles";
const SHOW_LOG_KEY: &'static str = "show_log";
const SHOW_MERK_VIEW_KEY: &'static str = "show_merk_view";
const PROFILES_KEY: &'static str = "profiles";

impl GroveDbgApp {
    fn new(
        storage: Option<&dyn Storage>,
        bus: CommandBus<'static>,
        updates_receiver: UpdatesReceiver,
        path_ctx: &'static PathCtx,
        dark_theme: bool,
    ) -> Self {
        GroveDbgApp {
            tree_view: TreeView::new(path_ctx),
            merk_view: MerkView::new(),
            bus,
            updates_receiver,
            path_ctx,
            query_builder: QueryBuilder::new(),
            proof_viewer: None,
            tree_data: TreeData::new(path_ctx),
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
            show_help: false,
            show_log: storage
                .and_then(|s| s.get_string(SHOW_LOG_KEY))
                .and_then(|param| param.parse::<bool>().ok())
                .unwrap_or(true),
            show_merk_view: storage
                .and_then(|s| s.get_string(SHOW_MERK_VIEW_KEY))
                .and_then(|param| param.parse::<bool>().ok())
                .unwrap_or(true),
            merk_panel_width: 0.,
            focused_subtree: None,
            blocked: false,
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
                            .on_hover_text("Hide profiles panel")
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
                            self.profiles_view.draw(frame, &self.bus, self.path_ctx);
                        });
                } else {
                    if ui
                        .button(egui_phosphor::variants::regular::BANK)
                        .on_hover_text("Show profiles panel")
                        .clicked()
                    {
                        self.show_profiles = true;
                    }
                }
            });
    }

    fn draw_query_builder_panel<'pf>(&mut self, ctx: &Context) {
        egui::SidePanel::left("query_builder")
            .default_width(10.)
            .show(ctx, |ui| {
                if self.show_query_builder {
                    ui.horizontal(|line| {
                        if line
                            .button(egui_phosphor::variants::regular::ARROW_FAT_LINES_LEFT)
                            .on_hover_text("Hide query builder panel")
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
                            self.query_builder.draw(
                                frame,
                                &self.path_ctx,
                                self.profiles_view.active_profile_root_ctx(),
                                &self.bus,
                            );
                        });
                } else {
                    if ui
                        .button(egui_phosphor::variants::regular::LIST_MAGNIFYING_GLASS)
                        .on_hover_text("Show query builder panel")
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
                            .on_hover_text("Hide proof viewer panel")
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
                                proof_viewer.draw(frame, &self.bus, &self.path_ctx);
                            } else {
                                frame.label("No proof to show yet");
                            }
                        });
                } else {
                    if ui
                        .button(egui_phosphor::variants::regular::LOCK_KEY)
                        .on_hover_text("Show proof viewer panel")
                        .clicked()
                    {
                        self.show_proof_viewer = true;
                    }
                }
            });
    }

    fn draw_log_panel(&mut self, ctx: &Context) {
        egui::SidePanel::right("log").default_width(10.).show(ctx, |ui| {
            if self.show_log {
                ui.horizontal(|line| {
                    line.label("Log");
                    if line
                        .button(egui_phosphor::variants::regular::ARROW_FAT_LINES_RIGHT)
                        .on_hover_text("Hide log panel")
                        .clicked()
                    {
                        self.show_log = false;
                    }
                });
                ui.separator();

                egui::Frame::default()
                    .outer_margin(PANEL_MARGIN)
                    .show(ui, |frame| {
                        egui_logger::logger_ui().show(frame);
                    });
            } else {
                if ui
                    .button(egui_phosphor::variants::regular::INFO)
                    .on_hover_text("Show log panel")
                    .clicked()
                {
                    self.show_log = true;
                }
            }
        });
    }

    fn draw_merk_view_panel(&mut self, ctx: &Context) {
        let width = egui::SidePanel::left("merk_view")
            .default_width(10.)
            .show(ctx, |ui| {
                if self.show_merk_view {
                    ui.horizontal(|line| {
                        if line
                            .button(egui_phosphor::variants::regular::ARROW_FAT_LINES_LEFT)
                            .on_hover_text("Hide merk view panel")
                            .clicked()
                        {
                            self.show_merk_view = false;
                        }
                        line.label("Merk view");
                    });
                    ui.separator();
                    egui::Frame::default()
                        .outer_margin(PANEL_MARGIN)
                        .show(ui, |frame| {
                            let (path, subtree_data, subtree_proof_data) = self.tree_data.get_merk_selected();
                            self.merk_view.draw(
                                frame,
                                &self.bus,
                                path,
                                subtree_data,
                                subtree_proof_data,
                                self.profiles_view.active_profile_root_ctx().fast_forward(path),
                            );
                        });
                } else {
                    if ui
                        .button(egui_phosphor::variants::regular::TREE_STRUCTURE)
                        .on_hover_text("Show merk view panel")
                        .clicked()
                    {
                        self.show_merk_view = true;
                        ui.set_width(ctx.available_rect().width() / 2.);
                    }
                }
                ui.max_rect().width()
            })
            .inner;

        self.merk_panel_width = width;
    }
}

impl App for GroveDbgApp {
    fn save(&mut self, storage: &mut dyn Storage) {
        storage.set_string(SHOW_QUERY_BUILDER_KEY, self.show_query_builder.to_string());
        storage.set_string(SHOW_PROOF_VIEWER_KEY, self.show_proof_viewer.to_string());
        storage.set_string(SHOW_PROFILES_KEY, self.show_profiles.to_string());
        storage.set_string(SHOW_LOG_KEY, self.show_log.to_string());
        storage.set_string(SHOW_MERK_VIEW_KEY, self.show_merk_view.to_string());
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
                // if line.button("Help").clicked() {
                //     self.show_help = !self.show_help;
                // }

                if line
                    .button("New session")
                    .on_hover_text(
                        "Reset existing session and request a new one to access the latest GroveDB data",
                    )
                    .clicked()
                {
                    self.bus.new_session();
                }

                if self.blocked {
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
                            self.tree_data.apply_node_update(update);
                        }
                    }
                    GroveGdbUpdate::Proof(proof, node_updates, proof_tree) => {
                        for update in node_updates.into_iter() {
                            self.tree_data.apply_node_update(update);
                        }
                        self.proof_viewer = Some(ProofViewer::new(proof));
                        self.tree_data.set_proof_tree(proof_tree);
                        self.show_proof_viewer = true;
                    }
                    GroveGdbUpdate::RootUpdate(Some(root_update)) => {
                        self.tree_data.apply_root_node_update(root_update);
                    }
                    GroveGdbUpdate::RootUpdate(None) => {
                        log::warn!("Received no root node: GroveDB is empty");
                    }
                    GroveGdbUpdate::Session(session_id) => {
                        self.bus.set_session(session_id);
                        self.bus.fetch_command(FetchCommand::FetchRoot);
                    }
                    GroveGdbUpdate::Block => self.blocked = true,
                    GroveGdbUpdate::Unblock => self.blocked = false,
                }
            } else {
                log::error!("Protocol thread was terminated, can't receive updates anymore");
            }
        }

        self.draw_log_panel(ctx);

        self.draw_profiles_panel(ctx);

        self.draw_query_builder_panel(ctx);

        self.draw_proof_viewer_panel(ctx);

        self.draw_merk_view_panel(ctx);

        if self.show_help {
            egui::Window::new("Help")
                .open(&mut self.show_help)
                .show(ctx, help::show_help);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.tree_view.draw(
                ui,
                &self.bus,
                self.merk_panel_width / 2.,
                self.profiles_view.active_profile_root_ctx(),
                &mut self.tree_data,
                &self.focused_subtree,
            );
        });

        self.bus.process_actions(|action| match action {
            bus::UserAction::FocusSubtree(path) => {
                if let Some((parent_path, parent_key)) = path.parent_with_key() {
                    self.bus.fetch_command(FetchCommand::FetchNode {
                        path: parent_path.to_vec(),
                        key: parent_key,
                    })
                }
                self.focused_subtree = Some(FocusedSubree { path, key: None })
            }
            bus::UserAction::FocusSubtreeKey(path, key) => {
                if let Some((parent_path, parent_key)) = path.parent_with_key() {
                    self.bus.fetch_command(FetchCommand::FetchNode {
                        path: parent_path.to_vec(),
                        key: parent_key,
                    })
                }
                self.focused_subtree = Some(FocusedSubree { path, key: Some(key) })
            }
            bus::UserAction::DropFocus => self.focused_subtree = None,
            bus::UserAction::SelectMerkView(path) => {
                let key = self.tree_data.get(path).root_key.as_ref().cloned();
                if let Some(key) = key {
                    self.tree_data.select_for_merk(path);
                    self.bus.fetch_command(FetchCommand::FetchNode {
                        path: path.to_vec(),
                        key,
                    });
                }
            }
        });

        self.dark_theme = ctx.style().visuals.dark_mode;
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

pub(crate) struct FocusedSubree<'pa> {
    pub path: Path<'pa>,
    pub key: Option<Key>,
}
