use eframe::egui::{self, CollapsingHeader, ScrollArea};

use crate::{
    bus::{CommandBus, UserAction},
    bytes_utils::BytesView,
    path_ctx::{Path, PathCtx},
};

pub(crate) struct ProofViewer {
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

    pub(crate) fn draw<'pa>(&mut self, ui: &mut egui::Ui, bus: &CommandBus<'pa>, path_ctx: &'pa PathCtx) {
        ScrollArea::vertical().show(ui, |scroll| {
            self.prove_options.draw(scroll);
            scroll.separator();
            self.root_layer.draw(scroll, bus, path_ctx.get_root());
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

    fn draw<'pa>(&mut self, ui: &mut egui::Ui, bus: &CommandBus<'pa>, path: Path<'pa>) {
        ui.label("Merk proof:");
        self.merk_proof.draw(ui);

        ui.separator();

        for (key, layer) in self.lower_layers.iter_mut() {
            ui.horizontal(|line| {
                key.draw(line);
                if line
                    .button(egui_phosphor::regular::TREE_STRUCTURE)
                    .on_hover_text("Select subtree for Merk view")
                    .clicked()
                {
                    bus.user_action(UserAction::SelectMerkView(path.child(key.bytes.to_vec())));
                }
            });
            CollapsingHeader::new("Layer proof")
                .id_source(&key.bytes)
                .show(ui, |collapsing| {
                    layer.draw(collapsing, bus, path.child(key.bytes.clone()));
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

pub(crate) enum MerkProofOpViewer {
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

pub(crate) enum MerkProofNodeViewer {
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

impl From<grovedbg_types::MerkProofNode> for MerkProofNodeViewer {
    fn from(value: grovedbg_types::MerkProofNode) -> Self {
        match value {
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
}

impl MerkProofNodeViewer {
    fn new(node: grovedbg_types::MerkProofNode) -> Self {
        node.into()
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            match self {
                MerkProofNodeViewer::Hash(hash) => {
                    ui.horizontal(|line| {
                        line.label("Hash:");
                        hash.draw(line);
                    });
                }
                MerkProofNodeViewer::KVHash(hash) => {
                    ui.horizontal(|line| {
                        line.label("KVHash:");
                        hash.draw(line);
                    });
                }
                MerkProofNodeViewer::KVDigest(key, hash) => {
                    ui.label("KVDigest:");
                    ui.horizontal(|line| {
                        line.label("Key:");
                        key.draw(line);
                    });
                    ui.horizontal(|line| {
                        line.label("Value hash:");
                        hash.draw(line);
                    });
                }
                MerkProofNodeViewer::KV(key, value) => {
                    ui.label("KV:");
                    ui.horizontal(|line| {
                        line.label("Key:");
                        key.draw(line);
                    });
                    ui.label("Value:");
                    value.draw(ui);
                }
                MerkProofNodeViewer::KVValueHash(key, value, hash) => {
                    ui.label("KVValueHash:");
                    ui.horizontal(|line| {
                        line.label("Key:");
                        key.draw(line);
                    });
                    ui.label("Value:");
                    value.draw(ui);
                    ui.horizontal(|line| {
                        line.label("Value hash:");
                        hash.draw(line);
                    });
                }
                MerkProofNodeViewer::KVValueHashFeatureType(key, value, hash, ft) => {
                    ui.label("KVValueHashFeatureType:");
                    ui.horizontal(|line| {
                        line.label("Key:");
                        key.draw(line);
                    });
                    ui.label("Value:");
                    value.draw(ui);
                    ui.horizontal(|line| {
                        line.label("Value hash:");
                        hash.draw(line);
                    });
                    match ft {
                        grovedbg_types::TreeFeatureType::BasicMerkNode => ui.label("Basic merk node"),
                        grovedbg_types::TreeFeatureType::SummedMerkNode(x) => {
                            ui.label(format!("Summed merk node: {x}"))
                        }
                    };
                }
                MerkProofNodeViewer::KVRefValueHash(key, value, hash) => {
                    ui.label("KVRefValueHash:");
                    ui.horizontal(|line| {
                        line.label("Key:");
                        key.draw(line);
                    });
                    ui.label("Ref value:");
                    value.draw(ui);
                    ui.horizontal(|line| {
                        line.label("Value hash:");
                        hash.draw(line);
                    });
                }
            };
        });
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

pub(crate) enum ElementViewer {
    Subtree {
        root_key: Option<BytesView>,
        element_flags: Option<BytesView>,
    },
    Sumtree {
        root_key: Option<BytesView>,
        sum: i64,
        element_flags: Option<BytesView>,
    },
    Item {
        value: BytesView,
        element_flags: Option<BytesView>,
    },
    SumItem {
        value: i64,
        element_flags: Option<BytesView>,
    },
    AbsolutePathReference {
        path: Vec<BytesView>,
        element_flags: Option<BytesView>,
    },
    UpstreamRootHeightReference {
        n_keep: u32,
        path_append: Vec<BytesView>,
        element_flags: Option<BytesView>,
    },
    UpstreamRootHeightWithParentPathAdditionReference {
        n_keep: u32,
        path_append: Vec<BytesView>,
        element_flags: Option<BytesView>,
    },
    UpstreamFromElementHeightReference {
        n_remove: u32,
        path_append: Vec<BytesView>,
        element_flags: Option<BytesView>,
    },
    CousinReference {
        swap_parent: BytesView,
        element_flags: Option<BytesView>,
    },
    RemovedCousinReference {
        swap_parent: Vec<BytesView>,
        element_flags: Option<BytesView>,
    },
    SiblingReference {
        sibling_key: BytesView,
        element_flags: Option<BytesView>,
    },
}

impl ElementViewer {
    fn new(element: grovedbg_types::Element) -> Self {
        match element {
            grovedbg_types::Element::Subtree {
                root_key,
                element_flags,
            } => ElementViewer::Subtree {
                root_key: root_key.map(|k| BytesView::new(k)),
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::Sumtree {
                root_key,
                sum,
                element_flags,
            } => ElementViewer::Sumtree {
                root_key: root_key.map(|k| BytesView::new(k)),
                sum,
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::Item { value, element_flags } => ElementViewer::Item {
                value: BytesView::new(value),
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::SumItem { value, element_flags } => ElementViewer::SumItem {
                value,

                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::Reference(grovedbg_types::Reference::AbsolutePathReference {
                path,
                element_flags,
            }) => ElementViewer::AbsolutePathReference {
                path: path.into_iter().map(|s| BytesView::new(s)).collect(),
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::Reference(grovedbg_types::Reference::UpstreamRootHeightReference {
                n_keep,
                path_append,
                element_flags,
            }) => ElementViewer::UpstreamRootHeightReference {
                n_keep,
                path_append: path_append.into_iter().map(|s| BytesView::new(s)).collect(),
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::Reference(
                grovedbg_types::Reference::UpstreamRootHeightWithParentPathAdditionReference {
                    n_keep,
                    path_append,
                    element_flags,
                },
            ) => ElementViewer::UpstreamRootHeightWithParentPathAdditionReference {
                n_keep,
                path_append: path_append.into_iter().map(|s| BytesView::new(s)).collect(),
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::Reference(
                grovedbg_types::Reference::UpstreamFromElementHeightReference {
                    n_remove,
                    path_append,
                    element_flags,
                },
            ) => ElementViewer::UpstreamFromElementHeightReference {
                n_remove,
                path_append: path_append.into_iter().map(|s| BytesView::new(s)).collect(),
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::Reference(grovedbg_types::Reference::CousinReference {
                swap_parent,
                element_flags,
            }) => ElementViewer::CousinReference {
                swap_parent: BytesView::new(swap_parent),
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::Reference(grovedbg_types::Reference::RemovedCousinReference {
                swap_parent,
                element_flags,
            }) => ElementViewer::RemovedCousinReference {
                swap_parent: swap_parent.into_iter().map(|s| BytesView::new(s)).collect(),
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
            grovedbg_types::Element::Reference(grovedbg_types::Reference::SiblingReference {
                sibling_key,
                element_flags,
            }) => ElementViewer::SiblingReference {
                sibling_key: BytesView::new(sibling_key),
                element_flags: element_flags.map(|f| BytesView::new(f)),
            },
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        match self {
            ElementViewer::Subtree {
                root_key: Some(key),
                element_flags,
            } => {
                ui.label("Subtree");
                ui.horizontal(|line| {
                    line.label("Root key:");
                    key.draw(line);
                });
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::Subtree {
                root_key: None,
                element_flags,
            } => {
                ui.label("Empty subtree");
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::Sumtree {
                root_key: Some(key),
                sum,
                element_flags,
            } => {
                ui.label(format!("Sum tree: {sum}"));
                ui.horizontal(|line| {
                    line.label("Root key:");
                    key.draw(line);
                });
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::Sumtree {
                root_key: None,
                sum,
                element_flags,
            } => {
                ui.label(format!("Empty sum tree: {sum}"));
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::Item { value, element_flags } => {
                ui.label("Item");
                value.draw(ui);
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::SumItem { value, element_flags } => {
                ui.label(format!("Sum item: {value}"));
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::AbsolutePathReference { path, element_flags } => {
                ui.label("Absolute path reference");
                for (i, segment) in path.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::UpstreamRootHeightReference {
                n_keep,
                path_append,
                element_flags,
            } => {
                ui.label("Upstream root height reference");
                ui.label(format!("N keep: {n_keep}"));
                for (i, segment) in path_append.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::UpstreamRootHeightWithParentPathAdditionReference {
                n_keep,
                path_append,
                element_flags,
            } => {
                ui.label("Upstream root height with parent path addition reference");
                ui.label(format!("N keep: {n_keep}"));
                for (i, segment) in path_append.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::UpstreamFromElementHeightReference {
                n_remove,
                path_append,
                element_flags,
            } => {
                ui.label("Upstream from element height reference ");
                ui.label(format!("N remove: {n_remove}"));
                for (i, segment) in path_append.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::CousinReference {
                swap_parent,
                element_flags,
            } => {
                ui.label("Cousin reference");
                swap_parent.draw(ui);
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::RemovedCousinReference {
                swap_parent,
                element_flags,
            } => {
                ui.label("Removed cousin reference");
                for (i, segment) in swap_parent.iter_mut().enumerate() {
                    ui.horizontal(|line| {
                        line.label(i.to_string());
                        segment.draw(line);
                    });
                }
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
            ElementViewer::SiblingReference {
                sibling_key,
                element_flags,
            } => {
                ui.label("Sibling reference");
                sibling_key.draw(ui);
                if let Some(flags) = element_flags {
                    ui.horizontal(|line| {
                        line.label("Flags:");
                        flags.draw(line);
                    });
                }
            }
        }
    }
}
