use eframe::egui::{Color32, Context};
use grovedbg_types::Element;

use super::element_view::WrappedElement;

const SUBTREE_COLOR_LIGHT: Color32 = Color32::from_rgb(200, 150, 0);
const SUBTREE_COLOR_DARK: Color32 = Color32::GOLD;

pub(crate) fn element_to_color(ctx: &Context, element: &WrappedElement) -> Color32 {
    if ctx.style().visuals.dark_mode {
        match element {
            WrappedElement::SubtreePlaceholder => Color32::DARK_RED,
            WrappedElement::Element(Element::Item { .. }) => Color32::GRAY,
            WrappedElement::Element(Element::SumItem { .. }) => Color32::DARK_GREEN,
            WrappedElement::Element(Element::Subtree { .. }) => SUBTREE_COLOR_DARK,
            WrappedElement::Element(Element::Sumtree { .. }) => Color32::GREEN,
            WrappedElement::Element(_) => Color32::DARK_BLUE,
        }
    } else {
        match element {
            WrappedElement::SubtreePlaceholder => Color32::DARK_RED,
            WrappedElement::Element(Element::Item { .. }) => Color32::GRAY,
            WrappedElement::Element(Element::SumItem { .. }) => Color32::DARK_GREEN,
            WrappedElement::Element(Element::Subtree { .. }) => SUBTREE_COLOR_LIGHT,
            WrappedElement::Element(Element::Sumtree { .. }) => Color32::from_rgb(0, 150, 0),
            WrappedElement::Element(_) => Color32::DARK_BLUE,
        }
    }
}

pub(crate) fn subtree_line_color(ctx: &Context) -> Color32 {
    if ctx.style().visuals.dark_mode {
        SUBTREE_COLOR_DARK
    } else {
        SUBTREE_COLOR_LIGHT
    }
}
