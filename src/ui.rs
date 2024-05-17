mod common;
mod node;
mod tree;

pub(crate) use common::DisplayVariant;
use eframe::egui;
use strum::IntoEnumIterator;
pub(crate) use tree::TreeDrawer;

use self::node::element_to_color;
use crate::model::Element;

pub(crate) fn draw_legend(ui: &mut egui::Ui) {
    egui::Area::new(egui::Id::new("legend"))
        .anchor(egui::Align2::RIGHT_TOP, [-20.0, 50.0])
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::default()
                .rounding(egui::Rounding::same(4.0))
                .inner_margin(egui::Margin::same(8.0))
                .stroke(ui.ctx().style().visuals.window_stroke)
                .fill(ui.style().visuals.panel_fill)
                .show(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    Element::iter().for_each(|element| {
                        ui.label(
                            egui::RichText::new(element.as_ref()).color(element_to_color(&element)),
                        );
                    });
                });
        });
}
