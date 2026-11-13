use super::*;
use crate::mouse::{Button, MouseEvent};

#[test]
fn mouse_operations_are_checked_and_sent_through_the_fake_system() {
    let mut rig = Rig::new("mouse", true);
    rig.invoke(action("mouse", &[]));
    assert_eq!(
        rig.system.calls(),
        [Call::Mouse(MouseEvent::Click {
            button: Button::Left,
            count: 1
        })]
    );
    for (name, button) in [
        ("left", Button::Left),
        ("right", Button::Right),
        ("middle", Button::Middle),
    ] {
        for (operation, count) in [("click", 1), ("double-click", 2)] {
            rig.system.clear_calls();
            rig.invoke(action(
                "mouse",
                &[
                    ("operation", Value::Text(operation.into())),
                    ("button", Value::Text(name.into())),
                ],
            ));
            assert_eq!(
                rig.system.calls(),
                [Call::Mouse(MouseEvent::Click { button, count })]
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
        rig.system.clear_calls();
        rig.invoke(action(
            "mouse",
            &[
                ("operation", Value::Text(operation.into())),
                ("x", Value::Integer(-30)),
                ("y", Value::Integer(40)),
                ("step", Value::Integer(7)),
            ],
        ));
        assert_eq!(rig.system.calls(), [Call::Mouse(expected)]);
    }
}

#[test]
fn mouse_rejects_bad_settings_checks_permission_and_reports_system_errors() {
    for (operation, key, value, label) in [
        ("bad", "step", Value::Integer(1), "Operation"),
        ("click", "button", Value::Text("extra".into()), "Button"),
        ("drag", "x", Value::Integer(100_001), "X"),
        ("move-relative", "y", Value::Text("40".into()), "Y"),
        ("scroll-up", "step", Value::Integer(0), "Scroll step"),
        ("scroll-down", "step", Value::Integer(1001), "Scroll step"),
    ] {
        let mut rig = Rig::new("mouse", true);
        rig.invoke(action(
            "mouse",
            &[("operation", Value::Text(operation.into())), (key, value)],
        ));
        rig.problem(label);
        assert!(rig.system.calls().is_empty());
    }
    let mut rig = Rig::new("mouse", false);
    rig.invoke(action("mouse", &[]));
    statuses(
        (0..crate::descriptions::KEY_KINDS.len())
            .map(|_| rig.recv())
            .collect(),
        false,
    );
    rig.problem("Accessibility");
    assert!(rig.system.calls().is_empty());
    rig.system.permission.store(true, Ordering::SeqCst);
    rig.system.fail(Some("mouse"));
    rig.invoke(action("mouse", &[]));
    statuses(
        (0..crate::descriptions::KEY_KINDS.len())
            .map(|_| rig.recv())
            .collect(),
        true,
    );
    rig.problem("42");
}

#[test]
fn mouse_description_has_the_form_group_and_knob_scroll_directions() {
    let description = crate::descriptions::description(crate::descriptions::MOUSE);
    assert_eq!(description.group, "System");
    assert!(description.needs_accessibility);
    assert!(opencraft_engine::icons::icon_set()
        .get(description.icon.as_deref().unwrap())
        .is_some());
    assert_eq!(
        description
            .fields
            .iter()
            .map(|f| f.key.as_str())
            .collect::<Vec<_>>(),
        ["operation", "button", "x", "y", "step"]
    );
    let direction = description.direction.unwrap();
    assert_eq!(
        (
            direction.key.as_str(),
            direction.left.as_str(),
            direction.right.as_str()
        ),
        ("operation", "scroll-up", "scroll-down")
    );
    let mut rig = Rig::new("mouse", true);
    for operation in [direction.left, direction.right] {
        rig.invoke(action("mouse", &[("operation", Value::Text(operation))]));
    }
    assert_eq!(
        rig.system.calls(),
        [
            Call::Mouse(MouseEvent::Scroll { x: 0, y: 3 }),
            Call::Mouse(MouseEvent::Scroll { x: 0, y: -3 })
        ]
    );
}
