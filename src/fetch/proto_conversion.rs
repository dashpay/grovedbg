//! Conversion definitions from received proto object to model.
use std::iter;

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
            grovedbg_types::Element::Subtree {
                root_key,
                element_flags,
            } => Element::Subtree {
                root_key,
                element_flags,
            },
            grovedbg_types::Element::Sumtree {
                root_key,
                sum,
                element_flags,
            } => Element::Sumtree {
                root_key,
                sum,
                element_flags,
            },
            grovedbg_types::Element::Item { value, element_flags } => Element::Item { value, element_flags },
            grovedbg_types::Element::SumItem { value, element_flags } => {
                Element::SumItem { value, element_flags }
            }
            grovedbg_types::Element::Reference(grovedbg_types::Reference::AbsolutePathReference {
                path,
                element_flags,
            }) => from_absolute_path_reference(path_ctx, path, element_flags)?,
            grovedbg_types::Element::Reference(grovedbg_types::Reference::UpstreamRootHeightReference {
                n_keep,
                path_append,
                element_flags,
            }) => from_upstream_root_height_reference(path_ctx, path, n_keep, path_append, element_flags)?,
            grovedbg_types::Element::Reference(
                grovedbg_types::Reference::UpstreamFromElementHeightReference {
                    n_remove,
                    path_append,
                    element_flags,
                },
            ) => {
                from_upstream_element_height_reference(path_ctx, path, n_remove, path_append, element_flags)?
            }
            grovedbg_types::Element::Reference(grovedbg_types::Reference::CousinReference {
                swap_parent,
                element_flags,
            }) => from_cousin_reference(path_ctx, path.to_vec(), key.to_vec(), swap_parent, element_flags)?,
            grovedbg_types::Element::Reference(grovedbg_types::Reference::RemovedCousinReference {
                swap_parent,
                element_flags,
            }) => from_removed_cousin_reference(
                path_ctx,
                path.to_vec(),
                key.to_vec(),
                swap_parent,
                element_flags,
            )?,
            grovedbg_types::Element::Reference(grovedbg_types::Reference::SiblingReference {
                sibling_key,
                element_flags,
            }) => from_sibling_reference(path_ctx, path.to_vec(), sibling_key, element_flags),
            grovedbg_types::Element::Reference(
                grovedbg_types::Reference::UpstreamRootHeightWithParentPathAdditionReference {
                    n_keep,
                    path_append,
                    element_flags,
                },
            ) => from_upstream_root_height_with_parent_path_addition_reference(
                path_ctx,
                path,
                n_keep,
                path_append,
                element_flags,
            )?,
        })
    }
}

fn from_absolute_path_reference<'c>(
    path_ctx: &'c PathCtx,
    mut path: grovedbg_types::Path,
    element_flags: Option<Vec<u8>>,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    if let Some(key) = path.pop() {
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
            element_flags,
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
    element_flags: Option<Vec<u8>>,
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
            element_flags,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_upstream_root_height_with_parent_path_addition_reference<'c>(
    path_ctx: &'c PathCtx,
    path: &[PathSegment],
    n_keep: u32,
    path_append: Path,
    element_flags: Option<Vec<u8>>,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    let mut path_iter = path.iter().cloned();
    let parent = path_iter.next_back().ok_or_else(|| ReferenceWithoutKey)?;
    let mut path: Vec<_> = path_iter
        .take(n_keep as usize)
        .chain(path_append.into_iter())
        .chain(iter::once(parent))
        .collect();
    if let Some(key) = path.pop() {
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
            element_flags,
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
    element_flags: Option<Vec<u8>>,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    let mut path_iter = path.iter();
    path_iter.nth_back(n_remove as usize);
    let mut path: Vec<_> = path_iter.cloned().chain(path_append.into_iter()).collect();
    if let Some(key) = path.pop() {
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
            element_flags,
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
    element_flags: Option<Vec<u8>>,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    if let Some(parent) = path.last_mut() {
        *parent = swap_parent;
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
            element_flags,
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
    element_flags: Option<Vec<u8>>,
) -> Result<Element<'c>, ReferenceWithoutKey> {
    if let Some(_) = path.pop() {
        path.extend(swap_parent);
        Ok(Element::Reference {
            path: path_ctx.add_path(path),
            key,
            element_flags,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_sibling_reference<'c>(
    path_ctx: &'c PathCtx,
    path: Path,
    sibling_key: Key,
    element_flags: Option<Vec<u8>>,
) -> Element<'c> {
    Element::Reference {
        path: path_ctx.add_path(path),
        key: sibling_key,
        element_flags,
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
        feature_type: Some(value.feature_type),
        value_hash: Some(value.value_hash),
        kv_digest_hash: Some(value.kv_digest_hash),
        ui_state: Default::default(),
    })
}
