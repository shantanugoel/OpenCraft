//! The handler of the kinds that ask the engine for work: change the page,
//! select a profile, open a folder, close a folder, toggle, sequence, screen
//! off, and device brightness.
//!
//! These handlers do no work on the computer. The kind modules in
//! [`crate::kinds`] read the settings and send requests to the engine with
//! [`opencraft_engine::Message::Engine`], through the same handler interface
//! as each other handler (architecture rule 8). The reasons are in
//! `docs/decisions/0016-pages-and-engine-requests-from-a-handler.md`,
//! `docs/decisions/0017-toggle-and-sequence-actions.md`, and
//! `docs/decisions/0018-many-profiles.md`.
//!
//! One [`RequestHandler`] serves one group. The navigation kinds share one
//! handler. The toggle, the sequence, and the device kinds each have one
//! handler of their own, so a long sequence does not delay a toggle.

use crate::backend::System;
use crate::kinds::problem;
use crate::kinds::{find_in, kinds_of, Group, Memory, Work};
use opencraft_engine::{Handler, KindDescription, MessageSender, Request};
use std::cell::RefCell;
use std::sync::Arc;

/// The handler for a [`Group`] of the kinds that ask the engine for work.
pub(crate) struct RequestHandler {
    group: Group,
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
    /// What the kinds of the group remember: the states of the toggles.
    memory: RefCell<Memory>,
}

impl RequestHandler {
    pub(crate) fn new(group: Group, system: Arc<dyn System>) -> Self {
        Self {
            group,
            system,
            sender: None,
            memory: RefCell::default(),
        }
    }
}

impl Handler for RequestHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        kinds_of(self.group)
            .map(|kind| kind.description())
            .collect()
    }

    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }

    fn handle(&mut self, request: Request) {
        match request {
            // The engine sends Appear again after each change of the page or
            // of the binding, and removes the old feedback at each change
            // (decision 0013).
            Request::Appear { binding, action } => {
                let Some(kind) = find_in(self.group, &action.kind) else {
                    return;
                };
                let work = Work::new(self.system.as_ref(), &self.sender)
                    .with_binding(binding)
                    .with_memory(&self.memory);
                kind.appear(&action.settings, &work);
            }
            Request::Invoke { binding, action } => {
                let Some(kind) = find_in(self.group, &action.kind) else {
                    return;
                };
                let work = Work::new(self.system.as_ref(), &self.sender)
                    .with_binding(binding)
                    .with_memory(&self.memory)
                    .with_shared_system(&self.system);
                problem(&self.sender, kind.invoke(&action.settings, &work));
            }
            _ => {}
        }
    }
}
