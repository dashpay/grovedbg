//! Visualizer subsystem dedicated to simplify interactions between loosely
//! coupled components.

use std::{cell::RefCell, collections::VecDeque};

use grovedbg_types::Key;

use crate::{path_ctx::Path, protocol::ProtocolCommand, ProtocolSender};

pub(crate) struct CommandBus<'pa> {
    protocol_sender: ProtocolSender,
    actions_queue: RefCell<VecDeque<UserAction<'pa>>>,
}

#[derive(Clone)]
pub(crate) enum UserAction<'pa> {
    FocusSubtree(Path<'pa>),
    FocusSubtreeKey(Path<'pa>, Key),
    SelectMerkView(Path<'pa>),
}

impl<'pa> CommandBus<'pa> {
    pub(crate) fn new(protocol_sender: ProtocolSender) -> Self {
        Self {
            protocol_sender,
            actions_queue: Default::default(),
        }
    }

    pub(crate) fn protocol_command(&self, command: ProtocolCommand) {
        let _ = self
            .protocol_sender
            .blocking_send(command)
            .inspect_err(|_| log::error!("Unable to reach GroveDBG protocol thread"));
    }

    pub(crate) fn user_action(&self, action: UserAction<'pa>) {
        self.actions_queue.borrow_mut().push_back(action);
    }

    pub(crate) fn process_actions(&self, mut f: impl FnMut(UserAction<'pa>)) {
        let mut queue = self.actions_queue.borrow_mut();

        for action in queue.drain(..) {
            f(action)
        }
    }
}
