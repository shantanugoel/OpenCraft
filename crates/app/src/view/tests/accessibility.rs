//! The accessible roles and labels of the main window (issue 243,
//! `docs/DESIGN.md` section 3.4).
//!
//! The test platform of GPUI makes no accessibility tree. Thus the test
//! reads the roles and the labels that the window gives to its elements
//! (`controls::accessibility_log`). The facts about the real tree are in
//! `docs/kb/gpui.md`, "Accessibility tree of the window".

use super::pages::{pages, profiles, send};
use super::*;
use crate::view::controls::accessibility_log;
use crate::view::sidebar::ProfileSettingsRow;
use gpui_kit::{MouseButton, MouseDownEvent, Role};

/// The roles and the labels of one new frame of the window.
fn frame(cx: &mut VisualTestContext) -> Vec<(Role, String)> {
    accessibility_log::take();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    accessibility_log::take()
}

fn has(nodes: &[(Role, String)], role: Role, label: &str) -> bool {
    nodes.iter().any(|(r, l)| *r == role && l == label)
}

#[track_caller]
fn assert_has(nodes: &[(Role, String)], role: Role, label: &str) {
    assert!(
        has(nodes, role, label),
        "No {role:?} “{label}” in the window. The window has: {nodes:#?}"
    );
}

/// A window with two profiles and three pages.
fn named_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    send(
        cx,
        &tx,
        profiles(&[("default", "Default"), ("work", "Work")], "default"),
    );
    send(cx, &tx, pages(&["Main", "Meetings", "Media"], 0));
    tx.try_send(Event::Connected).unwrap();
    cx.run_until_parked();
    (view, cx, tx, sent)
}

#[gpui_kit::test]
fn the_main_parts_have_a_role_and_the_label_of_the_record(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = named_window(cx);
    let nodes = frame(cx);
    // The sidebar (decision 0029, section 7).
    assert_has(&nodes, Role::Navigation, "Devices, profiles and pages");
    assert_has(&nodes, Role::Button, "Hide the sidebar");
    assert_has(
        &nodes,
        Role::Button,
        "Device Test device, connected. Select to show the device settings.",
    );
    assert_has(&nodes, Role::Button, "Add a profile");
    assert_has(&nodes, Role::Tree, "Profiles");
    assert_has(&nodes, Role::TreeItem, "Default, on device");
    assert_has(&nodes, Role::TreeItem, "Work");
    for page in ["Main", "Meetings", "Media"] {
        assert_has(&nodes, Role::TreeItem, page);
    }
    assert_has(&nodes, Role::Button, "Add a page");
    assert_has(&nodes, Role::Button, "Settings");
    // The toolbar (section 5).
    assert_has(&nodes, Role::Label, "Main, Default · 3 pages");
    assert_has(&nodes, Role::Label, "Live on the device");
    assert_has(
        &nodes,
        Role::Button,
        "Search actions, controls, pages and commands",
    );
    assert_has(&nodes, Role::Button, "Hide the inspector");
    // The canvas (section 8).
    assert_has(&nodes, Role::Label, "Page 1 of 3");
    assert_has(&nodes, Role::Button, "Next page");
    assert_has(&nodes, Role::Button, "Page settings");
    assert_has(&nodes, Role::Group, "Device Test device");
    assert_has(&nodes, Role::Button, "Key 2, “Mail”");
    assert_has(&nodes, Role::Button, "Bar panel 2, “Sound”");
    assert_has(&nodes, Role::Button, "Knob 2");
    assert_has(&nodes, Role::Button, "Touch bar: swipe gestures");
    // The right panel (section 9) and the status bar (section 6).
    assert_has(&nodes, Role::RadioButton, "Inspector");
    assert_has(&nodes, Role::RadioButton, "Library");
    assert_has(&nodes, Role::Label, "All changes saved");
}

#[gpui_kit::test]
fn the_inspector_of_a_key_has_labels_from_its_rows(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = named_window(cx);
    click(cx, "surface-key face 2");
    let nodes = frame(cx);
    assert_has(&nodes, Role::Group, "Key 2");
    assert_has(&nodes, Role::Button, "More for Key 2");
    assert_has(&nodes, Role::Group, "Appearance");
    assert_has(&nodes, Role::TextInput, "Title");
    assert_has(&nodes, Role::RadioButton, "Top");
    assert_has(&nodes, Role::ComboBox, "Text size");
    assert_has(&nodes, Role::Button, "Bold");
    assert_has(&nodes, Role::RadioButton, "No fill");
    assert_has(&nodes, Role::RadioButton, "Other colour");
    assert_has(&nodes, Role::Button, "Choose icon…");
}

#[gpui_kit::test]
fn the_library_and_the_device_settings_have_labels(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = named_window(cx);
    send(
        cx,
        &tx,
        Event::DeviceSettings(opencraft_engine::DeviceSettings {
            brightness: 80,
            haptic: true,
            knob_lights: opencraft_engine::KnobLights {
                mode: opencraft_engine::KnobLightMode::Colors,
                brightness: 100,
                colors: vec![Color::WHITE; 2],
            },
        }),
    );
    click(cx, "tab-library");
    let nodes = frame(cx);
    assert_has(&nodes, Role::SearchInput, "Search actions");
    click(cx, "device-card");
    click(cx, "tab-inspector");
    let nodes = frame(cx);
    assert_has(&nodes, Role::Group, "Test device");
    assert_has(&nodes, Role::Switch, "Haptic feedback");
    assert_has(&nodes, Role::RadioButton, "Colours");
    assert_has(&nodes, Role::Button, "Knob 1 light colour #FFFFFF");
}

#[gpui_kit::test]
fn the_profile_sheet_is_a_dialog_with_labels(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = named_window(cx);
    let position = cx
        .debug_bounds("profile-default")
        .expect("the profile row")
        .center();
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Right,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.run_until_parked();
    cx.dispatch_action(ProfileSettingsRow);
    cx.run_until_parked();
    let nodes = frame(cx);
    assert_has(&nodes, Role::Dialog, "Profile settings for Default");
    assert_has(&nodes, Role::Label, "Profile settings");
    assert_has(&nodes, Role::TextInput, "Name");
    assert_has(&nodes, Role::ComboBox, "First page");
    assert_has(&nodes, Role::Button, "Done");
}

#[gpui_kit::test]
fn each_control_with_a_role_has_a_label(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = named_window(cx);
    click(cx, "surface-key face 2");
    for (role, label) in frame(cx) {
        assert!(
            !label.trim().is_empty(),
            "A {role:?} has no accessible label"
        );
    }
}
