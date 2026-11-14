//! The action "Sound output".
//!
//! A missing operation is Set. Set selects one output by its stable UID.
//! Next cycles the outputs of the list "Cycle list" in order. An empty list
//! cycles all outputs. Next skips a saved UID that is not connected. Each
//! operation reads only its own field. A stored value in the other field
//! does not make the action fail (decision 0038). The output handler keeps
//! the bindings, the listener, and the feedback. This module reads the
//! settings, lists the choices, and changes the output.

use crate::descriptions::{choice, field, field_with_default, setting_problem, GROUP_MEDIA};
use crate::kinds::{Choices, Kind, Work};
use opencraft_engine::{ChoiceOption, InputType, KindDescription, Settings, Value};

/// The kind name.
pub(crate) const ID: &str = "opencraft.sound-output";

const NAME: &str = "Sound output";

/// The registry value for this kind.
pub(crate) static KIND: SoundOutputKind = SoundOutputKind;

pub(crate) struct SoundOutputKind;

impl Kind for SoundOutputKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(&read(settings)?, work)
    }

    fn choices(&self, field: &str, work: &Work<'_>) -> Option<Choices> {
        choices(field, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_MEDIA.into(),
        icon: Some("speaker".into()),
        fields: vec![
            field_with_default(
                "operation",
                "Operation",
                choice(&[("set", "Set device"), ("next", "Next device")]),
                Value::Text("set".into()),
            ),
            field("device", "Device", InputType::RuntimeChoice),
            field_with_default(
                "devices",
                "Cycle list",
                InputType::RuntimeChoiceList,
                Value::List(Vec::new()),
            ),
        ],
        ..KindDescription::default()
    }
}

/// One change of the sound output, after a successful read of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Change {
    /// Select this output.
    Set { device: String },
    /// Select the next output of this list. An empty list means all outputs.
    Next { devices: Vec<String> },
}

/// Read the settings. A missing operation is Set. The error is the text for
/// the user.
pub(crate) fn read(settings: &Settings) -> Result<Change, String> {
    let operation = match settings.get("operation") {
        None => "set",
        Some(Value::Text(value)) => value.as_str(),
        Some(_) => return Err(setting_problem(ID, "operation")),
    };
    match operation {
        "set" => match settings.get("device") {
            Some(Value::Text(device)) if !device.is_empty() => Ok(Change::Set {
                device: device.clone(),
            }),
            _ => Err(setting_problem(ID, "device")),
        },
        "next" => {
            let devices = match settings.get("devices") {
                None => Vec::new(),
                Some(Value::List(values)) => values
                    .iter()
                    .map(|value| match value {
                        Value::Text(device) => Ok(device.clone()),
                        _ => Err(setting_problem(ID, "devices")),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                Some(_) => return Err(setting_problem(ID, "devices")),
            };
            Ok(Change::Next { devices })
        }
        _ => Err(setting_problem(ID, "operation")),
    }
}

/// Select the output. A saved UID that is not connected is a problem for Set.
/// Next keeps the saved UIDs, but skips the UIDs that are not connected.
pub(crate) fn perform(change: &Change, work: &Work<'_>) -> Result<(), String> {
    let devices = work.system.output_devices().map_err(list_problem)?;
    let id = match change {
        Change::Set { device } => {
            if !devices.iter().any(|d| d.id == *device) {
                return Err("The selected sound output is not available.".into());
            }
            device.clone()
        }
        Change::Next { devices: selected } => {
            let ids: Vec<_> = if selected.is_empty() {
                devices.iter().map(|d| d.id.clone()).collect()
            } else {
                selected
                    .iter()
                    .filter(|id| devices.iter().any(|d| d.id == **id))
                    .cloned()
                    .collect()
            };
            if ids.is_empty() {
                return Err("No selected sound output is available.".into());
            }
            let current = work.system.default_output().map_err(read_problem)?;
            let next = ids
                .iter()
                .position(|id| *id == current.id)
                .map_or(0, |index| (index + 1) % ids.len());
            ids[next].clone()
        }
    };
    work.system
        .set_output(&id)
        .map_err(|error| format!("Cannot set the sound output: {error}."))
}

/// The outputs for the fields "Device" and "Cycle list". Other fields have no
/// runtime choices. A failed list gives no option and a problem.
pub(crate) fn choices(field: &str, work: &Work<'_>) -> Option<Choices> {
    if !["device", "devices"].contains(&field) {
        return None;
    }
    Some(match work.system.output_devices() {
        Ok(devices) => Choices {
            options: devices
                .into_iter()
                .map(|device| ChoiceOption {
                    value: device.id,
                    label: device.name,
                })
                .collect(),
            problem: None,
        },
        Err(error) => Choices {
            options: Vec::new(),
            problem: Some(list_problem(error)),
        },
    })
}

/// The text of a failed list of the outputs.
pub(crate) fn list_problem(error: impl std::fmt::Display) -> String {
    format!("Cannot list the sound outputs: {error}.")
}

/// The text of a failed read of the default output.
pub(crate) fn read_problem(error: impl std::fmt::Display) -> String {
    format!("Cannot read the sound output: {error}.")
}

/// The text of a failed follow of the changes of the default output.
pub(crate) fn follow_problem(error: impl std::fmt::Display) -> String {
    format!("Cannot follow the sound output: {error}.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::values;

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    fn list(values: &[&str]) -> Value {
        Value::List(values.iter().map(|value| text(value)).collect())
    }

    fn run(settings: &Settings, system: &System) -> Result<(), String> {
        let sender = None;
        KIND.invoke(settings, &Work::new(system, &sender))
    }

    fn next(devices: &[&str]) -> Settings {
        values(&[("operation", text("next")), ("devices", list(devices))])
    }

    #[test]
    fn the_description_is_the_form_with_set_as_the_default_operation() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_MEDIA);
        assert_eq!(description.icon.as_deref(), Some("speaker"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        let fields: Vec<_> = description
            .fields
            .iter()
            .map(|f| (f.key.as_str(), f.label.as_str(), f.default.clone()))
            .collect();
        assert_eq!(
            fields,
            [
                ("operation", "Operation", Some(text("set"))),
                ("device", "Device", None),
                ("devices", "Cycle list", Some(Value::List(Vec::new()))),
            ]
        );
        assert_eq!(description.fields[1].input, InputType::RuntimeChoice);
        assert_eq!(description.fields[2].input, InputType::RuntimeChoiceList);
        assert!(description.direction.is_none());
        assert!(description.shown_when.is_empty());
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn read_uses_set_and_reads_only_the_field_of_the_operation() {
        assert_eq!(
            read(&values(&[("device", text("b"))])).unwrap(),
            Change::Set { device: "b".into() }
        );
        // A bad value of the other field is ignored.
        assert_eq!(
            read(&values(&[
                ("device", text("b")),
                ("devices", Value::Boolean(true)),
            ]))
            .unwrap(),
            Change::Set { device: "b".into() }
        );
        assert_eq!(
            read(&values(&[
                ("operation", text("next")),
                ("device", Value::Integer(1)),
            ]))
            .unwrap(),
            Change::Next {
                devices: Vec::new()
            }
        );
        assert_eq!(
            read(&next(&["b", "a"])).unwrap(),
            Change::Next {
                devices: vec!["b".into(), "a".into()]
            }
        );
    }

    #[test]
    fn read_rejects_a_bad_operation_device_or_list() {
        for (settings, key) in [
            (Settings::new(), "device"),
            (values(&[("device", text(""))]), "device"),
            (values(&[("device", Value::Integer(1))]), "device"),
            (values(&[("operation", text("bad"))]), "operation"),
            (values(&[("operation", Value::Integer(1))]), "operation"),
            (
                values(&[
                    ("operation", text("next")),
                    ("devices", Value::Boolean(true)),
                ]),
                "devices",
            ),
            (
                values(&[
                    ("operation", text("next")),
                    ("devices", Value::List(vec![Value::Integer(1)])),
                ]),
                "devices",
            ),
        ] {
            assert_eq!(read(&settings).unwrap_err(), setting_problem(ID, key));
        }
    }

    #[test]
    fn a_bad_read_does_no_work() {
        let system = System::default();
        assert_eq!(
            run(&Settings::new(), &system).unwrap_err(),
            setting_problem(ID, "device")
        );
        assert!(system.calls().is_empty());
    }

    #[test]
    fn set_selects_a_connected_output_and_refuses_a_missing_one() {
        let system = System::default();
        run(&values(&[("device", text("b"))]), &system).unwrap();
        assert_eq!(system.calls(), [Call::Output("b".into())]);
        assert_eq!(system.output(), "b");
        assert_eq!(
            run(&values(&[("device", text("missing"))]), &system).unwrap_err(),
            "The selected sound output is not available."
        );
        assert_eq!(system.calls().len(), 1);
    }

    #[test]
    fn next_cycles_in_order_skips_a_missing_output_and_wraps() {
        let system = System::default();
        let settings = next(&["b", "missing", "a"]);
        // The default output is "a". It is the last connected one in the list.
        run(&settings, &system).unwrap();
        run(&settings, &system).unwrap();
        run(&settings, &system).unwrap();
        // An empty list cycles all outputs.
        run(&next(&[]), &system).unwrap();
        assert_eq!(
            system.calls(),
            [
                Call::Output("b".into()),
                Call::Output("a".into()),
                Call::Output("b".into()),
                Call::Output("a".into()),
            ]
        );
    }

    #[test]
    fn next_starts_at_the_first_output_when_the_default_is_not_in_the_list() {
        let system = System::default();
        system.select_output("b");
        run(&next(&["a", "missing"]), &system).unwrap();
        assert_eq!(system.calls(), [Call::Output("a".into())]);
    }

    #[test]
    fn next_with_no_connected_output_is_a_problem() {
        let system = System::default();
        assert_eq!(
            run(&next(&["missing"]), &system).unwrap_err(),
            "No selected sound output is available."
        );
        assert!(system.calls().is_empty());
    }

    #[test]
    fn a_failed_list_read_or_write_is_a_problem() {
        let system = System::default();
        system.fail(Fault::Code("outputs"));
        assert_eq!(
            run(&values(&[("device", text("b"))]), &system).unwrap_err(),
            "Cannot list the sound outputs: the system refused the operation (code 42)."
        );
        system.fail(Fault::Code("output"));
        assert_eq!(
            run(&next(&["a", "b"]), &system).unwrap_err(),
            "Cannot read the sound output: the system refused the operation (code 42)."
        );
        system.fail(Fault::Code("set-output"));
        assert_eq!(
            run(&values(&[("device", text("b"))]), &system).unwrap_err(),
            "Cannot set the sound output: the system refused the operation (code 42)."
        );
        // The failed write is recorded, but the default output does not change.
        assert_eq!(system.calls(), [Call::Output("b".into())]);
        assert_eq!(system.output(), "a");
    }

    #[test]
    fn the_choices_are_the_outputs_for_the_two_fields_and_do_not_change_the_system() {
        let system = System::default();
        let sender = None;
        let work = Work::new(&system, &sender);
        for field in ["device", "devices"] {
            let choices = KIND.choices(field, &work).expect(field);
            assert_eq!(choices.problem, None);
            assert_eq!(
                choices
                    .options
                    .iter()
                    .map(|o| (o.value.as_str(), o.label.as_str()))
                    .collect::<Vec<_>>(),
                [("a", "Speakers"), ("b", "Headphones")]
            );
        }
        assert!(KIND.choices("operation", &work).is_none());
        assert!(system.calls().is_empty());
        system.fail(Fault::Code("outputs"));
        let choices = KIND.choices("device", &work).unwrap();
        assert!(choices.options.is_empty());
        assert_eq!(
            choices.problem.as_deref(),
            Some("Cannot list the sound outputs: the system refused the operation (code 42).")
        );
    }
}
