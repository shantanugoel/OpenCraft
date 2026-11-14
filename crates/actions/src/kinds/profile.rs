//! The action "Select a profile".
//!
//! The action asks the engine to select a profile and, if the user wants it,
//! a page of that profile (decisions 0018 and 0035). The field `target` is
//! `profile`, `next`, or `previous`. A missing target is `profile`, so a
//! binding from a release up to 0.2.0 still sends the stored name. With
//! `profile`, the field `profile` is the name. The field `page` is a page
//! number from 1. The number 0 and a missing value select the first page of
//! the profile. The request has the number from 0.

use crate::descriptions::{choice, field, field_with_default, setting_problem, GROUP_NAVIGATION};
use crate::kinds::page::PAGE_LIMIT;
use crate::kinds::{nonblank, Kind, Work};
use opencraft_engine::{
    EngineRequest, InputType, KindDescription, ProfileTarget, Settings, ShownWhen, Value,
};

/// The kind name.
pub(crate) const ID: &str = "opencraft.profile";

/// The stored names of the targets.
const PROFILE: &str = "profile";
const NEXT: &str = "next";
const PREVIOUS: &str = "previous";

/// The targets in form order: the stored name and the label. The first is
/// the default. The description and [`read`] both use this table.
const TARGETS: [(&str, &str); 3] = [
    (PROFILE, "A profile"),
    (NEXT, "Next profile"),
    (PREVIOUS, "Previous profile"),
];

/// The page number of the default: the first page of the profile.
const PAGE_DEFAULT: i64 = 0;

const NAME: &str = "Select a profile";

/// The registry value for this kind.
pub(crate) static KIND: ProfileKind = ProfileKind;

pub(crate) struct ProfileKind;

impl Kind for ProfileKind {
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
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_NAVIGATION.into(),
        icon: Some("layers".into()),
        fields: vec![
            field_with_default(
                "target",
                "Select",
                choice(&TARGETS),
                Value::Text(TARGETS[0].0.into()),
            ),
            field("profile", "Profile", InputType::Profile),
            field_with_default(
                "page",
                "Page",
                InputType::Integer {
                    min: PAGE_DEFAULT,
                    max: PAGE_LIMIT,
                },
                Value::Integer(PAGE_DEFAULT),
            ),
        ],
        // The profile name is for the target "A profile" only.
        shown_when: vec![ShownWhen::new("profile", "target", &[PROFILE])],
        ..KindDescription::default()
    }
}

/// The request, after a successful read of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selection {
    pub(crate) target: ProfileTarget,
    /// The page from 0. `None` is the first page of the profile.
    pub(crate) page: Option<usize>,
}

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<Selection, String> {
    let target = match settings.get("target") {
        None => TARGETS[0].0,
        Some(Value::Text(target)) => target.as_str(),
        Some(_) => "",
    };
    let target = match target {
        NEXT => ProfileTarget::Next,
        PREVIOUS => ProfileTarget::Previous,
        PROFILE => ProfileTarget::Profile(nonblank(ID, settings, "profile")?.trim().to_string()),
        _ => return Err(setting_problem(ID, "target")),
    };
    let page = match settings.get("page") {
        None | Some(Value::Integer(PAGE_DEFAULT)) => None,
        Some(Value::Integer(page)) if (1..=PAGE_LIMIT).contains(page) => Some(*page as usize - 1),
        Some(_) => return Err(setting_problem(ID, "page")),
    };
    Ok(Selection { target, page })
}

/// Ask the engine to select the profile.
pub(crate) fn perform(selection: Selection, work: &Work<'_>) {
    work.ask(EngineRequest::SelectProfile {
        target: selection.target,
        page: selection.page,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::values;
    use crate::tests::support::Fake;
    use opencraft_engine::Message;
    use std::sync::mpsc;

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    #[test]
    fn the_description_has_a_target_a_profile_and_a_page() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_NAVIGATION);
        assert_eq!(description.icon.as_deref(), Some("layers"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        assert!(description.direction.is_none());
        assert!(description.states.is_none());
        assert!(!description.folder_back);
        assert_eq!(
            description.shown_when,
            [ShownWhen::new("profile", "target", &["profile"])]
        );
        let keys: Vec<_> = description.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, ["target", "profile", "page"]);
        assert_eq!(description.fields[0].default, Some(text("profile")));
        assert_eq!(description.fields[1].input, InputType::Profile);
        assert_eq!(description.fields[1].default, None);
        assert_eq!(description.fields[2].default, Some(Value::Integer(0)));
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn read_gets_an_identity_a_neighbour_and_a_page_from_zero() {
        assert_eq!(
            read(&values(&[
                ("target", text("profile")),
                ("profile", text(" work ")),
                ("page", Value::Integer(2)),
            ]))
            .unwrap(),
            Selection {
                target: ProfileTarget::Profile("work".into()),
                page: Some(1),
            }
        );
        // A binding from a release up to 0.2.0 stores a name and no target.
        assert_eq!(
            read(&values(&[("profile", text("Video calls"))])).unwrap(),
            Selection {
                target: ProfileTarget::Profile("Video calls".into()),
                page: None,
            }
        );
        for (target, expected, page, number) in [
            ("next", ProfileTarget::Next, Value::Integer(0), None),
            (
                "previous",
                ProfileTarget::Previous,
                Value::Integer(1),
                Some(0),
            ),
            ("next", ProfileTarget::Next, Value::Integer(100), Some(99)),
        ] {
            assert_eq!(
                read(&values(&[("target", text(target)), ("page", page)])).unwrap(),
                Selection {
                    target: expected,
                    page: number,
                }
            );
        }
    }

    #[test]
    fn read_does_not_use_the_profile_name_for_next_or_previous() {
        for target in ["next", "previous"] {
            let settings = values(&[("target", text(target)), ("profile", Value::Integer(1))]);
            assert!(read(&settings).is_ok());
        }
    }

    #[test]
    fn read_rejects_a_bad_target_profile_or_page() {
        let profile = "Select a profile: select a value in the field \"Profile\".";
        let target = "Select a profile: select a value in the field \"Select\".";
        let page = "Select a profile: set the field \"Page\" to a number from 0 to 100.";
        for (pairs, expected) in [
            (vec![], profile),
            (vec![("profile", text("  "))], profile),
            (vec![("profile", Value::Integer(1))], profile),
            (
                vec![("target", text("sideways")), ("profile", text("a"))],
                target,
            ),
            (
                vec![("target", Value::Integer(1)), ("profile", text("a"))],
                target,
            ),
            (
                vec![("target", text("next")), ("page", Value::Integer(101))],
                page,
            ),
            (
                vec![("target", text("next")), ("page", Value::Integer(-1))],
                page,
            ),
            (vec![("profile", text("a")), ("page", text("2"))], page),
        ] {
            assert_eq!(read(&values(&pairs)).unwrap_err(), expected);
        }
    }

    #[test]
    fn a_push_asks_the_engine_and_a_bad_setting_asks_for_nothing() {
        let system = Fake::default();
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        let work = Work::new(&system, &sender);
        KIND.invoke(
            &values(&[("profile", text("work")), ("page", Value::Integer(3))]),
            &work,
        )
        .unwrap();
        assert_eq!(
            messages.try_recv().unwrap(),
            Message::Engine(EngineRequest::SelectProfile {
                target: ProfileTarget::Profile("work".into()),
                page: Some(2),
            })
        );
        KIND.invoke(&Settings::new(), &work).unwrap_err();
        assert!(messages.try_recv().is_err());
        assert!(system.calls().is_empty());
    }
}
