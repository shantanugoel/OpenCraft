//! The "Library" tab (decision 0029, section 11): the groups, the search,
//! a click that sets a kind, and a click with no selected control. The
//! kinds are fake kinds with groups, as the engine sends them.

use super::*;
use crate::view::library::SELECT_FIRST;
use crate::view::status_bar::MESSAGE_TIME;

fn kind(id: &str, name: &str, group: &str, help: Option<&str>) -> KindDescription {
    KindDescription {
        kind: id.into(),
        name: name.into(),
        group: group.into(),
        help: help.map(Into::into),
        fields: vec![Field {
            key: "step".into(),
            label: "Step".into(),
            input: InputType::Integer { min: 1, max: 9 },
            default: Some(Value::Integer(5)),
        }],
        ..Default::default()
    }
}

/// Four kinds in three groups. The second group comes between two kinds
/// of the first group.
fn kinds() -> Event {
    Event::Kinds(vec![
        kind("test.open", "Open an application", "System", None),
        kind("test.media", "Media key", "Media", Some("Accessibility")),
        kind("test.text", "Type text", "System", Some("Accessibility")),
        kind("test.toggle", "Toggle", "Logic", None),
    ])
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

/// The window with the kinds, on the tab "Library".
fn library(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, kinds());
    click(cx, "tab-library");
    (view, cx, tx, sent)
}

fn search(view: &Entity<DeviceView>, cx: &mut VisualTestContext, text: &str) {
    let input = cx.update(|_, cx| view.read(cx).library.search.clone());
    cx.update(|window, cx| {
        input.update(cx, |input, cx| input.focus(window, cx));
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
    });
    cx.run_until_parked();
    if text.is_empty() {
        cx.update(|window, cx| {
            window.dispatch_action(Box::new(gpui_kit::component::input::Backspace), cx)
        });
    } else {
        cx.simulate_input(text);
    }
    cx.run_until_parked();
}

fn top(cx: &mut VisualTestContext, id: &'static str) -> gpui_kit::Pixels {
    cx.debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"))
        .origin
        .y
}

fn shows(cx: &mut VisualTestContext, id: &'static str) -> bool {
    cx.debug_bounds(id).is_some()
}

/// The last binding command.
fn last_set(sent: &Commands) -> Option<(BindingId, Option<Action>)> {
    sent.borrow().iter().rev().find_map(|c| match c {
        Command::SetBinding { binding, action } => Some((*binding, action.clone())),
        _ => None,
    })
}

fn message(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| view.read(cx).status_message.as_ref().map(|m| m.to_string()))
}

#[gpui_kit::test]
fn the_library_shows_the_groups_in_the_order_of_their_first_kind(cx: &mut TestAppContext) {
    let (_, cx, _, _) = library(cx);
    // The groups: System (kinds 0 and 2), Media (kind 1), Logic (kind 3).
    let order = [
        "library-group-0",
        "library-0",
        "library-2",
        "library-group-1",
        "library-1",
        "library-group-2",
        "library-3",
    ];
    for pair in order.windows(2) {
        assert!(
            top(cx, pair[0]) < top(cx, pair[1]),
            "{} is not above {}",
            pair[0],
            pair[1]
        );
    }
    assert!(!shows(cx, "library-group-3"));
    assert!(!shows(cx, "library-no-result"));
}

#[gpui_kit::test]
fn the_search_filters_the_kinds_of_all_groups(cx: &mut TestAppContext) {
    let (view, cx, _, _) = library(cx);
    // No difference between capital and small letters. A group with no
    // kind does not show.
    search(&view, cx, "KEY");
    assert!(shows(cx, "library-1"));
    for id in ["library-0", "library-2", "library-3", "library-group-1"] {
        assert!(!shows(cx, id), "{id} shows");
    }
    assert!(shows(cx, "library-group-0"), "The Media group shows first");

    // The text matches in all groups.
    search(&view, cx, "E");
    for id in ["library-0", "library-1", "library-2", "library-3"] {
        assert!(shows(cx, id), "{id} does not show");
    }

    // No result.
    search(&view, cx, "zzz");
    assert!(shows(cx, "library-no-result"));
    assert!(!shows(cx, "library-group-0"));

    // An empty field shows all kinds again.
    search(&view, cx, "");
    assert!(!shows(cx, "library-no-result"));
    assert!(shows(cx, "library-3"));
}

#[gpui_kit::test]
fn a_click_sets_the_kind_on_the_open_card_or_on_the_first_gesture(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = library(cx);
    let default = |kind: &str| Action {
        kind: kind.into(),
        settings: Settings::from([("step".into(), Value::Integer(5))]),
    };
    let key = ControlId::new(ControlKind::Key, 2);

    // Key 2 has no open card: the click uses "Press" and opens that card.
    click(cx, "surface-key face 2");
    click(cx, "library-1");
    assert_eq!(
        last_set(&sent),
        Some((
            BindingId {
                page: 0,
                control: key,
                gesture: GestureKind::Press,
                folder: None,
            },
            Some(default("test.media")),
        ))
    );
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("Key 2 gets “Media key” for Press")
    );
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.state.gesture, Some(GestureKind::Press));
        assert_eq!(
            view.panes.right_panel_tab,
            crate::settings::RightPanelTab::Library
        );
    });

    // With the card "Release" open, the click sets the kind on "Release".
    view.update(cx, |view, _| {
        view.state.open_card(Some(GestureKind::Release))
    });
    click(cx, "library-3");
    assert_eq!(
        last_set(&sent),
        Some((
            BindingId {
                page: 0,
                control: key,
                gesture: GestureKind::Release,
                folder: None,
            },
            Some(default("test.toggle")),
        ))
    );
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("Key 2 gets “Toggle” for Release")
    );

    // A knob with no open card: "Turn", which writes the two turn
    // bindings in one group (decision 0029, section 10.4).
    click(cx, "control-knob 2");
    click(cx, "library-0");
    let Some(Command::Group { name, commands }) = sent.borrow().last().cloned() else {
        panic!("expected a group")
    };
    assert_eq!(name, "Set Action");
    let knob = |gesture| BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Knob, 2),
        gesture,
        folder: None,
    };
    assert!(matches!(
        commands.as_slice(),
        [
            Command::SetBinding { binding: right, action: Some(_) },
            Command::SetBinding { binding: left, action: Some(_) },
        ] if *right == knob(GestureKind::TurnRight) && *left == knob(GestureKind::TurnLeft)
    ));
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("Knob 2 gets “Open an application” for Turn")
    );

    // The message goes after 4 seconds, and the last input shows again.
    cx.executor().advance_clock(MESSAGE_TIME);
    cx.run_until_parked();
    assert_eq!(message(&view, cx), None);
}

#[gpui_kit::test]
fn a_click_with_no_selected_control_only_shows_a_message(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = library(cx);
    // The page is the selection.
    cx.update(|window, cx| view.update(cx, |view, cx| view.select_page(window, cx)));
    click(cx, "library-2");
    assert!(last_set(&sent).is_none());
    assert_eq!(message(&view, cx).as_deref(), Some(SELECT_FIRST));

    // The message goes after 4 seconds.
    cx.executor().advance_clock(MESSAGE_TIME);
    cx.run_until_parked();
    assert_eq!(message(&view, cx), None);

    // The device is the selection, and a key was selected before.
    click(cx, "surface-key face 2");
    cx.update(|window, cx| view.update(cx, |view, cx| view.select_device(window, cx)));
    click(cx, "library-2");
    assert!(last_set(&sent).is_none());
    assert_eq!(message(&view, cx).as_deref(), Some(SELECT_FIRST));
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).panes.right_panel_tab,
            crate::settings::RightPanelTab::Library
        )
    });
}

#[gpui_kit::test]
fn the_kind_menu_of_a_card_has_a_title_for_each_group(cx: &mut TestAppContext) {
    let (_, cx, _, sent) = library(cx);
    click(cx, "surface-key face 2");
    click(cx, "tab-inspector");
    click(cx, "gesture-Press-kind");
    let order = [
        "kind-group-0",
        "kind-0",
        "kind-2",
        "kind-group-1",
        "kind-1",
        "kind-group-2",
        "kind-3",
    ];
    for pair in order.windows(2) {
        assert!(
            top(cx, pair[0]) < top(cx, pair[1]),
            "{} is not above {}",
            pair[0],
            pair[1]
        );
    }
    click(cx, "kind-2");
    assert_eq!(
        last_set(&sent)
            .and_then(|(_, action)| action)
            .map(|a| a.kind),
        Some("test.text".into())
    );
}

// MARK: Drag and drop (decision 0029, section 12)

/// Press on the library row `from` and move the pointer to `to` in eight
/// steps, with the left button down. The drag is then active.
fn drag_over(
    cx: &mut VisualTestContext,
    from: &'static str,
    to: gpui_kit::Point<gpui_kit::Pixels>,
) {
    let start = cx
        .debug_bounds(from)
        .unwrap_or_else(|| panic!("No element: {from}"))
        .center();
    cx.simulate_mouse_move(start, None, Modifiers::default());
    cx.simulate_mouse_down(start, gpui_kit::MouseButton::Left, Modifiers::default());
    for step in 1..=8 {
        let f = step as f32 / 8.;
        cx.simulate_mouse_move(
            gpui_kit::point(
                start.x + (to.x - start.x) * f,
                start.y + (to.y - start.y) * f,
            ),
            Some(gpui_kit::MouseButton::Left),
            Modifiers::default(),
        );
    }
    cx.run_until_parked();
}

/// Release the left button at `to`: the drop.
fn release(cx: &mut VisualTestContext, to: gpui_kit::Point<gpui_kit::Pixels>) {
    cx.simulate_mouse_up(to, gpui_kit::MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
}

/// A full drag from the library row `from` to the centre of `to`.
fn drag_to(cx: &mut VisualTestContext, from: &'static str, to: &'static str) {
    let end = cx
        .debug_bounds(to)
        .unwrap_or_else(|| panic!("No element: {to}"))
        .center();
    drag_over(cx, from, end);
    release(cx, end);
}

/// The name and the commands of the last command, a group.
fn last_group(sent: &Commands) -> (String, Vec<Command>) {
    match sent.borrow().last().cloned() {
        Some(Command::Group { name, commands }) => (name, commands),
        other => panic!("expected a group, got {other:?}"),
    }
}

fn default_action(kind: &str) -> Action {
    Action {
        kind: kind.into(),
        settings: Settings::from([("step".into(), Value::Integer(5))]),
    }
}

#[gpui_kit::test]
fn a_drag_shows_the_drop_target_and_the_row_in_the_drag(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = library(cx);
    assert!(!shows(cx, "drop-key face 2"));
    let key = cx.debug_bounds("surface-key face 2").unwrap().center();
    drag_over(cx, "library-1", key);
    assert!(cx.update(|_, cx| cx.has_active_drag()));
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert!(view.kind_drag(cx));
        assert_eq!(view.library.dragging.as_deref(), Some("test.media"));
    });
    // Each control has a drop target, which shows only under the pointer.
    for id in ["drop-key face 2", "drop-bar panel 3", "drop-knob 2"] {
        assert!(shows(cx, id), "{id} does not show");
    }
    assert!(sent.borrow().is_empty(), "a drag sends nothing");
    release(cx, key);
    assert!(!cx.update(|_, cx| cx.has_active_drag()));
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert!(!view.kind_drag(cx));
        assert_eq!(view.library.dragging, None);
    });
    assert!(!shows(cx, "drop-key face 2"));
}

#[gpui_kit::test]
fn a_drop_on_a_key_sets_press_as_one_step_and_selects_the_key(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = library(cx);
    // The bar panel is the selection before the drop.
    click(cx, "surface-bar panel 3");
    drag_to(cx, "library-1", "surface-key face 2");
    let press = BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Key, 2),
        gesture: GestureKind::Press,
        folder: None,
    };
    let (name, commands) = last_group(&sent);
    assert_eq!(name, "Drop Action");
    assert_eq!(
        commands,
        [Command::SetBinding {
            binding: press,
            action: Some(default_action("test.media")),
        }]
    );
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("Key 2 gets “Media key” for Press")
    );
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.state.selected, Some(crate::state::tests::key()));
        assert_eq!(view.state.gesture, Some(GestureKind::Press));
        // The right panel stays on its tab.
        assert_eq!(
            view.panes.right_panel_tab,
            crate::settings::RightPanelTab::Library
        );
    });
}

#[gpui_kit::test]
fn a_drop_replaces_the_action_of_the_first_gesture(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = library(cx);
    let key = ControlId::new(ControlKind::Key, 2);
    let binding = |gesture| BindingId {
        page: 0,
        control: key,
        gesture,
        folder: None,
    };
    for gesture in [GestureKind::Press, GestureKind::Release] {
        send(
            cx,
            &tx,
            Event::Binding(BindingState {
                binding: binding(gesture),
                action: Some(default_action("test.open")),
                available: true,
            }),
        );
    }
    // The card "Release" is open: a click would use it, a drop does not.
    click(cx, "surface-key face 2");
    view.update(cx, |view, _| {
        view.state.open_card(Some(GestureKind::Release))
    });
    drag_to(cx, "library-3", "surface-key face 2");
    let (name, commands) = last_group(&sent);
    assert_eq!(name, "Drop Action");
    assert_eq!(
        commands,
        [Command::SetBinding {
            binding: binding(GestureKind::Press),
            action: Some(default_action("test.toggle")),
        }]
    );
    cx.update(|_, cx| assert_eq!(view.read(cx).state.gesture, Some(GestureKind::Press)));
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("Key 2 gets “Toggle” for Press")
    );
}

#[gpui_kit::test]
fn a_drop_on_a_bar_panel_sets_tap(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = library(cx);
    drag_to(cx, "library-0", "surface-bar panel 3");
    let (name, commands) = last_group(&sent);
    assert_eq!(name, "Drop Action");
    assert_eq!(
        commands,
        [Command::SetBinding {
            binding: BindingId {
                page: 0,
                control: ControlId::new(ControlKind::BarPanel, 2),
                gesture: GestureKind::Tap,
                folder: None,
            },
            action: Some(default_action("test.open")),
        }]
    );
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("Bar panel 2 gets “Open an application” for Tap")
    );
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.state.selected, Some(crate::state::tests::bar()));
        assert_eq!(view.state.gesture, Some(GestureKind::Tap));
    });
}

#[gpui_kit::test]
fn a_drop_on_a_knob_sets_turn_as_one_step(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = library(cx);
    let knob = ControlId::new(ControlKind::Knob, 2);
    // The card "Push" is open: the drop uses "Turn" all the same.
    click(cx, "control-knob 2");
    view.update(cx, |view, _| view.state.open_card(Some(GestureKind::Push)));
    drag_to(cx, "library-2", "control-knob 2");
    let (name, commands) = last_group(&sent);
    assert_eq!(name, "Drop Action");
    let turn = |gesture| BindingId {
        page: 0,
        control: knob,
        gesture,
        folder: None,
    };
    // The same commands as a click with no open card: the two turn
    // bindings, "Turn right" first.
    assert!(
        matches!(
            commands.as_slice(),
            [
                Command::SetBinding { binding: right, action: Some(r) },
                Command::SetBinding { binding: left, action: Some(l) },
            ] if *right == turn(GestureKind::TurnRight)
                && *left == turn(GestureKind::TurnLeft)
                && r.kind == "test.text"
                && l.kind == "test.text"
        ),
        "{commands:?}"
    );
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("Knob 2 gets “Type text” for Turn")
    );
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.state.selected_control, Some(knob));
        assert_eq!(view.state.gesture, Some(GestureKind::TurnRight));
    });
}

#[gpui_kit::test]
fn a_drop_outside_a_control_does_nothing(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = library(cx);
    click(cx, "surface-key face 2");
    // Outside the device, on the empty part of the display, and on the
    // body of the device.
    let display = cx.debug_bounds("display").unwrap();
    let body = cx.debug_bounds("device-body").unwrap();
    let header = cx.debug_bounds("page-header").unwrap().center();
    for point in [
        header,
        display.origin + gpui_kit::point(px(4.), px(4.)),
        body.origin + gpui_kit::point(px(6.), body.size.height - px(6.)),
    ] {
        drag_over(cx, "library-1", point);
        assert!(cx.update(|_, cx| view.read(cx).kind_drag(cx)));
        release(cx, point);
        cx.update(|_, cx| assert_eq!(view.read(cx).library.dragging, None));
    }
    assert!(sent.borrow().is_empty(), "{:?}", sent.borrow());
    assert_eq!(message(&view, cx), None);
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.state.selected, Some(crate::state::tests::key()));
        assert!(!view.device.open);
    });
}

#[gpui_kit::test]
fn a_short_move_is_a_click_and_not_a_drag(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = library(cx);
    click(cx, "surface-key face 2");
    let start = cx.debug_bounds("library-1").unwrap().center();
    cx.simulate_mouse_move(start, None, Modifiers::default());
    cx.simulate_mouse_down(start, gpui_kit::MouseButton::Left, Modifiers::default());
    let end = start + gpui_kit::point(px(2.), px(0.));
    cx.simulate_mouse_move(end, Some(gpui_kit::MouseButton::Left), Modifiers::default());
    assert!(!cx.update(|_, cx| cx.has_active_drag()));
    release(cx, end);
    // A click is a step "Set Action" of the engine, not a group.
    assert_eq!(
        last_set(&sent).map(|(_, a)| a),
        Some(Some(default_action("test.media")))
    );
    assert!(!matches!(sent.borrow().last(), Some(Command::Group { .. })));
    cx.update(|_, cx| assert_eq!(view.read(cx).library.dragging, None));
}
