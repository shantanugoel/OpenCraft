//! Data and ports for action handlers. A handler has no access to the device.

use opencraft_core::{ControlId, Gesture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

/// The settings of an action. The engine does not read these values to run
/// an action. The export is the exception: it replaces the home directory
/// in a path setting, and it does not interpret the action. See decision
/// 0037.
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
    /// is the form of the input types [`InputType::Action`],
    /// [`InputType::ActionList`] and [`InputType::TimedActionList`] (decision
    /// 0017). A handler and a shell use
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
    LongPress,
    DoublePress,
}

impl GestureKind {
    /// The gestures that a binding of a control of this kind can have. A key
    /// has press, release, long press, and double press (decision 0042), a
    /// knob has two turns and a push, a bar panel has a tap, and the touch bar
    /// has two swipes (decision 0015). A binding with another gesture can never
    /// run.
    pub fn of_control(kind: opencraft_core::ControlKind) -> &'static [GestureKind] {
        use opencraft_core::ControlKind;
        match kind {
            ControlKind::Key => &[
                Self::Press,
                Self::Release,
                Self::LongPress,
                Self::DoublePress,
            ],
            ControlKind::Knob => &[Self::TurnLeft, Self::TurnRight, Self::Push],
            ControlKind::BarPanel => &[Self::Tap],
            ControlKind::TouchBar => &[Self::SwipeLeft, Self::SwipeRight],
        }
    }

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
///
/// `page` is always the active page, also while a folder is open: it is the
/// page under the folders. `folder` is the last open folder for a key
/// binding, and `None` for every other binding and when no folder is open
/// (decision 0036).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingId {
    pub page: usize,
    pub control: ControlId,
    pub gesture: GestureKind,
    /// The folder that holds this key binding. `None` when the binding is
    /// on the page under the folders.
    pub folder: Option<crate::profile::FolderId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Appear {
        binding: BindingId,
        action: Action,
    },
    Disappear {
        binding: BindingId,
    },
    Invoke {
        binding: BindingId,
        action: Action,
    },
    /// The key's Press binding handles a long press and has no explicit
    /// LongPress binding (decision 0042). `binding` identifies the Press binding.
    LongPress {
        binding: BindingId,
        action: Action,
    },
    /// A Press binding of a kind that receives release went down.
    HoldStart {
        binding: BindingId,
        action: Action,
    },
    /// The key went up or the engine cancelled the hold.
    HoldEnd {
        binding: BindingId,
    },
    Resolve {
        kind: String,
    },
    /// Read the choices for one field. The handler echoes the opaque id.
    Choices {
        id: u64,
        action: Action,
        field: String,
    },
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
    /// The index from 0 of the step that the next [`Request::Invoke`] runs.
    /// The engine sends it in [`crate::Event::Step`] and does not draw it.
    pub step: Option<usize>,
}

/// The state part of [`Feedback`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackState {
    Off,
    On,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Choices for a form request. This changes no stored setting.
    Choices {
        id: u64,
        options: Vec<ChoiceOption>,
    },
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
    /// Change the device light state. The engine applies decision 0034.
    /// After the stop state (`DisplayState::DeviceSleep`: `DC`, then `HAN`)
    /// the device sends no input report, so only the computer can wake it.
    DeviceDisplay(crate::DisplayState),
    /// Change and save device brightness through the usual settings path.
    DeviceBrightness(BrightnessChange),
    /// Select a profile (decision 0035, section 10).
    ///
    /// [`ProfileTarget::Profile`] is an identity. `page` is the page number
    /// from 0.
    /// `None` selects the first page of the profile. A selection of the
    /// active profile with no `page` does nothing. With `page`, the engine
    /// selects that page. A page that the profile does not have selects the
    /// first page, and the engine tells the shell.
    SelectProfile {
        target: ProfileTarget,
        page: Option<usize>,
    },
    /// Open a folder of keys. The engine checks the identity. See decision
    /// 0036.
    OpenFolder(crate::profile::FolderId),
    /// Close the last open folder. With no open folder, nothing changes.
    CloseFolder,
    /// Do the action `action`: the engine sends [`Request::Invoke`] with
    /// `binding` and `action` to the handler of the kind of `action`. The
    /// binding is the binding of the handler that asks. Thus the feedback of
    /// the action shows on the surface of that binding. If no handler has
    /// the kind, the engine tells the shell. The handler that asks gets no
    /// result. The reasons are in
    /// `docs/decisions/0017-toggle-and-sequence-actions.md`.
    Invoke { binding: BindingId, action: Action },
}

/// A brightness change. The engine fits the result to the device range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrightnessChange {
    Set(u8),
    Step(i16),
}

/// The profile that [`EngineRequest::SelectProfile`] selects.
///
/// The reasons are in decision 0035, section 10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileTarget {
    /// An identity. If no profile has it, the engine tells the shell.
    Profile(String),
    /// The profile after the active profile, in the order of the identities.
    /// After the last profile, the first profile.
    Next,
    /// The profile before the active profile. Before the first profile, the
    /// last profile.
    Previous,
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
    /// The icon of the kind on its kind tile: a name of the built-in icon
    /// set, for example `globe` (see [`crate::icons`], decision 0033). With
    /// no icon, the shell shows the first letter of the name.
    pub icon: Option<String>,
    /// The kind operates only with the Accessibility permission of macOS.
    /// The shell names these kinds where it tells the user about the
    /// permission.
    pub needs_accessibility: bool,
    /// A key's Press binding receives a hold start and end (decision 0042).
    /// Other gestures, tests and nested actions still receive Invoke.
    pub receives_release: bool,
    /// The Press binding receives [`Request::LongPress`] on a key with no
    /// explicit LongPress binding (decision 0042).
    pub handles_long_press: bool,
    pub fields: Vec<Field>,
    /// The direction of a turn of a knob, if the kind has one. The shell
    /// then shows one "Turn" card for the two turn gestures, and writes the
    /// two values to the two bindings (decision 0029, section 10.4).
    pub direction: Option<Direction>,
    /// The kind has two states, Off and On, and shows the state on the
    /// surface (decision 0017). The shell then shows the appearance of each
    /// state.
    pub states: Option<TwoStates>,
    /// True only for the kind "Back" (`opencraft.close-folder`). The shell
    /// uses it for the default Back key of a new folder and for the help
    /// text of a folder with no Back key. The shell compares no kind
    /// identifier (decision 0036).
    pub folder_back: bool,
    /// The list field whose item number is [`Feedback::step`] (decision 0042).
    /// The shell marks that row for the top-level action only.
    pub steps: Option<String>,
    /// Fields that the shell shows only for some values of a choice
    /// (decision 0038). All rules for a field must match. A field with no
    /// rule is always shown.
    pub shown_when: Vec<ShownWhen>,
    /// For a kind that needs the permission only for some values of a
    /// choice: the rule that names those values. The field of the rule
    /// is empty. The shell shows the status row, and it asks for the
    /// permission, only for an action whose settings satisfy the rule
    /// (decision 0038). `None` is no rule: [`Self::needs_accessibility`]
    /// decides.
    pub permission_when: Option<ShownWhen>,
}

impl KindDescription {
    /// Set the list field key whose item number is [`Feedback::step`].
    pub fn with_steps(mut self, field: impl Into<String>) -> Self {
        self.steps = Some(field.into());
        self
    }

    /// Declare that a Press binding starts a hold and receives its end.
    pub fn with_receives_release(mut self) -> Self {
        self.receives_release = true;
        self
    }

    /// Declare that a Press binding can handle a long press.
    pub fn with_handles_long_press(mut self) -> Self {
        self.handles_long_press = true;
        self
    }

    /// The text of the choice field `key` in `settings`. A missing value uses
    /// the default of that field. A value that is not text has no choice.
    pub fn choice_text(&self, settings: &Settings, key: &str) -> Option<String> {
        match settings.get(key) {
            Some(Value::Text(text)) => return Some(text.clone()),
            Some(_) => return None,
            None => {}
        }
        self.fields
            .iter()
            .find(|field| field.key == key)
            .and_then(|field| match &field.default {
                Some(Value::Text(text)) => Some(text.clone()),
                _ => None,
            })
    }

    /// True when an action of this kind with `settings` needs the
    /// Accessibility permission: the kind always needs it, or the rule
    /// [`Self::permission_when`] holds for the settings.
    pub fn needs_permission_for(&self, settings: &Settings) -> bool {
        self.needs_accessibility || self.permission_rule_holds(settings)
    }

    /// True when the status "needs the user" of this kind applies to an
    /// action with `settings`. A kind with no rule always shows its status.
    pub fn status_applies_to(&self, settings: &Settings) -> bool {
        self.permission_when.is_none() || self.permission_rule_holds(settings)
    }

    fn permission_rule_holds(&self, settings: &Settings) -> bool {
        self.permission_when.as_ref().is_some_and(|rule| {
            self.choice_text(settings, &rule.key)
                .is_some_and(|value| rule.values.contains(&value))
        })
    }
}

/// The shell shows [`ShownWhen::field`] only when the choice
/// [`ShownWhen::key`] has one of [`ShownWhen::values`]. It does not store
/// the field for a different value (decision 0038).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownWhen {
    /// The key of the field that the shell shows or hides.
    pub field: String,
    /// The key of the choice field that controls [`Self::field`].
    pub key: String,
    /// The values of [`Self::key`] for which the shell shows [`Self::field`].
    pub values: Vec<String>,
}

impl ShownWhen {
    /// One rule: show `field` when `key` has one of `values`.
    pub fn new(field: &str, key: &str, values: &[&str]) -> Self {
        Self {
            field: field.into(),
            key: key.into(),
            values: values.iter().map(|value| (*value).to_string()).collect(),
        }
    }
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
    /// The key of the legacy On label setting, if the kind has one.
    /// An edit of the On title in the shell replaces this setting with the
    /// stored label of the On face.
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

/// Which path the shell asks for. The value in the profile is text.
/// Kind `Application` tells the shell to show the list of installed
/// applications, and names the setting that stores the bundle identifier.
/// The shell reads this description. It has no list of kinds (rule 12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathKind {
    /// A file or a folder. The shell opens the file dialog.
    File,
    /// An application. The shell opens the list of installed applications.
    /// It stores the path in the field. When the application has a bundle
    /// identifier, the shell also stores that identifier in the setting
    /// named by `bundle_id`. An empty name means the field stores only the
    /// path. The path stays in the profile as the user gave it, and an
    /// export replaces the home directory (decision 0037).
    Application { bundle_id: String },
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
    /// Choices read from the handler when the form opens. Values are stable
    /// identifiers; an unavailable saved value is retained, not replaced.
    RuntimeChoice,
    /// Ordered selections from a run-time list. Stored as a list of text
    /// identifiers. A click adds at the end or removes an existing choice.
    RuntimeChoiceList,
    /// One profile of the device model. The shell shows the profiles from
    /// [`crate::Event::Profiles`] (the name and the colour) and stores the
    /// identity. A value that is not an identity stays. Decision 0035,
    /// section 10.
    Profile,
    /// An RGB colour as text #RRGGBB, with no alpha channel.
    Colour,
    Toggle,
    /// Modifiers, then one key; small letters and `+`. Decision 0014 gives
    /// the names: also the right-side modifiers, `fn`, and modifiers with
    /// no key.
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
    /// An action list with a delay before each step. The step settings use
    /// `delay_before_ms`; a missing value is zero for the first step, and
    /// the parent's `delay_ms` (default 100) for each later step.
    TimedActionList,
    /// The identity of a folder of the active profile, as the text form of
    /// a [`crate::profile::FolderId`]. The shell shows the folders from
    /// [`crate::Event::Pages`] (decision 0036).
    Folder,
}

impl InputType {
    /// `true` for each list of actions: [`InputType::ActionList`] and
    /// [`InputType::TimedActionList`].
    pub fn is_action_list(&self) -> bool {
        matches!(self, Self::ActionList | Self::TimedActionList)
    }
}

impl Field {
    /// The shared form field for the timing metadata of a contained step.
    pub fn step_delay(default: i64) -> Self {
        Self {
            key: "delay_before_ms".into(),
            label: "Delay before, in milliseconds".into(),
            input: InputType::Integer {
                min: 0,
                max: 10_000,
            },
            default: Some(Value::Integer(default)),
        }
    }
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
    /// The gesture on this control that prevents this binding from running.
    pub inert_by: Option<GestureKind>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    #[test]
    fn a_permission_rule_decides_for_the_settings_and_the_default_of_the_choice() {
        let kind = KindDescription {
            fields: vec![Field {
                key: "function".into(),
                label: "Function".into(),
                input: InputType::Text,
                default: Some(text("sleep")),
            }],
            permission_when: Some(ShownWhen::new("", "function", &["lock"])),
            ..Default::default()
        };
        let with = |value: Value| Settings::from([("function".to_string(), value)]);
        // A missing choice uses its default. A value that is not text has no
        // choice.
        assert!(!kind.needs_permission_for(&Settings::new()));
        assert!(!kind.needs_permission_for(&with(text("sleep"))));
        assert!(kind.needs_permission_for(&with(text("lock"))));
        assert!(!kind.needs_permission_for(&with(Value::Integer(1))));
        assert!(!kind.status_applies_to(&with(text("sleep"))));
        assert!(kind.status_applies_to(&with(text("lock"))));
        // A kind with no rule shows its status, and `needs_accessibility`
        // decides the permission.
        let always = KindDescription {
            needs_accessibility: true,
            ..Default::default()
        };
        assert!(always.needs_permission_for(&Settings::new()));
        assert!(always.status_applies_to(&Settings::new()));
        let none = KindDescription::default();
        assert!(!none.needs_permission_for(&Settings::new()));
        assert!(none.status_applies_to(&Settings::new()));
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
