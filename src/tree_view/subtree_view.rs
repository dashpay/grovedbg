use std::collections::BTreeMap;

use eframe::egui::{self, Align2, Color32, Pos2, Stroke};
use grovedbg_types::{Key, PathQuery, Query, QueryItem, SizedQuery, SubqueryBranch};

use super::{element_view::ElementView, SubtreeViewContext, NODE_WIDTH};
use crate::{
    path_ctx::{path_label, Path},
    protocol::Command,
    theme::subtree_line_color,
    tree_data::{SubtreeData, TreeData},
    CommandsSender,
};

const KV_PER_PAGE: usize = 10;
const NODE_MARGIN_HORIZONTAL: f32 = 50.;
const NODE_MARGIN_VERTICAL: f32 = 400.;

pub(crate) type SubtreeElements = BTreeMap<Key, ElementView>;

pub(crate) struct SubtreeView<'pa> {
    pub(super) path: Path<'pa>,
    commands_sender: CommandsSender,
    page_index: usize,
    width: usize,
}

impl<'pa> SubtreeView<'pa> {
    pub(crate) fn new(commands_sender: CommandsSender, path: Path<'pa>) -> Self {
        Self {
            path,
            commands_sender,
            page_index: 0,
            width: 1,
        }
    }

    pub(super) fn scroll_to(&mut self, key: &[u8], tree_data: &mut TreeData<'pa>) {
        let subtree_data = tree_data.get(self.path);
        let index = subtree_data
            .elements
            .iter()
            .enumerate()
            .find_map(|(i, (k, _))| (k.as_slice() == key).then_some(i))
            .unwrap_or_default();

        self.page_index = index / KV_PER_PAGE;
    }

    fn fetch(&self, limit: Option<u16>) {
        let _ = self
            .commands_sender
            .blocking_send(Command::FetchWithPathQuery {
                path_query: PathQuery {
                    path: self.path.to_vec(),
                    query: SizedQuery {
                        query: Query {
                            items: vec![QueryItem::RangeFull],
                            default_subquery_branch: SubqueryBranch {
                                subquery_path: None,
                                subquery: None,
                            },
                            conditional_subquery_branches: Vec::new(),
                            left_to_right: true,
                        },
                        limit,
                        offset: None,
                    },
                },
            })
            .inspect_err(|_| log::error!("Unable to reach GroveDBG protocol thread"));
    }

    fn fetch_n(&self, n: u16) {
        self.fetch(Some(n))
    }

    fn fetch_all(&self) {
        self.fetch(None)
    }

    fn fetch_key(&self, key: Vec<u8>) {
        let _ = self
            .commands_sender
            .blocking_send(Command::FetchNode {
                path: self.path.to_vec(),
                key,
            })
            .inspect_err(|_| log::error!("Unable to reach GroveDBG protocol thread"));
    }

    fn next_page(&mut self, ctx: &mut SubtreeViewContext) {
        ctx.drop_focus();
        self.page_index += 1;
    }

    fn prev_page(&mut self, ctx: &mut SubtreeViewContext) {
        ctx.drop_focus();
        self.page_index = self.page_index.saturating_sub(1);
    }

    /// Draw subtree control buttons
    fn draw_controls(&mut self, ui: &mut egui::Ui, tree_data: &mut TreeData<'pa>) {
        ui.horizontal(|controls_ui| {
            let subtree_data = tree_data.get(self.path);
            let root_key = subtree_data.root_key.clone();

            if controls_ui.button("10").on_hover_text("Fetch 10 items").clicked() {
                self.fetch_n(10);
            }

            if controls_ui
                .button("100")
                .on_hover_text("Fetch 100 items")
                .clicked()
            {
                self.fetch_n(100);
            }

            if controls_ui
                .button(egui_phosphor::regular::DATABASE)
                .on_hover_text("Fetch whole subtree")
                .clicked()
            {
                self.fetch_all();
            }

            if let Some(key) = subtree_data.root_key.as_ref() {
                if controls_ui
                    .button(egui_phosphor::regular::ANCHOR)
                    .on_hover_text("Fetch root node data")
                    .clicked()
                {
                    self.fetch_key(key.clone());
                }
            }

            if !subtree_data.elements.is_empty() {
                if controls_ui
                    .button(egui_phosphor::regular::BROOM)
                    .on_hover_text("Clear subtree data")
                    .clicked()
                {
                    subtree_data.elements.clear();
                }
            }

            if controls_ui
                .button(egui_phosphor::regular::LIST_MAGNIFYING_GLASS)
                .on_hover_text("Select this subtree for a path query")
                .clicked()
            {
                self.path.select_for_query();
            }

            if let Some(key) = root_key {
                if controls_ui
                    .button(egui_phosphor::regular::TREE_STRUCTURE)
                    .on_hover_text("Select subtree for Merk view")
                    .clicked()
                {
                    tree_data.select_for_merk(self.path);
                    let _ = self
                        .commands_sender
                        .blocking_send(Command::FetchNode {
                            path: self.path.to_vec(),
                            key: key.clone(),
                        })
                        .inspect_err(|_| log::error!("Unable to reach GroveDBG protocol thread"));
                }
            }
        });
    }

    /// Draw elements of the subtree as a list
    fn draw_elements<'af, 'pf>(
        &mut self,
        ui: &mut egui::Ui,
        subtree_view_ctx: &mut SubtreeViewContext<'af, 'pf, 'pa>,
        subtree_data: &mut SubtreeData,
    ) {
        let mut element_view_ctx = subtree_view_ctx.element_view_context(self.path);

        for (_, element) in subtree_data
            .elements
            .iter_mut()
            .skip(self.page_index * KV_PER_PAGE)
            .take(KV_PER_PAGE)
        {
            element.draw(ui, &mut element_view_ctx);

            ui.separator();
        }
    }

    /// Draw pagination buttons
    fn draw_pagination(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut SubtreeViewContext,
        subtree_data: &mut SubtreeData,
    ) {
        if subtree_data.elements.len() > KV_PER_PAGE {
            ui.horizontal(|pagination| {
                if pagination
                    .add_enabled(self.page_index > 0, egui::Button::new("⬅"))
                    .clicked()
                {
                    self.prev_page(ctx);
                }
                if pagination
                    .add_enabled(
                        (self.page_index + 1) * KV_PER_PAGE < subtree_data.elements.len(),
                        egui::Button::new("➡"),
                    )
                    .clicked()
                {
                    self.next_page(ctx);
                }
            });
        }
    }

    /// Draw a line to the parent if any
    fn draw_parent_connection(&self, ui: &mut egui::Ui, coords: Pos2) {
        if let Some(parent_path) = self.path.parent() {
            if let Some(parent_pos) =
                ui.memory(|mem| mem.area_rect(parent_path.id()).map(|rect| rect.center_bottom()))
            {
                let painter = ui.painter();
                painter.line_segment(
                    [parent_pos, coords + (NODE_WIDTH / 2., 0.).into()],
                    Stroke {
                        width: 1.0,
                        color: subtree_line_color(ui.ctx()),
                    },
                );
            }
        }
    }

    /// Draw a subtree list view
    pub(crate) fn draw<'af, 'pf>(
        &mut self,
        mut subtree_view_ctx: SubtreeViewContext<'af, 'pf, 'pa>,
        ui: &mut egui::Ui,
        tree_data: &mut TreeData<'pa>,
        subtrees: &mut BTreeMap<Path<'pa>, SubtreeView<'pa>>,
        coords: Option<Pos2>,
        merk_panel_width: f32,
    ) {
        let mut area_builder = egui::Area::new(self.path.id());
        area_builder = if let Some(coords) = coords {
            area_builder.fixed_pos(coords)
        } else {
            area_builder.anchor(Align2::CENTER_CENTER, (merk_panel_width, 0.))
        };

        let area_id = area_builder
            .constrain(false)
            .show(ui.ctx(), |area| {
                area.set_clip_rect(subtree_view_ctx.transform.inverse() * subtree_view_ctx.rect);

                egui::Frame::default()
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(Stroke {
                        width: 1.0,
                        color: Color32::DARK_GRAY,
                    })
                    .show(area, |subtree_ui| {
                        subtree_ui.set_max_width(NODE_WIDTH);
                        self.draw_controls(subtree_ui, tree_data);
                        subtree_ui.separator();

                        path_label(subtree_ui, self.path, &subtree_view_ctx.profile_ctx);
                        subtree_ui.separator();

                        let subtree_data = tree_data.get(self.path);

                        self.draw_elements(subtree_ui, &mut subtree_view_ctx, subtree_data);

                        self.draw_pagination(subtree_ui, &mut subtree_view_ctx, subtree_data);

                        if let Some(self_pos) = coords {
                            self.draw_parent_connection(subtree_ui, self_pos);
                        }
                    })
            })
            .response
            .layer_id;

        ui.ctx().set_transform_layer(area_id, subtree_view_ctx.transform);

        if let Some(bottom_pos) =
            ui.memory(|mem| mem.area_rect(self.path.id()).map(|rect| rect.center_bottom()))
        {
            let subtree_data = tree_data.get(self.path);
            let visible_subtrees_width = subtree_data
                .subtree_keys
                .iter()
                .cloned()
                .filter_map(|k| {
                    let path = self.path.child(k.clone());
                    path.visible().then(|| {
                        subtrees
                            .entry(path)
                            .or_insert_with(|| {
                                SubtreeView::new(self.commands_sender.clone(), self.path.child(k.clone()))
                            })
                            .width
                    })
                })
                .sum();

            let width: usize = std::cmp::max(visible_subtrees_width, 1);
            self.width = width;
            let width_f = width_to_egui(width);

            let mut current_x = bottom_pos.x - width_f / 2. - NODE_WIDTH / 2.;
            let y = bottom_pos.y + NODE_MARGIN_VERTICAL;

            for subtree_key in subtree_data.subtree_keys.clone() {
                let path = self.path.child(subtree_key.clone());

                let Some(mut subtree) = path.visible().then(|| subtrees.remove(&path)).flatten() else {
                    continue;
                };
                let subtree_width = width_to_egui(subtree.width);
                current_x += subtree_width / 2.;
                subtree.draw(
                    subtree_view_ctx.child(subtree_key.clone()),
                    ui,
                    tree_data,
                    subtrees,
                    Some((current_x, y).into()),
                    merk_panel_width,
                );
                subtrees.insert(path, subtree);
                current_x += subtree_width / 2. + NODE_MARGIN_HORIZONTAL;
            }
        }
    }
}

fn width_to_egui(width: usize) -> f32 {
    if width > 0 {
        width as f32 * NODE_WIDTH + (width - 1) as f32 * NODE_MARGIN_HORIZONTAL
    } else {
        0.
    }
}