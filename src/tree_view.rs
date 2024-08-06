mod element_view;
mod subtree_view;

use eframe::{
    egui::{self, Rect},
    emath::TSTransform,
};
use grovedbg_types::NodeUpdate;
use subtree_view::{RoutingNodeUpdate, SubtreeView};

use crate::{
    path_ctx::{Path, PathCtx},
    CommandsSender,
};

const NODE_WIDTH: f32 = 300.;

pub(crate) struct TreeView<'a> {
    transform: TSTransform,
    path_ctx: &'a PathCtx,
    root_subtree: SubtreeView<'a>,
}

impl<'a> TreeView<'a> {
    pub(crate) fn new(commands_sender: CommandsSender, path_ctx: &'a PathCtx) -> Self {
        Self {
            transform: TSTransform::default(),
            root_subtree: SubtreeView::new(commands_sender, path_ctx.get_root(), None),
            path_ctx,
        }
    }

    pub(crate) fn apply_node_update(&mut self, node_update: NodeUpdate) {
        self.root_subtree
            .apply_node_update(RoutingNodeUpdate::new(node_update));
    }

    pub(crate) fn apply_root_node_update(&mut self, node_update: NodeUpdate) {
        self.root_subtree.set_root(node_update.key.clone());
        self.root_subtree
            .apply_node_update(RoutingNodeUpdate::new(node_update));
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui) {
        let (id, rect) = ui.allocate_space(ui.available_size());

        let pointer_response = ui.interact(rect, id, egui::Sense::click_and_drag());

        if pointer_response.dragged() {
            self.transform.translation += pointer_response.drag_delta();
        }
        if pointer_response.double_clicked() {
            self.transform = TSTransform::default();
        }

        // let transform =
        // TSTransform::from_translation(ui.min_rect().left_top().to_vec2()) *
        // self.transform;

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

        self.root_subtree
            .draw(TreeViewContext::new(self.path_ctx, &self.transform, rect), ui);
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TreeViewContext<'a> {
    path_ctx: &'a PathCtx,
    transform: &'a TSTransform,
    rect: Rect,
}

impl<'a> TreeViewContext<'a> {
    pub(crate) fn new(path_ctx: &'a PathCtx, transform: &'a TSTransform, rect: Rect) -> Self {
        Self {
            path_ctx,
            transform,
            rect,
        }
    }

    pub(crate) fn root_context(&self) -> SubtreeViewContext<'a> {
        SubtreeViewContext {
            tree_view_context: *self,
            path: self.path_ctx.get_root(),
        }
    }

    pub(crate) fn focus(&mut self, path: Path<'a>) {
        todo!()
        //* transform = TSTransform::from_translation(
        //     node_ctx
        //         .child_subtree_ctx()
        //         .map(|ctx| ctx.subtree().get_subtree_input_point())
        //         .flatten()
        //         .map(|point| point.to_vec2() + Vec2::new(-1500., -900.))
        //         .unwrap_or_default(),
        // )
        // .inverse();
    }
}

pub(crate) struct SubtreeViewContext<'a> {
    tree_view_context: TreeViewContext<'a>,
    path: Path<'a>,
}

impl<'a> SubtreeViewContext<'a> {
    pub(crate) fn child(&self, key: Vec<u8>) -> SubtreeViewContext<'a> {
        SubtreeViewContext {
            tree_view_context: self.tree_view_context,
            path: self.path.child(key),
        }
    }

    pub(crate) fn focus_child(&self, key: Vec<u8>) {}

    pub(crate) fn path(&self) -> Path<'a> {
        self.path
    }
}
