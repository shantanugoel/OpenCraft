//! Set one sound output or cycle an ordered list of stable output UIDs.
use crate::audio::AudioWorker;
use crate::backend::System;
use crate::descriptions::{description, setting_problem, OUTPUT};
use crate::handlers::{problem, send};
use opencraft_engine::{
    Action, BindingId, ChoiceOption, Feedback, Handler, KindDescription, Message, MessageSender,
    Request, Value,
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
                let text = format!("Cannot read the sound output: {error}.");
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
    fn invoke(&self, action: &Action) -> crate::handlers::Result<()> {
        let operation = match action.settings.get("operation") {
            None => "set",
            Some(Value::Text(v)) => v,
            _ => return Err(setting_problem(OUTPUT, "operation")),
        };
        let devices = self
            .system
            .output_devices()
            .map_err(|e| format!("Cannot list the sound outputs: {e}."))?;
        let id = match operation {
            "set" => match action.settings.get("device") {
                Some(Value::Text(value)) if devices.iter().any(|d| d.id == *value) => value.clone(),
                Some(Value::Text(value)) if !value.is_empty() => {
                    return Err("The selected sound output is not available.".into())
                }
                _ => return Err(setting_problem(OUTPUT, "device")),
            },
            "next" => {
                let selected = match action.settings.get("devices") {
                    None => Vec::new(),
                    Some(Value::List(values)) => values
                        .iter()
                        .map(|v| match v {
                            Value::Text(value) => Ok(value.clone()),
                            _ => Err(setting_problem(OUTPUT, "devices")),
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    _ => return Err(setting_problem(OUTPUT, "devices")),
                };
                // Keep saved unavailable UIDs, but skip them for this invocation.
                let ids: Vec<_> = if selected.is_empty() {
                    devices.iter().map(|d| d.id.clone()).collect()
                } else {
                    selected
                        .into_iter()
                        .filter(|id| devices.iter().any(|d| d.id == *id))
                        .collect()
                };
                if ids.is_empty() {
                    return Err("No selected sound output is available.".into());
                }
                let current = self
                    .system
                    .default_output()
                    .map_err(|e| format!("Cannot read the sound output: {e}."))?;
                let next = ids
                    .iter()
                    .position(|id| *id == current.id)
                    .map_or(0, |i| (i + 1) % ids.len());
                ids[next].clone()
            }
            _ => return Err(setting_problem(OUTPUT, "operation")),
        };
        self.system
            .set_output(&id)
            .map_err(|e| format!("Cannot set the sound output: {e}."))
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
                    Message::Problem(format!("Cannot follow the sound output: {error}.")),
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
                    Message::Problem(format!("Cannot follow the sound output: {error}.")),
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
                Message::Problem(format!("Cannot follow the sound output: {error}.")),
            ),
        }
    }
}
impl Handler for OutputHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![description(OUTPUT)]
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Choices { id, action, field }
                if action.kind == OUTPUT && ["device", "devices"].contains(&field.as_str()) =>
            {
                let options = match self.system.output_devices() {
                    Ok(devices) => devices
                        .into_iter()
                        .map(|d| ChoiceOption {
                            value: d.id,
                            label: d.name,
                        })
                        .collect(),
                    Err(error) => {
                        send(
                            &self.sender,
                            Message::Problem(format!("Cannot list the sound outputs: {error}.")),
                        );
                        Vec::new()
                    }
                };
                send(&self.sender, Message::Choices { id, options });
            }
            Request::Appear { binding, action } if action.kind == OUTPUT => {
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
            Request::Invoke { binding, action } if action.kind == OUTPUT => {
                problem(&self.sender, self.invoke(&action));
                self.state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .refresh(self.system.as_ref(), &self.sender, Some(binding));
            }
            _ => {}
        }
    }
}
