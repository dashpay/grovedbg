pub(crate) mod alignment;
pub(crate) mod path_display;

use std::{
    cell::{RefCell, RefMut},
    cmp,
    collections::{BTreeMap, BTreeSet, HashSet},
};

use eframe::{egui, epaint::Pos2};
use grovedbg_types::{CryptoHash, TreeFeatureType};

use self::{
    alignment::{expanded_subtree_levels, leaves_level_count},
    path_display::{Path, PathCtx},
};
use crate::ui::DisplayVariant;

pub(crate) type Key = Vec<u8>;
pub(crate) type KeySlice<'a> = &'a [u8];

#[derive(Clone, Copy)]
struct SetVisibility<'t, 'c> {
    tree: &'t Tree<'c>,
    path: Path<'c>,
}

impl<'t, 'c> SetVisibility<'t, 'c> {
    pub(crate) fn set_visible(&self, key: KeySlice, visible: bool) {
        let path = self.path.child(key.to_vec());
        if let Some(subtree) = self.tree.get_subtree(&path) {
            subtree.subtree().set_visible(visible);

            if !visible {
                path.for_each_descendant_recursively(|desc_path| {
                    self.tree
                        .subtrees
                        .get(&desc_path)
                        .iter()
                        .for_each(|subtree| subtree.set_visible(false));
                });
            }
        }
    }

    pub(crate) fn visible(&self, key: KeySlice) -> bool {
        let path = self.path.child(key.to_vec());
        self.tree
            .get_subtree(&path)
            .map(|subtree| subtree.subtree().visible())
            .unwrap_or_default()
    }
}

/// Structure that holds the currently known state of GroveDB.
#[derive(Debug)]
pub(crate) struct Tree<'c> {
    pub(crate) subtrees: BTreeMap<Path<'c>, Subtree<'c>>,
    pub(crate) levels_heights: RefCell<Vec<Height>>,
    pub(crate) path_ctx: &'c PathCtx,
}

impl<'c> Tree<'c> {
    pub(crate) fn new(path_ctx: &'c PathCtx) -> Self {
        Self {
            subtrees: Default::default(),
            levels_heights: Default::default(),
            path_ctx,
        }
    }

    pub(crate) fn update_dimensions(&self) {
        let mut levels_heights = self.levels_heights.borrow_mut();
        let mut subtrees_iter = self.iter_subtrees().rev().peekable();
        let levels_count = subtrees_iter
            .peek()
            .map(|ctx| ctx.path().level())
            .unwrap_or_default();

        *levels_heights = vec![0; levels_count + 1];

        for subtree_ctx in subtrees_iter {
            if !subtree_ctx.subtree.visible() {
                continue;
            }

            let height = subtree_ctx.update_dimensions();
            let level = subtree_ctx.path().level();
            levels_heights[level] = cmp::max(levels_heights[level], height);
        }
    }

    pub(crate) fn set_root(&mut self, root_key: Key) {
        self.subtrees
            .entry(self.path_ctx.get_root())
            .or_default()
            .set_root(root_key)
            .set_visible(true);
    }

    pub(crate) fn iter_subtrees<'t>(
        &'t self,
    ) -> impl ExactSizeIterator<Item = SubtreeCtx<'t, 'c>> + DoubleEndedIterator<Item = SubtreeCtx<'t, 'c>>
    {
        self.subtrees.iter().map(|(path, subtree)| SubtreeCtx {
            path: *path,
            subtree,
            set_child_visibility: SetVisibility {
                tree: self,
                path: path.clone(),
            },
            tree: self,
        })
    }

    pub(crate) fn get_node<'a>(&'a self, path: &Path<'c>, key: KeySlice) -> Option<&'a Node> {
        self.subtrees
            .get(path)
            .map(|subtree| subtree.nodes.get(key))
            .flatten()
    }

    pub(crate) fn get_subtree<'a>(&'a self, path: &Path<'c>) -> Option<SubtreeCtx<'a, 'c>> {
        self.subtrees.get(path).map(|subtree| SubtreeCtx {
            subtree,
            path: *path,
            set_child_visibility: SetVisibility {
                tree: self,
                path: path.clone(),
            },
            tree: self,
        })
    }

    pub(crate) fn insert(&mut self, path: Path<'c>, key: Key, node: Node<'c>) {
        {
            let mut state = node.ui_state.borrow_mut();
            state.key_display_variant = DisplayVariant::guess(&key);
            if let Element::Item { value, .. } = &node.element {
                state.item_display_variant = DisplayVariant::guess(&value);
            }

            if let Some(bytes) = node.value_hash {
                state.value_hash_display_variant = DisplayVariant::guess(&bytes);
            }
            if let Some(bytes) = node.kv_digest_hash {
                state.kv_digest_hash_display_variant = DisplayVariant::guess(&bytes);
            }
        }

        // Make sure all subtrees exist and according nodes are there as well
        self.populate_subtrees_chain(path.clone());

        // If a new node inserted represents another subtree, it shall also be added;
        // Root node info is updated as well
        if let Element::Sumtree { root_key, .. } | Element::Subtree { root_key, .. } = &node.element {
            let child_path = path.child(key.clone());

            let child_subtree = self.subtrees.entry(child_path).or_default();
            if let Some(root_key) = root_key {
                child_subtree.set_root(root_key.clone());
            }
        }

        self.subtrees
            .get_mut(&path)
            .expect("model was updated")
            .insert(key, node);
    }

    pub(crate) fn remove(&mut self, path: &Path<'c>, key: KeySlice) {
        if let Some(subtree) = self.subtrees.get_mut(path) {
            subtree.remove(key);
        }
    }

    /// The data structure guarantees  that for a node representing a subtree
    /// an according subtree entry must exists, that means if there is a parent
    /// subtree with a node representing the root node of the deletion
    /// subject then in won't be deleted completely.
    pub(crate) fn clear_subtree(&mut self, path: Path<'c>) {
        if let Some(subtree) = self.subtrees.get_mut(&path) {
            subtree.nodes.clear();
        }
    }

    /// For a given path ensures all parent subtrees exist and each of them
    /// contains a node for a child subtree, all missing parts will be
    /// created.
    fn populate_subtrees_chain(&mut self, path: Path<'c>) {
        self.subtrees.entry(path).or_default();
        let mut current = path.parent_with_key();
        while let Some((parent, parent_key)) = current {
            let subtree = self.subtrees.entry(parent).or_default();
            subtree.insert_not_exists(parent_key, Node::new_subtree_placeholder());
            current = parent.parent_with_key();
        }
    }
}

struct SubtreeWidth {
    n_collapsed: usize,
    expanded: Vec<usize>,
}

#[derive(Debug, Default)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct SubtreeUiState {
    pub(crate) path_display_variant: DisplayVariant,
    pub(crate) expanded: bool,
    pub(crate) input_point: Pos2,
    pub(crate) output_point: Pos2,
    pub(crate) page: usize,
    pub(crate) visible: bool,
    pub(crate) width: usize,
    pub(crate) children_width: usize,
    pub(crate) height: f32,
    pub(crate) levels: u32,
    pub(crate) leaves: u32,
}

/// Subtree holds all the info about one specific subtree of GroveDB
#[derive(Debug, Default)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct Subtree<'c> {
    /// Actual root node of a subtree, may be unknown yet since it requires a
    /// parent subtree to tell, or a tree could be empty
    pub(crate) root_node: Option<Key>,
    /// Root nodes of subtree's clusters.
    /// In GroveDb there are no clusters but without whole picture fetched from
    /// GroveDb we may occasionally be unaware of all connections, but still
    /// want to know how to draw it. Since we're drawing from roots, we have to
    /// keep these "local" roots.
    /// TODO: a useless feature perhaps
    cluster_roots: BTreeSet<Key>,
    /// All fetched subtree nodes
    pub(crate) nodes: BTreeMap<Key, Node<'c>>,
    /// Subtree nodes' keys to keep track of nodes that are not yet fetched but
    /// referred by parent node
    waitlist: HashSet<Key>,
    /// UI state of a subtree
    ui_state: RefCell<SubtreeUiState>,
}

impl<'c> Subtree<'c> {
    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub(crate) fn levels(&self) -> u32 {
        self.ui_state.borrow().levels
    }

    pub(crate) fn leaves(&self) -> u32 {
        self.ui_state.borrow().leaves
    }

    pub(crate) fn width(&self) -> usize {
        self.ui_state.borrow().width
    }

    pub(crate) fn children_width(&self) -> usize {
        self.ui_state.borrow().children_width
    }

    fn new() -> Self {
        Default::default()
    }

    fn new_root(root_node: Key) -> Self {
        Self {
            root_node: Some(root_node),
            ..Default::default()
        }
    }

    pub(crate) fn visible(&self) -> bool {
        self.ui_state.borrow().visible
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        self.ui_state.borrow_mut().visible = visible;
    }

    pub(crate) fn page_idx(&self) -> usize {
        self.ui_state.borrow().page
    }

    pub(crate) fn next_page(&self) {
        self.ui_state.borrow_mut().page += 1;
    }

    pub(crate) fn prev_page(&self) {
        let page: &mut usize = &mut self.ui_state.borrow_mut().page;

        if *page > 0 {
            *page -= 1;
        }
    }

    pub(crate) fn first_page(&self) {
        self.ui_state.borrow_mut().page = 0;
    }

    pub(crate) fn n_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn is_expanded(&self) -> bool {
        self.ui_state.borrow().expanded
    }

    pub(crate) fn set_expanded(&self) {
        if !self.is_empty() {
            self.ui_state.borrow_mut().expanded = true;
        }
    }

    pub(crate) fn set_collapsed(&self) {
        self.ui_state.borrow_mut().expanded = false;
        for (_, node) in self.nodes.iter() {
            let mut state = node.ui_state.borrow_mut();
            state.show_left = false;
            state.show_right = false;
        }
    }

    pub(crate) fn set_input_point(&self, input_point: Pos2) {
        self.ui_state.borrow_mut().input_point = input_point;
    }

    pub(crate) fn set_output_point(&self, output_point: Pos2) {
        self.ui_state.borrow_mut().output_point = output_point;
    }

    pub(crate) fn path_display_variant_mut(&self) -> RefMut<DisplayVariant> {
        RefMut::map(self.ui_state.borrow_mut(), |state| {
            &mut state.path_display_variant
        })
    }

    pub(crate) fn iter_cluster_roots(&self) -> impl Iterator<Item = &Node> {
        self.cluster_roots
            .iter()
            .map(|key| self.nodes.get(key).expect("cluster roots are in sync"))
    }

    pub(crate) fn get_subtree_input_point(&self) -> Option<Pos2> {
        {
            let subtree_ui_state = self.ui_state.borrow();
            if !subtree_ui_state.expanded {
                return Some(subtree_ui_state.input_point);
            }
        }

        if let Some(root) = self.root_node() {
            return Some(root.ui_state.borrow().input_point);
        }

        if let Some(cluster) = self
            .cluster_roots
            .first()
            .as_ref()
            .map(|key| self.nodes.get(key.as_slice()))
            .flatten()
        {
            return Some(cluster.ui_state.borrow().input_point);
        }

        None
    }

    pub(crate) fn get_subtree_output_point(&self) -> Pos2 {
        self.ui_state.borrow().output_point
    }

    /// Get input point of a node, if subtree is collapsed it will return input
    /// point of a collapsed subtree frame instead
    pub(crate) fn get_node_input(&self, key: KeySlice) -> Option<Pos2> {
        let subtree_ui_state = self.ui_state.borrow();
        if !subtree_ui_state.expanded {
            Some(subtree_ui_state.input_point)
        } else {
            self.nodes.get(key).map(|node| node.ui_state.borrow().input_point)
        }
    }

    pub(crate) fn get_node_output(&self, key: KeySlice) -> Option<Pos2> {
        let subtree_ui_state = self.ui_state.borrow();
        if !subtree_ui_state.expanded {
            Some(subtree_ui_state.output_point)
        } else {
            self.nodes
                .get(key)
                .map(|node| node.ui_state.borrow().output_point)
        }
    }

    /// Set a root node of a subtree
    fn set_root(&mut self, root_node: Key) -> &mut Self {
        self.cluster_roots.remove(&root_node);
        self.root_node = Some(root_node);
        self
    }

    pub(crate) fn root_node(&self) -> Option<&Node> {
        self.root_node
            .as_ref()
            .map(|k| self.nodes.get(k.as_slice()))
            .flatten()
    }

    /// Remove a node, any node can be removed and a possibly splitted tree is
    /// taken care of.
    fn remove(&mut self, key: KeySlice) {
        if let Some(node) = self.nodes.remove(key) {
            // Update the waitlist since no one is waiting for these children anymore :(
            node.left_child.iter().for_each(|child| {
                self.waitlist.remove(child);
            });
            node.right_child.iter().for_each(|child| {
                self.waitlist.remove(child);
            });

            // However, since they have no parent now they're own cluster bosses
            if let Some(child) = node.left_child {
                if self.nodes.contains_key(&child) {
                    self.cluster_roots.insert(child);
                }
            }

            if let Some(child) = node.right_child {
                if self.nodes.contains_key(&child) {
                    self.cluster_roots.insert(child);
                }
            }

            // If the removed node is not a root and not a cluster root then someone else
            // will wait for it
            if self
                .root_node
                .as_ref()
                .map(|root_node| root_node != key)
                .unwrap_or(true)
                && !self.cluster_roots.contains(key)
            {
                self.waitlist.insert(key.to_vec());
            }
        }
    }

    /// Insert a node into the subtree that doesn't necessarily connected to the
    /// current state.
    fn insert(&mut self, key: Key, node: Node<'c>) {
        self.remove(&key);

        // There are three cases for a node:
        // 1. It is a root node. No additional actions needed.
        // 2. It is a child node with a parent inserted. Need to remove the entry from
        //    waitlist because no need to wait for the node anymore.
        // 3. It is a child node with no parent inserted. As no one is waiting for the
        //    node in waitlist, this one shall become a cluster root until the parent is
        //    found.
        //
        // For all three cases child nodes processing remains the same (waitlist and
        // cluster roots adjustments).

        if !self.waitlist.remove(&key)
            && self
                .root_node
                .as_ref()
                .map(|root_node| root_node != &key)
                .unwrap_or(true)
        {
            // An item was not found in the waitlist and it's not a root, that
            // means no parent is there yet and it shall become a root of a
            // cluster.
            self.cluster_roots.insert(key.clone());
        }

        // Each of the node's children are in waitlist now if missing and are not
        // cluster roots anymore if they were.
        let mut child_updates = |child_key: &Key| {
            if !self.nodes.contains_key(child_key) {
                self.waitlist.insert(child_key.clone());
            }
            self.cluster_roots.remove(child_key);
        };

        if let Some(child) = &node.left_child {
            child_updates(child);
        }

        if let Some(child) = &node.right_child {
            child_updates(child);
        }

        // Finally insert the node
        self.nodes.insert(key, node);
    }

    fn insert_not_exists(&mut self, key: Key, node: Node<'c>) {
        if !self.nodes.contains_key(&key) {
            self.insert(key, node);
        }
    }

    fn iter_subtree_keys(&self) -> impl Iterator<Item = &Key> {
        self.nodes.iter().filter_map(|(key, node)| {
            matches!(
                node.element,
                Element::Sumtree { .. } | Element::Subtree { .. } | Element::SubtreePlaceholder
            )
            .then(|| key)
        })
    }
}

/// A wrapper type to guarantee that the subtree has the specified path.
#[derive(Clone, Copy)]
pub(crate) struct SubtreeCtx<'t, 'c> {
    subtree: &'t Subtree<'c>,
    path: Path<'c>,
    set_child_visibility: SetVisibility<'t, 'c>,
    tree: &'t Tree<'c>,
}

type Height = usize;

impl<'a, 'c> SubtreeCtx<'a, 'c> {
    pub(crate) fn is_visible(&self) -> bool {
        self.subtree.ui_state.borrow().visible
    }

    fn update_dimensions(&self) -> Height {
        let mut state = self.subtree.ui_state.borrow_mut();
        let (height, self_width) = if state.expanded {
            let levels = expanded_subtree_levels(self.subtree);
            let leaves = leaves_level_count(levels as u32);
            state.levels = levels as u32;
            state.leaves = leaves;
            (levels, leaves * 2)
        } else {
            (2, 1)
        };

        let (count, mut children_width): (usize, usize) = self
            .iter_subtrees()
            .filter_map(|subtree_ctx| {
                let state = subtree_ctx.subtree.ui_state.borrow();
                state.visible.then_some((1, state.width))
            })
            .fold((0, 0), |acc, (count, width)| (acc.0 + count, acc.1 + width));

        children_width += count.saturating_sub(1); // Intervals also count
        state.children_width = children_width;

        state.width = cmp::max(self_width as usize, children_width);
        height * 2
    }

    pub(crate) fn iter_subtrees(&self) -> impl Iterator<Item = SubtreeCtx<'a, 'c>> + '_ {
        self.subtree.iter_subtree_keys().map(|key| {
            let path = self.path.child(key.to_vec());
            SubtreeCtx {
                subtree: &self.tree.subtrees[&path],
                path,
                set_child_visibility: SetVisibility {
                    tree: self.tree,
                    path,
                },
                tree: self.tree,
            }
        })
    }

    pub(crate) fn set_child_visibility(&self, key: KeySlice<'a>, visible: bool) {
        self.set_child_visibility.set_visible(key, visible)
    }

    pub(crate) fn set_children_invisible(&self) {
        self.subtree
            .nodes
            .iter()
            .filter_map(|(key, node)| {
                matches!(
                    node.element,
                    Element::Sumtree { .. } | Element::Subtree { .. } | Element::SubtreePlaceholder
                )
                .then_some(key)
            })
            .for_each(|key| self.set_child_visibility.set_visible(key, false));
    }

    pub(crate) fn is_child_visible(&self, key: KeySlice<'a>) -> bool {
        self.set_child_visibility.visible(key)
    }

    pub(crate) fn get_node(&self, key: Key) -> Option<NodeCtx<'a, 'c>> {
        self.subtree.nodes.get(&key).map(|node| NodeCtx {
            node,
            path: self.path,
            key,
            subtree_ctx: self.clone(),
        })
    }

    pub(crate) fn get_root(&self) -> Option<NodeCtx<'a, 'c>> {
        self.subtree
            .root_node
            .as_ref()
            .map(|key| self.get_node(key.to_vec()))
            .flatten()
    }

    pub(crate) fn subtree(&self) -> &'a Subtree {
        self.subtree
    }

    pub(crate) fn path(&self) -> Path<'c> {
        self.path
    }

    pub(crate) fn iter_nodes(&self) -> impl ExactSizeIterator<Item = NodeCtx<'a, 'c>> {
        let subtree: &'a Subtree<'c> = self.subtree;
        let path: Path<'c> = self.path;
        let subtree_ctx: SubtreeCtx<'a, 'c> = self.clone();
        subtree.nodes.iter().map(move |(key, node)| NodeCtx {
            node,
            path,
            key: key.clone(),
            subtree_ctx,
        })
    }

    pub(crate) fn egui_id(&self) -> egui::Id {
        egui::Id::new(("subtree", self.path))
    }
}

/// A wrapper type to guarantee that the node has specified path and key.
#[derive(Clone)]
pub(crate) struct NodeCtx<'a, 'c> {
    node: &'a Node<'c>,
    path: Path<'c>,
    key: Key,
    subtree_ctx: SubtreeCtx<'a, 'c>,
}

impl<'a, 'c> NodeCtx<'a, 'c> {
    pub(crate) fn child_subtree_ctx(&self) -> Option<SubtreeCtx<'a, 'c>> {
        let path = self.path.child(self.key().to_vec());
        self.subtree_ctx.tree.get_subtree(&path)
    }

    pub(crate) fn path(&self) -> Path {
        self.path
    }

    pub(crate) fn key(&self) -> KeySlice {
        &self.key
    }

    pub(crate) fn with_key_display_variant<T>(&self, f: impl FnOnce(&mut DisplayVariant) -> T) -> T {
        if matches!(
            self.node.element,
            Element::Subtree { .. } | Element::Sumtree { .. } | Element::SubtreePlaceholder
        ) {
            self.path
                .child(self.key.clone())
                .for_display_mut(f)
                .expect("not a root path")
        } else {
            f(&mut self.node.ui_state.borrow_mut().key_display_variant)
        }
    }

    pub(crate) fn node(&self) -> &'a Node<'c> {
        self.node
    }

    pub(crate) fn subtree(&self) -> &'a Subtree {
        self.subtree_ctx.subtree
    }

    pub(crate) fn subtree_ctx(&self) -> SubtreeCtx<'a, 'c> {
        self.subtree_ctx
    }

    pub(crate) fn egui_id(&self) -> egui::Id {
        egui::Id::new(("node", self.path, &self.key))
    }

    pub(crate) fn set_left_visible(&self) {
        self.node.ui_state.borrow_mut().show_left = true;
    }

    pub(crate) fn set_right_visible(&self) {
        self.node.ui_state.borrow_mut().show_right = true;
    }
}

// TODO: approach used in query builder and proof viewer seems to be more
// consistent and useful
#[derive(Debug, Clone, Default)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct NodeUiState {
    pub(crate) key_display_variant: DisplayVariant,
    pub(crate) item_display_variant: DisplayVariant,
    pub(crate) flags_display_variant: DisplayVariant,
    pub(crate) value_hash_display_variant: DisplayVariant,
    pub(crate) kv_digest_hash_display_variant: DisplayVariant,
    pub(crate) input_point: Pos2,
    pub(crate) output_point: Pos2,
    pub(crate) left_sibling_point: Pos2,
    pub(crate) right_sibling_point: Pos2,
    pub(crate) show_left: bool,
    pub(crate) show_right: bool,
    pub(crate) show_hashes: bool,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct Node<'c> {
    pub(crate) element: Element<'c>,
    pub(crate) left_child: Option<Key>,
    pub(crate) right_child: Option<Key>,
    pub(crate) feature_type: Option<TreeFeatureType>,
    pub(crate) value_hash: Option<CryptoHash>,
    pub(crate) kv_digest_hash: Option<CryptoHash>,
    pub(crate) ui_state: RefCell<NodeUiState>,
}

impl<'c> Node<'c> {
    #[cfg(test)]
    fn new_item(value: Vec<u8>) -> Self {
        Node {
            element: Element::Item {
                value,
                element_flags: None,
            },
            ..Default::default()
        }
    }

    #[cfg(test)]
    fn new_sumtree(root_key: Option<Key>, sum: i64) -> Self {
        Node {
            element: Element::Sumtree {
                root_key,
                sum,
                element_flags: None,
            },
            ..Default::default()
        }
    }

    #[cfg(test)]
    fn new_subtree(root_key: Option<Key>) -> Self {
        Node {
            element: Element::Subtree {
                root_key,
                element_flags: None,
            },
            ..Default::default()
        }
    }

    fn new_subtree_placeholder() -> Self {
        Node {
            element: Element::SubtreePlaceholder,
            ..Default::default()
        }
    }

    #[cfg(test)]
    fn with_left_child(mut self, key: Key) -> Self {
        self.left_child = Some(key);
        self
    }

    #[cfg(test)]
    fn with_right_child(mut self, key: Key) -> Self {
        self.right_child = Some(key);
        self
    }
}

/// A value that a subtree's node hold
#[derive(Debug, Clone, Default, PartialEq, strum::AsRefStr)]
pub(crate) enum Element<'c> {
    /// Scalar value, arbitrary bytes
    Item {
        value: Vec<u8>,
        element_flags: Option<Vec<u8>>,
    },
    /// Subtree item that will be summed in a sumtree that contains it
    SumItem {
        value: i64,
        element_flags: Option<Vec<u8>>,
    },
    /// Reference to another (or the same) subtree's node
    Reference {
        path: Path<'c>,
        key: Key,
        element_flags: Option<Vec<u8>>,
    },
    /// A link to a deeper level subtree which accumulates a sum of its sum
    /// items, `None` indicates an empty subtree
    Sumtree {
        root_key: Option<Key>,
        sum: i64,
        element_flags: Option<Vec<u8>>,
    },
    /// A link to a deeper level subtree that starts with root_key; `None`
    /// indicates an empty subtree.
    Subtree {
        root_key: Option<Key>,
        element_flags: Option<Vec<u8>>,
    },
    /// A placeholder of a not yet added node for a sub/sumtree in case we're
    /// aware of sub/sumtree existence (like by doing insertion using a path
    /// that mentions the subtree alongs its way)
    #[default]
    SubtreePlaceholder,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> Subtree<'static> {
        // root
        // ├── right1
        // │   ├── right2
        // │   └── left2
        // │       ├── right4
        // │       └── left4
        // └── left1
        //     └── right3

        let mut subtree = Subtree::new_root(b"root".to_vec());

        subtree.insert(
            b"root".to_vec(),
            Node::new_item(b"root_value".to_vec())
                .with_left_child(b"left1".to_vec())
                .with_right_child(b"right1".to_vec()),
        );
        subtree.insert(
            b"right1".to_vec(),
            Node::new_item(b"right1_value".to_vec())
                .with_left_child(b"left2".to_vec())
                .with_right_child(b"right2".to_vec()),
        );
        subtree.insert(
            b"left1".to_vec(),
            Node::new_item(b"left1_value".to_vec()).with_right_child(b"right3".to_vec()),
        );
        subtree.insert(b"right2".to_vec(), Node::new_item(b"right2_value".to_vec()));
        subtree.insert(
            b"left2".to_vec(),
            Node::new_item(b"left2_value".to_vec())
                .with_left_child(b"left4".to_vec())
                .with_right_child(b"right4".to_vec()),
        );
        subtree.insert(b"right3".to_vec(), Node::new_item(b"right3_value".to_vec()));
        subtree.insert(b"right4".to_vec(), Node::new_item(b"right4_value".to_vec()));
        subtree.insert(b"left4".to_vec(), Node::new_item(b"right4_value".to_vec()));

        subtree
    }

    #[test]
    fn simple_sequential_insertion_subtree() {
        let subtree = sample_tree();

        assert!(subtree.waitlist.is_empty());
        assert!(subtree.cluster_roots.is_empty());
    }

    #[test]
    fn subtree_node_leaf_removal() {
        let mut subtree = sample_tree();

        // "Unloading" a node from subtree, meaning it will be missed
        subtree.remove(b"left4");

        assert!(!subtree.nodes.contains_key(b"left4".as_ref()));
        assert_eq!(
            subtree.waitlist.iter().next().map(|k| k.as_slice()),
            Some(b"left4".as_ref())
        );
        assert!(subtree.cluster_roots.is_empty());
    }

    #[test]
    fn subtree_node_leaf_complete_removal() {
        let mut subtree = sample_tree();

        subtree.remove(b"left4");
        let mut old_parent = subtree.nodes.get(b"left2".as_ref()).unwrap().clone();
        old_parent.left_child = None;
        subtree.insert(b"left2".to_vec(), old_parent);

        assert!(!subtree.nodes.contains_key(b"left4".as_ref()));
        assert!(subtree.waitlist.is_empty());
        assert!(subtree.cluster_roots.is_empty());
    }

    #[test]
    fn subtree_mid_node_delete_creates_clusters() {
        let mut subtree = sample_tree();

        // Deleting a node in a middle of a subtree shall create clusters
        subtree.remove(b"right1");

        assert!(!subtree.nodes.contains_key(b"right1".as_ref()));
        assert_eq!(
            subtree.waitlist.iter().next().map(|k| k.as_slice()),
            Some(b"right1".as_ref())
        );
        assert_eq!(
            subtree.cluster_roots,
            [b"right2".to_vec(), b"left2".to_vec()].into_iter().collect()
        );

        // Adding (fetching) it back shall return the subtree into original state
        subtree.insert(
            b"right1".to_vec(),
            Node::new_item(b"right1_value".to_vec())
                .with_left_child(b"left2".to_vec())
                .with_right_child(b"right2".to_vec()),
        );

        assert_eq!(subtree, sample_tree());
    }

    #[test]
    fn model_populate_subtrees_chain() {
        let path_ctx = PathCtx::new();
        let mut model = Tree::new(&path_ctx);
        assert!(model.subtrees.is_empty());

        model.populate_subtrees_chain(path_ctx.add_iter([b"1", b"2", b"3", b"4"]));

        assert!(matches!(
            model
                .subtrees
                .get(&path_ctx.get_root())
                .unwrap()
                .nodes
                .first_key_value()
                .map(|(k, v)| (k.as_slice(), v))
                .unwrap(),
            (
                b"1",
                &Node {
                    element: Element::SubtreePlaceholder,
                    ..
                }
            )
        ));

        assert!(matches!(
            model
                .subtrees
                .get(&path_ctx.add_iter([b"1"]))
                .unwrap()
                .nodes
                .first_key_value()
                .map(|(k, v)| (k.as_slice(), v))
                .unwrap(),
            (
                b"2",
                &Node {
                    element: Element::SubtreePlaceholder,
                    ..
                }
            )
        ));

        assert!(model
            .subtrees
            .get(&path_ctx.add_iter([b"1", b"2", b"3", b"4"]))
            .unwrap()
            .nodes
            .first_key_value()
            .is_none());
    }

    #[test]
    fn model_insert_nested_sumtree_node_at_empty() {
        // Simulating the case when the first update is actually not a GroveDb root
        let path_ctx = PathCtx::new();
        let mut model = Tree::new(&path_ctx);

        // Insert two deeply nested nodes that share no path segment except root...
        model.insert(
            path_ctx.add_iter([b"hello", b"world"]),
            b"sumtree".to_vec(),
            Node::new_sumtree(b"yeet".to_vec().into(), 0),
        );

        model.insert(
            path_ctx.add_iter([b"top", b"kek"]),
            b"subtree".to_vec(),
            Node::new_subtree(b"swag".to_vec().into()),
        );

        // ...that means the root subtree will have two subtree placeholder
        // nodes, both will be cluster roots because no connections are yet known
        assert_eq!(
            model
                .subtrees
                .get(&path_ctx.get_root())
                .unwrap()
                .cluster_roots
                .len(),
            2
        );

        // Adding a node for a root subtree, that will have aforementioned
        // placeholder nodes as its left and right children
        model.insert(
            path_ctx.get_root(),
            b"very_root".to_vec(),
            Node::new_item(b"very_root_value".to_vec())
                .with_left_child(b"hello".to_vec())
                .with_right_child(b"top".to_vec()),
        );

        // And setting it as a root, so it will no longer be a cluster but a
        // proper tree root
        model
            .subtrees
            .get_mut(&path_ctx.get_root())
            .unwrap()
            .set_root(b"very_root".to_vec());

        assert!(model
            .subtrees
            .get(&path_ctx.get_root())
            .unwrap()
            .cluster_roots
            .is_empty());
    }
}
