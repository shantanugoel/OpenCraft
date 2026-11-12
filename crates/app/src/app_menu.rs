//! The menus of the application and their key commands (decision 0029,
//! section 13).
//!
//! The quit path is the one of decision 0020. The application menu also has
//! "About OpenCraft" and "Settings…", which open the Settings window. The
//! Edit menu has Undo and Redo with the name of the step (decision 0030).
//! The View menu shows and hides the two side panes of the window. The title
//! of each Edit and View command agrees with the state, thus the shell sets
//! the menus again after each change.

use crate::settings::Panes;
use gpui_kit::{actions, Action, App, KeyBinding, Menu, MenuItem};

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
        Redo
    ]
);

/// The key context of the root element of the window. The key commands of
/// the window operate only when the focus is in this element, so that a
/// focused text field keeps its own keys (`docs/kb/gpui.md`, "Key commands
/// in the window").
pub const WORKSPACE: &str = "Workspace";

/// The key context of the root element of the Settings window.
pub const SETTINGS: &str = "Settings";

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

/// The key commands that the Settings window lists, in the order of the
/// list (decision 0029, section 17). The window shows the keys from the key
/// bindings, and shows only a command that has a binding. A task that adds a
/// key command adds its row here.
pub fn shortcuts() -> Vec<(&'static str, Box<dyn Action>)> {
    vec![
        ("Undo", Box::new(Undo)),
        ("Redo", Box::new(Redo)),
        ("Hide or show the sidebar", Box::new(ToggleSidebar)),
        ("Hide or show the inspector", Box::new(ToggleInspector)),
        ("Settings", Box::new(OpenSettings)),
        ("Quit OpenCraft", Box::new(Quit)),
    ]
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
        KeyBinding::new("ctrl-cmd-s", ToggleSidebar, Some(WORKSPACE)),
        KeyBinding::new("alt-cmd-i", ToggleInspector, Some(WORKSPACE)),
        // A focused text field has its own bindings for these keys, in a
        // deeper key context: it keeps them (`docs/kb/gpui.md`).
        KeyBinding::new("cmd-z", Undo, Some(WORKSPACE)),
        KeyBinding::new("cmd-shift-z", Redo, Some(WORKSPACE)),
    ]);
    set_menus(cx, panes, &EditMenu::default());
}

/// Set the menus again, with the Edit titles for the steps and the View
/// titles for the state of the panes. macOS enables an item only if the
/// focused element of the window has a handler for its action: see
/// `DeviceView::render`.
pub fn set_menus(cx: &mut App, panes: &Panes, edit: &EditMenu) {
    cx.set_menus([
        Menu::new("OpenCraft").items([
            MenuItem::action("About OpenCraft", OpenAbout),
            MenuItem::separator(),
            MenuItem::action("Settings…", OpenSettings),
            MenuItem::separator(),
            MenuItem::action("Quit OpenCraft", Quit),
        ]),
        Menu::new("Edit").items([
            MenuItem::action(undo_title(edit.undo.as_deref()), Undo),
            MenuItem::action(redo_title(edit.redo.as_deref()), Redo),
        ]),
        Menu::new("View").items([
            MenuItem::action(sidebar_title(panes.sidebar_visible), ToggleSidebar),
            MenuItem::action(inspector_title(panes.inspector_visible), ToggleInspector),
        ]),
    ]);
}
