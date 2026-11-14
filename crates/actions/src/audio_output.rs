//! Set one sound output or cycle an ordered list of stable output UIDs.
use crate::audio::AudioWorker;
use crate::backend::System;
use crate::handlers::{problem, send};
use crate::kinds::{find_in, kinds_of, sound_output, Group, Work};
use opencraft_engine::{
    BindingId, Feedback, Handler, KindDescription, Message, MessageSender, Request,
};
use std::collections::BTreeMap;
use std::sync::{mpsc, Arc, Mutex};

#[derive(Default)]
struct State {
    bindings: BTreeMap<BindingId, Option<String>>,
    problem: Option<String>,
}
impl State {
    fn refresh(
        &mut self,
        system: &dyn System,
        sender: &Option<MessageSender>,
        force: Option<BindingId>,
    ) {
        let name = match system.default_output() {
            Ok(device) => {
                self.problem = None;
                device.name
            }
            Err(error) => {
                let text = sound_output::read_problem(error);
                if self.problem.as_ref() != Some(&text) {
                    send(sender, Message::Problem(text.clone()));
                    self.problem = Some(text);
                }
                "Not available".into()
            }
        };
        for (binding, previous) in &mut self.bindings {
            if previous.as_ref() != Some(&name) || force == Some(*binding) {
                feedback(sender, *binding, &name);
                *previous = Some(name.clone());
            }
        }
        if let Some(binding) = force.filter(|id| !self.bindings.contains_key(id)) {
            feedback(sender, binding, &name);
        }
    }
}
fn feedback(sender: &Option<MessageSender>, binding: BindingId, name: &str) {
    send(
        sender,
        Message::Feedback {
            binding,
            feedback: Some(Feedback {
                label: Some(name.into()),
                ..Default::default()
            }),
        },
    );
}

pub(crate) struct OutputHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
    state: Arc<Mutex<State>>,
    worker: Option<AudioWorker>,
}
impl OutputHandler {
    pub(crate) fn new(system: Arc<dyn System>) -> Self {
        Self {
            system,
            sender: None,
            state: Default::default(),
            worker: None,
        }
    }
    fn watch(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let (wake, changed) = mpsc::channel();
        let watch = match self.system.watch_output(wake.clone()) {
            Ok(watch) => watch,
            Err(error) => {
                send(
                    &self.sender,
                    Message::Problem(sound_output::follow_problem(error)),
                );
                return;
            }
        };
        let system = self.system.clone();
        let sender = self.sender.clone();
        let state = self.state.clone();
        match AudioWorker::spawn("opencraft-output", wake, changed, watch, move |result| {
            if let Err(error) = result {
                send(
                    &sender,
                    Message::Problem(sound_output::follow_problem(error)),
                );
            }
            state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .refresh(system.as_ref(), &sender, None);
        }) {
            Ok(worker) => self.worker = Some(worker),
            Err(error) => send(
                &self.sender,
                Message::Problem(sound_output::follow_problem(error)),
            ),
        }
    }
}
impl Handler for OutputHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        kinds_of(Group::Output)
            .map(|kind| kind.description())
            .collect()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Choices { id, action, field } => {
                let Some(kind) = find_in(Group::Output, &action.kind) else {
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
            Request::Appear { binding, action }
                if find_in(Group::Output, &action.kind).is_some() =>
            {
                self.state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .bindings
                    .insert(binding, None);
                self.watch();
                self.state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .refresh(self.system.as_ref(), &self.sender, Some(binding));
            }
            Request::Disappear { binding } => {
                let empty = {
                    let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                    state.bindings.remove(&binding);
                    state.bindings.is_empty()
                };
                if empty {
                    self.worker = None;
                }
            }
            Request::Invoke { binding, action } => {
                let Some(kind) = find_in(Group::Output, &action.kind) else {
                    return;
                };
                let work = Work::new(self.system.as_ref(), &self.sender);
                problem(&self.sender, kind.invoke(&action.settings, &work));
                self.state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .refresh(self.system.as_ref(), &self.sender, Some(binding));
            }
            _ => {}
        }
    }
}
