use std::{cell::Cell, fmt::Write, hash::Hash};

use eframe::egui::{self, Color32, Label, RichText, Sense, TextEdit};
use integer_encoding::VarInt;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

use crate::theme::input_error_color;

const MAX_BYTES: usize = 10;
const MAX_HEX_LENGTH: usize = 32;
const HEX_PARTS_LENGTH: usize = 12;

#[derive(Debug, AsRefStr, EnumIter, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub(crate) enum BytesDisplayVariant {
    #[default]
    #[strum(serialize = "u8 array")]
    U8,
    #[strum(serialize = "String")]
    String,
    #[strum(serialize = "Hex")]
    Hex,
    #[strum(serialize = "Signed integer")]
    SignedInt,
    #[strum(serialize = "Unigned integer")]
    UnsignedInt,
    #[strum(serialize = "Variable length integer")]
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

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui) {
        for variant in Self::iter() {
            ui.radio_value(self, variant, variant.as_ref());
        }
    }
}

#[derive(Debug, AsRefStr, EnumIter, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum BytesInputVariant {
    #[strum(serialize = "u8 array")]
    U8,
    #[strum(serialize = "String")]
    String,
    #[strum(serialize = "Hex")]
    Hex,
    #[strum(serialize = "Variable length integer")]
    VarInt,
    #[strum(serialize = "I16")]
    I16,
    #[strum(serialize = "I32")]
    I32,
    #[strum(serialize = "I64")]
    I64,
    #[strum(serialize = "U16")]
    U16,
    #[strum(serialize = "U32")]
    U32,
    #[strum(serialize = "U64")]
    U64,
}

impl BytesInputVariant {
    fn draw(&mut self, ui: &mut egui::Ui) {
        for variant in Self::iter() {
            ui.radio_value(self, variant, variant.as_ref());
        }
    }
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

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct BytesInput {
    input: String,
    input_variant: BytesInputVariant,
    #[serde(skip)]
    err: Cell<bool>,
}

impl Hash for BytesInput {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get_bytes().hash(state)
    }
}

impl PartialEq for BytesInput {
    fn eq(&self, other: &Self) -> bool {
        self.get_bytes() == other.get_bytes()
    }
}

impl Eq for BytesInput {}

impl BytesInput {
    pub(crate) fn new() -> Self {
        Self {
            input: String::new(),
            input_variant: BytesInputVariant::U8,
            err: false.into(),
        }
    }

    pub(crate) fn current_input(&self) -> &str {
        &self.input
    }

    pub(crate) fn new_from_bytes(bytes: Vec<u8>) -> Self {
        let mut input = String::new();
        let mut bytes_iter = bytes.iter();
        let last = bytes_iter.next_back();
        for b in bytes_iter {
            write!(&mut input, "{b} ").ok();
        }
        if let Some(b) = last {
            write!(&mut input, "{b}").ok();
        }
        BytesInput {
            input,
            input_variant: BytesInputVariant::U8,
            err: false.into(),
        }
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui) {
        ui.add(
            TextEdit::singleline(&mut self.input)
                .text_color_opt(self.err.get().then_some(input_error_color(ui.ctx()))),
        )
        .context_menu(|menu| self.input_variant.draw(menu));
    }

    pub(crate) fn get_bytes(&self) -> Vec<u8> {
        if self.input.is_empty() {
            self.err.set(false);
            return Vec::new();
        }

        let bytes_opt = match self.input_variant {
            BytesInputVariant::U8 => self
                .input
                .split_whitespace()
                .map(|int| int.parse::<u8>())
                .collect::<Result<Vec<u8>, _>>()
                .ok(),
            BytesInputVariant::String => Some(self.input.as_bytes().to_vec()),
            BytesInputVariant::Hex => hex::decode(&self.input).ok(),
            BytesInputVariant::VarInt => self.input.parse::<i64>().map(|int| int.encode_var_vec()).ok(),
            BytesInputVariant::I16 => self
                .input
                .parse::<i16>()
                .map(|int| int.to_be_bytes().to_vec())
                .ok(),
            BytesInputVariant::I32 => self
                .input
                .parse::<i32>()
                .map(|int| int.to_be_bytes().to_vec())
                .ok(),
            BytesInputVariant::I64 => self
                .input
                .parse::<i64>()
                .map(|int| int.to_be_bytes().to_vec())
                .ok(),
            BytesInputVariant::U16 => self
                .input
                .parse::<u16>()
                .map(|int| int.to_be_bytes().to_vec())
                .ok(),
            BytesInputVariant::U32 => self
                .input
                .parse::<u32>()
                .map(|int| int.to_be_bytes().to_vec())
                .ok(),
            BytesInputVariant::U64 => self
                .input
                .parse::<u64>()
                .map(|int| int.to_be_bytes().to_vec())
                .ok(),
        };

        if bytes_opt.is_none() {
            self.err.set(true);
        } else {
            self.err.set(false);
        }

        bytes_opt.unwrap_or_default()
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
        for variant in BytesDisplayVariant::iter() {
            menu.radio_value(display_variant, variant, variant.as_ref());
        }
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
