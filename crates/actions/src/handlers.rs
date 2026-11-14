//! Request handling and helper threads. No handler has access to the device.

use crate::backend::{RealSystem, System};
use crate::descriptions::*;
use crate::flow::{NavigationHandler, SequenceHandler, ToggleHandler};
use crate::system::Volume;
use opencraft_engine::{
    Action, BindingId, Feedback, Handler, KindDescription, Message, MessageSender, Request, Status,
    Value,
};
use std::collections::BTreeMap;
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(crate) type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy)]
pub(crate) struct Intervals {
    pub volume: Duration,
    pub permission: Duration,
}

impl Default for Intervals {
    fn default() -> Self {
        Self {
            volume: Duration::from_secs(2),
            // One shared check for all key kinds, also with no window or bindings.
            permission: Duration::from_secs(5),
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
        Box::new(crate::audio_output::OutputHandler::new(system.clone())),
        Box::new(crate::sound::SoundHandler::new(system.clone())),
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

/// The handler for [`crate::kinds::Group::Open`].
/// It does not read the settings. Each kind module reads its settings and
/// does its work. These kinds share this handler, its thread, and its queue.
struct OpenHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
}

impl Handler for OpenHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        crate::kinds::kinds_of(crate::kinds::Group::Open)
            .map(|kind| kind.description())
            .collect()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        let Request::Invoke { action, .. } = request else {
            return;
        };
        let Some(kind) = crate::kinds::find_in(crate::kinds::Group::Open, &action.kind) else {
            return;
        };
        let work = crate::kinds::Work::new(self.system.as_ref(), &self.sender);
        problem(&self.sender, kind.invoke(&action.settings, &work));
    }
}

fn key_status(sender: &Option<MessageSender>, ready: bool) {
    for kind in crate::kinds::kinds_of(crate::kinds::Group::Key) {
        send(
            sender,
            Message::Status {
                kind: kind.id().into(),
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

/// Read the permission and store it. Send the status when the permission is
/// missing, or when the previous read was missing, so that an Invoke repeats
/// the status.
fn permission_now(
    system: &dyn System,
    ready: &Mutex<bool>,
    sender: &Option<MessageSender>,
) -> bool {
    let mut previous = ready.lock().unwrap_or_else(|e| e.into_inner());
    let current = system.can_send_keys();
    if !current || !*previous {
        key_status(sender, current);
    }
    *previous = current;
    current
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

struct KeyHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
    ready: Arc<Mutex<bool>>,
    poll: Option<Worker>,
    intervals: Intervals,
}

impl KeyHandler {
    fn check_permission(&self) -> bool {
        permission_now(self.system.as_ref(), &self.ready, &self.sender)
    }

    fn watch_permission(&mut self) -> Result<()> {
        let system = self.system.clone();
        let sender = self.sender.clone();
        let ready = self.ready.clone();
        let interval = self.intervals.permission;
        self.poll = Some(
            Worker::spawn("opencraft-key-permission", move |cancel| {
                // TCC has no public change notification for this permission.
                // Sleep between checks; send no events for an unchanged state.
                // Keep checking after a grant, so revocation is visible too.
                while cancel.recv_timeout(interval) == Err(mpsc::RecvTimeoutError::Timeout) {
                    let mut previous = ready.lock().unwrap_or_else(|e| e.into_inner());
                    let current = system.can_send_keys();
                    if current != *previous {
                        *previous = current;
                        key_status(&sender, current);
                    }
                }
            })
            .map_err(|e| format!("Cannot check the key permission: {e}."))?,
        );
        Ok(())
    }

    /// Run one key kind. The kind reads the settings before it uses the
    /// permission, so a bad setting is not a permission problem. The kind
    /// decides whether these settings need the permission.
    fn invoke(&self, action: &Action) -> Result<()> {
        let Some(kind) = crate::kinds::find_in(crate::kinds::Group::Key, &action.kind) else {
            return Ok(());
        };
        let permitted = self.check_permission();
        let system = Arc::clone(&self.system);
        let ready = Arc::clone(&self.ready);
        let sender = self.sender.clone();
        let recheck = move || permission_now(system.as_ref(), &ready, &sender);
        let work = crate::kinds::Work::new(self.system.as_ref(), &self.sender)
            .with_keys(permitted, &recheck);
        kind.invoke(&action.settings, &work)
    }
}

impl Handler for KeyHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        crate::kinds::kinds_of(crate::kinds::Group::Key)
            .map(|kind| kind.description())
            .collect()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
        let ready = self.system.can_send_keys();
        *self.ready.lock().unwrap_or_else(|e| e.into_inner()) = ready;
        key_status(&self.sender, ready);
        let result = self.watch_permission();
        problem(&self.sender, result);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Invoke { action, .. } => {
                problem(&self.sender, self.invoke(&action));
            }
            Request::Resolve { kind }
                if crate::kinds::find_in(crate::kinds::Group::Key, &kind).is_some() =>
            {
                self.system.ask_to_send_keys();
                self.check_permission();
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

/// The handler for [`crate::kinds::Group::Volume`].
/// The kind module reads the settings and changes the volume. This handler
/// keeps the bindings, the feedback, and the poll.
struct VolumeHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
    state: Arc<Mutex<VolumeState>>,
    poll: Option<Worker>,
    intervals: Intervals,
}

impl VolumeHandler {
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
        crate::kinds::kinds_of(crate::kinds::Group::Volume)
            .map(|kind| kind.description())
            .collect()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Appear { binding, action }
                if crate::kinds::find_in(crate::kinds::Group::Volume, &action.kind).is_some() =>
            {
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
            Request::Invoke { binding, action } => {
                let Some(kind) = crate::kinds::find_in(crate::kinds::Group::Volume, &action.kind)
                else {
                    return;
                };
                let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                let work = crate::kinds::Work::new(self.system.as_ref(), &self.sender);
                if let Err(text) = kind.invoke(&action.settings, &work) {
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
