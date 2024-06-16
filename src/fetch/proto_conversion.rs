//! Conversion definitions from received proto object to model.
use grovedbg_types::{Key, Path, PathSegment};

use crate::model::{path_display::PathCtx, Element, Node};

#[derive(Debug, thiserror::Error)]
#[error("Computed reference has no key")]
pub(crate) struct ReferenceWithoutKey;

pub(crate) struct ElementCtx<'a, 'c> {
    pub element: grovedbg_types::Element,
    pub path: &'a [PathSegment],
    pub key: &'a [u8],
    pub path_ctx: &'c PathCtx,
}

impl<'a, 'c> TryFrom<ElementCtx<'a, 'c>> for Element<'c> {
    type Error = ReferenceWithoutKey;

    fn try_from(
        ElementCtx {
            element,
            path,
            key,
            path_ctx,
        }: ElementCtx<'a, 'c>,
    ) -> Result<Self, Self::Error> {
        Ok(match element {
            grovedbg_types::Element::Subtree { root_key } => Element::Subtree { root_key },
            grovedbg_types::Element::Sumtree { root_key, sum } => Element::Sumtree { root_key, sum },
            grovedbg_types::Element::Item { value } => Element::Item { value },
            grovedbg_types::Element::SumItem { value } => Element::SumItem { value },
            grovedbg_types::Element::AbsolutePathReference { path } => {
                from_absolute_path_reference(path_ctx, path)?
            }
            grovedbg_types::Element::UpstreamRootHeightReference { n_keep, path_append } => {
                from_upstream_root_height_reference(path_ctx, path, n_keep, path_append)?
            }
            grovedbg_types::Element::UpstreamFromElementHeightReference {
                n_remove,
                path_append,
            } => from_upstream_element_height_reference(path_ctx, path, n_remove, path_append)?,
            grovedbg_types::Element::CousinReference { swap_parent } => {
                from_cousin_reference(path_ctx, path.to_vec(), key.to_vec(), swap_parent)?
            }
            grovedbg_types::Element::RemovedCousinReference { swap_parent } => {
                from_removed_cousin_reference(path_ctx, path.to_vec(), key.to_vec(), swap_parent)?
            }
            grovedbg_types::Element::SiblingReference { sibling_key } => {
                from_sibling_reference(path_ctx, path.to_vec(), sibling_key)
            }
        })
    }
}

fn from_absolute_path_reference<'c>(
    path_ctx: &'c PathCtx,
    mut path: grovedbg_types::Path,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    if let Some(key) = path.pop() {
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_upstream_root_height_reference<'c>(
    path_ctx: &'c PathCtx,
    path: &[PathSegment],
    n_keep: u32,
    path_append: Path,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    let mut path: Vec<_> = path
        .iter()
        .cloned()
        .take(n_keep as usize)
        .chain(path_append.into_iter())
        .collect();
    if let Some(key) = path.pop() {
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_upstream_element_height_reference<'c>(
    path_ctx: &'c PathCtx,
    path: &[PathSegment],
    n_remove: u32,
    path_append: Path,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    let mut path_iter = path.iter();
    path_iter.nth_back(n_remove as usize);
    let mut path: Vec<_> = path_iter.cloned().chain(path_append.into_iter()).collect();
    if let Some(key) = path.pop() {
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_cousin_reference<'c>(
    path_ctx: &'c PathCtx,
    mut path: Path,
    key: Key,
    swap_parent: Key,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    if let Some(parent) = path.last_mut() {
        *parent = swap_parent;
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_removed_cousin_reference<'c>(
    path_ctx: &'c PathCtx,
    mut path: Path,
    key: Key,
    swap_parent: Vec<PathSegment>,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    if let Some(_) = path.pop() {
        path.extend(swap_parent);
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_sibling_reference<'c>(path_ctx: &'c PathCtx, path: Path, sibling_key: Key) -> Element<'c> {
    Element::Reference {
        path: path_ctx.add_path(path),
        key: sibling_key,
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum BadProtoElement {
    #[error(transparent)]
    EmptyPathReference(#[from] ReferenceWithoutKey),
    #[error("Proto Element is None")]
    NoneElement,
}

pub(crate) fn from_update<'c>(
    path_ctx: &'c PathCtx,
    value: grovedbg_types::NodeUpdate,
) -> Result<Node<'c>, BadProtoElement> {
    Ok(Node {
        element: ElementCtx {
            element: value.element,
            path: &value.path,
            key: &value.key,
            path_ctx,
        }
        .try_into()?,
        left_child: value.left_child,
        right_child: value.right_child,
        ..Default::default()
    })
}
