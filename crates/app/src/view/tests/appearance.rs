//! The light and the dark appearance of the window (decision 0029, section
//! 21, and `crate::appearance`): the palette of the frame, a change of the
//! appearance of macOS, and the choice of Settings > General.

use super::*;
use crate::{
    appearance,
    settings::{self, Appearance as Choice},
    shell::ShellCommand,
};
use gpui_kit::{AnyWindowHandle, Hsla, WindowAppearance};

/// True if the two colours are the same.
fn same(a: Hsla, b: Hsla) -> bool {
    (a.h - b.h).abs() < 1e-4
        && (a.s - b.s).abs() < 1e-4
        && (a.l - b.l).abs() < 1e-4
        && (a.a - b.a).abs() < 1e-4
}

/// True if the frame of the window paints a quad with `colour`.
fn painted(cx: &mut VisualTestContext, colour: gpui_kit::Rgba) -> bool {
    let wanted = Hsla::from(colour);
    cx.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .filter_map(|quad| quad.background.as_solid())
            .any(|solid| same(solid, wanted))
    })
}

/// True if the frame that the window `handle` drew last paints a quad with
/// `colour`.
fn painted_in(cx: &mut VisualTestContext, handle: AnyWindowHandle, colour: gpui_kit::Rgba) -> bool {
    let wanted = Hsla::from(colour);
    cx.update_window(handle, |_, window, _| {
        window
            .painted_quads()
            .iter()
            .filter_map(|quad| quad.background.as_solid())
            .any(|solid| same(solid, wanted))
    })
    .unwrap_or(false)
}

/// Draw a frame with the palette of `appearance` and the theme of the kit
/// for it.
fn show(cx: &mut VisualTestContext, appearance: theme::Appearance) {
    let choice = match appearance {
        theme::Appearance::Light => Choice::Light,
        theme::Appearance::Dark => Choice::Dark,
    };
    cx.update(|_, cx| appearance::refresh(choice, WindowAppearance::Light, cx));
    cx.run_until_parked();
}

/// The window paints the three panes in the colours of both palettes.
#[gpui_kit::test]
fn the_window_draws_in_both_appearances(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = window(cx);
    let _keep = theme::keep_appearance_for_test();
    for appearance in [theme::Appearance::Light, theme::Appearance::Dark] {
        show(cx, appearance);
        assert_eq!(theme::appearance(), appearance);
        for (what, colour) in [
            ("canvas", theme::bg_canvas()),
            ("sidebar", theme::bg_sidebar()),
            ("bar", theme::bg_bar()),
        ] {
            assert!(
                painted(cx, colour),
                "{appearance:?}: the window paints no {what}"
            );
        }
    }
}

/// A window that opens after macOS changed the appearance has the palette
/// of macOS (H2 of review 2). OpenCraft runs with no window open, thus no
/// observer of a window sees that change: the shell sets the palette before
/// it opens the window (`docs/kb/gpui.md` gives the frame that this saves).
#[gpui_kit::test]
fn a_window_that_opens_later_has_the_appearance_of_macos(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = window(cx);
    let _keep = theme::keep_appearance_for_test();
    let shell = cx.update(|_, cx| view.read(cx).shell.clone());
    let canvas = |appearance| {
        let _scope = theme::set_appearance_for_test(appearance);
        theme::bg_canvas()
    };
    let light_canvas = canvas(theme::Appearance::Light);
    let dark_canvas = canvas(theme::Appearance::Dark);
    // The palette is dark, as it was when macOS was dark, and the test
    // platform has the light appearance of macOS now.
    cx.update(|_, cx| appearance::refresh(Choice::Dark, WindowAppearance::Dark, cx));
    cx.run_until_parked();
    assert_eq!(theme::appearance(), theme::Appearance::Dark);
    assert_eq!(
        cx.update(|_, cx| cx.window_appearance()),
        WindowAppearance::Light
    );
    // The window opens, and its first frame has the palette of macOS: the
    // window of the test platform is not the window of the test.
    let before = cx.update(|_, cx| cx.windows());
    cx.update(|_, cx| shell.update(cx, |shell, cx| shell.open_window(cx)));
    let opened = cx
        .update(|_, cx| cx.windows())
        .into_iter()
        .find(|window| !before.contains(window))
        .expect("the window did not open");
    assert!(
        painted_in(cx, opened, light_canvas),
        "the first frame has the dark canvas"
    );
    assert!(!painted_in(cx, opened, dark_canvas));
    cx.run_until_parked();
    assert_eq!(theme::appearance(), theme::Appearance::Light);
    assert!(painted_in(cx, opened, light_canvas));
}

/// With the choice System, the appearance of macOS gives the palette of the
/// window: the observer of the window calls `appearance::refresh` with the
/// new appearance (`appearance::follow`). A forced choice ignores it.
#[gpui_kit::test]
fn a_change_of_the_appearance_of_macos_changes_the_palette(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = window(cx);
    let _keep = theme::keep_appearance_for_test();
    let shell = cx.update(|_, cx| view.read(cx).shell.clone());

    // The first start has the choice System.
    assert_eq!(
        cx.update(|_, cx| shell.read(cx).settings().appearance),
        Choice::System
    );
    cx.update(|_, cx| appearance::refresh(Choice::System, WindowAppearance::Dark, cx));
    cx.run_until_parked();
    assert_eq!(theme::appearance(), theme::Appearance::Dark);
    assert!(painted(cx, theme::bg_canvas()));
    cx.update(|_, cx| appearance::refresh(Choice::System, WindowAppearance::Light, cx));
    cx.run_until_parked();
    assert_eq!(theme::appearance(), theme::Appearance::Light);
    assert!(painted(cx, theme::bg_canvas()));

    // The choice Dark keeps the dark palette when macOS is light.
    cx.update(|_, cx| {
        shell.update(cx, |shell, cx| {
            shell.command(ShellCommand::SetAppearance(settings::Appearance::Dark), cx)
        })
    });
    cx.run_until_parked();
    assert_eq!(theme::appearance(), theme::Appearance::Dark);
    cx.update(|_, cx| appearance::refresh(Choice::Dark, WindowAppearance::Light, cx));
    cx.run_until_parked();
    assert_eq!(theme::appearance(), theme::Appearance::Dark);
    assert!(painted(cx, theme::bg_canvas()));
}

/// The caret of a text field stops when the field is idle: each blink draws
/// the whole window, thus a window with the focus in a text field would draw
/// two frames each second with no change (issue #459). The kit's own tests
/// check that a policy with an idle time keeps the caret visible and stops.
#[gpui_kit::test]
fn the_caret_of_a_text_field_stops_when_the_field_is_idle(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = window(cx);
    let _keep = theme::keep_appearance_for_test();
    cx.update(|_, cx| appearance::refresh(Choice::Dark, WindowAppearance::Dark, cx));
    let policy = cx.update(|_, cx| gpui_kit::component::Theme::global(cx).cursor_blink);
    assert_eq!(policy, appearance::CARET);
    assert_eq!(
        policy.idle_timeout(),
        Some(appearance::CARET_IDLE),
        "a caret that never stops draws an idle window"
    );
    assert!(
        !policy.is_steady(),
        "the caret must blink while the person types"
    );
}
