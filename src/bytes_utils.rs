use std::fmt::Write;

use eframe::egui::{self, Color32, Label, RichText, Sense};
use integer_encoding::VarInt;

const MAX_BYTES: usize = 10;
const MAX_HEX_LENGTH: usize = 32;
const HEX_PARTS_LENGTH: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BytesDisplayVariant {
    U8,
    String,
    Hex,
    SignedInt,
    UnsignedInt,
    VarInt,
}

impl BytesDisplayVariant {
    pub(crate) fn guess(bytes: &[u8]) -> Self {
        match bytes.len() {
            1 => Self::U8,
            2 | 4 | 8 => Self::SignedInt,
            32 => Self::Hex,
            _ => Self::String,
        }
    }
}

pub(crate) enum BytesInputVariant {
    U8,
    String,
    Hex,
    VarInt,
    I16,
    I32,
    I64,
    U16,
    U32,
    U64,
}

pub(crate) struct BytesView {
    pub(crate) bytes: Vec<u8>,
    display_variant: BytesDisplayVariant,
}

impl BytesView {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Self {
            display_variant: BytesDisplayVariant::guess(&bytes),
            bytes,
        }
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui) {
        binary_label(ui, &self.bytes, &mut self.display_variant);
    }
}

/// Represent binary data different ways and to choose from
pub(crate) fn binary_label_colored<'a>(
    ui: &mut egui::Ui,
    bytes: &[u8],
    display_variant: &mut BytesDisplayVariant,
    color: Color32,
) -> egui::Response {
    display_variant_dropdown(ui, bytes, display_variant, color)
}

fn display_variant_dropdown<'a>(
    ui: &mut egui::Ui,
    bytes: &[u8],
    display_variant: &mut BytesDisplayVariant,
    color: Color32,
) -> egui::Response {
    let text = bytes_by_display_variant(bytes, display_variant);
    let response = ui
        .add(
            Label::new(RichText::new(text).color(color))
                .truncate()
                .sense(Sense::click()),
        )
        .on_hover_ui(|hover| {
            hover.label(bytes_by_display_variant_explicit(bytes, display_variant));
        });

    response.context_menu(|menu| {
        menu.radio_value(display_variant, BytesDisplayVariant::U8, "u8 array");
        menu.radio_value(display_variant, BytesDisplayVariant::String, "UTF-8 String");
        menu.radio_value(display_variant, BytesDisplayVariant::Hex, "Hex String");
        menu.radio_value(display_variant, BytesDisplayVariant::SignedInt, "Signed Int");
        menu.radio_value(display_variant, BytesDisplayVariant::UnsignedInt, "Unsigned Int");
        menu.radio_value(display_variant, BytesDisplayVariant::VarInt, "VarInt");
    });
    response
}

pub(crate) fn binary_label<'a>(
    ui: &mut egui::Ui,
    bytes: &[u8],
    display_variant: &mut BytesDisplayVariant,
) -> egui::Response {
    binary_label_colored(ui, bytes, display_variant, Color32::GRAY)
}

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

pub(crate) fn bytes_as_signed_int(bytes: &[u8]) -> String {
    match bytes.len() {
        2 => TryInto::<[u8; 2]>::try_into(bytes)
            .map(|arr| format!("i16: {}", i16::from_be_bytes(arr)))
            .expect("len is 2"),
        4 => TryInto::<[u8; 4]>::try_into(bytes)
            .map(|arr| format!("i32: {}", i32::from_be_bytes(arr)))
            .expect("len is 4"),
        8 => TryInto::<[u8; 8]>::try_into(bytes)
            .map(|arr| format!("i64: {}", i64::from_be_bytes(arr)))
            .expect("len is 8"),
        _ => String::from("[E]: must be 2/4/8 bytes"),
    }
}

pub(crate) fn bytes_as_unsigned_int(bytes: &[u8]) -> String {
    match bytes.len() {
        2 => TryInto::<[u8; 2]>::try_into(bytes)
            .map(|arr| format!("u16: {}", u16::from_be_bytes(arr)))
            .expect("len is 2"),
        4 => TryInto::<[u8; 4]>::try_into(bytes)
            .map(|arr| format!("u32: {}", u32::from_be_bytes(arr)))
            .expect("len is 4"),
        8 => TryInto::<[u8; 8]>::try_into(bytes)
            .map(|arr| format!("u64: {}", u64::from_be_bytes(arr)))
            .expect("len is 8"),
        _ => String::from("[E]: must be 2/4/8 bytes"),
    }
}

fn bytes_as_varint(bytes: &[u8]) -> String {
    i64::decode_var(bytes)
        .map(|(x, _)| x.to_string())
        .unwrap_or_else(|| "varint: MSB".to_owned())
}

pub(crate) fn bytes_by_display_variant(bytes: &[u8], display_variant: &BytesDisplayVariant) -> String {
    if bytes.is_empty() {
        "empty".to_owned()
    } else {
        match display_variant {
            BytesDisplayVariant::U8 => bytes_as_slice(bytes),
            BytesDisplayVariant::String => format!("str: {}", String::from_utf8_lossy(bytes).to_string()),
            BytesDisplayVariant::Hex => format!("hex: {}", bytes_as_hex(bytes)),
            BytesDisplayVariant::SignedInt => bytes_as_signed_int(bytes),
            BytesDisplayVariant::UnsignedInt => bytes_as_unsigned_int(bytes),
            BytesDisplayVariant::VarInt => format!("varint: {}", bytes_as_varint(bytes)),
        }
    }
}

pub(crate) fn bytes_by_display_variant_explicit(
    bytes: &[u8],
    display_variant: &BytesDisplayVariant,
) -> String {
    match display_variant {
        BytesDisplayVariant::U8 => format!("{bytes:?}"),
        BytesDisplayVariant::String => String::from_utf8_lossy(bytes).to_string(),
        BytesDisplayVariant::Hex => hex::encode(bytes),
        BytesDisplayVariant::SignedInt => bytes_as_signed_int(bytes),
        BytesDisplayVariant::UnsignedInt => bytes_as_unsigned_int(bytes),
        BytesDisplayVariant::VarInt => bytes_as_varint(bytes),
    }
}
