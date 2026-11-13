//! The content of the menu of the menu bar item (decision 0029, section 19),
//! with no platform code. The module `menu_bar` shows it with AppKit on
//! macOS.
//!
//! The options "Show the Dock icon" and "Start at login", the version, and
//! the licence notices are in the Settings window (decision 0029, section
//! 17). The menu has the items that open that window.
//!
//! [`MenuState::rows`] gives the rows of the menu for a state. The rows
//! follow the profiles and the pages of the engine, thus the number of rows
//! changes. Each row that the user can select has its command in
//! [`Row::command`].
//!
//! To add an item, add it to [`Row`], to [`MenuState::rows`], and to
//! [`Row::command`].

// Only the macOS menu bar item and the tests read the menu.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::{settings::SettingsPane, shell::ShellCommand};
use opencraft_engine::ProfileEntry;

/// The display brightness, for the slider of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Brightness {
    /// The value, in percent.
    pub value: u8,
    /// The lowest value that the device model accepts, in percent.
    pub lowest: u8,
}

/// The highest display brightness, in percent.
pub const HIGHEST_BRIGHTNESS: u8 = 100;

/// One row of a submenu of pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRow {
    pub index: usize,
    pub title: String,
    pub checked: bool,
}

impl PageRow {
    /// The command that the row sends when the user selects it.
    pub fn command(&self) -> ShellCommand {
        ShellCommand::SelectPage(self.index)
    }
}

/// One row of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// The model name and the connection state. The user cannot select it.
    Device {
        title: String,
        connected: bool,
    },
    Separator,
    /// A title for the rows below it. The user cannot select it.
    Section(String),
    /// A profile. The active profile has a check mark.
    Profile {
        id: String,
        title: String,
        checked: bool,
    },
    /// The submenu of the pages of the active profile.
    Pages {
        title: String,
        pages: Vec<PageRow>,
    },
    /// The one custom view: the slider for the display brightness.
    Brightness(Brightness),
    /// Opens the window.
    Open,
    /// Opens the Settings window.
    Settings,
    /// Opens the Settings window at the pane "About".
    About,
    Quit,
}

impl Row {
    /// The command that the row sends when the user selects it. The rows with
    /// no command, and the brightness row, send nothing from a selection: the
    /// slider sends [`ShellCommand::SetBrightness`] when it moves.
    pub fn command(&self) -> Option<ShellCommand> {
        match self {
            Row::Profile { id, .. } => Some(ShellCommand::SelectProfile(id.clone())),
            Row::Open => Some(ShellCommand::Open),
            Row::Settings => Some(ShellCommand::OpenSettings(None)),
            Row::About => Some(ShellCommand::OpenSettings(Some(SettingsPane::About))),
            Row::Quit => Some(ShellCommand::Quit),
            Row::Device { .. }
            | Row::Separator
            | Row::Section(_)
            | Row::Pages { .. }
            | Row::Brightness(_) => None,
        }
    }

    /// The title, for the rows that are items with a title.
    pub fn title(&self) -> Option<&str> {
        match self {
            Row::Device { title, .. }
            | Row::Section(title)
            | Row::Profile { title, .. }
            | Row::Pages { title, .. } => Some(title),
            Row::Open => Some("Open OpenCraft"),
            Row::Settings => Some("Settings…"),
            Row::About => Some("About and licence notices"),
            Row::Quit => Some("Quit OpenCraft"),
            Row::Separator | Row::Brightness(_) => None,
        }
    }

    /// The key equivalent, with the Command key.
    pub fn key(&self) -> &'static str {
        match self {
            Row::Open => "o",
            Row::Settings => ",",
            Row::Quit => "q",
            _ => "",
        }
    }
}

/// True if the two lists of rows differ at most in the value of the
/// brightness. Then the menu keeps its items and only moves the slider.
pub fn same_shape(a: &[Row], b: &[Row]) -> bool {
    let shape = |row: &Row| match row {
        Row::Brightness(b) => Row::Brightness(Brightness { value: 0, ..*b }),
        other => other.clone(),
    };
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| shape(a) == shape(b))
}

/// The values that the menu shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuState {
    /// The last `Connected` or `Disconnected` event of the engine.
    pub connected: bool,
    /// The model name of the layout. `None` before the engine sends it.
    pub device: Option<String>,
    /// The profiles of the device model.
    pub profiles: Vec<ProfileEntry>,
    /// The identity of the active profile. `None` before the engine sends
    /// the profiles.
    pub profile: Option<String>,
    /// The names of the pages of the active profile.
    pub pages: Vec<String>,
    /// The number of the active page.
    pub page: usize,
    /// `None` before the engine sends the device settings.
    pub brightness: Option<Brightness>,
}

impl MenuState {
    /// The rows of the menu, in their order.
    pub fn rows(&self) -> Vec<Row> {
        let name = self.device.as_deref().unwrap_or("Device");
        let state = if self.connected {
            "connected"
        } else {
            "not connected"
        };
        let mut rows = vec![
            Row::Device {
                title: format!("{name} · {state}"),
                connected: self.connected,
            },
            Row::Separator,
        ];
        if !self.profiles.is_empty() {
            rows.push(Row::Section("Profile".into()));
            rows.extend(self.profiles.iter().map(|p| Row::Profile {
                id: p.id.clone(),
                title: p.name.clone(),
                checked: self.profile.as_ref() == Some(&p.id),
            }));
        }
        if let Some(active) = self.pages.get(self.page) {
            rows.push(Row::Pages {
                title: format!("Page: {active}"),
                pages: self
                    .pages
                    .iter()
                    .enumerate()
                    .map(|(index, title)| PageRow {
                        index,
                        title: title.clone(),
                        checked: index == self.page,
                    })
                    .collect(),
            });
        }
        if rows.len() > 2 {
            rows.push(Row::Separator);
        }
        if let Some(brightness) = self.brightness {
            rows.push(Row::Brightness(brightness));
            rows.push(Row::Separator);
        }
        rows.extend([
            Row::Open,
            Row::Settings,
            Row::Separator,
            Row::About,
            Row::Separator,
            Row::Quit,
        ]);
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, name: &str) -> ProfileEntry {
        ProfileEntry {
            id: id.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    fn state() -> MenuState {
        MenuState {
            connected: true,
            device: Some("Stream Dock N4 Pro".into()),
            profiles: vec![
                profile("desk", "Desk"),
                profile("video", "Video"),
                profile("game", "Game"),
            ],
            profile: Some("video".into()),
            pages: vec!["Main".into(), "Mail".into()],
            page: 1,
            brightness: Some(Brightness {
                value: 80,
                lowest: 15,
            }),
        }
    }

    fn titles(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| r.title().unwrap_or("-").to_string())
            .collect()
    }

    #[test]
    fn the_menu_has_the_rows_of_decision_0029() {
        // No version, no licence notices item, no "Show the Dock icon", and
        // no "Start at login": the Settings window has them.
        assert_eq!(
            titles(&state().rows()),
            [
                "Stream Dock N4 Pro · connected",
                "-",
                "Profile",
                "Desk",
                "Video",
                "Game",
                "Page: Mail",
                "-",
                "-",
                "-",
                "Open OpenCraft",
                "Settings…",
                "-",
                "About and licence notices",
                "-",
                "Quit OpenCraft",
            ]
        );
    }

    #[test]
    fn the_active_profile_and_the_active_page_have_the_check_mark() {
        let rows = state().rows();
        let checked: Vec<_> = rows
            .iter()
            .filter_map(|r| match r {
                Row::Profile { id, checked, .. } => Some((id.as_str(), *checked)),
                _ => None,
            })
            .collect();
        assert_eq!(checked, [("desk", false), ("video", true), ("game", false)]);
        let Some(Row::Pages { pages, .. }) = rows.iter().find(|r| matches!(r, Row::Pages { .. }))
        else {
            panic!("no submenu of pages");
        };
        assert_eq!(
            pages.iter().map(|p| p.checked).collect::<Vec<_>>(),
            [false, true]
        );
    }

    #[test]
    fn a_profile_row_selects_its_profile() {
        let rows = state().rows();
        let commands: Vec<_> = rows
            .iter()
            .filter(|r| matches!(r, Row::Profile { .. }))
            .map(|r| r.command())
            .collect();
        assert_eq!(
            commands,
            [
                Some(ShellCommand::SelectProfile("desk".into())),
                Some(ShellCommand::SelectProfile("video".into())),
                Some(ShellCommand::SelectProfile("game".into())),
            ]
        );
    }

    #[test]
    fn a_page_row_selects_its_page() {
        let rows = state().rows();
        let Some(Row::Pages { pages, .. }) = rows.iter().find(|r| matches!(r, Row::Pages { .. }))
        else {
            panic!("no submenu of pages");
        };
        assert_eq!(pages[0].command(), ShellCommand::SelectPage(0));
        assert_eq!(pages[1].command(), ShellCommand::SelectPage(1));
    }

    #[test]
    fn the_rows_that_open_something_or_quit_send_their_commands() {
        assert_eq!(Row::Open.command(), Some(ShellCommand::Open));
        assert_eq!(Row::Open.key(), "o");
        assert_eq!(
            Row::Settings.command(),
            Some(ShellCommand::OpenSettings(None))
        );
        assert_eq!(Row::Settings.key(), ",");
        assert_eq!(
            Row::About.command(),
            Some(ShellCommand::OpenSettings(Some(SettingsPane::About)))
        );
        assert_eq!(Row::About.key(), "");
        assert_eq!(Row::Quit.command(), Some(ShellCommand::Quit));
        assert_eq!(Row::Quit.key(), "q");
    }

    #[test]
    fn the_rows_with_only_information_send_nothing() {
        let rows = state().rows();
        for row in &rows {
            if matches!(
                row,
                Row::Device { .. }
                    | Row::Separator
                    | Row::Section(_)
                    | Row::Pages { .. }
                    | Row::Brightness(_)
            ) {
                assert_eq!(row.command(), None, "{row:?}");
            }
        }
    }

    #[test]
    fn the_order_is_the_order_of_decision_0029() {
        // "Settings…" after "Open OpenCraft", a separator, "About and licence
        // notices", a separator, and "Quit".
        let rows = state().rows();
        let n = rows.len();
        assert_eq!(
            rows[n - 6..],
            [
                Row::Open,
                Row::Settings,
                Row::Separator,
                Row::About,
                Row::Separator,
                Row::Quit
            ]
        );
        // The brightness row is between two separators, before "Open".
        assert!(matches!(rows[n - 8], Row::Brightness(_)));
        assert_eq!(rows[n - 9], Row::Separator);
        assert_eq!(rows[n - 7], Row::Separator);
    }

    #[test]
    fn the_menu_shows_the_state() {
        let mut state = state();
        state.connected = false;
        assert_eq!(
            titles(&state.rows())[0],
            "Stream Dock N4 Pro · not connected"
        );
        assert!(matches!(
            state.rows()[0],
            Row::Device {
                connected: false,
                ..
            }
        ));
    }

    #[test]
    fn the_menu_has_no_profile_and_no_page_rows_before_the_engine_sends_them() {
        let rows = MenuState::default().rows();
        assert_eq!(
            titles(&rows),
            [
                "Device · not connected",
                "-",
                "Open OpenCraft",
                "Settings…",
                "-",
                "About and licence notices",
                "-",
                "Quit OpenCraft",
            ]
        );
        // No brightness row before the device settings.
        assert!(!rows.iter().any(|r| matches!(r, Row::Brightness(_))));
    }

    #[test]
    fn a_new_brightness_value_keeps_the_shape() {
        let a = state().rows();
        let mut moved = state();
        moved.brightness = Some(Brightness {
            value: 30,
            lowest: 15,
        });
        assert!(same_shape(&a, &moved.rows()));
        assert_ne!(a, moved.rows());
        let mut other = state();
        other.profile = Some("desk".into());
        assert!(!same_shape(&a, &other.rows()));
        let mut lowest = state();
        lowest.brightness = Some(Brightness {
            value: 80,
            lowest: 0,
        });
        assert!(!same_shape(&a, &lowest.rows()));
    }
}
