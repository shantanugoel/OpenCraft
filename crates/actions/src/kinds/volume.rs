//! The action "Sound volume".
//!
//! A missing operation is Up, and a missing step is 5. The level stays in
//! the range 0 to 100. Up and Down clear mute. Mute switches mute. The
//! volume handler keeps the poll and the feedback. This module reads the
//! settings and changes the volume.

use crate::descriptions::{choice, field_with_default, GROUP_MEDIA};
use crate::kinds::{Kind, Work};
use opencraft_engine::{Direction, KindDescription, Settings, Value};

/// The kind name.
pub(crate) const ID: &str = "opencraft.volume";

const NAME: &str = "Sound volume";

/// The registry value for this kind.
pub(crate) static KIND: VolumeKind = VolumeKind;

pub(crate) struct VolumeKind;

impl Kind for VolumeKind {
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
        group: GROUP_MEDIA.into(),
        icon: Some("volume-2".into()),
        help: Some("Knob".into()),
        needs_accessibility: false,
        fields: vec![
            field_with_default(
                "operation",
                "Operation",
                choice(&[("up", "Up"), ("down", "Down"), ("mute", "Mute")]),
                Value::Text("up".into()),
            ),
            field_with_default(
                "step",
                "Step",
                opencraft_engine::InputType::Integer { min: 1, max: 25 },
                Value::Integer(5),
            ),
        ],
        direction: Some(Direction {
            key: "operation".into(),
            right: "up".into(),
            left: "down".into(),
        }),
        ..KindDescription::default()
    }
}

/// One change of the sound volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    Up,
    Down,
    Mute,
}

/// The change, after a successful read of the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Change {
    pub(crate) operation: Operation,
    pub(crate) step: u8,
}

/// Read the settings. A missing operation is Up. A missing step is 5.
/// The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<Change, String> {
    let operation = match settings.get("operation") {
        None => Operation::Up,
        Some(Value::Text(value)) => match value.as_str() {
            "up" => Operation::Up,
            "down" => Operation::Down,
            "mute" => Operation::Mute,
            _ => return Err(crate::descriptions::setting_problem(ID, "operation")),
        },
        Some(_) => return Err(crate::descriptions::setting_problem(ID, "operation")),
    };
    let step = match settings.get("step") {
        None => 5,
        Some(Value::Integer(step)) if (1..=25).contains(step) => *step as u8,
        Some(_) => return Err(crate::descriptions::setting_problem(ID, "step")),
    };
    Ok(Change { operation, step })
}

/// Change the volume. Up and Down clear mute. Mute switches the mute.
pub(crate) fn perform(change: &Change, work: &Work<'_>) -> Result<(), String> {
    let volume = work
        .system
        .volume()
        .map_err(|error| format!("Cannot read the sound volume: {error}."))?;
    let result = if change.operation == Operation::Mute {
        work.system.set_muted(!volume.muted)
    } else {
        let level = if change.operation == Operation::Up {
            volume.level.saturating_add(change.step).min(100)
        } else {
            volume.level.saturating_sub(change.step)
        };
        work.system.set_volume_level(level).and_then(|()| {
            if volume.muted {
                work.system.set_muted(false)
            } else {
                Ok(())
            }
        })
    };
    result.map_err(|error| format!("Cannot set the sound volume: {error}."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptions::setting_problem;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::{values, Work};
    use opencraft_engine::Settings;

    fn run(change: Change, system: &System) -> Result<(), String> {
        let sender = None;
        perform(&change, &Work::new(system, &sender))
    }

    #[test]
    fn the_description_is_the_knob_form_with_up_and_a_step_of_five() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_MEDIA);
        assert_eq!(description.icon.as_deref(), Some("volume-2"));
        assert_eq!(description.help.as_deref(), Some("Knob"));
        assert!(!description.needs_accessibility);
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(description.fields.len(), 2);
        assert_eq!(description.fields[0].key, "operation");
        assert_eq!(
            description.fields[0].default,
            Some(Value::Text("up".into()))
        );
        assert_eq!(description.fields[1].key, "step");
        assert_eq!(description.fields[1].default, Some(Value::Integer(5)));
        let direction = description.direction.expect("a direction");
        assert_eq!(
            (
                direction.key.as_str(),
                direction.right.as_str(),
                direction.left.as_str()
            ),
            ("operation", "up", "down")
        );
    }

    #[test]
    fn read_uses_up_and_five_and_rejects_a_bad_operation_or_step() {
        assert_eq!(
            read(&Settings::new()).unwrap(),
            Change {
                operation: Operation::Up,
                step: 5,
            }
        );
        assert_eq!(
            read(&values(&[
                ("operation", Value::Text("down".into())),
                ("step", Value::Integer(25)),
            ]))
            .unwrap(),
            Change {
                operation: Operation::Down,
                step: 25,
            }
        );
        assert_eq!(
            read(&values(&[("operation", Value::Text("mute".into()))])).unwrap(),
            Change {
                operation: Operation::Mute,
                step: 5,
            }
        );
        assert_eq!(
            read(&values(&[("operation", Value::Text("sideways".into()))])).unwrap_err(),
            setting_problem(ID, "operation")
        );
        assert_eq!(
            read(&values(&[("operation", Value::Integer(1))])).unwrap_err(),
            setting_problem(ID, "operation")
        );
        for step in [
            Value::Integer(0),
            Value::Integer(26),
            Value::Text("5".into()),
        ] {
            assert_eq!(
                read(&values(&[("step", step)])).unwrap_err(),
                setting_problem(ID, "step")
            );
        }
    }

    #[test]
    fn a_bad_read_does_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(
                &values(&[("operation", Value::Text("sideways".into()))]),
                &Work::new(&system, &sender),
            )
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "operation"));
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_clamps_clears_mute_and_switches_mute() {
        let system = System::default();
        system.set_volume(98, true);
        run(
            Change {
                operation: Operation::Up,
                step: 25,
            },
            &system,
        )
        .unwrap();
        system.set_volume(2, true);
        run(
            Change {
                operation: Operation::Down,
                step: 5,
            },
            &system,
        )
        .unwrap();
        system.set_volume(61, false);
        run(
            Change {
                operation: Operation::Mute,
                step: 5,
            },
            &system,
        )
        .unwrap();
        run(
            Change {
                operation: Operation::Mute,
                step: 5,
            },
            &system,
        )
        .unwrap();
        assert_eq!(
            system.calls(),
            [
                Call::Level(100),
                Call::Muted(false),
                Call::Level(0),
                Call::Muted(false),
                Call::Muted(true),
                Call::Muted(false),
            ]
        );
    }

    #[test]
    fn perform_reports_a_read_error_and_a_write_error() {
        let system = System::default();
        system.fail(Fault::Code("volume"));
        assert_eq!(
            run(
                Change {
                    operation: Operation::Up,
                    step: 5,
                },
                &system,
            )
            .unwrap_err(),
            "Cannot read the sound volume: the system refused the operation (code 42)."
        );
        assert!(system.calls().is_empty());
        system.fail(Fault::Code("level"));
        assert_eq!(
            run(
                Change {
                    operation: Operation::Up,
                    step: 5,
                },
                &system,
            )
            .unwrap_err(),
            "Cannot set the sound volume: the system refused the operation (code 42)."
        );
        system.fail(Fault::Code("muted"));
        assert_eq!(
            run(
                Change {
                    operation: Operation::Mute,
                    step: 5,
                },
                &system,
            )
            .unwrap_err(),
            "Cannot set the sound volume: the system refused the operation (code 42)."
        );
    }
}
