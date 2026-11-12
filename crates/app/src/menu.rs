//! The content of the menu of the menu bar item (decision 0020 and decision
//! 0029, section 19), with no platform code. The module `menu_bar` shows it
//! with AppKit on macOS.
//!
//! The options "Show the Dock icon" and "Start at login", the version, and
//! the licence notices are in the Settings window (decision 0029, section
//! 17). The menu has the items that open that window.
//!
//! To add an item, add it to [`Item`] and to [`ITEMS`], and give it a title.

// Only the macOS menu bar item and the tests read the menu.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::{settings::SettingsPane, shell::ShellCommand};

/// One item of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Device,
    Profile,
    Open,
    /// Opens the Settings window.
    Settings,
    /// Opens the Settings window at the pane "About".
    About,
    Quit,
    Separator,
}

/// What the user can do with an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A line of text that the user cannot select.
    Text,
    /// An item that sends a command.
    Command,
    Separator,
}

/// The items, in their order in the menu.
pub const ITEMS: [Item; 9] = [
    Item::Device,
    Item::Profile,
    Item::Separator,
    Item::Open,
    Item::Settings,
    Item::Separator,
    Item::About,
    Item::Separator,
    Item::Quit,
];

impl Item {
    pub fn kind(self) -> Kind {
        match self {
            Item::Device | Item::Profile => Kind::Text,
            Item::Open | Item::Settings | Item::About | Item::Quit => Kind::Command,
            Item::Separator => Kind::Separator,
        }
    }

    /// The command that the item sends when the user selects it.
    pub fn command(self) -> Option<ShellCommand> {
        match self {
            Item::Open => Some(ShellCommand::Open),
            Item::Settings => Some(ShellCommand::OpenSettings(None)),
            Item::About => Some(ShellCommand::OpenSettings(Some(SettingsPane::About))),
            Item::Quit => Some(ShellCommand::Quit),
            Item::Device | Item::Profile | Item::Separator => None,
        }
    }

    /// The key equivalent, with the Command key.
    pub fn key(self) -> &'static str {
        match self {
            Item::Settings => ",",
            Item::Quit => "q",
            _ => "",
        }
    }

    /// The number that identifies the item to the platform menu: its
    /// position in [`ITEMS`].
    pub fn tag(self) -> isize {
        ITEMS.iter().position(|&item| item == self).unwrap_or(0) as isize
    }

    /// The item at position `tag` in [`ITEMS`].
    pub fn from_tag(tag: isize) -> Option<Item> {
        usize::try_from(tag)
            .ok()
            .and_then(|i| ITEMS.get(i).copied())
    }
}

/// The values that the menu shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuState {
    /// The last `Connected` or `Disconnected` event of the engine.
    pub connected: bool,
    /// The name of the active profile. `None` before the engine sends the
    /// profiles.
    pub profile: Option<String>,
}

impl MenuState {
    /// The title of an item. `None` means that the menu does not show the
    /// item now.
    pub fn title(&self, item: Item) -> Option<String> {
        Some(match item {
            Item::Device if self.connected => "Device: connected".into(),
            Item::Device => "Device: not connected".into(),
            Item::Profile => format!("Profile: {}", self.profile.as_ref()?),
            Item::Open => "Open OpenCraft".into(),
            Item::Settings => "Settings…".into(),
            Item::About => "About and licence notices".into(),
            Item::Quit => "Quit OpenCraft".into(),
            Item::Separator => String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_has_the_items_of_decision_0029() {
        let state = MenuState {
            connected: true,
            profile: Some("Desk".into()),
        };
        let titles: Vec<_> = ITEMS
            .iter()
            .filter(|i| i.kind() != Kind::Separator)
            .map(|&i| state.title(i).unwrap())
            .collect();
        // No version, no licence notices item, no "Show the Dock icon", and
        // no "Start at login": the Settings window has them.
        assert_eq!(
            titles,
            [
                "Device: connected",
                "Profile: Desk",
                "Open OpenCraft",
                "Settings…",
                "About and licence notices",
                "Quit OpenCraft"
            ]
        );
        assert_eq!(Item::Quit.key(), "q");
        assert_eq!(Item::Quit.command(), Some(ShellCommand::Quit));
        assert_eq!(Item::Open.command(), Some(ShellCommand::Open));
        assert_eq!(Item::Device.command(), None);
        assert_eq!(Item::Device.kind(), Kind::Text);
        assert_eq!(Item::Profile.kind(), Kind::Text);
    }

    #[test]
    fn two_items_open_the_settings_window() {
        // "Settings…" has the key equivalent of the application menu. It
        // keeps the pane that was visible last. "About and licence notices"
        // opens the pane "About" (decision 0029, section 19).
        assert_eq!(Item::Settings.key(), ",");
        assert_eq!(Item::Settings.kind(), Kind::Command);
        assert_eq!(
            Item::Settings.command(),
            Some(ShellCommand::OpenSettings(None))
        );
        assert_eq!(Item::About.key(), "");
        assert_eq!(Item::About.kind(), Kind::Command);
        assert_eq!(
            Item::About.command(),
            Some(ShellCommand::OpenSettings(Some(SettingsPane::About)))
        );
        // The order of decision 0029: "Settings…" after "Open OpenCraft", a
        // separator, "About and licence notices", a separator, and "Quit".
        assert_eq!(Item::Settings.tag(), Item::Open.tag() + 1);
        assert_eq!(ITEMS[Item::Settings.tag() as usize + 1], Item::Separator);
        assert_eq!(Item::About.tag(), Item::Settings.tag() + 2);
        assert_eq!(ITEMS[Item::About.tag() as usize + 1], Item::Separator);
        assert_eq!(Item::Quit.tag() as usize, ITEMS.len() - 1);
    }

    #[test]
    fn the_menu_shows_the_state() {
        let state = MenuState::default();
        assert_eq!(
            state.title(Item::Device).as_deref(),
            Some("Device: not connected")
        );
        // No profile line before the engine sends the profiles.
        assert_eq!(state.title(Item::Profile), None);
    }

    #[test]
    fn each_item_has_a_tag() {
        for (index, &item) in ITEMS.iter().enumerate() {
            if item != Item::Separator {
                assert_eq!(item.tag(), index as isize);
                assert_eq!(Item::from_tag(item.tag()), Some(item));
            }
        }
        assert_eq!(Item::from_tag(-1), None);
        assert_eq!(Item::from_tag(ITEMS.len() as isize), None);
    }
}
