use std::collections::BTreeMap;

use eframe::egui::{self, Align2, Color32, Order, Stroke};
use grovedbg_types::{Element, Key, NodeUpdate, PathQuery, Query, QueryItem, SizedQuery, SubqueryBranch};

use super::{
    element_view::{ElementView, WrappedElement},
    SubtreeViewContext, TreeViewContext, NODE_WIDTH,
};
use crate::{path_ctx::Path, protocol::Command, CommandsSender};

const KV_PER_PAGE: usize = 10;

pub(crate) struct SubtreeView<'a> {
    path: Path<'a>,
    commands_sender: CommandsSender,
    root_key: Option<Vec<u8>>,
    subtrees_children: BTreeMap<Key, SubtreeView<'a>>,
    elements_children: BTreeMap<Key, ElementView>,
    page_index: usize,
    width: f32,
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
            width: NODE_WIDTH,
        }
    }

    pub(crate) fn set_root(&mut self, root_key: Vec<u8>) {
        self.root_key = Some(root_key);
    }

    pub(crate) fn apply_node_update(&mut self, mut node_update: RoutingNodeUpdate) {
        if node_update.is_subtree_reached() {
            let update = node_update.update;

            if let Element::Subtree { root_key, .. } | Element::Sumtree { root_key, .. } = &update.element {
                self.subtrees_children.insert(
                    update.key.clone(),
                    SubtreeView::new(
                        self.commands_sender.clone(),
                        self.path.child(update.key.clone()),
                        root_key.clone(),
                    ),
                );
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

    pub(crate) fn draw(&mut self, tree_view_context: TreeViewContext, ui: &mut egui::Ui) {
        let area_id = egui::Area::new(self.path.id())
            .order(Order::Background)
            .anchor(Align2::CENTER_CENTER, (0., 0.))
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
                    })
            })
            .response
            .layer_id;

        ui.ctx()
            .set_transform_layer(area_id, *tree_view_context.transform);
    }
}
