//! A field is a row only for the operations that use it (decision 0038).
use super::*;

fn press() -> BindingId {
    BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Key, 2),
        gesture: GestureKind::Press,
        folder: None,
    }
}

fn show(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, action: Action) {
    let kind = opencraft_actions::handlers()
        .into_iter()
        .flat_map(|handler| handler.kinds())
        .find(|kind| kind.kind == action.kind)
        .unwrap_or_else(|| panic!("no kind {}", action.kind));
    tx.try_send(Event::Kinds(vec![kind])).unwrap();
    tx.try_send(Event::Binding(BindingState {
        binding: press(),
        action: Some(action),
        available: true,
        inert_by: None,
    }))
    .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
}

fn binding(action: Action) -> Command {
    Command::SetBinding {
        binding: press(),
        action: Some(action),
    }
}

#[gpui_kit::test]
fn sound_volume_hides_the_step_for_mute_and_does_not_store_it(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    show(
        cx,
        &tx,
        Action {
            kind: "opencraft.volume".into(),
            settings: Settings::from([
                ("operation".into(), Value::Text("up".into())),
                ("step".into(), Value::Integer(12)),
            ]),
        },
    );
    assert!(cx.debug_bounds("field-1").is_some(), "Up shows Step");
    click(cx, "choice-0");
    click(cx, "choice-0-2");
    assert_eq!(
        sent.borrow().last(),
        Some(&binding(Action {
            kind: "opencraft.volume".into(),
            settings: Settings::from([("operation".into(), Value::Text("mute".into()))]),
        }))
    );
    assert!(
        cx.debug_bounds("field-1").is_none(),
        "Mute does not show Step"
    );
    click(cx, "choice-0");
    click(cx, "choice-0-0");
    assert_eq!(
        sent.borrow().last(),
        Some(&binding(Action {
            kind: "opencraft.volume".into(),
            settings: Settings::from([
                ("operation".into(), Value::Text("up".into())),
                ("step".into(), Value::Integer(5)),
            ]),
        }))
    );
    assert!(cx.debug_bounds("field-1").is_some());
}

#[gpui_kit::test]
fn brightness_shows_the_step_only_for_up_and_down(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    show(
        cx,
        &tx,
        Action {
            kind: "opencraft.device-brightness".into(),
            settings: Settings::from([
                ("operation".into(), Value::Text("up".into())),
                ("value".into(), Value::Integer(35)),
                ("step".into(), Value::Integer(9)),
            ]),
        },
    );
    assert!(cx.debug_bounds("field-1").is_none(), "Up hides Brightness");
    assert!(cx.debug_bounds("field-2").is_some(), "Up shows Step");
    click(cx, "choice-0");
    click(cx, "choice-0-2");
    assert_eq!(
        sent.borrow().last(),
        Some(&binding(Action {
            kind: "opencraft.device-brightness".into(),
            settings: Settings::from([
                ("operation".into(), Value::Text("set".into())),
                ("value".into(), Value::Integer(35)),
            ]),
        }))
    );
    assert!(cx.debug_bounds("field-1").is_some());
    assert!(cx.debug_bounds("field-2").is_none());
}

#[gpui_kit::test]
fn a_page_number_shows_only_for_that_choice(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    show(
        cx,
        &tx,
        Action {
            kind: "opencraft.page".into(),
            settings: Settings::from([
                ("page".into(), Value::Integer(4)),
                ("target".into(), Value::Text("number".into())),
            ]),
        },
    );
    assert!(cx.debug_bounds("field-1").is_some());
    click(cx, "choice-0");
    click(cx, "choice-0-0");
    assert_eq!(
        sent.borrow().last(),
        Some(&binding(Action {
            kind: "opencraft.page".into(),
            settings: Settings::from([("target".into(), Value::Text("next".into()))]),
        }))
    );
    assert!(cx.debug_bounds("field-1").is_none());
    click(cx, "choice-0");
    click(cx, "choice-0-2");
    assert_eq!(
        sent.borrow().last(),
        Some(&binding(Action {
            kind: "opencraft.page".into(),
            settings: Settings::from([
                ("page".into(), Value::Integer(1)),
                ("target".into(), Value::Text("number".into())),
            ]),
        }))
    );
    assert!(cx.debug_bounds("field-1").is_some());
}

#[gpui_kit::test]
fn sound_output_shows_the_device_or_the_cycle_list_for_its_operation(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    show(
        cx,
        &tx,
        Action {
            kind: "opencraft.sound-output".into(),
            settings: Settings::from([
                ("operation".into(), Value::Text("set".into())),
                ("device".into(), Value::Text("a".into())),
            ]),
        },
    );
    assert!(cx.debug_bounds("field-1").is_some(), "Set shows Device");
    assert!(cx.debug_bounds("field-2").is_none(), "Set hides Cycle list");
    click(cx, "choice-0");
    click(cx, "choice-0-1");
    assert_eq!(
        sent.borrow().last(),
        Some(&binding(Action {
            kind: "opencraft.sound-output".into(),
            settings: Settings::from([
                ("operation".into(), Value::Text("next".into())),
                ("devices".into(), Value::List(Vec::new())),
            ]),
        }))
    );
    assert!(cx.debug_bounds("field-1").is_none(), "Next hides Device");
    assert!(
        cx.debug_bounds("field-2").is_some(),
        "Next shows Cycle list"
    );
}

#[test]
fn a_new_mouse_event_on_a_knob_stores_the_step_and_not_the_hidden_button() {
    let kinds: Vec<_> = opencraft_actions::handlers()
        .into_iter()
        .flat_map(|handler| handler.kinds())
        .collect();
    let kind = kinds
        .iter()
        .find(|kind| kind.kind == "opencraft.mouse")
        .expect("the kind Mouse event");
    // The default action is a click: it has a button and no step.
    let mut settings: Settings = kind
        .fields
        .iter()
        .filter_map(|field| field.default.clone().map(|v| (field.key.clone(), v)))
        .collect();
    crate::view::actions::apply_shown(&kind.fields, &kind.shown_when, &mut settings);
    assert!(settings.contains_key("button"));
    assert!(!settings.contains_key("step"));
    let click = Action {
        kind: kind.kind.clone(),
        settings,
    };
    let (right, left) = crate::view::knob::turn_actions(&click, &kinds, false);
    for (action, operation) in [(&right, "scroll-down"), (&left, "scroll-up")] {
        assert_eq!(
            action.settings.get("operation"),
            Some(&Value::Text(operation.into()))
        );
        assert_eq!(action.settings.get("step"), Some(&Value::Integer(3)));
        for hidden in ["button", "x", "y"] {
            assert!(!action.settings.contains_key(hidden), "{hidden}");
        }
    }
}

#[gpui_kit::test]
fn select_a_profile_shows_the_profile_only_for_a_profile(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    show(
        cx,
        &tx,
        Action {
            kind: "opencraft.profile".into(),
            settings: Settings::from([
                ("target".into(), Value::Text("profile".into())),
                ("profile".into(), Value::Text("Work".into())),
            ]),
        },
    );
    assert!(cx.debug_bounds("field-1").is_some(), "A profile shows it");
    // The choices are "A profile", "Next profile", and "Previous profile".
    click(cx, "choice-0");
    click(cx, "choice-0-1");
    assert_eq!(
        sent.borrow().last(),
        Some(&binding(Action {
            kind: "opencraft.profile".into(),
            settings: Settings::from([("target".into(), Value::Text("next".into()))]),
        }))
    );
    assert!(
        cx.debug_bounds("field-1").is_none(),
        "Next profile does not show the profile"
    );
    assert!(cx.debug_bounds("field-2").is_some(), "The page stays");
    click(cx, "choice-0");
    click(cx, "choice-0-0");
    assert!(cx.debug_bounds("field-1").is_some());
}
