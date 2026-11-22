//! The appearance of the windows: the appearance of macOS, or the light or
//! the dark appearance (decision 0029, section 21).
//!
//! Settings > General holds the choice ([`crate::settings::Appearance`]).
//! "System" follows macOS: a change of the colour scheme of the system
//! sends an appearance event to each window
//! ([`Window::observe_window_appearance`]), and the window draws with the
//! palette of the new appearance. "Light" and "Dark" keep one appearance for
//! the whole application: [`App::set_window_appearance`] sets
//! `NSApplication.appearance`, thus the native parts also follow it (the
//! title bar with the window buttons, the menus, the alerts, and the file
//! panels).
//!
//! Two things change with the choice:
//!
//! - the palette of the window (`crate::view::theme`), which every colour
//!   token reads from `theme::appearance()`, and
//! - the theme of the controls of the kit (`gpui_kit::component::Theme`),
//!   for the text field, the scrollbar, and the tooltip.

use crate::settings::Appearance as Choice;
use crate::shell::Shell;
use crate::view::theme::{self, Appearance};
use gpui_kit::{
    component::{Theme, ThemeMode},
    App, Entity, WeakEntity, Window, WindowAppearance,
};

/// The palette of the window for `choice`, when the window of macOS has
/// `system`.
pub fn palette(choice: Choice, system: WindowAppearance) -> Appearance {
    match choice {
        Choice::System => match system {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
        },
        Choice::Light => Appearance::Light,
        Choice::Dark => Appearance::Dark,
    }
}

/// The appearance of the application windows for `choice`. `None` follows
/// macOS, also while the application runs.
pub fn window_appearance(choice: Choice) -> Option<WindowAppearance> {
    match choice {
        Choice::System => None,
        Choice::Light => Some(WindowAppearance::Light),
        Choice::Dark => Some(WindowAppearance::Dark),
    }
}

/// The theme of the controls of the kit for `appearance`.
fn theme_mode(appearance: Appearance) -> ThemeMode {
    match appearance {
        Appearance::Light => ThemeMode::Light,
        Appearance::Dark => ThemeMode::Dark,
    }
}

/// Set the appearance of the application windows for `choice`, and draw them
/// again. The native parts of macOS follow the same choice.
pub fn apply(choice: Choice, cx: &mut App) {
    cx.set_window_appearance(window_appearance(choice));
    // The appearance of the application now has the choice, also when this
    // call created it.
    refresh(choice, cx.window_appearance(), cx);
}

/// Set the palette of the window and the theme of the kit for `choice`, when
/// the window of macOS has `system`, and draw the windows again. An
/// appearance event of a window calls this function with the appearance of
/// that window.
pub fn refresh(choice: Choice, system: WindowAppearance, cx: &mut App) {
    let appearance = palette(choice, system);
    let changed = theme::appearance() != appearance;
    theme::set_appearance(appearance);
    let mode = theme_mode(appearance);
    // The kit refreshes every window. A mode that is already the mode of the
    // kit changes nothing.
    if changed || Theme::global(cx).mode != mode {
        Theme::change(mode, None, cx);
    }
    cx.refresh_windows();
}

/// Follow the appearance of the window of macOS for the choice of the shell
/// of this window. Register it for each window that the shell opens: the
/// window then changes the palette when macOS changes the colour scheme
/// (Settings > General "System"). A forced appearance sends no such event.
pub fn follow(window: &mut Window, shell: &Entity<Shell>) {
    let shell = shell.downgrade();
    window
        .observe_window_appearance(move |window, cx| {
            let choice = choice(&shell, cx);
            refresh(choice, window.appearance(), cx);
        })
        .detach();
}

/// The appearance choice of `shell`. A shell that is gone keeps the choice
/// that it had at the start.
fn choice(shell: &WeakEntity<Shell>, cx: &mut App) -> Choice {
    shell
        .upgrade()
        .map(|shell| shell.read(cx).settings().appearance)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "System" follows macOS, and the other two choices keep one
    /// appearance.
    #[test]
    fn system_follows_macos_and_the_other_choices_do_not() {
        assert_eq!(window_appearance(Choice::System), None);
        assert_eq!(
            window_appearance(Choice::Light),
            Some(WindowAppearance::Light)
        );
        assert_eq!(
            window_appearance(Choice::Dark),
            Some(WindowAppearance::Dark)
        );
        for system in [
            WindowAppearance::Light,
            WindowAppearance::VibrantLight,
            WindowAppearance::Dark,
            WindowAppearance::VibrantDark,
        ] {
            let expected = match system {
                WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
                WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
            };
            assert_eq!(palette(Choice::System, system), expected);
            assert_eq!(palette(Choice::Light, system), Appearance::Light);
            assert_eq!(palette(Choice::Dark, system), Appearance::Dark);
        }
        assert_eq!(theme_mode(Appearance::Light), ThemeMode::Light);
        assert_eq!(theme_mode(Appearance::Dark), ThemeMode::Dark);
    }
}
