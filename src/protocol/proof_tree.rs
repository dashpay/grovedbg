use std::collections::{BTreeMap, VecDeque};

use anyhow::{anyhow, Context};
use grovedbg_types::{MerkProofNode, NodeUpdate};
use reqwest::{Client, Url};

use super::fetch_root_node;

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
}

impl<'a> ProofTree<'a> {
    pub(crate) async fn new(
        client: &'a Client,
        address: &'a Url,
        proof: grovedbg_types::Proof,
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

        root_node.node_update = fetch_root_node(client, address).await?;

        Ok(Self {
            tree,
            client,
            address,
        })
    }

    fn fetch_additional_data(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub(crate) struct ProofSubtree {
    pub(crate) tree: Vec<ProofNode>,
    pub(crate) root: usize,
}

impl ProofSubtree {
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
