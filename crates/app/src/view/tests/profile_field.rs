//! The profile list of the action "Select a profile" (decision 0035,
//! section 10). The list shows the profiles of the model. A click stores
//! the identity. A saved name that is not an identity stays.

use super::*;
use crate::view::controls::accessibility_log;
use gpui_kit::base::test_support::texts;
use gpui_kit::Role;
use opencraft_engine::{Color, ProfileEntry};

fn kind() -> KindDescription {
    KindDescription {
        kind: "test.profile".into(),
        name: "Select a profile".into(),
        fields: vec![Field {
            key: "profile".into(),
            label: "Profile".into(),
            input: InputType::Profile,
            default: None,
        }],
        ..Default::default()
    }
}

fn binding() -> BindingId {
    BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Key, 2),
        gesture: GestureKind::Press,
    }
}

fn action(profile: &str) -> Action {
    Action {
        kind: "test.profile".into(),
        settings: Settings::from([("profile".into(), Value::Text(profile.into()))]),
    }
}

fn event(profile: &str) -> Event {
    Event::Binding(BindingState {
        binding: binding(),
        action: Some(action(profile)),
        available: true,
    })
}

fn profiles() -> Event {
    Event::Profiles {
        profiles: vec![
            ProfileEntry {
                id: "calls".into(),
                name: "Calls".into(),
                color: Some(Color::rgb(20, 90, 200)),
                pages: vec!["Main".into()],
                ..Default::default()
            },
            ProfileEntry {
                id: "desk".into(),
                name: "Desk".into(),
                pages: vec!["Main".into()],
                ..Default::default()
            },
        ],
        active: "desk".into(),
    }
}

fn shown(cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|window, _| {
        texts(window)
            .iter()
            .map(|text| text.text().to_string())
            .collect()
    })
}

fn show(cx: &mut VisualTestContext, text: &str) -> bool {
    shown(cx).iter().any(|shown| shown == text)
}

#[gpui_kit::test]
fn the_profile_list_stores_the_identity_and_keeps_an_unknown_value(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(1600.)));
    tx.try_send(Event::Kinds(vec![kind()])).unwrap();
    tx.try_send(profiles()).unwrap();
    tx.try_send(event("Work")).unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("profile-0").is_some(), "the profile field");
    assert!(show(cx, "Work (not found)"), "{:?}", shown(cx));
    assert!(show(
        cx,
        "Select the profile again, so that a rename does not break this key."
    ));

    accessibility_log::take();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let nodes = accessibility_log::take();
    assert!(
        nodes
            .iter()
            .any(|(role, label)| *role == Role::ComboBox && label == "Profile"),
        "{nodes:?}"
    );

    click(cx, "profile-0");
    assert!(cx.debug_bounds("profile-0-0").is_some());
    assert!(cx.debug_bounds("profile-0-1").is_some());
    let nodes = {
        accessibility_log::take();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        accessibility_log::take()
    };
    assert!(nodes
        .iter()
        .any(|(role, label)| *role == Role::MenuItem && label == "Calls"));
    assert!(nodes
        .iter()
        .any(|(role, label)| *role == Role::MenuItem && label == "Desk"));

    click(cx, "profile-0-0");
    assert!(
        cx.debug_bounds("profile-0-menu").is_none(),
        "the menu closes"
    );
    assert!(!show(cx, "Work (not found)"), "{:?}", shown(cx));
    assert!(!show(
        cx,
        "Select the profile again, so that a rename does not break this key."
    ));
    let stored = sent
        .borrow()
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::SetBinding {
                action: Some(action),
                ..
            } => action.settings.get("profile").cloned(),
            _ => None,
        });
    assert_eq!(stored, Some(Value::Text("calls".into())));
}
