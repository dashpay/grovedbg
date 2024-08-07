use std::collections::BTreeMap;

use eframe::egui::{self, Align2, Color32, Order, Pos2, Stroke};
use grovedbg_types::{Element, Key, NodeUpdate, PathQuery, Query, QueryItem, SizedQuery, SubqueryBranch};

use super::{
    element_view::{ElementView, WrappedElement},
    theme::subtree_line_color,
    SubtreeViewContext, TreeViewContext, NODE_WIDTH,
};
use crate::{
    path_ctx::{path_label, Path},
    protocol::Command,
    CommandsSender,
};

const KV_PER_PAGE: usize = 10;
const NODE_MARGIN_HORIZONTAL: f32 = 50.;
const NODE_MARGIN_VERTICAL: f32 = 400.;

pub(crate) struct SubtreeView<'a> {
    path: Path<'a>,
    commands_sender: CommandsSender,
    root_key: Option<Vec<u8>>,
    subtrees_children: BTreeMap<Key, SubtreeView<'a>>,
    elements_children: BTreeMap<Key, ElementView>,
    page_index: usize,
    width: usize,
    pub(super) show: bool,
}

/// `NodeUpdate` wrapper to navigate an update inside of a nested subtree
/// structure
pub(super) struct RoutingNodeUpdate {
    update: NodeUpdate,
    processed_segments: usize,
}

impl RoutingNodeUpdate {
    pub(super) fn new(update: NodeUpdate) -> Self {
        Self {
            update,
            processed_segments: 0,
        }
    }

    fn is_subtree_reached(&self) -> bool {
        self.processed_segments == self.update.path.len()
    }
}

impl<'a> SubtreeView<'a> {
    pub(crate) fn new(commands_sender: CommandsSender, path: Path<'a>, root_key: Option<Vec<u8>>) -> Self {
        Self {
            path,
            commands_sender,
            root_key,
            subtrees_children: BTreeMap::new(),
            elements_children: BTreeMap::new(),
            page_index: 0,
            width: 1,
            show: false,
        }
    }

    pub(crate) fn set_root(&mut self, root_key: Vec<u8>) {
        self.root_key = Some(root_key);
    }

    pub(crate) fn apply_node_update(&mut self, mut node_update: RoutingNodeUpdate) {
        if node_update.is_subtree_reached() {
            let update = node_update.update;

            if let Element::Subtree { root_key, .. } | Element::Sumtree { root_key, .. } = &update.element {
                self.subtrees_children
                    .entry(update.key.clone())
                    .and_modify(|sv| {
                        sv.root_key = root_key.clone();
                    })
                    .or_insert_with(|| {
                        SubtreeView::new(
                            self.commands_sender.clone(),
                            self.path.child(update.key.clone()),
                            root_key.clone(),
                        )
                    });
            }

            self.elements_children.insert(
                update.key.clone(),
                ElementView::new(
                    update.key,
                    WrappedElement::Element(update.element),
                    Some(update.kv_digest_hash),
                    Some(update.value_hash),
                ),
            );
        } else {
            let subtree_key = node_update.update.path[node_update.processed_segments].clone();
            let subtree = self
                .subtrees_children
                .entry(subtree_key.clone())
                .or_insert_with(|| {
                    self.elements_children.insert(
                        subtree_key.clone(),
                        ElementView::new(
                            subtree_key.clone(),
                            WrappedElement::SubtreePlaceholder,
                            None,
                            None,
                        ),
                    );
                    SubtreeView::new(
                        self.commands_sender.clone(),
                        (&self.path).child(subtree_key),
                        None,
                    )
                });
            node_update.processed_segments += 1;
            subtree.apply_node_update(node_update);
        }
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

    fn next_page(&mut self) {
        self.page_index += 1;
    }

    fn prev_page(&mut self) {
        self.page_index = self.page_index.saturating_sub(1);
    }

    pub(crate) fn draw<'t>(
        &'t mut self,
        tree_view_context: TreeViewContext<'a, 't>,
        ui: &mut egui::Ui,
        coords: Option<Pos2>,
    ) {
        let mut area_builder = egui::Area::new(self.path.id()).order(Order::Background);
        area_builder = if let Some(coords) = coords {
            area_builder.fixed_pos(coords)
        } else {
            area_builder.anchor(Align2::CENTER_CENTER, (0., 0.))
        };

        let area_id = area_builder
            .constrain(false)
            .show(ui.ctx(), |area| {
                area.set_clip_rect(tree_view_context.transform.inverse() * tree_view_context.rect);

                egui::Frame::default()
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(Stroke {
                        width: 1.0,
                        color: Color32::DARK_GRAY,
                    })
                    .show(area, |subtree_ui| {
                        subtree_ui.allocate_space((NODE_WIDTH, 0.).into());

                        // Control buttons area
                        subtree_ui.horizontal(|controls_ui| {
                            if controls_ui.button("10").clicked() {
                                self.fetch_n(10);
                            }

                            if controls_ui.button("100").clicked() {
                                self.fetch_n(100);
                            }

                            if controls_ui
                                .button(egui_phosphor::variants::regular::DATABASE)
                                .clicked()
                            {
                                self.fetch_all();
                            }

                            if let Some(key) = self.root_key.as_ref() {
                                if controls_ui
                                    .button(egui_phosphor::variants::regular::ANCHOR)
                                    .clicked()
                                {
                                    self.fetch_key(key.clone());
                                }
                            }
                        });

                        subtree_ui.separator();

                        // Subtree path area
                        path_label(subtree_ui, self.path);

                        subtree_ui.separator();

                        for (_, element) in self
                            .elements_children
                            .iter_mut()
                            .skip(self.page_index * KV_PER_PAGE)
                            .take(KV_PER_PAGE)
                        {
                            element.draw(
                                subtree_ui,
                                &mut SubtreeViewContext {
                                    tree_view_context,
                                    path: self.path,
                                    subtrees: &mut self.subtrees_children,
                                },
                            );

                            subtree_ui.separator();
                        }

                        if self.elements_children.len() > KV_PER_PAGE {
                            subtree_ui.horizontal(|pagination| {
                                if pagination
                                    .add_enabled(self.page_index > 0, egui::Button::new("⬅"))
                                    .clicked()
                                {
                                    self.prev_page();
                                }
                                if pagination
                                    .add_enabled(
                                        (self.page_index + 1) * KV_PER_PAGE < self.elements_children.len(),
                                        egui::Button::new("➡"),
                                    )
                                    .clicked()
                                {
                                    self.next_page();
                                }
                            });
                        }

                        // Connect to parent
                        if let (Some(parent_path), Some(self_pos)) = (self.path.parent(), coords) {
                            if let Some(parent_pos) = ui.memory(|mem| {
                                mem.area_rect(parent_path.id()).map(|rect| rect.center_bottom())
                            }) {
                                let painter = subtree_ui.painter();
                                painter.line_segment(
                                    [parent_pos, self_pos + (NODE_WIDTH / 2., 0.).into()],
                                    Stroke {
                                        width: 1.0,
                                        color: subtree_line_color(subtree_ui.ctx()),
                                    },
                                );
                            }
                        }
                    })
            })
            .response
            .layer_id;

        ui.ctx()
            .set_transform_layer(area_id, *tree_view_context.transform);

        if let Some(bottom_pos) =
            ui.memory(|mem| mem.area_rect(self.path.id()).map(|rect| rect.center_bottom()))
        {
            let width: usize = std::cmp::max(
                self.subtrees_children
                    .iter()
                    .filter(|(_, s)| s.show)
                    .map(|(_, s)| s.width)
                    .sum(),
                1,
            );
            self.width = width;
            let width_f = width_to_egui(width);

            let mut current_x = bottom_pos.x - width_f / 2. - NODE_WIDTH / 2.;
            let y = bottom_pos.y + NODE_MARGIN_VERTICAL;

            for (_, subtree) in self.subtrees_children.iter_mut().filter(|(_, s)| s.show) {
                let subtree_width = width_to_egui(subtree.width);
                current_x += subtree_width / 2.;
                subtree.draw(tree_view_context, ui, Some((current_x, y).into()));
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
