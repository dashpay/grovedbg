use std::{cell::RefCell, fmt, iter, ptr};

use slab::Slab;

use crate::ui::DisplayVariant;

type SegmentId = usize;

pub(crate) struct PathCtx {
    slab: RefCell<Slab<PathSegment>>,
}

impl fmt::Debug for PathCtx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("slab")
    }
}

impl PathCtx {
    pub fn new() -> Self {
        let mut slab = Slab::new();
        slab.insert(PathSegment {
            parent_slab_id: None,
            children_slab_ids: Vec::new(),
            bytes: Vec::new(),
            display: DisplayVariant::U8,
            level: 0,
        });
        PathCtx {
            slab: RefCell::new(slab),
        }
    }

    pub fn get_root(&self) -> PathTwo {
        PathTwo {
            head_slab_id: 0,
            ctx: self,
        }
    }
}

pub(crate) struct PathSegment {
    parent_slab_id: Option<SegmentId>,
    children_slab_ids: Vec<SegmentId>,
    bytes: Vec<u8>,
    display: DisplayVariant,
    level: usize,
}

impl PathSegment {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn display(&self) -> DisplayVariant {
        self.display
    }
}

#[derive(Debug)]
pub(crate) struct PathTwo<'c> {
    head_slab_id: SegmentId,
    ctx: &'c PathCtx,
}

impl PartialEq for PathTwo<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(&self.ctx, &other.ctx) && self.head_slab_id == other.head_slab_id
    }
}

impl PartialOrd for PathTwo<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        ptr::eq(&self.ctx, &other.ctx).then_some(self.head_slab_id.cmp(&other.head_slab_id))
    }
}

impl Eq for PathTwo<'_> {}

// TODO: comparing paths of different slabs makes no sence
impl Ord for PathTwo<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).expect("paths of different ctxes")
    }
}

impl<'c> PathTwo<'c> {
    pub fn level(&self) -> usize {
        self.for_last_segment(|k| k.level)
    }

    pub fn parent(&self) -> Option<PathTwo<'c>> {
        Some(PathTwo {
            head_slab_id: self.for_last_segment(|s| s.parent_slab_id)?,
            ctx: self.ctx,
        })
    }

    pub fn child(&self, key: &[u8]) -> PathTwo<'c> {
        if let Some(child_segment_id) = {
            let slab = self.ctx.slab.borrow();
            let segment = slab.get(self.head_slab_id).expect("ids must be valid");
            segment
                .children_slab_ids
                .iter()
                .find(|id| slab.get(**id).expect("ids must be valid").bytes == key)
                .copied()
        } {
            PathTwo {
                head_slab_id: child_segment_id,
                ctx: self.ctx,
            }
        } else {
            let mut slab = self.ctx.slab.borrow_mut();
            let level = slab[self.head_slab_id].level;
            let child_segment_id = slab.insert(PathSegment {
                parent_slab_id: (self.head_slab_id != 0).then_some(self.head_slab_id),
                children_slab_ids: Vec::new(),
                bytes: key.to_vec(),
                display: DisplayVariant::guess(key),
                level: level + 1,
            });
            let segment = &mut slab[self.head_slab_id];
            segment.children_slab_ids.push(child_segment_id);
            PathTwo {
                head_slab_id: child_segment_id,
                ctx: self.ctx,
            }
        }
    }

    pub fn for_last_segment<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&PathSegment) -> T,
    {
        let slab = self.ctx.slab.borrow();
        f(&slab[self.head_slab_id])
    }

    pub fn update_display_variant(&self, display: DisplayVariant) {
        let mut slab = self.ctx.slab.borrow_mut();
        let segment = &mut slab[self.head_slab_id];
        segment.display = display;
    }

    pub fn for_segments<F, T>(&self, f: F) -> T
    where
        F: FnOnce(SegmentsIter) -> T,
    {
        let slab = self.ctx.slab.borrow();
        let mut ids = Vec::new();
        let mut current_id = Some(self.head_slab_id);
        while let Some(current_segment) = current_id.map(|id| &slab[id]) {
            ids.push(current_segment);
            current_id = current_segment.parent_slab_id;
        }

        let segments_iter: SegmentsIter = ids.into_iter().rev();
        f(segments_iter)
    }

    pub fn for_children<F, T>(&self, f: F) -> T
    where
        F: FnOnce(ChildPathsIter) -> T,
    {
        let slab = self.ctx.slab.borrow();
        let segment = slab.get(self.head_slab_id).expect("ids must be valid");
        let paths_iter: ChildPathsIter = iter::repeat(self.ctx)
            .zip(segment.children_slab_ids.iter())
            .map(id_to_path);

        f(paths_iter)
    }
}

type SegmentsIter<'c> = iter::Rev<std::vec::IntoIter<&'c PathSegment>>;

type ChildPathsIter<'a, 'c> = iter::Map<
    iter::Zip<iter::Repeat<&'c PathCtx>, std::slice::Iter<'a, SegmentId>>,
    fn((&'c PathCtx, &'a SegmentId)) -> PathTwo<'c>,
>;

fn id_to_segment((slab, id): (&Slab<PathSegment>, SegmentId)) -> &PathSegment {
    slab.get(id).expect("ids must be valid")
}

fn id_to_path<'a, 's>((ctx, id): (&'s PathCtx, &'a SegmentId)) -> PathTwo<'s> {
    PathTwo {
        head_slab_id: *id,
        ctx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_reuse() {
        let ctx = PathCtx::new();
        let sub_1 = ctx.get_root().child(b"key1");
        let sub_2 = sub_1.child(b"key2");
        let sub_2_again = sub_1.child(b"key2");
        assert_eq!(sub_2.head_slab_id, sub_2_again.head_slab_id);
    }

    #[test]
    fn children_ids() {
        let ctx = PathCtx::new();
        let sub_1 = ctx.get_root().child(b"key1");
        sub_1.child(b"key2");
        sub_1.child(b"key3");
        let mut children: Vec<Vec<u8>> = Vec::new();
        sub_1.for_children(|children_iter| {
            children.extend(children_iter.map(|p| p.for_last_segment(|k| k.bytes().to_vec())))
        });
        assert_eq!(children, vec![b"key2", b"key3"]);
    }

    #[test]
    fn collect_path() {
        let ctx = PathCtx::new();
        let path = ctx
            .get_root()
            .child(b"key1")
            .child(b"key2")
            .child(b"key3")
            .child(b"key4");
        let mut path_vec = Vec::new();
        path.for_segments(|segments_iter| {
            path_vec = segments_iter
                .map(|segment| segment.bytes().to_vec())
                .collect()
        });
        assert_eq!(path_vec, vec![b"key1", b"key2", b"key3", b"key4"]);
        assert_eq!(path.for_last_segment(|k| k.level), 4);
    }
}
