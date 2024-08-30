use eframe::egui::{Color32, Context};
use grovedbg_types::Element;

use crate::tree_view::ElementOrPlaceholder;

const SUBTREE_COLOR_LIGHT: Color32 = Color32::from_rgb(180, 120, 0);
const SUBTREE_COLOR_DARK: Color32 = Color32::GOLD;

const ERROR_COLOR_DARK: Color32 = Color32::RED;
const ERROR_COLOR_LIGHT: Color32 = Color32::DARK_RED;

const REFERENCE_COLOR_LIGHT: Color32 = Color32::DARK_BLUE;
const REFERENCE_COLOR_DARK: Color32 = Color32::LIGHT_BLUE;

pub(crate) fn element_to_color(ctx: &Context, element: &ElementOrPlaceholder) -> Color32 {
    if ctx.style().visuals.dark_mode {
        // Dark theme
        match element {
            ElementOrPlaceholder::Placeholder => Color32::DARK_RED,
            ElementOrPlaceholder::Element(Element::Item { .. }) => Color32::GRAY,
            ElementOrPlaceholder::Element(Element::SumItem { .. }) => Color32::DARK_GREEN,
            ElementOrPlaceholder::Element(Element::Subtree { .. }) => SUBTREE_COLOR_DARK,
            ElementOrPlaceholder::Element(Element::Sumtree { .. }) => Color32::GREEN,
            ElementOrPlaceholder::Element(Element::Reference(..)) => REFERENCE_COLOR_DARK,
        }
    } else {
        // Light theme
        match element {
            ElementOrPlaceholder::Placeholder => Color32::DARK_RED,
            ElementOrPlaceholder::Element(Element::Item { .. }) => Color32::GRAY,
            ElementOrPlaceholder::Element(Element::SumItem { .. }) => Color32::DARK_GREEN,
            ElementOrPlaceholder::Element(Element::Subtree { .. }) => SUBTREE_COLOR_LIGHT,
            ElementOrPlaceholder::Element(Element::Sumtree { .. }) => Color32::from_rgb(0, 150, 0),
            ElementOrPlaceholder::Element(Element::Reference(..)) => REFERENCE_COLOR_LIGHT,
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

pub(crate) fn reference_line_color(ctx: &Context) -> Color32 {
    if ctx.style().visuals.dark_mode {
        REFERENCE_COLOR_DARK
    } else {
        REFERENCE_COLOR_LIGHT
    }
}

pub(crate) fn input_error_color(ctx: &Context) -> Color32 {
    if ctx.style().visuals.dark_mode {
        ERROR_COLOR_DARK
    } else {
        ERROR_COLOR_LIGHT
    }
}
