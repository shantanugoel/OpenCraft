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

/// The debug selectors of the rows that the last frame marks as the
/// selected row of their menu or list, in the order of the frame.
fn selected_rows(cx: &mut VisualTestContext) -> Vec<String> {
    accessibility_log::take_selected();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    accessibility_log::take_selected()
        .into_iter()
        .filter_map(|(row, selected)| selected.then_some(row))
        .collect()
}

/// The selected rows whose debug selector starts with `prefix`.
fn selected_rows_with(cx: &mut VisualTestContext, prefix: &str) -> Vec<String> {
    selected_rows(cx)
        .into_iter()
        .filter(|row| row.starts_with(prefix))
        .collect()
}

#[track_caller]
fn assert_selected_rows(cx: &mut VisualTestContext, prefix: &str, expected: &[&str]) {
    let rows = selected_rows_with(cx, prefix);
    assert_eq!(
        rows, expected,
        "The window marks the rows {rows:?} as selected, and not {expected:?}."
    );
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
#[gpui_kit::test]
fn knob_function_controls_expose_their_roles(cx: &mut TestAppContext) {
    use super::knob::{functions_window, turn};
    let (_, cx, tx, _) = functions_window(cx, &["Volume"]);
    tx.try_send(Event::Step {
        binding: turn(GestureKind::Push),
        step: Some(0),
    })
    .unwrap();
    cx.run_until_parked();
    let nodes = frame(cx);
    assert_has(&nodes, Role::Button, "Add a function");
    assert_has(
        &nodes,
        Role::Button,
        "Function 1: Volume, Sound volume, Active",
    );
    assert_has(&nodes, Role::Label, "Active");
    click(cx, "functions.0/function");
    let nodes = frame(cx);
    assert_has(&nodes, Role::Group, "Function 1");
    assert_has(&nodes, Role::TextInput, "Title");
    assert_has(&nodes, Role::Button, "More for this function");
    assert_has(&nodes, Role::RadioButton, "Right is up");
    assert_has(&nodes, Role::RadioButton, "Left is up");
    assert_has(&nodes, Role::Link, "Set left and right apart");
    click(cx, "functions.0/turn-apart");
    let nodes = frame(cx);
    assert_has(&nodes, Role::ComboBox, "Turn left");
    assert_has(&nodes, Role::ComboBox, "Turn right");
    assert_has(&nodes, Role::Link, "Join left and right");
}

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
fn the_state_picker_has_radio_buttons_for_both_faces(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = window(cx);
    let kind = KindDescription {
        kind: "test.states".into(),
        name: "Two states".into(),
        states: Some(opencraft_engine::TwoStates { on_label: None }),
        ..Default::default()
    };
    send(cx, &tx, Event::Kinds(vec![kind.clone()]));
    send(
        cx,
        &tx,
        Event::Binding(BindingState {
            binding: BindingId {
                page: 0,
                control: ControlId::new(ControlKind::Key, 2),
                gesture: GestureKind::Press,
                folder: None,
            },
            action: Some(Action {
                kind: kind.kind,
                settings: Default::default(),
            }),
            available: true,
            inert_by: None,
        }),
    );
    click(cx, "surface-key face 2");
    let nodes = frame(cx);
    assert_has(&nodes, Role::RadioButton, "Off");
    assert_has(&nodes, Role::RadioButton, "On");
}

#[gpui_kit::test]
fn an_action_cycle_announces_the_next_step(cx: &mut TestAppContext) {
    use super::nested::{cycle_action, open, press};

    let (_view, cx, tx, _) = open(cx, cycle_action());
    send(
        cx,
        &tx,
        Event::Step {
            binding: press(),
            step: Some(0),
        },
    );
    let nodes = frame(cx);
    assert_has(&nodes, Role::Label, "Next");
    assert_has(&nodes, Role::Button, "Step 1: Leaf, same, Next");
}

#[gpui_kit::test]
fn dynamic_fields_have_the_shared_roles_and_labels(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = window(cx);
    let kind = super::actions::dynamic_kind();
    let action = Action {
        kind: kind.kind.clone(),
        settings: kind
            .fields
            .iter()
            .map(|f| (f.key.clone(), f.default.clone().unwrap()))
            .collect(),
    };
    tx.try_send(Event::Kinds(vec![kind])).unwrap();
    tx.try_send(Event::Binding(BindingState {
        binding: BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Key, 2),
            gesture: GestureKind::Press,
            folder: None,
        },
        action: Some(action),
        available: true,
        inert_by: None,
    }))
    .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    let nodes = frame(cx);
    assert_has(&nodes, Role::ComboBox, "Device");
    assert_has(
        &nodes,
        Role::RadioButton,
        &format!("Colour #{:06X}", crate::view::theme::FILL_COLOURS[0]),
    );
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
    assert_has(&nodes, Role::Label, "Selected");
    assert_has(&nodes, Role::TreeItem, "Work");
    for page in ["Main", "Meetings", "Media"] {
        assert_has(&nodes, Role::TreeItem, page);
    }
    assert_has(&nodes, Role::Button, "Add a page");
    assert_has(&nodes, Role::Button, "Add a folder");
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
    let (_view, cx, tx, _sent) = named_window(cx);
    click(cx, "surface-key face 2");
    let nodes = frame(cx);
    for name in ["Press", "Release", "Long press", "Double press"] {
        assert_has(&nodes, Role::Button, &format!("{name}: Add an action"));
    }
    let kind = super::actions::dynamic_kind();
    let action = Action {
        kind: kind.kind.clone(),
        settings: Default::default(),
    };
    send(cx, &tx, Event::Kinds(vec![kind]));
    for gesture in [GestureKind::LongPress, GestureKind::DoublePress] {
        send(
            cx,
            &tx,
            Event::Binding(BindingState {
                binding: BindingId {
                    page: 0,
                    control: ControlId::new(ControlKind::Key, 2),
                    gesture,
                    folder: None,
                },
                action: Some(action.clone()),
                available: true,
                inert_by: None,
            }),
        );
    }
    let bound = frame(cx);
    assert_has(&bound, Role::Group, "Long press");
    assert_has(&bound, Role::Group, "Double press");
    assert_has(&nodes, Role::Group, "Key 2");
    assert_has(&nodes, Role::Button, "More for Key 2");
    assert_has(&nodes, Role::Group, "Appearance");
    assert_has(&nodes, Role::TextInput, "Title");
    assert_has(&nodes, Role::RadioButton, "Top");
    assert_has(&nodes, Role::ComboBox, "Text size");
    assert_has(&nodes, Role::ComboBox, "Font");
    assert_has(&nodes, Role::Button, "Bold");
    assert_has(&nodes, Role::Button, "Italic");
    assert_has(&nodes, Role::Button, "Underline");
    assert_has(&nodes, Role::RadioButton, "No fill");
    assert_has(&nodes, Role::RadioButton, "Other colour");
    assert_has(&nodes, Role::Button, "Choose icon…");
}

#[gpui_kit::test]
fn the_font_picker_of_a_label_has_labels(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = named_window(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.inspector
                .font_picker
                .use_fixed_families(fixed_font_families())
        })
    });
    click(cx, "surface-key face 2");
    click(cx, "label-font");
    let nodes = frame(cx);
    assert_has(&nodes, Role::Dialog, "Choose a font");
    assert_has(&nodes, Role::SearchInput, "Search fonts");
    assert_has(&nodes, Role::MenuItem, "System font");
    assert_has(&nodes, Role::MenuItem, "Menlo");
}

/// The families of a window test: the picker does not need the fonts of the
/// computer.
fn fixed_font_families() -> Vec<opencraft_engine::label::FontFamily> {
    [("Menlo", "Menlo"), ("Avenir Next", "Avenir Next")]
        .into_iter()
        .map(|(name, display)| opencraft_engine::label::FontFamily {
            name: name.into(),
            display: display.into(),
        })
        .collect()
}

#[gpui_kit::test]
fn a_family_that_is_not_installed_has_a_readable_hint(cx: &mut TestAppContext) {
    use opencraft_engine::{Auto, LabelStyle};
    let (view, cx, tx, _sent) = named_window(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.inspector
                .font_picker
                .use_fixed_families(fixed_font_families())
        })
    });
    click(cx, "surface-key face 2");
    send(
        cx,
        &tx,
        Event::Surface(SurfaceState {
            id: key(),
            content: SurfaceContent {
                label: Some("Mail".into()),
                label_style: LabelStyle {
                    font: Some(Auto::Value("Futura X".into())),
                    ..Default::default()
                },
                ..Default::default()
            },
            image: std::sync::Arc::new(RgbImage::new(90, 90)),
            image_size: None,
            image_area: None,
            on_image_size: None,
            on_image_area: None,
            states: None,
        }),
    );
    let nodes = frame(cx);
    assert_has(&nodes, Role::Label, crate::view::font_picker::MISSING_FONT);
}

#[gpui_kit::test]
fn the_size_menu_lists_the_sizes_of_a_large_emoji(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = named_window(cx);
    click(cx, "surface-key face 2");
    click(cx, "label-size");
    let nodes = frame(cx);
    assert_has(&nodes, Role::MenuItem, "10 pt");
    assert_has(&nodes, Role::MenuItem, "48 pt");
    assert_has(&nodes, Role::MenuItem, "80 pt");
}

/// The selected segment of a segmented control is the selected radio button
/// of the group, and the arrow keys change it (issues 481 and 483).
#[gpui_kit::test]
fn the_selected_segment_of_a_control_is_the_selected_radio_button(cx: &mut TestAppContext) {
    use opencraft_engine::{Auto, LabelStyle};
    let (_view, cx, tx, _sent) = named_window(cx);
    click(cx, "surface-key face 2");
    // The key has an image, thus "Middle" is the selected position.
    let selected = selected_rows(cx);
    assert!(
        selected.contains(&"label-position-middle".to_string()),
        "{selected:?}"
    );
    send(
        cx,
        &tx,
        Event::Surface(SurfaceState {
            id: key(),
            content: SurfaceContent {
                label: Some("Mail".into()),
                label_style: LabelStyle {
                    position: Some(Auto::Value(opencraft_engine::LabelPosition::Hidden)),
                    ..Default::default()
                },
                ..Default::default()
            },
            image: std::sync::Arc::new(RgbImage::new(90, 90)),
            image_size: None,
            image_area: None,
            on_image_size: None,
            on_image_area: None,
            states: None,
        }),
    );
    let selected = selected_rows(cx);
    assert!(
        selected.contains(&"label-position-none".to_string())
            && !selected.contains(&"label-position-middle".to_string()),
        "{selected:?}"
    );
    let nodes = frame(cx);
    assert_has(&nodes, Role::RadioButton, "Top");
    assert_has(&nodes, Role::RadioButton, "None");
}

/// The row that the arrow keys highlight is the selected row of the menu
/// for the accessibility API (issue 481).
#[gpui_kit::test]
fn the_highlighted_row_of_a_drawn_menu_is_selected(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = named_window(cx);
    click(cx, "surface-key face 2");
    click(cx, "label-size");
    // The first row is highlighted when the menu opens, and only that row.
    assert_selected_rows(cx, "label-size-", &["label-size-10"]);
    cx.simulate_keystrokes("down down");
    cx.run_until_parked();
    assert_selected_rows(cx, "label-size-", &["label-size-14"]);
    // The pointer moves the highlight too.
    let row = cx.debug_bounds("label-size-64").expect("the row of 64 pt");
    cx.simulate_mouse_move(row.center(), None, Modifiers::default());
    cx.run_until_parked();
    assert_selected_rows(cx, "label-size-", &["label-size-64"]);
}

/// The row of the current family is the selected row of the list, and the
/// arrow keys move the selected row (issue 481).
#[gpui_kit::test]
fn the_highlighted_row_of_the_font_picker_is_selected(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = named_window(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.inspector
                .font_picker
                .use_fixed_families(fixed_font_families())
        })
    });
    click(cx, "surface-key face 2");
    click(cx, "label-font");
    // The surface has no own family: "System font" is the current row.
    assert_selected_rows(cx, "font-row-", &["font-row-0"]);
    cx.simulate_keystrokes("down");
    cx.run_until_parked();
    assert_selected_rows(cx, "font-row-", &["font-row-1"]);
}

#[gpui_kit::test]
fn the_title_style_row_names_the_default_and_the_reset(cx: &mut TestAppContext) {
    use opencraft_engine::LabelStyle;
    let (_view, cx, tx, _sent) = named_window(cx);
    click(cx, "surface-key face 2");
    let nodes = frame(cx);
    assert_has(
        &nodes,
        Role::Label,
        "This title follows the profile default.",
    );
    send(
        cx,
        &tx,
        Event::Surface(SurfaceState {
            id: key(),
            content: SurfaceContent {
                label: Some("Mail".into()),
                label_style: LabelStyle {
                    italic: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
            image: std::sync::Arc::new(RgbImage::new(90, 90)),
            image_size: None,
            image_area: None,
            on_image_size: None,
            on_image_area: None,
            states: None,
        }),
    );
    let nodes = frame(cx);
    assert_has(&nodes, Role::Label, "This title has its own style.");
    assert_has(&nodes, Role::Button, "Reset to the profile default");
}

#[gpui_kit::test]
fn the_library_and_the_device_settings_have_labels(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = named_window(cx);
    send(
        cx,
        &tx,
        Event::DeviceSettings(opencraft_engine::DeviceSettings {
            brightness: 80,
            screen_off_minutes: 0,
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
    assert_has(&nodes, Role::Label, "Firmware: Not available");
    assert_has(&nodes, Role::Switch, "Haptic feedback");
    assert_has(&nodes, Role::ComboBox, "Screen off after");
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
    assert_has(&nodes, Role::TextInput, "Default title font");
    assert_has(&nodes, Role::ComboBox, "Default title size");
    assert_has(&nodes, Role::RadioButton, "Automatic");
    assert_has(&nodes, Role::RadioButton, "Middle");
    assert_has(&nodes, Role::RadioButton, "Bottom");
    assert_has(&nodes, Role::RadioButton, "None");
    assert_has(
        &nodes,
        Role::RadioButton,
        "Automatic: black or white for the fill",
    );
    assert_has(&nodes, Role::RadioButton, "Title #FFFFFF");
    assert_has(&nodes, Role::Button, "Bold");
    assert_has(&nodes, Role::Button, "Italic");
    assert_has(&nodes, Role::Button, "Underline");
    assert_has(&nodes, Role::Button, "Reset");
    assert_has(&nodes, Role::Button, "Add an application…");
    assert_has(&nodes, Role::Switch, "Default profile");
    assert_has(&nodes, Role::Button, "Done");
}

#[gpui_kit::test]
fn the_application_picker_has_a_dialog_and_rows(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = window(cx);
    cx.simulate_resize(size(px(1120.), px(2400.)));
    let kind = KindDescription {
        kind: "test.application".into(),
        name: "Open an application".into(),
        fields: vec![Field {
            key: "application".into(),
            label: "Application, file or folder".into(),
            input: InputType::Path {
                kind: PathKind::Application {
                    bundle_id: String::new(),
                },
            },
            default: Some(Value::Text(String::new())),
        }],
        ..Default::default()
    };
    let action = Action {
        kind: kind.kind.clone(),
        settings: kind
            .fields
            .iter()
            .map(|field| (field.key.clone(), field.default.clone().unwrap()))
            .collect(),
    };
    tx.try_send(Event::Kinds(vec![kind])).unwrap();
    tx.try_send(Event::Binding(BindingState {
        binding: BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Key, 2),
            gesture: GestureKind::Press,
            folder: None,
        },
        action: Some(action),
        available: true,
        inert_by: None,
    }))
    .unwrap();
    cx.run_until_parked();
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.use_fixed_applications(
                vec![crate::applications::InstalledApplication {
                    name: "Calendar".into(),
                    bundle_identifier: "com.example.calendar".into(),
                    path: std::path::PathBuf::from("/Applications/OpenCraftTestCalendar.app"),
                }],
                cx,
            );
        });
    });
    click(cx, "surface-key face 2");
    click(cx, "path-0");
    let nodes = frame(cx);
    assert_has(&nodes, Role::Dialog, "Choose an application");
    assert_has(&nodes, Role::SearchInput, "Search applications");
    assert_has(&nodes, Role::MenuItem, "Calendar");
    assert_has(&nodes, Role::MenuItem, "Choose a different application…");
    // The highlighted row is the selected row of the list, and the arrow keys
    // move it, also onto the footer row (issue 481).
    assert_selected_rows(cx, "app-row-", &["app-row-0"]);
    cx.simulate_keystrokes("down");
    cx.run_until_parked();
    assert_selected_rows(cx, "choose-different", &["choose-different-application"]);
}

#[gpui_kit::test]
fn the_import_command_in_the_palette_has_a_role_and_label(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = named_window(cx);
    cx.dispatch_action(crate::app_menu::Search);
    cx.run_until_parked();
    cx.simulate_input("import a profile");
    cx.run_until_parked();
    assert_has(&frame(cx), Role::Button, "Import a profile…");
}

#[gpui_kit::test]
fn a_profile_shows_its_application_and_the_default_mark(cx: &mut TestAppContext) {
    use opencraft_engine::{ActiveCause, ApplicationLink, ProfileEntry};
    let (_view, cx, tx, _sent) = window(cx);
    let calls = ProfileEntry {
        id: "calls".into(),
        name: "Calls".into(),
        pages: vec!["Main".into()],
        applications: vec![ApplicationLink {
            bundle_id: "us.zoom.xos".into(),
            name: "zoom.us".into(),
        }],
        ..Default::default()
    };
    let work = ProfileEntry {
        id: "work".into(),
        name: "Work".into(),
        pages: vec!["Mail".into()],
        applications: vec![
            ApplicationLink {
                bundle_id: "us.zoom.xos".into(),
                name: "zoom.us".into(),
            },
            ApplicationLink {
                bundle_id: "com.apple.Safari".into(),
                name: "Safari".into(),
            },
        ],
        ..Default::default()
    };
    // The list gives the links that apply and the owner of each link.
    let profiles = opencraft_engine::profile_list::ProfileList::new(vec![calls, work], None)
        .entries()
        .to_vec();
    tx.try_send(Event::Profiles {
        profiles,
        active: "calls".into(),
        default: "work".into(),
        cause: ActiveCause::Linked {
            bundle_id: "us.zoom.xos".into(),
        },
    })
    .unwrap();
    cx.run_until_parked();
    let nodes = frame(cx);
    assert_has(&nodes, Role::Label, "Because “zoom.us” is in front");
    // The first profile keeps zoom.us. Work shows the link that applies,
    // and the default mark, because Work is not active.
    assert_has(&nodes, Role::Label, "Safari DEFAULT");
    let position = cx
        .debug_bounds("profile-work")
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
    assert_has(
        &frame(cx),
        Role::Label,
        "Linked to “Calls”. That link applies.",
    );
}

#[gpui_kit::test]
fn a_control_with_no_surface_has_the_name_as_its_label(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = window(cx);
    let mut layout =
        cx.update(|_, cx| (*view.read(cx).state.layout.clone().expect("layout")).clone());
    layout.controls.push(opencraft_engine::ControlLayout {
        id: ControlId::new(ControlKind::Key, 7),
        row: 3,
        column: 1,
        surface: None,
        feedback_surface: None,
    });
    tx.try_send(Event::Layout(std::sync::Arc::new(layout)))
        .unwrap();
    cx.run_until_parked();
    assert_has(&frame(cx), Role::Button, "Key 7");
    click(cx, "control-key 7");
    assert_has(&frame(cx), Role::Group, "Key 7");
    assert_has(&frame(cx), Role::Button, "More for Key 7");
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

#[gpui_kit::test]
fn the_crop_of_an_image_has_its_labels(cx: &mut TestAppContext) {
    use super::crop::image_surface;
    let (_view, cx, tx, _sent) = named_window(cx);
    click(cx, "surface-key face 2");
    send(cx, &tx, image_surface(key(), None, (64, 64), (112, 112)));
    let nodes = frame(cx);
    assert_has(&nodes, Role::ComboBox, "Zoom");
    for label in [
        "Move the image left",
        "Move the image right",
        "Move the image up",
        "Move the image down",
        "Reset",
    ] {
        assert_has(&nodes, Role::Button, label);
    }
    assert_has(
        &nodes,
        Role::Label,
        "This image has 64 × 64 pixels. At 100 % it is smaller than the key face, thus it will look soft.",
    );
    // The menu of the zoom has its rows with their names.
    click(cx, "image-zoom");
    let nodes = frame(cx);
    assert_has(&nodes, Role::MenuItem, "200 %");
    assert_has(&nodes, Role::MenuItem, "400 %");
}
