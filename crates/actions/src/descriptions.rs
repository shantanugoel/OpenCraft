//! The descriptions that the shell uses to make the settings form.

use opencraft_engine::{
    ChoiceOption, Direction, Field, InputType, KindDescription, PathKind, Value,
};

pub(crate) const APPLICATION: &str = "opencraft.open-application";
pub(crate) const URL: &str = "opencraft.open-url";
pub(crate) const HOTKEY: &str = "opencraft.hotkey";
pub(crate) const TEXT: &str = "opencraft.type-text";
pub(crate) const MEDIA: &str = "opencraft.media-key";
pub(crate) const VOLUME: &str = "opencraft.volume";
pub(crate) const SHELL: &str = "opencraft.shell-command";
pub(crate) const KEY_KINDS: [&str; 3] = [HOTKEY, TEXT, MEDIA];
pub(crate) const PAGE: &str = "opencraft.page";
pub(crate) const PROFILE: &str = "opencraft.profile";
pub(crate) const TOGGLE: &str = "opencraft.toggle";
pub(crate) const SEQUENCE: &str = "opencraft.sequence";

/// The groups of the built-in kinds in the library (decision 0029, section
/// 11). The shell shows the groups in the order of their first kind.
const SYSTEM: &str = "System";
const MEDIA_GROUP: &str = "Media";
const NAVIGATION: &str = "Navigation";
const LOGIC: &str = "Logic";
/// The help text of a kind that needs the Accessibility permission.
const ACCESSIBILITY: &str = "Accessibility";

/// The largest page number that the page change accepts. The user sees page
/// numbers from 1.
pub(crate) const PAGE_LIMIT: i64 = 100;
/// The largest delay of a sequence, in milliseconds.
pub(crate) const DELAY_LIMIT: i64 = 10_000;
/// The delay of a sequence with no delay in its settings, in milliseconds.
pub(crate) const DEFAULT_DELAY: i64 = 100;

/// The label of the button that asks for the Accessibility permission.
pub(crate) const PERMISSION_BUTTON: &str = "Give the permission";

fn field(key: &str, label: &str, input: InputType) -> Field {
    Field {
        key: key.into(),
        label: label.into(),
        input,
        default: None,
    }
}

fn choice(options: &[(&str, &str)]) -> InputType {
    InputType::Choice {
        options: options
            .iter()
            .map(|(value, label)| ChoiceOption {
                value: (*value).into(),
                label: (*label).into(),
            })
            .collect(),
    }
}

pub(crate) fn description(kind: &str) -> KindDescription {
    let (name, fields) = match kind {
        APPLICATION => (
            "Open an application",
            vec![field(
                "application",
                "Application",
                InputType::Path {
                    kind: PathKind::Application,
                },
            )],
        ),
        URL => ("Open a URL", vec![field("url", "URL", InputType::Text)]),
        HOTKEY => (
            "Send a hotkey",
            vec![field("hotkey", "Hotkey", InputType::Hotkey)],
        ),
        TEXT => (
            "Type text",
            vec![field("text", "Text", InputType::LongText)],
        ),
        MEDIA => (
            "Media key",
            vec![field(
                "key",
                "Key",
                choice(&[
                    ("play-pause", "Play or pause"),
                    ("next", "Next"),
                    ("previous", "Previous"),
                    ("volume-up", "Volume up"),
                    ("volume-down", "Volume down"),
                    ("mute", "Mute"),
                ]),
            )],
        ),
        VOLUME => {
            let mut operation = field(
                "operation",
                "Operation",
                choice(&[("up", "Up"), ("down", "Down"), ("mute", "Mute")]),
            );
            operation.default = Some(Value::Text("up".into()));
            let mut step = field("step", "Step", InputType::Integer { min: 1, max: 25 });
            step.default = Some(Value::Integer(5));
            ("Sound volume", vec![operation, step])
        }
        SHELL => (
            "Run a shell command",
            vec![field("command", "Command", InputType::LongText)],
        ),
        PAGE => {
            let mut target = field(
                "target",
                "Page",
                choice(&[
                    ("next", "Next page"),
                    ("previous", "Previous page"),
                    ("number", "Page number"),
                ]),
            );
            target.default = Some(Value::Text("next".into()));
            let mut page = field(
                "page",
                "Page number",
                InputType::Integer {
                    min: 1,
                    max: PAGE_LIMIT,
                },
            );
            page.default = Some(Value::Integer(1));
            ("Change the page", vec![target, page])
        }
        PROFILE => (
            "Select a profile",
            vec![field("profile", "Profile name", InputType::Text)],
        ),
        TOGGLE => (
            "Toggle",
            vec![
                field("on", "Action for on", InputType::Action),
                field("off", "Action for off", InputType::Action),
                field("on_label", "Label for on", InputType::Text),
            ],
        ),
        SEQUENCE => {
            let mut delay = field(
                "delay_ms",
                "Delay between two actions, in milliseconds",
                InputType::Integer {
                    min: 0,
                    max: DELAY_LIMIT,
                },
            );
            delay.default = Some(Value::Integer(DEFAULT_DELAY));
            (
                "Sequence of actions",
                vec![field("actions", "Actions", InputType::ActionList), delay],
            )
        }
        _ => {
            return KindDescription {
                kind: kind.into(),
                ..KindDescription::default()
            }
        }
    };
    let (group, help) = library_place(kind);
    KindDescription {
        kind: kind.into(),
        name: name.into(),
        group: group.into(),
        help: help.map(Into::into),
        needs_accessibility: KEY_KINDS.contains(&kind),
        fields,
        direction: direction(kind),
    }
}

/// The direction field of a kind for a turn of a knob (decision 0014):
/// "Sound volume" goes up for a turn to the right, and down for a turn to
/// the left.
fn direction(kind: &str) -> Option<Direction> {
    match kind {
        VOLUME => Some(Direction {
            key: "operation".into(),
            right: "up".into(),
            left: "down".into(),
        }),
        _ => None,
    }
}

/// The group of a built-in kind in the library, and its help text
/// (decision 0029, section 11). The kinds that send keys need the
/// Accessibility permission, thus their help text names it.
fn library_place(kind: &str) -> (&'static str, Option<&'static str>) {
    match kind {
        HOTKEY | TEXT => (SYSTEM, Some(ACCESSIBILITY)),
        MEDIA => (MEDIA_GROUP, Some(ACCESSIBILITY)),
        VOLUME => (MEDIA_GROUP, Some("Knob")),
        PAGE | PROFILE => (NAVIGATION, None),
        TOGGLE | SEQUENCE => (LOGIC, None),
        APPLICATION | URL | SHELL => (SYSTEM, None),
        _ => ("", None),
    }
}

/// The text for the user when a handler cannot use the setting `key` of an
/// action of `kind`. The text gives the name of the kind and the label of the
/// field, as the editor shows them, and what to do. It never shows the key.
///
/// The text comes only from the description of the kind. Thus each handler
/// uses this function, and a new kind gets a correct text with no more code.
pub(crate) fn setting_problem(kind: &str, key: &str) -> String {
    let description = description(kind);
    let field = description.fields.iter().find(|f| f.key == key);
    let label = field.map_or("", |f| f.label.as_str());
    let what = match field.map(|f| &f.input) {
        None => "correct the settings".to_string(),
        Some(InputType::Integer { min, max }) => {
            format!("set the field \"{label}\" to a number from {min} to {max}")
        }
        Some(InputType::Choice { .. }) => format!("select a value in the field \"{label}\""),
        Some(InputType::Toggle) => format!("set the field \"{label}\" to On or Off"),
        Some(InputType::Hotkey) => format!("record a hotkey in the field \"{label}\""),
        Some(InputType::Action | InputType::ActionList) => {
            format!("select the action in the field \"{label}\" again")
        }
        Some(_) => format!("type a value in the field \"{label}\""),
    };
    let name = if description.name.is_empty() {
        "This action"
    } else {
        description.name.as_str()
    };
    format!("{name}: {what}.")
}
