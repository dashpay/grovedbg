use std::borrow::Borrow;

use eframe::{
    egui::{self, Vec2},
    emath::TSTransform,
    epaint::{Color32, Stroke},
};
use tokio::sync::mpsc::Sender;

use super::{
    common::{binary_label, binary_label_colored, bytes_by_display_variant, path_label},
    DisplayVariant, TreeDrawer,
};
use crate::{
    fetch::Message,
    model::{Element, NodeCtx},
};

pub(crate) fn draw_node<'a, 'c>(
    ui: &mut egui::Ui,
    transform: &mut TSTransform,
    sender: &Sender<Message>,
    node_ctx: &NodeCtx<'a, 'c>,
) {
    let mut stroke = Stroke::default();
    stroke.color = element_to_color(&node_ctx.node().element);
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

            node_ctx.with_key_display_variant(|display_variant| {
                binary_label(ui, &node_ctx.key(), display_variant);
            });

            draw_element(ui, transform, node_ctx);

            ui.horizontal(|footer| {
                if footer
                    .add_enabled(node_ctx.node().left_child.is_some(), egui::Button::new("⬅"))
                    .clicked()
                {
                    node_ctx.set_left_visible();
                    sender.blocking_send(Message::FetchNode {
                        path: node_ctx.path().to_vec(),
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
                    .add_enabled(
                        node_ctx.node().right_child.is_some(),
                        egui::Button::new("➡"),
                    )
                    .clicked()
                {
                    node_ctx.set_right_visible();

                    sender.blocking_send(Message::FetchNode {
                        path: node_ctx.path().to_vec(),
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

pub(crate) fn draw_element(ui: &mut egui::Ui, transform: &mut TSTransform, node_ctx: &NodeCtx) {
    // Draw key
    ui.horizontal(|key_line| {
        if matches!(
            node_ctx.node().element,
            Element::Subtree { .. } | Element::Sumtree { .. }
        ) {
            let prev_visibility = node_ctx.subtree_ctx().is_child_visible(node_ctx.key());
            let mut visibility = prev_visibility;
            key_line.checkbox(&mut visibility, "");

            if visibility && key_line.button("🔎").clicked() {
                *transform = TSTransform::from_translation(
                    node_ctx
                        .child_subtree_ctx()
                        .map(|ctx| ctx.subtree().get_subtree_input_point())
                        .flatten()
                        .map(|point| point.to_vec2() + Vec2::new(-1000., -500.))
                        .unwrap_or_default(),
                )
                .inverse();
            }
            if prev_visibility != visibility {
                node_ctx
                    .subtree_ctx()
                    .set_child_visibility(node_ctx.key(), visibility);
            }
        }

        node_ctx.with_key_display_variant(|display_variant| {
            binary_label_colored(
                key_line,
                node_ctx.key(),
                display_variant,
                element_to_color(&node_ctx.node().element),
            )
        });
    });

    // Draw value
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
            path_label(ui, *path);
            ui.horizontal(|line| {
                line.add_space(20.0);
                line.label(bytes_by_display_variant(
                    key,
                    &mut node.ui_state.borrow_mut().item_display_variant,
                ));
            });
        }
        Element::Sumtree { sum, .. } => {
            ui.label(format!("Sum: {sum}"));
        }
        _ => {}
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
