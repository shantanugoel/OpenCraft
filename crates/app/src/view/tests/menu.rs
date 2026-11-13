//! The keys of the menus that the window draws (issue 217): the kind menu
//! of a gesture card and the menu of a choice field, with fake kinds.

use super::*;

fn kind(id: &str, name: &str, group: &str) -> KindDescription {
    KindDescription {
        kind: id.into(),
        name: name.into(),
        group: group.into(),
        fields: vec![Field {
            key: "step".into(),
            label: "Step".into(),
            input: InputType::Integer { min: 1, max: 9 },
            default: Some(Value::Integer(5)),
        }],
        ..Default::default()
    }
}

/// Four kinds in three groups. The order of the menu is "Open an
/// application", "Type text" (group System), "Media key", "Toggle". It is not
/// the order of the list.
fn kinds() -> Event {
    Event::Kinds(vec![
        kind("test.open", "Open an application", "System"),
        kind("test.media", "Media key", "Media"),
        kind("test.text", "Type text", "System"),
        kind("test.toggle", "Toggle", "Logic"),
    ])
}

/// A kind with a choice of 40 options.
fn pick() -> KindDescription {
    KindDescription {
        kind: "test.pick".into(),
        name: "Pick".into(),
        fields: vec![Field {
            key: "pick".into(),
            label: "Pick".into(),
            input: InputType::Choice {
                options: (0..40)
                    .map(|n| ChoiceOption {
                        value: format!("o{n}"),
                        label: format!("Option {n}"),
                    })
                    .collect(),
            },
            default: Some(Value::Text("o0".into())),
        }],
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

/// The command that sets the kind with its default setting.
fn set(gesture: GestureKind, kind: &str) -> Command {
    Command::SetBinding {
        binding: binding(gesture),
        action: Some(Action {
            kind: kind.into(),
            settings: Settings::from([("step".to_string(), Value::Integer(5))]),
        }),
    }
}

fn start(
    cx: &mut TestAppContext,
) -> (
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (_view, cx, tx, sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(1000.)));
    tx.try_send(kinds()).unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    (cx, tx, sent)
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

/// The identifier of the nth row of the kind menu, in the order of the
/// menu: the order of the screen from the top.
fn nth_row(cx: &mut VisualTestContext, n: usize) -> &'static str {
    let mut rows: Vec<_> = ["kind-0", "kind-1", "kind-2", "kind-3"]
        .into_iter()
        .map(|id| (cx.debug_bounds(id).unwrap().top(), id))
        .collect();
    rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    rows[n].1
}

#[gpui_kit::test]
fn the_arrow_keys_and_return_select_an_item_of_the_kind_menu(cx: &mut TestAppContext) {
    let (cx, _tx, sent) = start(cx);
    click(cx, "gesture-Release-kind");
    assert!(cx.debug_bounds("kind-menu").is_some());
    // The first item is highlighted at the start: Down two times is the
    // third item. The titles of the groups are not items.
    assert_eq!(nth_row(cx, 2), "kind-1");
    keys(cx, "down down enter");
    assert!(cx.debug_bounds("kind-menu").is_none());
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Release, "test.media"))
    );

    // Return at the start selects the first item.
    click(cx, "gesture-Press-kind");
    keys(cx, "enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Press, "test.open"))
    );

    // Up at the first item goes to the last item, and Down at the last item
    // goes to the first item.
    click(cx, "gesture-Release-kind");
    keys(cx, "up enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Release, "test.toggle"))
    );
    click(cx, "gesture-Press-kind");
    keys(cx, "end down enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Press, "test.open"))
    );

    // Home and End.
    click(cx, "gesture-Release-kind");
    keys(cx, "down down home enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Release, "test.open"))
    );
    click(cx, "gesture-Press-kind");
    keys(cx, "end enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Press, "test.toggle"))
    );
}

#[gpui_kit::test]
fn escape_closes_the_menu_and_keys_with_no_menu_do_nothing(cx: &mut TestAppContext) {
    let (cx, _tx, sent) = start(cx);
    keys(cx, "down enter");
    assert!(sent.borrow().is_empty());
    click(cx, "gesture-Release-kind");
    keys(cx, "down escape");
    assert!(cx.debug_bounds("kind-menu").is_none());
    keys(cx, "enter");
    assert!(sent.borrow().is_empty());
    // A key with a modifier is not a key of the menu.
    click(cx, "gesture-Release-kind");
    keys(cx, "cmd-down alt-enter");
    assert!(cx.debug_bounds("kind-menu").is_some());
    assert!(sent.borrow().is_empty());
}

#[gpui_kit::test]
fn a_letter_key_goes_to_the_first_item_that_starts_with_it(cx: &mut TestAppContext) {
    let (cx, _tx, sent) = start(cx);
    click(cx, "gesture-Release-kind");
    // "T" is "Type text" and "Toggle": the first in the menu is "Type text".
    keys(cx, "t enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Release, "test.text"))
    );
    // Two letters are one text: "to" is "Toggle". The case does not matter.
    click(cx, "gesture-Press-kind");
    keys(cx, "T o enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Press, "test.toggle"))
    );
    // A letter with no item keeps the highlight.
    click(cx, "gesture-Release-kind");
    keys(cx, "down z enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Release, "test.text"))
    );
}

#[gpui_kit::test]
fn the_pointer_moves_the_highlight(cx: &mut TestAppContext) {
    let (cx, _tx, sent) = start(cx);
    click(cx, "gesture-Release-kind");
    let row = cx.debug_bounds("kind-3").unwrap().center();
    cx.simulate_mouse_move(row, None, Modifiers::default());
    cx.run_until_parked();
    // Up from the item under the pointer goes to the item before it.
    keys(cx, "up enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&set(GestureKind::Release, "test.media"))
    );
}

#[gpui_kit::test]
fn a_long_choice_menu_scrolls_to_the_highlighted_item(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    tx.try_send(Event::Kinds(vec![pick()])).unwrap();
    tx.try_send(Event::Binding(BindingState {
        binding: binding(GestureKind::Press),
        action: Some(Action {
            kind: "test.pick".into(),
            settings: Settings::from([("pick".to_string(), Value::Text("o0".into()))]),
        }),
        available: true,
    }))
    .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    click(cx, "choice-0");
    let menu = cx.debug_bounds("choice-0-menu").unwrap();
    let row = |cx: &mut VisualTestContext, n: usize| {
        let id: &'static str = Box::leak(format!("choice-0-{n}").into_boxed_str());
        cx.debug_bounds(id).unwrap()
    };
    let inside = |cx: &mut VisualTestContext, n: usize| {
        let r = row(cx, n);
        r.top() >= menu.top() && r.bottom() <= menu.bottom()
    };
    assert!(inside(cx, 0));
    assert!(!inside(cx, 39));
    // End shows the last item. Home shows the first item again. Each Down
    // key keeps the highlighted item visible.
    keys(cx, "end");
    assert!(inside(cx, 39));
    keys(cx, "home");
    assert!(inside(cx, 0));
    for n in 1..=30 {
        keys(cx, "down");
        assert!(inside(cx, n), "Option {n} is not visible");
    }
    for n in (10..30).rev() {
        keys(cx, "up");
        assert!(inside(cx, n), "Option {n} is not visible");
    }
    // Escape closes the menu, and the setting does not change.
    let count = sent.borrow().len();
    keys(cx, "escape");
    assert!(cx.debug_bounds("choice-0-menu").is_none());
    assert_eq!(sent.borrow().len(), count);
    click(cx, "choice-0");
    keys(
        cx,
        "down down down down down down down down down down enter",
    );
    assert!(cx.debug_bounds("choice-0-menu").is_none());
    let sent = sent.borrow();
    let Some(Command::SetBinding {
        action: Some(action),
        ..
    }) = sent.last()
    else {
        panic!("No command: {sent:?}");
    };
    assert_eq!(
        action.settings.get("pick"),
        Some(&Value::Text("o10".into()))
    );
}
