mod element_view;
mod subtree_view;

use std::collections::BTreeMap;

use eframe::{
    egui::{self, Context, Rect},
    emath::TSTransform,
};
use grovedbg_types::{Key, NodeUpdate};
use subtree_view::{RoutingNodeUpdate, SubtreeView};

use crate::{
    path_ctx::{Path, PathCtx},
    CommandsSender,
};
pub(crate) use element_view::WrappedElement;

const NODE_WIDTH: f32 = 300.;

pub(crate) struct TreeView<'a> {
    transform: TSTransform,
    root_subtree: SubtreeView<'a>,
}

impl<'a> TreeView<'a> {
    pub(crate) fn new(commands_sender: CommandsSender, path_ctx: &'a PathCtx) -> Self {
        let mut root_subtree = SubtreeView::new(commands_sender, path_ctx.get_root(), None);
        root_subtree.show = true;

        Self {
            transform: TSTransform::default(),
            root_subtree,
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

        let mut tree_view_context = TreeViewContext::new(ui.ctx().clone(), &mut self.transform, rect);
        self.root_subtree.draw(&mut tree_view_context, ui, None);
    }
}

pub(crate) struct TreeViewContext<'t> {
    transform: &'t mut TSTransform,
    rect: Rect,
    context: Context,
}

impl<'t> TreeViewContext<'t> {
    pub(crate) fn new(context: Context, transform: &'t mut TSTransform, rect: Rect) -> Self {
        Self {
            transform,
            rect,
            context,
        }
    }

    pub(crate) fn focus<'a>(&mut self, path: Path<'a>) {
        let self_pos = self
            .context
            .memory(|mem| mem.area_rect(path.id()).map(|rect| rect.center()));
        let root_pos = self
            .context
            .memory(|mem| mem.area_rect(path.get_root().id()).map(|rect| rect.center()));

        if let (Some(self_pos), Some(root_pos)) = (self_pos, root_pos) {
            *self.transform =
                TSTransform::from_translation(self_pos.to_vec2() - root_pos.to_vec2()).inverse();
        }
    }
}

pub(crate) struct SubtreeViewContext<'a, 't, 'b, 'tc> {
    tree_view_context: &'tc mut TreeViewContext<'t>,
    path: Path<'a>,
    subtrees: &'b mut BTreeMap<Key, SubtreeView<'a>>,
}

impl<'a, 't, 'b, 'tc> SubtreeViewContext<'a, 't, 'b, 'tc> {
    pub(crate) fn focus_child(&mut self, key: Vec<u8>) {
        let child_path = self.path.child(key);
        self.tree_view_context.focus(child_path);
    }

    pub(crate) fn path(&self) -> Path<'a> {
        self.path
    }

    pub(crate) fn subtree_visibility_mut(&mut self, key: &[u8]) -> Option<&mut bool> {
        self.subtrees.get_mut(key).map(|s| &mut s.show)
    }
}
