use eframe::egui::{self, Color32, Label, Layout, RichText, Vec2};
use grovedbg_types::Element;

use super::SubtreeViewContext;
use crate::bytes_utils::{binary_label, binary_label_colored, BytesDisplayVariant};

const ELEMENT_WIDTH: f32 = 300.;
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
    kv_digest_hash: Option<Vec<u8>>,
    value_hash: Option<Vec<u8>>,
    key_display: BytesDisplayVariant,
    value_display: BytesDisplayVariant,
    flags_display: BytesDisplayVariant,
    kv_digest_hash_display: BytesDisplayVariant,
    value_hash_display: BytesDisplayVariant,
    show_hashes: bool,
    subtree_visible: bool,
}

impl ElementView {
    pub(crate) fn new(
        key: Vec<u8>,
        value: WrappedElement,
        kv_digest_hash: Option<Vec<u8>>,
        value_hash: Option<Vec<u8>>,
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
            subtree_visible: false,
        }
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui, subtree_view_context: &mut SubtreeViewContext) {
        // Draw key
        ui.horizontal(|key_line| {
            if key_line.button("#").clicked() {
                self.show_hashes = !self.show_hashes;
            }
            if self.value.is_tree() {
                key_line.checkbox(&mut self.subtree_visible, "");
                if key_line.button("🔎").clicked() {
                    self.subtree_visible = true;
                    subtree_view_context.focus_child(self.key.clone());
                }

                let path = subtree_view_context.path().child(self.key.clone());

                if let Some(alias) = path.get_profiles_alias() {
                    key_line.add(
                        Label::new(RichText::new(alias).color(element_to_color(&self.value))).truncate(),
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
                        element_to_color(&self.value),
                    );

                    if display_variant != display_variant_old {
                        path.update_display_variant(display_variant);
                    }
                }
            }
        });

        // Draw value
        let layout = Layout::top_down(egui::Align::Min);
        ui.allocate_ui_with_layout(
            Vec2::new(ELEMENT_WIDTH, ELEMENT_HEIGHT),
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
                    if let Some(kv_digest_hash) = &self.kv_digest_hash {
                        value_ui.horizontal(|line| {
                            line.label("KV digest hash:");
                            binary_label(line, &kv_digest_hash, &mut self.kv_digest_hash_display);
                        });
                    }
                    if let Some(value_hash) = &self.value_hash {
                        value_ui.horizontal(|line| {
                            line.label("Value hash:");
                            binary_label(line, &value_hash, &mut self.value_hash_display);
                        });
                    }
                }
            },
        );
    }
}

fn element_to_color(element: &WrappedElement) -> Color32 {
    match element {
        WrappedElement::SubtreePlaceholder => Color32::DARK_RED,
        WrappedElement::Element(Element::Item { .. }) => Color32::GRAY,
        WrappedElement::Element(Element::SumItem { .. }) => Color32::DARK_GREEN,
        WrappedElement::Element(Element::Subtree { .. }) => Color32::GOLD,
        WrappedElement::Element(Element::Sumtree { .. }) => Color32::GREEN,
        WrappedElement::Element(_) => Color32::DARK_BLUE,
    }
}
