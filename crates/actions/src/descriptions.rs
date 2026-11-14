//! The descriptions that the shell uses to make the settings form.
//!
//! A kind that has a module in [`crate::kinds`] gives its description from
//! that module. The tables in this file serve only a kind that has not moved.

use opencraft_engine::{
    ChoiceOption, Direction, Field, InputType, KindDescription, PathKind, ShownWhen, TwoStates,
    Value,
};

pub(crate) const MICROPHONE: &str = "opencraft.microphone-mute";
pub(crate) const OUTPUT: &str = "opencraft.sound-output";
pub(crate) const PLAY_SOUND: &str = "opencraft.play-sound";
pub(crate) const STOP_SOUND: &str = "opencraft.stop-sound";
pub(crate) const PAGE: &str = "opencraft.page";
pub(crate) const PROFILE: &str = "opencraft.profile";
pub(crate) const OPEN_FOLDER: &str = "opencraft.open-folder";
pub(crate) const CLOSE_FOLDER: &str = "opencraft.close-folder";
pub(crate) const TOGGLE: &str = "opencraft.toggle";
pub(crate) const SEQUENCE: &str = "opencraft.sequence";
pub(crate) const SCREEN_OFF: &str = "opencraft.screen-off";
pub(crate) const DEVICE_BRIGHTNESS: &str = "opencraft.device-brightness";

/// The groups of the built-in kinds in the library (decision 0029, section
/// 11). The shell shows the groups in the order of their first kind.
pub(crate) const GROUP_SYSTEM: &str = "System";
pub(crate) const GROUP_MEDIA: &str = "Media";
const NAVIGATION: &str = "Navigation";
const LOGIC: &str = "Logic";

/// The largest page number that the page change accepts. The user sees page
/// numbers from 1.
pub(crate) const PAGE_LIMIT: i64 = 100;
/// The largest delay of a sequence, in milliseconds.
pub(crate) const DELAY_LIMIT: i64 = 10_000;
/// The delay of a sequence with no delay in its settings, in milliseconds.
pub(crate) const DEFAULT_DELAY: i64 = 100;

/// The label of the button that asks for the Accessibility permission.
pub(crate) const PERMISSION_BUTTON: &str = "Give the permission";

pub(crate) fn field(key: &str, label: &str, input: InputType) -> Field {
    Field {
        key: key.into(),
        label: label.into(),
        input,
        default: None,
    }
}

/// A [`field`] with a default value.
pub(crate) fn field_with_default(
    key: &str,
    label: &str,
    input: InputType,
    default: Value,
) -> Field {
    Field {
        key: key.into(),
        label: label.into(),
        input,
        default: Some(default),
    }
}

pub(crate) fn choice(options: &[(&str, &str)]) -> InputType {
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
    // A kind module owns its description. This match is only for a kind
    // that has not moved into [`crate::kinds`].
    if let Some(kind) = crate::kinds::find(kind) {
        return kind.description();
    }
    let (name, fields) = match kind {
        DEVICE_BRIGHTNESS => {
            let mut operation = field(
                "operation",
                "Operation",
                choice(&[("up", "Up"), ("down", "Down"), ("set", "Set value")]),
            );
            operation.default = Some(Value::Text("up".into()));
            let mut value = field(
                "value",
                "Brightness",
                InputType::Integer { min: 0, max: 100 },
            );
            value.default = Some(Value::Integer(80));
            let mut step = field("step", "Step", InputType::Integer { min: 1, max: 100 });
            step.default = Some(Value::Integer(5));
            ("Device brightness", vec![operation, value, step])
        }
        SCREEN_OFF => ("Screen off", vec![]),
        MICROPHONE => ("Microphone mute", vec![]),
        OUTPUT => {
            let mut operation = field(
                "operation",
                "Operation",
                choice(&[("set", "Set device"), ("next", "Next device")]),
            );
            operation.default = Some(Value::Text("set".into()));
            let mut devices = field("devices", "Cycle list", InputType::RuntimeChoiceList);
            devices.default = Some(Value::List(Vec::new()));
            (
                "Sound output",
                vec![
                    operation,
                    field("device", "Device", InputType::RuntimeChoice),
                    devices,
                ],
            )
        }
        PLAY_SOUND => {
            let mut volume = field("volume", "Volume", InputType::Integer { min: 0, max: 100 });
            volume.default = Some(Value::Integer(100));
            let mut device = field("device", "Output", InputType::RuntimeChoice);
            device.default = Some(Value::Text(String::new()));
            (
                "Play a sound",
                vec![
                    field(
                        "file",
                        "File",
                        InputType::Path {
                            kind: PathKind::File,
                        },
                    ),
                    volume,
                    device,
                ],
            )
        }
        STOP_SOUND => ("Stop all sound", vec![]),
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
            // Off in an old profile: the key keeps its own title.
            let show = field("show_number", "Show the page number", InputType::Toggle);
            ("Change the page", vec![target, page, show])
        }
        PROFILE => {
            let mut target = field(
                "target",
                "Select",
                choice(&[
                    ("profile", "A profile"),
                    ("next", "Next profile"),
                    ("previous", "Previous profile"),
                ]),
            );
            target.default = Some(Value::Text("profile".into()));
            let mut page = field(
                "page",
                "Page",
                InputType::Integer {
                    min: 0,
                    max: PAGE_LIMIT,
                },
            );
            page.default = Some(Value::Integer(0));
            (
                "Select a profile",
                vec![
                    target,
                    field("profile", "Profile", InputType::Profile),
                    page,
                ],
            )
        }
        OPEN_FOLDER => (
            "Open a folder",
            vec![field("folder", "Folder", InputType::Folder)],
        ),
        CLOSE_FOLDER => ("Back", vec![]),
        TOGGLE => (
            "Toggle",
            vec![
                field("on", "From Off to On", InputType::Action),
                field("off", "From On to Off", InputType::Action),
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
                vec![
                    field("actions", "Actions", InputType::TimedActionList),
                    delay,
                ],
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
        icon: Some(icon(kind).into()),
        // A kind module sets this flag on its own description.
        needs_accessibility: false,
        fields,
        direction: direction(kind),
        // The toggle shows its state on the surface (decision 0017).
        states: (kind == TOGGLE).then(|| TwoStates {
            on_label: Some("on_label".into()),
        }),
        folder_back: kind == CLOSE_FOLDER,
        shown_when: shown_when(kind),
    }
}

/// The fields that the shell shows only for some values of a choice
/// (decision 0038).
fn shown_when(kind: &str) -> Vec<ShownWhen> {
    match kind {
        DEVICE_BRIGHTNESS => vec![
            ShownWhen::new("value", "operation", &["set"]),
            ShownWhen::new("step", "operation", &["up", "down"]),
        ],
        PAGE => vec![ShownWhen::new("page", "target", &["number"])],
        _ => Vec::new(),
    }
}

/// The icon of a built-in kind on its kind tile: a name of the icon set of
/// the engine (decision 0029, section 20, and decision 0033).
fn icon(kind: &str) -> &'static str {
    match kind {
        MICROPHONE => "mic",
        OUTPUT => "speaker",
        PLAY_SOUND => "music",
        STOP_SOUND => "circle-stop",
        PAGE => "arrow-right",
        PROFILE => "layers",
        OPEN_FOLDER => "folder",
        CLOSE_FOLDER => "corner-up-left",
        TOGGLE => "toggle-left",
        SEQUENCE => "list",
        SCREEN_OFF => "monitor-off",
        DEVICE_BRIGHTNESS => "sun",
        _ => "circle-help",
    }
}

/// The direction field of a kind that has not moved (decision 0014).
/// Device brightness goes up for a turn to the right, and down for a turn
/// to the left.
fn direction(kind: &str) -> Option<Direction> {
    match kind {
        DEVICE_BRIGHTNESS => Some(Direction {
            key: "operation".into(),
            right: "up".into(),
            left: "down".into(),
        }),
        _ => None,
    }
}

/// The group of a built-in kind that has not moved, and its help text
/// (decision 0029, section 11).
fn library_place(kind: &str) -> (&'static str, Option<&'static str>) {
    match kind {
        SCREEN_OFF => ("Device", Some("Computer wake")),
        DEVICE_BRIGHTNESS => ("Device", Some("Knob")),
        MICROPHONE | OUTPUT | PLAY_SOUND | STOP_SOUND => (GROUP_MEDIA, None),
        PAGE | PROFILE | OPEN_FOLDER | CLOSE_FOLDER => (NAVIGATION, None),
        TOGGLE | SEQUENCE => (LOGIC, None),
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
        Some(
            InputType::Choice { .. }
            | InputType::RuntimeChoice
            | InputType::RuntimeChoiceList
            | InputType::Profile
            | InputType::Folder,
        ) => {
            format!("select a value in the field \"{label}\"")
        }
        Some(InputType::Toggle) => format!("set the field \"{label}\" to On or Off"),
        Some(InputType::Hotkey) => format!("record a hotkey in the field \"{label}\""),
        Some(InputType::Action | InputType::ActionList | InputType::TimedActionList) => {
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
