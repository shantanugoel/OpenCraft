//! The check of the settings of a requested action.
//!
//! The window cannot make a binding with a kind that does not exist or with
//! a value that its form refuses. A request must not make one either. The
//! rules come only from the descriptions of the kinds (decision 0014): the
//! fields, their input types, and the limits. The text form of a hotkey has
//! one reader, [`opencraft_actions::hotkey::Hotkey`].

use super::protocol as w;
use opencraft_actions::hotkey::Hotkey;
use opencraft_engine as e;

/// The key of the delay of a step of a timed list. [`e::Field::step_delay`]
/// has the same key and the limits.
const DELAY_KEY: &str = "delay_before_ms";
const DELAY_MAX: i64 = 10_000;

/// Return the text of the first problem of `action`, or `Ok`.
pub(super) fn action(action: &e::Action, kinds: &[w::Kind]) -> Result<(), String> {
    check(action, kinds, false)
}

fn check(action: &e::Action, kinds: &[w::Kind], timed: bool) -> Result<(), String> {
    let kind = kinds
        .iter()
        .find(|k| k.kind == action.kind)
        .ok_or_else(|| format!("The action kind \"{}\" does not exist.", action.kind))?;
    for (key, value) in &action.settings {
        if timed && key == DELAY_KEY {
            match value {
                e::Value::Integer(ms) if (0..=DELAY_MAX).contains(ms) => continue,
                _ => {
                    return Err(format!(
                        "The setting \"{key}\" must be a number from 0 to {DELAY_MAX}."
                    ))
                }
            }
        }
        match kind.fields.iter().find(|f| f.key == *key) {
            Some(field) => value_of(&field.input, value, kinds)
                .map_err(|why| format!("The setting \"{key}\" of \"{}\": {why}", kind.kind))?,
            // The key of a bundle identifier that the field of an application
            // stores next to the path.
            None if kind.fields.iter().any(|f| {
                matches!(&f.input, w::Input::Path { kind: w::PathKind::Application { bundle_id } } if bundle_id == key)
            }) =>
            {
                if !matches!(value, e::Value::Text(_)) {
                    return Err(format!("The setting \"{key}\" must be text."));
                }
            }
            None => {
                return Err(format!(
                    "The action kind \"{}\" has no setting \"{key}\".",
                    kind.kind
                ))
            }
        }
    }
    Ok(())
}

fn value_of(input: &w::Input, value: &e::Value, kinds: &[w::Kind]) -> Result<(), String> {
    use e::Value as V;
    use w::Input as I;
    let wrong = |what: &str| Err(format!("the value must be {what}."));
    match (input, value) {
        (
            I::Text | I::LongText | I::RuntimeChoice | I::Profile | I::Folder | I::Path { .. },
            V::Text(_),
        ) => Ok(()),
        (I::Text | I::LongText | I::RuntimeChoice | I::Profile | I::Folder, _) => wrong("text"),
        (I::Path { .. }, _) => wrong("a file path as text"),
        (I::Integer { min, max }, V::Integer(n)) if (min..=max).contains(&n) => Ok(()),
        (I::Integer { min, max }, _) => wrong(&format!("a number from {min} to {max}")),
        (I::Choice { options }, V::Text(text)) if options.iter().any(|o| o.value == *text) => {
            Ok(())
        }
        (I::Choice { options }, _) => wrong(&format!(
            "one of: {}",
            options
                .iter()
                .map(|o| o.value.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        (I::RuntimeChoiceList, V::List(items)) if items.iter().all(|v| matches!(v, V::Text(_))) => {
            Ok(())
        }
        (I::RuntimeChoiceList, _) => wrong("a list of text"),
        (I::Colour, V::Text(text)) if text.parse::<e::Color>().is_ok() => Ok(()),
        (I::Colour, _) => wrong("a colour of the form #RRGGBB"),
        (I::Toggle, V::Boolean(_)) => Ok(()),
        (I::Toggle, _) => wrong("true or false"),
        (I::Hotkey, V::Text(text)) if text.parse::<Hotkey>().is_ok() => Ok(()),
        (I::Hotkey, _) => wrong("a hotkey, for example shift+cmd+4"),
        (I::Action, _) => {
            let action = e::Action::from_value(value)
                .ok_or("the value must be an action with a kind and settings.")?;
            check(&action, kinds, false)
        }
        (I::ActionList, _) => list(value, kinds, false),
        (I::TimedActionList, _) => list(value, kinds, true),
    }
}

fn list(value: &e::Value, kinds: &[w::Kind], timed: bool) -> Result<(), String> {
    let actions = e::Action::list_from_value(value)
        .ok_or("the value must be a list of actions with a kind and settings.")?;
    actions
        .iter()
        .try_for_each(|action| check(action, kinds, timed))
}
