//! Audio actions. The System port owns platform work and property listeners.

use crate::backend::System;
use crate::descriptions::{description, MICROPHONE};
use crate::handlers::{problem, send};
use opencraft_engine::{
    BindingId, Feedback, FeedbackState, Handler, KindDescription, Message, MessageSender, Request,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Default)]
struct InputState {
    bindings: BTreeMap<BindingId, Option<Feedback>>,
    problem: Option<String>,
}

impl InputState {
    fn refresh(
        &mut self,
        system: &dyn System,
        sender: &Option<MessageSender>,
        force: Option<BindingId>,
    ) {
        let muted = match system.microphone_muted() {
            Ok(value) => {
                self.problem = None;
                value
            }
            Err(error) => {
                let text = format!("Cannot read the microphone mute: {error}.");
                if self.problem.as_ref() != Some(&text) {
                    send(sender, Message::Problem(text.clone()));
                    self.problem = Some(text);
                }
                None
            }
        };
        let feedback = Feedback {
            label: Some(
                match muted {
                    Some(true) => "Microphone muted",
                    Some(false) => "Microphone on",
                    None => "Not supported",
                }
                .into(),
            ),
            state: muted.map(|muted| {
                if muted {
                    FeedbackState::On
                } else {
                    FeedbackState::Off
                }
            }),
            ..Default::default()
        };
        for (binding, previous) in &mut self.bindings {
            if previous.as_ref() != Some(&feedback) || force == Some(*binding) {
                send(
                    sender,
                    Message::Feedback {
                        binding: *binding,
                        feedback: Some(feedback.clone()),
                    },
                );
                *previous = Some(feedback.clone());
            }
        }
        if let Some(binding) = force.filter(|id| !self.bindings.contains_key(id)) {
            send(
                sender,
                Message::Feedback {
                    binding,
                    feedback: Some(feedback),
                },
            );
        }
    }
}

pub(crate) struct AudioWorker {
    wake: mpsc::Sender<()>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl AudioWorker {
    pub(crate) fn spawn(
        name: &str,
        wake: mpsc::Sender<()>,
        changed: mpsc::Receiver<()>,
        mut watch: Box<dyn crate::system::AudioWatch>,
        mut refresh: impl FnMut(crate::system::Result<()>) + Send + 'static,
    ) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = stop.clone();
        let thread = std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                while changed.recv().is_ok() && !cancelled.load(Ordering::Acquire) {
                    refresh(watch.refresh());
                }
            })?;
        Ok(Self {
            wake,
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for AudioWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.wake.send(());
        // The worker drops its listener before it ends. No callback can send
        // feedback after this join, or hold the handler alive.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(crate) struct MicrophoneHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
    state: Arc<Mutex<InputState>>,
    worker: Option<AudioWorker>,
}
impl MicrophoneHandler {
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
        let watch = match self.system.watch_microphone(wake.clone()) {
            Ok(watch) => watch,
            Err(error) => {
                send(
                    &self.sender,
                    Message::Problem(format!("Cannot follow the microphone mute: {error}.")),
                );
                return;
            }
        };
        let system = self.system.clone();
        let sender = self.sender.clone();
        let state = self.state.clone();
        // A callback does no Core Audio work. The worker reattaches after a
        // change of the default input, then reads and sends only changed feedback.
        match AudioWorker::spawn(
            "opencraft-input-mute",
            wake,
            changed,
            watch,
            move |result| {
                if let Err(error) = result {
                    send(
                        &sender,
                        Message::Problem(format!("Cannot follow the microphone mute: {error}.")),
                    );
                }
                state.lock().unwrap_or_else(|e| e.into_inner()).refresh(
                    system.as_ref(),
                    &sender,
                    None,
                );
            },
        ) {
            Ok(worker) => self.worker = Some(worker),
            Err(error) => send(
                &self.sender,
                Message::Problem(format!("Cannot follow the microphone mute: {error}.")),
            ),
        }
    }
}
impl Handler for MicrophoneHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![description(MICROPHONE)]
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Appear { binding, action } if action.kind == MICROPHONE => {
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
            Request::Invoke { binding, action } if action.kind == MICROPHONE => {
                let result = self
                    .system
                    .microphone_muted()
                    .and_then(|muted| match muted {
                        Some(muted) => self.system.set_microphone_muted(!muted),
                        None => Err(crate::system::Error::NoInputMute),
                    })
                    .map_err(|e| format!("Cannot set the microphone mute: {e}."));
                problem(&self.sender, result);
                self.state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .refresh(self.system.as_ref(), &self.sender, Some(binding));
            }
            _ => {}
        }
    }
}
