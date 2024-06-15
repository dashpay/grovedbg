use std::{
    cell::RefCell,
    collections::VecDeque,
    fmt::{self, Display},
    hash::{Hash, Hasher},
    iter, ptr,
};

use slab::Slab;

use crate::ui::{common::bytes_by_display_variant, DisplayVariant};

type SegmentId = usize;

pub(crate) struct PathCtx {
    slab: RefCell<Slab<PathSegment>>,
    root_children_slab_ids: RefCell<Vec<SegmentId>>,
}

impl fmt::Debug for PathCtx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("slab")
    }
}

impl PathCtx {
    pub fn new() -> Self {
        PathCtx {
            slab: Default::default(),
            root_children_slab_ids: Default::default(),
        }
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

    pub fn add_profiles_alias(&self, path: Vec<Vec<u8>>, profiles_alias: String) -> Path {
        let path = self.add_path(path);
        if let Some(id) = path.head_slab_id {
            let mut slab = self.slab.borrow_mut();
            slab[id].profiles_alias = Some(profiles_alias);
        }
        path
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
}

pub(crate) struct PathSegment {
    parent_slab_id: Option<SegmentId>,
    children_slab_ids: Vec<SegmentId>,
    bytes: Vec<u8>,
    display: DisplayVariant,
    level: usize,
    profiles_alias: Option<String>,
}

impl PathSegment {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn display(&self) -> DisplayVariant {
        self.display
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
    pub fn get_profiles_alias(&self) -> Option<String> {
        self.for_last_segment(|s| s.profiles_alias.clone()).flatten()
    }

    pub fn clear_profile_alias(&self) {
        if let Some(id) = self.head_slab_id {
            let mut slab = self.ctx.slab.borrow_mut();
            slab[id].profiles_alias = None;
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
        let mut slab = self.ctx.slab.borrow_mut();
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
            let child_segment_id = slab.insert(PathSegment {
                parent_slab_id: self.head_slab_id,
                children_slab_ids: Vec::new(),
                display: DisplayVariant::guess(&key),
                bytes: key,
                level: level + 1,
                profiles_alias: None,
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

    fn is_root(&self) -> bool {
        self.head_slab_id.is_none()
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

    pub fn update_display_variant(&self, display: DisplayVariant) {
        self.head_slab_id.into_iter().for_each(|id| {
            let mut slab = self.ctx.slab.borrow_mut();
            let segment = &mut slab[id];
            segment.display = display;
        });
    }

    pub fn get_display_variant(&self) -> Option<DisplayVariant> {
        self.head_slab_id.map(|id| {
            let mut slab = self.ctx.slab.borrow_mut();
            let segment = &mut slab[id];
            segment.display
        })
    }

    pub fn for_display_mut<T>(&self, f: impl FnOnce(&mut DisplayVariant) -> T) -> Option<T> {
        if let Some(id) = self.head_slab_id {
            let mut slab = self.ctx.slab.borrow_mut();
            Some(f(&mut slab[id].display))
        } else {
            None
        }
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

    pub fn for_children<F, T>(&self, f: F) -> T
    where
        F: FnOnce(ChildPathsIter) -> T,
    {
        let slab = self.ctx.slab.borrow();
        let root_children = self.ctx.root_children_slab_ids.borrow();
        let children_vec = if let Some(segment_id) = self.head_slab_id {
            &slab[segment_id].children_slab_ids
        } else {
            root_children.as_ref()
        };
        let paths_iter: ChildPathsIter = iter::repeat(self.ctx).zip(children_vec.iter()).map(id_to_path);

        f(paths_iter)
    }

    pub fn for_each_descendant_recursively<F>(&self, f: F)
    where
        F: Fn(Path),
    {
        let slab = self.ctx.slab.borrow();
        let root_children = self.ctx.root_children_slab_ids.borrow();

        let mut descendant_paths: VecDeque<_> = if let Some(segment_id) = self.head_slab_id {
            &slab[segment_id].children_slab_ids
        } else {
            root_children.as_ref()
        }
        .iter()
        .map(|id| id_to_path((self.ctx, id)))
        .collect();

        while let Some(desc_path) = descendant_paths.pop_front() {
            descendant_paths.extend(
                slab[desc_path
                    .head_slab_id
                    .expect("child vectors can't contain root node")]
                .children_slab_ids
                .iter()
                .map(|id| id_to_path((self.ctx, id))),
            );
            f(desc_path);
        }
    }
}

type SegmentsIter<'c> = iter::Rev<std::vec::IntoIter<&'c PathSegment>>;

type ChildPathsIter<'a, 'c> = iter::Map<
    iter::Zip<iter::Repeat<&'c PathCtx>, std::slice::Iter<'a, SegmentId>>,
    fn((&'c PathCtx, &'a SegmentId)) -> Path<'c>,
>;

fn id_to_segment((slab, id): (&Slab<PathSegment>, SegmentId)) -> &PathSegment {
    slab.get(id).expect("ids must be valid")
}

fn id_to_path<'a, 's>((ctx, id): (&'s PathCtx, &'a SegmentId)) -> Path<'s> {
    Path {
        head_slab_id: Some(*id),
        ctx,
    }
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
    fn children_ids() {
        let ctx = PathCtx::new();
        let sub_1 = ctx.get_root().child(b"key1".to_vec());
        sub_1.child(b"key2".to_vec());
        sub_1.child(b"key3".to_vec());
        let mut children: Vec<Vec<u8>> = Vec::new();
        sub_1.for_children(|children_iter| {
            children.extend(children_iter.map(|p| p.for_last_segment(|k| k.bytes().to_vec()).unwrap()))
        });
        assert_eq!(children, vec![b"key2", b"key3"]);
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
