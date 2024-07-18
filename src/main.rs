mod fetch;
mod model;
mod profiles;
#[cfg(test)]
mod test_utils;
mod ui;

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use eframe::egui::{self, emath::TSTransform, Vec2, Visuals};
use fetch::Message;
use model::{
    path_display::{Path, PathCtx},
    Node,
};
use profiles::{drive_profile, EnabledProfile};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use ui::QueryBuilder;

use crate::{model::Tree, ui::TreeDrawer};

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use profiles::drive_profile;

    egui_logger::init().unwrap();
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    let (sender, receiver) = channel(10);
    let path_ctx: &'static PathCtx = Box::leak(Box::new(PathCtx::new()));

    drive_profile().enable_profile(path_ctx);

    let tree: Arc<Mutex<Tree>> = Arc::new(Mutex::new(Tree::new(path_ctx)));

    let t = Arc::clone(&tree);
    wasm_bindgen_futures::spawn_local(async move {
        fetch::process_messages(receiver, t.as_ref(), path_ctx).await;
    });

    sender.blocking_send(Message::FetchRoot).unwrap();

    wasm_bindgen_futures::spawn_local(async move {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(move |cc| Box::new(App::new(cc, tree, path_ctx, sender))),
            )
            .await
            .expect("failed to start eframe");
    });
}

struct App<'c> {
    transform: TSTransform,
    tree: Arc<Mutex<Tree<'c>>>,
    path_ctx: &'c PathCtx,
    sender: Sender<Message>,
    // TODO: shouldn't be hardcoded eventually
    drive_profile: Option<EnabledProfile<'c>>,
    query_builder: QueryBuilder<'c>,
}

impl<'c> App<'c> {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        tree: Arc<Mutex<Tree<'c>>>,
        path_ctx: &'c PathCtx,
        sender: Sender<Message>,
    ) -> Self {
        App {
            transform: TSTransform::from_translation(Vec2::new(1000., 500.)),
            tree,
            path_ctx,
            query_builder: QueryBuilder::new(path_ctx, sender.clone()),
            sender,
            drive_profile: Some(drive_profile().enable_profile(path_ctx)),
        }
    }
}

impl<'c> eframe::App for App<'c> {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_visuals(Visuals::dark());

            ui.label("GroveDB Visualizer");
            ui.separator();

            let (id, rect) = ui.allocate_space(ui.available_size());

            let response = ui.interact(rect, id, egui::Sense::click_and_drag());
            // Allow dragging the background as well.
            if response.dragged() {
                self.transform.translation += response.drag_delta();
            }

            // Plot-like reset
            if response.double_clicked() {
                self.transform = TSTransform::default();
            }

            let local_transform =
                TSTransform::from_translation(ui.min_rect().left_top().to_vec2()) * self.transform;

            if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                // Note: doesn't catch zooming / panning if a button in this PanZoom container
                // is hovered.
                if response.hovered() {
                    let pointer_in_layer = local_transform.inverse() * pointer;
                    let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                    let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

                    // Zoom in on pointer:
                    self.transform = self.transform
                        * TSTransform::from_translation(pointer_in_layer.to_vec2())
                        * TSTransform::from_scaling(zoom_delta)
                        * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                    // Pan:
                    self.transform = TSTransform::from_translation(pan_delta) * self.transform;
                }
            }

            {
                let lock = self.tree.lock().unwrap();
                let drawer = TreeDrawer::new(ui, &mut self.transform, rect, &lock, &self.sender);
                drawer.draw_tree();
            }

            egui::Window::new("Log").default_pos((0., 100.)).show(ctx, |ui| {
                egui_logger::logger_ui(ui);
            });

            egui::Window::new("Profiles")
                .default_pos((0., 500.))
                .show(ctx, |ui| {
                    let mut drive_profile_checked = self.drive_profile.is_some();
                    ui.checkbox(&mut drive_profile_checked, "Drive profile");
                    if !drive_profile_checked {
                        if let Some(enabled_profile) = self.drive_profile.take() {
                            enabled_profile.disable();
                        }
                    } else if self.drive_profile.is_none() {
                        self.drive_profile = Some(drive_profile().enable_profile(self.path_ctx));
                    } else {
                        if let Some(enabled_profile) = &self.drive_profile {
                            ui.collapsing("Profile Subtrees", |profile_subtrees| {
                                for profile_path in enabled_profile.iter_aliases() {
                                    if let Some(alias) = profile_path.get_profiles_alias() {
                                        profile_subtrees.horizontal(|line| {
                                            if line.button("Fetch").clicked() {
                                                if let Some((path, key)) = profile_path.parent_with_key() {
                                                    let _ = self
                                                        .sender
                                                        .blocking_send(Message::FetchNode {
                                                            path: path.to_vec(),
                                                            key,
                                                            show: true,
                                                        })
                                                        .inspect_err(|_| {
                                                            log::error!("Can't reach data fetching thread")
                                                        });
                                                }
                                            }
                                            {
                                                let lock = self.tree.lock().unwrap();
                                                if let Some(subtree) = lock.get_subtree(profile_path) {
                                                    if line.button("🔎").clicked() {
                                                        subtree.subtree().set_visible(true);
                                                        self.transform = TSTransform::from_translation(
                                                            subtree
                                                                .subtree()
                                                                .get_subtree_input_point()
                                                                .map(|point| {
                                                                    point.to_vec2() + Vec2::new(-1500., -900.)
                                                                })
                                                                .unwrap_or_default(),
                                                        )
                                                        .inverse();
                                                    }
                                                }
                                            }
                                            line.label(alias);
                                        });
                                    }
                                }
                            });
                        }
                    }
                });

            egui::Window::new("Query builder")
                .default_pos((0., 600.))
                .default_open(false)
                .show(ctx, |ui| self.query_builder.draw(ui));

            ctx.request_repaint_after(Duration::from_secs(5));
        });
    }
}
