//! The pane "Shortcuts" (decision 0029, section 17): the key commands of the
//! window. The user cannot change them.

use super::{
    parts::{group, line_row},
    SettingsView,
};
use crate::app_menu;
use gpui_kit::{div, prelude::*, Action, AnyElement, App, Context, Modifiers};

/// One keystroke in the form of the macOS menus: the modifier keys in the
/// order ⌃⌥⇧⌘, and then the key, for example "⌃⌘S".
pub fn keystroke_text(modifiers: &Modifiers, key: &str) -> String {
    let mut text = String::new();
    for (pressed, symbol) in [
        (modifiers.control, '⌃'),
        (modifiers.alt, '⌥'),
        (modifiers.shift, '⇧'),
        (modifiers.platform, '⌘'),
    ] {
        if pressed {
            text.push(symbol);
        }
    }
    match key {
        "backspace" => text.push('⌫'),
        "delete" => text.push('⌦'),
        "escape" => text.push('⎋'),
        "enter" => text.push('↩'),
        "tab" => text.push('⇥'),
        "space" => text.push_str("Space"),
        "up" => text.push('↑'),
        "down" => text.push('↓'),
        "left" => text.push('←'),
        "right" => text.push('→'),
        key => text.push_str(&key.to_uppercase()),
    }
    text
}

/// The keys of the first key binding of `action`, in the form of the menus:
/// "⌃⌘S". `None` if the action has no key binding.
pub fn keys(action: &dyn Action, cx: &App) -> Option<String> {
    let keymap = cx.key_bindings();
    let keymap = keymap.borrow();
    let binding = keymap.bindings_for_action(action).next_back()?;
    let keys: Vec<String> = binding
        .keystrokes()
        .iter()
        .map(|keystroke| keystroke_text(keystroke.modifiers(), keystroke.key()))
        .collect();
    Some(keys.join(" "))
}

/// The rows of the list: the name of each key command and its keys. The list
/// shows only the commands that have a key binding.
pub fn rows(cx: &App) -> Vec<(&'static str, String)> {
    app_menu::shortcuts()
        .into_iter()
        .filter_map(|(name, action)| Some((name, keys(action.as_ref(), cx)?)))
        .collect()
}

impl SettingsView {
    pub(super) fn shortcuts(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("shortcuts")
            .debug_selector(|| "shortcuts".into())
            .child(group(
                rows(cx)
                    .into_iter()
                    .map(|(name, keys)| line_row(name, keys)),
            ))
            .into_any_element()
    }
}
