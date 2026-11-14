//! The action "Mouse event".
//!
//! A missing field has its default. The work is one complete event, so no
//! operation leaves a button held. The work needs the Accessibility permission.
//! A turn of a knob scrolls down to the right and up to the left.

use crate::descriptions::{choice, field_with_default, GROUP_SYSTEM};
use crate::kinds::{Kind, Work};
use crate::mouse::{Button, MouseEvent};
use opencraft_engine::{Direction, KindDescription, Settings, ShownWhen, Value};

/// The kind name.
pub(crate) const ID: &str = "opencraft.mouse";

const NAME: &str = "Mouse event";

/// The registry value for this kind.
pub(crate) static KIND: MouseKind = MouseKind;

pub(crate) struct MouseKind;

impl Kind for MouseKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        let event = read(settings)?;
        if !work.keys_allowed() {
            return Err(crate::handlers::no_permission());
        }
        perform(event, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_SYSTEM.into(),
        icon: Some("mouse-pointer-2".into()),
        help: Some("Accessibility".into()),
        needs_accessibility: true,
        fields: vec![
            field_with_default(
                "operation",
                "Operation",
                choice(&[
                    ("click", "Click"),
                    ("double-click", "Double click"),
                    ("move", "Move to position"),
                    ("move-relative", "Move by offset"),
                    ("scroll-up", "Scroll up"),
                    ("scroll-down", "Scroll down"),
                    ("scroll-left", "Scroll left"),
                    ("scroll-right", "Scroll right"),
                    ("drag", "Drag to position"),
                ]),
                Value::Text("click".into()),
            ),
            field_with_default(
                "button",
                "Button",
                choice(&[("left", "Left"), ("right", "Right"), ("middle", "Middle")]),
                Value::Text("left".into()),
            ),
            field_with_default(
                "x",
                "X",
                opencraft_engine::InputType::Integer {
                    min: -100_000,
                    max: 100_000,
                },
                Value::Integer(0),
            ),
            field_with_default(
                "y",
                "Y",
                opencraft_engine::InputType::Integer {
                    min: -100_000,
                    max: 100_000,
                },
                Value::Integer(0),
            ),
            field_with_default(
                "step",
                "Scroll step",
                opencraft_engine::InputType::Integer { min: 1, max: 1000 },
                Value::Integer(3),
            ),
        ],
        direction: Some(Direction {
            key: "operation".into(),
            right: "scroll-down".into(),
            left: "scroll-up".into(),
        }),
        shown_when: vec![
            ShownWhen::new("button", "operation", &["click", "double-click", "drag"]),
            ShownWhen::new("x", "operation", &["move", "move-relative", "drag"]),
            ShownWhen::new("y", "operation", &["move", "move-relative", "drag"]),
            ShownWhen::new(
                "step",
                "operation",
                &["scroll-up", "scroll-down", "scroll-left", "scroll-right"],
            ),
        ],
        ..KindDescription::default()
    }
}

fn text<'a>(settings: &'a Settings, key: &str, default: &'a str) -> Result<&'a str, String> {
    match settings.get(key) {
        None => Ok(default),
        Some(Value::Text(value)) => Ok(value),
        _ => Err(crate::descriptions::setting_problem(ID, key)),
    }
}

fn number(settings: &Settings, key: &str, default: i32, min: i32, max: i32) -> Result<i32, String> {
    match settings.get(key) {
        None => Ok(default),
        Some(Value::Integer(value)) if (i64::from(min)..=i64::from(max)).contains(value) => {
            Ok(*value as i32)
        }
        _ => Err(crate::descriptions::setting_problem(ID, key)),
    }
}

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<MouseEvent, String> {
    let button = || match text(settings, "button", "left")? {
        "left" => Ok(Button::Left),
        "right" => Ok(Button::Right),
        "middle" => Ok(Button::Middle),
        _ => Err(crate::descriptions::setting_problem(ID, "button")),
    };
    let point = || -> Result<(i32, i32), String> {
        Ok((
            number(settings, "x", 0, -100_000, 100_000)?,
            number(settings, "y", 0, -100_000, 100_000)?,
        ))
    };
    Ok(match text(settings, "operation", "click")? {
        operation @ ("click" | "double-click") => MouseEvent::Click {
            button: button()?,
            count: if operation == "click" { 1 } else { 2 },
        },
        operation @ ("move" | "move-relative") => {
            let (x, y) = point()?;
            MouseEvent::Move {
                x,
                y,
                relative: operation == "move-relative",
            }
        }
        "drag" => {
            let (x, y) = point()?;
            MouseEvent::Drag {
                button: button()?,
                x,
                y,
            }
        }
        operation @ ("scroll-up" | "scroll-down" | "scroll-left" | "scroll-right") => {
            let step = number(settings, "step", 3, 1, 1000)?;
            let (x, y) = match operation {
                "scroll-up" => (0, step),
                "scroll-down" => (0, -step),
                "scroll-left" => (step, 0),
                _ => (-step, 0),
            };
            MouseEvent::Scroll { x, y }
        }
        _ => return Err(crate::descriptions::setting_problem(ID, "operation")),
    })
}

/// Send one complete mouse event.
pub(crate) fn perform(event: MouseEvent, work: &Work<'_>) -> Result<(), String> {
    work.system
        .mouse_event(event)
        .map_err(|error| work.input_error(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptions::setting_problem;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::{values, Work};
    use opencraft_engine::Settings;

    fn run(event: MouseEvent, system: &System) -> Result<(), String> {
        let sender = None;
        perform(event, &Work::new(system, &sender))
    }

    #[test]
    fn the_description_has_the_form_and_the_knob_scroll_directions() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_SYSTEM);
        assert_eq!(description.icon.as_deref(), Some("mouse-pointer-2"));
        assert_eq!(description.help.as_deref(), Some("Accessibility"));
        assert!(description.needs_accessibility);
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(
            description
                .fields
                .iter()
                .map(|field| field.key.as_str())
                .collect::<Vec<_>>(),
            ["operation", "button", "x", "y", "step"]
        );
        assert_eq!(
            description.shown_when,
            vec![
                ShownWhen::new("button", "operation", &["click", "double-click", "drag"]),
                ShownWhen::new("x", "operation", &["move", "move-relative", "drag"]),
                ShownWhen::new("y", "operation", &["move", "move-relative", "drag"]),
                ShownWhen::new(
                    "step",
                    "operation",
                    &["scroll-up", "scroll-down", "scroll-left", "scroll-right"],
                ),
            ]
        );
        assert_eq!(
            description.fields[0].default,
            Some(Value::Text("click".into()))
        );
        assert_eq!(
            description.fields[1].default,
            Some(Value::Text("left".into()))
        );
        assert_eq!(description.fields[2].default, Some(Value::Integer(0)));
        assert_eq!(description.fields[4].default, Some(Value::Integer(3)));
        let direction = description.direction.expect("a direction");
        assert_eq!(
            (
                direction.key.as_str(),
                direction.left.as_str(),
                direction.right.as_str()
            ),
            ("operation", "scroll-up", "scroll-down")
        );
    }

    #[test]
    fn read_uses_the_defaults_and_builds_each_operation() {
        assert_eq!(
            read(&Settings::new()).unwrap(),
            MouseEvent::Click {
                button: Button::Left,
                count: 1,
            }
        );
        for (name, button) in [
            ("left", Button::Left),
            ("right", Button::Right),
            ("middle", Button::Middle),
        ] {
            for (operation, count) in [("click", 1), ("double-click", 2)] {
                assert_eq!(
                    read(&values(&[
                        ("operation", Value::Text(operation.into())),
                        ("button", Value::Text(name.into())),
                    ]))
                    .unwrap(),
                    MouseEvent::Click { button, count }
                );
            }
        }
        for (operation, expected) in [
            (
                "move",
                MouseEvent::Move {
                    x: -30,
                    y: 40,
                    relative: false,
                },
            ),
            (
                "move-relative",
                MouseEvent::Move {
                    x: -30,
                    y: 40,
                    relative: true,
                },
            ),
            (
                "drag",
                MouseEvent::Drag {
                    button: Button::Left,
                    x: -30,
                    y: 40,
                },
            ),
            ("scroll-up", MouseEvent::Scroll { x: 0, y: 7 }),
            ("scroll-down", MouseEvent::Scroll { x: 0, y: -7 }),
            ("scroll-left", MouseEvent::Scroll { x: 7, y: 0 }),
            ("scroll-right", MouseEvent::Scroll { x: -7, y: 0 }),
        ] {
            let pairs = vec![
                ("operation", Value::Text(operation.into())),
                ("x", Value::Integer(-30)),
                ("y", Value::Integer(40)),
                ("step", Value::Integer(7)),
            ];
            assert_eq!(read(&values(&pairs)).unwrap(), expected, "{operation}");
        }
    }

    #[test]
    fn read_rejects_a_bad_operation_button_point_or_step() {
        for (operation, key, value, field) in [
            ("bad", "step", Value::Integer(1), "operation"),
            ("click", "button", Value::Text("extra".into()), "button"),
            ("drag", "x", Value::Integer(100_001), "x"),
            ("move-relative", "y", Value::Text("40".into()), "y"),
            ("scroll-up", "step", Value::Integer(0), "step"),
            ("scroll-down", "step", Value::Integer(1001), "step"),
        ] {
            assert_eq!(
                read(&values(&[
                    ("operation", Value::Text(operation.into())),
                    (key, value),
                ]))
                .unwrap_err(),
                setting_problem(ID, field),
                "{operation}/{key}"
            );
        }
    }

    #[test]
    fn a_bad_read_and_a_missing_permission_do_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(
                &values(&[("operation", Value::Text("bad".into()))]),
                &Work::new(&system, &sender),
            )
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "operation"));
        let allow = || true;
        let blocked = Work::new(&system, &sender).with_keys(false, &allow);
        let error = KIND.invoke(&Settings::new(), &blocked).unwrap_err();
        assert_eq!(error, crate::handlers::no_permission());
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_sends_the_event_and_reports_a_system_error() {
        let system = System::default();
        let event = MouseEvent::Scroll { x: 0, y: 3 };
        run(event, &system).unwrap();
        assert_eq!(system.calls(), [Call::Mouse(event)]);
        system.fail(Fault::Code("mouse"));
        assert_eq!(
            run(event, &system).unwrap_err(),
            "Cannot send keys or mouse events: the system refused the operation (code 42)."
        );
    }
}
