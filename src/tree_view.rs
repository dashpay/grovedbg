mod element_view;
mod subtree_view;

use std::collections::BTreeMap;

use eframe::{
    egui::{self, Context, Rect},
    emath::TSTransform,
};
pub(crate) use element_view::{ElementOrPlaceholder, ElementView};
pub(crate) use subtree_view::SubtreeElements;
use subtree_view::SubtreeView;

use crate::{
    bus::{CommandBus, UserAction},
    path_ctx::{Path, PathCtx},
    profiles::{ActiveProfileSubtreeContext, RootActiveProfileContext},
    tree_data::TreeData,
    FocusedSubree,
};

pub(crate) const NODE_WIDTH: f32 = 300.;

pub(crate) struct TreeView<'pa> {
    transform: TSTransform,
    pub(super) subtrees: BTreeMap<Path<'pa>, SubtreeView<'pa>>,
    path_ctx: &'pa PathCtx,
}

impl<'pa> TreeView<'pa> {
    pub(crate) fn new(path_ctx: &'pa PathCtx) -> Self {
        let root_subtree = SubtreeView::new(path_ctx.get_root());
        let mut subtrees = BTreeMap::new();
        subtrees.insert(path_ctx.get_root(), root_subtree);

        Self {
            transform: TSTransform::default(),
            subtrees,
            path_ctx,
        }
    }

    pub(crate) fn draw<'pf, 'b, 'af>(
        &mut self,
        ui: &mut egui::Ui,
        bus: &'b CommandBus<'pa>,
        merk_panel_width: f32,
        root_profile_ctx: RootActiveProfileContext<'pf>,
        tree_data: &mut TreeData<'pa>,
        focused_subtree: &'af Option<FocusedSubree<'pa>>,
    ) {
        let (id, rect) = ui.allocate_space(ui.available_size());

        let pointer_response = ui.interact(rect, id, egui::Sense::click_and_drag());

        let transform_before = self.transform;

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

                // Zoom in on pointer:
                self.transform = self.transform
                    * TSTransform::from_translation(pointer_in_layer.to_vec2())
                    * TSTransform::from_scaling(zoom_delta)
                    * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                // Pan:
                self.transform = TSTransform::from_translation(pan_delta) * self.transform;
            }
        }

        if transform_before != self.transform {
            bus.user_action(UserAction::DropFocus);
        }

        if let Some(FocusedSubree { path, key }) = focused_subtree {
            // Show focused subtree
            path.for_segments(|segments_iter| {
                let mut current_path = path.get_root();
                for segment in segments_iter {
                    let subtree_data = tree_data.get(current_path);
                    subtree_data.visible_keys.insert(segment.bytes().to_vec());
                    current_path = current_path.child(segment.bytes().to_vec());
                }
                if let Some(k) = key {
                    if let Some(s) = self.subtrees.get_mut(&current_path) {
                        s.scroll_to(k, tree_data);
                    }
                }
            });

            let context = ui.ctx();

            let self_pos = context.memory(|mem| mem.area_rect(path.id()).map(|rect| rect.center()));
            let root_pos =
                context.memory(|mem| mem.area_rect(path.get_root().id()).map(|rect| rect.center()));

            if let (Some(self_pos), Some(root_pos)) = (self_pos, root_pos) {
                self.transform =
                    TSTransform::from_translation(self_pos.to_vec2() - root_pos.to_vec2()).inverse();
            }
        }

        let subtree_view_ctx =
            SubtreeViewContext::new_root(ui.ctx().clone(), self.transform, rect, root_profile_ctx, bus);

        if let Some(mut root) = self.subtrees.remove(&self.path_ctx.get_root()) {
            root.draw(
                subtree_view_ctx,
                ui,
                tree_data,
                &mut self.subtrees,
                None,
                merk_panel_width,
            );
            self.subtrees.insert(self.path_ctx.get_root(), root);
        };
    }
}

pub(crate) struct SubtreeViewContext<'pf, 'pa, 'b> {
    transform: TSTransform,
    rect: Rect,
    context: Context,
    profile_ctx: ActiveProfileSubtreeContext<'pf>,
    bus: &'b CommandBus<'pa>,
}

impl<'pf, 'pa, 'b> SubtreeViewContext<'pf, 'pa, 'b> {
    pub(crate) fn new_root(
        context: Context,
        transform: TSTransform,
        rect: Rect,
        root_profile_ctx: RootActiveProfileContext<'pf>,
        bus: &'b CommandBus<'pa>,
    ) -> Self {
        Self {
            transform,
            rect,
            context,
            profile_ctx: root_profile_ctx.into_inner(),
            bus,
        }
    }

    pub(crate) fn child(&mut self, key: Vec<u8>) -> SubtreeViewContext<'pf, 'pa, 'b> {
        SubtreeViewContext {
            rect: self.rect,
            transform: self.transform,
            context: self.context.clone(),
            profile_ctx: self.profile_ctx.child(key),
            bus: self.bus,
        }
    }

    pub(crate) fn element_view_context<'sc>(
        &'sc mut self,
        path: Path<'pa>,
    ) -> ElementViewContext<'sc, 'pa, 'pf, 'b> {
        ElementViewContext {
            path,
            profile_ctx: &mut self.profile_ctx,
            bus: self.bus,
        }
    }
}

pub(crate) struct ElementViewContext<'af, 'pa, 'pf, 'b> {
    pub(crate) path: Path<'pa>,
    pub(crate) profile_ctx: &'af mut ActiveProfileSubtreeContext<'pf>,
    pub(crate) bus: &'b CommandBus<'pa>,
}

impl<'af, 'pa, 'pf, 'cs> ElementViewContext<'af, 'pa, 'pf, 'cs> {
    pub(crate) fn focus_child_subtree(&mut self, key: Vec<u8>) {
        self.bus
            .user_action(UserAction::FocusSubtree(self.path.child(key)));
    }

    pub(crate) fn focus(&mut self, path: Path<'pa>, key: Option<Vec<u8>>) {
        if let Some(key) = key {
            self.bus.user_action(UserAction::FocusSubtreeKey(path, key));
        } else {
            self.bus.user_action(UserAction::FocusSubtree(path));
        }
    }

    pub(crate) fn path(&self) -> Path<'pa> {
        self.path
    }

    pub(crate) fn profile_ctx(&self) -> &ActiveProfileSubtreeContext {
        &self.profile_ctx
    }
}
