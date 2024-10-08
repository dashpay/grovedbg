use eframe::egui::{self, CollapsingHeader, Color32, Frame, Margin, RadioButton, RichText};
use grovedbg_types::{PathQuery, Query, QueryItem, SubqueryBranch};
use integer_encoding::VarInt;
use strum::IntoEnumIterator;

use crate::{
    bus::CommandBus,
    bytes_utils::BytesInputVariant,
    path_ctx::{path_label, Path, PathCtx},
    profiles::RootActiveProfileContext,
    protocol::FetchCommand,
};

const MARGIN: f32 = 20.;

pub(crate) struct QueryBuilder {
    limit_input: OptionalNumberInput,
    offset_input: OptionalNumberInput,
    query: QueryInput,
}

impl QueryBuilder {
    pub fn new() -> Self {
        QueryBuilder {
            limit_input: OptionalNumberInput::new("Limit".to_owned()),
            offset_input: OptionalNumberInput::new("Offset".to_owned()),
            query: QueryInput::new(0),
        }
    }

    pub fn draw<'pf>(
        &mut self,
        ui: &mut egui::Ui,
        path_ctx: &PathCtx,
        profile_ctx: RootActiveProfileContext<'pf>,
        bus: &CommandBus,
    ) {
        if let Some(path) = path_ctx.get_selected_for_query() {
            let profile_ctx = profile_ctx.fast_forward(path);
            path_label(ui, path, &profile_ctx);
            self.limit_input.draw(ui);
            self.offset_input.draw(ui);
            self.query.draw(ui);

            ui.horizontal(|line| {
                if line.button("Prove").clicked() {
                    self.prove_query(&path, bus);
                }
                if line.button("Fetch").clicked() {
                    self.fetch_query(&path, bus);
                }
            });
        } else {
            ui.label("No query path selected, click on a subtree header with path first");
        }
    }

    fn prove_query(&self, path: &Path, bus: &CommandBus) {
        let path_query = PathQuery {
            path: path.to_vec(),
            query: grovedbg_types::SizedQuery {
                query: self.query.get_query(),
                limit: self.limit_input.number,
                offset: self.offset_input.number,
            },
        };

        bus.fetch_command(FetchCommand::ProvePathQuery { path_query });
    }

    fn fetch_query(&self, path: &Path, bus: &CommandBus) {
        let path_query = PathQuery {
            path: path.to_vec(),
            query: grovedbg_types::SizedQuery {
                query: self.query.get_query(),
                limit: self.limit_input.number,
                offset: self.offset_input.number,
            },
        };

        bus.fetch_command(FetchCommand::FetchWithPathQuery { path_query });
    }
}

struct OptionalNumberInput {
    number: Option<u16>,
    input: String,
    label: String,
    err: bool,
}

impl OptionalNumberInput {
    fn new(label: String) -> Self {
        Self {
            number: None,
            input: String::new(),
            err: false,
            label,
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|line| {
            let label = line.label(RichText::new(&self.label).color(if self.err {
                Color32::RED
            } else {
                Color32::PLACEHOLDER
            }));

            if line
                .text_edit_singleline(&mut self.input)
                .labelled_by(label.id)
                .lost_focus()
            {
                if let Ok(x) = self.input.parse() {
                    self.number = Some(x);
                    self.err = false;
                } else {
                    self.err = !self.input.is_empty();
                    self.number = None
                }
            }
        });
    }
}

struct BytesInput {
    bytes: Vec<u8>,
    input: String,
    display_variant: BytesInputVariant,
    label: String,
    err: bool,
}

impl BytesInput {
    fn new(label: String) -> Self {
        Self {
            bytes: Vec::new(),
            input: String::new(),
            display_variant: BytesInputVariant::Hex,
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
                for variant in BytesInputVariant::iter() {
                    menu.radio_value(&mut self.display_variant, variant, variant.as_ref());
                }
            });

            if response.lost_focus() {
                self.err = false;
                self.bytes = match self.display_variant {
                    BytesInputVariant::U8 => self
                        .input
                        .split_whitespace()
                        .map(|int| int.parse::<u8>())
                        .collect::<Result<Vec<u8>, _>>()
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    BytesInputVariant::String => self.input.as_bytes().to_vec(),
                    BytesInputVariant::Hex => hex::decode(&self.input)
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    BytesInputVariant::VarInt => self
                        .input
                        .parse::<i64>()
                        .map(|int| int.encode_var_vec())
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    BytesInputVariant::I16 => self
                        .input
                        .parse::<i16>()
                        .map(|int| int.to_be_bytes().to_vec())
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    BytesInputVariant::I32 => self
                        .input
                        .parse::<i32>()
                        .map(|int| int.to_be_bytes().to_vec())
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    BytesInputVariant::I64 => self
                        .input
                        .parse::<i64>()
                        .map(|int| int.to_be_bytes().to_vec())
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    BytesInputVariant::U16 => self
                        .input
                        .parse::<u16>()
                        .map(|int| int.to_be_bytes().to_vec())
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    BytesInputVariant::U32 => self
                        .input
                        .parse::<u32>()
                        .map(|int| int.to_be_bytes().to_vec())
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                    BytesInputVariant::U64 => self
                        .input
                        .parse::<u64>()
                        .map(|int| int.to_be_bytes().to_vec())
                        .inspect_err(|_| self.err = true)
                        .unwrap_or_default(),
                }
            }
        });
    }
}

struct QueryItemInput {
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
            input_type: QueryInputType::Key(BytesInput::new("Key".to_owned())),
            subquery_idx,
            item_idx,
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("Query item type")
            .id_salt(self.subquery_idx * 1000 + self.item_idx)
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

    fn get_query_item(&self) -> QueryItem {
        match &self.input_type {
            QueryInputType::Key(input) => QueryItem::Key(input.bytes.clone()),
            QueryInputType::Range { start, end } => QueryItem::Range {
                start: start.bytes.clone(),
                end: end.bytes.clone(),
            },
            QueryInputType::RangeInclusive { start, end } => QueryItem::RangeInclusive {
                start: start.bytes.clone(),
                end: end.bytes.clone(),
            },
            QueryInputType::RangeFull => QueryItem::RangeFull,
            QueryInputType::RangeFrom(input) => QueryItem::RangeFrom(input.bytes.clone()),
            QueryInputType::RangeTo(input) => QueryItem::RangeTo(input.bytes.clone()),
            QueryInputType::RangeToInclusive(input) => QueryItem::RangeToInclusive(input.bytes.clone()),
            QueryInputType::RangeAfter(input) => QueryItem::RangeAfter(input.bytes.clone()),
            QueryInputType::RangeAfterTo { after, to } => QueryItem::RangeAfterTo {
                after: after.bytes.clone(),
                to: to.bytes.clone(),
            },
            QueryInputType::RangeAfterToInclusive { after, to } => QueryItem::RangeAfterToInclusive {
                after: after.bytes.clone(),
                to: to.bytes.clone(),
            },
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
            if line.button("-").clicked() {
                self.items.pop();
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
            if line.button("-").clicked() {
                self.conditional_subquery_branches.pop();
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

    fn get_query(&self) -> Query {
        Query {
            items: self.items.iter().map(|item| item.get_query_item()).collect(),
            default_subquery_branch: self
                .default_subquery_branch
                .as_ref()
                .map(|subquery| subquery.get_subquery_branch())
                .unwrap_or_else(|| SubqueryBranch {
                    subquery_path: None,
                    subquery: None,
                }),
            conditional_subquery_branches: self
                .conditional_subquery_branches
                .iter()
                .map(|cond| cond.get_conditional_subquery_pair())
                .collect(),
            left_to_right: self.left_to_right,
        }
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

    fn get_subquery_branch(&self) -> SubqueryBranch {
        SubqueryBranch {
            subquery_path: Some(self.relative_path.get_path()),
            subquery: Some(Box::new(self.subquery.get_query())),
        }
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

    fn get_conditional_subquery_pair(&self) -> (QueryItem, SubqueryBranch) {
        (
            self.query_item.get_query_item(),
            self.subquery_branch.get_subquery_branch(),
        )
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
            if line.button("-").clicked() {
                self.path.pop();
            }
        });
        for segment in self.path.iter_mut() {
            segment.draw(ui);
        }
    }

    fn get_path(&self) -> Vec<Vec<u8>> {
        self.path.iter().map(|segment| segment.bytes.clone()).collect()
    }
}
