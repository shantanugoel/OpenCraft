//! Convert toolkit key names to the text form of decision 0014.

use gpui_kit::Keystroke;

/// Return no value for an unsupported key or the cancel key.
pub(super) fn hotkey(stroke: &Keystroke) -> Option<String> {
    let m = stroke.modifiers;
    if m.function || (stroke.key == "escape" && !m.control && !m.alt && !m.shift && !m.platform) {
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
    let mut parts = Vec::new();
    for (set, name) in [
        (m.control, "ctrl"),
        (m.alt, "alt"),
        (m.shift || shifted, "shift"),
        (m.platform, "cmd"),
    ] {
        if set {
            parts.push(name);
        }
    }
    parts.push(key);
    Some(parts.join("+"))
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
}
