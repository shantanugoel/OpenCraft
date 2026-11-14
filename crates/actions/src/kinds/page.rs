//! The action "Change the page".
//!
//! The action asks the engine to show the next page, the previous page, or a
//! page number (decision 0016). The field `target` is `next`, `previous`, or
//! `number`. A missing target is `next`. With `number`, the field `page` is
//! the number that the user sees: the first page is 1. The request has the
//! number from 0. A missing page is page 1. The action reads the field `page`
//! only for `number` (decision 0038).
//!
//! With `show_number` On, the key shows its page number as a label. The
//! engine sends Appear again after each change of the page or of the
//! binding, and it removes the old feedback at each change (decision 0013).

use crate::descriptions::{choice, field, field_with_default, setting_problem, GROUP_NAVIGATION};
use crate::kinds::{Kind, Work};
use opencraft_engine::{
    EngineRequest, Feedback, InputType, KindDescription, Message, PageTarget, Settings, ShownWhen,
    Value,
};

/// The kind name.
pub(crate) const ID: &str = "opencraft.page";

const NAME: &str = "Change the page";

/// The largest page number that a page change accepts. The user sees page
/// numbers from 1.
pub(crate) const PAGE_LIMIT: i64 = 100;

/// The registry value for this kind.
pub(crate) static KIND: PageKind = PageKind;

pub(crate) struct PageKind;

impl Kind for PageKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(read(settings)?, work);
        Ok(())
    }

    fn appear(&self, settings: &Settings, work: &Work<'_>) {
        let Some(binding) = work.binding() else {
            return;
        };
        if settings.get("show_number") == Some(&Value::Boolean(true)) {
            work.send(Message::Feedback {
                binding,
                feedback: Some(label(binding.page)),
            });
        }
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_NAVIGATION.into(),
        icon: Some("arrow-right".into()),
        fields: vec![
            field_with_default(
                "target",
                "Page",
                choice(&[
                    ("next", "Next page"),
                    ("previous", "Previous page"),
                    ("number", "Page number"),
                ]),
                Value::Text("next".into()),
            ),
            field_with_default(
                "page",
                "Page number",
                InputType::Integer {
                    min: 1,
                    max: PAGE_LIMIT,
                },
                Value::Integer(1),
            ),
            // Off in an old profile: the key keeps its own title.
            field("show_number", "Show the page number", InputType::Toggle),
        ],
        shown_when: vec![ShownWhen::new("page", "target", &["number"])],
        ..KindDescription::default()
    }
}

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<PageTarget, String> {
    let target = match settings.get("target") {
        None => "next",
        Some(Value::Text(target)) => target.as_str(),
        Some(_) => "",
    };
    match target {
        "next" => Ok(PageTarget::Next),
        "previous" => Ok(PageTarget::Previous),
        "number" => match settings.get("page") {
            None => Ok(PageTarget::Number(0)),
            Some(Value::Integer(page)) if (1..=PAGE_LIMIT).contains(page) => {
                Ok(PageTarget::Number(*page as usize - 1))
            }
            Some(_) => Err(setting_problem(ID, "page")),
        },
        _ => Err(setting_problem(ID, "target")),
    }
}

/// Ask the engine to select the page.
pub(crate) fn perform(target: PageTarget, work: &Work<'_>) {
    work.ask(EngineRequest::SelectPage(target));
}

/// The feedback of a key on `page` (counted from 0): its number from 1.
fn label(page: usize) -> Feedback {
    Feedback {
        label: Some((page + 1).to_string()),
        ..Feedback::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::recording::System;
    use crate::kinds::values;
    use opencraft_engine::{BindingId, ControlId, ControlKind, GestureKind};
    use std::sync::mpsc;

    fn binding(page: usize) -> BindingId {
        BindingId {
            page,
            control: ControlId::new(ControlKind::Key, 1),
            gesture: GestureKind::Press,
            folder: None,
        }
    }

    #[test]
    fn the_description_has_a_page_number_that_only_the_number_target_shows() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_NAVIGATION);
        assert_eq!(description.group, "Navigation");
        assert_eq!(description.icon.as_deref(), Some("arrow-right"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        assert!(description.direction.is_none());
        assert!(description.states.is_none());
        assert!(!description.folder_back);
        let keys: Vec<_> = description.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, ["target", "page", "show_number"]);
        assert_eq!(
            description.fields[0].default,
            Some(Value::Text("next".into()))
        );
        assert_eq!(description.fields[1].default, Some(Value::Integer(1)));
        assert_eq!(description.fields[2].input, InputType::Toggle);
        assert_eq!(description.fields[2].default, None);
        assert_eq!(
            description.shown_when,
            vec![ShownWhen::new("page", "target", &["number"])]
        );
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn read_uses_next_and_counts_the_page_number_from_zero() {
        let text = |value: &str| Value::Text(value.into());
        assert_eq!(read(&Settings::new()).unwrap(), PageTarget::Next);
        for (pairs, expected) in [
            (vec![("target", text("next"))], PageTarget::Next),
            (vec![("target", text("previous"))], PageTarget::Previous),
            (vec![("target", text("number"))], PageTarget::Number(0)),
            (
                vec![("target", text("number")), ("page", Value::Integer(3))],
                PageTarget::Number(2),
            ),
            (
                vec![("target", text("number")), ("page", Value::Integer(100))],
                PageTarget::Number(99),
            ),
        ] {
            assert_eq!(read(&values(&pairs)).unwrap(), expected);
        }
    }

    #[test]
    fn read_does_not_use_the_page_number_for_next_or_previous() {
        for target in ["next", "previous"] {
            let settings = values(&[
                ("target", Value::Text(target.into())),
                ("page", Value::Text("bad".into())),
            ]);
            assert!(read(&settings).is_ok());
        }
    }

    #[test]
    fn read_rejects_a_bad_target_or_page_number() {
        let number = || ("target", Value::Text("number".into()));
        for (pairs, text) in [
            (
                vec![("target", Value::Text("sideways".into()))],
                "Change the page: select a value in the field \"Page\".",
            ),
            (
                vec![("target", Value::Integer(1))],
                "Change the page: select a value in the field \"Page\".",
            ),
            (
                vec![number(), ("page", Value::Integer(0))],
                "Change the page: set the field \"Page number\" to a number from 1 to 100.",
            ),
            (
                vec![number(), ("page", Value::Integer(101))],
                "Change the page: set the field \"Page number\" to a number from 1 to 100.",
            ),
            (
                vec![number(), ("page", Value::Text("2".into()))],
                "Change the page: set the field \"Page number\" to a number from 1 to 100.",
            ),
        ] {
            assert_eq!(read(&values(&pairs)).unwrap_err(), text);
        }
    }

    #[test]
    fn a_push_asks_the_engine_and_a_bad_setting_asks_for_nothing() {
        let system = System::default();
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        let work = Work::new(&system, &sender);
        KIND.invoke(
            &values(&[
                ("target", Value::Text("number".into())),
                ("page", Value::Integer(3)),
            ]),
            &work,
        )
        .unwrap();
        assert_eq!(
            messages.try_recv().unwrap(),
            Message::Engine(EngineRequest::SelectPage(PageTarget::Number(2)))
        );
        KIND.invoke(&values(&[("target", Value::Integer(1))]), &work)
            .unwrap_err();
        assert!(messages.try_recv().is_err());
        assert!(system.calls().is_empty());
    }

    #[test]
    fn a_key_shows_its_page_number_from_one_only_when_the_setting_is_on() {
        let system = System::default();
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        for page in [0, 1, 99] {
            let work = Work::new(&system, &sender).with_binding(binding(page));
            KIND.appear(&values(&[("show_number", Value::Boolean(true))]), &work);
            assert_eq!(
                messages.try_recv().unwrap(),
                Message::Feedback {
                    binding: binding(page),
                    feedback: Some(label(page)),
                }
            );
        }
        assert_eq!(label(4).label.as_deref(), Some("5"));
        // An old profile has no setting. Off, and a wrong type, send nothing.
        let work = Work::new(&system, &sender).with_binding(binding(2));
        for settings in [
            Settings::new(),
            values(&[("show_number", Value::Boolean(false))]),
            values(&[("show_number", Value::Text("yes".into()))]),
        ] {
            KIND.appear(&settings, &work);
        }
        assert!(messages.try_recv().is_err());
    }
}
