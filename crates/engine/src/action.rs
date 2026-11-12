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
///
/// A tap is on a bar panel control. A swipe is on the touch bar control. The
/// reasons are in `docs/decisions/0015-four-bar-panels-and-touch-gestures.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GestureKind {
    Press,
    Release,
    TurnLeft,
    TurnRight,
    Push,
    Tap,
    SwipeLeft,
    SwipeRight,
}

impl GestureKind {
    /// The gesture of a binding for an input gesture. The start of a touch,
    /// a touch point, and a touch end give `None`: a binding cannot have
    /// them, because the device did not identify a gesture.
    pub fn from_gesture(gesture: Gesture) -> Option<Self> {
        Some(match gesture {
            Gesture::Press => Self::Press,
            Gesture::Release => Self::Release,
            Gesture::TurnLeft => Self::TurnLeft,
            Gesture::TurnRight => Self::TurnRight,
            Gesture::Push => Self::Push,
            Gesture::Tap { .. } => Self::Tap,
            Gesture::SwipeLeft => Self::SwipeLeft,
            Gesture::SwipeRight => Self::SwipeRight,
            Gesture::TouchStart { .. } | Gesture::TouchPoint { .. } | Gesture::TouchEnd => {
                return None
            }
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
    /// Ask the engine for work. The engine obeys it as it obeys the same
    /// command from the shell.
    Engine(EngineRequest),
}

/// Work that a handler asks the engine to do. A request names no binding: it
/// applies to the active profile. The reasons are in
/// `docs/decisions/0016-pages-and-engine-requests-from-a-handler.md` and
/// `docs/decisions/0018-many-profiles.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineRequest {
    /// Make a different page the active page.
    SelectPage(PageTarget),
    /// Make the profile with the name `name` for the user the active
    /// profile, as [`crate::Command::SelectProfile`] does. If two profiles of
    /// the model have the name, the first in the order of the identities
    /// wins. If no profile has it, the engine tells the shell.
    SelectProfile { name: String },
}

/// The page that [`EngineRequest::SelectPage`] selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTarget {
    /// The page after the active page. After the last page, the first page.
    Next,
    /// The page before the active page. Before the first page, the last page.
    Previous,
    /// The page with this number. Page numbers start at 0.
    Number(usize),
}

impl PageTarget {
    /// The number of the target page, from the number of the active page and
    /// the number of pages. `Number` gives its number, also when it is not a
    /// page.
    pub(crate) fn resolve(self, active: usize, count: usize) -> usize {
        match self {
            PageTarget::Next => (active + 1) % count,
            PageTarget::Previous => (active + count - 1) % count,
            PageTarget::Number(page) => page,
        }
    }
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
