use std::collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque};

use grovedbg_types::{Key, NodeUpdate};

use crate::{
    path_ctx::{Path, PathCtx},
    proof_viewer::{MerkProofOpViewer, ProofTree},
    tree_view::{ElementOrPlaceholder, ElementView, SubtreeElements},
};

pub(crate) struct TreeData<'pa> {
    path_ctx: &'pa PathCtx,
    data: BTreeMap<Path<'pa>, SubtreeData>,
    proof_data: Option<BTreeMap<Path<'pa>, BTreeMap<Key, MerkProofOpViewer>>>,
    merk_selected: Path<'pa>,
}

#[derive(Default)]
pub(crate) struct SubtreeData {
    pub(crate) elements: SubtreeElements,
    pub(crate) root_key: Option<Key>,
    pub(crate) subtree_keys: BTreeSet<Key>,
}

impl SubtreeData {
    pub(crate) fn get_root(&mut self) -> Option<&mut ElementView> {
        self.root_key.as_ref().and_then(|k| self.elements.get_mut(k))
    }
}

impl<'pa> TreeData<'pa> {
    pub(crate) fn build_proof_data(&mut self, proof: grovedbg_types::Proof) {
        let mut queue = VecDeque::new();
        queue.push_back((self.path_ctx.get_root(), proof.root_layer));

        let mut proof_data = BTreeMap::new();

        while let Some((path, proof)) = queue.pop_front() {
            let proof_tree = ProofTree::from_iter(proof.merk_proof);
        }
    }

    pub(crate) fn new(path_ctx: &'pa PathCtx) -> Self {
        Self {
            path_ctx,
            data: Default::default(),
            merk_selected: path_ctx.get_root(),
            proof_data: None,
        }
    }

    pub(crate) fn get_merk_selected(&mut self) -> (Path<'pa>, &mut SubtreeData) {
        (self.merk_selected, self.get(self.merk_selected))
    }

    pub(crate) fn select_for_merk(&mut self, path: Path<'pa>) {
        self.merk_selected = path;
    }

    pub(crate) fn get(&mut self, path: Path<'pa>) -> &mut SubtreeData {
        // NLL issue
        if self.data.contains_key(&path) {
            self.data.get_mut(&path).unwrap()
        } else {
            self.get_create_missing_parents(path)
        }
    }

    fn get_create_missing_parents(&mut self, path: Path<'pa>) -> &mut SubtreeData {
        let mut current_path = path;
        while let Some((parent, key)) = current_path.parent_with_key() {
            let parent_value = self.data.entry(parent).or_default();
            parent_value
                .elements
                .entry(key.clone())
                .or_insert_with(|| ElementView::new_placeholder(key));

            current_path = parent;
        }

        self.data.entry(path).or_default()
    }

    pub(crate) fn apply_root_node_update(&mut self, node_update: NodeUpdate) {
        self.get(self.path_ctx.get_root()).root_key = Some(node_update.key.clone());
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
            feature_type,
            value_hash,
            kv_digest_hash,
        }: NodeUpdate,
    ) {
        let subtree_path = self.path_ctx.add_path(path);

        if let grovedbg_types::Element::Subtree { root_key, .. }
        | grovedbg_types::Element::Sumtree { root_key, .. } = &element
        {
            let child_subtree_path = subtree_path.child(key.clone());
            self.get(child_subtree_path).root_key = root_key.clone();
            self.get(subtree_path).subtree_keys.insert(key.clone());
        }

        let subtree = self.get(subtree_path);

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
}
