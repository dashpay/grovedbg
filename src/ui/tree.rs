//! Tree structure UI module

use std::collections::VecDeque;

use eframe::{
    egui::{self, Id},
    emath::TSTransform,
    epaint::{Color32, Pos2, Rect, Stroke},
};
use tokio::sync::mpsc::Sender;

use super::{
    common::path_label,
    node::{draw_element, draw_node},
};
use crate::{
    fetch::{FetchLimit, Message},
    model::{path_display::Path, Element, Key, KeySlice, NodeCtx, SubtreeCtx, Tree},
};

pub(crate) const CELL_X: f32 = 300.0;
pub(crate) const CELL_Y: f32 = 200.0;

const KV_PER_PAGE: usize = 10;

pub(crate) struct TreeDrawer<'u, 't, 'c> {
    pub(crate) ui: &'u mut egui::Ui,
    pub(crate) transform: &'u mut TSTransform,
    pub(crate) rect: Rect,
    pub(crate) references: Vec<(Pos2, Path<'t>, Key)>,
    pub(crate) tree: &'t Tree<'c>,
    pub(crate) sender: &'t Sender<Message>,
}

impl<'u, 't, 'c> TreeDrawer<'u, 't, 'c> {
    pub(crate) fn new(
        ui: &'u mut egui::Ui,
        transform: &'u mut TSTransform,
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

                draw_node(ui, &mut self.transform, self.sender, node_ctx);
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
            .set_transform_layer(layer_response.layer_id, self.transform.clone());
    }

    fn draw_subtree_part(&mut self, (mut x_coord, mut y_coord): (i64, usize), node_ctx: NodeCtx<'t, 't>) {
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
                                cur_node_ctx
                                    .node()
                                    .left_child
                                    .as_deref()
                                    .map(|child_key| (Some(cur_node_ctx.key().to_vec()), child_key)),
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
                                cur_node_ctx
                                    .node()
                                    .right_child
                                    .as_deref()
                                    .map(|child_key| (Some(cur_node_ctx.key().to_vec()), child_key)),
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

    fn draw_subtree_collapsed(&mut self, (coord_x, coord_y): (i64, usize), subtree_ctx: SubtreeCtx<'t, 'c>) {
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
                                    let _ = self
                                        .sender
                                        .blocking_send(Message::FetchBranch {
                                            path: subtree_ctx.path().to_vec(),
                                            key: key.clone(),
                                            limit: FetchLimit::Unbounded,
                                        })
                                        .inspect_err(|_| log::error!("Can't reach data fetching thread"));
                                }
                            }

                            if menu.button("Fetch first 100").clicked() {
                                if let Some(key) = &subtree.root_node {
                                    let _ = self
                                        .sender
                                        .blocking_send(Message::FetchBranch {
                                            path: subtree_ctx.path().to_vec(),
                                            key: key.clone(),
                                            limit: FetchLimit::Count(100),
                                        })
                                        .inspect_err(|_| {
                                            log::error!("Can't reach data fetching thread");
                                        });
                                }
                            }

                            if let Some(key) = &subtree.root_node {
                                if menu.button("Fetch root").clicked() {
                                    let _ = self
                                        .sender
                                        .blocking_send(Message::FetchNode {
                                            path: subtree_ctx.path().to_vec(),
                                            key: key.clone(),
                                        })
                                        .inspect_err(|_| log::error!("Can't reach data fetching thread"));
                                }
                            }

                            if menu.button("Unload").clicked() {
                                let _ = self
                                    .sender
                                    .blocking_send(Message::UnloadSubtree {
                                        path: subtree_ctx.path().to_vec(),
                                    })
                                    .inspect_err(|_| log::error!("Can't reach data fetching thread"));
                                subtree_ctx.subtree().first_page();
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

                            draw_element(ui, &mut self.transform, &node_ctx);

                            ui.allocate_ui(egui::Vec2 { x: CELL_X, y: 10.0 }, |ui| ui.separator());
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
            .set_transform_layer(layer_response.layer_id, self.transform.clone());
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
            let mut current_children_x = x - parent_subtree_ctx.subtree().children_width() as i64 / 2;
            let current_parent_children_queue: VecDeque<_> = parent_subtree_ctx
                .iter_subtrees()
                .filter(SubtreeCtx::is_visible)
                .collect();

            for child in current_parent_children_queue.iter() {
                let child_x = current_children_x + child.subtree().width() as i64 / 2;
                parents_queue.push_back((child_x, child.clone()));

                self.draw_subtree((child_x, current_height), child.clone());

                let output_point = child
                    .path()
                    .for_last_segment(|key| parent_subtree_ctx.subtree().get_node_output(key.bytes()))
                    .flatten();
                if let (Some(output_point), Some(input_point)) =
                    (output_point, child.subtree().get_subtree_input_point())
                {
                    let layer_response = egui::Area::new(Id::new(("subtree_lines", child.path())))
                        .default_pos(Pos2::new(0.0, 0.0))
                        .order(egui::Order::Background)
                        .show(self.ui.ctx(), |ui| {
                            ui.set_clip_rect(self.transform.inverse() * self.rect);

                            let painter = ui.painter();
                            painter.line_segment(
                                [output_point, input_point],
                                Stroke {
                                    width: 1.0,
                                    color: Color32::GOLD,
                                },
                            );
                        })
                        .response;
                    self.ui
                        .ctx()
                        .set_transform_layer(layer_response.layer_id, self.transform.clone());
                }

                current_children_x += child.subtree().width() as i64 + 1;
            }
        }

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
            .set_transform_layer(layer_response.layer_id, self.transform.clone());
    }
}
