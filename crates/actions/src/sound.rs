//! The handler of the sound kinds: play a sound, and stop all sound. The
//! kind modules read the settings and call the System interface. This handler
//! keeps one queue for both kinds, so a stop does not pass a play that came
//! before it.
use crate::backend::System;
use crate::kinds::{find_in, kinds_of, Group, Work};
use crate::kinds::{problem, send};
use opencraft_engine::{Handler, KindDescription, Message, MessageSender, Request};
use std::sync::Arc;

pub(crate) struct SoundHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
}

impl SoundHandler {
    pub(crate) fn new(system: Arc<dyn System>) -> Self {
        Self {
            system,
            sender: None,
        }
    }
}

impl Handler for SoundHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        kinds_of(Group::Sound)
            .map(|kind| kind.description())
            .collect()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Choices { id, action, field } => {
                let Some(kind) = find_in(Group::Sound, &action.kind) else {
                    return;
                };
                let work = Work::new(self.system.as_ref(), &self.sender);
                let Some(choices) = kind.choices(&field, &work) else {
                    return;
                };
                if let Some(text) = choices.problem {
                    send(&self.sender, Message::Problem(text));
                }
                send(
                    &self.sender,
                    Message::Choices {
                        id,
                        options: choices.options,
                    },
                );
            }
            Request::Invoke { action, .. } => {
                let Some(kind) = find_in(Group::Sound, &action.kind) else {
                    return;
                };
                let work = Work::new(self.system.as_ref(), &self.sender);
                problem(&self.sender, kind.invoke(&action.settings, &work));
            }
            _ => {}
        }
    }
}
