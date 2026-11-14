//! The first start window: with and without a device, the selection of a
//! card, "Start", and the Accessibility request after "Start" (decision
//! 0029, section 18). The tests use the no-display test platform, a shell
//! with a fake command function, and open no application.

use super::*;
use crate::{
    app_menu,
    settings::{self, Settings},
    state::tests::layout,
    view::DeviceView,
};
use gpui_kit::{
    component::{Theme, ThemeMode},
    AnyWindowHandle, Modifiers, TestAppContext, VisualTestContext,
};
use opencraft_engine::{Command, Event, KindDescription, Status};
use std::{cell::RefCell, rc::Rc};

type Sent = Rc<RefCell<Vec<Command>>>;

struct Fixture {
    shell: Entity<Shell>,
    events: async_channel::Sender<Event>,
    sent: Sent,
    _dir: tempfile::TempDir,
}

/// A shell at the first start, with the layout of the test device.
fn shell(cx: &mut TestAppContext) -> Fixture {
    cx.update(|cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        app_menu::install(cx, &settings::Panes::default());
    });
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().to_path_buf();
    let sent: Sent = Default::default();
    let log = sent.clone();
    let (events, receiver) = async_channel::unbounded();
    let shell = cx.update(|cx| {
        cx.new(|cx| {
            let mut shell = Shell::new(
                config.clone(),
                Settings::default(),
                Rc::new(move |command| log.borrow_mut().push(command)),
            );
            #[cfg(target_os = "macos")]
            shell.set_login_dir(config.join("LaunchAgents"));
            shell.set_first_start(true);
            shell.listen(receiver, async_channel::unbounded().1, cx);
            shell
        })
    });
    events.try_send(layout()).unwrap();
    cx.run_until_parked();
    Fixture {
        shell,
        events,
        sent,
        _dir: dir,
    }
}

/// A kind that needs the Accessibility permission, as its description says.
fn key_kind(kind: &str, name: &str) -> KindDescription {
    KindDescription {
        kind: kind.into(),
        name: name.into(),
        needs_accessibility: true,
        ..Default::default()
    }
}

/// The kinds of the starter profiles that need the permission, and the
/// status of a permission that is not given.
fn no_permission(f: &Fixture, cx: &mut TestAppContext) {
    let kinds = [
        ("opencraft.hotkey", "Send a hotkey"),
        ("opencraft.type-text", "Type text"),
        ("opencraft.media-key", "Media key"),
    ];
    f.events
        .try_send(Event::Kinds(
            kinds.iter().map(|(k, n)| key_kind(k, n)).collect(),
        ))
        .unwrap();
    for (kind, _) in kinds {
        f.events
            .try_send(Event::Status {
                kind: kind.into(),
                status: Status::NeedsUser {
                    text: "No permission.".into(),
                    button: "Give the permission".into(),
                },
            })
            .unwrap();
    }
    cx.run_until_parked();
}

/// The view of `window`, if it has the type `V`.
fn view_of<V: 'static>(cx: &mut TestAppContext, window: AnyWindowHandle) -> Option<Entity<V>> {
    cx.update(|cx| {
        window
            .update(cx, |root, _, cx| {
                root.downcast::<gpui_kit::base::Root>()
                    .ok()
                    .and_then(|root| root.read(cx).view().clone().downcast::<V>().ok())
            })
            .ok()
            .flatten()
    })
}

/// The open window with a view of the type `V`.
fn window_of<V: 'static>(cx: &mut TestAppContext) -> Option<(AnyWindowHandle, Entity<V>)> {
    let windows = cx.update(|cx| cx.windows());
    windows
        .into_iter()
        .find_map(|w| view_of::<V>(cx, w).map(|view| (w, view)))
}

/// Open the window through the shell: at the first start, it is the first
/// start window.
fn open(f: &Fixture, cx: &mut TestAppContext) -> (Entity<FirstStartView>, VisualTestContext) {
    f.shell.update(cx, |shell, cx| shell.open_window(cx));
    assert!(matches!(
        f.sent.borrow_mut().pop(),
        Some(Command::WakeDevice)
    ));
    cx.run_until_parked();
    let (window, view) = window_of::<FirstStartView>(cx).expect("The first start window opens.");
    let visual = VisualTestContext::from_window(window, cx);
    let (width, height) = theme::FIRST_START_SIZE;
    visual.simulate_resize(size(px(width), px(height)));
    visual.run_until_parked();
    (view, visual)
}

fn click(cx: &mut VisualTestContext, id: &'static str) {
    let bounds = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"));
    cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
    cx.simulate_click(bounds.center(), Modifiers::default());
    cx.run_until_parked();
}

fn windows(cx: &mut TestAppContext) -> usize {
    cx.update(|cx| cx.windows().len())
}

/// The names of the starter profiles that the shell sent to the engine.
fn started(sent: &Sent) -> Vec<String> {
    sent.borrow()
        .iter()
        .filter_map(|c| match c {
            Command::AddStarterProfile { profile } => Some(profile.name.clone()),
            _ => None,
        })
        .collect()
}

#[gpui_kit::test]
fn with_a_device_the_window_shows_the_model_the_badge_and_the_four_cards(cx: &mut TestAppContext) {
    let f = shell(cx);
    f.events.try_send(Event::Connected).unwrap();
    cx.run_until_parked();
    let (view, mut visual) = open(&f, cx);
    assert_eq!(windows(cx), 1, "no main window at the first start");
    let snapshot = visual.update(|_, cx| view.read(cx).snapshot.clone());
    assert_eq!(
        Texts::of(&snapshot),
        Texts {
            title: "Your device is ready",
            device: "Test device found".into(),
            device_text: CONNECTED_TEXT,
        }
    );
    assert!(visual.debug_bounds("first-start-badge").is_some());
    for card in ["starter-0", "starter-1", "starter-2", "starter-3"] {
        assert!(visual.debug_bounds(card).is_some(), "{card}");
    }
    // Two columns of cards, in the right column.
    let left = visual.debug_bounds("first-start-left").unwrap();
    let first = visual.debug_bounds("starter-0").unwrap();
    let second = visual.debug_bounds("starter-1").unwrap();
    let third = visual.debug_bounds("starter-2").unwrap();
    assert!(first.left() > left.right());
    assert_eq!(first.top(), second.top());
    assert!(second.left() > first.right());
    assert!(third.top() > first.bottom());
    assert_eq!(visual.update(|_, cx| view.read(cx).selected), 0);
    // The picture and the cards show the images of the engine for the
    // surfaces of the layout.
    let images = visual.update(|_, cx| {
        let view = view.read(cx);
        view.previews
            .as_ref()
            .map(|p| p.images.iter().map(|m| m.len()).collect::<Vec<_>>())
    });
    assert_eq!(images, Some(vec![2, 2, 2, 0]), "key face 2 and bar panel 3");
    assert!(view_of::<FirstStartView>(cx, visual.window_handle()).is_some());
}

#[gpui_kit::test]
fn a_cell_of_a_card_shows_the_fill_colour_of_its_key_and_no_image(cx: &mut TestAppContext) {
    let f = shell(cx);
    f.events.try_send(Event::Connected).unwrap();
    cx.run_until_parked();
    let (view, mut visual) = open(&f, cx);
    let key = crate::state::tests::key();
    let ids = [
        ("starter-0", "starter-0-cell-0"),
        ("starter-1", "starter-1-cell-0"),
        ("starter-2", "starter-2-cell-0"),
        ("starter-3", "starter-3-cell-0"),
    ];
    for (index, starter) in STARTERS.iter().enumerate() {
        let (card, cell) = ids[index];
        let cell = visual
            .debug_bounds(cell)
            .unwrap_or_else(|| panic!("No cell in the card {}", starter.name));
        let card = visual.debug_bounds(card).unwrap();
        // The cell keeps its height, and it is in the card.
        assert_eq!(cell.size.height, theme::STARTER_CELL, "{}", starter.name);
        assert!(card.contains(&cell.origin) && card.contains(&cell.bottom_right()));
        // The colour is the fill of the key in the profile, from the engine.
        let profile = starter.profile().unwrap();
        let page = profile.page(profile.first_page()).unwrap();
        let expected = page.content(key).color;
        let shown = visual.update(|_, cx| {
            view.read(cx)
                .fills(index)
                .and_then(|m| m.get(&key).copied())
        });
        assert_eq!(shown, expected, "{}", starter.name);
        assert_eq!(shown.is_some(), starter.name != "Empty");
    }
}

#[gpui_kit::test]
fn with_no_device_the_window_says_connect_your_device_and_the_user_can_continue(
    cx: &mut TestAppContext,
) {
    let f = shell(cx);
    let (view, mut visual) = open(&f, cx);
    let snapshot = visual.update(|_, cx| view.read(cx).snapshot.clone());
    assert_eq!(
        Texts::of(&snapshot),
        Texts {
            title: "Welcome to OpenCraft",
            device: NO_DEVICE_TITLE.into(),
            device_text: NO_DEVICE_TEXT,
        }
    );
    assert!(visual.debug_bounds("first-start-badge").is_none());
    // The window follows the device.
    f.events.try_send(Event::Connected).unwrap();
    cx.run_until_parked();
    assert!(visual.debug_bounds("first-start-badge").is_some());
    f.events.try_send(Event::Disconnected).unwrap();
    cx.run_until_parked();
    assert!(visual.debug_bounds("first-start-badge").is_none());
    // "Start" also operates with no device.
    click(&mut visual, "first-start-start");
    assert_eq!(started(&f.sent), ["Everyday Mac"]);
    assert!(window_of::<FirstStartView>(cx).is_none());
    assert!(window_of::<DeviceView>(cx).is_some());
    assert!(!cx.update(|cx| f.shell.read(cx).first_start()));
}

#[gpui_kit::test]
fn a_click_selects_a_card_and_start_makes_that_profile_and_opens_the_editor(
    cx: &mut TestAppContext,
) {
    let f = shell(cx);
    let (view, mut visual) = open(&f, cx);
    click(&mut visual, "starter-3");
    assert_eq!(visual.update(|_, cx| view.read(cx).selected), 3);
    click(&mut visual, "starter-1");
    assert_eq!(visual.update(|_, cx| view.read(cx).selected), 1);
    assert!(f.sent.borrow().is_empty(), "a selection sends nothing");
    click(&mut visual, "first-start-start");
    assert_eq!(started(&f.sent), ["Media and sound"]);
    assert_eq!(windows(cx), 1);
    let (_, main) = window_of::<DeviceView>(cx).expect("The main window opens.");
    // The permission is given (no status "needs the user"): no request.
    cx.update(|cx| assert_eq!(main.read(cx).state.permission_request, None));
    // The next "Open OpenCraft" brings the main window.
    f.shell.update(cx, |shell, cx| shell.open_window(cx));
    cx.run_until_parked();
    assert_eq!(windows(cx), 1);
    assert!(window_of::<DeviceView>(cx).is_some());
}

#[gpui_kit::test]
fn return_starts_the_arrow_keys_select_and_a_double_click_starts(cx: &mut TestAppContext) {
    let f = shell(cx);
    let (view, mut visual) = open(&f, cx);
    let selected = |visual: &mut VisualTestContext| visual.update(|_, cx| view.read(cx).selected);
    for (key, want) in [
        ("left", 0),
        ("up", 0),
        ("right", 1),
        ("right", 1),
        ("down", 3),
        ("down", 3),
        ("left", 2),
        ("up", 0),
    ] {
        visual.simulate_keystrokes(key);
        visual.run_until_parked();
        assert_eq!(selected(&mut visual), want, "{key}");
    }
    visual.simulate_keystrokes("down");
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    assert_eq!(started(&f.sent), ["Video calls"]);
    assert!(window_of::<FirstStartView>(cx).is_none());

    // A double-click on a card starts with that card.
    let f = shell(cx);
    let (_, mut visual) = open(&f, cx);
    let bounds = visual.debug_bounds("starter-3").unwrap();
    visual.simulate_mouse_move(bounds.center(), None, Modifiers::default());
    visual.simulate_event(gpui_kit::MouseDownEvent {
        button: gpui_kit::MouseButton::Left,
        position: bounds.center(),
        modifiers: Modifiers::default(),
        click_count: 2,
        first_mouse: false,
    });
    visual.simulate_event(gpui_kit::MouseUpEvent {
        button: gpui_kit::MouseButton::Left,
        position: bounds.center(),
        modifiers: Modifiers::default(),
        click_count: 2,
    });
    visual.run_until_parked();
    assert_eq!(started(&f.sent), ["Default"]);
}

#[gpui_kit::test]
fn the_box_names_the_kinds_that_need_the_permission_only_when_it_is_not_given(
    cx: &mut TestAppContext,
) {
    let f = shell(cx);
    let (_, mut visual) = open(&f, cx);
    assert!(visual.debug_bounds("first-start-permission").is_none());
    no_permission(&f, cx);
    assert!(visual.debug_bounds("first-start-permission").is_some());
    let permission = cx.update(|cx| permission(&f.shell.read(cx).state));
    assert_eq!(
        permission_note(&permission).unwrap(),
        "Three actions need the Accessibility permission: Send a hotkey, Type text and \
         Media key. OpenCraft asks for it when you add the first one."
    );
    // The window does not ask for the permission: it sends nothing.
    assert!(f.sent.borrow().is_empty());
    assert_eq!(
        permission_note(&Permission {
            given: false,
            kinds: vec!["Send a hotkey".into()],
        })
        .unwrap(),
        "One action needs the Accessibility permission: Send a hotkey. OpenCraft asks for it \
         when you add the first one."
    );
    assert_eq!(
        permission_note(&Permission {
            given: true,
            kinds: vec!["Send a hotkey".into()],
        }),
        None
    );
}

#[gpui_kit::test]
fn a_starter_profile_with_a_key_action_shows_the_request_in_the_editor_one_time(
    cx: &mut TestAppContext,
) {
    let f = shell(cx);
    no_permission(&f, cx);
    let (_, mut visual) = open(&f, cx);
    // "Media and sound" has media keys.
    click(&mut visual, "starter-1");
    click(&mut visual, "first-start-start");
    let (window, main) = window_of::<DeviceView>(cx).unwrap();
    cx.update(|cx| {
        assert_eq!(
            main.read(cx).state.permission_request.as_deref(),
            Some("Media key")
        )
    });
    let mut visual = VisualTestContext::from_window(window, cx);
    let (width, height) = theme::WINDOW_SIZE;
    visual.simulate_resize(size(px(width), px(height)));
    visual.run_until_parked();
    assert!(visual.debug_bounds("permission-sheet").is_some());
    // Escape is "Not now". A new window does not show the request again.
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    assert!(visual.debug_bounds("permission-sheet").is_none());
    assert_eq!(
        cx.update(|cx| f.shell.read(cx).state.permission_request.clone()),
        None
    );
    assert!(!f
        .shell
        .update(cx, |shell, _| shell.request_permission("Type text".into())));
}

#[gpui_kit::test]
fn a_starter_profile_with_no_key_action_shows_no_request(cx: &mut TestAppContext) {
    let f = shell(cx);
    no_permission(&f, cx);
    let (_, mut visual) = open(&f, cx);
    // "Everyday Mac" opens applications and web pages only.
    click(&mut visual, "first-start-start");
    let (_, main) = window_of::<DeviceView>(cx).unwrap();
    cx.update(|cx| assert_eq!(main.read(cx).state.permission_request, None));
    // The request can still show later in this run.
    assert!(f.shell.update(cx, |shell, _| shell
        .request_permission("Send a hotkey".into())));
}

#[gpui_kit::test]
fn a_closed_first_start_window_opens_again_until_the_user_starts(cx: &mut TestAppContext) {
    let f = shell(cx);
    let (_, visual) = open(&f, cx);
    let window = visual.window_handle();
    cx.update(|cx| {
        window
            .update(cx, |_, window, _| window.remove_window())
            .unwrap()
    });
    cx.run_until_parked();
    assert_eq!(windows(cx), 0);
    assert!(f.sent.borrow().is_empty());
    // "Open OpenCraft" from the menu bar item.
    let _ = open(&f, cx);
    assert_eq!(windows(cx), 1);
    f.shell.update(cx, |shell, cx| shell.open_window(cx));
    cx.run_until_parked();
    assert_eq!(windows(cx), 1, "the same window comes to the front");
}

#[gpui_kit::test]
fn each_part_of_the_first_start_window_has_a_role_and_a_label(cx: &mut TestAppContext) {
    use gpui_kit::Role;
    let f = shell(cx);
    f.events.try_send(Event::Connected).unwrap();
    cx.run_until_parked();
    let (_view, mut visual) = open(&f, cx);
    crate::view::controls::accessibility_log::take();
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let nodes = crate::view::controls::accessibility_log::take();
    let has = |role: Role, label: &str| nodes.iter().any(|(r, l)| *r == role && l == label);
    assert!(has(Role::Label, "Your device is ready"), "{nodes:#?}");
    assert!(has(Role::Label, "Test device found"), "{nodes:#?}");
    for starter in crate::starters::STARTERS {
        assert!(
            has(Role::RadioButton, starter.name),
            "No card {}: {nodes:#?}",
            starter.name
        );
    }
    assert!(has(Role::Button, "Ready profiles…"), "{nodes:#?}");
    assert!(has(Role::Button, "Start"), "{nodes:#?}");
}

/// The ID of the control with the keyboard focus after `key`.
fn focus_after(visual: &mut VisualTestContext, key: &str) -> Option<String> {
    visual.simulate_keystrokes(key);
    visual.run_until_parked();
    crate::view::controls::tests::focused(visual)
}

#[gpui_kit::test]
fn tab_reaches_the_selected_card_and_start_and_the_arrows_move_the_focus(cx: &mut TestAppContext) {
    let f = shell(cx);
    let (view, mut visual) = open(&f, cx);
    // The cards are one group: Tab reaches the selected card only.
    assert_eq!(
        focus_after(&mut visual, "tab").as_deref(),
        Some("starter-0")
    );
    assert_eq!(
        focus_after(&mut visual, "tab").as_deref(),
        Some("first-start-ready")
    );
    assert_eq!(
        focus_after(&mut visual, "tab").as_deref(),
        Some("first-start-start")
    );
    assert_eq!(
        focus_after(&mut visual, "tab").as_deref(),
        Some("starter-0")
    );
    // An arrow key moves the selection, and the focus goes with it.
    assert_eq!(
        focus_after(&mut visual, "right").as_deref(),
        Some("starter-1")
    );
    assert_eq!(visual.update(|_, cx| view.read(cx).selected), 1);
    assert_eq!(
        focus_after(&mut visual, "shift-tab").as_deref(),
        Some("first-start-start")
    );
    assert_eq!(
        focus_after(&mut visual, "shift-tab").as_deref(),
        Some("first-start-ready")
    );
    assert_eq!(
        focus_after(&mut visual, "shift-tab").as_deref(),
        Some("starter-1")
    );
}

#[gpui_kit::test]
fn start_adds_the_ready_profiles_that_are_on_before_the_starter(cx: &mut TestAppContext) {
    let f = shell(cx);
    let (_view, mut visual) = open(&f, cx);
    click(&mut visual, "first-start-ready");
    click(&mut visual, "ready-0");
    click(&mut visual, "ready-2");
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    assert!(
        visual.debug_bounds("first-start-ready-list").is_none(),
        "Escape closes the list"
    );
    click(&mut visual, "first-start-start");
    assert_eq!(started(&f.sent), ["Safari", "Zoom", "Everyday Mac"]);
}
