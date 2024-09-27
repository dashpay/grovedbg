use std::collections::{BTreeMap, VecDeque};

use anyhow::{anyhow, bail, Context};
use grovedbg_types::{Element, MerkProofNode, NodeUpdate, SessionId};
use reqwest::{Client, Url};

use super::{fetch_node, fetch_root_node};

#[derive(Clone, Debug)]
pub(crate) struct ProofNode {
    pub(crate) left: Option<usize>,
    pub(crate) right: Option<usize>,
    pub(crate) proof_value: MerkProofNode,
    pub(crate) node_update: Option<NodeUpdate>,
}

impl From<grovedbg_types::MerkProofNode> for ProofNode {
    fn from(value: grovedbg_types::MerkProofNode) -> Self {
        ProofNode {
            left: None,
            right: None,
            proof_value: value.into(),
            node_update: None,
        }
    }
}

pub(crate) struct ProofTree<'a> {
    pub(crate) tree: BTreeMap<Vec<Vec<u8>>, ProofSubtree>,
    client: &'a Client,
    address: &'a Url,
    session_id: SessionId,
}

impl<'a> ProofTree<'a> {
    pub(crate) async fn new(
        client: &'a Client,
        address: &'a Url,
        proof: grovedbg_types::Proof,
        session_id: SessionId,
    ) -> anyhow::Result<Self> {
        let mut queue = VecDeque::new();
        queue.push_back((vec![], proof.root_layer));

        let mut tree = BTreeMap::new();

        while let Some((path, proof)) = queue.pop_front() {
            let subtree_proof = ProofSubtree::from_iter(proof.merk_proof)?;
            tree.insert(path.clone(), subtree_proof);
            for (key, lower_proof) in proof.lower_layers.into_iter() {
                let mut lower_path = path.clone();
                lower_path.push(key);
                queue.push_back((lower_path, lower_proof));
            }
        }

        let idx = tree[[].as_slice()].root;
        let root_node = tree.get_mut([].as_slice()).unwrap().tree.get_mut(idx).unwrap();

        root_node.node_update = fetch_root_node(client, address, session_id).await?;

        Ok(Self {
            tree,
            client,
            address,
            session_id,
        })
    }

    async fn fetch_subtree(&mut self, path: Vec<Vec<u8>>) -> anyhow::Result<()> {
        let mut queue = VecDeque::new();
        queue.push_back(
            self.tree
                .get_mut(&path)
                .ok_or_else(|| anyhow!("missing subtree"))?
                .root,
        );

        while let Some(idx) = queue.pop_front() {
            let node = self
                .tree
                .get_mut(&path)
                .ok_or_else(|| anyhow!("missing subtree"))?
                .tree[idx]
                .clone();

            let Some(node_update) = node.node_update.as_ref().cloned() else {
                bail!("expected node data to be fetched before")
            };

            if let NodeUpdate {
                key,
                element:
                    Element::Subtree {
                        root_key: Some(root_key),
                        ..
                    }
                    | Element::Sumtree {
                        root_key: Some(root_key),
                        ..
                    },
                ..
            } = &node_update
            {
                let mut new_path = path.clone();
                new_path.push(key.clone());

                if let Some(subtree) = self.tree.get_mut(&new_path) {
                    subtree.tree[subtree.root].node_update = fetch_node(
                        self.client,
                        self.address,
                        self.session_id,
                        new_path,
                        root_key.clone(),
                    )
                    .await?;
                };
            }

            if let Some(proof_left) = node.left {
                queue.push_back(proof_left);
                let Some(left_child) = node_update.left_child else {
                    bail!("proof tree contains left child, but actual data doesn't")
                };
                let update = fetch_node(
                    self.client,
                    self.address,
                    self.session_id,
                    path.clone(),
                    left_child.clone(),
                )
                .await?;
                if let Some(NodeUpdate {
                    element:
                        Element::Subtree {
                            root_key: Some(root_key),
                            ..
                        }
                        | Element::Sumtree {
                            root_key: Some(root_key),
                            ..
                        },
                    ..
                }) = &update
                {
                    let mut new_path = path.clone();
                    new_path.push(left_child);
                    if let Some(subtree) = self.tree.get_mut(&new_path) {
                        subtree.tree[subtree.root].node_update = fetch_node(
                            self.client,
                            self.address,
                            self.session_id,
                            new_path,
                            root_key.clone(),
                        )
                        .await?;
                    };
                }

                self.tree
                    .get_mut(&path)
                    .ok_or_else(|| anyhow!("missing subtree"))?
                    .tree
                    .get_mut(proof_left)
                    .ok_or_else(|| anyhow!("proof data diverged from actual state 3"))?
                    .node_update = update;
            }

            if let Some(proof_right) = node.right {
                queue.push_back(proof_right);
                let Some(right_child) = node_update.right_child else {
                    bail!("proof tree contains right child, but actual data doesn't")
                };
                let update = fetch_node(
                    self.client,
                    self.address,
                    self.session_id,
                    path.clone(),
                    right_child.clone(),
                )
                .await?;
                if let Some(NodeUpdate {
                    element:
                        Element::Subtree {
                            root_key: Some(root_key),
                            ..
                        }
                        | Element::Sumtree {
                            root_key: Some(root_key),
                            ..
                        },
                    ..
                }) = &update
                {
                    let mut new_path = path.clone();
                    new_path.push(right_child);
                    if let Some(subtree) = self.tree.get_mut(&new_path) {
                        subtree.tree[subtree.root].node_update = fetch_node(
                            self.client,
                            self.address,
                            self.session_id,
                            new_path,
                            root_key.clone(),
                        )
                        .await?;
                    };
                }

                self.tree
                    .get_mut(&path)
                    .ok_or_else(|| anyhow!("missing subtree"))?
                    .tree
                    .get_mut(proof_right)
                    .ok_or_else(|| anyhow!("proof data diverged from actual state 3"))?
                    .node_update = update;
            }
        }

        Ok(())
    }

    pub(crate) async fn fetch_additional_data(&mut self) -> anyhow::Result<()> {
        let paths: Vec<_> = self.tree.keys().cloned().collect();
        for path in paths.into_iter() {
            self.fetch_subtree(path).await?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProofSubtree {
    pub(crate) tree: Vec<ProofNode>,
    pub(crate) root: usize,
}

impl ProofSubtree {
    pub(crate) fn to_proof_tree_data(self) -> BTreeMap<Vec<u8>, grovedbg_types::MerkProofNode> {
        self.tree
            .into_iter()
            .filter_map(
                |ProofNode {
                     proof_value,
                     node_update,
                     ..
                 }| node_update.map(|NodeUpdate { key, .. }| (key, proof_value)),
            )
            .collect()
    }

    pub(crate) fn from_iter<I>(iter: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = grovedbg_types::MerkProofOp>,
    {
        let mut stack: Vec<usize> = Vec::new();
        let mut tree: Vec<ProofNode> = Vec::new();

        for op in iter.into_iter() {
            match op {
                grovedbg_types::MerkProofOp::Push(x) => {
                    tree.push(x.into());
                    stack.push(tree.len() - 1);
                }
                grovedbg_types::MerkProofOp::PushInverted(x) => {
                    tree.push(x.into());
                    stack.push(tree.len() - 1);
                }
                grovedbg_types::MerkProofOp::Parent => {
                    // Pops the top stack item as `parent`. Pops the next top stack item as
                    // `child`. Attaches `child` as the left child of `parent`. Pushes the
                    // updated `parent` back on the stack.

                    let parent_idx = stack.pop().context("expected a parent item on the proof stack")?;
                    let child_idx = stack.pop().context("expected a child item on the proof stack")?;

                    tree[parent_idx].left = Some(child_idx);
                    stack.push(parent_idx);
                }
                grovedbg_types::MerkProofOp::Child => {
                    // Pops the top stack item as `child`. Pops the next top stack item as
                    // `parent`. Attaches `child` as the right child of `parent`. Pushes the
                    // updated `parent` back on the stack.

                    let child_idx = stack.pop().context("expected a child item on the proof stack")?;
                    let parent_idx = stack.pop().context("expected a parent item on the proof stack")?;

                    tree[parent_idx].right = Some(child_idx);
                    stack.push(parent_idx);
                }
                grovedbg_types::MerkProofOp::ParentInverted => {
                    // Pops the top stack item as `parent`. Pops the next top stack item as
                    // `child`. Attaches `child` as the right child of `parent`. Pushes the
                    // updated `parent` back on the stack.

                    let parent_idx = stack.pop().context("expected a parent item on the proof stack")?;
                    let child_idx = stack.pop().context("expected a child item on the proof stack")?;

                    tree[parent_idx].right = Some(child_idx);
                    stack.push(parent_idx);
                }
                grovedbg_types::MerkProofOp::ChildInverted => {
                    // Pops the top stack item as `child`. Pops the next top stack item as
                    // `parent`. Attaches `child` as the left child of `parent`. Pushes the
                    // updated `parent` back on the stack.

                    let child_idx = stack.pop().context("expected a child item on the proof stack")?;
                    let parent_idx = stack.pop().context("expected a parent item on the proof stack")?;

                    tree[parent_idx].left = Some(child_idx);
                    stack.push(parent_idx);
                }
            }
        }

        (stack.len() == 1)
            .then(|| ProofSubtree { tree, root: stack[0] })
            .ok_or_else(|| anyhow!("the proof stack must contain only one item"))
    }
}
