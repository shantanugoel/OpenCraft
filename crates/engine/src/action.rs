//! Data and ports for action handlers. A handler has no access to the device.

use opencraft_core::{ControlId, Gesture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

/// The settings of an action. The engine does not read these values.
pub type Settings = BTreeMap<String, Value>;

/// A value that TOML keeps without loss.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Text(String),
    Integer(i64),
    Boolean(bool),
    List(Vec<Value>),
    Table(Settings),
}

/// A unit of work, as data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub kind: String,
    #[serde(default)]
    pub settings: Settings,
}

/// A gesture with no position data. The text form is stable in the profile.
/// Later phases can add tap and swipe variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GestureKind {
    Press,
    Release,
    TurnLeft,
    TurnRight,
    Push,
}

impl GestureKind {
    pub fn from_gesture(gesture: Gesture) -> Option<Self> {
        Some(match gesture {
            Gesture::Press => Self::Press,
            Gesture::Release => Self::Release,
            Gesture::TurnLeft => Self::TurnLeft,
            Gesture::TurnRight => Self::TurnRight,
            Gesture::Push => Self::Push,
            _ => return None,
        })
    }
}

/// The identity of a binding. Page numbers start at 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingId {
    pub page: usize,
    pub control: ControlId,
    pub gesture: GestureKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Appear { binding: BindingId, action: Action },
    Disappear { binding: BindingId },
    Invoke { binding: BindingId, action: Action },
    Resolve { kind: String },
}

/// The label replaces the content label. The level is from 0 to 100.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Feedback {
    pub label: Option<String>,
    pub level: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// `None` removes the feedback of the binding.
    Feedback {
        binding: BindingId,
        feedback: Option<Feedback>,
    },
    Status {
        kind: String,
        status: Status,
    },
    Problem(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindDescription {
    pub kind: String,
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub input: InputType,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathKind {
    File,
    Application,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputType {
    Text,
    LongText,
    Integer {
        min: i64,
        max: i64,
    },
    Choice {
        options: Vec<ChoiceOption>,
    },
    Toggle,
    /// Modifiers ctrl, alt, shift, cmd, then one key; small letters and `+`.
    Hotkey,
    Path {
        kind: PathKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Ready,
    NeedsUser { text: String, button: String },
}

pub type MessageSender = Sender<Message>;

/// The runtime calls each handler on its own thread, never on the engine thread.
pub trait Handler: Send + 'static {
    /// The descriptions of all kinds of this handler. The runtime reads them
    /// before it starts the engine thread.
    fn kinds(&self) -> Vec<KindDescription>;
    /// The runtime calls this one time, before the first request. The handler
    /// can clone the sender and send a message at any time.
    fn start(&mut self, sender: MessageSender);
    /// One request, in queue order. The handler checks the settings. It starts
    /// long work on a separate thread, so its next request does not wait.
    fn handle(&mut self, request: Request);
}

/// A non-blocking request queue and the descriptions of its handler.
/// Tests read the queue directly, with no handler thread.
#[derive(Debug, Clone)]
pub struct HandlerPort {
    pub kinds: Vec<KindDescription>,
    pub sender: Sender<Request>,
}

/// The state of a binding for a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingState {
    pub binding: BindingId,
    pub action: Option<Action>,
    pub available: bool,
}
