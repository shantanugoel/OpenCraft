//! The action "Mac system function".
//!
//! A missing function is Mission Control. The work reports the cause when
//! macOS refuses Sleep. Every function needs the Accessibility permission
//! today. [`needs_permission`] answers for one function, so a function that
//! sends no key can later answer false without a change to the handler.

use crate::descriptions::{choice, field_with_default, GROUP_SYSTEM};
use crate::kinds::{Kind, Work};
use crate::mac_function::MacFunction;
use opencraft_engine::{KindDescription, Settings, Value};

/// The kind name.
pub(crate) const ID: &str = "opencraft.mac-function";

const NAME: &str = "Mac system function";

/// The registry value for this kind.
pub(crate) static KIND: MacFunctionKind = MacFunctionKind;

pub(crate) struct MacFunctionKind;

impl Kind for MacFunctionKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        let function = read(settings)?;
        if needs_permission(function) && !work.keys_allowed() {
            return Err(crate::handlers::no_permission());
        }
        perform(function, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_SYSTEM.into(),
        icon: Some("monitor".into()),
        help: Some("Accessibility".into()),
        needs_accessibility: true,
        fields: vec![field_with_default(
            "function",
            "Function",
            choice(&[
                ("mission-control", "Mission Control"),
                ("launchpad", "Launchpad"),
                ("screenshot", "Screenshot"),
                ("screen-saver", "Screen saver"),
                ("sleep", "Sleep"),
                ("lock", "Lock"),
                ("show-desktop", "Show desktop"),
            ]),
            Value::Text("mission-control".into()),
        )],
        ..KindDescription::default()
    }
}

/// True when `function` needs the Accessibility permission.
///
/// Every function needs the permission. A function that sends no key can
/// answer false on its own arm. The key handler does not choose.
pub(crate) fn needs_permission(function: MacFunction) -> bool {
    match function {
        MacFunction::MissionControl
        | MacFunction::Launchpad
        | MacFunction::Screenshot
        | MacFunction::ScreenSaver
        | MacFunction::Sleep
        | MacFunction::Lock
        | MacFunction::ShowDesktop => true,
    }
}

/// Read the settings. A missing value is Mission Control. The error is the
/// text for the user.
pub(crate) fn read(settings: &Settings) -> Result<MacFunction, String> {
    let name = match settings.get("function") {
        None => "mission-control",
        Some(Value::Text(value)) => value.as_str(),
        Some(_) => return Err(crate::descriptions::setting_problem(ID, "function")),
    };
    MacFunction::from_name(name).ok_or_else(|| crate::descriptions::setting_problem(ID, "function"))
}

/// Do the function. A system error keeps its own cause.
pub(crate) fn perform(function: MacFunction, work: &Work<'_>) -> Result<(), String> {
    work.system
        .mac_function(function)
        .map_err(|error| format!("Cannot do the Mac system function: {error}."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptions::setting_problem;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::{values, Work};
    use opencraft_engine::{InputType, Settings};

    fn run(function: MacFunction, system: &System) -> Result<(), String> {
        let sender = None;
        perform(function, &Work::new(system, &sender))
    }

    #[test]
    fn the_description_lists_every_function_and_needs_the_permission() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_SYSTEM);
        assert_eq!(description.icon.as_deref(), Some("monitor"));
        assert_eq!(description.help.as_deref(), Some("Accessibility"));
        assert!(description.needs_accessibility);
        assert!(description.direction.is_none());
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(description.fields.len(), 1);
        assert_eq!(description.fields[0].key, "function");
        assert_eq!(description.fields[0].label, "Function");
        assert_eq!(
            description.fields[0].default,
            Some(Value::Text("mission-control".into()))
        );
        let InputType::Choice { options } = &description.fields[0].input else {
            panic!("expected a choice");
        };
        assert_eq!(
            options
                .iter()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            MacFunction::ALL.map(MacFunction::name)
        );
    }

    #[test]
    fn every_function_needs_the_permission() {
        for function in MacFunction::ALL {
            assert!(needs_permission(function), "{}", function.name());
        }
    }

    #[test]
    fn read_defaults_to_mission_control_and_rejects_an_unknown_function() {
        assert_eq!(read(&Settings::new()).unwrap(), MacFunction::MissionControl);
        for function in MacFunction::ALL {
            assert_eq!(
                read(&values(&[(
                    "function",
                    Value::Text(function.name().into())
                )]))
                .unwrap(),
                function
            );
        }
        for value in [
            Value::Text("do-not-disturb".into()),
            Value::Text(String::new()),
            Value::Integer(1),
        ] {
            assert_eq!(
                read(&values(&[("function", value)])).unwrap_err(),
                setting_problem(ID, "function")
            );
        }
    }

    #[test]
    fn a_bad_read_and_a_missing_permission_do_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(
                &values(&[("function", Value::Text("do-not-disturb".into()))]),
                &Work::new(&system, &sender),
            )
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "function"));
        let allow = || true;
        let blocked = Work::new(&system, &sender).with_keys(false, &allow);
        for function in MacFunction::ALL {
            let error = KIND
                .invoke(
                    &values(&[("function", Value::Text(function.name().into()))]),
                    &blocked,
                )
                .unwrap_err();
            assert_eq!(error, crate::handlers::no_permission());
        }
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_does_each_function_and_keeps_the_cause_of_a_system_error() {
        let system = System::default();
        for function in MacFunction::ALL {
            run(function, &system).unwrap();
        }
        assert_eq!(system.calls(), MacFunction::ALL.map(Call::MacFunction));
        system.fail(Fault::SleepNotAllowed);
        assert_eq!(
            run(MacFunction::Sleep, &system).unwrap_err(),
            format!(
                "Cannot do the Mac system function: {}.",
                crate::system::Error::SleepNotAllowed
            )
        );
        system.fail(Fault::Code("mac-function"));
        let sender = None;
        let gone = || false;
        let work = Work::new(&system, &sender).with_keys(true, &gone);
        let error = KIND
            .invoke(&values(&[("function", Value::Text("lock".into()))]), &work)
            .unwrap_err();
        assert!(error.contains("code 42"), "{error}");
        assert!(!error.contains("Accessibility"), "{error}");
    }
}
