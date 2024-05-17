//! Module of useful components

use std::fmt::Write;

use eframe::{
    egui::{self, Label, Response, RichText, Sense},
    epaint::Color32,
};

use crate::model::Path;

const MAX_BYTES: usize = 10;
const MAX_HEX_LENGTH: usize = 20;
const HEX_PARTS_LENGTH: usize = 8;

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

pub(crate) fn bytes_by_display_variant(bytes: &[u8], display_variant: &DisplayVariant) -> String {
    match display_variant {
        DisplayVariant::U8 => bytes_as_slice(bytes),
        DisplayVariant::String => String::from_utf8_lossy(bytes).to_string(),
        DisplayVariant::Hex => bytes_as_hex(bytes),
    }
}

/// Represent binary data different ways and to choose from
pub(crate) fn binary_label_colored<'a>(
    ui: &mut egui::Ui,
    bytes: &[u8],
    display_variant: &mut DisplayVariant,
    color: Color32,
) -> Response {
    let text = bytes_by_display_variant(bytes, &display_variant);
    display_variant_dropdown(ui, &text, display_variant, color)
}

fn display_variant_dropdown<'a>(
    ui: &mut egui::Ui,
    text: &str,
    display_variant: &mut DisplayVariant,
    color: Color32,
) -> Response {
    let response = ui.add(Label::new(RichText::new(text).color(color)).sense(Sense::click()));
    response.context_menu(|menu| {
        menu.radio_value(display_variant, DisplayVariant::U8, "Integers");
        menu.radio_value(display_variant, DisplayVariant::String, "UTF-8 String");
        menu.radio_value(display_variant, DisplayVariant::Hex, "Hex String");
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

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub(crate) enum DisplayVariant {
    #[default]
    U8,
    String,
    Hex,
}

pub(crate) fn path_label<'a>(
    ui: &mut egui::Ui,
    path: &'a Path,
    display_variant: &mut DisplayVariant,
) -> egui::Response {
    let mut iter = path.iter();
    if let Some(key) = iter.next_back() {
        let mut text = String::from("[");
        if let Some(parent) = iter.next_back() {
            if iter.next_back().is_some() {
                text.push_str("..., ");
            }
            text.push_str(&bytes_by_display_variant(parent, display_variant));
            text.push_str(", ");
        }

        text.push_str(&bytes_by_display_variant(key, display_variant));
        text.push_str("]");

        let response = display_variant_dropdown(ui, &text, display_variant, Color32::LIGHT_GRAY);

        response.on_hover_ui_at_pointer(|hover_ui| {
            let mut text = String::from("[");
            let mut iter = path.iter();
            let last = iter.next_back();
            iter.for_each(|segment| {
                text.push_str(&bytes_by_display_variant(segment, display_variant));
                text.push_str(", ");
            });
            last.into_iter().for_each(|segment| {
                text.push_str(&bytes_by_display_variant(segment, display_variant));
                text.push_str("]");
            });
            hover_ui.label(text);
        })
    } else {
        ui.label("Root subtree")
    }
}
