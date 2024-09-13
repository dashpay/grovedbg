mod reference_view;

use eframe::egui::{self, Context, Label, Layout, RichText, Vec2};
use grovedbg_types::{CryptoHash, Element, Key};
use reference_view::draw_reference;

use super::{ElementViewContext, NODE_WIDTH};
use crate::{
    bytes_utils::{binary_label, binary_label_colored, bytes_by_display_variant, BytesDisplayVariant},
    path_ctx::{full_path_display, full_path_display_iter},
    protocol::ProtocolCommand,
    theme::element_to_color,
};

const ELEMENT_HEIGHT: f32 = 20.;

/// Same as `Element` of `grovedbg-types` except with an addition of
/// `SubtreePlaceholder` to represent known but incomplete subtree mentions.
pub(crate) enum ElementOrPlaceholder {
    Element(Element),
    Placeholder,
}

pub(crate) struct ElementView {
    pub(crate) key: Key,
    pub(crate) value: ElementOrPlaceholder,
    pub(crate) left_child: Option<Key>,
    pub(crate) right_child: Option<Key>,
    pub(crate) kv_digest_hash: Option<CryptoHash>,
    pub(crate) value_hash: Option<CryptoHash>,
    pub(crate) value_display: BytesDisplayVariant,
    pub(crate) flags_display: BytesDisplayVariant,
    pub(crate) kv_digest_hash_display: BytesDisplayVariant,
    pub(crate) value_hash_display: BytesDisplayVariant,
    pub(crate) node_hash: Option<CryptoHash>,
    pub(crate) node_hash_display: BytesDisplayVariant,
    pub(crate) show_hashes: bool,
    pub(crate) show_reference_details: bool,
    pub(crate) merk_visible: bool,
}

impl ElementView {
    pub(crate) fn new_placeholder(key: Key) -> Self {
        Self {
            key,
            value: ElementOrPlaceholder::Placeholder,
            left_child: None,
            right_child: None,
            kv_digest_hash: None,
            value_hash: None,
            value_display: Default::default(),
            flags_display: Default::default(),
            kv_digest_hash_display: BytesDisplayVariant::Hex,
            value_hash_display: BytesDisplayVariant::Hex,
            node_hash: None,
            node_hash_display: BytesDisplayVariant::Hex,
            show_hashes: Default::default(),
            show_reference_details: Default::default(),
            merk_visible: false,
        }
    }

    pub(crate) fn new(
        key: Key,
        value: ElementOrPlaceholder,
        left_child: Option<Key>,
        right_child: Option<Key>,
        kv_digest_hash: Option<CryptoHash>,
        value_hash: Option<CryptoHash>,
    ) -> Self {
        let value_display = if let ElementOrPlaceholder::Element(Element::Item { value, .. }) = &value {
            BytesDisplayVariant::guess(&value)
        } else {
            BytesDisplayVariant::Hex
        };
        Self {
            key,
            value,
            left_child,
            right_child,
            value_display,
            kv_digest_hash,
            value_hash,
            flags_display: BytesDisplayVariant::U8,
            kv_digest_hash_display: BytesDisplayVariant::Hex,
            value_hash_display: BytesDisplayVariant::Hex,
            node_hash: None,
            node_hash_display: BytesDisplayVariant::Hex,
            show_hashes: false,
            show_reference_details: false,
            merk_visible: false,
        }
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui, element_view_context: &mut ElementViewContext) {
        let ctx: Context = ui.ctx().clone();
        let path_with_key = element_view_context.path().child(self.key.clone());

        // Draw key
        ui.horizontal(|key_line| {
            if key_line
                .button(egui_phosphor::regular::ARROW_CLOCKWISE)
                .on_hover_text("Refetch the node")
                .clicked()
            {
                element_view_context
                    .bus
                    .protocol_command(ProtocolCommand::FetchNode {
                        path: element_view_context.path().to_vec(),
                        key: self.key.clone(),
                    });
            }
            if key_line
                .button(egui_phosphor::regular::HASH)
                .on_hover_text("Show item hashes received from GroveDB")
                .clicked()
            {
                self.show_hashes = !self.show_hashes;
            }

            if let Some(alias) = element_view_context.profile_ctx().key_view(&self.key) {
                key_line.add(
                    Label::new(RichText::new(alias).color(element_to_color(&ctx, &self.value))).truncate(),
                );
            } else {
                let display_variant_old = path_with_key
                    .get_display_variant()
                    .expect("None variant represents root subtree and there can be no parent to toggle it");
                let mut display_variant: BytesDisplayVariant = display_variant_old;

                binary_label_colored(
                    key_line,
                    &self.key,
                    &mut display_variant,
                    element_to_color(&ctx, &self.value),
                );

                if display_variant != display_variant_old {
                    path_with_key.update_display_variant(display_variant);
                }
            }
        });

        // Draw value
        let layout = Layout::top_down(egui::Align::Min);
        ui.allocate_ui_with_layout(
            Vec2::new(NODE_WIDTH, ELEMENT_HEIGHT),
            layout,
            |value_ui: &mut egui::Ui| {
                match &self.value {
                    ElementOrPlaceholder::Element(Element::Item { value, element_flags }) => {
                        binary_label(value_ui, value, &mut self.value_display);
                        if let Some(flags) = element_flags {
                            value_ui.horizontal(|line| {
                                line.label("Flags:");
                                binary_label(line, flags, &mut self.flags_display);
                            });
                        }
                    }
                    ElementOrPlaceholder::Element(Element::SumItem { value, element_flags }) => {
                        value_ui.label(format!("Value: {value}"));
                        if let Some(flags) = element_flags {
                            value_ui.horizontal(|line| {
                                line.label("Flags:");
                                binary_label(line, flags, &mut self.flags_display);
                            });
                        }
                    }
                    ElementOrPlaceholder::Element(Element::Reference(reference)) => {
                        draw_reference(
                            value_ui,
                            element_view_context,
                            &self.key,
                            reference,
                            &mut self.show_reference_details,
                            &mut self.flags_display,
                        )
                        .inspect_err(|e| {
                            let path_display = element_view_context.path().for_segments(|segments_iter| {
                                full_path_display(full_path_display_iter(
                                    segments_iter,
                                    element_view_context.profile_ctx(),
                                ))
                            });

                            log::warn!(
                                "Bad reference at {} under the key {}, {}",
                                path_display,
                                bytes_by_display_variant(
                                    &self.key,
                                    &path_with_key
                                        .get_display_variant()
                                        .unwrap_or_else(|| BytesDisplayVariant::guess(&self.key)),
                                ),
                                e.0,
                            );
                        })
                        .unwrap_or_else(|_| {
                            value_ui.label("Bad reference");
                        });
                    }
                    ElementOrPlaceholder::Element(Element::Sumtree {
                        sum, element_flags, ..
                    }) => {
                        value_ui.horizontal(|line| {
                            element_view_context
                                .path()
                                .child(self.key.clone())
                                .for_visible_mut(|subtree_visible| {
                                    line.checkbox(subtree_visible, "");
                                    *subtree_visible
                                });
                            if line.button(egui_phosphor::regular::MAGNIFYING_GLASS).clicked() {
                                element_view_context.focus_child_subtree(self.key.clone());
                            }
                            line.label(format!("Sum: {sum}"));
                        });
                        if let Some(flags) = element_flags {
                            value_ui.horizontal(|line| {
                                line.label("Flags:");
                                binary_label(line, flags, &mut self.flags_display);
                            });
                        }
                    }
                    ElementOrPlaceholder::Element(Element::Subtree { element_flags, .. }) => {
                        value_ui.horizontal(|line| {
                            element_view_context
                                .path()
                                .child(self.key.clone())
                                .for_visible_mut(|subtree_visible| {
                                    line.checkbox(subtree_visible, "");
                                    *subtree_visible
                                });
                            if line.button(egui_phosphor::regular::MAGNIFYING_GLASS).clicked() {
                                element_view_context.focus_child_subtree(self.key.clone());
                            }
                            line.label("Subtree");
                        });
                        if let Some(flags) = element_flags {
                            value_ui.horizontal(|line| {
                                line.label("Flags:");
                                binary_label(line, flags, &mut self.flags_display);
                            });
                        }
                    }
                    ElementOrPlaceholder::Placeholder => {
                        value_ui.label("Placeholder");
                    }
                };
                if self.show_hashes {
                    value_ui.horizontal(|line| {
                        if let Some(hash) = &self.node_hash {
                            line.label("Node hash:");
                            binary_label(line, hash, &mut self.node_hash_display);
                        }
                    });
                    value_ui.horizontal(|line| {
                        if let Some(hash) = &self.kv_digest_hash {
                            line.label("KV digest hash:");
                            binary_label(line, hash, &mut self.kv_digest_hash_display);
                        }
                    });
                    value_ui.horizontal(|line| {
                        if let Some(hash) = &self.value_hash {
                            line.label("Value hash:");
                            binary_label(line, hash, &mut self.value_hash_display);
                        }
                    });
                }
            },
        );
    }
}
