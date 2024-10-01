use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
};

use grovedbg_types::{Key, NodeUpdate};

use crate::{
    path_ctx::{Path, PathCtx},
    proof_viewer::MerkProofNodeViewer,
    tree_view::{ElementOrPlaceholder, ElementView, SubtreeElements},
};

pub(crate) type SubtreeProofData = BTreeMap<Key, MerkProofNodeViewer>;
pub(crate) type ProofData<'pa> = BTreeMap<Path<'pa>, SubtreeProofData>;
pub(crate) type SubtreeDataMap<'pa> = BTreeMap<Path<'pa>, RefCell<SubtreeData>>;

pub(crate) struct TreeData<'pa> {
    path_ctx: &'pa PathCtx,
    pub(crate) data: SubtreeDataMap<'pa>,
    pub(crate) proof_data: ProofData<'pa>,
    pub(crate) merk_selected: Path<'pa>,
}

#[derive(Default)]
pub(crate) struct SubtreeData {
    pub(crate) elements: SubtreeElements,
    pub(crate) root_key: Option<Key>,
    pub(crate) subtree_keys: BTreeSet<Key>,
    pub(crate) visible_keys: BTreeSet<Key>,
}

impl SubtreeData {
    pub(crate) fn get_root(&mut self) -> Option<&mut ElementView> {
        self.root_key.as_ref().and_then(|k| self.elements.get_mut(k))
    }
}

impl<'pa> TreeData<'pa> {
    pub(crate) fn new(path_ctx: &'pa PathCtx) -> Self {
        Self {
            path_ctx,
            data: Default::default(),
            merk_selected: path_ctx.get_root(),
            proof_data: Default::default(),
        }
    }

    pub(crate) fn select_for_merk(&mut self, path: Path<'pa>) {
        self.merk_selected = path;
    }

    pub(crate) fn get_or_create_mut(&mut self, path: Path<'pa>) -> RefMut<SubtreeData> {
        // NLL issue
        if self.data.contains_key(&path) {
            self.data.get(&path).unwrap().borrow_mut()
        } else {
            self.get_create_missing_parents(path).borrow_mut()
        }
    }

    pub(crate) fn get_or_create(&mut self, path: Path<'pa>) -> Ref<SubtreeData> {
        // NLL issue
        if self.data.contains_key(&path) {
            self.data.get(&path).unwrap().borrow()
        } else {
            self.get_create_missing_parents(path).borrow()
        }
    }

    pub(crate) fn get_mut(&self, path: &Path<'pa>) -> Option<RefMut<SubtreeData>> {
        self.data.get(path).map(RefCell::borrow_mut)
    }

    pub(crate) fn get(&self, path: &Path<'pa>) -> Option<Ref<SubtreeData>> {
        self.data.get(path).map(RefCell::borrow)
    }

    fn get_create_missing_parents(&mut self, path: Path<'pa>) -> &RefCell<SubtreeData> {
        let mut current_path = path;
        while let Some((parent, key)) = current_path.parent_with_key() {
            let parent_value = self.data.entry(parent).or_default();
            RefCell::borrow_mut(parent_value)
                .elements
                .entry(key.clone())
                .or_insert_with(|| ElementView::new_placeholder(key));

            current_path = parent;
        }

        self.data.entry(path).or_default()
    }

    pub(crate) fn apply_root_node_update(&mut self, node_update: NodeUpdate) {
        self.get_or_create_mut(self.path_ctx.get_root()).root_key = Some(node_update.key.clone());
        self.apply_node_update(node_update);
    }

    pub(crate) fn apply_node_update(
        &mut self,
        NodeUpdate {
            left_child,
            left_merk_hash,
            right_child,
            right_merk_hash,
            path,
            key,
            element,
            value_hash,
            kv_digest_hash,
            ..
        }: NodeUpdate,
    ) {
        let subtree_path = self.path_ctx.add_path(path);

        if let grovedbg_types::Element::Subtree { root_key, .. }
        | grovedbg_types::Element::Sumtree { root_key, .. } = &element
        {
            let child_subtree_path = subtree_path.child(key.clone());
            self.get_or_create_mut(child_subtree_path).root_key = root_key.clone();
            self.get_or_create_mut(subtree_path)
                .subtree_keys
                .insert(key.clone());
        }

        let mut subtree = self.get_or_create_mut(subtree_path);

        match subtree.elements.entry(key.clone()) {
            Entry::Vacant(e) => {
                e.insert(ElementView::new(
                    key,
                    ElementOrPlaceholder::Element(element),
                    left_child.clone(),
                    right_child.clone(),
                    Some(kv_digest_hash),
                    Some(value_hash),
                ));
            }
            Entry::Occupied(mut o) => {
                let e = o.get_mut();

                e.value = ElementOrPlaceholder::Element(element);
                e.left_child = left_child.clone();
                e.right_child = right_child.clone();
                e.kv_digest_hash = Some(kv_digest_hash);
                e.value_hash = Some(value_hash);
            }
        };

        if let (Some(left_hash), Some(left_key)) = (left_merk_hash, left_child) {
            match subtree.elements.entry(left_key.clone()) {
                Entry::Vacant(e) => {
                    let element = e.insert(ElementView::new_placeholder(left_key));
                    element.node_hash = Some(left_hash);
                }
                Entry::Occupied(mut o) => {
                    let e = o.get_mut();
                    e.node_hash = Some(left_hash);
                }
            };
        }

        if let (Some(right_hash), Some(right_key)) = (right_merk_hash, right_child) {
            match subtree.elements.entry(right_key.clone()) {
                Entry::Vacant(e) => {
                    let element = e.insert(ElementView::new_placeholder(right_key));
                    element.node_hash = Some(right_hash);
                }
                Entry::Occupied(mut o) => {
                    let e = o.get_mut();
                    e.node_hash = Some(right_hash);
                }
            };
        }
    }

    pub(crate) fn set_proof_tree(
        &mut self,
        proof_tree: BTreeMap<Vec<Vec<u8>>, BTreeMap<Vec<u8>, grovedbg_types::MerkProofNode>>,
    ) {
        self.proof_data = proof_tree
            .into_iter()
            .map(|(path_vec, proof_subtree)| {
                (
                    self.path_ctx.add_path(path_vec),
                    proof_subtree
                        .into_iter()
                        .map(|(key, proof_node)| (key, proof_node.into()))
                        .collect(),
                )
            })
            .collect();
    }
}
