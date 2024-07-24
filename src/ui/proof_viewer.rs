use eframe::egui::{self, CollapsingHeader, ScrollArea};

use super::{common::binary_label, DisplayVariant};

const MARGIN: f32 = 20.;

pub(crate) struct ProofViewer {
    // proof: grovedbg_types::Proof,
    prove_options: ProveOptionsView,
    root_layer: ProofLayerView,
}

impl ProofViewer {
    pub(crate) fn new(proof: grovedbg_types::Proof) -> Self {
        ProofViewer {
            prove_options: ProveOptionsView::new(proof.prove_options),
            root_layer: ProofLayerView::new(proof.root_layer),
        }
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |scroll| {
            self.prove_options.draw(scroll);
            scroll.separator();
            self.root_layer.draw(scroll);
        });
    }
}

struct ProofLayerView {
    merk_proof: MerkProofViewer,
    lower_layers: Vec<(BytesView, ProofLayerView)>,
}

impl ProofLayerView {
    fn new(layer: grovedbg_types::ProofLayer) -> Self {
        Self {
            merk_proof: MerkProofViewer::new(layer.merk_proof),
            lower_layers: layer
                .lower_layers
                .into_iter()
                .map(|(k, v)| (BytesView::new(k), ProofLayerView::new(v)))
                .collect(),
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.label("Merk proof:");
        self.merk_proof.draw(ui);

        ui.separator();

        for (key, layer) in self.lower_layers.iter_mut() {
            key.draw(ui);
            CollapsingHeader::new("Layer proof")
                .id_source(&key.bytes)
                .show(ui, |collapsing| {
                    layer.draw(collapsing);
                });
        }
    }
}

struct MerkProofViewer {
    merk_proof: Vec<MerkProofOpViewer>,
}

impl MerkProofViewer {
    fn new(merk_proof: Vec<grovedbg_types::MerkProofOp>) -> Self {
        Self {
            merk_proof: merk_proof
                .into_iter()
                .map(|op| MerkProofOpViewer::new(op))
                .collect(),
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        for op in self.merk_proof.iter_mut() {
            op.draw(ui);
        }
    }
}

enum MerkProofOpViewer {
    Push(MerkProofNodeViewer),
    PushInverted(MerkProofNodeViewer),
    Parent,
    Child,
    ParentInverted,
    ChildInverted,
}

impl MerkProofOpViewer {
    fn new(op: grovedbg_types::MerkProofOp) -> Self {
        match op {
            grovedbg_types::MerkProofOp::Push(node) => {
                MerkProofOpViewer::Push(MerkProofNodeViewer::new(node))
            }
            grovedbg_types::MerkProofOp::PushInverted(node) => {
                MerkProofOpViewer::PushInverted(MerkProofNodeViewer::new(node))
            }
            grovedbg_types::MerkProofOp::Parent => MerkProofOpViewer::Parent,
            grovedbg_types::MerkProofOp::Child => MerkProofOpViewer::Child,
            grovedbg_types::MerkProofOp::ParentInverted => MerkProofOpViewer::ParentInverted,
            grovedbg_types::MerkProofOp::ChildInverted => MerkProofOpViewer::ChildInverted,
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        match self {
            MerkProofOpViewer::Push(node) => {
                ui.horizontal(|line| {
                    line.label("Push:");
                    node.draw(line);
                });
            }
            MerkProofOpViewer::PushInverted(node) => {
                ui.horizontal(|line| {
                    line.label("Push inverted:");
                    node.draw(line);
                });
            }
            MerkProofOpViewer::Parent => {
                ui.label("Parent");
            }
            MerkProofOpViewer::Child => {
                ui.label("Child");
            }
            MerkProofOpViewer::ParentInverted => {
                ui.label("ParentInverted");
            }
            MerkProofOpViewer::ChildInverted => {
                ui.label("ChildInverted");
            }
        };
    }
}

enum MerkProofNodeViewer {
    Hash(BytesView),
    KVHash(BytesView),
    KVDigest(BytesView, BytesView),
    KV(BytesView, ElementViewer),
    KVValueHash(BytesView, ElementViewer, BytesView),
    KVValueHashFeatureType(
        BytesView,
        ElementViewer,
        BytesView,
        grovedbg_types::TreeFeatureType,
    ),
    KVRefValueHash(BytesView, ElementViewer, BytesView),
}

impl MerkProofNodeViewer {
    fn new(node: grovedbg_types::MerkProofNode) -> Self {
        match node {
            grovedbg_types::MerkProofNode::Hash(hash) => {
                MerkProofNodeViewer::Hash(BytesView::new(hash.to_vec()))
            }
            grovedbg_types::MerkProofNode::KVHash(hash) => {
                MerkProofNodeViewer::KVHash(BytesView::new(hash.to_vec()))
            }
            grovedbg_types::MerkProofNode::KVDigest(key, hash) => {
                MerkProofNodeViewer::KVDigest(BytesView::new(key), BytesView::new(hash.to_vec()))
            }
            grovedbg_types::MerkProofNode::KV(key, element) => {
                MerkProofNodeViewer::KV(BytesView::new(key), ElementViewer::new(element))
            }
            grovedbg_types::MerkProofNode::KVValueHash(key, element, hash) => {
                MerkProofNodeViewer::KVValueHash(
                    BytesView::new(key),
                    ElementViewer::new(element),
                    BytesView::new(hash.to_vec()),
                )
            }
            grovedbg_types::MerkProofNode::KVValueHashFeatureType(key, element, hash, ft) => {
                MerkProofNodeViewer::KVValueHashFeatureType(
                    BytesView::new(key),
                    ElementViewer::new(element),
                    BytesView::new(hash.to_vec()),
                    ft,
                )
            }
            grovedbg_types::MerkProofNode::KVRefValueHash(key, element, hash) => {
                MerkProofNodeViewer::KVRefValueHash(
                    BytesView::new(key),
                    ElementViewer::new(element),
                    BytesView::new(hash.to_vec()),
                )
            }
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        match self {
            MerkProofNodeViewer::Hash(hash) => ui.vertical(|line| {
                line.label("Hash:");
                hash.draw(line);
            }),
            MerkProofNodeViewer::KVHash(hash) => ui.vertical(|line| {
                line.label("KVHash:");
                hash.draw(line);
            }),
            MerkProofNodeViewer::KVDigest(key, hash) => ui.vertical(|line| {
                line.label("KVDigest:");
                key.draw(line);
                hash.draw(line);
            }),
            MerkProofNodeViewer::KV(key, value) => ui.vertical(|line| {
                line.label("KV:");
                key.draw(line);
                value.draw(line);
            }),
            MerkProofNodeViewer::KVValueHash(key, value, hash) => ui.vertical(|line| {
                line.label("KVValueHash:");
                key.draw(line);
                value.draw(line);
                hash.draw(line);
            }),
            MerkProofNodeViewer::KVValueHashFeatureType(key, value, hash, ft) => ui.vertical(|line| {
                line.label("KVValueHashFeatureType:");
                key.draw(line);
                value.draw(line);
                hash.draw(line);
                match ft {
                    grovedbg_types::TreeFeatureType::BasicMerkNode => line.label("Basic merk node"),
                    grovedbg_types::TreeFeatureType::SummedMerkNode(x) => {
                        line.label(format!("Summed merk node: {x}"))
                    }
                };
            }),
            MerkProofNodeViewer::KVRefValueHash(key, value, hash) => ui.vertical(|line| {
                line.label("KVRefValueHash:");
                key.draw(line);
                value.draw(line);
                hash.draw(line);
            }),
        };
    }
}

struct BytesView {
    bytes: Vec<u8>,
    display_variant: DisplayVariant,
}

impl BytesView {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            display_variant: DisplayVariant::guess(&bytes),
            bytes,
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        binary_label(ui, &self.bytes, &mut self.display_variant);
    }
}

struct ProveOptionsView {
    prove_options: grovedbg_types::ProveOptions,
}

impl ProveOptionsView {
    fn new(prove_options: grovedbg_types::ProveOptions) -> Self {
        Self { prove_options }
    }

    fn draw(&self, ui: &mut egui::Ui) {
        ui.label("Prove options: ");

        ui.horizontal(|line| {
            line.label("Decrease limit on empty sub query result:");
            line.label(
                self.prove_options
                    .decrease_limit_on_empty_sub_query_result
                    .to_string(),
            );
        });
    }
}

enum ElementViewer {
    Subtree {
        root_key: Option<BytesView>,
    },
    Sumtree {
        root_key: Option<BytesView>,
        sum: i64,
    },
    Item {
        value: BytesView,
    },
    SumItem {
        value: i64,
    },
    AbsolutePathReference {
        path: Vec<BytesView>,
    },
    UpstreamRootHeightReference {
        n_keep: u32,
        path_append: Vec<BytesView>,
    },
    UpstreamRootHeightWithParentPathAdditionReference {
        n_keep: u32,
        path_append: Vec<BytesView>,
    },
    UpstreamFromElementHeightReference {
        n_remove: u32,
        path_append: Vec<BytesView>,
    },
    CousinReference {
        swap_parent: BytesView,
    },
    RemovedCousinReference {
        swap_parent: Vec<BytesView>,
    },
    SiblingReference {
        sibling_key: BytesView,
    },
}

impl ElementViewer {
    fn new(element: grovedbg_types::Element) -> Self {
        match element {
            grovedbg_types::Element::Subtree { root_key } => ElementViewer::Subtree {
                root_key: root_key.map(|k| BytesView::new(k)),
            },
            grovedbg_types::Element::Sumtree { root_key, sum } => ElementViewer::Sumtree {
                root_key: root_key.map(|k| BytesView::new(k)),
                sum,
            },
            grovedbg_types::Element::Item { value } => ElementViewer::Item {
                value: BytesView::new(value),
            },
            grovedbg_types::Element::SumItem { value } => ElementViewer::SumItem { value },
            grovedbg_types::Element::AbsolutePathReference { path } => ElementViewer::AbsolutePathReference {
                path: path.into_iter().map(|s| BytesView::new(s)).collect(),
            },
            grovedbg_types::Element::UpstreamRootHeightReference { n_keep, path_append } => {
                ElementViewer::UpstreamRootHeightReference {
                    n_keep,
                    path_append: path_append.into_iter().map(|s| BytesView::new(s)).collect(),
                }
            }
            grovedbg_types::Element::UpstreamRootHeightWithParentPathAdditionReference {
                n_keep,
                path_append,
            } => ElementViewer::UpstreamRootHeightWithParentPathAdditionReference {
                n_keep,
                path_append: path_append.into_iter().map(|s| BytesView::new(s)).collect(),
            },
            grovedbg_types::Element::UpstreamFromElementHeightReference {
                n_remove,
                path_append,
            } => ElementViewer::UpstreamFromElementHeightReference {
                n_remove,
                path_append: path_append.into_iter().map(|s| BytesView::new(s)).collect(),
            },
            grovedbg_types::Element::CousinReference { swap_parent } => ElementViewer::CousinReference {
                swap_parent: BytesView::new(swap_parent),
            },
            grovedbg_types::Element::RemovedCousinReference { swap_parent } => {
                ElementViewer::RemovedCousinReference {
                    swap_parent: swap_parent.into_iter().map(|s| BytesView::new(s)).collect(),
                }
            }
            grovedbg_types::Element::SiblingReference { sibling_key } => ElementViewer::SiblingReference {
                sibling_key: BytesView::new(sibling_key),
            },
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        match self {
            ElementViewer::Subtree { root_key: Some(key) } => {
                ui.label("Subtree");
                ui.horizontal(|line| {
                    line.label("Root key:");
                    key.draw(line);
                });
            }
            ElementViewer::Subtree { root_key: None } => {
                ui.label("Empty subtree");
            }
            ElementViewer::Sumtree {
                root_key: Some(key),
                sum,
            } => {
                ui.label(format!("Sum tree: {sum}"));
                ui.horizontal(|line| {
                    line.label("Root key:");
                    key.draw(line);
                });
            }
            ElementViewer::Sumtree { root_key: None, sum } => {
                ui.label(format!("Empty sum tree: {sum}"));
            }
            ElementViewer::Item { value } => {
                ui.label("Item");
                value.draw(ui);
            }
            ElementViewer::SumItem { value } => {
                ui.label(format!("Sum item: {value}"));
            }
            ElementViewer::AbsolutePathReference { path } => {
                ui.label("Absolute path reference");
                for (i, segment) in path.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
            }
            ElementViewer::UpstreamRootHeightReference { n_keep, path_append } => {
                ui.label("Upstream root height reference");
                ui.label(format!("N keep: {n_keep}"));
                for (i, segment) in path_append.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
            }
            ElementViewer::UpstreamRootHeightWithParentPathAdditionReference { n_keep, path_append } => {
                ui.label("Upstream root height with parent path addition reference");
                ui.label(format!("N keep: {n_keep}"));
                for (i, segment) in path_append.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
            }
            ElementViewer::UpstreamFromElementHeightReference {
                n_remove,
                path_append,
            } => {
                ui.label("Upstream from element height reference ");
                ui.label(format!("N remove: {n_remove}"));
                for (i, segment) in path_append.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
            }
            ElementViewer::CousinReference { swap_parent } => {
                ui.label("Cousin reference");
                swap_parent.draw(ui);
            }
            ElementViewer::RemovedCousinReference { swap_parent } => {
                ui.label("Removed cousin reference");
                for (i, segment) in swap_parent.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
            }
            ElementViewer::SiblingReference { sibling_key } => {
                ui.label("Sibling reference");
                sibling_key.draw(ui);
            }
        }
    }
}
