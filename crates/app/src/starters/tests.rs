//! Each starter profile loads, is valid for the model of the first
//! descriptor and for the built-in kinds, and has no path of one computer
//! and no image file (decision 0029, section 18).

use super::*;
use crate::state::gestures;
use opencraft_actions::hotkey::Hotkey;
use opencraft_engine::{
    icon_set, page_preview, profile::default_model, store::SCHEMA_VERSION, DeviceLayout, InputType,
    KindDescription, SurfaceKind, Value,
};

/// The descriptions of the built-in kinds, as the engine sends them.
fn kinds() -> Vec<KindDescription> {
    opencraft_actions::handlers()
        .iter()
        .flat_map(|handler| handler.kinds())
        .collect()
}

/// A problem of the setting `key` = `value` of an action of `kind`, if the
/// value does not agree with the field of the description.
fn setting_problem(kind: &KindDescription, key: &str, value: &Value) -> Option<String> {
    let Some(field) = kind.fields.iter().find(|f| f.key == key) else {
        return Some(format!("{}: no field {key}", kind.name));
    };
    let text = match value {
        Value::Text(text) => Some(text.as_str()),
        _ => None,
    };
    let ok = match (&field.input, value) {
        (InputType::Text | InputType::LongText, Value::Text(t)) => !t.trim().is_empty(),
        (InputType::Integer { min, max }, Value::Integer(n)) => (*min..=*max).contains(n),
        (InputType::Choice { options }, Value::Text(t)) => options.iter().any(|o| o.value == *t),
        (InputType::Toggle, Value::Boolean(_)) => true,
        (InputType::Hotkey, Value::Text(t)) => t.parse::<Hotkey>().is_ok(),
        // An application by its name: `open -a` finds it on each Mac.
        (InputType::Path { .. }, Value::Text(t)) => !t.contains('/') && !t.trim().is_empty(),
        (InputType::Action, value) => Action::from_value(value).is_some(),
        (InputType::ActionList | InputType::TimedActionList, value) => {
            Action::list_from_value(value).is_some()
        }
        _ => false,
    };
    // No path of one computer: no home folder, and no absolute path.
    let personal = text.is_some_and(|t| t.contains("/Users/") || t.starts_with('~'));
    (!ok || personal).then(|| format!("{}: {key} = {value:?}", kind.name))
}

#[test]
fn each_starter_profile_loads_and_is_valid() {
    let kinds = kinds();
    let layout = DeviceLayout::of_default_model();
    let mut names = Vec::new();
    for starter in STARTERS {
        let profile = starter
            .profile()
            .unwrap_or_else(|e| panic!("{}: {e}", starter.name));
        assert!(
            starter
                .text
                .contains(&format!("schema_version = {SCHEMA_VERSION}")),
            "{}: the current schema version",
            starter.name
        );
        assert_eq!(profile.model, default_model(), "{}", starter.name);
        assert!(!profile.name.trim().is_empty(), "{}", starter.name);
        assert_eq!(
            profile.image_paths().count(),
            0,
            "{}: no image file",
            starter.name
        );
        names.push(profile.name.clone());
        for page in profile.pages() {
            let place = format!("{} / {}", starter.name, page.name);
            for (surface, content) in page.surfaces() {
                assert!(layout.surface(surface).is_some(), "{place}: {surface:?}");
                // The name of an icon is in the built-in set. The engine
                // keeps a name that the set does not have, and the surface
                // then shows no icon.
                if let Some(icon) = &content.icon {
                    assert!(
                        icon_set().get(&icon.name).is_some(),
                        "{place}: {surface:?}: the set has no icon {:?}",
                        icon.name
                    );
                }
            }
            for ((control, gesture), _) in page.bindings() {
                assert!(
                    layout.controls.iter().any(|c| c.id == control),
                    "{place}: {control:?}"
                );
                assert!(
                    gestures(control.kind).iter().any(|(g, _)| *g == gesture),
                    "{place}: {control:?} has no {gesture:?}"
                );
            }
        }
        for action in actions(&profile) {
            let kind = kinds
                .iter()
                .find(|k| k.kind == action.kind)
                .unwrap_or_else(|| panic!("{}: no kind {}", starter.name, action.kind));
            for (key, value) in &action.settings {
                if let Some(problem) = setting_problem(kind, key, value) {
                    panic!("{}: {problem}", starter.name);
                }
            }
            // A URL is a web page or a pane of System Settings.
            if let Some(Value::Text(url)) = action.settings.get("url") {
                assert!(
                    url.starts_with("https://") || url.starts_with("x-apple.systempreferences:"),
                    "{}: {url}",
                    starter.name
                );
            }
        }
    }
    names.sort();
    names.dedup();
    assert_eq!(names.len(), STARTERS.len(), "each profile has its own name");
}

#[test]
fn the_cards_have_the_texts_of_the_record_and_empty_is_one_empty_page() {
    let cards: Vec<_> = STARTERS.iter().map(|s| (s.name, s.description)).collect();
    assert_eq!(
        cards,
        [
            ("Everyday Mac", "Apps, web pages, sleep, page change"),
            ("Media and sound", "Play, next track, volume on the knobs"),
            ("Video calls", "Mute toggle, camera, leave the call"),
            ("Empty", "One empty page"),
        ]
    );
    let empty = STARTERS[3].profile().unwrap();
    assert_eq!(empty.pages().len(), 1);
    assert_eq!(empty.pages()[0].surfaces().count(), 0);
    assert_eq!(empty.pages()[0].bindings().count(), 0);
    // The other starter profiles have a title on each key of the first page,
    // and actions.
    for starter in &STARTERS[..3] {
        let profile = starter.profile().unwrap();
        let page = &profile.pages()[profile.first_page()];
        let titled = page
            .surfaces()
            .filter(|(id, c)| id.kind == SurfaceKind::KeyFace && c.label.is_some())
            .count();
        assert_eq!(titled, 10, "{}", starter.name);
        assert!(page.bindings().count() >= 10, "{}", starter.name);
    }
}

#[test]
fn each_key_face_and_bar_panel_of_a_starter_profile_has_an_icon_and_a_title() {
    let layout = DeviceLayout::of_default_model();
    for starter in &STARTERS[..3] {
        let profile = starter.profile().unwrap();
        for page in profile.pages() {
            let place = format!("{} / {}", starter.name, page.name);
            let shown: Vec<_> = layout
                .surfaces
                .iter()
                .filter(|s| matches!(s.id.kind, SurfaceKind::KeyFace | SurfaceKind::BarPanel))
                .map(|s| s.id)
                .collect();
            assert_eq!(shown.len(), 14, "{place}");
            for id in shown {
                let content = page
                    .surfaces()
                    .find(|(surface, _)| *surface == id)
                    .map(|(_, content)| content)
                    .unwrap_or_else(|| panic!("{place}: {id:?} has no face"));
                assert!(
                    content.known_icon().is_some(),
                    "{place}: {id:?} has no icon"
                );
                assert!(content.label.is_some(), "{place}: {id:?} has no title");
                // The icon has the colour of the title, so that the icon
                // and the title show well on the same fill.
                assert_eq!(
                    content.icon.as_ref().and_then(|i| i.color),
                    content.label_style.color,
                    "{place}: {id:?}"
                );
            }
        }
    }
}

#[test]
fn the_engine_makes_the_pictures_of_the_starter_profiles() {
    let layout = DeviceLayout::of_default_model();
    for starter in STARTERS {
        let profile = starter.profile().unwrap();
        let preview = page_preview(&profile.pages()[0], &layout);
        assert_eq!(preview.len(), 14, "{}", starter.name);
    }
}

#[test]
fn the_actions_of_a_profile_include_the_actions_in_a_toggle() {
    let profile = STARTERS[2].profile().unwrap();
    let all = actions(&profile);
    let toggles = all.iter().filter(|a| a.kind == "opencraft.toggle").count();
    // Two toggles on each of three pages, each with two hotkeys.
    assert_eq!(toggles, 6);
    let in_toggles = all.iter().filter(|a| a.kind == "opencraft.hotkey").count();
    assert!(in_toggles >= 12, "{in_toggles}");
}
