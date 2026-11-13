//! The nested cards, the step lists, and the appearance of each state
//! (decision 0029, section 10.5), with fake kinds.

use super::actions::{at, type_at};
use super::*;
use gpui_kit::Pixels;
use opencraft_engine::{engine::StateImages, FeedbackState, TwoStates};

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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
    }
}

/// A kind with two states, as a toggle: two actions and a label for On.
fn toggle() -> KindDescription {
    KindDescription {
        kind: "test.toggle".into(),
        name: "Two states".into(),
        fields: vec![
            Field {
                key: "on".into(),
                label: "From Off to On".into(),
                input: InputType::Action,
                default: None,
            },
            Field {
                key: "off".into(),
                label: "From On to Off".into(),
                input: InputType::Action,
                default: None,
            },
            Field {
                key: "label".into(),
                label: "Label for on".into(),
                input: InputType::Text,
                default: None,
            },
        ],
        states: Some(TwoStates {
            on_label: Some("label".into()),
        }),
        ..Default::default()
    }
}

fn kinds() -> Event {
    let mut timed = many();
    timed.kind = "test.timed".into();
    timed.fields[0].input = InputType::TimedActionList;
    timed.fields[1].key = "delay_ms".into();
    Event::Kinds(vec![leaf(), two(), many(), toggle(), timed])
}

#[gpui_kit::test]
fn a_timed_step_edits_and_moves_its_delay_with_the_action(cx: &mut TestAppContext) {
    let root = act(
        "test.timed",
        vec![
            ("delay_ms", Value::Integer(250)),
            (
                "items",
                Action::list_to_value(&[leaf_action("a"), leaf_action("b")]),
            ),
        ],
    );
    let (view, cx, _, sent) = open(cx, root);
    click(cx, "items.1/step");
    let path = at(&[("items", Some(1))], "delay_before_ms");
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).form.inputs[&path].state.value(cx).as_ref(),
            "250"
        );
    });
    type_at(&view, cx, &path, "725");
    let edited = sent_action(&sent);
    let steps = super::super::actions::list(&edited, "items");
    assert_eq!(steps[1].settings["delay_before_ms"], Value::Integer(725));
    // The same shared field supplies the accessible name.
    let labels = super::super::controls::accessibility_log::take();
    assert!(labels
        .iter()
        .any(|(_, label)| label == "Delay before, in milliseconds"));
    // Use the normal step-menu path, also used by drag and drop.
    click(cx, "items.1/more");
    choose(&view, cx, super::super::nested::MoveStepUp);
    let moved = sent_action(&sent);
    let steps = super::super::actions::list(&moved, "items");
    assert_eq!(steps[0].settings["delay_before_ms"], Value::Integer(725));
    assert_eq!(steps[1].settings["delay_before_ms"], Value::Integer(0));
}

fn press() -> BindingId {
    BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Key, 2),
        gesture: GestureKind::Press,
        folder: None,
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

/// The action and the name of the step of the last command for the binding:
/// a `SetBinding`, or a group of one `SetBinding` with a name.
fn sent_step(sent: &Commands) -> (Action, Option<String>) {
    let binding_action = |command: &Command| match command {
        Command::SetBinding {
            binding,
            action: Some(action),
        } if *binding == press() => Some(action.clone()),
        _ => None,
    };
    match sent.borrow().last() {
        Some(Command::Group { name, commands }) if commands.len() == 1 => (
            binding_action(&commands[0]).expect("a binding command in the group"),
            Some(name.clone()),
        ),
        Some(other) => (
            binding_action(other).unwrap_or_else(|| panic!("No binding command: {other:?}")),
            None,
        ),
        None => panic!("No command"),
    }
}

fn sent_action(sent: &Commands) -> Action {
    sent_step(sent).0
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
    // The test window must be active, else no loss of the focus comes
    // (`docs/kb/gpui.md`).
    cx.update(|window, _| window.activate_window());
    click(cx, "surface-key face 2");
    (view, cx, tx, sent)
}

/// The centre of an element.
fn centre(cx: &mut VisualTestContext, id: &'static str) -> gpui_kit::Point<Pixels> {
    cx.debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"))
        .center()
}

/// A click on the head `id` of an open nested card, out of its kind
/// button: the click closes the card. The click of `click` is in the
/// centre, and the kind button of a long kind name can be there.
fn close(cx: &mut VisualTestContext, id: &'static str) {
    let bounds = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"));
    let point = gpui_kit::point(bounds.right() - px(40.), bounds.center().y);
    cx.simulate_mouse_move(point, None, Modifiers::default());
    cx.simulate_click(point, Modifiers::default());
    cx.run_until_parked();
}

/// Choose `item` in the open step menu. The native menu of macOS sends its
/// action to the root element of the window. A different platform can show
/// its own menu with the focus in it: the test gives the focus back first,
/// as `tests/clipboard.rs` does. Then a click on the inspector header,
/// which does nothing, closes such a menu.
fn choose(view: &Entity<DeviceView>, cx: &mut VisualTestContext, item: impl gpui_kit::Action) {
    cx.update(|window, cx| {
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx)
    });
    cx.run_until_parked();
    cx.dispatch_action(item);
    cx.run_until_parked();
    // A drawn menu of a different platform closes at a click out of it.
    click(cx, "inspector-header");
}

/// A right click on the step row `id`, and then a choice of the step menu.
fn step_menu(
    view: &Entity<DeviceView>,
    cx: &mut VisualTestContext,
    id: &'static str,
    item: impl gpui_kit::Action,
) {
    let position = centre(cx, id);
    cx.simulate_mouse_move(position, None, Modifiers::default());
    cx.simulate_event(gpui_kit::MouseDownEvent {
        button: gpui_kit::MouseButton::Right,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.run_until_parked();
    choose(view, cx, item);
}

/// A drag with the left button from the element `from` to the element
/// `to`, in eight steps.
fn drag(cx: &mut VisualTestContext, from: &'static str, to: &'static str) {
    let (from, to) = (centre(cx, from), centre(cx, to));
    cx.simulate_mouse_move(from, None, Modifiers::default());
    cx.run_until_parked();
    cx.simulate_mouse_down(from, gpui_kit::MouseButton::Left, Modifiers::default());
    for step in 1..=8 {
        let f = step as f32 / 8.;
        let at = gpui_kit::point(from.x + (to.x - from.x) * f, from.y + (to.y - from.y) * f);
        cx.simulate_mouse_move(at, gpui_kit::MouseButton::Left, Modifiers::default());
    }
    cx.simulate_mouse_up(to, gpui_kit::MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
}

#[gpui_kit::test]
fn the_two_actions_of_a_toggle_are_nested_cards_and_each_change_is_one_step(
    cx: &mut TestAppContext,
) {
    let (view, cx, tx, sent) = open(cx, act("test.two", vec![]));
    // Two empty nested cards.
    assert!(cx.debug_bounds("first/action-kind").is_some());
    assert!(cx.debug_bounds("second/action-kind").is_some());
    assert!(cx.debug_bounds("first/field-0").is_none());

    // A new action opens its card.
    click(cx, "first/action-kind");
    click(cx, "first/kind-0");
    let mut expected = act("test.two", vec![("first", leaf_action("start").to_value())]);
    assert_eq!(sent_step(&sent), (expected.clone(), None));
    assert!(cx.debug_bounds("first/field-0").is_some());
    // No reply yet: the form shows the edit, and the next edit keeps it.
    type_at(&view, cx, &at(&[("first", None)], "text"), "on");
    expected
        .settings
        .insert("first".into(), leaf_action("on").to_value());
    assert_eq!(sent_action(&sent), expected);
    click(cx, "first/choice-1");
    click(cx, "first/choice-1-1");
    let mut first = leaf_action("on");
    first
        .settings
        .insert("choice".into(), Value::Text("b".into()));
    expected.settings.insert("first".into(), first.to_value());
    assert_eq!(sent_action(&sent), expected);

    // The second action, while the text field has the focus: the text edit
    // ends first, so that the change is a step of its own (decision 0030).
    type_at(&view, cx, &at(&[("first", None)], "text"), "on");
    let typed = sent.borrow().len();
    click(cx, "second/action-kind");
    click(cx, "second/kind-0");
    let commands = sent.borrow()[typed..].to_vec();
    assert_eq!(
        commands.first(),
        Some(&Command::EndTextEdit),
        "{commands:?}"
    );
    expected
        .settings
        .insert("second".into(), leaf_action("start").to_value());
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
        super::actions::input_text(&view, cx, &at(&[("first", None)], "text"),),
        "on"
    );

    // A click on the head closes the card: one row with the short value.
    // A click on the row opens it again.
    close(cx, "first/action-head");
    assert!(cx.debug_bounds("first/field-0").is_none());
    assert!(cx.debug_bounds("first/action-head").is_some());
    cx.update(|_, cx| assert_eq!(view.read(cx).short_value(&first), "on"));
    click(cx, "first/action-head");
    assert!(cx.debug_bounds("first/field-0").is_some());

    // The same kind keeps the settings. "Remove the action" removes it, as
    // one step with its name.
    let count = sent.borrow().len();
    click(cx, "first/action-kind");
    click(cx, "first/kind-0");
    assert_eq!(sent.borrow().len(), count);
    click(cx, "first/action-kind");
    click(cx, "first/kind-remove");
    expected.settings.remove("first");
    assert_eq!(
        sent_step(&sent),
        (expected, Some("Remove the Action".into()))
    );
}

#[gpui_kit::test]
fn a_step_list_adds_moves_and_removes_steps(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = open(cx, act("test.many", vec![]));
    let with_items =
        |items: &[Action]| act("test.many", vec![("items", Action::list_to_value(items))]);
    let a = leaf_action("start");
    let b = act("test.two", vec![]);
    let m = act("test.many", vec![("delay", Value::Integer(100))]);
    // The step list is at the top of the card, and the delay is a row of
    // the form below it.
    assert!(cx.debug_bounds("items/add").is_some());
    assert!(cx.debug_bounds("field-1").is_some());

    // "Add a step" opens the kind menu. The new step opens, and closes the
    // step that was open.
    click(cx, "items/add");
    click(cx, "items/add-kind-0");
    assert_eq!(
        sent_step(&sent),
        (
            with_items(std::slice::from_ref(&a)),
            Some("Add Step".into())
        )
    );
    assert!(cx.debug_bounds("items.0/field-0").is_some());
    click(cx, "items/add");
    click(cx, "items/add-kind-1");
    assert!(cx.debug_bounds("items.0/field-0").is_none());
    assert!(cx.debug_bounds("items.1/first/action-kind").is_some());
    click(cx, "items/add");
    click(cx, "items/add-kind-0");
    click(cx, "items/add");
    click(cx, "items/add-kind-2");
    assert_eq!(
        sent_action(&sent),
        with_items(&[a.clone(), b.clone(), a.clone(), m.clone()])
    );
    // Four rows. A closed step shows its short value.
    for id in [
        "items.0/step",
        "items.1/step",
        "items.2/step",
        "items.3/step",
    ] {
        assert!(cx.debug_bounds(id).is_some(), "{id}");
    }
    cx.update(|_, cx| {
        assert_eq!(view.read(cx).short_value(&a), "start");
        assert_eq!(view.read(cx).short_value(&b), "Two actions");
        assert_eq!(
            view.read(cx)
                .short_value(&with_items(std::slice::from_ref(&a))),
            "1 step"
        );
    });

    // The step menu: "Move Up" of the first step does nothing; "Move Down"
    // moves it.
    let count = sent.borrow().len();
    step_menu(&view, cx, "items.0/step", super::super::nested::MoveStepUp);
    assert_eq!(sent.borrow().len(), count);
    step_menu(
        &view,
        cx,
        "items.0/step",
        super::super::nested::MoveStepDown,
    );
    assert_eq!(
        sent_step(&sent),
        (
            with_items(&[b.clone(), a.clone(), a.clone(), m.clone()]),
            Some("Move Step".into())
        )
    );
    step_menu(&view, cx, "items.1/step", super::super::nested::MoveStepUp);
    assert_eq!(
        sent_action(&sent),
        with_items(&[a.clone(), b.clone(), a.clone(), m.clone()])
    );
    // The open last step moved with "Move Up": it stays open at its new
    // place. Close it.
    close(cx, "items.3/action-head");

    // A drag of the handle of the last step to the first row.
    drag(cx, "items.3/handle", "items.0/step");
    assert_eq!(
        sent_step(&sent),
        (
            with_items(&[m.clone(), a.clone(), b.clone(), a.clone()]),
            Some("Move Step".into())
        )
    );

    // "Remove" of the step menu, and of the "⋯" button of an open step.
    step_menu(&view, cx, "items.2/step", super::super::nested::RemoveStep);
    assert_eq!(
        sent_step(&sent),
        (
            with_items(&[m.clone(), a.clone(), a.clone()]),
            Some("Remove Step".into())
        )
    );
    click(cx, "items.0/step");
    assert!(cx.debug_bounds("items.0/more").is_some());
    click(cx, "items.0/more");
    choose(&view, cx, super::super::nested::RemoveStep);
    assert_eq!(sent_action(&sent), with_items(&[a.clone(), a.clone()]));
    // A step has no "Remove the action" in its kind menu: the step menu
    // removes it.
    click(cx, "items.0/step");
    click(cx, "items.0/action-kind");
    assert!(cx.debug_bounds("items.0/kind-remove").is_none());
}

#[gpui_kit::test]
fn a_toggle_contains_a_sequence(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = open(cx, act("test.two", vec![]));
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
    let expected = act("test.two", vec![("second", second.to_value())]);
    assert_eq!(sent_action(&sent), expected);
    cx.update(|_, cx| assert_eq!(view.read(cx).short_value(&second), "1 step"));

    // A contained kind with no handler shows that it is not available, and
    // the status of a contained kind shows next to it.
    let broken = act(
        "test.two",
        vec![(
            "second",
            act(
                "test.many",
                vec![(
                    "items",
                    Action::list_to_value(&[leaf_action("a"), act("test.missing", vec![])]),
                )],
            )
            .to_value(),
        )],
    );
    tx.try_send(Event::Binding(BindingState {
        binding: press(),
        action: Some(broken),
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
    click(cx, "second/items.1/step");
    assert!(cx.debug_bounds("second/items.1/unavailable").is_some());
    click(cx, "second/items.0/step");
    assert!(cx.debug_bounds("second/items.0/unavailable").is_none());
    click(cx, "second/items.0/resolve-kind");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::Resolve {
            kind: "test.leaf".into()
        })
    );
}

/// The state of the key with the image of each state.
fn surface_with_states(now: FeedbackState) -> Event {
    let image = |shade: u8| Arc::new(RgbImage::from_pixel(90, 90, image::Rgb([shade; 3])));
    Event::Surface(SurfaceState {
        id: crate::state::tests::key(),
        content: SurfaceContent {
            label: Some("Mic".into()),
            ..Default::default()
        },
        image: image(10),
        states: Some(StateImages {
            now,
            off: image(10),
            on: image(200),
        }),
    })
}

#[gpui_kit::test]
fn a_kind_with_two_states_shows_the_appearance_of_each_state(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = open(cx, act("test.toggle", vec![]));
    tx.try_send(surface_with_states(FeedbackState::Off))
        .unwrap();
    cx.run_until_parked();
    // The faces "Off" and "On" in place of "Appearance", and the state in
    // the header.
    assert!(cx.debug_bounds("state-off").is_some());
    assert!(cx.debug_bounds("state-on").is_some());
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(
            view.state_line().as_deref(),
            Some("Two states · the state now is Off")
        );
        assert!(view.faces.images.contains_key(&crate::state::tests::key()));
    });
    // "Off" is selected: the rows of "Appearance".
    assert!(cx.debug_bounds("title-field").is_some());
    assert!(cx.debug_bounds("on-label-field").is_none());
    // The card of "Press" has the two nested cards, and not the label for
    // on: the face "On" has it.
    assert!(cx.debug_bounds("on/action-kind").is_some());
    assert!(cx.debug_bounds("off/action-kind").is_some());
    assert!(cx.debug_bounds("field-2").is_none());

    // "On" selected: the field of the label for on.
    click(cx, "state-on");
    assert!(cx.debug_bounds("title-field").is_none());
    let input = cx.update(|_, cx| view.read(cx).faces.input.clone());
    cx.update(|window, cx| window.focus(&input.focus_handle(cx), cx));
    cx.run_until_parked();
    assert_eq!(sent.borrow().last(), Some(&Command::StartTextEdit));
    cx.simulate_input("Live");
    cx.run_until_parked();
    assert_eq!(
        sent_action(&sent),
        act("test.toggle", vec![("label", Value::Text("Live".into()))])
    );

    // The state On from the engine.
    tx.try_send(surface_with_states(FeedbackState::On)).unwrap();
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state_line().as_deref(),
            Some("Two states · the state now is On")
        )
    });
    // A kind with one state has "Appearance".
    tx.try_send(Event::Binding(BindingState {
        binding: press(),
        action: Some(leaf_action("x")),
        available: true,
    }))
    .unwrap();
    cx.run_until_parked();
    assert!(cx.debug_bounds("state-on").is_none());
    assert!(cx.debug_bounds("title-field").is_some());
}
