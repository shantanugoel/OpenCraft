//! The focus goes back to the control that opened a popover, a sheet, or
//! the command palette, when it closes (issue 248, `docs/DESIGN.md`,
//! section 5). It goes back only when a key opened the overlay: after a
//! click, no control has the focus and no focus ring shows.

use super::pages::{pages, profiles, send};
use super::*;
use crate::view::{controls::tests::focused, sidebar::ProfileSettingsRow};
use gpui_kit::{KeyUpEvent, MouseButton, MouseDownEvent};
use opencraft_engine::{
    Background, BackgroundState, DeviceLayout, DeviceSettings, KnobLightMode, KnobLights,
    SurfaceLayout,
};

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

/// Push the key `key`: the key goes down and up.
fn push(cx: &mut VisualTestContext, key: &'static str) {
    keys(cx, key);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).unwrap(),
    });
    cx.run_until_parked();
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

/// A window with key face 2 selected.
fn key_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
) {
    let (view, cx, tx, _sent) = window(cx);
    cx.update(|window, _| window.activate_window());
    send(cx, &tx, profiles(&[("work", "Work")], "work"));
    send(cx, &tx, pages(&["Main", "Media"], 0));
    click(cx, "surface-key face 2");
    (view, cx, tx)
}

/// Open the popover of `button` with Space, check that it is open, close it
/// with Escape, and check that `button` has the focus. The next Tab then
/// goes to `next`.
fn space_then_escape(cx: &mut VisualTestContext, button: &str, popover: &'static str, next: &str) {
    tab_to(cx, button);
    push(cx, "space");
    assert!(cx.debug_bounds(popover).is_some(), "{popover} is open");
    // The field has the focus. The popover of "Other colour" is a child of
    // its sample, thus the answer comes from the sample.
    let field = focused(cx);
    assert!(
        field.is_none() || field.as_deref() == Some(button),
        "{field:?}"
    );
    keys(cx, "escape");
    assert!(cx.debug_bounds(popover).is_none(), "{popover} is closed");
    assert_eq!(focused(cx).as_deref(), Some(button));
    keys(cx, "tab");
    assert_eq!(focused(cx).as_deref(), Some(next));
}

#[gpui_kit::test]
fn escape_gives_the_focus_back_to_the_text_colour_button(cx: &mut TestAppContext) {
    let (_view, cx, _tx) = key_window(cx);
    space_then_escape(cx, "label-colour", "label-colour-popover", "label-bold");
}

#[gpui_kit::test]
fn escape_gives_the_focus_back_to_the_other_colour_sample(cx: &mut TestAppContext) {
    let (_view, cx, _tx) = key_window(cx);
    space_then_escape(
        cx,
        "other-colour",
        "other-colour-popover",
        "gesture-Press-kind",
    );
}

#[gpui_kit::test]
fn escape_gives_the_focus_back_to_the_icon_button(cx: &mut TestAppContext) {
    let (_view, cx, _tx) = key_window(cx);
    space_then_escape(cx, "choose-icon", "icon-picker", "choose-image");
}

#[gpui_kit::test]
fn escape_gives_the_focus_back_to_a_knob_light(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = window(cx);
    send(
        cx,
        &tx,
        Event::DeviceSettings(DeviceSettings {
            brightness: 80,
            screen_off_minutes: 0,
            haptic: true,
            knob_lights: KnobLights {
                mode: KnobLightMode::Colors,
                brightness: 100,
                colors: vec![Color::WHITE; 2],
            },
        }),
    );
    click(cx, "device-card");
    tab_to(cx, "knob-2-colour");
    push(cx, "enter");
    assert!(cx.debug_bounds("knob-colour-popover").is_some());
    keys(cx, "escape");
    assert!(cx.debug_bounds("knob-colour-popover").is_none());
    assert_eq!(focused(cx).as_deref(), Some("knob-2-colour"));
    // The next Tab goes to a control after that light, not to the start of
    // the window.
    keys(cx, "tab");
    let next = focused(cx);
    assert!(next.is_some());
    assert_ne!(next.as_deref(), Some("hide-sidebar"));
}

#[gpui_kit::test]
fn escape_gives_the_focus_back_to_the_page_background_colour(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = window(cx);
    cx.update(|window, _| window.activate_window());
    let Event::Layout(layout) = layout() else {
        unreachable!()
    };
    let mut layout = DeviceLayout::clone(&layout);
    layout.surfaces.push(SurfaceLayout {
        id: SurfaceId::new(SurfaceKind::Background, 1),
        x: 0,
        y: 0,
        width: 400,
        height: 300,
    });
    send(cx, &tx, Event::Layout(Arc::new(layout)));
    send(cx, &tx, profiles(&[("work", "Work")], "work"));
    send(cx, &tx, pages(&["Main", "Media"], 1));
    click(cx, "page-settings");
    send(
        cx,
        &tx,
        Event::Background(BackgroundState {
            background: Some(Background::Color(Color::WHITE)),
            image: None,
        }),
    );
    assert!(cx.debug_bounds("other-colour").is_some());
    tab_to(cx, "other-colour");
    push(cx, "space");
    assert!(cx.debug_bounds("other-colour-popover").is_some());
    keys(cx, "escape");
    assert!(cx.debug_bounds("other-colour-popover").is_none());
    assert_eq!(focused(cx).as_deref(), Some("other-colour"));
}

#[gpui_kit::test]
fn return_in_the_field_gives_the_focus_back(cx: &mut TestAppContext) {
    let (_view, cx, _tx) = key_window(cx);
    tab_to(cx, "label-colour");
    push(cx, "space");
    cx.simulate_input("#123456");
    cx.run_until_parked();
    keys(cx, "enter");
    assert!(cx.debug_bounds("label-colour-popover").is_none());
    assert_eq!(focused(cx).as_deref(), Some("label-colour"));
}

#[gpui_kit::test]
fn a_sample_of_the_popover_gives_the_focus_back(cx: &mut TestAppContext) {
    let (_view, cx, _tx) = key_window(cx);
    tab_to(cx, "label-colour");
    push(cx, "space");
    // Tab stays in the popover; Space on a sample applies it and closes.
    tab_to(cx, "label-colour-2");
    push(cx, "space");
    assert!(cx.debug_bounds("label-colour-popover").is_none());
    assert_eq!(focused(cx).as_deref(), Some("label-colour"));
}

#[gpui_kit::test]
fn a_click_that_closes_a_popover_that_a_key_opened_shows_no_focus_ring(cx: &mut TestAppContext) {
    let (view, cx, _tx) = key_window(cx);
    tab_to(cx, "label-colour");
    push(cx, "space");
    // A click out of the popover closes it and does nothing else. As a
    // click that opens a popover, it leaves no control with the focus.
    cx.simulate_click(gpui_kit::point(px(2.), px(2.)), Modifiers::default());
    cx.run_until_parked();
    assert!(cx.debug_bounds("label-colour-popover").is_none());
    assert_eq!(focused(cx), None);
    assert!(workspace_has_focus(&view, cx));
}

#[gpui_kit::test]
fn a_popover_that_a_click_opened_leaves_no_control_with_the_focus(cx: &mut TestAppContext) {
    let (view, cx, _tx) = key_window(cx);
    for (button, popover) in [
        ("label-colour", "label-colour-popover"),
        ("other-colour", "other-colour-popover"),
        ("choose-icon", "icon-picker"),
    ] {
        click(cx, button);
        assert!(cx.debug_bounds(popover).is_some(), "{popover} is open");
        keys(cx, "escape");
        assert!(cx.debug_bounds(popover).is_none());
        assert_eq!(focused(cx), None, "{button}");
        assert!(workspace_has_focus(&view, cx), "{button}");
        // A click out of the popover does the same.
        click(cx, button);
        cx.simulate_click(gpui_kit::point(px(2.), px(2.)), Modifiers::default());
        cx.run_until_parked();
        assert!(cx.debug_bounds(popover).is_none());
        assert_eq!(focused(cx), None, "{button}");
        assert!(workspace_has_focus(&view, cx), "{button}");
    }
}

#[gpui_kit::test]
fn a_mouse_click_after_a_key_does_not_use_the_old_control(cx: &mut TestAppContext) {
    let (view, cx, _tx) = key_window(cx);
    // A key with the focus on a control, then a click that opens a popover.
    tab_to(cx, "label-bold");
    click(cx, "label-colour");
    assert!(cx.debug_bounds("label-colour-popover").is_some());
    keys(cx, "escape");
    assert_eq!(focused(cx), None);
    assert!(workspace_has_focus(&view, cx));
}

#[gpui_kit::test]
fn the_palette_gives_the_focus_back_to_the_control_that_had_it(cx: &mut TestAppContext) {
    let (view, cx, _tx) = key_window(cx);
    tab_to(cx, "label-bold");
    keys(cx, "cmd-k");
    cx.update(|_, cx| assert!(view.read(cx).palette.open));
    keys(cx, "escape");
    cx.update(|_, cx| assert!(!view.read(cx).palette.open));
    assert_eq!(focused(cx).as_deref(), Some("label-bold"));
    // With the focus in the root element, the palette gives it back to the
    // root element.
    click(cx, "tab-library");
    keys(cx, "cmd-k");
    keys(cx, "escape");
    assert_eq!(focused(cx), None);
    assert!(workspace_has_focus(&view, cx));
}

#[gpui_kit::test]
fn a_sheet_gives_the_focus_back_to_the_control_that_had_it(cx: &mut TestAppContext) {
    let (view, cx, _tx) = key_window(cx);
    let position = cx.debug_bounds("profile-work").unwrap().center();
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Right,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.run_until_parked();
    // The user goes to a control with Tab and chooses the command of the
    // menu with a key.
    tab_to(cx, "label-bold");
    keys(cx, "left");
    cx.dispatch_action(ProfileSettingsRow);
    cx.run_until_parked();
    cx.update(|_, cx| assert!(view.read(cx).sheet.profile.is_some()));
    keys(cx, "escape");
    cx.update(|_, cx| assert!(view.read(cx).sheet.profile.is_none()));
    assert_eq!(focused(cx).as_deref(), Some("label-bold"));
}
