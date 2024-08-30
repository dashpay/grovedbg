//! Subtrees paths manipulation and storage module.
//! Path identification, comparison, display and shared subtrees properties like
//! visibility -- all goes through `PathCtx`.

use std::{
    cell::RefCell,
    fmt::{self, Write},
    hash::{Hash, Hasher},
    iter,
};

use eframe::egui::{self, Label};
use slab::Slab;

use crate::{
    bytes_utils::{bytes_by_display_variant, BytesDisplayVariant},
    profiles::ActiveProfileSubtreeContext,
};

type SegmentId = usize;

#[derive(Default)]
pub(crate) struct PathCtx {
    slab: RefCell<Slab<PathSegment>>,
    root_children_slab_ids: RefCell<Vec<SegmentId>>,
    selected_for_query: RefCell<Option<SelectedForQuery>>,
}

#[derive(Clone, Copy)]
enum SelectedForQuery {
    Root,
    Subtree(SegmentId),
}

impl fmt::Debug for PathCtx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("slab")
    }
}

impl PathCtx {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_root(&self) -> Path {
        Path {
            head_slab_id: None,
            ctx: self,
        }
    }

    pub fn add_path(&self, path: Vec<Vec<u8>>) -> Path {
        let mut current_path = self.get_root();
        for segment in path.into_iter() {
            current_path = current_path.child(segment);
        }
        current_path
    }

    pub fn add_iter<S, I>(&self, path: I) -> Path
    where
        I: IntoIterator<Item = S>,
        S: AsRef<[u8]>,
    {
        let mut current_path = self.get_root();
        for segment in path.into_iter() {
            current_path = current_path.child(segment.as_ref().to_vec());
        }
        current_path
    }

    pub fn get_selected_for_query(&self) -> Option<Path> {
        self.selected_for_query.borrow().map(|id| Path {
            head_slab_id: match id {
                SelectedForQuery::Root => None,
                SelectedForQuery::Subtree(s) => Some(s),
            },
            ctx: self,
        })
    }
}

pub(crate) struct PathSegment {
    parent_slab_id: Option<SegmentId>,
    children_slab_ids: Vec<SegmentId>,
    bytes: Vec<u8>,
    display: BytesDisplayVariant,
    level: usize,
    visible: RefCell<bool>,
}

impl PathSegment {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn view_by_display(&self) -> String {
        bytes_by_display_variant(&self.bytes, &self.display)
    }

    pub fn set_visible(&self) {
        *self.visible.borrow_mut() = true;
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Path<'c> {
    head_slab_id: Option<SegmentId>,
    ctx: &'c PathCtx,
}

impl Hash for Path<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.for_segments(|segments_iter| segments_iter.for_each(|seg| state.write(seg.bytes())));
    }
}

impl PartialEq for Path<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.head_slab_id == other.head_slab_id
    }
}

impl PartialOrd for Path<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            self.level()
                .cmp(&other.level())
                .then_with(|| self.head_slab_id.cmp(&other.head_slab_id)),
        )
    }
}

impl Eq for Path<'_> {}

// TODO: comparing paths of different slabs makes no sence
impl Ord for Path<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<'c> Path<'c> {
    pub fn get_ctx(&self) -> &'c PathCtx {
        self.ctx
    }

    pub fn for_visible_mut<T>(&self, f: impl FnOnce(&mut bool) -> T) -> Option<T> {
        self.head_slab_id.map(|id| {
            let slab = self.ctx.slab.borrow();
            let mut segment_visible = slab[id].visible.borrow_mut();
            f(&mut segment_visible)
        })
    }

    pub fn visible(&self) -> bool {
        self.for_last_segment(|s| *s.visible.borrow()).unwrap_or_default()
    }

    pub fn get_root(&self) -> Path<'c> {
        Path {
            head_slab_id: None,
            ctx: self.ctx,
        }
    }

    pub fn level(&self) -> usize {
        self.for_last_segment(|k| k.level).unwrap_or_default()
    }

    pub fn parent(&self) -> Option<Path<'c>> {
        self.head_slab_id.map(|id| {
            let slab = self.ctx.slab.borrow();
            let segment = &slab[id];
            Path {
                head_slab_id: segment.parent_slab_id,
                ctx: self.ctx,
            }
        })
    }

    pub fn parent_with_key(&self) -> Option<(Path<'c>, Vec<u8>)> {
        self.head_slab_id.map(|id| {
            let slab = self.ctx.slab.borrow();
            let segment = &slab[id];
            (
                Path {
                    head_slab_id: segment.parent_slab_id,
                    ctx: self.ctx,
                },
                segment.bytes().to_vec(),
            )
        })
    }

    pub fn child(&self, key: Vec<u8>) -> Path<'c> {
        let slab = self.ctx.slab.borrow();
        let mut root_children = self.ctx.root_children_slab_ids.borrow_mut();
        let level = self.head_slab_id.map(|id| slab[id].level).unwrap_or_default();

        if let Some(child_segment_id) = {
            let children_vec = self
                .head_slab_id
                .map(|id| &slab[id].children_slab_ids)
                .unwrap_or(&root_children);
            children_vec.iter().find(|id| &slab[**id].bytes == &key).copied()
        } {
            Path {
                head_slab_id: Some(child_segment_id),
                ctx: self.ctx,
            }
        } else {
            drop(slab);
            let mut slab = self.ctx.slab.borrow_mut();
            let child_segment_id = slab.insert(PathSegment {
                parent_slab_id: self.head_slab_id,
                children_slab_ids: Vec::new(),
                display: BytesDisplayVariant::guess(&key),
                bytes: key,
                level: level + 1,
                visible: RefCell::new(false),
            });
            let children_vec = self
                .head_slab_id
                .map(|id| &mut slab[id].children_slab_ids)
                .unwrap_or(&mut root_children);
            children_vec.push(child_segment_id);
            Path {
                head_slab_id: Some(child_segment_id),
                ctx: self.ctx,
            }
        }
    }

    pub fn for_last_segment<F, T>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&PathSegment) -> T,
    {
        self.head_slab_id.map(|id| {
            let slab = self.ctx.slab.borrow();
            f(&slab[id])
        })
    }

    pub fn update_display_variant(&self, display: BytesDisplayVariant) {
        self.head_slab_id.into_iter().for_each(|id| {
            let mut slab = self.ctx.slab.borrow_mut();
            let segment = &mut slab[id];
            segment.display = display;
        });
    }

    pub fn get_display_variant(&self) -> Option<BytesDisplayVariant> {
        self.head_slab_id.map(|id| {
            let mut slab = self.ctx.slab.borrow_mut();
            let segment = &mut slab[id];
            segment.display
        })
    }

    pub fn for_segments<F, T>(&self, f: F) -> T
    where
        F: FnOnce(SegmentsIter) -> T,
    {
        let slab = self.ctx.slab.borrow();
        let mut ids = Vec::new();
        let mut current_id = self.head_slab_id;
        while let Some(current_segment) = current_id.map(|id| &slab[id]) {
            ids.push(current_segment);
            current_id = current_segment.parent_slab_id;
        }

        let segments_iter: SegmentsIter = ids.into_iter().rev();
        f(segments_iter)
    }

    pub fn to_vec(&self) -> Vec<Vec<u8>> {
        let slab = self.ctx.slab.borrow();
        let mut path = Vec::new();
        let mut current_id = self.head_slab_id;
        while let Some(current_segment) = current_id.map(|id| &slab[id]) {
            path.push(current_segment.bytes.clone());
            current_id = current_segment.parent_slab_id;
        }

        path.reverse();
        path
    }

    pub fn select_for_query(&self) {
        *self.ctx.selected_for_query.borrow_mut() = Some(
            self.head_slab_id
                .map(SelectedForQuery::Subtree)
                .unwrap_or(SelectedForQuery::Root),
        );
    }

    pub fn id(&self) -> egui::Id {
        egui::Id::new(self.head_slab_id.map(|x| x + 1).unwrap_or_default())
    }
}

type SegmentsIter<'c> = iter::Rev<std::vec::IntoIter<&'c PathSegment>>;

pub(crate) fn full_path_display_iter<'c>(
    segments_iter: SegmentsIter<'c>,
    profile_ctx: &'c ActiveProfileSubtreeContext,
) -> impl Iterator<Item = String> + 'c + Clone + ExactSizeIterator + DoubleEndedIterator {
    segments_iter
        .map(|s| s.view_by_display())
        .zip(profile_ctx.path_segments_aliases().iter())
        .map(|(segment_variant, profile_alias)| {
            profile_alias
                .as_ref()
                .map(|alias| alias.clone())
                .unwrap_or(segment_variant)
        })
}

pub(crate) fn full_path_display<I>(mut full_path_iter: I) -> String
where
    I: Iterator<Item = String> + ExactSizeIterator + DoubleEndedIterator,
{
    if full_path_iter.len() > 0 {
        let mut buffer = String::from("[");
        let last = full_path_iter.next_back().expect("checked length");
        full_path_iter.for_each(|s| {
            write!(&mut buffer, "{s}, ").ok();
        });
        write!(&mut buffer, "{last}]").ok();
        buffer
    } else {
        "Root tree".to_owned()
    }
}

pub(crate) fn path_label(ui: &mut egui::Ui, path: Path, profile_ctx: &ActiveProfileSubtreeContext) {
    path.for_segments(|segments_iter| {
        let mut path_segments_iter = full_path_display_iter(segments_iter, profile_ctx);
        let full_path_iter = path_segments_iter.clone();

        let text = if path_segments_iter.len() == 0 {
            "Root subtree".to_owned()
        } else {
            if path_segments_iter.len() < 3 {
                let mut buffer = String::from("[");
                let last = path_segments_iter.next_back().expect("checked length");
                path_segments_iter.for_each(|s| {
                    write!(&mut buffer, "{}, ", s).ok();
                });
                write!(&mut buffer, "{}]", last).ok();
                buffer
            } else {
                let last = path_segments_iter.next_back().expect("checked length");
                let pre_last = path_segments_iter.next_back().expect("checked length");
                format!("[..., {pre_last}, {last}]")
            }
        };

        ui.add(Label::new(text).truncate())
            .on_hover_text(full_path_display(full_path_iter));
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_reuse() {
        let ctx = PathCtx::new();
        let sub_1 = ctx.get_root().child(b"key1".to_vec());
        let sub_2 = sub_1.child(b"key2".to_vec());
        let sub_2_again = sub_1.child(b"key2".to_vec());
        assert_eq!(sub_2.head_slab_id, sub_2_again.head_slab_id);
    }

    #[test]
    fn collect_path() {
        let ctx = PathCtx::new();
        let path = ctx
            .get_root()
            .child(b"key1".to_vec())
            .child(b"key2".to_vec())
            .child(b"key3".to_vec())
            .child(b"key4".to_vec());
        let mut path_vec = Vec::new();
        path.for_segments(|segments_iter| {
            path_vec = segments_iter.map(|segment| segment.bytes().to_vec()).collect()
        });
        assert_eq!(path_vec, vec![b"key1", b"key2", b"key3", b"key4"]);
        assert_eq!(path.level(), 4);
    }

    #[test]
    fn collect_for_root() {
        let ctx = PathCtx::new();
        let path = ctx.get_root();
        let mut path_vec = Vec::new();
        path.for_segments(|segments_iter| {
            path_vec = segments_iter.map(|segment| segment.bytes().to_vec()).collect()
        });
        assert_eq!(path_vec, Vec::<Vec<u8>>::new());
        assert_eq!(path.level(), 0);
    }
}
