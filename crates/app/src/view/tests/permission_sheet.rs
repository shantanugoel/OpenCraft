//! The Accessibility request (decision 0029, section 18): it opens for the
//! first binding that the window sets with a kind that needs the permission,
//! when the permission is not given, one time in each run. The tests open no
//! application: the shell has fake desktop functions.

use super::*;
use crate::shell::ACCESSIBILITY_PANE;
use crate::view::permission_sheet::request_text;

thread_local! {
    static OPENED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn open_url(url: &str) -> bool {
    OPENED.with(|opened| opened.borrow_mut().push(url.to_string()));
    true
}

const HOTKEY: &str = "test.hotkey";
const OPEN: &str = "test.open";

/// A kind that needs the permission, and a kind that does not.
fn kinds() -> Event {
    Event::Kinds(vec![
        KindDescription {
            kind: HOTKEY.into(),
            name: "Send a hotkey".into(),
            needs_accessibility: true,
            ..Default::default()
        },
        KindDescription {
            kind: OPEN.into(),
            name: "Open a URL".into(),
            ..Default::default()
        },
    ])
}

fn status(ready: bool) -> Event {
    Event::Status {
        kind: HOTKEY.into(),
        status: if ready {
            Status::Ready
        } else {
            Status::NeedsUser {
                text: "No permission.".into(),
                button: "Give the permission".into(),
            }
        },
    }
}

fn binding(gesture: GestureKind) -> BindingId {
    BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Key, 2),
        gesture,
    }
}

fn action(kind: &str) -> Action {
    Action {
        kind: kind.into(),
        settings: Settings::new(),
    }
}

/// The engine sends `action` back for `id`, as after a `SetBinding`.
fn reply(tx: &async_channel::Sender<Event>, id: BindingId, action: Action) {
    tx.try_send(Event::Binding(BindingState {
        binding: id,
        action: Some(action),
        available: true,
    }))
    .unwrap();
}

/// The window with the kinds, the permission state, and a shell with fake
/// desktop functions. Key 2 is selected.
fn rig(
    cx: &mut TestAppContext,
    ready: bool,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    OPENED.with(|opened| opened.borrow_mut().clear());
    let (view, cx, tx, sent) = window(cx);
    let shell = cx.update(|_, cx| view.read(cx).shell.clone());
    cx.update(|_, cx| shell.update(cx, |shell, _| shell.set_desktop(|_| true, open_url)));
    tx.try_send(kinds()).unwrap();
    tx.try_send(status(ready)).unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    (view, cx, tx, sent)
}

/// Choose the kind `index` of the kind menu on the card of `gesture`.
fn choose(cx: &mut VisualTestContext, gesture: &str, index: usize) {
    let kind: &'static str = Box::leak(format!("gesture-{gesture}-kind").into_boxed_str());
    let item: &'static str = Box::leak(format!("kind-{index}").into_boxed_str());
    click(cx, kind);
    click(cx, item);
}

fn sheet_open(cx: &mut VisualTestContext) -> bool {
    cx.debug_bounds("permission-sheet").is_some()
}

#[gpui_kit::test]
fn the_first_binding_that_needs_the_permission_opens_the_request_one_time(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = rig(cx, false);
    choose(cx, "Press", 0);
    assert!(matches!(
        sent.borrow().last(),
        Some(Command::SetBinding { action: Some(a), .. }) if a.kind == HOTKEY
    ));
    // The request waits for the engine: the binding is in the profile then.
    assert!(!sheet_open(cx));
    reply(&tx, binding(GestureKind::Press), action(HOTKEY));
    cx.run_until_parked();
    assert!(sheet_open(cx));
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.permission_request.as_deref(),
            Some("Send a hotkey")
        )
    });
    assert_eq!(
        request_text("Send a hotkey"),
        "“Send a hotkey” sends input to other apps. macOS lets OpenCraft do this only with the \
         Accessibility permission. The binding stays in the profile. The action operates after \
         you give the permission."
    );
    // The key commands of the window do not operate while the sheet is open.
    let before = sent.borrow().len();
    cx.simulate_keystrokes("cmd-z");
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), before);

    click(cx, "permission-not-now");
    assert!(!sheet_open(cx));
    assert!(OPENED.with(|o| o.borrow().is_empty()));
    // A second binding of a kind that needs the permission: no request in
    // this run.
    choose(cx, "Release", 0);
    reply(&tx, binding(GestureKind::Release), action(HOTKEY));
    cx.run_until_parked();
    assert!(!sheet_open(cx));
}

#[gpui_kit::test]
fn open_system_settings_opens_the_pane_and_asks_the_engine_to_wait(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = rig(cx, false);
    choose(cx, "Press", 0);
    reply(&tx, binding(GestureKind::Press), action(HOTKEY));
    cx.run_until_parked();
    assert!(sheet_open(cx));
    click(cx, "permission-open-settings");
    assert!(!sheet_open(cx));
    assert_eq!(
        OPENED.with(|o| o.borrow().clone()),
        [ACCESSIBILITY_PANE.to_string()]
    );
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::Resolve {
            kind: HOTKEY.into()
        })
    );
}

#[gpui_kit::test]
fn return_in_the_request_opens_system_settings(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = rig(cx, false);
    choose(cx, "Press", 0);
    reply(&tx, binding(GestureKind::Press), action(HOTKEY));
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(!sheet_open(cx));
    assert_eq!(OPENED.with(|o| o.borrow().len()), 1);
}

#[gpui_kit::test]
fn no_request_for_a_given_permission_a_page_change_or_a_kind_with_no_need(cx: &mut TestAppContext) {
    // The permission is given.
    let (_view, cx, tx, _sent) = rig(cx, true);
    choose(cx, "Press", 0);
    reply(&tx, binding(GestureKind::Press), action(HOTKEY));
    cx.run_until_parked();
    assert!(!sheet_open(cx));

    // The bindings of a page change come from the engine with no command
    // of the window.
    tx.try_send(status(false)).unwrap();
    tx.try_send(Event::Pages {
        names: vec!["Page 1".into()],
        active: 0,
        first: 0,
    })
    .unwrap();
    reply(&tx, binding(GestureKind::Release), action(HOTKEY));
    cx.run_until_parked();
    assert!(!sheet_open(cx));

    // A kind that does not need the permission.
    choose(cx, "Press", 1);
    reply(&tx, binding(GestureKind::Press), action(OPEN));
    cx.run_until_parked();
    assert!(!sheet_open(cx));
}

#[gpui_kit::test]
fn a_toggle_that_contains_a_key_action_opens_the_request(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = rig(cx, false);
    choose(cx, "Press", 1);
    let toggle = Action {
        kind: "test.toggle".into(),
        settings: Settings::from([("on".to_string(), action(HOTKEY).to_value())]),
    };
    reply(&tx, binding(GestureKind::Press), toggle);
    cx.run_until_parked();
    assert!(sheet_open(cx));
    // Escape is "Not now".
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!sheet_open(cx));
    cx.update(|_, cx| assert_eq!(view.read(cx).state.permission_request, None));
}
