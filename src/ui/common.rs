//! Module of useful components

use std::fmt::Write;

use eframe::{
    egui::{self, Label, Response, RichText, Sense},
    epaint::Color32,
};
use integer_encoding::VarInt;

use crate::model::path_display::Path;

const MAX_BYTES: usize = 10;
const MAX_HEX_LENGTH: usize = 32;
const HEX_PARTS_LENGTH: usize = 12;

fn bytes_as_slice(bytes: &[u8]) -> String {
    if bytes.len() <= MAX_BYTES {
        format!("{:?}", bytes)
    } else {
        let mut buf = String::from("[");
        bytes.iter().take(MAX_BYTES).for_each(|b| {
            let _ = write!(buf, "{b},");
        });
        buf.push_str("...");
        buf
    }
}

pub(crate) fn bytes_as_hex(bytes: &[u8]) -> String {
    let hex_str = hex::encode(bytes);
    if hex_str.len() <= MAX_HEX_LENGTH {
        hex_str
    } else {
        let mut buf = String::from(&hex_str[0..HEX_PARTS_LENGTH]);
        buf.push_str("..");
        buf.push_str(&hex_str[(hex_str.len() - HEX_PARTS_LENGTH)..]);
        buf
    }
}

pub(crate) fn bytes_as_int(bytes: &[u8]) -> String {
    if let Ok(arr) = bytes.try_into() {
        i64::from_be_bytes(arr).to_string()
    } else {
        String::from("[E]: must be 8 bytes")
    }
}

fn bytes_as_varint(bytes: &[u8]) -> String {
    i64::decode_var(bytes)
        .map(|(x, _)| x.to_string())
        .unwrap_or_else(|| "varint: MSB".to_owned())
}

pub(crate) fn bytes_by_display_variant(bytes: &[u8], display_variant: &DisplayVariant) -> String {
    match display_variant {
        DisplayVariant::U8 => bytes_as_slice(bytes),
        DisplayVariant::String => String::from_utf8_lossy(bytes).to_string(),
        DisplayVariant::Hex => bytes_as_hex(bytes),
        DisplayVariant::Int => bytes_as_int(bytes),
        DisplayVariant::VarInt => bytes_as_varint(bytes),
    }
}

pub(crate) fn bytes_by_display_variant_explicit(bytes: &[u8], display_variant: &DisplayVariant) -> String {
    match display_variant {
        DisplayVariant::U8 => format!("{bytes:?}"),
        DisplayVariant::String => String::from_utf8_lossy(bytes).to_string(),
        DisplayVariant::Hex => hex::encode(bytes),
        DisplayVariant::Int => bytes_as_int(bytes),
        DisplayVariant::VarInt => bytes_as_varint(bytes),
    }
}

/// Represent binary data different ways and to choose from
pub(crate) fn binary_label_colored<'a>(
    ui: &mut egui::Ui,
    bytes: &[u8],
    display_variant: &mut DisplayVariant,
    color: Color32,
) -> Response {
    display_variant_dropdown(ui, bytes, display_variant, color)
}

fn display_variant_dropdown<'a>(
    ui: &mut egui::Ui,
    bytes: &[u8],
    display_variant: &mut DisplayVariant,
    color: Color32,
) -> Response {
    let text = bytes_by_display_variant(bytes, display_variant);
    let response = ui
        .add(
            Label::new(RichText::new(text).color(color))
                .truncate(true)
                .sense(Sense::click()),
        )
        .on_hover_ui(|hover| {
            hover.label(bytes_by_display_variant_explicit(bytes, display_variant));
        });

    response.context_menu(|menu| {
        menu.radio_value(display_variant, DisplayVariant::U8, "u8 array");
        menu.radio_value(display_variant, DisplayVariant::String, "UTF-8 String");
        menu.radio_value(display_variant, DisplayVariant::Hex, "Hex String");
        menu.radio_value(display_variant, DisplayVariant::Int, "i64");
        menu.radio_value(display_variant, DisplayVariant::VarInt, "VarInt");
    });
    response
}

pub(crate) fn binary_label<'a>(
    ui: &mut egui::Ui,
    bytes: &[u8],
    display_variant: &mut DisplayVariant,
) -> Response {
    binary_label_colored(ui, bytes, display_variant, Color32::GRAY)
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub(crate) enum DisplayVariant {
    #[default]
    U8,
    String,
    Hex,
    Int,
    VarInt,
}

impl DisplayVariant {
    pub fn guess(bytes: &[u8]) -> Self {
        match bytes.len() {
            1 => DisplayVariant::U8,
            8 => DisplayVariant::Int,
            32 => DisplayVariant::Hex,
            _ => DisplayVariant::String,
        }
    }
}

pub(crate) fn path_label<'a>(ui: &mut egui::Ui, path: Path<'a>) -> egui::Response {
    path.for_segments(|mut iter| {
        if let Some(key) = iter.next_back() {
            let text = path.get_profiles_alias().unwrap_or_else(|| {
                let mut text = String::from("[");
                if let Some(parent) = iter.next_back() {
                    if iter.next_back().is_some() {
                        text.push_str("..., ");
                    }
                    text.push_str(&bytes_by_display_variant(parent.bytes(), &parent.display()));
                    text.push_str(", ");
                }

                text.push_str(&bytes_by_display_variant(key.bytes(), &key.display()));
                text.push_str("]");
                text
            });

            let response = ui.label(text);

            if response.clicked() {
                path.select_for_query();
            }

            response.on_hover_ui_at_pointer(|hover_ui| {
                let mut text = String::from("[");
                path.for_segments(|mut iter| {
                    let last = iter.next_back();
                    iter.for_each(|segment| {
                        text.push_str(&bytes_by_display_variant(segment.bytes(), &segment.display()));
                        text.push_str(", ");
                    });
                    last.into_iter().for_each(|segment| {
                        text.push_str(&bytes_by_display_variant(segment.bytes(), &segment.display()));
                        text.push_str("]");
                    });
                    hover_ui.label(text);
                })
            })
        } else {
            ui.label("Root subtree")
        }
    })
}
