//! The action "Type text".
//!
//! The text keeps its spaces. Paste and Return are Off when the setting is
//! missing. The work needs the Accessibility permission. Return is a hotkey
//! after the text, and only when the text was sent.

use crate::descriptions::{field, GROUP_SYSTEM};
use crate::kinds::{required_text, toggle, Kind, Work};
use opencraft_engine::{InputType, KindDescription, Settings};

/// The kind name.
pub(crate) const ID: &str = "opencraft.type-text";

const NAME: &str = "Type text";

/// The registry value for this kind.
pub(crate) static KIND: TypeText = TypeText;

pub(crate) struct TypeText;

impl Kind for TypeText {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        let text = read(settings)?;
        if !work.keys_allowed() {
            return Err(crate::handlers::no_permission());
        }
        perform(&text, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_SYSTEM.into(),
        icon: Some("type".into()),
        help: Some("Accessibility".into()),
        needs_accessibility: true,
        fields: vec![
            field("text", "Text", InputType::LongText),
            field("paste", "Paste with clipboard", InputType::Toggle),
            field("return_after", "Push Return after", InputType::Toggle),
        ],
        ..KindDescription::default()
    }
}

/// The text to send, after a successful read of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Typed {
    pub(crate) text: String,
    pub(crate) paste: bool,
    pub(crate) return_after: bool,
}

/// Read the settings. The text keeps its spaces. The error is the text for
/// the user.
pub(crate) fn read(settings: &Settings) -> Result<Typed, String> {
    Ok(Typed {
        text: required_text(ID, settings, "text")?.to_string(),
        paste: toggle(ID, settings, "paste")?,
        return_after: toggle(ID, settings, "return_after")?,
    })
}

/// Type or paste the text, then press Return when the settings ask for it.
pub(crate) fn perform(text: &Typed, work: &Work<'_>) -> Result<(), String> {
    let result = if text.paste {
        work.system.paste_text(&text.text)
    } else {
        work.system.type_text(&text.text)
    };
    result
        .and_then(|()| {
            if text.return_after {
                work.system
                    .send_hotkey(&"return".parse().expect("Return key"))
            } else {
                Ok(())
            }
        })
        .map_err(|error| work.input_error(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptions::setting_problem;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::{values, Work};
    use opencraft_engine::{Settings, Value};

    fn run(text: &Typed, system: &System) -> Result<(), String> {
        let sender = None;
        perform(text, &Work::new(system, &sender))
    }

    fn typed(text: &str, paste: bool, return_after: bool) -> Typed {
        Typed {
            text: text.into(),
            paste,
            return_after,
        }
    }

    #[test]
    fn the_description_is_the_system_form_with_paste_and_return() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_SYSTEM);
        assert_eq!(description.icon.as_deref(), Some("type"));
        assert_eq!(description.help.as_deref(), Some("Accessibility"));
        assert!(description.needs_accessibility);
        assert!(description.direction.is_none());
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(
            description
                .fields
                .iter()
                .map(|field| (
                    field.key.as_str(),
                    field.input.clone(),
                    field.default.clone()
                ))
                .collect::<Vec<_>>(),
            [
                ("text", InputType::LongText, None),
                ("paste", InputType::Toggle, None),
                ("return_after", InputType::Toggle, None),
            ]
        );
        assert_eq!(description.fields[1].label, "Paste with clipboard");
        assert_eq!(description.fields[2].label, "Push Return after");
    }

    #[test]
    fn read_keeps_spaces_and_treats_a_missing_option_as_off() {
        assert_eq!(
            read(&values(&[
                ("text", Value::Text(" \n ".into())),
                ("extra", Value::Boolean(false)),
            ]))
            .unwrap(),
            typed(" \n ", false, false)
        );
        assert_eq!(
            read(&values(&[
                ("text", Value::Text("Long text 😀".into())),
                ("paste", Value::Boolean(true)),
                ("return_after", Value::Boolean(true)),
            ]))
            .unwrap(),
            typed("Long text 😀", true, true)
        );
        assert_eq!(
            read(&Settings::new()).unwrap_err(),
            setting_problem(ID, "text")
        );
        assert_eq!(
            read(&values(&[("text", Value::Text(String::new()))])).unwrap_err(),
            setting_problem(ID, "text")
        );
        assert_eq!(
            read(&values(&[
                ("text", Value::Text("Text".into())),
                ("paste", Value::Integer(1)),
            ]))
            .unwrap_err(),
            setting_problem(ID, "paste")
        );
        assert_eq!(
            read(&values(&[
                ("text", Value::Text("Text".into())),
                ("return_after", Value::Text("yes".into())),
            ]))
            .unwrap_err(),
            setting_problem(ID, "return_after")
        );
    }

    #[test]
    fn a_bad_read_and_a_missing_permission_do_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(&Settings::new(), &Work::new(&system, &sender))
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "text"));
        let allow = || true;
        let blocked = Work::new(&system, &sender).with_keys(false, &allow);
        let error = KIND
            .invoke(&values(&[("text", Value::Text("Text".into()))]), &blocked)
            .unwrap_err();
        assert_eq!(error, crate::handlers::no_permission());
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_types_or_pastes_and_presses_return_only_after_success() {
        let system = System::default();
        run(&typed("Long text 😀", false, true), &system).unwrap();
        run(&typed("Long text 😀", true, false), &system).unwrap();
        assert_eq!(
            system.calls(),
            [
                Call::Text("Long text 😀".into()),
                Call::Hotkey("return".into()),
                Call::Paste("Long text 😀".into()),
            ]
        );
        system.fail(Fault::Code("text"));
        assert_eq!(
            run(&typed("Text", false, true), &system).unwrap_err(),
            "Cannot send keys or mouse events: the system refused the operation (code 42)."
        );
        assert_eq!(system.calls().len(), 4);
        assert!(!matches!(system.calls().last(), Some(Call::Hotkey(_))));
    }
}
