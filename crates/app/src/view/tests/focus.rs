//! The keyboard focus of the controls (issue 242, `docs/DESIGN.md`,
//! section 5): Tab and Shift-Tab move the focus in the order of the window,
//! Space and Return operate the control with the focus, a click does not
//! give a control the focus, and the key commands of the window keep their
//! work while a control has the focus.

use super::pages::{pages, profiles, send};
use super::*;
use crate::{
    settings::RightPanelTab,
    view::{controls::tests::focused, sidebar::ProfileSettingsRow},
};
use gpui_kit::{KeyUpEvent, MouseButton, MouseDownEvent};
use opencraft_engine::{DeviceSettings, KnobLightMode, KnobLights, LabelChange};

/// A window with two profiles and two pages in the active profile.
fn focus_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    send(
        cx,
        &tx,
        profiles(&[("default", "Default"), ("work", "Work")], "default"),
    );
    send(cx, &tx, pages(&["Main", "Meetings"], 0));
    (view, cx, tx, sent)
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

/// Push the key `key`: the key goes down and up. GPUI operates a control
/// with the focus at the key up of Space and Return.
fn push(cx: &mut VisualTestContext, key: &'static str) {
    keys(cx, key);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).unwrap(),
    });
    cx.run_until_parked();
}

/// The IDs of the controls that `count` presses of `key` give the focus to.
/// "-" is a stop that is not a control: a text field.
fn order(cx: &mut VisualTestContext, key: &str, count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            keys(cx, key);
            focused(cx).unwrap_or_else(|| "-".into())
        })
        .collect()
}

/// Press Tab until the control `id` has the focus.
fn tab_to(cx: &mut VisualTestContext, id: &str) {
    for _ in 0..80 {
        keys(cx, "tab");
        if focused(cx).as_deref() == Some(id) {
            return;
        }
    }
    panic!("Tab does not get to {id}");
}

fn workspace_has_focus(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| view.read(cx).focus.is_focused(window))
}

/// The start of the order: the sidebar, the toolbar, the page header, and
/// the tabs of the right panel.
const START: [&str; 16] = [
    "hide-sidebar",
    "device-card",
    "profile-add",
    "profile-default",
    "disclosure-default",
    "page-0",
    "page-1",
    "page-add",
    "profile-work",
    "sidebar-settings",
    "toolbar-search",
    "toggle-inspector",
    "next-page",
    "page-settings",
    "tab-inspector",
    "tab-library",
];

#[gpui_kit::test]
fn tab_and_shift_tab_move_the_focus_in_the_order_of_the_window(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = focus_window(cx);
    assert_eq!(order(cx, "tab", START.len()), START);
    let mut back: Vec<&str> = START[..START.len() - 1].to_vec();
    back.reverse();
    assert_eq!(order(cx, "shift-tab", back.len()), back);
    // Shift-Tab at the first control goes to the last one, and Tab goes
    // back to the first one.
    let last = order(cx, "shift-tab", 1);
    assert_ne!(last, ["hide-sidebar"]);
    assert_eq!(order(cx, "tab", 1), ["hide-sidebar"]);
}

#[gpui_kit::test]
fn tab_reaches_each_control_of_the_inspector_of_a_key(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = focus_window(cx);
    click(cx, "surface-key face 2");
    let ids = order(cx, "tab", START.len() + 27);
    assert_eq!(ids[..START.len()], START);
    let mut inspector = vec![
        "more-button",
        "choose-icon",
        "choose-image",
        "-",
        "label-position-top",
        "label-position-middle",
        "label-position-bottom",
        "label-position-none",
        "label-size",
        "label-colour",
        "label-bold",
        "no-colour",
    ];
    let colours: Vec<String> = (0..COLOURS.len()).map(|n| format!("colour-{n}")).collect();
    inspector.extend(colours.iter().map(String::as_str));
    inspector.extend(["other-colour", "gesture-Press-kind", "gesture-Release-kind"]);
    assert_eq!(ids[START.len()..], inspector[..ids.len() - START.len()]);
}

#[gpui_kit::test]
fn tab_reaches_the_switch_the_segments_and_the_sliders_of_the_device(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = focus_window(cx);
    send(
        cx,
        &tx,
        Event::DeviceSettings(DeviceSettings {
            brightness: 80,
            haptic: true,
            knob_lights: KnobLights {
                mode: KnobLightMode::Effect,
                brightness: 100,
                colors: vec![Color::WHITE; 2],
            },
        }),
    );
    click(cx, "device-card");
    let ids = order(cx, "tab", START.len() + 6);
    assert_eq!(ids[..START.len()], START);
    let device = &ids[START.len()..];
    assert!(device.contains(&"haptic-switch".to_string()), "{device:?}");
    assert!(
        device.iter().any(|id| id.starts_with("slider-keys")),
        "{device:?}"
    );
    // The arrow keys move the slider with the focus.
    tab_to(cx, "haptic-switch");
    push(cx, "space");
    let haptic = |sent: &Commands| {
        sent.borrow().iter().rev().find_map(|c| match c {
            Command::SetDeviceSettings(s) => Some(s.haptic),
            _ => None,
        })
    };
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    assert_eq!(haptic(&sent), Some(false), "Space operates the switch");
    // The arrow keys move the slider with the focus: after the switch, it
    // is the brightness of the knob lights.
    while !focused(cx).is_some_and(|id| id.starts_with("slider-keys")) {
        keys(cx, "tab");
    }
    keys(cx, "left");
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    let brightness = sent.borrow().iter().rev().find_map(|c| match c {
        Command::SetDeviceSettings(s) => Some(s.knob_lights.brightness),
        _ => None,
    });
    assert_eq!(brightness, Some(99));
}

#[gpui_kit::test]
fn space_and_return_operate_the_control_with_the_focus(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = focus_window(cx);
    tab_to(cx, "page-1");
    push(cx, "space");
    assert!(sent
        .borrow()
        .iter()
        .any(|c| matches!(c, Command::SelectPage { page: 1 })));
    tab_to(cx, "tab-library");
    push(cx, "enter");
    cx.update(|_, cx| assert_eq!(view.read(cx).panes.right_panel_tab, RightPanelTab::Library));
    // The control keeps the focus after its work.
    assert_eq!(focused(cx).as_deref(), Some("tab-library"));
    tab_to(cx, "hide-sidebar");
    push(cx, "space");
    cx.update(|_, cx| assert!(!view.read(cx).panes.sidebar_visible));
}

#[gpui_kit::test]
fn a_click_does_not_give_the_focus_to_a_control(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = focus_window(cx);
    click(cx, "tab-library");
    assert_eq!(focused(cx), None);
    assert!(workspace_has_focus(&view, cx));
    click(cx, "tab-inspector");
    // A click on a control takes the focus from a text field, as before.
    click(cx, "surface-key face 2");
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        input.update(cx, |input, cx| input.focus(window, cx));
    });
    click(cx, "label-bold");
    assert!(workspace_has_focus(&view, cx));
}

#[gpui_kit::test]
fn the_key_commands_operate_while_a_control_has_the_focus(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = focus_window(cx);
    click(cx, "surface-key face 2");
    tab_to(cx, "label-bold");
    // Delete clears the selected control, as Edit > Clear does: a button
    // has no use for the key (decision 0029, "Focus rule").
    keys(cx, "backspace");
    assert!(sent
        .borrow()
        .iter()
        .any(|c| matches!(c, Command::ClearControl { .. })));
    assert_eq!(focused(cx).as_deref(), Some("label-bold"));
    keys(cx, "ctrl-cmd-s");
    cx.update(|_, cx| assert!(!view.read(cx).panes.sidebar_visible));
    keys(cx, "cmd-k");
    cx.update(|_, cx| assert!(view.read(cx).palette.open));
}

#[gpui_kit::test]
fn a_menu_opens_with_space_and_its_button_keeps_the_focus(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = focus_window(cx);
    click(cx, "surface-key face 2");
    tab_to(cx, "label-size");
    push(cx, "space");
    assert!(cx.debug_bounds("label-size-menu").is_some());
    // Tab does nothing while the menu is open.
    keys(cx, "tab");
    assert_eq!(focused(cx).as_deref(), Some("label-size"));
    keys(cx, "down");
    push(cx, "space");
    assert!(cx.debug_bounds("label-size-menu").is_none());
    assert!(sent.borrow().iter().any(|c| matches!(
        c,
        Command::SetLabelStyle {
            change: LabelChange::Size(_),
            ..
        }
    )));
    assert_eq!(focused(cx).as_deref(), Some("label-size"));
    // Escape closes the menu that Return opened.
    push(cx, "enter");
    assert!(cx.debug_bounds("label-size-menu").is_some());
    keys(cx, "escape");
    assert!(cx.debug_bounds("label-size-menu").is_none());
    let _ = view;
}

#[gpui_kit::test]
fn the_focus_comes_back_to_the_window_when_its_control_goes_away(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = focus_window(cx);
    send(cx, &tx, Event::Problem("Cannot save the profile.".into()));
    tab_to(cx, "dismiss-0");
    push(cx, "space");
    cx.update(|_, cx| assert!(view.read(cx).state.problems.is_empty()));
    assert!(workspace_has_focus(&view, cx));
    keys(cx, "ctrl-cmd-s");
    cx.update(|_, cx| assert!(!view.read(cx).panes.sidebar_visible));
}

#[gpui_kit::test]
fn tab_stays_in_a_sheet_and_return_does_the_main_action(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = focus_window(cx);
    let position = cx.debug_bounds("profile-default").unwrap().center();
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Right,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.run_until_parked();
    cx.dispatch_action(ProfileSettingsRow);
    cx.run_until_parked();
    let sheet = [
        "-",
        "profile-colour-none",
        "profile-colour-0",
        "profile-colour-1",
        "profile-colour-2",
        "profile-colour-3",
        "profile-colour-4",
        "profile-colour-5",
        "first-page",
        "profile-export",
        "profile-import",
        "profile-remove",
        "profile-done",
    ];
    assert_eq!(
        order(cx, "tab", sheet.len() + 1),
        [&sheet[..], &sheet[..1]].concat()
    );
    // The focus is on the name field: Shift-Tab goes to "Done" first.
    let mut back = sheet.to_vec();
    back.reverse();
    assert_eq!(order(cx, "shift-tab", sheet.len()), back);
    // Space operates the control with the focus; Return does "Done".
    tab_to(cx, "profile-colour-2");
    push(cx, "space");
    tab_to(cx, "profile-import");
    push(cx, "enter");
    cx.update(|_, cx| assert!(view.read(cx).sheet.profile.is_none()));
    assert!(workspace_has_focus(&view, cx));
}
