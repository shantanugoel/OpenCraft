//! Request handling and helper threads. No handler has access to the device.

use crate::backend::{RealSystem, ShellChild, System};
use crate::descriptions::*;
use crate::flow::{NavigationHandler, SequenceHandler, ToggleHandler};
use crate::hotkey::Hotkey;
use crate::system::{MediaKey, Volume};
use opencraft_engine::{
    Action, BindingId, Feedback, Handler, KindDescription, Message, MessageSender, Request,
    Settings, Status, Value,
};
use std::collections::BTreeMap;
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub(crate) type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy)]
pub(crate) struct Intervals {
    pub volume: Duration,
    pub permission: Duration,
    pub permission_limit: Duration,
}

impl Default for Intervals {
    fn default() -> Self {
        Self {
            volume: Duration::from_secs(2),
            permission: Duration::from_secs(1),
            permission_limit: Duration::from_secs(10 * 60),
        }
    }
}

pub(crate) fn real_handlers() -> Vec<Box<dyn Handler>> {
    build(Arc::new(RealSystem), Intervals::default())
}

pub(crate) fn build(system: Arc<dyn System>, intervals: Intervals) -> Vec<Box<dyn Handler>> {
    vec![
        Box::new(OpenHandler {
            system: system.clone(),
            sender: None,
        }),
        Box::new(KeyHandler {
            system: system.clone(),
            sender: None,
            ready: Arc::new(Mutex::new(false)),
            poll: None,
            intervals,
        }),
        Box::new(VolumeHandler {
            system: system.clone(),
            sender: None,
            state: Arc::new(Mutex::new(VolumeState::default())),
            poll: None,
            intervals,
        }),
        Box::new(crate::device::DeviceHandler::default()),
        Box::new(crate::audio::MicrophoneHandler::new(system.clone())),
        Box::new(NavigationHandler::default()),
        Box::new(ToggleHandler::default()),
        Box::new(SequenceHandler::new(system)),
    ]
}

pub(crate) fn send(sender: &Option<MessageSender>, message: Message) {
    if let Some(sender) = sender {
        let _ = sender.send(message);
    }
}

pub(crate) fn problem(sender: &Option<MessageSender>, result: Result<()>) {
    if let Err(text) = result {
        send(sender, Message::Problem(text));
    }
}

/// The text in the setting `key` of `action`. Empty text is a problem.
fn text<'a>(action: &'a Action, key: &str) -> Result<&'a str> {
    match action.settings.get(key) {
        Some(Value::Text(value)) if !value.is_empty() => Ok(value),
        _ => Err(setting_problem(&action.kind, key)),
    }
}

/// As [`text`], but text with only spaces and line ends is also a problem.
pub(crate) fn nonblank<'a>(action: &'a Action, key: &str) -> Result<&'a str> {
    let value = text(action, key)?;
    if value.trim().is_empty() {
        Err(setting_problem(&action.kind, key))
    } else {
        Ok(value)
    }
}

/// A helper sleeps on a channel. Its owner stops and joins it on drop.
struct Worker {
    cancel: Sender<()>,
    thread: JoinHandle<()>,
}

impl Worker {
    fn spawn(name: &str, work: impl FnOnce(Receiver<()>) + Send + 'static) -> io::Result<Self> {
        let (cancel, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(name.into())
            .spawn(move || work(receiver))?;
        Ok(Self { cancel, thread })
    }

    fn stop(self) {
        let _ = self.cancel.send(());
        let _ = self.thread.join();
    }
}

struct OpenHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
}

impl OpenHandler {
    fn invoke(&self, action: Action) -> Result<()> {
        match action.kind.as_str() {
            APPLICATION => self
                .system
                .open_application(nonblank(&action, "application")?)
                .map_err(|e| format!("Cannot open the application: {e}.")),
            URL => self
                .system
                .open_url(nonblank(&action, "url")?)
                .map_err(|e| format!("Cannot open the URL: {e}.")),
            SHELL => {
                let command = nonblank(&action, "command")?.to_string();
                // Make the waiter first. If its thread cannot start, no child
                // exists that the system must remove.
                let (children, child) = mpsc::channel::<Box<dyn ShellChild>>();
                let sender = self.sender.clone();
                thread::Builder::new()
                    .name("opencraft-shell-command".into())
                    .spawn(move || {
                        let Ok(child) = child.recv() else { return };
                        let result = (|| match child
                            .wait()
                            .map_err(|e| format!("Cannot wait for the shell command: {e}."))?
                        {
                            Some(0) => Ok(()),
                            Some(code) => Err(format!("The shell command ends with code {code}.")),
                            None => Err("A signal stops the shell command (no exit code).".into()),
                        })();
                        problem(&sender, result);
                    })
                    .map_err(|e| format!("Cannot start the shell command thread: {e}."))?;
                let child = self
                    .system
                    .start_shell_command(&command)
                    .map_err(|e| format!("Cannot start the shell command: {e}."))?;
                children
                    .send(child)
                    .map_err(|_| "Cannot wait for the shell command.".into())
            }
            _ => Ok(()),
        }
    }
}

impl Handler for OpenHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        [APPLICATION, URL, SHELL].map(description).into()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        if let Request::Invoke { action, .. } = request {
            problem(&self.sender, self.invoke(action));
        }
    }
}

fn key_status(sender: &Option<MessageSender>, ready: bool) {
    for kind in KEY_KINDS {
        send(
            sender,
            Message::Status {
                kind: kind.into(),
                status: if ready {
                    Status::Ready
                } else {
                    Status::NeedsUser {
                        text: "OpenCraft needs the Accessibility permission to send keys and mouse events.".into(),
                        button: PERMISSION_BUTTON.into(),
                    }
                },
            },
        );
    }
}

enum KeyWork<'a> {
    Hotkey(Hotkey),
    Text(&'a str),
    Media(MediaKey),
    Mouse(crate::mouse::MouseEvent),
    MacFunction(crate::mac_function::MacFunction),
}

/// The problem of a key action when OpenCraft does not have the permission
/// to send input. The input kinds send the same text, so that the window
/// shows it one time.
pub(crate) fn no_permission() -> String {
    format!(
        "OpenCraft did not send the keys or mouse events, because it does not have the Accessibility \
         permission. Select the control, and click \"{PERMISSION_BUTTON}\" below the action."
    )
}

fn key_work(action: &Action) -> Result<KeyWork<'_>> {
    Ok(match action.kind.as_str() {
        HOTKEY => KeyWork::Hotkey(
            text(action, "hotkey")?
                .parse()
                .map_err(|_| setting_problem(HOTKEY, "hotkey"))?,
        ),
        MAC_FUNCTION => KeyWork::MacFunction(
            crate::mac_function::MacFunction::from_name(match action.settings.get("function") {
                None => "mission-control",
                Some(Value::Text(value)) => value,
                _ => return Err(setting_problem(MAC_FUNCTION, "function")),
            })
            .ok_or_else(|| setting_problem(MAC_FUNCTION, "function"))?,
        ),
        MOUSE => KeyWork::Mouse(crate::mouse::work(action)?),
        TEXT => KeyWork::Text(text(action, "text")?),
        MEDIA => KeyWork::Media(match text(action, "key")? {
            "play-pause" => MediaKey::PlayPause,
            "next" => MediaKey::Next,
            "previous" => MediaKey::Previous,
            "volume-up" => MediaKey::VolumeUp,
            "volume-down" => MediaKey::VolumeDown,
            "mute" => MediaKey::Mute,
            _ => return Err(setting_problem(MEDIA, "key")),
        }),
        _ => return Err("This handler does not have that action kind.".into()),
    })
}

struct KeyHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
    ready: Arc<Mutex<bool>>,
    poll: Option<Worker>,
    intervals: Intervals,
}

impl KeyHandler {
    fn check_permission(&self) -> bool {
        let mut previous = self.ready.lock().unwrap_or_else(|e| e.into_inner());
        let ready = self.system.can_send_keys();
        if !ready || !*previous {
            key_status(&self.sender, ready);
        }
        *previous = ready;
        ready
    }

    fn resolve(&mut self) -> Result<()> {
        self.system.ask_to_send_keys();
        if self.check_permission() {
            if let Some(worker) = self.poll.take() {
                worker.stop();
            }
            return Ok(());
        }
        if self.poll.as_ref().is_some_and(|w| w.thread.is_finished()) {
            if let Some(worker) = self.poll.take() {
                worker.stop();
            }
        }
        if self.poll.is_none() {
            let system = self.system.clone();
            let sender = self.sender.clone();
            let ready = self.ready.clone();
            let intervals = self.intervals;
            self.poll = Some(
                Worker::spawn("opencraft-key-permission", move |cancel| {
                    let deadline = Instant::now() + intervals.permission_limit;
                    loop {
                        let left = deadline.saturating_duration_since(Instant::now());
                        if left.is_zero()
                            || cancel.recv_timeout(intervals.permission.min(left))
                                != Err(mpsc::RecvTimeoutError::Timeout)
                        {
                            break;
                        }
                        let mut ready = ready.lock().unwrap_or_else(|e| e.into_inner());
                        if system.can_send_keys() {
                            if !*ready {
                                *ready = true;
                                key_status(&sender, true);
                            }
                            break;
                        }
                    }
                })
                .map_err(|e| format!("Cannot check the key permission: {e}."))?,
            );
        }
        Ok(())
    }

    fn invoke(&self, action: &Action) -> Result<()> {
        let permitted = self.check_permission();
        let work = key_work(action)?;
        if !permitted {
            // The user must see the cause at the time of the gesture. The
            // shell shows one message for the same text.
            return Err(no_permission());
        }
        let result = match work {
            KeyWork::Hotkey(hotkey) => self.system.send_hotkey(&hotkey),
            KeyWork::Text(text) => self.system.type_text(text),
            KeyWork::Media(key) => self.system.media_key(key),
            KeyWork::Mouse(event) => self.system.mouse_event(event),
            KeyWork::MacFunction(function) => {
                return self
                    .system
                    .mac_function(function)
                    .map_err(|e| format!("Cannot do the Mac system function: {e}."));
            }
        };
        result.map_err(|e| format!("Cannot send keys or mouse events: {e}."))
    }
}

impl Handler for KeyHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        KEY_KINDS.map(description).into()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
        let ready = self.system.can_send_keys();
        *self.ready.lock().unwrap_or_else(|e| e.into_inner()) = ready;
        key_status(&self.sender, ready);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Invoke { action, .. } if KEY_KINDS.contains(&action.kind.as_str()) => {
                problem(&self.sender, self.invoke(&action));
            }
            Request::Resolve { kind } if KEY_KINDS.contains(&kind.as_str()) => {
                let result = self.resolve();
                problem(&self.sender, result);
            }
            _ => {}
        }
    }
}

impl Drop for KeyHandler {
    fn drop(&mut self) {
        if let Some(worker) = self.poll.take() {
            worker.stop();
        }
    }
}

#[derive(Default)]
struct VolumeState {
    bindings: BTreeMap<BindingId, Option<Volume>>,
    problem: Option<String>,
}

fn volume_feedback(sender: &Option<MessageSender>, binding: BindingId, volume: Volume) {
    send(
        sender,
        Message::Feedback {
            binding,
            feedback: Some(Feedback {
                label: volume.muted.then(|| "Muted".into()),
                level: Some(if volume.muted {
                    0
                } else {
                    volume.level.min(100)
                }),
                ..Feedback::default()
            }),
        },
    );
}

impl VolumeState {
    fn refresh(
        &mut self,
        system: &dyn System,
        sender: &Option<MessageSender>,
        force: Option<BindingId>,
    ) {
        match system.volume() {
            Ok(volume) => {
                self.problem = None;
                for (binding, previous) in &mut self.bindings {
                    if *previous != Some(volume) || force == Some(*binding) {
                        volume_feedback(sender, *binding, volume);
                        *previous = Some(volume);
                    }
                }
                if let Some(binding) = force.filter(|id| !self.bindings.contains_key(id)) {
                    volume_feedback(sender, binding, volume);
                }
            }
            Err(e) => {
                let text = format!("Cannot read the sound volume: {e}.");
                if self.problem.as_ref() != Some(&text) {
                    send(sender, Message::Problem(text.clone()));
                    self.problem = Some(text);
                }
            }
        }
    }
}

fn volume_settings(settings: &Settings) -> Result<(&str, u8)> {
    let operation = match settings.get("operation") {
        None => "up",
        Some(Value::Text(value)) if ["up", "down", "mute"].contains(&value.as_str()) => value,
        _ => return Err(setting_problem(VOLUME, "operation")),
    };
    let step = match settings.get("step") {
        None => 5,
        Some(Value::Integer(step)) if (1..=25).contains(step) => *step as u8,
        _ => return Err(setting_problem(VOLUME, "step")),
    };
    Ok((operation, step))
}

struct VolumeHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
    state: Arc<Mutex<VolumeState>>,
    poll: Option<Worker>,
    intervals: Intervals,
}

impl VolumeHandler {
    fn invoke(&self, settings: &Settings) -> Result<()> {
        let (operation, step) = volume_settings(settings)?;
        let volume = self
            .system
            .volume()
            .map_err(|e| format!("Cannot read the sound volume: {e}."))?;
        let result = if operation == "mute" {
            self.system.set_muted(!volume.muted)
        } else {
            let level = if operation == "up" {
                volume.level.saturating_add(step).min(100)
            } else {
                volume.level.saturating_sub(step)
            };
            self.system.set_volume_level(level).and_then(|()| {
                if volume.muted {
                    self.system.set_muted(false)
                } else {
                    Ok(())
                }
            })
        };
        result.map_err(|e| format!("Cannot set the sound volume: {e}."))
    }

    fn start_poll(&mut self) {
        if self.poll.is_some() {
            return;
        }
        let system = self.system.clone();
        let state = self.state.clone();
        let sender = self.sender.clone();
        let interval = self.intervals.volume;
        match Worker::spawn("opencraft-volume", move |cancel| {
            while cancel.recv_timeout(interval) == Err(mpsc::RecvTimeoutError::Timeout) {
                // Keep the lock through the read and send. Disappear cannot
                // return while a helper still sends feedback for that binding.
                let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                if !state.bindings.is_empty() {
                    state.refresh(system.as_ref(), &sender, None);
                }
            }
        }) {
            Ok(worker) => self.poll = Some(worker),
            Err(e) => send(
                &self.sender,
                Message::Problem(format!("Cannot check the sound volume: {e}.")),
            ),
        }
    }
}

impl Handler for VolumeHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![description(VOLUME)]
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Appear { binding, action } if action.kind == VOLUME => {
                {
                    let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                    state.bindings.insert(binding, None);
                    state.refresh(self.system.as_ref(), &self.sender, Some(binding));
                }
                self.start_poll();
            }
            Request::Disappear { binding } => {
                let empty = {
                    let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                    state.bindings.remove(&binding);
                    state.bindings.is_empty()
                };
                if empty {
                    if let Some(worker) = self.poll.take() {
                        worker.stop();
                    }
                }
            }
            Request::Invoke { binding, action } if action.kind == VOLUME => {
                let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                if let Err(text) = self.invoke(&action.settings) {
                    state.problem = Some(text.clone());
                    send(&self.sender, Message::Problem(text));
                }
                state.refresh(self.system.as_ref(), &self.sender, Some(binding));
            }
            _ => {}
        }
    }
}

impl Drop for VolumeHandler {
    fn drop(&mut self) {
        if let Some(worker) = self.poll.take() {
            worker.stop();
        }
    }
}
