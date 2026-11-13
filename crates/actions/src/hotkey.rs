//! The text form of a hotkey.
//!
//! The form is in `docs/decisions/0014-action-settings-description.md`: the
//! modifiers and an optional key name, in small letters with `+` between
//! them. Example: `shift+cmd+4` or `rightctrl`.

use std::fmt;
use std::str::FromStr;

/// A key with modifier keys.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub cmd: bool,
    pub right_ctrl: bool,
    pub right_alt: bool,
    pub right_shift: bool,
    pub right_cmd: bool,
    pub function: bool,
    /// The name of the key, in small letters. Empty for modifiers only.
    /// See [`KEY_NAMES`].
    pub key: String,
}

/// The text is not a hotkey.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseHotkeyError {
    #[error("the hotkey has no key")]
    NoKey,
    #[error("the hotkey has more than one key: {0}")]
    TwoKeys(String),
    #[error("the key {0} is not known")]
    UnknownKey(String),
}

/// Each key name that a hotkey can have, apart from one letter, one digit,
/// and `f1` to `f20`.
pub const KEY_NAMES: &[&str] = &[
    "return",
    "tab",
    "space",
    "delete",
    "forwarddelete",
    "escape",
    "left",
    "right",
    "up",
    "down",
    "home",
    "end",
    "pageup",
    "pagedown",
    "minus",
    "equal",
    "leftbracket",
    "rightbracket",
    "backslash",
    "semicolon",
    "quote",
    "comma",
    "period",
    "slash",
    "grave",
];

/// True if `name` is the name of a key.
pub fn is_key_name(name: &str) -> bool {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return c.is_ascii_lowercase() || c.is_ascii_digit();
    }
    if let Some(number) = name.strip_prefix('f') {
        if !number.starts_with('0') {
            if let Ok(n) = number.parse::<u8>() {
                return (1..=20).contains(&n);
            }
        }
    }
    KEY_NAMES.contains(&name)
}

impl FromStr for Hotkey {
    type Err = ParseHotkeyError;

    /// Read a hotkey. Capital letters and spaces around a part are permitted.
    /// The modifiers can be in each order.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut hotkey = Hotkey::default();
        for part in s.split('+') {
            let part = part.trim().to_ascii_lowercase();
            match part.as_str() {
                "" => {}
                "ctrl" => hotkey.ctrl = true,
                "alt" => hotkey.alt = true,
                "shift" => hotkey.shift = true,
                "cmd" => hotkey.cmd = true,
                "rightctrl" => hotkey.right_ctrl = true,
                "rightalt" => hotkey.right_alt = true,
                "rightshift" => hotkey.right_shift = true,
                "rightcmd" => hotkey.right_cmd = true,
                "fn" => hotkey.function = true,
                _ if !hotkey.key.is_empty() => return Err(ParseHotkeyError::TwoKeys(part)),
                _ if is_key_name(&part) => hotkey.key = part,
                _ => return Err(ParseHotkeyError::UnknownKey(part)),
            }
        }
        if hotkey == Hotkey::default() {
            return Err(ParseHotkeyError::NoKey);
        }
        Ok(hotkey)
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        for (on, name) in [
            (self.ctrl, "ctrl"),
            (self.right_ctrl, "rightctrl"),
            (self.alt, "alt"),
            (self.right_alt, "rightalt"),
            (self.shift, "shift"),
            (self.right_shift, "rightshift"),
            (self.cmd, "cmd"),
            (self.right_cmd, "rightcmd"),
            (self.function, "fn"),
        ] {
            if on {
                parts.push(name);
            }
        }
        if !self.key.is_empty() {
            parts.push(&self.key);
        }
        f.write_str(&parts.join("+"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hotkey_goes_to_text_and_back() {
        for text in [
            "a",
            "shift+cmd+4",
            "ctrl+alt+shift+cmd+f12",
            "alt+space",
            "rightctrl+rightalt+rightshift+rightcmd+fn+a",
            "cmd",
            "fn",
            "ctrl+rightctrl",
            "shift+cmd",
        ] {
            let hotkey: Hotkey = text.parse().unwrap();
            assert_eq!(hotkey.to_string(), text);
        }
    }

    #[test]
    fn the_text_can_have_capital_letters_spaces_and_a_different_order() {
        let hotkey: Hotkey = " Shift + CMD+K ".parse().unwrap();
        assert_eq!(hotkey.to_string(), "shift+cmd+k");
    }

    #[test]
    fn an_incorrect_text_is_an_error() {
        assert_eq!("".parse::<Hotkey>(), Err(ParseHotkeyError::NoKey));
        assert_eq!(" + ".parse::<Hotkey>(), Err(ParseHotkeyError::NoKey));
        assert_eq!(
            "cmd+a+b".parse::<Hotkey>(),
            Err(ParseHotkeyError::TwoKeys("b".into()))
        );
        for bad in ["cmd+f0", "f21", "f01", "cmd+é", "win+a", "ab"] {
            assert!(
                matches!(bad.parse::<Hotkey>(), Err(ParseHotkeyError::UnknownKey(_))),
                "{bad}"
            );
        }
    }
}
