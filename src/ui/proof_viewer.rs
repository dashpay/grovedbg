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

struct MerkProofOpViewer {
    op: grovedbg_types::MerkProofOp,
    node_viewer: Option<MerkProofNodeViewer>,
}

impl MerkProofOpViewer {
    fn new(op: grovedbg_types::MerkProofOp) -> Self {
        Self {
            node_viewer: match &op {
                grovedbg_types::MerkProofOp::Push(node) => Some(MerkProofNodeViewer::new(node.clone())),
                grovedbg_types::MerkProofOp::PushInverted(node) => {
                    Some(MerkProofNodeViewer::new(node.clone()))
                }
                _ => None,
            },
            op,
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        match &mut self.op {
            grovedbg_types::MerkProofOp::Push(node) => {
                ui.horizontal(|line| {
                    line.label("Push:");
                    self.node_viewer.as_mut().unwrap().draw(line);
                });
            }
            grovedbg_types::MerkProofOp::PushInverted(node) => {
                ui.horizontal(|line| {
                    line.label("Push inverted:");
                    self.node_viewer.as_mut().unwrap().draw(line);
                });
            }
            grovedbg_types::MerkProofOp::Parent => {
                ui.label("Parent");
            }
            grovedbg_types::MerkProofOp::Child => {
                ui.label("Child");
            }
            grovedbg_types::MerkProofOp::ParentInverted => {
                ui.label("ParentInverted");
            }
            grovedbg_types::MerkProofOp::ChildInverted => {
                ui.label("ChildInverted");
            }
        };
    }
}

enum MerkProofNodeViewer {
    Hash(BytesView),
    KVHash(BytesView),
    KVDigest(BytesView, BytesView),
    KV(BytesView, BytesView),
    KVValueHash(BytesView, BytesView, BytesView),
    KVValueHashFeatureType(BytesView, BytesView, BytesView, grovedbg_types::TreeFeatureType),
    KVRefValueHash(BytesView, BytesView, BytesView),
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
            grovedbg_types::MerkProofNode::KV(key, value) => {
                MerkProofNodeViewer::KV(BytesView::new(key), BytesView::new(value))
            }
            grovedbg_types::MerkProofNode::KVValueHash(key, value, hash) => MerkProofNodeViewer::KVValueHash(
                BytesView::new(key),
                BytesView::new(value),
                BytesView::new(hash.to_vec()),
            ),
            grovedbg_types::MerkProofNode::KVValueHashFeatureType(key, value, hash, ft) => {
                MerkProofNodeViewer::KVValueHashFeatureType(
                    BytesView::new(key),
                    BytesView::new(value),
                    BytesView::new(hash.to_vec()),
                    ft,
                )
            }
            grovedbg_types::MerkProofNode::KVRefValueHash(key, value, hash) => {
                MerkProofNodeViewer::KVRefValueHash(
                    BytesView::new(key),
                    BytesView::new(value),
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
            bytes,
            display_variant: DisplayVariant::Hex,
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
