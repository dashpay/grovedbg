use eframe::emath::TSTransform;

use crate::path_ctx::{Path, PathCtx};

mod element_view;

#[derive(Clone, Copy)]
pub(crate) struct TreeViewContext<'a> {
    path_ctx: &'a PathCtx,
    transform: &'a TSTransform,
}

impl<'a> TreeViewContext<'a> {
    pub(crate) fn new(path_ctx: &'a PathCtx, transform: &'a TSTransform) -> Self {
        Self { path_ctx, transform }
    }

    pub(crate) fn root_context(&self) -> SubtreeViewContext<'a> {
        SubtreeViewContext {
            tree_view_context: *self,
            path: self.path_ctx.get_root(),
        }
    }

    pub(crate) fn focus(&mut self, path: Path<'a>) {
        todo!()
        //* transform = TSTransform::from_translation(
        //     node_ctx
        //         .child_subtree_ctx()
        //         .map(|ctx| ctx.subtree().get_subtree_input_point())
        //         .flatten()
        //         .map(|point| point.to_vec2() + Vec2::new(-1500., -900.))
        //         .unwrap_or_default(),
        // )
        // .inverse();
    }
}

pub(crate) struct SubtreeViewContext<'a> {
    tree_view_context: TreeViewContext<'a>,
    path: Path<'a>,
}

impl<'a> SubtreeViewContext<'a> {
    pub(crate) fn child(&self, key: Vec<u8>) -> SubtreeViewContext<'a> {
        SubtreeViewContext {
            tree_view_context: self.tree_view_context,
            path: self.path.child(key),
        }
    }

    pub(crate) fn focus_child(&self, key: Vec<u8>) {}

    pub(crate) fn path(&self) -> Path<'a> {
        self.path
    }
}
