//! Each ready profile loads, links one application, and uses only built-in
//! icons and shortcuts that the hotkey parser accepts (issue #340).

use super::*;
use crate::starters::actions;
use opencraft_actions::hotkey::Hotkey;
use opencraft_engine::{
    icon_set, profile::default_model, store::SCHEMA_VERSION, DeviceLayout, FolderId, SurfaceKind,
    Value,
};

#[test]
fn each_ready_profile_loads_and_checks_its_link() {
    let layout = DeviceLayout::of_default_model();
    let kinds: Vec<_> = opencraft_actions::handlers()
        .iter()
        .flat_map(|handler| handler.kinds())
        .collect();
    let mut names = Vec::new();
    for ready in READY {
        let profile = ready
            .profile()
            .unwrap_or_else(|error| panic!("{}: {error}", ready.name));
        assert!(
            ready
                .text
                .contains(&format!("schema_version = {SCHEMA_VERSION}")),
            "{}: schema version {SCHEMA_VERSION}",
            ready.name
        );
        assert_eq!(profile.model, default_model(), "{}", ready.name);
        assert_eq!(profile.name, ready.name);
        assert_eq!(profile.image_paths().count(), 0, "{}: no image", ready.name);
        assert_eq!(profile.applications.len(), 1, "{}: one link", ready.name);
        assert_eq!(profile.applications[0].bundle_id, ready.bundle_id);
        assert_eq!(profile.applications[0].name, ready.application_name);
        names.push(profile.name.clone());

        let mut folder_ids = Vec::new();
        for folder in profile.folders() {
            folder_ids.push(folder.id);
            assert!(!folder.page.name.trim().is_empty(), "{}", ready.name);
            check_surfaces(
                &layout,
                &format!("{} / {}", ready.name, folder.page.name),
                &folder.page,
                false,
            );
        }
        for page in profile.pages() {
            check_surfaces(
                &layout,
                &format!("{} / {}", ready.name, page.name),
                page,
                true,
            );
        }
        let mut closed = false;
        for action in actions(&profile) {
            let kind = kinds
                .iter()
                .find(|kind| kind.kind == action.kind)
                .unwrap_or_else(|| panic!("{}: no kind {}", ready.name, action.kind));
            for (key, value) in &action.settings {
                assert!(
                    kind.fields.iter().any(|field| field.key == *key),
                    "{}: {} has no field {key}",
                    ready.name,
                    kind.name
                );
                match (key.as_str(), value) {
                    ("hotkey", Value::Text(text)) => {
                        text.parse::<Hotkey>()
                            .unwrap_or_else(|error| panic!("{}: {text}: {error}", ready.name));
                    }
                    ("operation", Value::Text(text)) => {
                        assert!(
                            matches!(text.as_str(), "up" | "down" | "mute"),
                            "{}: {text}",
                            ready.name
                        );
                    }
                    ("step", Value::Integer(step)) => {
                        assert!((1..=25).contains(step), "{}: {step}", ready.name);
                    }
                    ("folder", Value::Text(text)) => {
                        let id: FolderId = text
                            .parse()
                            .unwrap_or_else(|_| panic!("{}: folder {text}", ready.name));
                        assert!(
                            folder_ids.contains(&id),
                            "{}: the folder {text} is not in the profile",
                            ready.name
                        );
                    }
                    _ => panic!("{}: {} = {value:?}", ready.name, key),
                }
                assert!(
                    !matches!(value, Value::Text(text) if text.contains("/Users/") || text.starts_with('~')),
                    "{}: a path of one computer",
                    ready.name
                );
            }
            if kind.folder_back {
                closed = true;
            }
        }
        assert_eq!(
            closed,
            !folder_ids.is_empty(),
            "{}: a folder has Back",
            ready.name
        );
    }
    names.sort();
    names.dedup();
    assert_eq!(names.len(), READY.len(), "each profile has its own name");
}

/// Each shown key face and bar panel of a page has an icon of the set and a
/// title. A folder has only the keys that it defines.
fn check_surfaces(
    layout: &DeviceLayout,
    place: &str,
    page: &opencraft_engine::Page,
    full_page: bool,
) {
    if full_page {
        let shown: Vec<_> = layout
            .surfaces
            .iter()
            .filter(|surface| {
                matches!(
                    surface.id.kind,
                    SurfaceKind::KeyFace | SurfaceKind::BarPanel
                )
            })
            .map(|surface| surface.id)
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
            assert_eq!(
                content.icon.as_ref().and_then(|icon| icon.color),
                content.label_style.color,
                "{place}: {id:?}"
            );
        }
    }
    for (id, content) in page.surfaces() {
        if let Some(icon) = &content.icon {
            assert!(
                icon_set().get(&icon.name).is_some(),
                "{place}: {id:?}: no icon {:?}",
                icon.name
            );
        }
    }
}

#[test]
fn the_button_text_counts_the_selected_profiles() {
    assert_eq!(button_text(0), "Ready profiles…");
    assert_eq!(button_text(1), "1 ready profile…");
    assert_eq!(button_text(3), "3 ready profiles…");
}
