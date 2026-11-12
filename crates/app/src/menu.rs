//! The content of the menu of the menu bar item (decision 0020), with no
//! platform code. The module `menu_bar` shows it with AppKit on macOS.
//!
//! To add an item, add it to [`Item`] and to [`ITEMS`], and give it a title.
//! A check mark item also needs a value in [`MenuState`] and a command.

// Only the macOS menu bar item and the tests read the menu.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::shell::ShellCommand;

/// One item of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Device,
    Profile,
    Open,
    ShowDockIcon,
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
    /// An item with a check mark that sends a command.
    Check,
    Separator,
}

/// The items, in their order in the menu.
pub const ITEMS: [Item; 8] = [
    Item::Device,
    Item::Profile,
    Item::Separator,
    Item::Open,
    Item::Separator,
    Item::ShowDockIcon,
    Item::Separator,
    Item::Quit,
];

impl Item {
    pub fn kind(self) -> Kind {
        match self {
            Item::Device | Item::Profile => Kind::Text,
            Item::Open | Item::Quit => Kind::Command,
            Item::ShowDockIcon => Kind::Check,
            Item::Separator => Kind::Separator,
        }
    }

    /// The command that the item sends when the user selects it.
    pub fn command(self) -> Option<ShellCommand> {
        match self {
            Item::Open => Some(ShellCommand::Open),
            Item::ShowDockIcon => Some(ShellCommand::ToggleDockIcon),
            Item::Quit => Some(ShellCommand::Quit),
            Item::Device | Item::Profile | Item::Separator => None,
        }
    }

    /// The key equivalent, with the Command key.
    pub fn key(self) -> &'static str {
        match self {
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
    pub show_dock_icon: bool,
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
            Item::ShowDockIcon => "Show the Dock icon".into(),
            Item::Quit => "Quit OpenCraft".into(),
            Item::Separator => String::new(),
        })
    }

    /// True if a check mark item shows its check mark.
    pub fn checked(&self, item: Item) -> bool {
        match item {
            Item::ShowDockIcon => self.show_dock_icon,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_has_the_items_of_decision_0020() {
        let state = MenuState {
            connected: true,
            profile: Some("Desk".into()),
            show_dock_icon: true,
        };
        let titles: Vec<_> = ITEMS
            .iter()
            .filter(|i| i.kind() != Kind::Separator)
            .map(|&i| state.title(i).unwrap())
            .collect();
        assert_eq!(
            titles,
            [
                "Device: connected",
                "Profile: Desk",
                "Open OpenCraft",
                "Show the Dock icon",
                "Quit OpenCraft"
            ]
        );
        assert!(state.checked(Item::ShowDockIcon));
        assert_eq!(Item::Quit.key(), "q");
        assert_eq!(Item::Quit.command(), Some(ShellCommand::Quit));
        assert_eq!(Item::Open.command(), Some(ShellCommand::Open));
        assert_eq!(
            Item::ShowDockIcon.command(),
            Some(ShellCommand::ToggleDockIcon)
        );
        assert_eq!(Item::Device.command(), None);
        assert_eq!(Item::Device.kind(), Kind::Text);
        assert_eq!(Item::Profile.kind(), Kind::Text);
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
        assert!(!state.checked(Item::ShowDockIcon));
        assert!(!state.checked(Item::Open));
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
