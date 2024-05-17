mod fetch;
mod model;
#[cfg(test)]
mod test_utils;
mod ui;

use std::sync::{Arc, Mutex};

use eframe::egui::{self, emath::TSTransform, Visuals};
use fetch::Message;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    model::Tree,
    ui::{draw_legend, TreeDrawer},
};

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    let (sender, receiver) = channel(10);
    let tree: Arc<Mutex<Tree>> = Default::default();

    let t = Arc::clone(&tree);
    wasm_bindgen_futures::spawn_local(async move {
        fetch::process_messages(receiver, t).await;
    });

    sender.blocking_send(Message::FetchRoot).unwrap();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(move |cc| Box::new(App::new(cc, tree, sender))),
            )
            .await
            .expect("failed to start eframe");
    });
}

struct App {
    transform: TSTransform,
    tree: Arc<Mutex<Tree>>,
    sender: Sender<Message>,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        tree: Arc<Mutex<Tree>>,
        sender: Sender<Message>,
    ) -> Self {
        App {
            transform: Default::default(),
            tree,
            sender,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
                let drawer = TreeDrawer::new(ui, self.transform, rect, &lock, &self.sender);
                drawer.draw_tree();
            }

            draw_legend(ui);
        });
    }
}
