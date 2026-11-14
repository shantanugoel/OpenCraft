//! The action "Quit an application".
//!
//! A missing "Force quit" value is Off. The call does not wait for the
//! application to end.

use crate::descriptions::{field, GROUP_SYSTEM};
use crate::kinds::{nonblank, toggle, Kind, Work};
use crate::system::Error;
use opencraft_engine::{InputType, KindDescription, PathKind, Settings};

/// The kind name.
pub(crate) const ID: &str = "opencraft.quit-application";

const NAME: &str = "Quit an application";

/// The registry value for this kind.
pub(crate) static KIND: QuitApplication = QuitApplication;

pub(crate) struct QuitApplication;

impl Kind for QuitApplication {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(&read(settings)?, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_SYSTEM.into(),
        icon: Some("circle-x".into()),
        fields: vec![
            field(
                "application",
                "Application",
                InputType::Path {
                    kind: PathKind::File,
                },
            ),
            field("force", "Force quit", InputType::Toggle),
        ],
        ..KindDescription::default()
    }
}

/// The application to quit, after a successful read of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Quit {
    pub(crate) application: String,
    pub(crate) force: bool,
}

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<Quit, String> {
    Ok(Quit {
        application: nonblank(ID, settings, "application")?.trim().to_string(),
        force: toggle(ID, settings, "force")?,
    })
}

/// Ask the application to quit. `force` stops it at once.
pub(crate) fn perform(quit: &Quit, work: &Work<'_>) -> Result<(), String> {
    work.system
        .quit_application(&quit.application, quit.force)
        .map_err(|e| match e {
            Error::NotRunning => "Cannot quit the application: it does not run.".into(),
            e => format!("Cannot quit the application: {e}."),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptions::setting_problem;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::{assert_system_kind, values, Work};
    use opencraft_engine::{Settings, Value};

    fn run(quit: &Quit, system: &System) -> Result<(), String> {
        let sender = None;
        perform(quit, &Work::new(system, &sender))
    }

    #[test]
    fn the_description_has_an_application_field_and_a_force_switch() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_system_kind(&description, NAME, "circle-x");
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(description.fields.len(), 2);
        assert_eq!(description.fields[0].key, "application");
        assert_eq!(description.fields[0].label, "Application");
        assert!(matches!(
            description.fields[0].input,
            InputType::Path {
                kind: PathKind::File,
            }
        ));
        assert_eq!(description.fields[0].default, None);
        assert_eq!(description.fields[1].key, "force");
        assert_eq!(description.fields[1].label, "Force quit");
        assert_eq!(description.fields[1].input, InputType::Toggle);
        // Force is Off, unless the user turns it On.
        assert_eq!(description.fields[1].default, None);
    }

    #[test]
    fn read_trims_the_application_and_treats_a_missing_force_as_off() {
        assert_eq!(
            read(&values(&[(
                "application",
                Value::Text("Calculator".into())
            )]))
            .unwrap(),
            Quit {
                application: "Calculator".into(),
                force: false,
            }
        );
        assert_eq!(
            read(&values(&[
                (
                    "application",
                    Value::Text(" /System/Applications/Calculator.app ".into()),
                ),
                ("force", Value::Boolean(true)),
            ]))
            .unwrap(),
            Quit {
                application: "/System/Applications/Calculator.app".into(),
                force: true,
            }
        );
        let quit = read(&values(&[
            ("application", Value::Text("Calculator".into())),
            ("force", Value::Boolean(false)),
        ]))
        .unwrap();
        assert!(!quit.force);
    }

    #[test]
    fn read_rejects_a_blank_application_and_a_force_value_that_is_not_a_toggle() {
        assert_eq!(
            read(&Settings::new()).unwrap_err(),
            setting_problem(ID, "application")
        );
        assert_eq!(
            read(&values(&[("application", Value::Text("  ".into()))])).unwrap_err(),
            setting_problem(ID, "application")
        );
        assert_eq!(
            read(&values(&[
                ("application", Value::Text("Calculator".into())),
                ("force", Value::Text("yes".into())),
            ]))
            .unwrap_err(),
            setting_problem(ID, "force")
        );
    }

    #[test]
    fn a_bad_read_does_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(&Settings::new(), &Work::new(&system, &sender))
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "application"));
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_quits_the_application_and_reports_a_system_error() {
        let system = System::default();
        run(
            &Quit {
                application: "Calculator".into(),
                force: false,
            },
            &system,
        )
        .unwrap();
        run(
            &Quit {
                application: "/System/Applications/Calculator.app".into(),
                force: true,
            },
            &system,
        )
        .unwrap();
        assert_eq!(
            system.calls(),
            [
                Call::Quit("Calculator".into(), false),
                Call::Quit("/System/Applications/Calculator.app".into(), true),
            ]
        );
        system.fail(Fault::NotRunning);
        assert_eq!(
            run(
                &Quit {
                    application: "Calculator".into(),
                    force: false,
                },
                &system,
            )
            .unwrap_err(),
            "Cannot quit the application: it does not run."
        );
        system.fail(Fault::Code("quit"));
        assert_eq!(
            run(
                &Quit {
                    application: "Calculator".into(),
                    force: false,
                },
                &system,
            )
            .unwrap_err(),
            "Cannot quit the application: the system refused the operation (code 42)."
        );
    }
}
