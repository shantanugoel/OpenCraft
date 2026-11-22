//! The light and the dark appearance of the window (decision 0029, section
//! 21, and `crate::appearance`): the palette of the frame, a change of the
//! appearance of macOS, and the choice of Settings > General.

use super::*;
use crate::{
    appearance,
    settings::{self, Appearance as Choice},
    shell::ShellCommand,
};
use gpui_kit::{Hsla, WindowAppearance};

/// True if the frame of the window paints a quad with `colour`.
fn painted(cx: &mut VisualTestContext, colour: gpui_kit::Rgba) -> bool {
    let wanted = Hsla::from(colour);
    let same = |a: Hsla, b: Hsla| {
        (a.h - b.h).abs() < 1e-4
            && (a.s - b.s).abs() < 1e-4
            && (a.l - b.l).abs() < 1e-4
            && (a.a - b.a).abs() < 1e-4
    };
    cx.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .filter_map(|quad| quad.background.as_solid())
            .any(|solid| same(solid, wanted))
    })
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
