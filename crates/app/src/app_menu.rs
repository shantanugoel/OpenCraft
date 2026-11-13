//! The menus of the application, their key commands, and the commands of
//! the command palette (decision 0029, sections 13 and 15).
//!
//! One table, [`commands`], holds each command of the window: its title, its
//! action, its place in a menu, its name in the pane "Shortcuts" of the
//! Settings window, and its place in the command palette. The menus, the
//! pane "Shortcuts", and the palette read this table. A task that adds a
//! command adds one row here, and the three places show it.
//!
//! The quit path is the one of decision 0020. The application menu also has
//! "About OpenCraft" and "Settings…", which open the Settings window. The
//! Edit menu has Undo and Redo with the name of the step (decision 0030).
//! The Edit menu also has Copy, Paste, and Clear for the selected control
//! (#187), and "Search…", which opens the command palette (#192). The View
//! menu shows and hides the two side panes of the window. The title of each
//! Edit and View command agrees with the state, thus the shell sets the
//! menus again after each change.

use crate::settings::Panes;
use gpui_kit::{actions, assets::IconName, Action, App, KeyBinding, Menu, MenuItem};

actions!(
    opencraft,
    [
        Quit,
        ToggleSidebar,
        ToggleInspector,
        OpenSettings,
        OpenAbout,
        CloseWindow,
        Undo,
        Redo,
        Copy,
        Paste,
        Clear,
        CopyToAllPages,
        PasteNotPossible,
        Search,
        AddPage,
        PageSettings,
        DeviceSettings,
        AddProfile,
        SelectPrevious,
        SelectNext,
        OpenInInspector
    ]
);

/// The key context of the root element of the window. The key commands of
/// the window operate only when the focus is in this element, so that a
/// focused text field keeps its own keys (`docs/kb/gpui.md`, "Key commands
/// in the window").
pub const WORKSPACE: &str = "Workspace";

/// The key context of the root element of the Settings window.
pub const SETTINGS: &str = "Settings";

/// The key context of the command palette. The arrow keys and ⌘↵ of its
/// search field go to the palette, not to the field.
pub const PALETTE: &str = "CommandPalette";

/// The key context predicate of the search field of the palette.
const PALETTE_FIELD: &str = "CommandPalette > Input";

/// The key context of the root element of the first start window (decision
/// 0029, section 18).
pub const FIRST_START: &str = "FirstStart";

/// The title of the View command for the sidebar.
pub fn sidebar_title(visible: bool) -> &'static str {
    if visible {
        "Hide Sidebar"
    } else {
        "Show Sidebar"
    }
}

/// The title of the View command for the right panel.
pub fn inspector_title(visible: bool) -> &'static str {
    if visible {
        "Hide Inspector"
    } else {
        "Show Inspector"
    }
}

/// The title of the Undo command: "Undo" and the name of the step, for
/// example "Undo Set Title". With no step, "Undo".
pub fn undo_title(step: Option<&str>) -> String {
    step.map_or_else(|| "Undo".to_string(), |name| format!("Undo {name}"))
}

/// The title of the Redo command. See [`undo_title`].
pub fn redo_title(step: Option<&str>) -> String {
    step.map_or_else(|| "Redo".to_string(), |name| format!("Redo {name}"))
}

/// The names of the steps that the Edit menu shows. While a text field has
/// the focus, the commands go to the field, thus the menu shows no name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditMenu {
    pub undo: Option<String>,
    pub redo: Option<String>,
}

/// A menu of the menu bar, in the order of the menu bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuName {
    App,
    Edit,
    View,
}

impl MenuName {
    const ALL: [MenuName; 3] = [MenuName::App, MenuName::Edit, MenuName::View];

    fn title(self) -> &'static str {
        match self {
            MenuName::App => "OpenCraft",
            MenuName::Edit => "Edit",
            MenuName::View => "View",
        }
    }
}

/// One command of the window: one row of [`commands`].
pub struct AppCommand {
    /// The title in the menu and in the command palette, for example
    /// "Undo Set Title" or "Hide Sidebar".
    pub title: String,
    /// The icon in the command palette.
    pub icon: IconName,
    pub action: Box<dyn Action>,
    /// The menu, and the part of the menu: a separator goes between two
    /// parts. `None`: the command is not in a menu.
    pub menu: Option<(MenuName, u8)>,
    /// The name in the pane "Shortcuts" (decision 0029, section 17). The
    /// pane shows the command only if it also has a key binding.
    pub shortcut: Option<&'static str>,
    /// The place in the group "Commands" of the command palette, from 0.
    /// `None`: the palette does not show the command.
    pub palette: Option<u8>,
}

impl AppCommand {
    fn new(title: impl Into<String>, icon: IconName, action: impl Action) -> Self {
        Self {
            title: title.into(),
            icon,
            action: Box::new(action),
            menu: None,
            shortcut: None,
            palette: None,
        }
    }

    fn menu(mut self, menu: MenuName, part: u8) -> Self {
        self.menu = Some((menu, part));
        self
    }

    fn shortcut(mut self, name: &'static str) -> Self {
        self.shortcut = Some(name);
        self
    }

    fn palette(mut self, place: u8) -> Self {
        self.palette = Some(place);
        self
    }
}

/// The commands of the window, with the titles for the state of the panes
/// and for the steps of the history. The order of this table is the order
/// of the pane "Shortcuts" (decision 0029, section 17). The palette shows
/// the commands in the order of their places: first the commands of
/// section 15, then the other commands. The palette does not show
/// "Search…": it is the palette.
pub fn commands(panes: &Panes, edit: &EditMenu) -> Vec<AppCommand> {
    use IconName as I;
    use MenuName::{App as AppMenu, Edit, View};
    vec![
        AppCommand::new("Add a page", I::Plus, AddPage).palette(0),
        AppCommand::new("Page settings", I::Settings, PageSettings).palette(1),
        AppCommand::new("Device settings", I::Grid3x2, DeviceSettings).palette(2),
        AppCommand::new("Add a profile", I::Plus, AddProfile).palette(3),
        AppCommand::new("Search…", I::Search, Search)
            .menu(Edit, 2)
            .shortcut("Search"),
        AppCommand::new(undo_title(edit.undo.as_deref()), I::Undo2, Undo)
            .menu(Edit, 0)
            .shortcut("Undo")
            .palette(7),
        AppCommand::new(redo_title(edit.redo.as_deref()), I::Redo2, Redo)
            .menu(Edit, 0)
            .shortcut("Redo")
            .palette(8),
        AppCommand::new("Copy", I::Copy, Copy)
            .menu(Edit, 1)
            .shortcut("Copy the control")
            .palette(9),
        AppCommand::new("Paste", I::ClipboardPaste, Paste)
            .menu(Edit, 1)
            .shortcut("Paste on the control")
            .palette(10),
        AppCommand::new("Clear", I::Eraser, Clear)
            .menu(Edit, 1)
            .shortcut("Clear the control")
            .palette(11),
        AppCommand::new("Copy to All Pages", I::CopyPlus, CopyToAllPages).palette(12),
        AppCommand::new(
            sidebar_title(panes.sidebar_visible),
            I::PanelLeft,
            ToggleSidebar,
        )
        .menu(View, 0)
        .shortcut("Hide or show the sidebar")
        .palette(5),
        AppCommand::new(
            inspector_title(panes.inspector_visible),
            I::PanelRight,
            ToggleInspector,
        )
        .menu(View, 0)
        .shortcut("Hide or show the inspector")
        .palette(6),
        AppCommand::new("Settings…", I::Settings, OpenSettings)
            .menu(AppMenu, 1)
            .shortcut("Settings")
            .palette(4),
        AppCommand::new("About OpenCraft", I::Info, OpenAbout)
            .menu(AppMenu, 0)
            .palette(13),
        AppCommand::new("Quit OpenCraft", I::Power, Quit)
            .menu(AppMenu, 2)
            .shortcut("Quit OpenCraft")
            .palette(14),
    ]
}

/// The key commands that the Settings window lists, in the order of the
/// list (decision 0029, section 17). The window shows the keys from the key
/// bindings, and shows only a command that has a binding.
pub fn shortcuts() -> Vec<(&'static str, Box<dyn Action>)> {
    commands(&Panes::default(), &EditMenu::default())
        .into_iter()
        .filter_map(|command| Some((command.shortcut?, command.action)))
        .collect()
}

/// Bind the keys and set the menus. Cmd-Q and the menu item start the quit
/// path (decision 0020).
pub fn install(cx: &mut App, panes: &Panes) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        // Settings: the key command works in each window, also in the
        // Settings window (decision 0029, section 13).
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-w", CloseWindow, Some(SETTINGS)),
        KeyBinding::new("cmd-w", CloseWindow, Some(FIRST_START)),
        KeyBinding::new("ctrl-cmd-s", ToggleSidebar, Some(WORKSPACE)),
        KeyBinding::new("alt-cmd-i", ToggleInspector, Some(WORKSPACE)),
        // A focused text field has its own bindings for these keys, in a
        // deeper key context: it keeps them (`docs/kb/gpui.md`).
        KeyBinding::new("cmd-z", Undo, Some(WORKSPACE)),
        KeyBinding::new("cmd-shift-z", Redo, Some(WORKSPACE)),
        KeyBinding::new("cmd-c", Copy, Some(WORKSPACE)),
        // The window has a handler for Paste only when a paste is possible
        // (the Edit menu item is then enabled). Else the key goes on to the
        // binding before it, which tells the user why (`docs/kb/gpui.md`).
        KeyBinding::new("cmd-v", PasteNotPossible, Some(WORKSPACE)),
        KeyBinding::new("cmd-v", Paste, Some(WORKSPACE)),
        // The Delete key of a Mac keyboard. The Edit menu shows the first
        // binding of an action, thus Clear has only this one.
        KeyBinding::new("backspace", Clear, Some(WORKSPACE)),
        // A focused text field has no binding for ⌘K, thus ⌘K opens the
        // palette also from a field (`docs/kb/gpui.md`).
        KeyBinding::new("cmd-k", Search, Some(WORKSPACE)),
        // The search field of the palette has its own bindings for these
        // keys, at the same depth. Of two bindings at one depth, the later
        // one wins (`docs/kb/gpui.md`), and the kit binds its keys first
        // (`gpui_kit::init`).
        KeyBinding::new("up", SelectPrevious, Some(PALETTE_FIELD)),
        KeyBinding::new("down", SelectNext, Some(PALETTE_FIELD)),
        KeyBinding::new("ctrl-p", SelectPrevious, Some(PALETTE_FIELD)),
        KeyBinding::new("ctrl-n", SelectNext, Some(PALETTE_FIELD)),
        KeyBinding::new("cmd-enter", OpenInInspector, Some(PALETTE_FIELD)),
    ]);
    set_menus(cx, panes, &EditMenu::default());
}

/// Set the menus again, with the Edit titles for the steps and the View
/// titles for the state of the panes. macOS enables an item only if the
/// focused element of the window has a handler for its action: see
/// `DeviceView::render`.
pub fn set_menus(cx: &mut App, panes: &Panes, edit: &EditMenu) {
    cx.set_menus(menus(panes, edit));
}

/// The menus of the menu bar, from [`commands`]: the commands of each menu
/// in the order of their parts, with a separator between two parts.
pub fn menus(panes: &Panes, edit: &EditMenu) -> Vec<Menu> {
    let mut commands: Vec<_> = commands(panes, edit)
        .into_iter()
        .filter_map(|command| Some((command.menu?, command)))
        .collect();
    // A stable sort keeps the order of the table in each part.
    commands.sort_by_key(|((_, part), _)| *part);
    MenuName::ALL
        .into_iter()
        .map(|menu| {
            let mut items = Vec::new();
            let mut last = None;
            for ((_, part), command) in commands.iter().filter(|((m, _), _)| *m == menu) {
                if last.is_some_and(|last| last != *part) {
                    items.push(MenuItem::separator());
                }
                last = Some(*part);
                items.push(MenuItem::Action {
                    name: command.title.clone().into(),
                    action: command.action.boxed_clone(),
                    os_action: None,
                    checked: false,
                    disabled: false,
                });
            }
            Menu::new(menu.title()).items(items)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names of the items of each menu, with "-" for a separator.
    fn names(menus: &[Menu]) -> Vec<(String, Vec<String>)> {
        menus
            .iter()
            .map(|menu| {
                let items = menu
                    .items
                    .iter()
                    .map(|item| match item {
                        MenuItem::Action { name, .. } => name.to_string(),
                        _ => "-".to_string(),
                    })
                    .collect();
                (menu.name.to_string(), items)
            })
            .collect()
    }

    #[test]
    fn the_menus_come_from_the_table_of_commands() {
        let edit = EditMenu {
            undo: Some("Set Title".into()),
            redo: None,
        };
        let menus = menus(&Panes::default(), &edit);
        assert_eq!(
            names(&menus),
            [
                (
                    "OpenCraft".to_string(),
                    vec![
                        "About OpenCraft".to_string(),
                        "-".into(),
                        "Settings…".into(),
                        "-".into(),
                        "Quit OpenCraft".into(),
                    ]
                ),
                (
                    "Edit".to_string(),
                    vec![
                        "Undo Set Title".to_string(),
                        "Redo".into(),
                        "-".into(),
                        "Copy".into(),
                        "Paste".into(),
                        "Clear".into(),
                        "-".into(),
                        "Search…".into(),
                    ]
                ),
                (
                    "View".to_string(),
                    vec!["Hide Sidebar".to_string(), "Hide Inspector".into()]
                ),
            ]
        );
    }

    #[test]
    fn each_command_has_one_place_in_the_palette_or_none() {
        let commands = commands(&Panes::default(), &EditMenu::default());
        let mut places: Vec<u8> = commands.iter().filter_map(|c| c.palette).collect();
        places.sort();
        let count = places.len();
        places.dedup();
        assert_eq!(places.len(), count, "Two commands have one place");
        // The palette does not show itself.
        assert!(commands
            .iter()
            .any(|c| c.title == "Search…" && c.palette.is_none()));
    }
}
