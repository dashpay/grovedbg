//! Conversion definitions from received proto object to model.
use grovedbg_types::{Key, Path, PathSegment};

use crate::model::{Element, Node};

#[derive(Debug, thiserror::Error)]
#[error("Computed reference has no key")]
pub(crate) struct ReferenceWithoutKey;

pub(crate) struct ElementCtx<'a> {
    pub element: grovedbg_types::Element,
    pub path: &'a [PathSegment],
    pub key: &'a [u8],
}

impl<'a> TryFrom<ElementCtx<'a>> for Element {
    type Error = ReferenceWithoutKey;

    fn try_from(ElementCtx { element, path, key }: ElementCtx) -> Result<Self, Self::Error> {
        Ok(match element {
            grovedbg_types::Element::Subtree { root_key } => Element::Subtree { root_key },
            grovedbg_types::Element::Sumtree { root_key, sum } => {
                Element::Sumtree { root_key, sum }
            }
            grovedbg_types::Element::Item { value } => Element::Item { value },
            grovedbg_types::Element::SumItem { value } => Element::SumItem { value },
            grovedbg_types::Element::AbsolutePathReference { path } => {
                from_absolute_path_reference(path)?
            }
            grovedbg_types::Element::UpstreamRootHeightReference {
                n_keep,
                path_append,
            } => from_upstream_root_height_reference(path, n_keep, path_append)?,
            grovedbg_types::Element::UpstreamFromElementHeightReference {
                n_remove,
                path_append,
            } => from_upstream_element_height_reference(path, n_remove, path_append)?,
            grovedbg_types::Element::CousinReference { swap_parent } => {
                from_cousin_reference(path.to_vec(), key.to_vec(), swap_parent)?
            }
            grovedbg_types::Element::RemovedCousinReference { swap_parent } => {
                from_removed_cousin_reference(path.to_vec(), key.to_vec(), swap_parent)?
            }
            grovedbg_types::Element::SiblingReference { sibling_key } => {
                from_sibling_reference(path.to_vec(), sibling_key)
            }
        })
    }
}

fn from_absolute_path_reference(
    mut path: grovedbg_types::Path,
) -> Result<Element, ReferenceWithoutKey> {
    if let Some(key) = path.pop() {
        Ok(Element::Reference {
            path: path.into(),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_upstream_root_height_reference(
    path: &[PathSegment],
    n_keep: u32,
    path_append: Path,
) -> Result<Element, ReferenceWithoutKey> {
    let mut path: Vec<_> = path
        .iter()
        .cloned()
        .take(n_keep as usize)
        .chain(path_append.into_iter())
        .collect();
    if let Some(key) = path.pop() {
        Ok(Element::Reference {
            path: path.into(),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_upstream_element_height_reference(
    path: &[PathSegment],
    n_remove: u32,
    path_append: Path,
) -> Result<Element, ReferenceWithoutKey> {
    let mut path_iter = path.iter();
    path_iter.nth_back(n_remove as usize);
    let mut path: Vec<_> = path_iter.cloned().chain(path_append.into_iter()).collect();
    if let Some(key) = path.pop() {
        Ok(Element::Reference {
            path: path.into(),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_cousin_reference(
    mut path: Path,
    key: Key,
    swap_parent: Key,
) -> Result<Element, ReferenceWithoutKey> {
    if let Some(parent) = path.last_mut() {
        *parent = swap_parent;
        Ok(Element::Reference {
            path: path.into(),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_removed_cousin_reference(
    mut path: Path,
    key: Key,
    swap_parent: Vec<PathSegment>,
) -> Result<Element, ReferenceWithoutKey> {
    if let Some(_) = path.pop() {
        path.extend(swap_parent);
        Ok(Element::Reference {
            path: path.into(),
            key,
        })
    } else {
        Err(ReferenceWithoutKey)
    }
}

fn from_sibling_reference(path: Path, sibling_key: Key) -> Element {
    Element::Reference {
        path: path.into(),
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

impl TryFrom<grovedbg_types::NodeUpdate> for Node {
    type Error = BadProtoElement;

    fn try_from(value: grovedbg_types::NodeUpdate) -> Result<Self, Self::Error> {
        Ok(Node {
            element: ElementCtx {
                element: value.element,
                path: &value.path,
                key: &value.key,
            }
            .try_into()?,
            left_child: value.left_child,
            right_child: value.right_child,
            ..Default::default()
        })
    }
}
