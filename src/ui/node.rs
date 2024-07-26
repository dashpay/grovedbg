use std::borrow::{Borrow, BorrowMut};

use eframe::{
    egui::{self, Label, Layout, RichText, Vec2},
    emath::TSTransform,
    epaint::{Color32, Stroke},
};
use tokio::sync::mpsc::Sender;

use super::{
    common::{binary_label, binary_label_colored, bytes_by_display_variant, path_label},
    tree::CELL_X,
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
                    let _ = sender
                        .blocking_send(Message::FetchNode {
                            path: node_ctx.path().to_vec(),
                            key: node_ctx
                                .node()
                                .left_child
                                .as_ref()
                                .expect("checked above")
                                .clone(),
                            show: false,
                        })
                        .inspect_err(|_| log::error!("Can't reach data fetching thread"));
                }
                footer.label("|");
                if footer
                    .add_enabled(node_ctx.node().right_child.is_some(), egui::Button::new("➡"))
                    .clicked()
                {
                    node_ctx.set_right_visible();

                    let _ = sender
                        .blocking_send(Message::FetchNode {
                            path: node_ctx.path().to_vec(),
                            key: node_ctx
                                .node()
                                .right_child
                                .as_ref()
                                .expect("checked above")
                                .clone(),
                            show: false,
                        })
                        .inspect_err(|_| log::error!("Can't reach data fetching thread"));
                }
            });
        })
        .response;
}

pub(crate) fn draw_element(ui: &mut egui::Ui, transform: &mut TSTransform, node_ctx: &NodeCtx) {
    // Draw key
    ui.horizontal(|key_line| {
        if key_line.button("#").clicked() {
            let mut state = node_ctx.node().ui_state.borrow_mut();
            state.show_hashes = !state.show_hashes;
        }
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
                        .map(|point| point.to_vec2() + Vec2::new(-1500., -900.))
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

        if let Some(alias) = node_ctx
            .child_subtree_ctx()
            .map(|ctx| ctx.path().get_profiles_alias())
            .flatten()
        {
            key_line.add(
                Label::new(RichText::new(alias).color(element_to_color(&node_ctx.node().element)))
                    .truncate(true),
            );
        } else {
            node_ctx.with_key_display_variant(|display_variant| {
                binary_label_colored(
                    key_line,
                    node_ctx.key(),
                    display_variant,
                    element_to_color(&node_ctx.node().element),
                )
            });
        }
    });

    // Draw value
    let node = node_ctx.node();

    let layout = Layout::top_down(egui::Align::Min);
    ui.allocate_ui_with_layout(Vec2::new(CELL_X, 20.), layout, |value_ui: &mut egui::Ui| {
        match &node.element {
            Element::Item { value, element_flags } => {
                let mut state = node.ui_state.borrow_mut();
                binary_label(value_ui, value, &mut state.item_display_variant);
                if let Some(flags) = element_flags {
                    value_ui.horizontal(|line| {
                        line.label("Flags:");
                        binary_label(line, flags, &mut state.flags_display_variant);
                    });
                }
            }
            Element::SumItem { value, element_flags } => {
                value_ui.label(format!("Value: {value}"));
                if let Some(flags) = element_flags {
                    value_ui.horizontal(|line| {
                        line.label("Flags:");
                        binary_label(line, flags, &mut node.ui_state.borrow_mut().flags_display_variant);
                    });
                }
            }
            Element::Reference {
                path,
                key,
                element_flags,
            } => {
                let mut state = node.ui_state.borrow_mut();
                path_label(value_ui, *path);
                value_ui.horizontal(|line| {
                    line.add_space(20.0);
                    line.label(bytes_by_display_variant(key, &mut state.item_display_variant));
                });
                if let Some(flags) = element_flags {
                    value_ui.horizontal(|line| {
                        line.label("Flags:");
                        binary_label(line, flags, &mut state.flags_display_variant);
                    });
                }
            }
            Element::Sumtree {
                sum, element_flags, ..
            } => {
                value_ui.label(format!("Sum: {sum}"));
                if let Some(flags) = element_flags {
                    value_ui.horizontal(|line| {
                        line.label("Flags:");
                        binary_label(line, flags, &mut node.ui_state.borrow_mut().flags_display_variant);
                    });
                }
            }
            Element::Subtree { element_flags, .. } => {
                value_ui.label("Subtree");
                if let Some(flags) = element_flags {
                    value_ui.horizontal(|line| {
                        line.label("Flags:");
                        binary_label(line, flags, &mut node.ui_state.borrow_mut().flags_display_variant);
                    });
                }
            }
            Element::SubtreePlaceholder => {
                value_ui.label("Subtree");
            }
        };
        let mut state = node_ctx.node().ui_state.borrow_mut();
        if state.show_hashes {
            if let Some(kv_digest_hash) = node_ctx.node().kv_digest_hash {
                value_ui.horizontal(|line| {
                    line.label("KV digest hash:");
                    binary_label(line, &kv_digest_hash, &mut state.kv_digest_hash_display_variant);
                });
            }
            if let Some(value_hash) = node_ctx.node().value_hash {
                value_ui.horizontal(|line| {
                    line.label("Value hash:");
                    binary_label(line, &value_hash, &mut state.value_hash_display_variant);
                });
            }
        }
    });
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
