//! Convert toolkit key names to the text form of decision 0014.

use gpui_kit::Keystroke;

/// Return no value for an unsupported key or the cancel key.
pub(super) fn hotkey(stroke: &Keystroke) -> Option<String> {
    hotkey_with_sides(stroke, side_flags())
}

fn hotkey_with_sides(stroke: &Keystroke, sides: u64) -> Option<String> {
    let m = stroke.modifiers;
    if stroke.key == "escape" && !m.modified() {
        return None;
    }
    // GPUI on macOS folds Shift into printable punctuation and digits.
    let shifted = matches!(
        stroke.key.as_str(),
        "!" | "@"
            | "#"
            | "$"
            | "%"
            | "^"
            | "&"
            | "*"
            | "("
            | ")"
            | "_"
            | "+"
            | "{"
            | "}"
            | "|"
            | ":"
            | "\""
            | "<"
            | ">"
            | "?"
            | "~"
    );
    let lower = stroke.key.to_ascii_lowercase();
    let key = match lower.as_str() {
        "!" => "1",
        "@" => "2",
        "#" => "3",
        "$" => "4",
        "%" => "5",
        "^" => "6",
        "&" => "7",
        "*" => "8",
        "(" => "9",
        ")" => "0",
        "enter" | "return" => "return",
        "backspace" => "delete",
        "delete" | "forwarddelete" => "forwarddelete",
        " " | "space" => "space",
        "-" | "_" | "minus" => "minus",
        "=" | "+" | "equal" => "equal",
        "[" | "{" | "leftbracket" => "leftbracket",
        "]" | "}" | "rightbracket" => "rightbracket",
        "\\" | "|" | "backslash" => "backslash",
        ";" | ":" | "semicolon" => "semicolon",
        "'" | "\"" | "quote" => "quote",
        "," | "<" | "comma" => "comma",
        "." | ">" | "period" => "period",
        "/" | "?" | "slash" => "slash",
        "`" | "~" | "grave" => "grave",
        "tab" | "escape" | "left" | "right" | "up" | "down" | "home" | "end" | "pageup"
        | "pagedown" => lower.as_str(),
        s if s.len() == 1 && s.as_bytes()[0].is_ascii_alphanumeric() => s,
        s if s
            .strip_prefix('f')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=20).contains(&n)) =>
        {
            s
        }
        _ => return None,
    };
    let mut modifiers = m;
    modifiers.shift |= shifted;
    let mut parts = modifier_parts(modifiers, sides);
    parts.push(key);
    Some(parts.join("+"))
}

/// The short text of a hotkey value for the user, for example "⇧⌘A" for
/// `shift+cmd+a`: the modifier symbols in the order of macOS, and then the
/// key (decision 0029, section 10.5). A value of a different form stays as
/// it is.
pub(super) fn symbols(value: &str) -> String {
    let mut parts: Vec<&str> = value.split('+').collect();
    let names = [
        "ctrl",
        "rightctrl",
        "alt",
        "rightalt",
        "shift",
        "rightshift",
        "cmd",
        "rightcmd",
        "fn",
    ];
    let key = if parts.last().is_some_and(|key| names.contains(key)) {
        ""
    } else {
        let Some(key) = parts.pop().filter(|k| !k.is_empty()) else {
            return value.to_string();
        };
        key
    };
    let mut out: Vec<String> = Vec::new();
    for (name, symbol) in [
        ("ctrl", "⌃"),
        ("rightctrl", "R⌃"),
        ("alt", "⌥"),
        ("rightalt", "R⌥"),
        ("shift", "⇧"),
        ("rightshift", "R⇧"),
        ("cmd", "⌘"),
        ("rightcmd", "R⌘"),
        ("fn", "Fn"),
    ] {
        if parts.contains(&name) {
            out.push(symbol.into());
        }
    }
    if parts.iter().any(|p| !names.contains(p)) {
        return value.to_string();
    }
    let key = match key {
        "" => String::new(),
        "return" => "↩".to_string(),
        "tab" => "⇥".to_string(),
        "space" => "Space".to_string(),
        "delete" => "⌫".to_string(),
        "forwarddelete" => "⌦".to_string(),
        "escape" => "⎋".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        "minus" => "-".to_string(),
        "equal" => "=".to_string(),
        "leftbracket" => "[".to_string(),
        "rightbracket" => "]".to_string(),
        "backslash" => "\\".to_string(),
        "semicolon" => ";".to_string(),
        "quote" => "'".to_string(),
        "comma" => ",".to_string(),
        "period" => ".".to_string(),
        "slash" => "/".to_string(),
        "grave" => "`".to_string(),
        other if other.len() == 1 => other.to_uppercase(),
        other => {
            let mut text = other.to_string();
            text[..1].make_ascii_uppercase();
            text
        }
    };
    out.push(key);
    out.concat()
}

/// GPUI combines the left and right sides. Read the flags of the current
/// local AppKit event, not a global monitor. This needs no new permission.
#[cfg(all(target_os = "macos", not(test)))]
fn side_flags() -> u64 {
    use objc2::{class, msg_send, runtime::AnyObject};
    // SAFETY: this runs on the window thread during an AppKit event. The
    // application owns the event for the duration of this call.
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let event: *mut AnyObject = msg_send![app, currentEvent];
        if event.is_null() {
            0
        } else {
            msg_send![event, modifierFlags]
        }
    }
}

#[cfg(any(not(target_os = "macos"), test))]
fn side_flags() -> u64 {
    0
}

fn modifier_parts(m: gpui_kit::Modifiers, sides: u64) -> Vec<&'static str> {
    let mut parts = Vec::new();
    // IOLLEvent.h device-dependent flags. No side information means the
    // legacy (left) spelling, also on platforms without AppKit.
    for (on, left, right, left_name, right_name) in [
        (m.control, 0x01, 0x2000, "ctrl", "rightctrl"),
        (m.alt, 0x20, 0x40, "alt", "rightalt"),
        (m.shift, 0x02, 0x04, "shift", "rightshift"),
        (m.platform, 0x08, 0x10, "cmd", "rightcmd"),
    ] {
        if on {
            if sides & right == 0 || sides & left != 0 {
                parts.push(left_name);
            }
            if sides & right != 0 {
                parts.push(right_name);
            }
        }
    }
    if m.function {
        parts.push("fn");
    }
    parts
}

/// A modifier-only chord completes when all its modifiers are released.
/// Keep the last chord on a press, not its smaller release states.
#[derive(Default)]
pub(super) struct ModifierRecorder {
    held: Vec<&'static str>,
    candidate: Option<String>,
}

impl ModifierRecorder {
    pub(super) fn changed(&mut self, m: gpui_kit::Modifiers) -> Option<String> {
        self.update(m, side_flags())
    }

    fn update(&mut self, m: gpui_kit::Modifiers, sides: u64) -> Option<String> {
        let parts = modifier_parts(m, sides);
        if parts.is_empty() {
            self.held.clear();
            return self.candidate.take();
        }
        if parts.iter().any(|p| !self.held.contains(p)) {
            self.candidate = Some(parts.join("+"));
        }
        self.held = parts;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::Modifiers;

    #[test]
    fn names_and_modifier_order() {
        for (key, expected) in [
            ("a", "a"),
            ("Z", "z"),
            ("4", "4"),
            ("enter", "return"),
            ("tab", "tab"),
            ("space", "space"),
            ("backspace", "delete"),
            ("delete", "forwarddelete"),
            ("escape", "escape"),
            ("left", "left"),
            ("right", "right"),
            ("up", "up"),
            ("down", "down"),
            ("home", "home"),
            ("end", "end"),
            ("pageup", "pageup"),
            ("pagedown", "pagedown"),
            ("-", "minus"),
            ("=", "equal"),
            ("[", "leftbracket"),
            ("]", "rightbracket"),
            ("\\", "backslash"),
            (";", "semicolon"),
            ("'", "quote"),
            (",", "comma"),
            (".", "period"),
            ("/", "slash"),
            ("`", "grave"),
        ] {
            let stroke = Keystroke {
                key: key.into(),
                key_char: None,
                modifiers: Modifiers {
                    control: true,
                    alt: true,
                    shift: true,
                    platform: true,
                    ..Default::default()
                },
            };
            assert_eq!(
                hotkey(&stroke),
                Some(format!("ctrl+alt+shift+cmd+{expected}"))
            );
        }
        for n in 1..=20 {
            assert_eq!(
                hotkey(&Keystroke {
                    key: format!("f{n}"),
                    key_char: None,
                    modifiers: Modifiers::default()
                }),
                Some(format!("f{n}"))
            );
        }
        for key in ["escape", "f21", "fn", "capslock", "é"] {
            assert_eq!(
                hotkey(&Keystroke {
                    key: key.into(),
                    key_char: None,
                    modifiers: Modifiers::default()
                }),
                None
            );
        }
        assert_eq!(
            hotkey(&Keystroke::parse("shift-cmd-4").unwrap()),
            Some("shift+cmd+4".into())
        );
        assert_eq!(
            hotkey(&Keystroke::parse("cmd-$").unwrap()),
            Some("shift+cmd+4".into())
        );
        for (symbol, key) in [
            ("!", "1"),
            ("@", "2"),
            ("#", "3"),
            ("%", "5"),
            ("^", "6"),
            ("&", "7"),
            ("*", "8"),
            ("(", "9"),
            (")", "0"),
            ("_", "minus"),
            ("+", "equal"),
            ("{", "leftbracket"),
            ("}", "rightbracket"),
            ("|", "backslash"),
            (":", "semicolon"),
            ("\"", "quote"),
            ("<", "comma"),
            (">", "period"),
            ("?", "slash"),
            ("~", "grave"),
        ] {
            assert_eq!(
                hotkey(&Keystroke {
                    key: symbol.into(),
                    key_char: None,
                    modifiers: Modifiers::default()
                }),
                Some(format!("shift+{key}"))
            );
        }
    }

    #[test]
    fn right_sides_fn_and_modifier_only_chords() {
        let m = Modifiers {
            control: true,
            alt: true,
            shift: true,
            platform: true,
            function: true,
        };
        let stroke = Keystroke {
            key: "a".into(),
            key_char: None,
            modifiers: m,
        };
        assert_eq!(
            hotkey_with_sides(&stroke, 0x2054),
            Some("rightctrl+rightalt+rightshift+rightcmd+fn+a".into())
        );
        let mut recorder = ModifierRecorder::default();
        assert_eq!(recorder.update(m, 0x2054), None);
        assert_eq!(
            recorder.update(
                Modifiers {
                    function: true,
                    ..Default::default()
                },
                0
            ),
            None
        );
        assert_eq!(
            recorder.update(Modifiers::default(), 0),
            Some("rightctrl+rightalt+rightshift+rightcmd+fn".into())
        );
        assert_eq!(recorder.update(Modifiers::default(), 0), None);
        assert_eq!(
            recorder.update(
                Modifiers {
                    function: true,
                    ..Default::default()
                },
                0
            ),
            None
        );
        assert_eq!(recorder.update(Modifiers::default(), 0), Some("fn".into()));
        assert_eq!(
            symbols("rightctrl+rightalt+rightshift+rightcmd+fn+a"),
            "R⌃R⌥R⇧R⌘FnA"
        );
        assert_eq!(symbols("rightcmd"), "R⌘");
        assert_eq!(symbols("fn"), "Fn");
    }

    #[test]
    fn the_symbols_of_a_hotkey_are_in_the_order_of_macos() {
        assert_eq!(symbols("shift+cmd+a"), "⇧⌘A");
        assert_eq!(symbols("ctrl+alt+shift+cmd+return"), "⌃⌥⇧⌘↩");
        assert_eq!(symbols("cmd+f12"), "⌘F12");
        assert_eq!(symbols("space"), "Space");
        assert_eq!(symbols(""), "");
        assert_eq!(symbols("hyper+a"), "hyper+a");
    }
}
