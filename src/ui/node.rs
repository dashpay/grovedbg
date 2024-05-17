use eframe::{
    egui,
    epaint::{Color32, Stroke},
};
use tokio::sync::mpsc::Sender;

use super::common::{binary_label, bytes_by_display_variant, path_label};
use crate::{
    fetch::Message,
    model::{Element, Node, NodeCtx},
};

pub(crate) fn draw_node<'a>(ui: &mut egui::Ui, sender: &Sender<Message>, node_ctx: NodeCtx<'a>) {
    let (node, _, key) = node_ctx.split();

    let mut stroke = Stroke::default();
    stroke.color = element_to_color(&node.element);
    stroke.width = 1.0;

    egui::Frame::default()
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(8.0))
        .stroke(stroke)
        .fill(Color32::BLACK)
        .show(ui, |ui| {
            ui.style_mut().wrap = Some(false);

            ui.collapsing("🖧", |menu| {
                if menu.button("Collapse").clicked() {
                    node_ctx.subtree().set_collapsed();
                }
            });

            binary_label(ui, key, &mut node.ui_state.borrow_mut().key_display_variant);
            draw_element(ui, node_ctx);

            ui.horizontal(|footer| {
                if footer
                    .add_enabled(node.left_child.is_some(), egui::Button::new("⬅"))
                    .clicked()
                {
                    node_ctx.set_left_visible();
                    sender.blocking_send(Message::FetchNode {
                        path: node_ctx.path().clone(),
                        key: node_ctx
                            .node()
                            .left_child
                            .as_ref()
                            .expect("checked above")
                            .clone(),
                    });
                }
                footer.label("|");
                if footer
                    .add_enabled(node.right_child.is_some(), egui::Button::new("➡"))
                    .clicked()
                {
                    node_ctx.set_right_visible();

                    sender.blocking_send(Message::FetchNode {
                        path: node_ctx.path().clone(),
                        key: node_ctx
                            .node()
                            .right_child
                            .as_ref()
                            .expect("checked above")
                            .clone(),
                    });
                }
            });
        })
        .response;
}

pub(crate) fn draw_element(ui: &mut egui::Ui, node_ctx: NodeCtx) {
    let node = node_ctx.node();
    match &node.element {
        Element::Item { value } => {
            binary_label(
                ui,
                value,
                &mut node.ui_state.borrow_mut().item_display_variant,
            );
        }
        Element::SumItem { value } => {
            ui.label(format!("Value: {value}"));
        }
        Element::Reference { path, key } => {
            path_label(
                ui,
                path,
                &mut node.ui_state.borrow_mut().item_display_variant,
            );
            ui.horizontal(|line| {
                line.add_space(20.0);
                line.label(bytes_by_display_variant(
                    key,
                    &mut node.ui_state.borrow_mut().item_display_variant,
                ));
            });
        }
        Element::Sumtree { sum, .. } => {
            let subtree_ctx = node_ctx.subtree_ctx();
            let prev_visibility = subtree_ctx.is_child_visible(node_ctx.key());
            let mut visibility = prev_visibility;
            ui.checkbox(&mut visibility, "");
            if prev_visibility != visibility {
                subtree_ctx.set_child_visibility(node_ctx.key(), visibility);
            }
            ui.label(format!("Sum: {sum}"));
        }
        Element::Subtree { .. } => {
            let subtree_ctx = node_ctx.subtree_ctx();
            let prev_visibility = subtree_ctx.is_child_visible(node_ctx.key());
            let mut visibility = prev_visibility;
            ui.checkbox(&mut visibility, "");
            if prev_visibility != visibility {
                subtree_ctx.set_child_visibility(node_ctx.key(), visibility);
            }
            ui.label("Subtree");
        }
        Element::SubtreePlaceholder => {
            let subtree_ctx = node_ctx.subtree_ctx();
            let prev_visibility = subtree_ctx.is_child_visible(node_ctx.key());
            let mut visibility = prev_visibility;
            ui.checkbox(&mut visibility, "");
            if prev_visibility != visibility {
                subtree_ctx.set_child_visibility(node_ctx.key(), visibility);
            }
            ui.label("Subtree");
        }
    }
}

pub(crate) fn element_to_color(element: &Element) -> Color32 {
    match element {
        Element::Item { .. } => Color32::WHITE,
        Element::SumItem { .. } => Color32::DARK_GREEN,
        Element::Reference { .. } => Color32::LIGHT_BLUE,
        Element::Subtree { .. } => Color32::GOLD,
        Element::SubtreePlaceholder => Color32::RED,
        Element::Sumtree { .. } => Color32::GREEN,
    }
}
