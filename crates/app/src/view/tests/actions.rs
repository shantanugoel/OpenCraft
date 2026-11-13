//! The form uses a fake kind, fake events, and the no-display platform.

use super::*;
use crate::view::actions::{FieldPath, Slot};
use gpui_kit::component::input::AnyInputState;

/// The path of a field of the action of the binding.
pub(super) fn top(key: &str) -> FieldPath {
    at(&[], key)
}

/// The path of a field of a contained action. Each step is a key, and a
/// position for an action list.
pub(super) fn at(steps: &[(&str, Option<usize>)], key: &str) -> FieldPath {
    FieldPath {
        action: steps
            .iter()
            .map(|(key, item)| Slot {
                key: (*key).into(),
                item: *item,
            })
            .collect(),
        key: key.into(),
    }
}

fn input_state(
    view: &Entity<DeviceView>,
    cx: &mut VisualTestContext,
    path: &FieldPath,
) -> AnyInputState {
    cx.update(|_, cx| view.read(cx).form.inputs[path].state.clone())
}

pub(super) fn input_text(
    view: &Entity<DeviceView>,
    cx: &mut VisualTestContext,
    path: &FieldPath,
) -> String {
    let input = input_state(view, cx, path);
    cx.update(|_, cx| input.value(cx).to_string())
}

pub(super) fn dynamic_kind() -> KindDescription {
    KindDescription {
        kind: "test.dynamic".into(),
        name: "Dynamic fields".into(),
        group: "Tests".into(),
        fields: vec![
            Field {
                key: "device".into(),
                label: "Device".into(),
                input: InputType::RuntimeChoice,
                default: Some(Value::Text("saved-device".into())),
            },
            Field {
                key: "colour".into(),
                label: "Colour".into(),
                input: InputType::Colour,
                default: Some(Value::Text("#1A6FD8".into())),
            },
        ],
        ..Default::default()
    }
}

#[gpui_kit::test]
fn runtime_choices_keep_missing_values_reject_late_replies_and_edit_colours(
    cx: &mut TestAppContext,
) {
    let (view, cx, tx, sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(1600.)));
    let kind = dynamic_kind();
    let mut expected = Action {
        kind: kind.kind.clone(),
        settings: kind
            .fields
            .iter()
            .map(|f| (f.key.clone(), f.default.clone().unwrap()))
            .collect(),
    };
    tx.try_send(Event::Kinds(vec![kind])).unwrap();
    tx.try_send(event(binding(GestureKind::Press), expected.clone(), true))
        .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    let request = sent
        .borrow()
        .iter()
        .find_map(|c| match c {
            Command::Choices { id, field, .. } if field == "device" => Some(*id),
            _ => None,
        })
        .unwrap();
    let before = sent.borrow().len();
    tx.try_send(Event::Choices {
        id: request,
        options: vec![ChoiceOption {
            value: "new-device".into(),
            label: "New output".into(),
        }],
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), before);
    click(cx, "choice-0");
    click(cx, "choice-0-0");
    expect_change(
        &sent,
        &mut expected,
        "device",
        Value::Text("new-device".into()),
    );
    click(cx, "field-colour-1-0");
    expect_change(
        &sent,
        &mut expected,
        "colour",
        Value::Text(format!("#{:06X}", crate::view::theme::FILL_COLOURS[0])),
    );
    type_field(&view, cx, "colour", "#0a0b0c");
    expect_change(
        &sent,
        &mut expected,
        "colour",
        Value::Text("#0A0B0C".into()),
    );
    let before = sent.borrow().len();
    type_field(&view, cx, "colour", "bad");
    assert!(!sent.borrow()[before..]
        .iter()
        .any(|c| matches!(c, Command::SetBinding { .. })));
    // A closed form no longer accepts the reply of its earlier request.
    click(cx, "surface-key face 2");
    tx.try_send(Event::Choices {
        id: request,
        options: vec![],
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn runtime_choice_lists_keep_order_and_remove_saved_unavailable_values(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(1600.)));
    let mut kind = dynamic_kind();
    kind.fields = vec![Field {
        key: "devices".into(),
        label: "Cycle list".into(),
        input: InputType::RuntimeChoiceList,
        default: None,
    }];
    let mut expected = Action {
        kind: kind.kind.clone(),
        settings: Settings::from([(
            "devices".into(),
            Value::List(vec![Value::Text("missing".into())]),
        )]),
    };
    tx.try_send(Event::Kinds(vec![kind])).unwrap();
    tx.try_send(event(binding(GestureKind::Press), expected.clone(), true))
        .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    let id = sent
        .borrow()
        .iter()
        .find_map(|c| match c {
            Command::Choices { id, .. } => Some(*id),
            _ => None,
        })
        .unwrap();
    tx.try_send(Event::Choices {
        id,
        options: vec![
            ChoiceOption {
                value: "a".into(),
                label: "Speakers".into(),
            },
            ChoiceOption {
                value: "b".into(),
                label: "Headphones".into(),
            },
        ],
    })
    .unwrap();
    cx.run_until_parked();
    click(cx, "choice-0");
    click(cx, "choice-0-1");
    expect_change(
        &sent,
        &mut expected,
        "devices",
        Value::List(vec![Value::Text("missing".into()), Value::Text("b".into())]),
    );
    click(cx, "choice-0");
    click(cx, "choice-0-0");
    expect_change(
        &sent,
        &mut expected,
        "devices",
        Value::List(vec![
            Value::Text("missing".into()),
            Value::Text("b".into()),
            Value::Text("a".into()),
        ]),
    );
    click(cx, "choice-0");
    click(cx, "choice-0-2");
    expect_change(
        &sent,
        &mut expected,
        "devices",
        Value::List(vec![Value::Text("b".into()), Value::Text("a".into())]),
    );
}

fn kind() -> KindDescription {
    KindDescription {
        kind: "test.kind".into(),
        name: "Test action".into(),
        fields: vec![
            ("text", InputType::Text, Value::Text("start".into())),
            (
                "long",
                InputType::LongText,
                Value::Text("many\nlines".into()),
            ),
            (
                "integer",
                InputType::Integer { min: -5, max: 10 },
                Value::Integer(2),
            ),
            (
                "choice",
                InputType::Choice {
                    options: vec![
                        ChoiceOption {
                            value: "a".into(),
                            label: "First option".into(),
                        },
                        ChoiceOption {
                            value: "b".into(),
                            label: "Second option".into(),
                        },
                    ],
                },
                Value::Text("a".into()),
            ),
            ("toggle", InputType::Toggle, Value::Boolean(false)),
            ("hotkey", InputType::Hotkey, Value::Text("cmd+a".into())),
            (
                "file",
                InputType::Path {
                    kind: PathKind::File,
                },
                Value::Text(String::new()),
            ),
            (
                "application",
                InputType::Path {
                    kind: PathKind::Application,
                },
                Value::Text(String::new()),
            ),
        ]
        .into_iter()
        .map(|(key, input, default)| Field {
            key: key.into(),
            label: format!("Field {key}"),
            input,
            default: Some(default),
        })
        .collect(),
        ..Default::default()
    }
}

fn binding(gesture: GestureKind) -> BindingId {
    BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Key, 2),
        gesture,
    }
}

fn action() -> Action {
    let k = kind();
    let mut settings: Settings = k
        .fields
        .iter()
        .map(|f| (f.key.clone(), f.default.clone().unwrap()))
        .collect();
    settings.insert(
        "unknown".into(),
        Value::Table(Settings::from([(
            "nested".into(),
            Value::List(vec![Value::Integer(42)]),
        )])),
    );
    Action {
        kind: k.kind,
        settings,
    }
}

fn event(id: BindingId, action: Action, available: bool) -> Event {
    Event::Binding(BindingState {
        binding: id,
        action: Some(action),
        available,
    })
}

fn type_field(view: &Entity<DeviceView>, cx: &mut VisualTestContext, key: &str, value: &str) {
    type_at(view, cx, &top(key), value);
}

pub(super) fn type_at(
    view: &Entity<DeviceView>,
    cx: &mut VisualTestContext,
    path: &FieldPath,
    value: &str,
) {
    let input = input_state(view, cx, path);
    cx.update(|window, cx| {
        window.focus(&input.focus_handle(cx), cx);
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
    });
    cx.run_until_parked();
    cx.simulate_input(value);
    cx.run_until_parked();
}

fn expect_change(sent: &Commands, expected: &mut Action, key: &str, value: Value) {
    expected.settings.insert(key.into(), value);
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: binding(GestureKind::Press),
            action: Some(expected.clone())
        })
    );
}

#[gpui_kit::test]
fn all_field_types_send_full_settings_and_keep_unknown_keys(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    // A tall test viewport lets the test click each field without scrolling.
    cx.simulate_resize(size(px(1120.), px(2400.)));
    tx.try_send(Event::Kinds(vec![kind()])).unwrap();
    let mut expected = action();
    tx.try_send(event(binding(GestureKind::Press), expected.clone(), true))
        .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    for id in [
        "field-0", "field-1", "field-2", "field-3", "field-4", "field-5", "field-6", "field-7",
    ] {
        assert!(cx.debug_bounds(id).is_some());
    }
    assert!(matches!(
        input_state(&view, cx, &top("long")),
        AnyInputState::Textarea(_)
    ));
    type_field(&view, cx, "text", "changed");
    expect_change(&sent, &mut expected, "text", Value::Text("changed".into()));
    // No reply yet: the next edit must retain the first edit.
    type_field(&view, cx, "long", "line one\nline two");
    expect_change(
        &sent,
        &mut expected,
        "long",
        Value::Text("line one\nline two".into()),
    );
    type_field(&view, cx, "integer", "999");
    expect_change(&sent, &mut expected, "integer", Value::Integer(10));
    type_field(&view, cx, "integer", "-999");
    expect_change(&sent, &mut expected, "integer", Value::Integer(-5));
    click(cx, "choice-3");
    click(cx, "choice-3-1");
    expect_change(&sent, &mut expected, "choice", Value::Text("b".into()));
    click(cx, "toggle-4");
    expect_change(&sent, &mut expected, "toggle", Value::Boolean(true));
    click(cx, "hotkey-5");
    cx.simulate_keystrokes("shift-cmd-4");
    cx.run_until_parked();
    expect_change(
        &sent,
        &mut expected,
        "hotkey",
        Value::Text("shift+cmd+4".into()),
    );
    click(cx, "hotkey-5");
    let count = sent.borrow().len();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), count);
    click(cx, "clear-hotkey-5");
    expect_change(&sent, &mut expected, "hotkey", Value::Text(String::new()));
    type_field(&view, cx, "file", "/tmp/test-file");
    expect_change(
        &sent,
        &mut expected,
        "file",
        Value::Text("/tmp/test-file".into()),
    );
    click(cx, "path-7");
    assert!(cx.did_prompt_for_paths());
    cx.simulate_path_prompt_response(|options| {
        assert!(options.files);
        assert!(!options.multiple);
        Some(vec!["/Applications/Test.app".into()])
    });
    cx.run_until_parked();
    expect_change(
        &sent,
        &mut expected,
        "application",
        Value::Text("/Applications/Test.app".into()),
    );
    cx.update(|_, cx| {
        // Sending commands does not change confirmed bindings.
        assert_eq!(
            view.read(cx).state.bindings[&binding(GestureKind::Press)].action,
            Some(action())
        );
    });
    tx.try_send(event(binding(GestureKind::Press), expected.clone(), true))
        .unwrap();
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.bindings[&binding(GestureKind::Press)].action,
            Some(expected)
        )
    });
}

/// The action of `kind()` with the default value of each field.
fn defaults() -> Action {
    Action {
        kind: "test.kind".into(),
        settings: kind()
            .fields
            .iter()
            .map(|f| (f.key.clone(), f.default.clone().unwrap()))
            .collect(),
    }
}

fn open_card(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> Option<GestureKind> {
    cx.update(|_, cx| view.read(cx).state.gesture)
}

#[gpui_kit::test]
fn cards_open_set_a_kind_change_a_setting_and_clear_an_action(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(2400.)));
    tx.try_send(Event::Kinds(vec![kind()])).unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    // A key has the cards "Press" and "Release". With no action, no card is
    // open, and each card is a dashed card "Add an action".
    for (kind, head) in [
        ("gesture-Press-kind", "gesture-Press-head"),
        ("gesture-Release-kind", "gesture-Release-head"),
    ] {
        assert!(cx.debug_bounds(kind).is_some());
        assert!(cx.debug_bounds(head).is_none());
    }
    assert_eq!(open_card(&view, cx), None);
    assert!(
        cx.debug_bounds("gesture-Press").unwrap().bottom()
            < cx.debug_bounds("gesture-Release").unwrap().top()
    );

    // Set a kind: a click on the empty card opens the kind menu. A focus in
    // the title field does not stay after a click in the menu.
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        window.focus(&input.focus_handle(cx), cx);
    });
    cx.run_until_parked();
    click(cx, "gesture-Release-kind");
    assert!(cx.debug_bounds("kind-menu").is_some());
    assert!(cx.debug_bounds("kind-remove").is_none());
    click(cx, "kind-0");
    assert!(cx.debug_bounds("kind-menu").is_none());
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        assert!(!input.focus_handle(cx).is_focused(window));
    });
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: binding(GestureKind::Release),
            action: Some(defaults())
        })
    );
    // The choice opens the card. The engine event is the source of truth.
    assert_eq!(open_card(&view, cx), Some(GestureKind::Release));
    cx.update(|_, cx| assert!(view.read(cx).state.bindings.is_empty()));
    tx.try_send(event(binding(GestureKind::Release), defaults(), true))
        .unwrap();
    cx.run_until_parked();
    assert!(cx.debug_bounds("gesture-Release-head").is_some());
    assert!(cx.debug_bounds("field-0").is_some());

    // Change a setting of the open card.
    type_field(&view, cx, "text", "changed");
    let mut changed = defaults();
    changed
        .settings
        .insert("text".into(), Value::Text("changed".into()));
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: binding(GestureKind::Release),
            action: Some(changed.clone())
        })
    );
    tx.try_send(event(binding(GestureKind::Release), changed, true))
        .unwrap();

    // One card is open at a time: a click on the head of a closed card
    // opens it and closes the open card. A second click closes it.
    tx.try_send(event(binding(GestureKind::Press), defaults(), true))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(open_card(&view, cx), Some(GestureKind::Release));
    let count = sent.borrow().len();
    click(cx, "gesture-Press-name");
    assert_eq!(open_card(&view, cx), Some(GestureKind::Press));
    assert_eq!(input_text(&view, cx, &top("text")), "start");
    click(cx, "gesture-Press-name");
    assert_eq!(open_card(&view, cx), None);
    assert!(cx.debug_bounds("field-0").is_none());
    click(cx, "gesture-Release-name");
    assert_eq!(input_text(&view, cx, &top("text")), "changed");
    assert_eq!(sent.borrow().len(), count, "A card opens with no command");

    // The inspector remembers the open card of each control.
    click(cx, "surface-bar panel 3");
    assert_eq!(open_card(&view, cx), None);
    click(cx, "surface-key face 2");
    assert_eq!(open_card(&view, cx), Some(GestureKind::Release));

    // A closed card with an action has its popup: the same kind opens the
    // card and keeps the settings.
    click(cx, "gesture-Press-kind");
    click(cx, "kind-0");
    assert_eq!(sent.borrow().len(), count);
    assert_eq!(open_card(&view, cx), Some(GestureKind::Press));

    // Clear an action: "Remove the action" in the menu of the card.
    click(cx, "gesture-Press-kind");
    click(cx, "kind-remove");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: binding(GestureKind::Press),
            action: None
        })
    );
    assert_eq!(open_card(&view, cx), None);
    tx.try_send(Event::Binding(BindingState {
        binding: binding(GestureKind::Press),
        action: None,
        available: false,
    }))
    .unwrap();
    cx.run_until_parked();
    assert!(cx.debug_bounds("gesture-Press-head").is_none());
    assert!(cx.debug_bounds("gesture-Release-head").is_some());

    // A knob has the cards of its gestures. The old gesture buttons are
    // gone.
    // With no turn binding, the knob has the one "Turn" card.
    click(cx, "control-knob 2");
    for id in ["gesture-Turn", "gesture-Push"] {
        assert!(cx.debug_bounds(id).is_some(), "{id}");
    }
    assert!(cx.debug_bounds("gesture-TurnLeft").is_none());
    assert!(cx.debug_bounds("gesture-Press").is_none());
    click(cx, "gesture-Push-kind");
    click(cx, "kind-0");
    assert!(matches!(
        sent.borrow().last(),
        Some(Command::SetBinding {
            binding: BindingId {
                gesture: GestureKind::Push,
                control: ControlId {
                    kind: ControlKind::Knob,
                    ..
                },
                ..
            },
            action: Some(_)
        })
    ));
    click(cx, "surface-bar panel 3");
    assert!(cx.debug_bounds("choose-image").is_some());
    assert!(cx.debug_bounds("gesture-Tap").is_some());
}

#[gpui_kit::test]
fn a_menu_closes_with_escape_and_with_a_click_out_of_it(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(2400.)));
    tx.try_send(Event::Kinds(vec![kind()])).unwrap();
    tx.try_send(event(binding(GestureKind::Press), defaults(), true))
        .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    let count = sent.borrow().len();
    click(cx, "gesture-Release-kind");
    assert!(cx.debug_bounds("kind-menu").is_some());
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("kind-menu").is_none());
    // A click out of the menu closes it, and the element below does not
    // get the click.
    click(cx, "choice-3");
    assert!(cx.debug_bounds("choice-3-menu").is_some());
    click(cx, "gesture-Press-name");
    assert!(cx.debug_bounds("choice-3-menu").is_none());
    assert!(cx.debug_bounds("field-0").is_some());
    // One menu is open at a time.
    click(cx, "gesture-Release-kind");
    click(cx, "gesture-Press-kind");
    assert!(cx.debug_bounds("kind-menu").is_none());
    click(cx, "gesture-Press-kind");
    assert!(cx.debug_bounds("kind-remove").is_some());
    click(cx, "gesture-Press-kind");
    assert!(cx.debug_bounds("kind-menu").is_none());
    assert_eq!(sent.borrow().len(), count);
}

#[gpui_kit::test]
fn unavailable_binding_and_status_are_kept_until_engine_events(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    let missing = Action {
        kind: "test.missing".into(),
        settings: action().settings,
    };
    tx.try_send(event(binding(GestureKind::Press), missing.clone(), false))
        .unwrap();
    tx.try_send(Event::Status {
        kind: "test.missing".into(),
        status: Status::NeedsUser {
            text: "Do a step.".into(),
            button: "Continue".into(),
        },
    })
    .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("unavailable").is_some());
    click(cx, "resolve-kind");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::Resolve {
            kind: "test.missing".into()
        })
    );
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.bindings[&binding(GestureKind::Press)].action,
            Some(missing)
        )
    });
    assert!(cx.debug_bounds("resolve-kind").is_some());
    tx.try_send(Event::Status {
        kind: "test.missing".into(),
        status: Status::Ready,
    })
    .unwrap();
    tx.try_send(Event::Problem("The handler reports a problem.".into()))
        .unwrap();
    cx.run_until_parked();
    assert!(cx.debug_bounds("resolve-kind").is_none());
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.problems,
            ["The handler reports a problem."]
        )
    });
    // The card of a kind with no handler has the kinds and "Remove the
    // action" in its menu.
    assert!(cx.debug_bounds("gesture-Press-head").is_some());
    click(cx, "gesture-Press-kind");
    assert!(cx.debug_bounds("kind-menu").is_some());
    click(cx, "kind-remove");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: binding(GestureKind::Press),
            action: None
        })
    );
}

#[gpui_kit::test]
fn delayed_path_dialog_targets_original_binding_and_cancel_sends_nothing(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(2400.)));
    tx.try_send(Event::Kinds(vec![kind()])).unwrap();
    tx.try_send(event(binding(GestureKind::Press), action(), true))
        .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    click(cx, "path-6");
    cx.simulate_path_prompt_response(|_| None);
    cx.run_until_parked();
    assert!(sent.borrow().is_empty());
    click(cx, "path-6");
    click(cx, "control-knob 2");
    cx.simulate_path_prompt_response(|_| Some(vec!["/tmp/selected".into()]));
    cx.run_until_parked();
    let mut expected = action();
    expect_change(
        &sent,
        &mut expected,
        "file",
        Value::Text("/tmp/selected".into()),
    );
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.selected_control.unwrap().kind,
            ControlKind::Knob
        )
    });
}

#[gpui_kit::test]
fn old_replies_keep_later_edits_and_new_events_replace_the_form(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    tx.try_send(Event::Kinds(vec![kind()])).unwrap();
    tx.try_send(event(binding(GestureKind::Press), action(), true))
        .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    type_field(&view, cx, "text", "first edit");
    let Command::SetBinding {
        action: Some(first),
        ..
    } = sent.borrow().last().unwrap().clone()
    else {
        panic!()
    };
    type_field(&view, cx, "long", "second edit");
    let Command::SetBinding {
        action: Some(second),
        ..
    } = sent.borrow().last().unwrap().clone()
    else {
        panic!()
    };
    tx.try_send(event(binding(GestureKind::Press), first.clone(), true))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(input_text(&view, cx, &top("long")), "second edit");
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.bindings[&binding(GestureKind::Press)].action,
            Some(first)
        );
    });
    tx.try_send(event(binding(GestureKind::Press), second, true))
        .unwrap();
    cx.run_until_parked();
    let mut external = action();
    external
        .settings
        .insert("text".into(), Value::Text("engine value".into()));
    tx.try_send(event(binding(GestureKind::Press), external, true))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(input_text(&view, cx, &top("text")), "engine value");
    let count = sent.borrow().len();
    // Selecting the same kind must not reset its settings to defaults.
    click(cx, "gesture-Press-kind");
    click(cx, "kind-0");
    assert_eq!(sent.borrow().len(), count);
    tx.try_send(Event::Binding(BindingState {
        binding: binding(GestureKind::Press),
        action: None,
        available: false,
    }))
    .unwrap();
    cx.run_until_parked();
    assert!(cx.debug_bounds("field-0").is_none());
}

#[gpui_kit::test]
fn editor_scrolls_at_the_minimum_window_size(cx: &mut TestAppContext) {
    let (_, cx, tx, _) = window(cx);
    cx.simulate_resize(size(px(900.), px(640.)));
    tx.try_send(Event::Kinds(vec![kind()])).unwrap();
    tx.try_send(event(binding(GestureKind::Press), action(), true))
        .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    for _ in 0..20 {
        let editor = cx.debug_bounds("editor").unwrap();
        let target = cx.debug_bounds("path-7").unwrap();
        if editor.contains(&target.center()) {
            click(cx, "path-7");
            assert!(cx.did_prompt_for_paths());
            cx.simulate_path_prompt_response(|_| None);
            cx.run_until_parked();
            return;
        }
        cx.simulate_event(gpui_kit::ScrollEvent {
            position: editor.center(),
            delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-90.))),
            modifiers: Modifiers::default(),
            touch_phase: gpui_kit::TouchPhase::Moved,
            source: gpui_kit::ScrollSource::Pointer,
        });
        cx.run_until_parked();
    }
    panic!("The editor does not scroll to the last field.");
}

#[gpui_kit::test]
fn the_button_test_the_action_sends_the_command_for_the_gesture_of_its_card(
    cx: &mut TestAppContext,
) {
    let (view, cx, tx, sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(2400.)));
    tx.try_send(Event::Kinds(vec![kind()])).unwrap();
    tx.try_send(event(binding(GestureKind::Press), action(), true))
        .unwrap();
    let missing = Action {
        kind: "test.missing".into(),
        settings: Settings::new(),
    };
    tx.try_send(event(binding(GestureKind::Release), missing, false))
        .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");

    // The open card has the button as its last row. The closed card has no
    // button.
    assert_eq!(open_card(&view, cx), Some(GestureKind::Press));
    let test = cx.debug_bounds("gesture-Press-test").unwrap();
    assert!(test.top() > cx.debug_bounds("field-7").unwrap().bottom());
    assert!(cx.debug_bounds("gesture-Release-test").is_none());
    let count = sent.borrow().len();
    click(cx, "gesture-Press-test");
    assert_eq!(sent.borrow().len(), count + 1);
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::TestAction {
            binding: binding(GestureKind::Press)
        })
    );
    // The window tells that the test went to the engine. The card stays open.
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).status_message.as_ref().map(|m| m.to_string()),
            Some("Testing “Test action” of Key 2 for Press".to_string())
        )
    });
    assert_eq!(open_card(&view, cx), Some(GestureKind::Press));

    // An action with no handler cannot run: its open card has no button.
    click(cx, "gesture-Release-name");
    assert_eq!(open_card(&view, cx), Some(GestureKind::Release));
    assert!(cx.debug_bounds("unavailable").is_some());
    assert!(cx.debug_bounds("gesture-Release-test").is_none());
}
