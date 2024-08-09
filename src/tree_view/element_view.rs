use eframe::egui::{self, Context, Label, Layout, RichText, Vec2};
use grovedbg_types::{CryptoHash, Element};

use super::{SubtreeViewContext, NODE_WIDTH};
use crate::{
    bytes_utils::{binary_label, binary_label_colored, BytesDisplayVariant},
    theme::element_to_color,
};

const ELEMENT_HEIGHT: f32 = 20.;

/// Same as `Element` of `grovedbg-types` except with an addition of
/// `SubtreePlaceholder` to represent known but incomplete subtree mentions.
pub(crate) enum WrappedElement {
    Element(Element),
    SubtreePlaceholder,
}

impl WrappedElement {
    fn is_tree(&self) -> bool {
        matches!(
            self,
            WrappedElement::SubtreePlaceholder
                | WrappedElement::Element(Element::Subtree { .. })
                | WrappedElement::Element(Element::Sumtree { .. })
        )
    }
}

pub(crate) struct ElementView {
    key: Vec<u8>,
    value: WrappedElement,
    kv_digest_hash: Option<CryptoHash>,
    value_hash: Option<CryptoHash>,
    key_display: BytesDisplayVariant,
    value_display: BytesDisplayVariant,
    flags_display: BytesDisplayVariant,
    kv_digest_hash_display: BytesDisplayVariant,
    value_hash_display: BytesDisplayVariant,
    show_hashes: bool,
}

impl ElementView {
    pub(crate) fn new(
        key: Vec<u8>,
        value: WrappedElement,
        kv_digest_hash: Option<CryptoHash>,
        value_hash: Option<CryptoHash>,
    ) -> Self {
        let key_display = BytesDisplayVariant::guess(&key);
        let value_display = if let WrappedElement::Element(Element::Item { value, .. }) = &value {
            BytesDisplayVariant::guess(&value)
        } else {
            BytesDisplayVariant::Hex
        };
        Self {
            key,
            value,
            key_display,
            value_display,
            kv_digest_hash,
            value_hash,
            flags_display: BytesDisplayVariant::U8,
            kv_digest_hash_display: BytesDisplayVariant::Hex,
            value_hash_display: BytesDisplayVariant::Hex,
            show_hashes: false,
        }
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui, subtree_view_context: &mut SubtreeViewContext) {
        let ctx: Context = ui.ctx().clone();
        // Draw key
        ui.horizontal(|key_line| {
            if key_line.button("#").clicked() {
                self.show_hashes = !self.show_hashes;
            }
            if self.value.is_tree() {
                if let Some(subtree_visible) = subtree_view_context.subtree_visibility_mut(&self.key) {
                    key_line.checkbox(subtree_visible, "");
                    if *subtree_visible {
                        if key_line.button("🔎").clicked() {
                            subtree_view_context.focus_child(self.key.clone());
                        }
                    }
                }

                let path = subtree_view_context.path().child(self.key.clone());

                if let Some(alias) = path.get_profiles_alias() {
                    key_line.add(
                        Label::new(RichText::new(alias).color(element_to_color(&ctx, &self.value)))
                            .truncate(),
                    );
                } else {
                    let display_variant_old = path.get_display_variant().expect(
                        "None variant represents root subtree and there can be no parent to toggle it",
                    );
                    let mut display_variant: BytesDisplayVariant = display_variant_old;

                    binary_label_colored(
                        key_line,
                        &self.key,
                        &mut display_variant,
                        element_to_color(&ctx, &self.value),
                    );

                    if display_variant != display_variant_old {
                        path.update_display_variant(display_variant);
                    }
                }
            } else {
                binary_label_colored(
                    key_line,
                    &self.key,
                    &mut self.key_display,
                    element_to_color(&ctx, &self.value),
                );
            }
        });

        // Draw value
        let layout = Layout::top_down(egui::Align::Min);
        ui.allocate_ui_with_layout(
            Vec2::new(NODE_WIDTH, ELEMENT_HEIGHT),
            layout,
            |value_ui: &mut egui::Ui| {
                match &self.value {
                    WrappedElement::Element(Element::Item { value, element_flags }) => {
                        binary_label(value_ui, value, &mut self.value_display);
                        if let Some(flags) = element_flags {
                            value_ui.horizontal(|line| {
                                line.label("Flags:");
                                binary_label(line, flags, &mut self.flags_display);
                            });
                        }
                    }
                    WrappedElement::Element(Element::SumItem { value, element_flags }) => {
                        value_ui.label(format!("Value: {value}"));
                        if let Some(flags) = element_flags {
                            value_ui.horizontal(|line| {
                                line.label("Flags:");
                                binary_label(line, flags, &mut self.flags_display);
                            });
                        }
                    }
                    // Element::Reference {
                    //     path,
                    //     key,
                    //     element_flags,
                    // } => {
                    //     let mut state = node.ui_state.borrow_mut();
                    //     path_label(value_ui, *path);
                    //     value_ui.horizontal(|line| {
                    //         line.add_space(20.0);
                    //         line.label(bytes_by_display_variant(key, &mut state.item_display_variant));
                    //     });
                    //     if let Some(flags) = element_flags {
                    //         value_ui.horizontal(|line| {
                    //             line.label("Flags:");
                    //             binary_label(line, flags, &mut state.flags_display_variant);
                    //         });
                    //     }
                    // }
                    WrappedElement::Element(Element::Sumtree {
                        sum, element_flags, ..
                    }) => {
                        value_ui.label(format!("Sum: {sum}"));
                        if let Some(flags) = element_flags {
                            value_ui.horizontal(|line| {
                                line.label("Flags:");
                                binary_label(line, flags, &mut self.flags_display);
                            });
                        }
                    }
                    WrappedElement::Element(Element::Subtree { element_flags, .. }) => {
                        value_ui.label("Subtree");
                        if let Some(flags) = element_flags {
                            value_ui.horizontal(|line| {
                                line.label("Flags:");
                                binary_label(line, flags, &mut self.flags_display);
                            });
                        }
                    }
                    WrappedElement::SubtreePlaceholder => {
                        value_ui.label("Subtree");
                    }
                    _ => todo!(), // references
                };
                if self.show_hashes {
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
