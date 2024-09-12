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
    path_ctx::{Path, PathCtx},
    profiles::{ActiveProfileSubtreeContext, RootActiveProfileContext},
    tree_data::TreeData,
    CommandsSender, FocusedSubree,
};

pub(crate) const NODE_WIDTH: f32 = 300.;

pub(crate) struct TreeView<'pa> {
    transform: TSTransform,
    pub(super) auto_focus: Option<FocusedSubree<'pa>>,
    pub(super) subtrees: BTreeMap<Path<'pa>, SubtreeView<'pa>>,
    path_ctx: &'pa PathCtx,
    commands_sender: CommandsSender,
}

impl<'pa> TreeView<'pa> {
    pub(crate) fn new(commands_sender: CommandsSender, path_ctx: &'pa PathCtx) -> Self {
        let root_subtree = SubtreeView::new(path_ctx.get_root());
        let mut subtrees = BTreeMap::new();
        subtrees.insert(path_ctx.get_root(), root_subtree);

        Self {
            transform: TSTransform::default(),
            auto_focus: None,
            subtrees,
            path_ctx,
            commands_sender,
        }
    }

    pub(crate) fn draw<'pf>(
        &mut self,
        ui: &mut egui::Ui,
        merk_panel_width: f32,
        root_profile_ctx: RootActiveProfileContext<'pf>,
        tree_data: &mut TreeData<'pa>,
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
            self.auto_focus = None;
        }

        if let Some(FocusedSubree { path, key }) = &self.auto_focus {
            // Show focused subtree
            path.for_segments(|segments_iter| {
                let mut current_path = path.get_root();
                for segment in segments_iter {
                    tree_data.get(current_path);
                    current_path = current_path.child(segment.bytes().to_vec());
                    segment.set_visible();
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

        let subtree_view_ctx = SubtreeViewContext::new_root(
            ui.ctx().clone(),
            &mut self.auto_focus,
            self.transform,
            rect,
            root_profile_ctx,
            &self.commands_sender,
        );

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

pub(crate) struct SubtreeViewContext<'af, 'pf, 'pa, 'cs> {
    auto_focus: &'af mut Option<FocusedSubree<'pa>>,
    transform: TSTransform,
    rect: Rect,
    context: Context,
    profile_ctx: ActiveProfileSubtreeContext<'pf>,
    commands_sender: &'cs CommandsSender,
}

impl<'af, 'pf, 'pa, 'cs> SubtreeViewContext<'af, 'pf, 'pa, 'cs> {
    pub(crate) fn new_root(
        context: Context,
        auto_focus: &'af mut Option<FocusedSubree<'pa>>,
        transform: TSTransform,
        rect: Rect,
        root_profile_ctx: RootActiveProfileContext<'pf>,
        commands_sender: &'cs CommandsSender,
    ) -> Self {
        Self {
            auto_focus,
            transform,
            rect,
            context,
            profile_ctx: root_profile_ctx.into_inner(),
            commands_sender,
        }
    }

    pub(crate) fn drop_focus(&mut self) {
        *self.auto_focus = None;
    }

    pub(crate) fn child<'s>(&'s mut self, key: Vec<u8>) -> SubtreeViewContext<'s, 'pf, 'pa, 'cs> {
        SubtreeViewContext {
            auto_focus: &mut self.auto_focus,
            rect: self.rect,
            transform: self.transform,
            context: self.context.clone(),
            profile_ctx: self.profile_ctx.child(key),
            commands_sender: self.commands_sender,
        }
    }

    pub(crate) fn element_view_context<'sc>(
        &'sc mut self,
        path: Path<'pa>,
    ) -> ElementViewContext<'sc, 'pa, 'pf, 'cs> {
        ElementViewContext {
            path,
            focus_subtree: self.auto_focus,
            profile_ctx: &mut self.profile_ctx,
            commands_sender: self.commands_sender,
        }
    }
}

pub(crate) struct ElementViewContext<'af, 'pa, 'pf, 'cs> {
    pub(crate) path: Path<'pa>,
    pub(crate) focus_subtree: &'af mut Option<FocusedSubree<'pa>>,
    pub(crate) profile_ctx: &'af mut ActiveProfileSubtreeContext<'pf>,
    pub(crate) commands_sender: &'cs CommandsSender,
}

impl<'af, 'pa, 'pf, 'cs> ElementViewContext<'af, 'pa, 'pf, 'cs> {
    pub(crate) fn focus_child_subtree(&mut self, key: Vec<u8>) {
        let child_path = self.path.child(key);
        *self.focus_subtree = Some(FocusedSubree {
            path: child_path,
            key: None,
        });
    }

    pub(crate) fn focus(&mut self, path: Path<'pa>, key: Option<Vec<u8>>) {
        *self.focus_subtree = Some(FocusedSubree { path, key });
    }

    pub(crate) fn path(&self) -> Path<'pa> {
        self.path
    }

    pub(crate) fn profile_ctx(&self) -> &ActiveProfileSubtreeContext {
        &self.profile_ctx
    }
}
