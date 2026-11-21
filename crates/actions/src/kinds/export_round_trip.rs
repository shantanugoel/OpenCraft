//! A path in the home folder survives an export and an import (decision
//! 0037). The export writes `~`. Each kind that has a Path field must then
//! give the full path to the system.

use super::{Work, REGISTERED};
use crate::tests::support::{Call, Fake};
use opencraft_engine::share::{export, import};
use opencraft_engine::{
    Action, ControlId, ControlKind, GestureKind, InputType, Page, Profile, ProfileStore, Settings,
    Value,
};

fn invoke(action: &Action, system: &Fake) -> Result<(), String> {
    let sender = None;
    let kind = super::find(&action.kind).expect("a registered kind");
    kind.invoke(&action.settings, &Work::new(system, &sender))
}

/// Export the one action in a profile, import the file, and return the
/// action of the import.
fn round_trip(action: &Action, kinds: &[opencraft_engine::KindDescription]) -> Action {
    let dir = tempfile::TempDir::new().unwrap();
    let store = ProfileStore::new(dir.path().join("work"));
    let mut profile = Profile::new("Work", vec![Page::new("Main")]);
    let control = ControlId::new(ControlKind::Key, 1);
    profile
        .page_mut(0)
        .unwrap()
        .set_binding(control, GestureKind::Press, Some(action.clone()));
    let file = dir.path().join("Work.opencraft");
    export(&profile, &store, &file, kinds).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains('~'), "{text}");
    let package = import(&file).unwrap();
    package
        .profile
        .page(0)
        .unwrap()
        .binding(control, GestureKind::Press)
        .expect("the imported action")
        .clone()
}

fn home() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let home = home.trim_end_matches('/').to_string();
    (!home.is_empty()).then_some(home)
}

#[test]
fn each_kind_with_a_path_field_gives_the_full_path_after_an_export_and_an_import() {
    let Some(home) = home() else {
        return;
    };
    let kinds: Vec<_> = REGISTERED
        .iter()
        .map(|entry| entry.kind.description())
        .collect();
    let mut checked = 0;
    for description in &kinds {
        let paths: Vec<_> = description
            .fields
            .iter()
            .filter(|field| matches!(field.input, InputType::Path { .. }))
            .collect();
        if paths.is_empty() {
            continue;
        }
        let mut settings = Settings::new();
        if description.fields.iter().any(|field| field.key == "url") {
            settings.insert("url".into(), Value::Text("https://example.com".into()));
        }
        for field in &paths {
            settings.insert(
                field.key.clone(),
                Value::Text(format!("{home}/Test/{}.app", field.key)),
            );
        }
        let action = Action {
            kind: description.kind.clone(),
            settings,
        };
        let before = Fake::default();
        invoke(&action, &before).unwrap();
        let imported = round_trip(&action, &kinds);
        assert!(
            imported
                .settings
                .values()
                .any(|value| matches!(value, Value::Text(text) if text.starts_with("~/"))),
            "{}: the import keeps the `~`",
            description.kind
        );
        let after = Fake::default();
        invoke(&imported, &after).unwrap();
        assert_eq!(after.calls(), before.calls(), "{}", description.kind);
        assert!(
            format!("{:?}", after.calls()).contains(&format!("{home}/Test/")),
            "{}: {:?}",
            description.kind,
            after.calls()
        );
        assert!(
            !format!("{:?}", after.calls()).contains('~'),
            "{}: {:?}",
            description.kind,
            after.calls()
        );
        checked += 1;
    }
    // Open an application, quit an application, open a URL, and play a sound.
    assert_eq!(checked, 4);
}

#[test]
fn an_action_cycle_keeps_two_equal_steps_through_export_and_import() {
    use super::{cycle, Kind};
    let step = Action {
        kind: super::open_application::ID.into(),
        settings: Settings::from([(
            "application".into(),
            Value::Text("~/Documents/Test/App.app".into()),
        )]),
    };
    let steps = [step.clone(), step];
    let action = Action {
        kind: cycle::ID.into(),
        settings: Settings::from([("steps".into(), Action::list_to_value(&steps))]),
    };
    let imported = round_trip(
        &action,
        &[
            cycle::KIND.description(),
            super::open_application::description(),
        ],
    );
    assert_eq!(imported.kind, action.kind);
    assert_eq!(cycle::read(&imported.settings).unwrap(), steps);
}

#[test]
fn a_hold_hotkey_keeps_its_kind_and_chord_through_export_and_import() {
    let dir = tempfile::tempdir().unwrap();
    let store = ProfileStore::new(dir.path().join("talk"));
    let mut page = Page::new("Main");
    let key = ControlId::new(ControlKind::Key, 1);
    let action = Action {
        kind: super::hold_hotkey::ID.into(),
        settings: Settings::from([("hotkey".into(), Value::Text("cmd+shift+space".into()))]),
    };
    page.set_binding(key, GestureKind::Press, Some(action.clone()));
    let profile = Profile::new("Talk", vec![page]);
    let file = dir.path().join("Talk.opencraft");
    export(
        &profile,
        &store,
        &file,
        &[super::hold_hotkey::description()],
    )
    .unwrap();
    let package = import(&file).unwrap();
    assert_eq!(
        package
            .profile
            .page(0)
            .unwrap()
            .binding(key, GestureKind::Press),
        Some(&action)
    );
}

#[test]
fn the_home_folder_alone_is_a_folder_to_open_and_not_an_application_name() {
    let Some(home) = home() else {
        return;
    };
    let action = Action {
        kind: super::open_application::ID.into(),
        settings: [("application".into(), Value::Text(home.clone()))].into(),
    };
    let kinds = vec![super::open_application::description()];
    let imported = round_trip(&action, &kinds);
    assert_eq!(
        imported.settings.get("application"),
        Some(&Value::Text("~".into()))
    );
    let system = Fake::default();
    invoke(&imported, &system).unwrap();
    assert_eq!(system.calls(), [Call::File(home)]);
}
