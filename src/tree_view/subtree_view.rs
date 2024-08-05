use eframe::egui::{self, Align2, Order};

use super::TreeViewContext;
use crate::path_ctx::Path;

pub(crate) struct SubtreeView<'a> {
    path: Path<'a>,
}

impl<'a> SubtreeView<'a> {
    pub(crate) fn new(path: Path<'a>) -> Self {
        Self { path }
    }

    pub(crate) fn draw(&mut self, tree_view_ctx: TreeViewContext, ui: &mut egui::Ui) {
        let area_id = egui::Area::new(self.path.id())
            .order(Order::Background)
            .anchor(Align2::CENTER_CENTER, (0., 0.))
            .show(ui.ctx(), |area| {
                area.set_clip_rect(tree_view_ctx.transform.inverse() * tree_view_ctx.rect);
                area.label("test");
            })
            .response
            .layer_id;

        ui.ctx().transform_layer_shapes(area_id, *tree_view_ctx.transform);
    }
}
