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
            },
            Some(default("test.toggle")),
        ))
    );
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("Key 2 gets “Toggle” for Release")
    );

    // A knob with no open card: "Turn right".
    click(cx, "control-knob 2");
    click(cx, "library-0");
    assert_eq!(
        last_set(&sent).map(|(binding, _)| binding),
        Some(BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Knob, 2),
            gesture: GestureKind::TurnRight,
        })
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
    cx.update(|window, cx| view.update(cx, |view, cx| view.toggle_device_panel(window, cx)));
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
