//! The knob inspector (decision 0029, section 10.4): the "Turn" card with
//! the direction of the kind, the cards apart, and the section of the bar
//! panel. The kind is a fake kind with the direction description of "Sound
//! volume".

use super::actions::{top, type_at};
use super::*;
use opencraft_engine::Direction;

const VOLUME: &str = "test.volume";

fn volume() -> KindDescription {
    let option = |value: &str, label: &str| ChoiceOption {
        value: value.into(),
        label: label.into(),
    };
    KindDescription {
        kind: VOLUME.into(),
        name: "Sound volume".into(),
        group: "Media".into(),
        fields: vec![
            Field {
                key: "operation".into(),
                label: "Operation".into(),
                input: InputType::Choice {
                    options: vec![
                        option("up", "Up"),
                        option("down", "Down"),
                        option("mute", "Mute"),
                    ],
                },
                default: Some(Value::Text("up".into())),
            },
            Field {
                key: "step".into(),
                label: "Step".into(),
                input: InputType::Integer { min: 1, max: 25 },
                default: Some(Value::Integer(5)),
            },
        ],
        direction: Some(Direction {
            key: "operation".into(),
            right: "up".into(),
            left: "down".into(),
        }),
        ..Default::default()
    }
}

fn knob() -> ControlId {
    ControlId::new(ControlKind::Knob, 2)
}

fn turn(gesture: GestureKind) -> BindingId {
    BindingId {
        page: 0,
        control: knob(),
        gesture,
        folder: None,
    }
}

fn sound(operation: &str, step: i64) -> Action {
    Action {
        kind: VOLUME.into(),
        settings: Settings::from([
            ("operation".into(), Value::Text(operation.into())),
            ("step".into(), Value::Integer(step)),
        ]),
    }
}

/// The two turn bindings of the last command, as a group of two
/// `SetBinding`: the action of "Turn right" and of "Turn left".
fn last_turn(sent: &Commands) -> (Option<Action>, Option<Action>) {
    let Some(Command::Group { commands, .. }) = sent.borrow().last().cloned() else {
        panic!("expected a group, got {:?}", sent.borrow().last())
    };
    match commands.as_slice() {
        [Command::SetBinding {
            binding: right,
            action: r,
        }, Command::SetBinding {
            binding: left,
            action: l,
        }] => {
            assert_eq!(*right, turn(GestureKind::TurnRight));
            assert_eq!(*left, turn(GestureKind::TurnLeft));
            (r.clone(), l.clone())
        }
        other => panic!("expected two bindings, got {other:?}"),
    }
}

/// The engine replies to the commands: the bindings of the knob.
fn reply(tx: &async_channel::Sender<Event>, right: Option<Action>, left: Option<Action>) {
    for (gesture, action) in [
        (GestureKind::TurnRight, right),
        (GestureKind::TurnLeft, left),
    ] {
        tx.try_send(Event::Binding(BindingState {
            binding: turn(gesture),
            available: action.is_some(),
            action,
        }))
        .unwrap();
    }
}

fn knob_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(2400.)));
    tx.try_send(Event::Kinds(vec![volume()])).unwrap();
    cx.run_until_parked();
    click(cx, "control-knob 2");
    (view, cx, tx, sent)
}

fn shows(cx: &mut VisualTestContext, id: &'static str) -> bool {
    cx.debug_bounds(id).is_some()
}

#[gpui_kit::test]
fn sound_volume_on_turn_writes_two_bindings_and_the_direction_reverses(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = knob_window(cx);
    // The header, and the one "Turn" card with no action.
    assert!(shows(cx, "gesture-Turn"));
    assert!(shows(cx, "gesture-Push"));
    assert!(!shows(cx, "gesture-TurnLeft"));
    click(cx, "gesture-Turn-kind");
    click(cx, "kind-0");
    let (right, left) = last_turn(&sent);
    assert_eq!(right, Some(sound("up", 5)));
    assert_eq!(left, Some(sound("down", 5)));
    assert_eq!(
        sent.borrow().len(),
        1,
        "One command: one step of the history"
    );
    reply(&tx, right, left);
    cx.run_until_parked();

    // The open card: the direction, the step, and the link. The direction
    // field has no popup of its own.
    assert!(shows(cx, "gesture-Turn-head"));
    assert!(shows(cx, "turn-direction-right"));
    assert!(shows(cx, "turn-direction-left"));
    assert!(!shows(cx, "choice-0"));
    assert!(shows(cx, "field-1"));
    assert!(shows(cx, "turn-apart"));

    // "Left is up" reverses the two values.
    click(cx, "turn-direction-left");
    let (right, left) = last_turn(&sent);
    assert_eq!(right, Some(sound("down", 5)));
    assert_eq!(left, Some(sound("up", 5)));
    reply(&tx, right, left);
    cx.run_until_parked();
    // The card stays one card, and a click on the selected segment does
    // nothing.
    assert!(shows(cx, "gesture-Turn"));
    let count = sent.borrow().len();
    click(cx, "turn-direction-left");
    assert_eq!(sent.borrow().len(), count);
}

#[gpui_kit::test]
fn a_setting_of_turn_changes_the_two_bindings(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = knob_window(cx);
    reply(&tx, Some(sound("down", 5)), Some(sound("up", 5)));
    cx.run_until_parked();
    // The first card with an action is open: the "Turn" card.
    assert!(shows(cx, "turn-direction-left"));
    type_at(&view, cx, &top("step"), "12");
    let (right, left) = last_turn(&sent);
    assert_eq!(
        right.as_ref().map(|a| a.settings["step"].clone()),
        Some(Value::Integer(12))
    );
    // Each binding keeps its own direction value.
    assert_eq!(right, Some(sound("down", 12)));
    assert_eq!(left, Some(sound("up", 12)));
    // "Remove the action" removes the two actions in one group.
    click(cx, "gesture-Turn-kind");
    click(cx, "kind-remove");
    let Some(Command::Group { name, commands }) = sent.borrow().last().cloned() else {
        panic!()
    };
    assert_eq!(name, "Remove the Action");
    assert!(commands
        .iter()
        .all(|c| matches!(c, Command::SetBinding { action: None, .. })));
    assert_eq!(commands.len(), 2);
}

#[gpui_kit::test]
fn left_and_right_apart_show_two_cards_and_join_again(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = knob_window(cx);
    reply(&tx, Some(sound("up", 5)), Some(sound("down", 5)));
    cx.run_until_parked();
    click(cx, "turn-apart");
    assert!(sent.borrow().is_empty(), "The bindings do not change");
    assert!(shows(cx, "gesture-TurnLeft"));
    assert!(shows(cx, "gesture-TurnRight"));
    assert!(!shows(cx, "gesture-Turn"));
    // The open card is a turn card with the link to join.
    assert!(shows(cx, "turn-join"));
    // "Turn left" stays open (the first card with an action). A change of
    // one card changes only its binding.
    click(cx, "choice-0");
    click(cx, "choice-0-2");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: turn(GestureKind::TurnLeft),
            action: Some(sound("mute", 5)),
        })
    );
    // The bindings are not one pair now: two cards, and no link to join.
    reply(&tx, Some(sound("up", 5)), Some(sound("mute", 5)));
    cx.run_until_parked();
    assert!(!shows(cx, "turn-join"));
    // Back to one pair: the link joins the two cards again.
    reply(&tx, Some(sound("up", 5)), Some(sound("down", 5)));
    cx.run_until_parked();
    click(cx, "turn-join");
    assert!(shows(cx, "gesture-Turn"));
    assert!(!shows(cx, "gesture-TurnLeft"));
}

#[gpui_kit::test]
fn two_different_bindings_show_two_cards(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = knob_window(cx);
    reply(&tx, Some(sound("up", 5)), Some(sound("down", 7)));
    cx.run_until_parked();
    assert!(shows(cx, "gesture-TurnLeft"));
    assert!(shows(cx, "gesture-TurnRight"));
    assert!(!shows(cx, "gesture-Turn"));
    assert!(!shows(cx, "turn-join"));
    // Only one turn binding: two cards also.
    reply(&tx, Some(sound("up", 5)), None);
    cx.run_until_parked();
    assert!(shows(cx, "gesture-TurnLeft"));
    assert!(!shows(cx, "gesture-Turn"));
}

#[gpui_kit::test]
fn the_bar_panel_section_edits_the_bar_panel_of_the_feedback(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = knob_window(cx);
    // The knob has no frame of a surface, but the section edits bar panel 2.
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, None));
    assert!(shows(cx, "knob-panel"));
    assert!(shows(cx, "knob-panel-face"));
    let face = cx.debug_bounds("knob-panel-face").unwrap();
    assert_eq!(face.size.width, px(148.));
    // The test layout has a bar panel of 180 × 60.
    // GPUI puts the edges on half points.
    assert!((f32::from(face.size.height) - 148. / 3.).abs() <= 0.5);
    cx.update(|_, cx| assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "Sound"));
    click(cx, "colour-3");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetColor {
            surface: bar(),
            color: Some(colour(COLOURS[3])),
        })
    );
    click(cx, "label-position-top");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetLabelStyle {
            surface: bar(),
            change: opencraft_engine::LabelChange::Position(Some(
                opencraft_engine::LabelPosition::Top
            )),
        })
    );
    // The title field writes the label of the bar panel.
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        window.focus(&input.focus_handle(cx), cx);
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
    });
    cx.run_until_parked();
    cx.simulate_input("Vol");
    cx.run_until_parked();
    assert!(sent.borrow().contains(&Command::SetLabel {
        surface: bar(),
        label: Some("Vol".into()),
    }));
    // The "Tap" card sets the tap of bar panel 2.
    click(cx, "gesture-Tap-kind");
    click(cx, "kind-0");
    assert!(matches!(
        sent.borrow().last(),
        Some(Command::SetBinding {
            binding: BindingId {
                gesture: GestureKind::Tap,
                control: ControlId {
                    kind: ControlKind::BarPanel,
                    number: 2,
                },
                page: 0,
                folder: None,
            },
            action: Some(_),
        })
    ));
    cx.update(|_, cx| {
        let state = &view.read(cx).state;
        assert_eq!(state.gesture, Some(GestureKind::Tap));
        assert_eq!(
            state.binding_id().map(|b| b.control),
            Some(ControlId::new(ControlKind::BarPanel, 2))
        );
    });
}
