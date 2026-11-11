//! The descriptions that the shell uses to make the settings form.

use opencraft_engine::{ChoiceOption, Field, InputType, KindDescription, PathKind, Value};

pub(crate) const APPLICATION: &str = "opencraft.open-application";
pub(crate) const URL: &str = "opencraft.open-url";
pub(crate) const HOTKEY: &str = "opencraft.hotkey";
pub(crate) const TEXT: &str = "opencraft.type-text";
pub(crate) const MEDIA: &str = "opencraft.media-key";
pub(crate) const VOLUME: &str = "opencraft.volume";
pub(crate) const SHELL: &str = "opencraft.shell-command";
pub(crate) const KEY_KINDS: [&str; 3] = [HOTKEY, TEXT, MEDIA];

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
        _ => {
            return KindDescription {
                kind: kind.into(),
                name: String::new(),
                fields: vec![],
            }
        }
    };
    KindDescription {
        kind: kind.into(),
        name: name.into(),
        fields,
    }
}
