use eframe::egui::{self, Color32, RadioButton, RichText, TextEdit};
use grovedbg_types::QueryItem;
use integer_encoding::VarInt;

use super::{common::path_label, DisplayVariant};
use crate::model::path_display::PathCtx;

pub(crate) struct QueryBuilder<'p> {
    path_ctx: &'p PathCtx,
    limit: Option<u16>,
    limit_input: String,
    offset: Option<u16>,
    offset_input: String,
    test: QueryItemInput,
}

impl<'p> QueryBuilder<'p> {
    pub fn new(path_ctx: &'p PathCtx) -> Self {
        QueryBuilder {
            path_ctx,
            limit: None,
            offset: None,
            limit_input: String::new(),
            offset_input: String::new(),
            test: QueryItemInput::new(),
        }
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) {
        if let Some(path) = self.path_ctx.get_selected_for_query() {
            path_label(ui, path);
            opt_number_input(ui, "Limit: ", &mut self.limit_input, &mut self.limit);
            opt_number_input(ui, "Offset: ", &mut self.offset_input, &mut self.offset);
            self.test.draw(ui);
        } else {
            ui.label("No query path selected, click on a subtree header with path first");
        }
    }
}

fn opt_number_input(ui: &mut egui::Ui, hint: &'static str, input: &mut String, value: &mut Option<u16>) {
    ui.horizontal(|line| {
        let label = line.label(hint);
        if line
            .text_edit_singleline(input)
            .labelled_by(label.id)
            .lost_focus()
        {
            if let Ok(input) = input.parse() {
                *value = Some(input);
            } else {
                *value = None;
            }
        }
    });
}

struct BytesInput {
    bytes: Vec<u8>,
    input: String,
    display_variant: DisplayVariant,
    label: &'static str,
    err: bool,
}

impl BytesInput {
    fn new(label: &'static str) -> Self {
        Self {
            bytes: Vec::new(),
            input: String::new(),
            display_variant: DisplayVariant::Hex,
            label,
            err: false,
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|line| {
            let label = line.label(RichText::new(self.label).color(if self.err {
                Color32::RED
            } else {
                Color32::PLACEHOLDER
            }));

            let response = line.text_edit_singleline(&mut self.input).labelled_by(label.id);

            response.context_menu(|menu| {
                menu.radio_value(&mut self.display_variant, DisplayVariant::U8, "u8 array");
                menu.radio_value(&mut self.display_variant, DisplayVariant::String, "UTF-8 String");
                menu.radio_value(&mut self.display_variant, DisplayVariant::Hex, "Hex String");
                menu.radio_value(&mut self.display_variant, DisplayVariant::Int, "i64");
                menu.radio_value(&mut self.display_variant, DisplayVariant::VarInt, "VarInt");
            });

            if response.lost_focus() {
                self.err = false;
                self.bytes = match self.display_variant {
                    DisplayVariant::U8 => self
                        .input
                        .split_whitespace()
                        .map(|int| int.parse::<u8>())
                        .collect::<Result<Vec<u8>, _>>()
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    DisplayVariant::String => self.input.as_bytes().to_vec(),
                    DisplayVariant::Hex => hex::decode(&self.input)
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    DisplayVariant::Int => self
                        .input
                        .parse::<i64>()
                        .map(|int| int.to_be_bytes().to_vec())
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    DisplayVariant::VarInt => self
                        .input
                        .parse::<i64>()
                        .map(|int| int.encode_var_vec())
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                }
            }
        });
    }
}

struct QueryItemInput {
    value: Option<QueryItem>,
    input_type: QueryInputType,
}

enum QueryInputType {
    Key(BytesInput),
    Range { start: BytesInput, end: BytesInput },
    RangeInclusive { start: BytesInput, end: BytesInput },
    RangeFull,
    RangeFrom(BytesInput),
    RangeTo(BytesInput),
    RangeToInclusive(BytesInput),
    RangeAfter(BytesInput),
    RangeAfterTo { after: BytesInput, to: BytesInput },
    RangeAfterToInclusive { after: BytesInput, to: BytesInput },
}

impl QueryItemInput {
    fn new() -> Self {
        Self {
            value: None,
            input_type: QueryInputType::Key(BytesInput::new("Key")),
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Query item type", |collapsing| {
            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::Key(..)),
                    "Key",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::Key(BytesInput::new("Key"))
            }

            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::Range { .. }),
                    "Range",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::Range {
                    start: BytesInput::new("Start"),
                    end: BytesInput::new("End"),
                };
            }

            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::RangeInclusive { .. }),
                    "RangeInclusive",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::RangeInclusive {
                    start: BytesInput::new("Start"),
                    end: BytesInput::new("End"),
                };
            }

            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::RangeFull),
                    "RangeFull",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::RangeFull
            }

            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::RangeFrom(..)),
                    "RangeFrom",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::RangeFrom(BytesInput::new("From"))
            }

            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::RangeTo(..)),
                    "RangeTo",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::RangeTo(BytesInput::new("To"))
            }

            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::RangeToInclusive(..)),
                    "RangeToInclusive",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::RangeToInclusive(BytesInput::new("To"))
            }

            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::RangeAfter(..)),
                    "RangeAfter",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::RangeAfter(BytesInput::new("After"))
            }

            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::RangeAfterTo { .. }),
                    "RangeAfterTo",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::RangeAfterTo {
                    after: BytesInput::new("After"),
                    to: BytesInput::new("To"),
                };
            }

            if collapsing
                .add(RadioButton::new(
                    matches!(self.input_type, QueryInputType::RangeAfterToInclusive { .. }),
                    "RangeAfterToInclusive",
                ))
                .clicked()
            {
                self.input_type = QueryInputType::RangeAfterToInclusive {
                    after: BytesInput::new("After"),
                    to: BytesInput::new("To"),
                };
            }
        });

        match &mut self.input_type {
            QueryInputType::Key(input) => input.draw(ui),
            QueryInputType::Range { start, end } => {
                start.draw(ui);
                end.draw(ui);
            }
            QueryInputType::RangeInclusive { start, end } => {
                start.draw(ui);
                end.draw(ui);
            }
            QueryInputType::RangeFull => {
                ui.label("Full range");
            }
            QueryInputType::RangeFrom(input) => input.draw(ui),
            QueryInputType::RangeTo(input) => input.draw(ui),
            QueryInputType::RangeToInclusive(input) => input.draw(ui),
            QueryInputType::RangeAfter(input) => input.draw(ui),
            QueryInputType::RangeAfterTo { after, to } => {
                after.draw(ui);
                to.draw(ui);
            }
            QueryInputType::RangeAfterToInclusive { after, to } => {
                after.draw(ui);
                to.draw(ui);
            }
        }
    }
}
