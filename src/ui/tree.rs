//! Tree structure UI module

use std::borrow::Borrow;

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
        alignment::{COLLAPSED_SUBTREE_WIDTH, NODE_HEIGHT},
        Element, Key, KeySlice, NodeCtx, Path, SubtreeCtx, Tree,
    },
};

const KV_PER_PAGE: usize = 10;

pub(crate) struct TreeDrawer<'u, 't> {
    ui: &'u mut egui::Ui,
    transform: TSTransform,
    rect: Rect,
    references: Vec<(Pos2, Path, Key)>,
    tree: &'t Tree,
    sender: &'t Sender<Message>,
}

impl<'u, 't> TreeDrawer<'u, 't> {
    pub(crate) fn new(
        ui: &'u mut egui::Ui,
        transform: TSTransform,
        rect: Rect,
        tree: &'t Tree,
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

    fn draw_node_area<'b>(
        &mut self,
        parent_coords: Option<Pos2>,
        coords: Pos2,
        node_ctx: NodeCtx<'b>,
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

    fn draw_subtree_part<'a>(&mut self, mut coords: Pos2, node_ctx: NodeCtx<'a>) {
        let subtree_ctx = node_ctx.subtree_ctx();
        let mut current_level_nodes: Vec<Option<(Option<KeySlice>, KeySlice)>> = Vec::new();
        let mut next_level_nodes: Vec<Option<(Option<KeySlice>, KeySlice)>> = Vec::new();
        let mut level: u32 = 0;
        let levels = node_ctx.subtree().levels();
        let leafs = node_ctx.subtree().leafs();

        current_level_nodes.push(Some((None, node_ctx.key())));

        let max_width = node_ctx.subtree().width();
        let x_base = coords.x - max_width / 2.;

        let unit = max_width / leafs as f32;

        while level <= levels {
            coords.x = x_base;
            if level < levels {
                coords.x += 2u32.pow(levels - level - 1) as f32 * unit;
            }

            for item in current_level_nodes.drain(..) {
                if let Some((parent_key, cur_node_ctx)) = item
                    .map(|(p, k)| subtree_ctx.get_node(&k).map(|ctx| (p, ctx)))
                    .flatten()
                {
                    let parent_out_coords = parent_key
                        .map(|k| subtree_ctx.subtree().get_node_output(&k))
                        .flatten();
                    self.draw_node_area(parent_out_coords, coords, cur_node_ctx);

                    let (node, _, key) = cur_node_ctx.split();

                    if let Element::Reference { path, key } = &node.element {
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
                                node.left_child
                                    .as_deref()
                                    .map(|child_key| (Some(key), child_key)),
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
                                node.right_child
                                    .as_deref()
                                    .map(|child_key| (Some(key), child_key)),
                            )
                            .flatten(),
                    );
                } else {
                    next_level_nodes.push(None);
                    next_level_nodes.push(None);
                }
                coords.x += 2u32.pow(levels - level) as f32 * unit;
            }
            if next_level_nodes.iter().all(Option::is_none) {
                break;
            }

            coords.y += NODE_HEIGHT;
            std::mem::swap(&mut current_level_nodes, &mut next_level_nodes);
            level += 1;
        }
    }

    fn draw_subtree(&mut self, coords: Pos2, subtree_ctx: SubtreeCtx) {
        if subtree_ctx.subtree().is_expanded() {
            self.draw_subtree_expanded(coords, subtree_ctx);
        } else {
            self.draw_subtree_collapsed(coords, subtree_ctx);
        }
    }

    fn draw_subtree_collapsed(&mut self, coords: Pos2, subtree_ctx: SubtreeCtx) {
        let subtree = subtree_ctx.subtree();
        let layer_response = egui::Area::new(subtree_ctx.egui_id())
            .fixed_pos(coords)
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
                                        path: subtree_ctx.path().clone(),
                                        key: key.clone(),
                                    });
                                }
                            }

                            if let Some(key) = &subtree.root_node {
                                if menu.button("Fetch root").clicked() {
                                    // TODO error handling
                                    let _ = self.sender.blocking_send(Message::FetchNode {
                                        path: subtree_ctx.path().clone(),
                                        key: key.clone(),
                                    });
                                }
                            }

                            if menu.button("Unload").clicked() {
                                // TODO error handling
                                let _ = self.sender.blocking_send(Message::UnloadSubtree {
                                    path: subtree_ctx.path().clone(),
                                });
                            }
                        });

                        ui.allocate_ui(
                            egui::Vec2 {
                                x: COLLAPSED_SUBTREE_WIDTH - 50.,
                                y: 10.0,
                            },
                            |ui| ui.separator(),
                        );

                        path_label(
                            ui,
                            subtree_ctx.path(),
                            &mut subtree.path_display_variant_mut(),
                        );

                        ui.allocate_ui(
                            egui::Vec2 {
                                x: COLLAPSED_SUBTREE_WIDTH - 50.,
                                y: 10.0,
                            },
                            |ui| ui.separator(),
                        );

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
                                if subtree_ctx.path() != ref_path {
                                    self.references.push((
                                        subtree.get_subtree_output_point(),
                                        ref_path.clone(),
                                        ref_key.clone(),
                                    ));
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
                                binary_label_colored(
                                    key_line,
                                    node_ctx.key(),
                                    &mut node_ctx.node().ui_state.borrow_mut().key_display_variant,
                                    color,
                                );
                            });

                            if matches!(
                                node_ctx.node().element,
                                Element::Item { .. }
                                    | Element::SumItem { .. }
                                    | Element::Sumtree { .. }
                                    | Element::Reference { .. }
                            ) {
                                draw_element(ui, node_ctx);
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

    fn draw_subtree_expanded(&mut self, coords: Pos2, subtree_ctx: SubtreeCtx) {
        subtree_ctx.get_root().into_iter().for_each(|node_ctx| {
            self.draw_subtree_part(coords, node_ctx);
        });
    }

    pub(crate) fn draw_tree(mut self) {
        self.tree.update_dimensions();

        let mut current_level = 0;
        let mut current_height = 100.;
        let mut current_parent = None;
        let mut current_x_per_parent = 500.;

        for subtree_ctx in self
            .tree
            .iter_subtrees()
            .filter(|ctx| ctx.subtree().visible())
        {
            let parent_path = if subtree_ctx.path().len() == 0 {
                None
            } else {
                Some(&subtree_ctx.path()[0..subtree_ctx.path().len() - 1])
            };
            if current_parent != parent_path {
                current_parent = parent_path;
                if let Some(path) = current_parent {
                    let path: Path = path.to_vec().into();
                    let parent_subtree = self.tree.subtrees.get(&path).expect("parent must exist");
                    current_x_per_parent = parent_subtree.get_subtree_input_point().unwrap().x
                        - parent_subtree.width() / 2.0
                        - COLLAPSED_SUBTREE_WIDTH / 2.0;
                }
            }
            if subtree_ctx.path().len() > current_level {
                current_height += self.tree.levels_dimentions.borrow()[current_level].1
                    + self.tree.levels_dimentions.borrow()[current_level].0 * 0.05;
                current_level += 1;
            }

            if subtree_ctx.path().len() > 0 {
                current_x_per_parent += subtree_ctx.subtree().width() / 2.0;
            }
            self.draw_subtree(Pos2::new(current_x_per_parent, current_height), subtree_ctx);
            if subtree_ctx.path().len() > 0 {
                current_x_per_parent += subtree_ctx.subtree().width() / 2.0;
            }

            let root_in = subtree_ctx.subtree().get_subtree_input_point();
            let mut parent_path = subtree_ctx.path().clone();
            let key = parent_path.pop();
            let subtree_parent_out: Option<Pos2> = self
                .tree
                .get_subtree(&parent_path)
                .map(|s| key.map(|k| s.subtree().get_node_output(&k)))
                .flatten()
                .flatten();
            if let (Some(in_point), Some(out_point)) = (root_in, subtree_parent_out) {
                let layer_response =
                    egui::Area::new(Id::new(("subtree_lines", subtree_ctx.path())))
                        .default_pos(Pos2::new(0.0, 0.0))
                        .order(egui::Order::Background)
                        .show(self.ui.ctx(), |ui| {
                            ui.set_clip_rect(self.transform.inverse() * self.rect);

                            let painter = ui.painter();
                            painter.line_segment(
                                [out_point, in_point],
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
            .set_transform_layer(layer_response.layer_id, self.transform);
    }
}
