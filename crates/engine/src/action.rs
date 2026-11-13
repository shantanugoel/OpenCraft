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

/// The key of the kind in the settings form of an action.
const KIND_KEY: &str = "kind";
/// The key of the settings in the settings form of an action.
const SETTINGS_KEY: &str = "settings";

impl Action {
    /// Read an action from a value in the settings of a different action:
    /// a table with the text `kind` and an optional table `settings`. This
    /// is the form of the input types [`InputType::Action`] and
    /// [`InputType::ActionList`] (decision 0017). A handler and a shell use
    /// this function and [`Action::to_value`], so that they use one form.
    ///
    /// `None` if the value does not have this form: for example a different
    /// type, an empty kind, or a key that is not `kind` or `settings`.
    pub fn from_value(value: &Value) -> Option<Action> {
        let Value::Table(table) = value else {
            return None;
        };
        if table.keys().any(|k| k != KIND_KEY && k != SETTINGS_KEY) {
            return None;
        }
        let kind = match table.get(KIND_KEY)? {
            Value::Text(kind) if !kind.trim().is_empty() => kind.clone(),
            _ => return None,
        };
        let settings = match table.get(SETTINGS_KEY) {
            None => Settings::new(),
            Some(Value::Table(settings)) => settings.clone(),
            Some(_) => return None,
        };
        Some(Action { kind, settings })
    }

    /// The value of this action in the settings of a different action. See
    /// [`Action::from_value`].
    pub fn to_value(&self) -> Value {
        Value::Table(Settings::from([
            (KIND_KEY.to_string(), Value::Text(self.kind.clone())),
            (
                SETTINGS_KEY.to_string(),
                Value::Table(self.settings.clone()),
            ),
        ]))
    }

    /// Read a list of actions, in its order, from a value: a list of tables
    /// of the form of [`Action::from_value`]. `None` if the value is not a
    /// list, or if one item is not an action.
    pub fn list_from_value(value: &Value) -> Option<Vec<Action>> {
        let Value::List(items) = value else {
            return None;
        };
        items.iter().map(Action::from_value).collect()
    }

    /// The value of a list of actions. See [`Action::list_from_value`].
    pub fn list_to_value(actions: &[Action]) -> Value {
        Value::List(actions.iter().map(Action::to_value).collect())
    }
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

/// What a surface shows for a binding, as data. Each part is optional. The
/// engine makes the image: see [`crate::render::surface_image_with_feedback`]
/// and decisions 0013 and 0017.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Feedback {
    /// A text that replaces the label of the surface.
    pub label: Option<String>,
    /// A value from 0 to 100. The engine draws it as a bar.
    pub level: Option<u8>,
    /// On or off. For on, the engine draws a frame around the surface, in
    /// the colour of the label. For off, it draws no frame.
    pub state: Option<FeedbackState>,
    /// The feedback of the other state, for a feedback with a state. The
    /// engine makes the image of each state from the two feedbacks and sends
    /// both images to the shell (decision 0017). Without it, the image of
    /// the other state has the same parts with the other state.
    pub other_state: Option<Box<Feedback>>,
}

/// The state part of [`Feedback`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackState {
    Off,
    On,
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

/// Work that a handler asks the engine to do. A request for a page or a
/// profile names no binding: it applies to the active profile. The reasons
/// are in `docs/decisions/0016-pages-and-engine-requests-from-a-handler.md`
/// and `docs/decisions/0018-many-profiles.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineRequest {
    /// Make a different page the active page.
    SelectPage(PageTarget),
    /// Make the profile with the name `name` for the user the active
    /// profile, as [`crate::Command::SelectProfile`] does. If two profiles of
    /// the model have the name, the first in the order of the identities
    /// wins. If no profile has it, the engine tells the shell.
    SelectProfile { name: String },
    /// Do the action `action`: the engine sends [`Request::Invoke`] with
    /// `binding` and `action` to the handler of the kind of `action`. The
    /// binding is the binding of the handler that asks. Thus the feedback of
    /// the action shows on the surface of that binding. If no handler has
    /// the kind, the engine tells the shell. The handler that asks gets no
    /// result. The reasons are in
    /// `docs/decisions/0017-toggle-and-sequence-actions.md`.
    Invoke { binding: BindingId, action: Action },
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

/// The description of one kind: see
/// `docs/decisions/0014-action-settings-description.md`. The shell makes the
/// library, the kind menu, and the settings form from it, and has no list of
/// kinds (rule 8).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KindDescription {
    pub kind: String,
    pub name: String,
    /// The group of the kind in the library and in the kind menu, for
    /// example "System". The shell shows the groups in the order of their
    /// first kind.
    pub group: String,
    /// A short help text for the library row, for example "Accessibility".
    pub help: Option<String>,
    /// The kind operates only with the Accessibility permission of macOS.
    /// The shell names these kinds where it tells the user about the
    /// permission.
    pub needs_accessibility: bool,
    pub fields: Vec<Field>,
    /// The direction of a turn of a knob, if the kind has one. The shell
    /// then shows one "Turn" card for the two turn gestures, and writes the
    /// two values to the two bindings (decision 0029, section 10.4).
    pub direction: Option<Direction>,
    /// The kind has two states, Off and On, and shows the state on the
    /// surface (decision 0017). The shell then shows the appearance of each
    /// state.
    pub states: Option<TwoStates>,
}

/// The direction field of a kind: a choice field, and its values for a turn
/// to the right and for a turn to the left. For "Sound volume": the field
/// `operation`, right `up`, left `down`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Direction {
    /// The key of a field of the input type Choice.
    pub key: String,
    /// The value of that field for the binding "Turn right".
    pub right: String,
    /// The value of that field for the binding "Turn left".
    pub left: String,
}

/// The two states of a kind: see [`KindDescription::states`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TwoStates {
    /// The key of the text field with the label of the state On, if the kind
    /// has one. The shell shows this field with the face of the state On.
    pub on_label: Option<String>,
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
    /// One action: a kind and its settings. The value is a table with
    /// `kind` and `settings`: see [`Action::from_value`]. The shell makes the
    /// form of the contained action from the fields of its kind.
    Action,
    /// A list of actions, in an order. The value is a list of such tables:
    /// see [`Action::list_from_value`].
    ActionList,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    #[test]
    fn an_action_goes_to_a_settings_value_and_back() {
        let action = Action {
            kind: "test.kind".into(),
            settings: Settings::from([("key".into(), text("value"))]),
        };
        let value = action.to_value();
        assert_eq!(
            value,
            Value::Table(Settings::from([
                ("kind".into(), text("test.kind")),
                (
                    "settings".into(),
                    Value::Table(Settings::from([("key".into(), text("value"))]))
                ),
            ]))
        );
        assert_eq!(Action::from_value(&value), Some(action.clone()));

        let list = vec![action.clone(), Action::from_value(&value).unwrap()];
        assert_eq!(
            Action::list_from_value(&Action::list_to_value(&list)),
            Some(list)
        );
        assert_eq!(
            Action::list_from_value(&Value::List(vec![])),
            Some(Vec::new())
        );
    }

    #[test]
    fn an_action_with_no_settings_has_empty_settings() {
        let value = Value::Table(Settings::from([("kind".into(), text("test.kind"))]));
        let action = Action::from_value(&value).unwrap();
        assert_eq!(action.kind, "test.kind");
        assert!(action.settings.is_empty());
    }

    #[test]
    fn a_value_of_a_different_form_is_not_an_action() {
        let table = |entries: Vec<(&str, Value)>| {
            Value::Table(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
        };
        for bad in [
            text("test.kind"),
            Value::Integer(1),
            Value::List(vec![]),
            table(vec![]),
            table(vec![("kind", text(""))]),
            table(vec![("kind", text("  "))]),
            table(vec![("kind", Value::Integer(3))]),
            table(vec![("kind", text("test.kind")), ("settings", text("x"))]),
            table(vec![("kind", text("test.kind")), ("extra", text("x"))]),
        ] {
            assert_eq!(Action::from_value(&bad), None, "{bad:?}");
        }
        let good = table(vec![("kind", text("test.kind"))]);
        for bad in [good.clone(), Value::List(vec![good, Value::Integer(1)])] {
            assert_eq!(Action::list_from_value(&bad), None, "{bad:?}");
        }
    }
}
