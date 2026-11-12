//! The form of actions that contain actions, with fake kinds.

use super::actions::{at, type_at};
use super::*;

/// A kind with one text field and one choice.
fn leaf() -> KindDescription {
    KindDescription {
        kind: "test.leaf".into(),
        name: "Leaf".into(),
        fields: vec![
            Field {
                key: "text".into(),
                label: "Text".into(),
                input: InputType::Text,
                default: Some(Value::Text("start".into())),
            },
            Field {
                key: "choice".into(),
                label: "Choice".into(),
                input: InputType::Choice {
                    options: vec![
                        ChoiceOption {
                            value: "a".into(),
                            label: "A".into(),
                        },
                        ChoiceOption {
                            value: "b".into(),
                            label: "B".into(),
                        },
                    ],
                },
                default: None,
            },
        ],
    }
}

/// A kind with two actions, as a toggle.
fn two() -> KindDescription {
    KindDescription {
        kind: "test.two".into(),
        name: "Two actions".into(),
        fields: vec![
            Field {
                key: "first".into(),
                label: "First".into(),
                input: InputType::Action,
                default: None,
            },
            Field {
                key: "second".into(),
                label: "Second".into(),
                input: InputType::Action,
                default: None,
            },
        ],
    }
}

/// A kind with a list of actions, as a sequence.
fn many() -> KindDescription {
    KindDescription {
        kind: "test.many".into(),
        name: "Many actions".into(),
        fields: vec![
            Field {
                key: "items".into(),
                label: "Actions".into(),
                input: InputType::ActionList,
                default: None,
            },
            Field {
                key: "delay".into(),
                label: "Delay".into(),
                input: InputType::Integer { min: 0, max: 1000 },
                default: Some(Value::Integer(100)),
            },
        ],
    }
}

fn kinds() -> Event {
    Event::Kinds(vec![leaf(), two(), many()])
}

fn press() -> BindingId {
    BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Key, 2),
        gesture: GestureKind::Press,
    }
}

fn act(kind: &str, settings: Vec<(&str, Value)>) -> Action {
    Action {
        kind: kind.into(),
        settings: settings.into_iter().map(|(k, v)| (k.into(), v)).collect(),
    }
}

fn leaf_action(text: &str) -> Action {
    act("test.leaf", vec![("text", Value::Text(text.into()))])
}

fn sent_action(sent: &Commands) -> Action {
    match sent.borrow().last() {
        Some(Command::SetBinding {
            binding,
            action: Some(action),
        }) if *binding == press() => action.clone(),
        other => panic!("No binding command: {other:?}"),
    }
}

fn open(
    cx: &mut TestAppContext,
    action: Action,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    // A tall test viewport lets the test click each field without scrolling.
    cx.simulate_resize(size(px(1120.), px(3200.)));
    tx.try_send(kinds()).unwrap();
    tx.try_send(Event::Binding(BindingState {
        binding: press(),
        action: Some(action),
        available: true,
    }))
    .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    (view, cx, tx, sent)
}

#[gpui_kit::test]
fn two_contained_actions_write_their_kinds_and_fields(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = open(cx, act("test.two", vec![]));
    assert!(cx.debug_bounds("first/action-kind").is_some());
    assert!(cx.debug_bounds("second/action-kind").is_some());
    assert!(cx.debug_bounds("first/field-0").is_none());

    click(cx, "first/action-kind");
    click(cx, "first/kind-0");
    let mut expected = act("test.two", vec![("first", leaf_action("start").to_value())]);
    assert_eq!(sent_action(&sent), expected);
    // No reply yet: the form shows the edit, and the next edit keeps it.
    type_at(&view, cx, &at(&[("first", None)], "text"), "on");
    expected
        .settings
        .insert("first".into(), leaf_action("on").to_value());
    assert_eq!(sent_action(&sent), expected);
    click(cx, "first/choice-1-1");
    let mut first = leaf_action("on");
    first
        .settings
        .insert("choice".into(), Value::Text("b".into()));
    expected.settings.insert("first".into(), first.to_value());
    assert_eq!(sent_action(&sent), expected);

    // A contained action can contain actions: a list in the second action.
    click(cx, "second/action-kind");
    click(cx, "second/kind-2");
    click(cx, "second/items/add");
    click(cx, "second/items/add-kind-0");
    type_at(
        &view,
        cx,
        &at(&[("second", None), ("items", Some(0))], "text"),
        "deep",
    );
    let second = act(
        "test.many",
        vec![
            ("delay", Value::Integer(100)),
            ("items", Action::list_to_value(&[leaf_action("deep")])),
        ],
    );
    expected.settings.insert("second".into(), second.to_value());
    assert_eq!(sent_action(&sent), expected);

    // The engine confirms the edits: the inputs keep their text.
    tx.try_send(Event::Binding(BindingState {
        binding: press(),
        action: Some(expected.clone()),
        available: true,
    }))
    .unwrap();
    cx.run_until_parked();
    assert_eq!(
        super::actions::input_text(&view, cx, &at(&[("first", None)], "text")),
        "on"
    );
    // The same kind keeps the settings. "No action" removes the action.
    let count = sent.borrow().len();
    click(cx, "first/action-kind");
    click(cx, "first/kind-0");
    assert_eq!(sent.borrow().len(), count);
    click(cx, "first/action-kind");
    click(cx, "first/kind-none");
    expected.settings.remove("first");
    assert_eq!(sent_action(&sent), expected);
}

#[gpui_kit::test]
fn an_action_list_adds_removes_and_moves_actions(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = open(cx, act("test.many", vec![]));
    let with_items =
        |items: &[Action]| act("test.many", vec![("items", Action::list_to_value(items))]);
    let a = leaf_action("start");
    let b = act("test.two", vec![]);
    click(cx, "items/add");
    click(cx, "items/add-kind-0");
    assert_eq!(sent_action(&sent), with_items(std::slice::from_ref(&a)));
    click(cx, "items/add");
    click(cx, "items/add-kind-1");
    assert_eq!(sent_action(&sent), with_items(&[a.clone(), b.clone()]));
    // The first action has no move up, and the last has no move down.
    let count = sent.borrow().len();
    click(cx, "items.0/up");
    click(cx, "items.1/down");
    assert_eq!(sent.borrow().len(), count);
    click(cx, "items.0/down");
    assert_eq!(sent_action(&sent), with_items(&[b.clone(), a.clone()]));
    click(cx, "items.1/up");
    assert_eq!(sent_action(&sent), with_items(&[a.clone(), b.clone()]));
    // An item of the list has no "No action": it has a remove button.
    click(cx, "items.1/action-kind");
    assert!(cx.debug_bounds("items.1/kind-none").is_none());
    click(cx, "items.1/kind-0");
    assert_eq!(sent_action(&sent), with_items(&[a.clone(), a.clone()]));
    click(cx, "items.0/remove");
    assert_eq!(sent_action(&sent), with_items(std::slice::from_ref(&a)));

    // A contained kind with no handler shows that it is not available, and
    // the status of a contained kind shows next to it.
    let missing = act("test.missing", vec![]);
    tx.try_send(Event::Binding(BindingState {
        binding: press(),
        action: Some(with_items(&[a.clone(), missing])),
        available: true,
    }))
    .unwrap();
    tx.try_send(Event::Status {
        kind: "test.leaf".into(),
        status: Status::NeedsUser {
            text: "Do a step.".into(),
            button: "Continue".into(),
        },
    })
    .unwrap();
    cx.run_until_parked();
    assert!(cx.debug_bounds("items.1/unavailable").is_some());
    assert!(cx.debug_bounds("items.0/unavailable").is_none());
    click(cx, "items.0/resolve-kind");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::Resolve {
            kind: "test.leaf".into()
        })
    );
}
