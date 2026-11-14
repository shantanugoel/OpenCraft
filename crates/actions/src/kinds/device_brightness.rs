//! The action "Device brightness".
//!
//! The action asks the engine to change the brightness of the device screen.
//! A missing operation is Up. Up and Down use the field `step`. A missing
//! step is 5. Set uses the field `value`. A missing value is 80. The action
//! reads only the field that the operation uses, and a stored value of a
//! hidden field does not make the action fail (decision 0038). The engine
//! fits the result to the range of the device.

use crate::descriptions::{choice, field_with_default, setting_problem, GROUP_DEVICE};
use crate::kinds::{Kind, Work};
use opencraft_engine::{
    BrightnessChange, Direction, EngineRequest, InputType, KindDescription, Settings, ShownWhen,
    Value,
};

/// The kind name.
pub(crate) const ID: &str = "opencraft.device-brightness";

const NAME: &str = "Device brightness";

/// The registry value for this kind.
pub(crate) static KIND: DeviceBrightnessKind = DeviceBrightnessKind;

pub(crate) struct DeviceBrightnessKind;

impl Kind for DeviceBrightnessKind {
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

/// The description of this kind (decision 0014). A turn to the right goes up,
/// and a turn to the left goes down.
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_DEVICE.into(),
        icon: Some("sun".into()),
        help: Some("Knob".into()),
        fields: vec![
            field_with_default(
                "operation",
                "Operation",
                choice(&[("up", "Up"), ("down", "Down"), ("set", "Set value")]),
                Value::Text("up".into()),
            ),
            field_with_default(
                "value",
                "Brightness",
                InputType::Integer { min: 0, max: 100 },
                Value::Integer(80),
            ),
            field_with_default(
                "step",
                "Step",
                InputType::Integer { min: 1, max: 100 },
                Value::Integer(5),
            ),
        ],
        direction: Some(Direction {
            key: "operation".into(),
            right: "up".into(),
            left: "down".into(),
        }),
        shown_when: vec![
            ShownWhen::new("value", "operation", &["set"]),
            ShownWhen::new("step", "operation", &["up", "down"]),
        ],
        ..KindDescription::default()
    }
}

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<BrightnessChange, String> {
    let integer = |key: &str, default: i64, min: i64| match settings.get(key) {
        None => Ok(default),
        Some(Value::Integer(n)) if (min..=100).contains(n) => Ok(*n),
        _ => Err(setting_problem(ID, key)),
    };
    let operation = match settings.get("operation") {
        None => "up",
        Some(Value::Text(value)) => value.as_str(),
        _ => return Err(setting_problem(ID, "operation")),
    };
    match operation {
        "set" => Ok(BrightnessChange::Set(integer("value", 80, 0)? as u8)),
        "up" | "down" => {
            let step = integer("step", 5, 1)? as i16;
            Ok(BrightnessChange::Step(if operation == "down" {
                -step
            } else {
                step
            }))
        }
        _ => Err(setting_problem(ID, "operation")),
    }
}

/// Ask the engine to change the brightness.
pub(crate) fn perform(change: BrightnessChange, work: &Work<'_>) {
    work.ask(EngineRequest::DeviceBrightness(change));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::recording::System;
    use crate::kinds::values;
    use opencraft_engine::Message;
    use std::sync::mpsc;

    fn operation(value: &str) -> (&'static str, Value) {
        ("operation", Value::Text(value.into()))
    }

    #[test]
    fn the_description_has_the_knob_form_and_one_rule_for_each_hidden_field() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_DEVICE);
        assert_eq!(description.icon.as_deref(), Some("sun"));
        assert_eq!(description.help.as_deref(), Some("Knob"));
        assert!(!description.needs_accessibility);
        assert!(description.states.is_none());
        assert!(!description.folder_back);
        let keys: Vec<_> = description.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, ["operation", "value", "step"]);
        assert_eq!(
            description.fields[0].default,
            Some(Value::Text("up".into()))
        );
        assert_eq!(description.fields[1].default, Some(Value::Integer(80)));
        assert_eq!(description.fields[2].default, Some(Value::Integer(5)));
        let direction = description.direction.clone().expect("a direction");
        assert_eq!(
            (
                direction.key.as_str(),
                direction.right.as_str(),
                direction.left.as_str()
            ),
            ("operation", "up", "down")
        );
        assert_eq!(
            description.shown_when,
            vec![
                ShownWhen::new("value", "operation", &["set"]),
                ShownWhen::new("step", "operation", &["up", "down"]),
            ]
        );
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn read_uses_up_and_a_step_of_five_and_the_set_value() {
        assert_eq!(read(&Settings::new()).unwrap(), BrightnessChange::Step(5));
        for (name, expected) in [
            ("up", BrightnessChange::Step(7)),
            ("down", BrightnessChange::Step(-7)),
            ("set", BrightnessChange::Set(35)),
        ] {
            let settings = values(&[
                operation(name),
                ("step", Value::Integer(7)),
                ("value", Value::Integer(35)),
            ]);
            assert_eq!(read(&settings).unwrap(), expected);
        }
        assert_eq!(
            read(&values(&[operation("set")])).unwrap(),
            BrightnessChange::Set(80)
        );
        assert_eq!(
            read(&values(&[operation("set"), ("value", Value::Integer(0))])).unwrap(),
            BrightnessChange::Set(0)
        );
    }

    #[test]
    fn read_ignores_the_field_that_the_operation_does_not_use() {
        let bad = Value::Text("bad".into());
        assert_eq!(
            read(&values(&[operation("set"), ("step", bad.clone())])).unwrap(),
            BrightnessChange::Set(80)
        );
        for name in ["up", "down"] {
            assert!(read(&values(&[operation(name), ("value", bad.clone())])).is_ok());
        }
    }

    #[test]
    fn read_rejects_a_bad_operation_step_or_value() {
        let operation_problem = "Device brightness: select a value in the field \"Operation\".";
        let step_problem = "Device brightness: set the field \"Step\" to a number from 1 to 100.";
        let value_problem =
            "Device brightness: set the field \"Brightness\" to a number from 0 to 100.";
        for (pairs, expected) in [
            (vec![operation("bad")], operation_problem),
            (vec![("operation", Value::Integer(1))], operation_problem),
            (vec![("step", Value::Integer(0))], step_problem),
            (vec![("step", Value::Integer(101))], step_problem),
            (
                vec![operation("down"), ("step", Value::Integer(0))],
                step_problem,
            ),
            (
                vec![operation("set"), ("value", Value::Integer(101))],
                value_problem,
            ),
            (
                vec![operation("set"), ("value", Value::Integer(-1))],
                value_problem,
            ),
        ] {
            assert_eq!(read(&values(&pairs)).unwrap_err(), expected);
        }
    }

    #[test]
    fn a_push_asks_the_engine_and_a_bad_setting_asks_for_nothing() {
        let system = System::default();
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        let work = Work::new(&system, &sender);
        KIND.invoke(
            &values(&[operation("down"), ("step", Value::Integer(10))]),
            &work,
        )
        .unwrap();
        assert_eq!(
            messages.try_recv().unwrap(),
            Message::Engine(EngineRequest::DeviceBrightness(BrightnessChange::Step(-10)))
        );
        KIND.invoke(&values(&[operation("bad")]), &work)
            .unwrap_err();
        assert!(messages.try_recv().is_err());
        assert!(system.calls().is_empty());
    }
}
