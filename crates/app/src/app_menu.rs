//! The menus of the application and their key commands (decision 0029,
//! section 13).
//!
//! The quit path is the one of decision 0020. The View menu shows and hides
//! the two side panes of the window. The title of each View command agrees
//! with the state of its pane, thus the shell sets the menus again after
//! each change.

use crate::settings::Panes;
use gpui_kit::{actions, App, KeyBinding, Menu, MenuItem};

actions!(opencraft, [Quit, ToggleSidebar, ToggleInspector]);

/// The key context of the root element of the window. The key commands of
/// the window operate only when the focus is in this element, so that a
/// focused text field keeps its own keys (`docs/kb/gpui.md`, "Key commands
/// in the window").
pub const WORKSPACE: &str = "Workspace";

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

/// Bind the keys and set the menus. Cmd-Q and the menu item start the quit
/// path (decision 0020).
pub fn install(cx: &mut App, panes: &Panes) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("ctrl-cmd-s", ToggleSidebar, Some(WORKSPACE)),
        KeyBinding::new("alt-cmd-i", ToggleInspector, Some(WORKSPACE)),
    ]);
    set_menus(cx, panes);
}

/// Set the menus again, with the View titles for the state of the panes.
pub fn set_menus(cx: &mut App, panes: &Panes) {
    cx.set_menus([
        Menu::new("OpenCraft").items([MenuItem::action("Quit OpenCraft", Quit)]),
        Menu::new("View").items([
            MenuItem::action(sidebar_title(panes.sidebar_visible), ToggleSidebar),
            MenuItem::action(inspector_title(panes.inspector_visible), ToggleInspector),
        ]),
    ]);
}
