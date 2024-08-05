use eframe::egui::{self, Align2, Color32, Order, Stroke};
use grovedbg_types::{PathQuery, Query, QueryItem, SizedQuery, SubqueryBranch};

use super::{TreeViewContext, NODE_WIDTH};
use crate::{path_ctx::Path, protocol::Command, CommandsSender};

pub(crate) struct SubtreeView<'a> {
    path: Path<'a>,
    commands_sender: CommandsSender,
    root_key: Option<Vec<u8>>,
    children: Vec<SubtreeView<'a>>,
}

impl<'a> SubtreeView<'a> {
    pub(crate) fn new(commands_sender: CommandsSender, path: Path<'a>) -> Self {
        Self {
            path,
            commands_sender,
            root_key: None,
            children: Vec::new(),
        }
    }

    pub(crate) fn new_with_root(commands_sender: CommandsSender, path: Path<'a>, root_key: Vec<u8>) -> Self {
        Self {
            path,
            commands_sender,
            root_key: Some(root_key),
            children: Vec::new(),
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

    pub(crate) fn draw(&mut self, tree_view_ctx: TreeViewContext, ui: &mut egui::Ui) {
        let area_id = egui::Area::new(self.path.id())
            .order(Order::Background)
            .anchor(Align2::CENTER_CENTER, (0., 0.))
            .show(ui.ctx(), |area| {
                area.set_clip_rect(tree_view_ctx.transform.inverse() * tree_view_ctx.rect);

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

                        //     if let Some(key) = &subtree.root_node {
                        //         if menu.button("Fetch root").clicked() {
                        //             let _ = self
                        //                 .sender
                        //                 .blocking_send(Message::FetchNode {
                        //                     path:
                        // subtree_ctx.path().to_vec(),
                        //                     key: key.clone(),
                        //                     show: false,
                        //                 })
                        //                 .inspect_err(|_| log::error!("Can't
                        // reach data fetching thread"));
                        //         }
                        //     }

                        //     if menu.button("Unload").clicked() {
                        //         let _ = self
                        //             .sender
                        //             .blocking_send(Message::UnloadSubtree {
                        //                 path: subtree_ctx.path().to_vec(),
                        //             })
                        //             .inspect_err(|_| log::error!("Can't reach
                        // data fetching thread"));
                        //         subtree_ctx.subtree().first_page();
                        //     }
                        // });

                        // ui.allocate_ui(egui::Vec2 { x: CELL_X, y: 10.0 },
                        // |ui| ui.separator());

                        // path_label(ui, subtree_ctx.path());

                        // ui.allocate_ui(egui::Vec2 { x: CELL_X, y: 10.0 },
                        // |ui| ui.separator());

                        // for node_ctx in subtree_ctx
                        //     .iter_nodes()
                        //     .skip(subtree.page_idx() * KV_PER_PAGE)
                        //     .take(KV_PER_PAGE)
                        // {
                        //     if let Element::Reference {
                        //         path: ref_path,
                        //         key: ref_key,
                        //         ..
                        //     } = &node_ctx.node().element
                        //     {
                        //         if subtree_ctx.path() != *ref_path {
                        //             let point =
                        // subtree.get_subtree_output_point();
                        //             let key = ref_key.clone();
                        //             let path: Path<'c> = *ref_path;
                        //             self.references.push((point, path, key));
                        //         }
                        //     }

                        //     draw_element(ui, &mut self.transform, &node_ctx);

                        //     ui.allocate_ui(egui::Vec2 { x: CELL_X, y: 10.0 },
                        // |ui| ui.separator()); }

                        // if subtree.nodes.len() > KV_PER_PAGE {
                        //     ui.horizontal(|pagination| {
                        //         if pagination
                        //             .add_enabled(subtree.page_idx() > 0,
                        // egui::Button::new("⬅"))
                        //             .clicked()
                        //         {
                        //             subtree.prev_page();
                        //         }
                        //         if pagination
                        //             .add_enabled(
                        //                 (subtree.page_idx() + 1) *
                        // KV_PER_PAGE < subtree.n_nodes(),
                        //                 egui::Button::new("➡"),
                        //             )
                        //             .clicked()
                        //         {
                        //             subtree.next_page();
                        //         }
                        //     });
                        // }
                    })
            })
            .response
            .layer_id;

        ui.ctx().set_transform_layer(area_id, *tree_view_ctx.transform);
    }
}
