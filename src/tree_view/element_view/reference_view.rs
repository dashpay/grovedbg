use std::{borrow::Cow, cmp, fmt::Write};

use eframe::egui::{self, Color32, Painter, Pos2, Stroke, Vec2};
use grovedb_epoch_based_storage_flags::StorageFlags;
use grovedbg_types::ReferencePath;

use crate::{
    bytes_utils::{binary_label, bytes_by_display_variant, BytesDisplayVariant},
    path_ctx::{path_label, Path},
    theme::{bidi_reference_line_color, reference_line_color},
    tree_data::SubtreeDataMap,
    tree_view::ElementViewContext,
};

const REFERENCE_LINE_TOP_MARGIN: f32 = 50.;

pub(super) fn draw_reference(
    ui: &mut egui::Ui,
    element_view_context: &mut ElementViewContext,
    key: &[u8],
    reference: &ReferencePath,
    flags: Option<&[u8]>,
    show_details: &mut bool,
    flags_display: &mut BytesDisplayVariant,
    subtrees_map: &SubtreeDataMap,
) -> Result<(), ReferenceError> {
    let color = reference_line_color(ui.ctx());
    draw_reference_internal(
        ui,
        element_view_context,
        key,
        reference,
        flags,
        show_details,
        flags_display,
        subtrees_map,
        None,
        color,
    )
}

pub(super) fn draw_bidi_reference(
    ui: &mut egui::Ui,
    element_view_context: &mut ElementViewContext,
    key: &[u8],
    reference: &ReferencePath,
    slot_idx: u8,
    cascade_on_update: bool,
    flags: Option<&[u8]>,
    show_details: &mut bool,
    flags_display: &mut BytesDisplayVariant,
    subtrees_map: &SubtreeDataMap,
) -> Result<(), ReferenceError> {
    let color = bidi_reference_line_color(ui.ctx());
    let bidi_details = BidiDetails {
        cascade_on_update,
        slot_idx,
    };
    draw_reference_internal(
        ui,
        element_view_context,
        key,
        reference,
        flags,
        show_details,
        flags_display,
        subtrees_map,
        Some(bidi_details),
        color,
    )
}

fn draw_reference_internal(
    ui: &mut egui::Ui,
    element_view_context: &mut ElementViewContext,
    key: &[u8],
    reference: &ReferencePath,
    flags: Option<&[u8]>,
    show_details: &mut bool,
    flags_display: &mut BytesDisplayVariant,
    subtrees_map: &SubtreeDataMap,
    bidi_details: Option<BidiDetails>, // todo: refactor
    color: Color32,
) -> Result<(), ReferenceError> {
    let (referenced_path, referenced_key) =
        get_absolute_path_key(element_view_context.path(), key, reference)?;

    let is_self_reference = referenced_path == element_view_context.path();

    ui.horizontal(|line| {
        if line
            .button(egui_phosphor::regular::LIST)
            .on_hover_text("Show reference definition (ref path type)")
            .clicked()
        {
            *show_details = !*show_details;
        }

        if line
            .button(egui_phosphor::regular::MAGNIFYING_GLASS)
            .on_hover_text("Focus on referenced subtree")
            .clicked()
        {
            element_view_context.focus(referenced_path, Some(referenced_key.to_vec()));
        }

        if is_self_reference {
            line.label("This subtree");
        } else {
            path_label(
                line,
                referenced_path.child(referenced_key.to_vec()),
                &element_view_context
                    .profile_ctx()
                    .root_context()
                    .fast_forward(referenced_path),
            );
        }
    });

    ui.horizontal(|line| {
        line.label(format!(
            "Key: {}",
            bytes_by_display_variant(
                &referenced_key,
                &referenced_path
                    .child(referenced_key.to_vec())
                    .get_display_variant()
                    .unwrap_or_else(|| BytesDisplayVariant::guess(&referenced_key))
            )
        ));
    });

    if let Some(flags) = flags {
        ui.horizontal(|line| {
            line.label("Flags:");
            if let Some(storage_flags) = StorageFlags::deserialize(&flags).ok().flatten() {
                line.label(format!("{storage_flags}"));
            } else {
                binary_label(line, flags, flags_display);
            }
        });
    }

    if *show_details {
        draw_reference_details(ui, reference, bidi_details);
    }

    // Draw reference arrow
    if let Some((rect_from, rect_to)) = (!is_self_reference
        && referenced_path
            .parent_with_key()
            .and_then(|(referenced_path_parent, referenced_path_parent_key)| {
                subtrees_map
                    .get(&referenced_path_parent)
                    .map(|s| s.borrow().visible_keys.contains(&referenced_path_parent_key))
            })
            .unwrap_or_default())
    .then(|| {
        ui.memory(|mem| {
            mem.area_rect(element_view_context.path().id())
                .and_then(|rect_from| {
                    mem.area_rect(referenced_path.id())
                        .map(|rect_to| (rect_from, rect_to))
                })
        })
    })
    .flatten()
    {
        let painter = ui.painter();

        fn adjust_y(top_y: f32, mut side_center: Pos2) -> Pos2 {
            side_center.y = cmp::min_by(side_center.y, top_y + REFERENCE_LINE_TOP_MARGIN, |a, b| {
                a.partial_cmp(b).unwrap_or(cmp::Ordering::Equal)
            });
            side_center
        }

        let (from, to) = {
            if rect_from.center().x < rect_to.center().x {
                // Left to right arrow
                (
                    adjust_y(rect_from.center_top().y, rect_from.right_center()),
                    adjust_y(rect_to.center_top().y, rect_to.left_center()),
                )
            } else {
                // Right to left arrow
                (
                    adjust_y(rect_from.center_top().y, rect_from.left_center()),
                    adjust_y(rect_to.center_top().y, rect_to.right_center()),
                )
            }
        };
        arrow(painter, from, to - from, Stroke { width: 1.0, color });
    }

    Ok(())
}

fn arrow(painter: &Painter, origin: Pos2, vec: Vec2, stroke: impl Into<Stroke>) {
    use egui::emath::*;
    let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
    let tip_length = 10.;
    let tip = origin + vec;
    let dir = vec.normalized();
    let stroke = stroke.into();
    painter.line_segment([origin, tip], stroke);
    painter.line_segment([tip, tip - tip_length * (rot * dir)], stroke);
    painter.line_segment([tip, tip - tip_length * (rot.inverse() * dir)], stroke);
}

struct BidiDetails {
    cascade_on_update: bool,
    slot_idx: u8,
}

fn draw_reference_details(ui: &mut egui::Ui, reference: &ReferencePath, bidi_details: Option<BidiDetails>) {
    if bidi_details.is_some() {
        ui.label("Bidirectional reference");
    }
    match reference {
        ReferencePath::AbsolutePathReference { path, .. } => {
            ui.label("Absolute path");
            ui.label(format!("Path: {}", hex_array(path)));
        }
        ReferencePath::UpstreamRootHeightReference {
            n_keep, path_append, ..
        } => {
            ui.label("Upstream root height");
            ui.label(format!("N keep: {n_keep}"));
            ui.label(format!("Path append: {}", hex_array(path_append)));
        }
        ReferencePath::UpstreamRootHeightWithParentPathAdditionReference {
            n_keep, path_append, ..
        } => {
            ui.label("Upstream root height with parent path addition");
            ui.label(format!("N keep: {n_keep}"));
            ui.label(format!("Path append: {}", hex_array(path_append)));
        }
        ReferencePath::UpstreamFromElementHeightReference {
            n_remove,
            path_append,
            ..
        } => {
            ui.label("Upstream from element height");
            ui.label(format!("N remove: {n_remove}"));
            ui.label(format!("Path append: {}", hex_array(path_append)));
        }
        ReferencePath::CousinReference { swap_parent, .. } => {
            ui.label("Cousin");
            ui.label(format!("Swap parent: {}", hex::encode(swap_parent)));
        }
        ReferencePath::RemovedCousinReference { swap_parent, .. } => {
            ui.label("Removed cousin");
            ui.label(format!("Swap parent: {}", hex_array(swap_parent)));
        }
        ReferencePath::SiblingReference { sibling_key, .. } => {
            ui.label("Sibling");
            ui.label(format!("Sibling key: {}", hex::encode(sibling_key)));
        }
    }

    if let Some(bidi) = bidi_details {
        ui.label(format!("Cascade on update: {}", bidi.cascade_on_update));
        ui.label(format!("Backward reference slot: {}", bidi.slot_idx));
    }
}

fn hex_array(byte_slices: &[impl AsRef<[u8]>]) -> String {
    if byte_slices.is_empty() {
        return "[]".to_owned();
    }
    let mut buf = String::from("[");
    let mut iter = byte_slices.into_iter().map(|s| s.as_ref());
    let last = iter.next_back().expect("checked above");
    for slice in iter {
        write!(&mut buf, "{}", hex::encode(slice)).ok();
        write!(&mut buf, ", ").ok();
    }
    write!(&mut buf, "{}", hex::encode(last)).ok();
    write!(&mut buf, "]").ok();

    buf
}

pub(super) struct ReferenceError(pub(super) &'static str);

fn get_absolute_path_key<'a, 'b>(
    current_path: Path<'a>,
    current_key: &'b [u8],
    reference: &'b ReferencePath,
) -> Result<(Path<'a>, Cow<'b, [u8]>), ReferenceError> {
    match reference {
        ReferencePath::AbsolutePathReference { path, .. } => {
            let mut path = path.iter();
            let key = path
                .next_back()
                .ok_or_else(|| ReferenceError("empty absolute reference"))?;
            Ok((current_path.get_ctx().add_iter(path), key.into()))
        }
        ReferencePath::UpstreamRootHeightReference {
            n_keep, path_append, ..
        } => {
            if (*n_keep as usize) > current_path.level() {
                return Err(ReferenceError("current path is to short to keep enough segments"));
            }
            let to_remove = current_path.level() - (*n_keep as usize);
            let mut shrinked_path = current_path;
            for _ in 0..to_remove {
                shrinked_path = shrinked_path.parent().expect("checked above");
            }

            for segment in path_append {
                shrinked_path = shrinked_path.child(segment.to_owned());
            }

            shrinked_path
                .parent_with_key()
                .map(|(path, key)| (path, key.into()))
                .ok_or_else(|| ReferenceError("the computed absolute path is empty"))
        }
        ReferencePath::UpstreamRootHeightWithParentPathAdditionReference {
            n_keep, path_append, ..
        } => {
            if (*n_keep as usize) > current_path.level() {
                return Err(ReferenceError("current path is to short to keep enough segments"));
            }
            let to_remove = current_path.level() - (*n_keep as usize);
            let mut shrinked_path = current_path;
            for _ in 0..to_remove {
                shrinked_path = shrinked_path.parent().expect("checked above");
            }

            for segment in path_append {
                shrinked_path = shrinked_path.child(segment.to_owned());
            }

            current_path.for_last_segment(|s| shrinked_path = shrinked_path.child(s.bytes().to_vec()));

            shrinked_path
                .parent_with_key()
                .map(|(path, key)| (path, key.into()))
                .ok_or_else(|| ReferenceError("the computed absolute path is empty"))
        }
        ReferencePath::UpstreamFromElementHeightReference {
            n_remove,
            path_append,
            ..
        } => {
            if (*n_remove as usize) > current_path.level() {
                return Err(ReferenceError(
                    "current path is to short to remove enough segments",
                ));
            }

            let mut shrinked_path = current_path;

            for _ in 0..(*n_remove as usize) {
                shrinked_path = shrinked_path.parent().expect("checked above");
            }

            for segment in path_append {
                shrinked_path = shrinked_path.child(segment.to_owned());
            }

            shrinked_path
                .parent_with_key()
                .map(|(path, key)| (path, key.into()))
                .ok_or_else(|| ReferenceError("the computed absolute path is empty"))
        }
        ReferencePath::CousinReference { swap_parent, .. } => Ok((
            current_path
                .parent()
                .ok_or_else(|| ReferenceError("no parent to swap"))?
                .child(swap_parent.to_vec()),
            current_key.into(),
        )),
        ReferencePath::RemovedCousinReference { swap_parent, .. } => {
            let mut new_path = current_path
                .parent()
                .ok_or_else(|| ReferenceError("can't swap parent of an empty path"))?;
            for segment in swap_parent {
                new_path = new_path.child(segment.to_vec());
            }
            Ok((new_path, current_key.into()))
        }
        ReferencePath::SiblingReference { sibling_key, .. } => Ok((current_path, sibling_key.into())),
    }
}
