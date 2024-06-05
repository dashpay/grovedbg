//! Tree structure UI module

use std::{cmp, collections::VecDeque};

use eframe::{
    egui::{self, Id},
    emath::TSTransform,
    epaint::{Color32, Pos2, Rect, Stroke},
};
use tokio::sync::mpsc::Sender;

use super::{
    common::{binary_label_colored, path_label},
    node::{draw_element, draw_node, element_to_color},
};
use crate::{
    fetch::Message,
    model::{
        alignment::COLLAPSED_SUBTREE_WIDTH, path_display::Path, Element, Key, KeySlice, NodeCtx,
        SubtreeCtx, Tree,
    },
};

const CELL_X: f32 = 300.0;
const CELL_Y: f32 = 150.0;

const KV_PER_PAGE: usize = 10;

pub(crate) struct TreeDrawer<'u, 't, 'c> {
    ui: &'u mut egui::Ui,
    transform: TSTransform,
    rect: Rect,
    references: Vec<(Pos2, Path<'t>, Key)>,
    tree: &'t Tree<'c>,
    sender: &'t Sender<Message>,
}

impl<'u, 't, 'c> TreeDrawer<'u, 't, 'c> {
    pub(crate) fn new(
        ui: &'u mut egui::Ui,
        transform: TSTransform,
        rect: Rect,
        tree: &'t Tree<'c>,
        sender: &'t Sender<Message>,
    ) -> Self {
        Self {
            ui,
            transform,
            rect,
            references: vec![],
            tree,
            sender,
        }
    }

    fn draw_node_area<'a>(
        &'a mut self,
        parent_coords: Option<Pos2>,
        coords: Pos2,
        node_ctx: &NodeCtx<'a, 't>,
    ) {
        let layer_response = egui::Area::new(Id::new(("area", node_ctx.egui_id())))
            .fixed_pos(coords)
            .order(egui::Order::Foreground)
            .show(self.ui.ctx(), |ui| {
                ui.set_clip_rect(self.transform.inverse() * self.rect);
                if let Some(out_coords) = parent_coords {
                    let painter = ui.painter();
                    painter.line_segment(
                        [out_coords, node_ctx.node().ui_state.borrow().input_point],
                        Stroke {
                            width: 1.0,
                            color: Color32::GRAY,
                        },
                    );
                }

                draw_node(ui, self.sender, node_ctx);
            })
            .response;

        {
            let mut state = node_ctx.node().ui_state.borrow_mut();
            state.input_point = layer_response.rect.center_top();
            state.output_point = layer_response.rect.center_bottom();
            state.left_sibling_point = layer_response.rect.left_center();
            state.right_sibling_point = layer_response.rect.right_center();
        };
        self.ui
            .ctx()
            .set_transform_layer(layer_response.layer_id, self.transform);
    }

    fn draw_subtree_part(
        &mut self,
        (mut x_coord, mut y_coord): (i64, usize),
        node_ctx: NodeCtx<'t, 't>,
    ) {
        let subtree_ctx = node_ctx.subtree_ctx();
        let mut current_level_nodes: Vec<Option<(Option<Key>, KeySlice)>> = Vec::new();
        let mut next_level_nodes: Vec<Option<(Option<Key>, KeySlice)>> = Vec::new();
        let mut level: u32 = 0;
        let levels = node_ctx.subtree().levels();
        let leaves = node_ctx.subtree().leaves();

        current_level_nodes.push(Some((None, node_ctx.key())));

        let x_base = x_coord - leaves as i64;

        while level < levels {
            x_coord = x_base;
            x_coord += 2i64.pow(levels - level - 1) - 1;

            for item in current_level_nodes.drain(..) {
                if let Some((parent_key, cur_node_ctx)) = item
                    .map(|(p, k)| subtree_ctx.get_node(k.to_vec()).map(|ctx| (p, ctx)))
                    .flatten()
                {
                    let parent_out_coords = parent_key
                        .map(|k| subtree_ctx.subtree().get_node_output(&k))
                        .flatten();
                    self.draw_node_area(
                        parent_out_coords,
                        Pos2::new(x_coord as f32 * CELL_X, y_coord as f32 * CELL_Y),
                        &cur_node_ctx,
                    );

                    // let (node, _, key) = cur_node_ctx.split();

                    if let Element::Reference { path, key } = &cur_node_ctx.node().element {
                        self.references.push((
                            cur_node_ctx.node().ui_state.borrow().output_point,
                            path.clone(),
                            key.clone(),
                        ));
                    }

                    next_level_nodes.push(
                        cur_node_ctx
                            .node()
                            .ui_state
                            .borrow()
                            .show_left
                            .then_some(
                                cur_node_ctx.node().left_child.as_deref().map(|child_key| {
                                    (Some(cur_node_ctx.key().to_vec()), child_key)
                                }),
                            )
                            .flatten(),
                    );

                    next_level_nodes.push(
                        cur_node_ctx
                            .node()
                            .ui_state
                            .borrow()
                            .show_right
                            .then_some(
                                cur_node_ctx.node().right_child.as_deref().map(|child_key| {
                                    (Some(cur_node_ctx.key().to_vec()), child_key)
                                }),
                            )
                            .flatten(),
                    );
                } else {
                    next_level_nodes.push(None);
                    next_level_nodes.push(None);
                }
                x_coord += 2i64.pow(levels - level);
            }
            if next_level_nodes.iter().all(Option::is_none) {
                break;
            }

            y_coord += 2;
            std::mem::swap(&mut current_level_nodes, &mut next_level_nodes);
            level += 1;
        }
    }

    fn draw_subtree(&mut self, coords: (i64, usize), subtree_ctx: SubtreeCtx<'t, 'c>) {
        if subtree_ctx.subtree().is_expanded() {
            self.draw_subtree_expanded(coords, subtree_ctx);
        } else {
            self.draw_subtree_collapsed(coords, subtree_ctx);
        }
    }

    fn draw_subtree_collapsed(
        &mut self,
        (coord_x, coord_y): (i64, usize),
        subtree_ctx: SubtreeCtx<'t, 'c>,
    ) {
        let subtree = subtree_ctx.subtree();
        let layer_response = egui::Area::new(subtree_ctx.egui_id())
            .fixed_pos((coord_x as f32 * CELL_X, coord_y as f32 * CELL_Y))
            .order(egui::Order::Foreground)
            .show(self.ui.ctx(), |ui| {
                ui.set_clip_rect(self.transform.inverse() * self.rect);

                let mut stroke = Stroke::default();
                stroke.width = 1.0;

                egui::Frame::default()
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(stroke)
                    .fill(Color32::BLACK)
                    .show(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.collapsing("🖧", |menu| {
                            if !subtree.is_empty()
                                && subtree.root_node().is_some()
                                && menu.button("Expand").clicked()
                            {
                                subtree.set_expanded();
                                subtree_ctx.set_children_invisible();
                            }

                            if menu.button("Fetch all").clicked() {
                                if let Some(key) = &subtree.root_node {
                                    // TODO error handling
                                    let _ = self.sender.blocking_send(Message::FetchBranch {
                                        path: subtree_ctx.path().to_vec(),
                                        key: key.clone(),
                                    });
                                }
                            }

                            if let Some(key) = &subtree.root_node {
                                if menu.button("Fetch root").clicked() {
                                    // TODO error handling
                                    let _ = self.sender.blocking_send(Message::FetchNode {
                                        path: subtree_ctx.path().to_vec(),
                                        key: key.clone(),
                                    });
                                }
                            }

                            if menu.button("Unload").clicked() {
                                // TODO error handling
                                let _ = self.sender.blocking_send(Message::UnloadSubtree {
                                    path: subtree_ctx.path().to_vec(),
                                });
                            }
                        });

                        ui.allocate_ui(egui::Vec2 { x: CELL_X, y: 10.0 }, |ui| ui.separator());

                        path_label(ui, subtree_ctx.path());

                        ui.allocate_ui(egui::Vec2 { x: CELL_X, y: 10.0 }, |ui| ui.separator());

                        for node_ctx in subtree_ctx
                            .iter_nodes()
                            .skip(subtree.page_idx() * KV_PER_PAGE)
                            .take(KV_PER_PAGE)
                        {
                            if let Element::Reference {
                                path: ref_path,
                                key: ref_key,
                            } = &node_ctx.node().element
                            {
                                if subtree_ctx.path() != *ref_path {
                                    let point = subtree.get_subtree_output_point();
                                    let key = ref_key.clone();
                                    let path: Path<'c> = *ref_path;
                                    self.references.push((point, path, key));
                                }
                            }

                            let color = element_to_color(&node_ctx.node().element);

                            ui.horizontal(|key_line| {
                                if matches!(
                                    node_ctx.node().element,
                                    Element::Subtree { .. } | Element::Sumtree { .. }
                                ) {
                                    let prev_visibility =
                                        subtree_ctx.is_child_visible(node_ctx.key());
                                    let mut visibility = prev_visibility;
                                    key_line.checkbox(&mut visibility, "");
                                    if prev_visibility != visibility {
                                        subtree_ctx
                                            .set_child_visibility(node_ctx.key(), visibility);
                                    }
                                }

                                node_ctx.with_key_display_variant(|display_variant| {
                                    binary_label_colored(
                                        key_line,
                                        node_ctx.key(),
                                        display_variant,
                                        color,
                                    )
                                })
                            });

                            if matches!(
                                node_ctx.node().element,
                                Element::Item { .. }
                                    | Element::SumItem { .. }
                                    | Element::Sumtree { .. }
                                    | Element::Reference { .. }
                            ) {
                                draw_element(ui, &node_ctx);
                            }

                            ui.allocate_ui(
                                egui::Vec2 {
                                    x: COLLAPSED_SUBTREE_WIDTH - 50.,
                                    y: 10.0,
                                },
                                |ui| ui.separator(),
                            );
                        }

                        if subtree.nodes.len() > KV_PER_PAGE {
                            ui.horizontal(|pagination| {
                                if pagination
                                    .add_enabled(subtree.page_idx() > 0, egui::Button::new("⬅"))
                                    .clicked()
                                {
                                    subtree.prev_page();
                                }
                                if pagination
                                    .add_enabled(
                                        (subtree.page_idx() + 1) * KV_PER_PAGE < subtree.n_nodes(),
                                        egui::Button::new("➡"),
                                    )
                                    .clicked()
                                {
                                    subtree.next_page();
                                }
                            });
                        }
                    });
            })
            .response;

        subtree.set_input_point(layer_response.rect.center_top());
        subtree.set_output_point(layer_response.rect.center_bottom());

        self.ui
            .ctx()
            .set_transform_layer(layer_response.layer_id, self.transform);
    }

    fn draw_subtree_expanded(&mut self, coords: (i64, usize), subtree_ctx: SubtreeCtx<'t, 't>) {
        subtree_ctx.get_root().into_iter().for_each(|node_ctx| {
            self.draw_subtree_part(coords, node_ctx);
        });
    }

    pub(crate) fn draw_tree(mut self) {
        self.tree.update_dimensions();

        let mut parents_queue = VecDeque::new();

        let Some(root_subtree) = self.tree.get_subtree(&self.tree.path_ctx.get_root()) else {
            return;
        };

        parents_queue.push_back((0 as i64, root_subtree));

        self.draw_subtree((0, 0), root_subtree);

        while let Some((x, parent_subtree_ctx)) = parents_queue.pop_front() {
            let current_height = self
                .tree
                .levels_heights
                .borrow()
                .iter()
                .take(parent_subtree_ctx.path().level() + 1)
                .sum();
            let mut current_children_x = x - parent_subtree_ctx.subtree().width() as i64 / 2;
            let current_parent_children_queue: VecDeque<_> = parent_subtree_ctx
                .iter_subtrees()
                .filter(SubtreeCtx::is_visible)
                .collect();

            for child in current_parent_children_queue.iter() {
                let child_x = current_children_x + child.subtree().width() as i64 / 2;
                parents_queue.push_back((child_x, child.clone()));

                self.draw_subtree((child_x, current_height), child.clone());

                if let Some(input_point) = child.subtree().get_subtree_input_point() {
                    let layer_response = egui::Area::new(Id::new(("subtree_lines", child.path())))
                        .default_pos(Pos2::new(0.0, 0.0))
                        .order(egui::Order::Background)
                        .show(self.ui.ctx(), |ui| {
                            ui.set_clip_rect(self.transform.inverse() * self.rect);

                            let painter = ui.painter();
                            painter.line_segment(
                                [
                                    parent_subtree_ctx.subtree().get_subtree_output_point(),
                                    input_point,
                                ],
                                Stroke {
                                    width: 1.0,
                                    color: Color32::GOLD,
                                },
                            );
                        })
                        .response;
                    self.ui
                        .ctx()
                        .set_transform_layer(layer_response.layer_id, self.transform);
                }

                current_children_x += child.subtree().width() as i64 + 1;
            }
        }

        // for subtree_ctx in self
        //     .tree
        //     .iter_subtrees()
        //     .filter(|ctx| ctx.subtree().visible())
        // {
        //     let parent_path = subtree_ctx.path().parent();
        //     if current_parent != parent_path {
        //         current_parent = parent_path;
        //         if let Some(path) = current_parent {
        //             let parent_subtree = self.tree.subtrees.get(&path).expect("parent
        // must exist");             current_x_per_parent =
        // parent_subtree.get_subtree_input_point().unwrap().x
        //                 - parent_subtree.width() / 2.0
        //                 - COLLAPSED_SUBTREE_WIDTH / 2.0;
        //         }
        //     }
        //     if subtree_ctx.path().level() > current_level {
        //         current_height +=
        // self.tree.levels_dimentions.borrow()[current_level].1
        //             + self.tree.levels_dimentions.borrow()[current_level].0 * 0.05;
        //         current_level += 1;
        //     }

        //     if subtree_ctx.path().level() > 0 {
        //         current_x_per_parent += subtree_ctx.subtree().width() / 2.0;
        //     }
        //     self.draw_subtree(Pos2::new(current_x_per_parent, current_height),
        // subtree_ctx);     if subtree_ctx.path().level() > 0 {
        //         current_x_per_parent += subtree_ctx.subtree().width() / 2.0;
        //     }

        //     let root_in = subtree_ctx.subtree().get_subtree_input_point();
        //     let key = subtree_ctx
        //         .path()
        //         .for_last_segment(|key| key.bytes().to_vec());
        //     let subtree_parent_out: Option<Pos2> = parent_path
        //         .as_ref()
        //         .map(|path| self.tree.get_subtree(path))
        //         .flatten()
        //         .map(|s| key.map(|k| s.subtree().get_node_output(&k)))
        //         .flatten()
        //         .flatten();
        //     if let (Some(in_point), Some(out_point)) = (root_in, subtree_parent_out)
        // {         let layer_response =
        //             egui::Area::new(Id::new(("subtree_lines", subtree_ctx.path())))
        //                 .default_pos(Pos2::new(0.0, 0.0))
        //                 .order(egui::Order::Background)
        //                 .show(self.ui.ctx(), |ui| {
        //                     ui.set_clip_rect(self.transform.inverse() * self.rect);

        //                     let painter = ui.painter();
        //                     painter.line_segment(
        //                         [out_point, in_point],
        //                         Stroke {
        //                             width: 1.0,
        //                             color: Color32::GOLD,
        //                         },
        //                     );
        //                 })
        //                 .response;
        //         self.ui
        //             .ctx()
        //             .set_transform_layer(layer_response.layer_id, self.transform);
        //     }
        // }

        let layer_response = egui::Area::new(Id::new("references"))
            .default_pos(Pos2::new(0.0, 0.0))
            .order(egui::Order::Background)
            .show(self.ui.ctx(), |ui| {
                ui.set_clip_rect(self.transform.inverse() * self.rect);
                let painter = ui.painter();

                for (out_point, in_path, in_key) in self.references.into_iter() {
                    let Some(in_point) = self
                        .tree
                        .subtrees
                        .get(&in_path)
                        .map(|subtree| subtree.get_node_input(&in_key))
                        .flatten()
                    else {
                        continue;
                    };
                    painter.line_segment(
                        [out_point, in_point],
                        Stroke {
                            width: 1.0,
                            color: Color32::LIGHT_BLUE,
                        },
                    );
                }
            })
            .response;
        self.ui
            .ctx()
            .set_transform_layer(layer_response.layer_id, self.transform);
    }
}
