//! Nodes/Subtrees alignment implementation.

use super::Subtree;

const NODE_WIDTH: f32 = 150.;
pub(crate) const NODE_HEIGHT: f32 = 200.;
pub(crate) const COLLAPSED_SUBTREE_WIDTH: f32 = 400.;
pub(crate) const COLLAPSED_SUBTREE_HEIGHT: f32 = 600.;

fn leaves_level_count(n_levels: u32) -> u32 {
    if n_levels > 0 {
        2u32.pow(n_levels - 1)
    } else {
        0
    }
}

pub(super) fn expanded_subtree_dimentions(subtree: &Subtree) -> (f32, f32, u32, u32) {
    if let Some(root_node) = subtree.root_node() {
        let mut queue = vec![(1, root_node)];
        let mut levels = 0;
        while let Some((level, node)) = queue.pop() {
            levels = levels.max(level);
            let state = node.ui_state.borrow();
            state
                .show_left
                .then_some(node.left_child.as_ref())
                .flatten()
                .and_then(|key| subtree.nodes.get(key))
                .into_iter()
                .for_each(|node| queue.push((level + 1, node)));
            state
                .show_right
                .then_some(node.right_child.as_ref())
                .flatten()
                .and_then(|key| subtree.nodes.get(key))
                .into_iter()
                .for_each(|node| queue.push((level + 1, node)));
        }
        let leafs = leaves_level_count(levels);

        (
            leafs as f32 * NODE_WIDTH,
            levels as f32 * NODE_HEIGHT,
            levels,
            leafs,
        )
    } else {
        (0., 0., 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaves_level_count() {
        assert_eq!(leaves_level_count(0), 0);
        assert_eq!(leaves_level_count(1), 1);
        assert_eq!(leaves_level_count(2), 2);
        assert_eq!(leaves_level_count(3), 4);
        assert_eq!(leaves_level_count(4), 8);
    }
}
