//! The page tabs, the profile menu, the touch gestures, and stale edits.

use super::actions::{input_text, top};
use super::*;
use opencraft_engine::ProfileEntry;

fn pages(names: &[&str], active: usize) -> Event {
    Event::Pages {
        names: names.iter().map(|n| (*n).into()).collect(),
        active,
    }
}

fn profiles(entries: &[(&str, &str)], active: &str) -> Event {
    Event::Profiles {
        profiles: entries
            .iter()
            .map(|(id, name)| ProfileEntry {
                id: (*id).into(),
                name: (*name).into(),
            })
            .collect(),
        active: active.into(),
    }
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

fn test_kind() -> KindDescription {
    KindDescription {
        kind: "test.kind".into(),
        name: "Test action".into(),
        fields: vec![
            Field {
                key: "text".into(),
                label: "Text".into(),
                input: InputType::Text,
                default: Some(Value::Text("start".into())),
            },
            Field {
                key: "file".into(),
                label: "File".into(),
                input: InputType::Path {
                    kind: PathKind::File,
                },
                default: None,
            },
        ],
    }
}

fn test_action(text: &str) -> Action {
    Action {
        kind: "test.kind".into(),
        settings: Settings::from([("text".into(), Value::Text(text.into()))]),
    }
}

fn key_press(page: usize) -> BindingId {
    BindingId {
        page,
        control: ControlId::new(ControlKind::Key, 2),
        gesture: GestureKind::Press,
    }
}

fn bound(id: BindingId, action: Action) -> Event {
    Event::Binding(BindingState {
        binding: id,
        action: Some(action),
        available: true,
    })
}

#[gpui_kit::test]
fn page_tabs_send_each_page_command_and_follow_the_engine(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, pages(&["Home", "Media", "Work"], 0));
    for id in ["page-0", "page-1", "page-2", "page-add"] {
        assert!(cx.debug_bounds(id).is_some(), "{id}");
    }
    click(cx, "page-1");
    assert_eq!(sent.borrow().last(), Some(&Command::SelectPage { page: 1 }));
    // The tabs show the page of the engine, not the click.
    cx.update(|_, cx| assert_eq!(view.read(cx).state.page, 0));
    // An action on the device changes the page: the tabs follow.
    send(cx, &tx, pages(&["Home", "Media", "Work"], 1));
    cx.update(|_, cx| {
        assert_eq!(view.read(cx).state.page, 1);
        assert_eq!(view.read(cx).state.pages, ["Home", "Media", "Work"]);
    });
    let count = sent.borrow().len();
    click(cx, "page-1");
    assert_eq!(sent.borrow().len(), count, "The active page is selected");

    click(cx, "page-add");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::AddPage { copy_of: None })
    );
    click(cx, "page-duplicate");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::AddPage { copy_of: Some(1) })
    );
    click(cx, "page-left");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::MovePage { page: 1, to: 0 })
    );
    click(cx, "page-right");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::MovePage { page: 1, to: 2 })
    );

    click(cx, "page-rename");
    assert!(cx.debug_bounds("page-name").is_some());
    assert!(cx.debug_bounds("page-1").is_none());
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).lists.page_name.read(cx).value().as_ref(),
            "Media"
        )
    });
    cx.simulate_input("Games");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::RenamePage {
            page: 1,
            name: "Games".into()
        })
    );
    assert!(cx.debug_bounds("page-name").is_none());
    let count = sent.borrow().len();
    click(cx, "page-rename");
    cx.simulate_input("Other");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("page-name").is_none());
    click(cx, "page-rename");
    click(cx, "page-name-cancel");
    assert!(cx.debug_bounds("page-name").is_none());
    assert_eq!(
        sent.borrow().len(),
        count,
        "A cancelled rename sends nothing"
    );

    // The removal needs a confirmation.
    click(cx, "page-remove");
    assert_eq!(sent.borrow().len(), count);
    click(cx, "page-remove-cancel");
    assert!(cx.debug_bounds("page-remove-confirm").is_none());
    assert_eq!(sent.borrow().len(), count);
    click(cx, "page-remove");
    click(cx, "page-remove-confirm");
    assert_eq!(sent.borrow().last(), Some(&Command::RemovePage { page: 1 }));
    // A page change stops a confirmation: the number can be a different page.
    click(cx, "page-remove");
    send(cx, &tx, pages(&["Home", "Media", "Work"], 2));
    assert!(cx.debug_bounds("page-remove-confirm").is_none());

    // The first page has no move to the left, and the last page cannot go.
    send(cx, &tx, pages(&["Home"], 0));
    let count = sent.borrow().len();
    for id in ["page-left", "page-right", "page-remove"] {
        click(cx, id);
    }
    assert!(cx.debug_bounds("page-remove-confirm").is_none());
    assert_eq!(sent.borrow().len(), count);
}

#[gpui_kit::test]
fn profile_menu_sends_each_profile_command_and_follows_the_engine(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    let list = [("default", "Default"), ("work", "Work")];
    send(cx, &tx, profiles(&list, "default"));
    assert!(cx.debug_bounds("profiles").is_none());
    click(cx, "profile-menu");
    assert!(cx.debug_bounds("profiles").is_some());
    click(cx, "profile-1");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SelectProfile {
            profile: "work".into()
        })
    );
    assert!(cx.debug_bounds("profiles").is_none());
    cx.update(|_, cx| assert_eq!(view.read(cx).state.profile.as_deref(), Some("default")));
    // An action on the device selects a profile: the menu follows.
    send(cx, &tx, profiles(&list, "work"));
    cx.update(|_, cx| {
        assert_eq!(view.read(cx).state.profile.as_deref(), Some("work"));
        assert_eq!(view.read(cx).state.profiles.len(), 2);
    });

    click(cx, "profile-menu");
    click(cx, "profile-add");
    assert!(cx.debug_bounds("profile-name").is_some());
    let count = sent.borrow().len();
    // An empty name sends nothing.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), count);
    cx.simulate_input("Games");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::AddProfile {
            name: "Games".into(),
            copy_of: None
        })
    );
    assert!(cx.debug_bounds("profiles").is_none());

    click(cx, "profile-menu");
    click(cx, "profile-duplicate");
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).lists.profile_name.read(cx).value().as_ref(),
            "Work copy"
        )
    });
    click(cx, "profile-name-save");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::AddProfile {
            name: "Work copy".into(),
            copy_of: Some("work".into())
        })
    );

    click(cx, "profile-menu");
    click(cx, "profile-rename");
    cx.simulate_input("Office");
    click(cx, "profile-name-save");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::RenameProfile {
            profile: "work".into(),
            name: "Office".into()
        })
    );
    // The menu stays open after a rename, and the list follows the engine.
    assert!(cx.debug_bounds("profiles").is_some());
    send(
        cx,
        &tx,
        profiles(&[("default", "Default"), ("work", "Office")], "work"),
    );
    cx.update(|_, cx| assert_eq!(view.read(cx).state.profiles[1].name, "Office"));

    let count = sent.borrow().len();
    click(cx, "profile-remove");
    assert_eq!(
        sent.borrow().len(),
        count,
        "The removal needs a confirmation"
    );
    click(cx, "profile-edit-cancel");
    click(cx, "profile-remove");
    click(cx, "profile-remove-confirm");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::RemoveProfile {
            profile: "work".into()
        })
    );
    assert!(cx.debug_bounds("profiles").is_none());

    // A click outside the menu closes it and sends nothing.
    let count = sent.borrow().len();
    click(cx, "profile-menu");
    click(cx, "profile-backdrop");
    assert!(cx.debug_bounds("profiles").is_none());
    assert_eq!(sent.borrow().len(), count);

    // The last profile cannot go.
    send(cx, &tx, profiles(&[("default", "Default")], "default"));
    click(cx, "profile-menu");
    click(cx, "profile-remove");
    assert!(cx.debug_bounds("profile-remove-confirm").is_none());
    assert_eq!(sent.borrow().len(), count);
}

#[gpui_kit::test]
fn a_bar_panel_offers_tap_and_the_touch_bar_offers_the_swipes(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, Event::Kinds(vec![test_kind()]));
    click(cx, "surface-bar panel 3");
    assert!(cx.debug_bounds("gesture-Tap").is_some());
    for id in ["gesture-Press", "gesture-SwipeLeft", "gesture-TurnLeft"] {
        assert!(cx.debug_bounds(id).is_none(), "{id}");
    }
    click(cx, "action-kind");
    click(cx, "kind-0");
    let tap = BindingId {
        page: 0,
        control: ControlId::new(ControlKind::BarPanel, 2),
        gesture: GestureKind::Tap,
    };
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: tap,
            action: Some(test_action("start"))
        })
    );
    // The label, colour, and image of the bar panel stay in the editor.
    assert!(cx.debug_bounds("choose-image").is_some());

    click(cx, "control-touch bar 2");
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, None));
    assert!(cx.debug_bounds("gesture-Tap").is_none());
    assert!(cx.debug_bounds("choose-image").is_none());
    let swipe = |gesture| BindingId {
        page: 0,
        control: ControlId::new(ControlKind::TouchBar, 2),
        gesture,
    };
    click(cx, "action-kind");
    click(cx, "kind-0");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: swipe(GestureKind::SwipeLeft),
            action: Some(test_action("start"))
        })
    );
    click(cx, "gesture-SwipeRight");
    click(cx, "action-kind");
    click(cx, "kind-0");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: swipe(GestureKind::SwipeRight),
            action: Some(test_action("start"))
        })
    );
}

#[gpui_kit::test]
fn pages_and_profiles_events_drop_stale_edits(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    // A tall test viewport lets the test click each field without scrolling.
    cx.simulate_resize(size(px(1120.), px(2400.)));
    send(
        cx,
        &tx,
        profiles(&[("default", "Default"), ("work", "Work")], "default"),
    );
    send(cx, &tx, pages(&["Home", "Media"], 0));
    send(cx, &tx, Event::Kinds(vec![test_kind()]));
    send(cx, &tx, bound(key_press(0), test_action("home")));
    click(cx, "surface-key face 2");

    // A path dialog that ends after a page change sends nothing. The engine
    // removed the active page, so the same number is a different page.
    click(cx, "path-1");
    // The engine sends the surfaces of the new page before the pages.
    send(cx, &tx, surface(key(), "Media key"));
    send(cx, &tx, pages(&["Media"], 0));
    send(cx, &tx, bound(key_press(0), test_action("media")));
    let count = sent.borrow().len();
    cx.simulate_path_prompt_response(|_| Some(vec!["/tmp/late".into()]));
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), count);

    // The form and the label show the new page.
    assert_eq!(
        cx.update(|_, cx| view.read(cx).input.read(cx).value().to_string()),
        "Media key"
    );
    let text = input_text(&view, cx, &top("text"));
    assert_eq!(text, "media");

    // An image dialog that ends after a page change sends nothing.
    click(cx, "choose-image");
    send(cx, &tx, pages(&["Media", "Page 2"], 1));
    cx.simulate_path_prompt_response(|_| Some(vec!["/tmp/late.png".into()]));
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), count);

    // A text edit in flight is gone after a change of the profile: the new
    // profile has a binding with the same identity.
    send(cx, &tx, bound(key_press(1), test_action("page two")));
    let input = view.update(cx, |view, _| view.form.inputs[&top("text")].state.clone());
    cx.update(|window, cx| {
        window.focus(&input.focus_handle(cx), cx);
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
    });
    cx.run_until_parked();
    cx.simulate_input("typed");
    cx.run_until_parked();
    assert!(matches!(
        sent.borrow().last(),
        Some(Command::SetBinding { binding, .. }) if *binding == key_press(1)
    ));
    send(
        cx,
        &tx,
        profiles(&[("default", "Default"), ("work", "Work")], "work"),
    );
    cx.update(|_, cx| {
        assert!(view.read(cx).state.bindings.is_empty());
        assert!(view.read(cx).state.pages.is_empty());
    });
    send(cx, &tx, pages(&["Work", "Work 2"], 1));
    send(cx, &tx, bound(key_press(1), test_action("work")));
    let text = input_text(&view, cx, &top("text"));
    assert_eq!(text, "work");
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.bindings[&key_press(1)].action,
            Some(test_action("work"))
        )
    });
}

/// A layout with the shape of the N4 Pro: 10 keys, 4 bar panels, 4 knobs,
/// and the touch bar.
fn full_layout() -> Event {
    use opencraft_engine::{ControlLayout, DeviceLayout, SurfaceLayout};
    let mut surfaces = Vec::new();
    let mut controls = Vec::new();
    for n in 1..=10u8 {
        let id = SurfaceId::new(SurfaceKind::KeyFace, n);
        let (row, column) = ((n - 1) / 5, (n - 1) % 5);
        surfaces.push(SurfaceLayout {
            id,
            x: 6 + 170 * u16::from(column),
            y: 8 + 170 * u16::from(row),
            width: 112,
            height: 112,
        });
        controls.push(ControlLayout {
            id: ControlId::new(ControlKind::Key, n),
            row: row + 1,
            column: column + 1,
            surface: Some(id),
            feedback_surface: Some(id),
        });
    }
    for n in 1..=4u8 {
        let id = SurfaceId::new(SurfaceKind::BarPanel, n);
        surfaces.push(SurfaceLayout {
            id,
            x: 203 * u16::from(n - 1),
            y: 358,
            width: 176,
            height: 112,
        });
        controls.push(ControlLayout {
            id: ControlId::new(ControlKind::Knob, n),
            row: 1,
            column: n,
            surface: None,
            feedback_surface: Some(id),
        });
        controls.push(ControlLayout {
            id: ControlId::new(ControlKind::BarPanel, n),
            row: 1,
            column: n,
            surface: Some(id),
            feedback_surface: None,
        });
    }
    controls.push(ControlLayout {
        id: ControlId::new(ControlKind::TouchBar, 1),
        row: 1,
        column: 1,
        surface: None,
        feedback_surface: None,
    });
    Event::Layout(Arc::new(DeviceLayout {
        name: "Test device".into(),
        display_width: 800,
        display_height: 480,
        surfaces,
        controls,
        settings: opencraft_engine::SettingsSupport {
            lowest_brightness: 15,
            knob_lights: 4,
            haptic: true,
        },
    }))
}

#[gpui_kit::test]
fn the_window_shows_all_controls_at_the_minimum_size(cx: &mut TestAppContext) {
    let (_, cx, tx, _) = window(cx);
    let (width, height) = theme::WINDOW_MIN;
    cx.simulate_resize(size(px(width), px(height)));
    send(cx, &tx, full_layout());
    send(
        cx,
        &tx,
        profiles(&[("default", "Default"), ("work", "Work")], "default"),
    );
    let names: Vec<String> = (1..=8).map(|n| format!("Page {n}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    send(cx, &tx, pages(&names, 0));
    let within = |cx: &mut VisualTestContext, id: &'static str, area: &'static str| {
        let bounds = cx
            .debug_bounds(id)
            .unwrap_or_else(|| panic!("No element: {id}"));
        let area = cx.debug_bounds(area).unwrap();
        assert!(
            bounds.left() >= area.left()
                && bounds.top() >= area.top()
                && bounds.right() <= area.right()
                && bounds.bottom() <= area.bottom(),
            "{id} is outside {area:?}: {bounds:?}"
        );
        bounds
    };
    for id in ["sidebar", "toolbar", "canvas", "right-panel", "status-bar"] {
        let bounds = cx.debug_bounds(id).unwrap();
        assert!(
            bounds.right() <= px(width) && bounds.bottom() <= px(height),
            "{id} is outside the window: {bounds:?}"
        );
    }
    for id in [
        "toolbar-title",
        "profile-menu",
        "open-device-settings",
        "connection",
        "toggle-inspector",
    ] {
        within(cx, id, "toolbar");
    }
    for id in [
        "page-0",
        "page-rename",
        "page-left",
        "page-right",
        "page-duplicate",
        "page-remove",
        "surface-key face 1",
        "control-knob 1",
    ] {
        within(cx, id, "canvas");
    }
    within(cx, "editor", "right-panel");
    within(cx, "device-card", "sidebar");
    let panel = within(cx, "surface-bar panel 4", "canvas");
    let strip = within(cx, "control-touch bar 1", "canvas");
    let knob = within(cx, "control-knob 4", "canvas");
    assert!(panel.bottom() <= strip.top() && strip.bottom() <= knob.top());
    // The tabs and the page buttons do not overlap.
    assert!(
        cx.debug_bounds("page-remove").unwrap().left()
            > cx.debug_bounds("page-tabs").unwrap().right()
    );
    click(cx, "profile-menu");
    let menu = cx.debug_bounds("profiles").unwrap();
    assert!(menu.left() >= px(0.) && menu.right() <= px(width) && menu.bottom() <= px(height));
    // While the menu is open, a click outside it only closes it.
    click(cx, "control-touch bar 1");
    assert!(cx.debug_bounds("profiles").is_none());
    assert!(cx.debug_bounds("gesture-SwipeRight").is_none());
    click(cx, "control-touch bar 1");
    within(cx, "gesture-SwipeRight", "right-panel");
}
