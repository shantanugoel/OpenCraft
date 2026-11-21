//! The helpers that the kind modules use to make their descriptions.
//!
//! Each built-in kind has a module in [`crate::kinds`], and that module gives
//! the description of the kind (decision 0014). This file has the names of the
//! groups of the library, the helpers for a field, and the text of a problem
//! with a setting.

use opencraft_engine::{ChoiceOption, Field, InputType, KindDescription, Value};

/// The groups of the built-in kinds in the library (decision 0029, section
/// 11). The shell shows the groups in the order of their first kind.
pub(crate) const GROUP_SYSTEM: &str = "System";
pub(crate) const GROUP_MEDIA: &str = "Media";
pub(crate) const GROUP_NAVIGATION: &str = "Navigation";
pub(crate) const GROUP_LOGIC: &str = "Logic";
pub(crate) const GROUP_DEVICE: &str = "Device";

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

/// The description of the kind `kind`. A name that is not a built-in kind
/// gets an empty description that has only the name.
pub(crate) fn description(kind: &str) -> KindDescription {
    match crate::kinds::find(kind) {
        Some(kind) => kind.description(),
        None => KindDescription {
            kind: kind.into(),
            ..KindDescription::default()
        },
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
        Some(InputType::TurnList) => format!("select the functions in the field \"{label}\" again"),
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
