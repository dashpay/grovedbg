use eframe::{
    egui::{self, Button, Color32, Context, FontId, Id, Pos2, Rect, Stroke, Vec2},
    emath::TSTransform,
};
use grovedbg_types::Key;
use reingold_tilford::{Coordinate, NodeInfo};

use crate::{
    bus::CommandBus,
    path_ctx::Path,
    profiles::ActiveProfileSubtreeContext,
    protocol::FetchCommand,
    theme::proof_node_color,
    tree_data::{SubtreeData, SubtreeProofData},
    tree_view::{ElementView, ElementViewContext, SubtreeElements, NODE_WIDTH},
};

const INNER_MARGIN: f32 = 8.;

struct MerkTree<T>(T);

impl<'a> NodeInfo<&'a Key> for MerkTree<&'a SubtreeElements> {
    type Key = &'a Key;

    fn key(&self, key: &'a Key) -> Self::Key {
        key
    }

    fn children(&self, key: &'a Key) -> reingold_tilford::SmallVec<&'a Key> {
        self.0
            .get(key)
            .and_then(|a| a.left_child.as_ref())
            .and_then(|lc| self.0.get(lc).map(|n| n.merk_visible.then_some(lc)))
            .flatten()
            .into_iter()
            .chain(
                self.0
                    .get(key)
                    .and_then(|a| a.right_child.as_ref())
                    .and_then(|rc| self.0.get(rc).map(|n| n.merk_visible.then_some(rc)))
                    .flatten()
                    .into_iter(),
            )
            .collect()
    }
}

pub(crate) struct MerkView {
    initial_focus: bool,
    transform: TSTransform,
    node_focus: Option<Key>,
}

impl MerkView {
    pub(crate) fn new() -> Self {
        MerkView {
            transform: TSTransform::default(),
            initial_focus: false,
            node_focus: None,
        }
    }

    fn draw_node<'pd>(
        &mut self,
        ctx: &Context,
        rect: Rect,
        bus: &CommandBus,
        subtree_data: &mut SubtreeData,
        subtree_proof_data: &mut Option<&mut SubtreeProofData>,
        path: Path,
        element_view_context: &mut ElementViewContext,
        key: Key,
        coords: Pos2,
    ) {
        let Some(mut element_view) = subtree_data.elements.remove(&key) else {
            return;
        };

        let area_id = egui::Area::new(Id::new(&key))
            .constrain(false)
            .fixed_pos(coords)
            .show(ctx, |area| {
                area.set_clip_rect(self.transform.inverse() * rect);
                let color = subtree_proof_data
                    .as_ref()
                    .and_then(|pd| pd.contains_key(&key).then(|| proof_node_color(ctx)))
                    .unwrap_or(Color32::DARK_GRAY);

                let mut center_bottom = egui::Frame::default()
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(INNER_MARGIN))
                    .stroke(Stroke { width: 1., color })
                    .show(area, |node_ui| {
                        node_ui.set_max_width(NODE_WIDTH);

                        element_view.draw(node_ui, element_view_context, &mut subtree_data.visible_keys);

                        if let Some(proof_node) = subtree_proof_data.as_mut().and_then(|s| s.get_mut(&key)) {
                            node_ui.separator();
                            proof_node.draw(node_ui);
                        }

                        node_ui.separator();

                        let left_button = Button::new(egui_phosphor::regular::ARROW_LEFT);
                        node_ui.horizontal(|line| {
                            if let Some(left) = element_view.left_child.as_ref() {
                                if subtree_proof_data
                                    .as_ref()
                                    .map(|p| p.contains_key(left))
                                    .unwrap_or_default()
                                {
                                    subtree_data
                                        .elements
                                        .entry(left.clone())
                                        .or_insert_with(|| ElementView::new_placeholder(left.clone()))
                                        .merk_visible = true;
                                }
                                if line
                                    .add(left_button)
                                    .on_hover_text("Fetch and show left child")
                                    .clicked()
                                {
                                    self.node_focus = Some(left.clone());
                                    subtree_data
                                        .elements
                                        .entry(left.clone())
                                        .or_insert_with(|| ElementView::new_placeholder(left.clone()))
                                        .merk_visible = true;

                                    bus.fetch_command(FetchCommand::FetchNode {
                                        path: path.to_vec(),
                                        key: left.clone(),
                                    });
                                }
                            } else {
                                line.add_enabled(false, left_button);
                            }

                            let right_button = Button::new(egui_phosphor::regular::ARROW_RIGHT);
                            if let Some(right) = element_view.right_child.as_ref() {
                                if subtree_proof_data
                                    .as_mut()
                                    .map(|p| p.contains_key(right))
                                    .unwrap_or_default()
                                {
                                    subtree_data
                                        .elements
                                        .entry(right.clone())
                                        .or_insert_with(|| ElementView::new_placeholder(right.clone()))
                                        .merk_visible = true;
                                }
                                if line
                                    .add(right_button)
                                    .on_hover_text("Fetch and show right child")
                                    .clicked()
                                {
                                    self.node_focus = Some(right.clone());
                                    subtree_data
                                        .elements
                                        .entry(right.clone())
                                        .or_insert_with(|| ElementView::new_placeholder(right.clone()))
                                        .merk_visible = true;

                                    bus.fetch_command(FetchCommand::FetchNode {
                                        path: path.to_vec(),
                                        key: right.clone(),
                                    });
                                }
                            } else {
                                line.add_enabled(false, right_button);
                            }
                        });

                        node_ui.max_rect().center_bottom()
                    })
                    .inner;

                center_bottom.y += INNER_MARGIN;

                if let Some(k) = element_view.left_child.as_ref().and_then(|c| {
                    subtree_data
                        .elements
                        .get(c)
                        .map(|e| e.merk_visible)
                        .unwrap_or_default()
                        .then_some(c)
                }) {
                    if let Some(left_pos) =
                        area.memory(|mem| mem.area_rect(Id::new(&k)).map(|rect| rect.center_top()))
                    {
                        let painter = area.painter();

                        painter.text(
                            left_pos
                                - ((left_pos.to_vec2() - center_bottom.to_vec2()) / 5.)
                                - Vec2::new(20., 0.),
                            egui::Align2::CENTER_CENTER,
                            "L",
                            FontId::monospace(11.),
                            Color32::DARK_GRAY,
                        );

                        painter.line_segment(
                            [center_bottom, left_pos],
                            Stroke {
                                width: 1.,
                                color: Color32::DARK_GRAY,
                            },
                        );
                    }
                }

                if let Some(k) = element_view.right_child.as_ref().and_then(|c| {
                    subtree_data
                        .elements
                        .get(c)
                        .map(|e| e.merk_visible)
                        .unwrap_or_default()
                        .then_some(c)
                }) {
                    if let Some(right_pos) =
                        area.memory(|mem| mem.area_rect(Id::new(&k)).map(|rect| rect.center_top()))
                    {
                        let painter = area.painter();

                        painter.text(
                            right_pos - ((right_pos.to_vec2() - center_bottom.to_vec2()) / 5.)
                                + Vec2::new(20., 0.),
                            egui::Align2::CENTER_CENTER,
                            "R",
                            FontId::monospace(11.),
                            Color32::DARK_GRAY,
                        );

                        painter.line_segment(
                            [center_bottom, right_pos],
                            Stroke {
                                width: 1.,
                                color: Color32::DARK_GRAY,
                            },
                        );
                    }
                }
            })
            .response
            .layer_id;

        ctx.set_transform_layer(area_id, self.transform);
        subtree_data.elements.insert(key, element_view);
    }

    pub(crate) fn draw<'pa>(
        &mut self,
        ui: &mut egui::Ui,
        bus: &CommandBus<'pa>,
        path: Path<'pa>,
        subtree_data: &mut SubtreeData,
        mut subtree_proof_data: Option<&mut SubtreeProofData>,
        mut profile_ctx: ActiveProfileSubtreeContext,
    ) {
        let Some(root_key) = subtree_data.root_key.clone() else {
            return;
        };

        if !self.initial_focus {
            self.node_focus = Some(root_key.clone());
            self.initial_focus = true;
        }

        subtree_data
            .get_root()
            .into_iter()
            .for_each(|r| r.merk_visible = true);

        let (id, rect) = ui.allocate_space(ui.available_size());

        let pointer_response = ui.interact(rect, id, egui::Sense::click_and_drag());

        let transform_before = self.transform;

        if pointer_response.dragged() {
            self.transform.translation += pointer_response.drag_delta();
        }
        if pointer_response.double_clicked() {
            self.node_focus = Some(root_key.clone());
        }

        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            if pointer_response.hovered() {
                let pointer_in_layer = self.transform.inverse() * pointer;
                let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

                // // Zoom in on pointer:
                self.transform = self.transform
                    * TSTransform::from_translation(pointer_in_layer.to_vec2())
                    * TSTransform::from_scaling(zoom_delta)
                    * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                // Pan:
                self.transform = TSTransform::from_translation(pan_delta) * self.transform;
            }
        }

        if transform_before != self.transform {
            self.node_focus = None;
        }

        if let Some(focused_node) = &self.node_focus {
            let node_pos = ui
                .ctx()
                .memory(|mem| mem.area_rect(Id::new(focused_node)).map(|rect| rect.center()));
            let root_pos = ui.max_rect().center();
            if let Some(node_pos) = node_pos {
                self.transform =
                    TSTransform::from_translation(node_pos.to_vec2() - root_pos.to_vec2()).inverse();
            }
        }

        let tree = MerkTree(&subtree_data.elements);

        let layout: Vec<(Key, Coordinate)> = reingold_tilford::layout(&tree, &root_key)
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();

        let mut element_view_context = ElementViewContext {
            path,
            profile_ctx: &mut profile_ctx,
            bus,
        };

        for (key, Coordinate { x, y }) in layout {
            let coords = Pos2::new(x as f32, y as f32) * NODE_WIDTH * 1.2;

            self.draw_node(
                ui.ctx(),
                rect,
                bus,
                subtree_data,
                &mut subtree_proof_data,
                path,
                &mut element_view_context,
                key,
                coords,
            );
        }
    }
}
