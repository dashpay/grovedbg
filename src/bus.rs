//! Visualizer subsystem dedicated to simplify interactions between loosely
//! coupled components.

use std::{cell::RefCell, collections::VecDeque};

use grovedbg_types::{Key, SessionId};

use crate::{
    path_ctx::Path,
    protocol::{FetchCommand, ProtocolCommand},
    ProtocolSender,
};

pub(crate) struct CommandBus<'pa> {
    session: RefCell<Option<SessionId>>,
    protocol_sender: ProtocolSender,
    actions_queue: RefCell<VecDeque<UserAction<'pa>>>,
}

#[derive(Clone)]
pub(crate) enum UserAction<'pa> {
    FocusSubtree(Path<'pa>),
    FocusSubtreeKey(Path<'pa>, Key),
    DropFocus,
    SelectMerkView(Path<'pa>),
}

impl<'pa> CommandBus<'pa> {
    pub(crate) fn new(protocol_sender: ProtocolSender) -> Self {
        Self {
            session: Default::default(),
            protocol_sender,
            actions_queue: Default::default(),
        }
    }

    pub(crate) fn new_session(&self) {
        let _ = self
            .protocol_sender
            .blocking_send(ProtocolCommand::NewSession {
                old_session: self.session.take(),
            })
            .inspect_err(|_| log::error!("Unable to reach GroveDBG protocol thread"));
    }

    pub(crate) fn set_session(&self, session_id: SessionId) {
        *self.session.borrow_mut() = Some(session_id);
    }

    pub(crate) fn fetch_command(&self, command: FetchCommand) {
        if let Some(session_id) = self.session.borrow().as_ref() {
            let _ = self
                .protocol_sender
                .blocking_send(ProtocolCommand::Fetch {
                    session_id: *session_id,
                    command,
                })
                .inspect_err(|_| log::error!("Unable to reach GroveDBG protocol thread"));
        } else {
            log::warn!("Need to start a session first");
        }
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
