//! The check of the settings of a requested action.
//!
//! The window cannot make a binding with a kind that does not exist, with a
//! field that its form does not show, or with a value that its form refuses.
//! A request must not make one either. The rules come only from the
//! descriptions of the kinds (decision 0014): the fields, their input types,
//! their limits, the options, and the field that shows a field (decision
//! 0038). The text form of a hotkey has one reader,
//! [`opencraft_actions::hotkey::Hotkey`]. A field of the type Profile or
//! Folder must name a thing that exists: the window shows only the profiles
//! and the folders that exist.

use super::protocol as w;
use opencraft_actions::hotkey::Hotkey;
use opencraft_engine as e;

/// What the check can see of the state at the place of a request.
pub(super) struct At<'a> {
    pub state: &'a w::State,
    /// The identity of the profile that the request names.
    pub profile: &'a str,
}

/// Return the text of the first problem of `action`, or `Ok`.
pub(super) fn action(action: &e::Action, kinds: &[w::Kind], at: &At) -> Result<(), String> {
    check(action, kinds, at, false)
}

fn check(action: &e::Action, kinds: &[w::Kind], at: &At, timed: bool) -> Result<(), String> {
    let kind = kinds
        .iter()
        .find(|k| k.kind == action.kind)
        .ok_or_else(|| format!("The action kind \"{}\" does not exist.", action.kind))?;
    // The key and the limits of the delay come from the engine, as the form
    // of the window reads them. A second copy here could disagree with it.
    let delay = e::Field::step_delay(0);
    let delay_input = delay.input.clone();
    for (key, value) in &action.settings {
        if timed && key == &delay.key {
            match (value, &delay_input) {
                (e::Value::Integer(ms), e::InputType::Integer { min, max })
                    if (*min..=*max).contains(ms) =>
                {
                    continue
                }
                (_, e::InputType::Integer { min, max }) => {
                    return Err(format!(
                        "The setting \"{key}\" must be a number from {min} to {max}."
                    ))
                }
                _ => unreachable!("the delay of a step is a number"),
            }
        }
        match kind.fields.iter().find(|f| f.key == *key) {
            Some(_) if !shown(kind, &action.settings, key) => {
                return Err(format!(
                    "The setting \"{key}\" of \"{}\" is not shown for these settings. \
                     Read `shown_when` in `kinds`.",
                    kind.kind
                ))
            }
            Some(field) => value_of(&field.input, value, kinds, at)
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

/// True when the form of the window shows the field `key` for these settings
/// (decision 0038). A field with no rule is shown. A value that is not text
/// shows no field.
fn shown(kind: &w::Kind, settings: &e::Settings, key: &str) -> bool {
    kind.shown_when
        .iter()
        .filter(|rule| rule.field == key)
        .all(|rule| {
            let value = match settings.get(&rule.key) {
                Some(e::Value::Text(text)) => Some(text.clone()),
                Some(_) => None,
                // A field that the request does not give has the default of its kind.
                None => kind
                    .fields
                    .iter()
                    .find(|field| field.key == rule.key)
                    .and_then(|field| field.default.as_ref())
                    .and_then(|value| value.as_str().map(str::to_string)),
            };
            value.is_some_and(|value| rule.values.contains(&value))
        })
}

fn value_of(input: &w::Input, value: &e::Value, kinds: &[w::Kind], at: &At) -> Result<(), String> {
    use e::Value as V;
    use w::Input as I;
    let wrong = |what: &str| Err(format!("the value must be {what}."));
    match (input, value) {
        (I::Text | I::LongText | I::RuntimeChoice | I::Path { .. }, V::Text(_)) => Ok(()),
        (I::Text | I::LongText | I::RuntimeChoice, _) => wrong("text"),
        (I::Path { .. }, _) => wrong("a file path as text"),
        // The window shows the profiles and the folders that exist, thus a
        // value must name one. A folder of the active profile must be in the
        // profile: the engine checks a folder of another profile at the
        // place.
        (I::Profile, V::Text(id)) => {
            if at.state.profiles.profiles.iter().any(|p| p.id == *id) {
                Ok(())
            } else {
                wrong(&format!("the identity of a profile, not \"{id}\""))
            }
        }
        (I::Profile, _) => wrong("the identity of a profile as text"),
        (I::Folder, V::Text(id)) => {
            let Ok(folder) = id.parse::<e::FolderId>() else {
                return wrong(&format!("the identity of a folder, not \"{id}\""));
            };
            // The cache lists the folders of the active profile. The engine
            // checks a folder of another profile at the place.
            let active = at.state.profiles.active.as_deref() == Some(at.profile);
            let listed = at
                .state
                .pages
                .folders
                .iter()
                .any(|f| f.id == folder.to_string());
            if !active || listed {
                Ok(())
            } else {
                wrong(&format!("the identity of a folder, not \"{id}\""))
            }
        }
        (I::Folder, _) => wrong("the identity of a folder as text"),
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
            check(&action, kinds, at, false)
        }
        (I::ActionList, _) => list(value, kinds, at, false),
        (I::TimedActionList, _) => list(value, kinds, at, true),
    }
}

fn list(value: &e::Value, kinds: &[w::Kind], at: &At, timed: bool) -> Result<(), String> {
    let actions = e::Action::list_from_value(value)
        .ok_or("the value must be a list of actions with a kind and settings.")?;
    actions
        .iter()
        .try_for_each(|action| check(action, kinds, at, timed))
}
