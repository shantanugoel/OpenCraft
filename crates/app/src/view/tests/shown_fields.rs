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
