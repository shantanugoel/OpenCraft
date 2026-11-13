//! The kinds that ask the engine for work: change the page, select a
//! profile, toggle, and sequence.
//!
//! These handlers do no work on the computer. They send requests to the
//! engine with [`Message::Engine`], through the same handler interface as
//! each other handler (architecture rule 8). The reasons are in
//! `docs/decisions/0016-pages-and-engine-requests-from-a-handler.md`,
//! `docs/decisions/0017-toggle-and-sequence-actions.md`, and
//! `docs/decisions/0018-many-profiles.md`.

use crate::backend::System;
use crate::descriptions::*;
use crate::handlers::{nonblank, problem, send, Result};
use opencraft_engine::{
    Action, BindingId, EngineRequest, Feedback, FeedbackState, Field, Handler, KindDescription,
    Message, MessageSender, PageTarget, Request, Settings, Value,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// --- Change the page, and select a profile ----------------------------------------

/// The kinds `opencraft.page` and `opencraft.profile`.
#[derive(Default)]
pub(crate) struct NavigationHandler {
    sender: Option<MessageSender>,
}

/// The page that the settings of `opencraft.page` select. The field
/// `target` is `next`, `previous`, or `number`. With `number`, the field
/// `page` is the page number as the user sees it: the first page is 1. The
/// request has the number from 0. A missing field has its default value.
fn page_target(settings: &Settings) -> Result<PageTarget> {
    let target = match settings.get("target") {
        None => "next",
        Some(Value::Text(target)) => target.as_str(),
        Some(_) => "",
    };
    match target {
        "next" => Ok(PageTarget::Next),
        "previous" => Ok(PageTarget::Previous),
        "number" => match settings.get("page") {
            None => Ok(PageTarget::Number(0)),
            Some(Value::Integer(page)) if (1..=PAGE_LIMIT).contains(page) => {
                Ok(PageTarget::Number(*page as usize - 1))
            }
            Some(_) => Err(setting_problem(PAGE, "page")),
        },
        _ => Err(setting_problem(PAGE, "target")),
    }
}

impl NavigationHandler {
    fn request(&self, action: &Action) -> Result<EngineRequest> {
        match action.kind.as_str() {
            PAGE => page_target(&action.settings).map(EngineRequest::SelectPage),
            PROFILE => Ok(EngineRequest::SelectProfile {
                name: nonblank(action, "profile")?.trim().to_string(),
            }),
            _ => Err("This handler does not have that action kind.".into()),
        }
    }
}

impl Handler for NavigationHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        [PAGE, PROFILE].map(description).into()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        if let Request::Invoke { action, .. } = request {
            let result = self
                .request(&action)
                .map(|request| send(&self.sender, Message::Engine(request)));
            problem(&self.sender, result);
        }
    }
}

// --- Toggle -----------------------------------------------------------------------

/// The settings of `opencraft.toggle`. Each part is optional. A toggle with
/// no action only changes its state.
struct Toggle {
    on: Option<Action>,
    off: Option<Action>,
    on_label: Option<String>,
}

/// Read the settings of a toggle. A value of a different form is an error.
fn toggle_settings(settings: &Settings) -> Result<Toggle> {
    let action = |key: &str| match settings.get(key) {
        None => Ok(None),
        Some(value) => Action::from_value(value)
            .map(Some)
            .ok_or_else(|| setting_problem(TOGGLE, key)),
    };
    let on_label = match settings.get("on_label") {
        None => None,
        Some(Value::Text(label)) => Some(label.clone()).filter(|l| !l.trim().is_empty()),
        Some(_) => return Err(setting_problem(TOGGLE, "on_label")),
    };
    Ok(Toggle {
        on: action("on")?,
        off: action("off")?,
        on_label,
    })
}

/// The kind `opencraft.toggle`.
///
/// The state of a toggle is in memory, for each binding and its settings.
/// A toggle starts in the state "off". The state stays after Disappear, so a
/// page change and a change back show the state from before. An Appear with
/// different settings forgets the old state, so a change of the settings
/// gives the state "off". The state of a toggle that the binding contains,
/// for example in a sequence, stays too.
#[derive(Default)]
pub(crate) struct ToggleHandler {
    sender: Option<MessageSender>,
    states: BTreeMap<BindingId, Vec<(Settings, bool)>>,
}

impl ToggleHandler {
    fn state(&self, binding: BindingId, settings: &Settings) -> bool {
        self.states
            .get(&binding)
            .and_then(|list| list.iter().find(|(s, _)| s == settings))
            .is_some_and(|(_, on)| *on)
    }

    fn set_state(&mut self, binding: BindingId, settings: &Settings, on: bool) {
        let list = self.states.entry(binding).or_default();
        match list.iter_mut().find(|(s, _)| s == settings) {
            Some(entry) => entry.1 = on,
            None => list.push((settings.clone(), on)),
        }
    }

    /// Send the state, and the label for "on" when the state is on. The
    /// feedback of the other state lets the engine make the image of both
    /// states for the shell.
    fn feedback(&self, binding: BindingId, on: bool, on_label: Option<String>) {
        let state = |on: bool| Feedback {
            label: on_label.clone().filter(|_| on),
            state: Some(if on {
                FeedbackState::On
            } else {
                FeedbackState::Off
            }),
            ..Feedback::default()
        };
        let feedback = Feedback {
            other_state: Some(Box::new(state(!on))),
            ..state(on)
        };
        send(
            &self.sender,
            Message::Feedback {
                binding,
                feedback: Some(feedback),
            },
        );
    }

    fn appear(&mut self, binding: BindingId, settings: &Settings) {
        if let Some(list) = self.states.get_mut(&binding) {
            list.retain(|(s, _)| s == settings || contains(settings, s));
        }
        // The settings are checked at Invoke. Here a bad label is no label.
        let on_label = toggle_settings(settings).ok().and_then(|t| t.on_label);
        self.feedback(binding, self.state(binding, settings), on_label);
    }

    fn invoke(&mut self, binding: BindingId, settings: &Settings) -> Result<()> {
        let toggle = toggle_settings(settings)?;
        // The order of decision 0017: the state, the request, the feedback.
        let on = !self.state(binding, settings);
        self.set_state(binding, settings, on);
        let action = if on { toggle.on } else { toggle.off };
        if let Some(action) = action {
            send(
                &self.sender,
                Message::Engine(EngineRequest::Invoke { binding, action }),
            );
        }
        self.feedback(binding, on, toggle.on_label);
        Ok(())
    }
}

/// True if a value in `settings`, at any depth, is the table `inner`: for
/// example the settings of an action that the settings contain.
fn contains(settings: &Settings, inner: &Settings) -> bool {
    fn value_contains(value: &Value, inner: &Settings) -> bool {
        match value {
            Value::Table(table) => table == inner || contains(table, inner),
            Value::List(list) => list.iter().any(|v| value_contains(v, inner)),
            _ => false,
        }
    }
    settings.values().any(|v| value_contains(v, inner))
}

impl Handler for ToggleHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![description(TOGGLE)]
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Appear { binding, action } if action.kind == TOGGLE => {
                self.appear(binding, &action.settings);
            }
            Request::Invoke { binding, action } if action.kind == TOGGLE => {
                let result = self.invoke(binding, &action.settings);
                problem(&self.sender, result);
            }
            _ => {}
        }
    }
}

// --- Sequence ---------------------------------------------------------------------

/// Read the settings of a sequence: each action in its order, with the
/// delay before it (decision 0017 and its amendment for #282). A missing
/// list is no action. A step with no `delay_before_ms` waits for 0 if it is
/// the first step, else for `delay_ms` (default [`DEFAULT_DELAY`]). Thus an
/// old profile keeps its timing. The step delay is not a setting of the
/// step's kind, so it is removed before the engine gets the action.
fn sequence_settings(settings: &Settings) -> Result<Vec<(Action, Duration)>> {
    let actions = match settings.get("actions") {
        None => Vec::new(),
        Some(value) => {
            Action::list_from_value(value).ok_or_else(|| setting_problem(SEQUENCE, "actions"))?
        }
    };
    let delay = match settings.get("delay_ms") {
        None => DEFAULT_DELAY,
        Some(Value::Integer(ms)) if (0..=DELAY_LIMIT).contains(ms) => *ms,
        Some(_) => return Err(setting_problem(SEQUENCE, "delay_ms")),
    };
    let step = Field::step_delay(0);
    actions
        .into_iter()
        .enumerate()
        .map(|(index, mut action)| {
            let ms = match action.settings.remove(&step.key) {
                None if index == 0 => 0,
                None => delay,
                Some(Value::Integer(ms)) if (0..=DELAY_LIMIT).contains(&ms) => ms,
                Some(_) => {
                    return Err(format!(
                        "Sequence of actions: set the field \"{}\" of step {} to a number from 0 to {DELAY_LIMIT}.",
                        step.label,
                        index + 1
                    ))
                }
            };
            Ok((action, Duration::from_millis(ms as u64)))
        })
        .collect()
}

/// The kind `opencraft.sequence`.
///
/// At each Invoke, a thread of its own asks the engine to do each action in
/// the order of the list, and waits for the delay before each action. The
/// handler does not wait for the thread, so its next request does not wait
/// (decision 0012). Each Invoke starts a new thread: two sequences can run
/// at the same time. The thread ends after the last action, also when the
/// handler stops.
pub(crate) struct SequenceHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
}

impl SequenceHandler {
    pub(crate) fn new(system: Arc<dyn System>) -> Self {
        Self {
            system,
            sender: None,
        }
    }

    fn invoke(&self, binding: BindingId, settings: &Settings) -> Result<()> {
        let actions = sequence_settings(settings)?;
        if actions.is_empty() {
            return Ok(());
        }
        let system = self.system.clone();
        let sender = self.sender.clone();
        thread::Builder::new()
            .name("opencraft-sequence".into())
            .spawn(move || {
                for (action, delay) in actions {
                    if !delay.is_zero() {
                        system.sleep(delay);
                    }
                    send(
                        &sender,
                        Message::Engine(EngineRequest::Invoke { binding, action }),
                    );
                }
            })
            .map(|_| ())
            .map_err(|e| format!("Cannot start the sequence: {e}."))
    }
}

impl Handler for SequenceHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![description(SEQUENCE)]
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        if let Request::Invoke { binding, action } = request {
            if action.kind == SEQUENCE {
                problem(&self.sender, self.invoke(binding, &action.settings));
            }
        }
    }
}
