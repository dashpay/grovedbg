use eframe::egui::{
    self, Align, CollapsingHeader, Color32, Frame, Layout, Margin, RadioButton, RichText, TextEdit, Vec2,
};
use grovedbg_types::QueryItem;
use integer_encoding::VarInt;

use super::{common::path_label, DisplayVariant};
use crate::model::path_display::PathCtx;

const MARGIN: f32 = 20.;

pub(crate) struct QueryBuilder<'p> {
    path_ctx: &'p PathCtx,
    limit: Option<u16>,
    limit_input: String,
    offset: Option<u16>,
    offset_input: String,
    query: QueryInput,
}

impl<'p> QueryBuilder<'p> {
    pub fn new(path_ctx: &'p PathCtx) -> Self {
        QueryBuilder {
            path_ctx,
            limit: None,
            offset: None,
            limit_input: String::new(),
            offset_input: String::new(),
            query: QueryInput::new(0),
        }
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) {
        if let Some(path) = self.path_ctx.get_selected_for_query() {
            path_label(ui, path);
            opt_number_input(ui, "Limit: ", &mut self.limit_input, &mut self.limit);
            opt_number_input(ui, "Offset: ", &mut self.offset_input, &mut self.offset);
            self.query.draw(ui);
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
    label: String,
    err: bool,
}

impl BytesInput {
    fn new(label: String) -> Self {
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
            let label = line.label(RichText::new(&self.label).color(if self.err {
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
    subquery_idx: usize,
    item_idx: usize,
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
    fn new(subquery_idx: usize, item_idx: usize) -> Self {
        Self {
            value: None,
            input_type: QueryInputType::Key(BytesInput::new("Key".to_owned())),
            subquery_idx,
            item_idx,
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("Query item type")
            .id_source(self.subquery_idx * 1000 + self.item_idx)
            .show(ui, |collapsing| {
                if collapsing
                    .add(RadioButton::new(
                        matches!(self.input_type, QueryInputType::Key(..)),
                        "Key",
                    ))
                    .clicked()
                {
                    self.input_type = QueryInputType::Key(BytesInput::new("Key".to_owned()))
                }

                if collapsing
                    .add(RadioButton::new(
                        matches!(self.input_type, QueryInputType::Range { .. }),
                        "Range",
                    ))
                    .clicked()
                {
                    self.input_type = QueryInputType::Range {
                        start: BytesInput::new("Start".to_owned()),
                        end: BytesInput::new("End".to_owned()),
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
                        start: BytesInput::new("Start".to_owned()),
                        end: BytesInput::new("End".to_owned()),
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
                    self.input_type = QueryInputType::RangeFrom(BytesInput::new("From".to_owned()))
                }

                if collapsing
                    .add(RadioButton::new(
                        matches!(self.input_type, QueryInputType::RangeTo(..)),
                        "RangeTo",
                    ))
                    .clicked()
                {
                    self.input_type = QueryInputType::RangeTo(BytesInput::new("To".to_owned()))
                }

                if collapsing
                    .add(RadioButton::new(
                        matches!(self.input_type, QueryInputType::RangeToInclusive(..)),
                        "RangeToInclusive",
                    ))
                    .clicked()
                {
                    self.input_type = QueryInputType::RangeToInclusive(BytesInput::new("To".to_owned()))
                }

                if collapsing
                    .add(RadioButton::new(
                        matches!(self.input_type, QueryInputType::RangeAfter(..)),
                        "RangeAfter",
                    ))
                    .clicked()
                {
                    self.input_type = QueryInputType::RangeAfter(BytesInput::new("After".to_owned()))
                }

                if collapsing
                    .add(RadioButton::new(
                        matches!(self.input_type, QueryInputType::RangeAfterTo { .. }),
                        "RangeAfterTo",
                    ))
                    .clicked()
                {
                    self.input_type = QueryInputType::RangeAfterTo {
                        after: BytesInput::new("After".to_owned()),
                        to: BytesInput::new("To".to_owned()),
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
                        after: BytesInput::new("After".to_owned()),
                        to: BytesInput::new("To".to_owned()),
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

struct QueryInput {
    items: Vec<QueryItemInput>,
    default_subquery_branch: Option<SubqueryBranchInput>,
    conditional_subquery_branches: Vec<ConditionalSubqueryBranchInput>,
    left_to_right: bool,
    subquery_idx: usize,
}

impl QueryInput {
    fn new(subquery_idx: usize) -> Self {
        Self {
            items: Vec::new(),
            default_subquery_branch: None,
            conditional_subquery_branches: Vec::new(),
            left_to_right: true,
            subquery_idx,
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.left_to_right, "Left to right");
        ui.horizontal(|line| {
            line.label("Query items");
            if line.button("+").clicked() {
                self.items
                    .push(QueryItemInput::new(self.subquery_idx, self.items.len()));
            }
        });
        for item in self.items.iter_mut() {
            item.draw(ui);
        }

        let mut subquery_checked = self.default_subquery_branch.is_some();
        ui.checkbox(&mut subquery_checked, "Default subquery");
        if !subquery_checked {
            self.default_subquery_branch = None;
        } else if self.default_subquery_branch.is_none() {
            self.default_subquery_branch = Some(SubqueryBranchInput::new(self.subquery_idx + 1));
        }
        if let Some(subquery) = self.default_subquery_branch.as_mut() {
            Frame::none()
                .outer_margin(Margin {
                    left: MARGIN,
                    ..Default::default()
                })
                .show(ui, |subquery_frame| {
                    subquery.draw(subquery_frame);
                });
        }

        ui.horizontal(|line| {
            line.label("Subquery branches");
            if line.button("+").clicked() {
                self.conditional_subquery_branches
                    .push(ConditionalSubqueryBranchInput::new(
                        self.subquery_idx
                            + self
                                .default_subquery_branch
                                .as_ref()
                                .map(|_| 1)
                                .unwrap_or_default()
                            + self.conditional_subquery_branches.len(),
                    ));
            }
        });

        Frame::none()
            .outer_margin(Margin {
                left: MARGIN,
                ..Default::default()
            })
            .show(ui, |subquery_branches_frame| {
                for branch in self.conditional_subquery_branches.iter_mut() {
                    branch.draw(subquery_branches_frame);
                }
            });
    }
}

struct SubqueryBranchInput {
    relative_path: PathInput,
    subquery: Box<QueryInput>,
}

impl SubqueryBranchInput {
    fn new(subquery_idx: usize) -> Self {
        Self {
            relative_path: PathInput::new(),
            subquery: Box::new(QueryInput::new(subquery_idx)),
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|layout| {
            self.relative_path.draw(layout);
            self.subquery.draw(layout);
        });
    }
}

struct ConditionalSubqueryBranchInput {
    query_item: QueryItemInput,
    subquery_branch: SubqueryBranchInput,
}

impl ConditionalSubqueryBranchInput {
    fn new(subquery_idx: usize) -> Self {
        Self {
            query_item: QueryItemInput::new(subquery_idx * 10, 0),
            subquery_branch: SubqueryBranchInput::new(subquery_idx * 100),
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.label("Condition:");
        self.query_item.draw(ui);
        ui.label("Conditional subquery:");
        self.subquery_branch.draw(ui);
    }
}

struct PathInput {
    path: Vec<BytesInput>,
}

impl PathInput {
    fn new() -> Self {
        Self { path: Vec::new() }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|line| {
            line.label("Path");
            if line.button("+").clicked() {
                self.path.push(BytesInput::new(self.path.len().to_string()));
            }
        });
        for segment in self.path.iter_mut() {
            segment.draw(ui);
        }
    }
}
