//! The profile settings sheet (decision 0029, section 16): it opens from the
//! right-click menu of a profile, it is modal, each control sends its
//! command, and the export and the import go to the engine.

use super::pages::{pages, send};
use super::*;
use crate::view::sidebar::ProfileSettingsRow;
use gpui_kit::{MouseButton, MouseDownEvent, Pixels, Role};
use opencraft_engine::ProfileEntry;
use std::path::PathBuf;

fn entry(id: &str, name: &str, pages: &[&str], first_page: usize) -> ProfileEntry {
    ProfileEntry {
        id: id.into(),
        name: name.into(),
        color: None,
        pages: pages.iter().map(|p| p.to_string()).collect(),
        first_page,
        ..Default::default()
    }
}

fn profiles(list: Vec<ProfileEntry>, active: &str) -> Event {
    Event::Profiles {
        profiles: list,
        active: active.into(),
        default: active.into(),
        cause: opencraft_engine::ActiveCause::Selected,
    }
}

/// A window with the active profile `default` (three pages) and the profile
/// `work` (two pages, the second one first).
fn sheet_window(
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
        profiles(
            vec![
                entry("default", "Default", &["Main", "Meetings", "Media"], 0),
                entry("work", "Work", &["Mail", "Calls"], 1),
            ],
            "default",
        ),
    );
    send(cx, &tx, pages(&["Main", "Meetings", "Media"], 0));
    (view, cx, tx, sent)
}

fn centre(cx: &mut VisualTestContext, id: &'static str) -> gpui_kit::Point<Pixels> {
    cx.debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"))
        .center()
}

/// The right click on a profile row, and "Profile Settings…".
fn open_sheet(cx: &mut VisualTestContext, row: &'static str) {
    let position = centre(cx, row);
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
}

fn is_open(cx: &mut VisualTestContext) -> bool {
    cx.debug_bounds("profile-sheet").is_some()
}

fn last(sent: &Commands) -> Option<Command> {
    sent.borrow().last().cloned()
}

#[gpui_kit::test]
fn the_menu_opens_the_sheet_of_the_profile_and_the_sheet_is_modal(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = sheet_window(cx);
    let items = cx.update(|_, cx| {
        view.read(cx)
            .menu_items(&crate::view::pages::Row::Profile("work".into()))
    });
    assert!(items.contains(&(crate::view::pages::MenuItem::ProfileSettings, true)));
    assert!(!is_open(cx));
    open_sheet(cx, "profile-work");
    assert!(is_open(cx));
    cx.update(|window, cx| {
        let view = view.read(cx);
        assert_eq!(view.sheet.profile.as_deref(), Some("work"));
        assert_eq!(view.sheet.name.read(cx).value().as_ref(), "Work");
        assert!(
            view.sheet.focus.is_focused(window),
            "the sheet has the focus"
        );
    });
    // The sheet is in the centre, below the toolbar, and 600 wide.
    let card = cx.debug_bounds("profile-sheet").unwrap();
    let layer = cx.debug_bounds("profile-sheet-layer").unwrap();
    assert_eq!(card.size.width, theme::PROFILE_SHEET_WIDTH);
    assert_eq!(card.origin.y, theme::SHEET_TOP);
    assert!((card.center().x - layer.center().x).abs() < px(1.));
    let viewport = cx.update(|window, _| window.viewport_size());
    assert_eq!(layer.size, viewport, "the layer covers the window");
    // The pages of the profile that is not active come from its entry.
    let first = cx.debug_bounds("first-page").unwrap();
    assert_eq!(first.size.width, theme::WIDTH_SHEET_POPUP);

    // The key commands of the window do not operate, and a click on the
    // layer does not get to the sidebar and does not close the sheet.
    cx.simulate_keystrokes("ctrl-cmd-s");
    cx.run_until_parked();
    cx.update(|_, cx| assert!(view.read(cx).panes.sidebar_visible));
    let row = centre(cx, "profile-default");
    cx.simulate_click(row, Modifiers::default());
    cx.run_until_parked();
    assert!(is_open(cx));
    assert!(sent.borrow().is_empty(), "{:?}", sent.borrow());

    // Escape closes the sheet, and the focus goes back to the window.
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!is_open(cx));
    cx.update(|window, cx| assert!(view.read(cx).focus.is_focused(window)));
    cx.simulate_keystrokes("ctrl-cmd-s");
    cx.run_until_parked();
    cx.update(|_, cx| assert!(!view.read(cx).panes.sidebar_visible));
    assert!(sent.borrow().is_empty(), "no change: {:?}", sent.borrow());
}

#[gpui_kit::test]
fn the_colour_and_the_first_page_go_to_the_engine(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = sheet_window(cx);
    open_sheet(cx, "profile-work");
    click(cx, "profile-colour-1");
    assert_eq!(
        last(&sent),
        Some(Command::SetProfileColor {
            profile: "work".into(),
            color: Some(colour(theme::PROFILE_COLOURS[1])),
        })
    );
    // The engine replies, and the circle and the sidebar icon follow.
    let mut work = entry("work", "Work", &["Mail", "Calls"], 1);
    work.color = Some(colour(theme::PROFILE_COLOURS[1]));
    send(
        cx,
        &tx,
        profiles(
            vec![
                entry("default", "Default", &["Main", "Meetings", "Media"], 0),
                work,
            ],
            "default",
        ),
    );
    click(cx, "profile-colour-1");
    assert_eq!(sent.borrow().len(), 1, "the same colour sends nothing");
    click(cx, "profile-colour-none");
    assert_eq!(
        last(&sent),
        Some(Command::SetProfileColor {
            profile: "work".into(),
            color: None,
        })
    );

    // The menu of "First page" has the pages of the profile.
    click(cx, "first-page");
    assert!(cx.debug_bounds("first-page-menu").is_some());
    assert!(cx.debug_bounds("first-page-1").is_some());
    assert!(cx.debug_bounds("first-page-2").is_none());
    // Escape closes the menu first, and then the sheet.
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("first-page-menu").is_none());
    assert!(is_open(cx));
    click(cx, "first-page");
    click(cx, "first-page-0");
    assert_eq!(
        last(&sent),
        Some(Command::SetFirstPage {
            profile: "work".into(),
            page: 0,
        })
    );
    assert!(cx.debug_bounds("first-page-menu").is_none());

    // For the active profile, the pages come from the engine pages event.
    click(cx, "profile-done");
    assert!(!is_open(cx));
    open_sheet(cx, "profile-default");
    let first = cx.update(|_, cx| view.read(cx).sheet_pages("default"));
    assert_eq!(first.0, ["Main", "Meetings", "Media"]);
    click(cx, "first-page");
    assert!(cx.debug_bounds("first-page-2").is_some());
}

#[gpui_kit::test]
fn a_new_name_goes_to_the_engine_when_the_sheet_closes(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = sheet_window(cx);
    open_sheet(cx, "profile-work");
    cx.update(|window, cx| {
        let input = view.read(cx).sheet.name.clone();
        input.update(cx, |input, cx| {
            input.set_value("Office", window, cx);
            input.focus(window, cx);
        });
    });
    cx.run_until_parked();
    // Return closes the sheet, as "Done" does, and keeps the name.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(!is_open(cx));
    assert_eq!(
        sent.borrow().as_slice(),
        [Command::RenameProfile {
            profile: "work".into(),
            name: "Office".into(),
        }]
    );

    // An empty name sends nothing.
    open_sheet(cx, "profile-work");
    cx.update(|window, cx| {
        let input = view.read(cx).sheet.name.clone();
        input.update(cx, |input, cx| input.set_value("  ", window, cx));
    });
    click(cx, "profile-done");
    assert_eq!(sent.borrow().len(), 1);
}

#[gpui_kit::test]
fn remove_asks_for_a_confirmation_and_the_sheet_closes_with_its_profile(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = sheet_window(cx);
    open_sheet(cx, "profile-work");
    click(cx, "profile-remove");
    let (message, _) = cx.pending_prompt().expect("a question");
    assert_eq!(message, "Remove the profile “Work”?");
    cx.simulate_prompt_answer("Remove");
    cx.run_until_parked();
    assert_eq!(
        last(&sent),
        Some(Command::RemoveProfile {
            profile: "work".into()
        })
    );
    send(
        cx,
        &tx,
        profiles(
            vec![entry(
                "default",
                "Default",
                &["Main", "Meetings", "Media"],
                0,
            )],
            "default",
        ),
    );
    assert!(!is_open(cx));

    // With one profile, the button does nothing.
    open_sheet(cx, "profile-default");
    click(cx, "profile-remove");
    assert!(!cx.has_pending_prompt());
}

#[gpui_kit::test]
fn export_and_import_send_the_file_and_an_import_shows_the_new_profile(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = sheet_window(cx);
    open_sheet(cx, "profile-work");
    click(cx, "profile-export");
    assert!(cx.did_prompt_for_new_path());
    let file = PathBuf::from("/exports/Work.opencraft");
    let chosen = file.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert_eq!(
        last(&sent),
        Some(Command::ExportProfile {
            profile: "work".into(),
            path: file.clone(),
        })
    );
    send(
        cx,
        &tx,
        Event::ProfileExported {
            profile: "work".into(),
            path: file.clone(),
        },
    );
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).status_message.as_deref(),
            Some("“Work” is in the file “Work.opencraft”")
        )
    });
    assert!(is_open(cx), "the sheet stays open");

    assert!(cx.debug_bounds("profile-import").is_none());
    click(cx, "profile-done");
    cx.dispatch_action(crate::app_menu::ImportProfile);
    cx.run_until_parked();
    assert!(cx.did_prompt_for_paths());
    let chosen = file.clone();
    cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
    cx.run_until_parked();
    assert_eq!(last(&sent), Some(Command::ImportProfile { path: file }));
    // The engine adds the profile and makes it active. No settings sheet
    // opens: the import adds a profile, not a setting of the old profile.
    send(
        cx,
        &tx,
        profiles(
            vec![
                entry("default", "Default", &["Main", "Meetings", "Media"], 0),
                entry("work", "Work", &["Mail", "Calls"], 1),
                entry("work-2", "Work 2", &["Mail", "Calls"], 1),
            ],
            "work-2",
        ),
    );
    send(cx, &tx, pages(&["Mail", "Calls"], 1));
    send(
        cx,
        &tx,
        Event::ProfileImported {
            profile: "work-2".into(),
        },
    );
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert!(view.sheet.profile.is_none());
        assert_eq!(view.state.profile.as_deref(), Some("work-2"));
        assert_eq!(view.state.pages, ["Mail", "Calls"]);
        assert_eq!(view.state.page, 1);
        assert_eq!(
            view.status_message.as_deref(),
            Some("The profile “Work 2” is imported, and it is on the device")
        );
    });
    // A cancelled dialog sends nothing.
    let count = sent.borrow().len();
    cx.dispatch_action(crate::app_menu::ImportProfile);
    cx.run_until_parked();
    cx.simulate_path_prompt_response(|_| None);
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), count);
}

#[gpui_kit::test]
fn the_keys_of_the_first_page_menu_do_not_close_the_sheet(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = sheet_window(cx);
    open_sheet(cx, "profile-work");
    click(cx, "first-page");
    // Down and Up go to the first item again. Return selects it, and the
    // sheet stays open.
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("up");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(cx.debug_bounds("first-page-menu").is_none());
    assert!(is_open(cx));
    assert_eq!(
        last(&sent),
        Some(Command::SetFirstPage {
            profile: "work".into(),
            page: 0,
        })
    );
    // With no menu, Return closes the sheet.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(!is_open(cx));
}

#[gpui_kit::test]
fn a_link_and_the_default_mark_send_the_commands(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = sheet_window(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.use_fixed_applications(
                vec![crate::applications::InstalledApplication {
                    name: "Safari".into(),
                    bundle_identifier: "com.apple.Safari".into(),
                    path: std::path::PathBuf::from("/Applications/Safari.app"),
                }],
                cx,
            );
        });
    });
    open_sheet(cx, "profile-work");
    click(cx, "profile-add-application");
    click(cx, "app-row-0");
    assert_eq!(
        last(&sent),
        Some(Command::LinkApplication {
            profile: "work".into(),
            link: opencraft_engine::ApplicationLink {
                bundle_id: "com.apple.Safari".into(),
                name: "Safari".into(),
            },
        })
    );
    // "default" has the mark, so Work can take it.
    click(cx, "default-profile");
    assert_eq!(
        last(&sent),
        Some(Command::SetDefaultProfile {
            profile: "work".into()
        })
    );
    // Escape closes the list, and it does not close the sheet.
    click(cx, "profile-add-application");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("application-picker").is_none());
    assert!(is_open(cx));
}

/// Decision 0035, section 5: before a link moves, the application list
/// shows the profile that already has it.
#[gpui_kit::test]
fn the_application_list_names_the_profile_that_already_has_the_link(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = sheet_window(cx);
    let mut home = entry("default", "Default", &["Main", "Meetings", "Media"], 0);
    home.applications.push(opencraft_engine::ApplicationLink {
        bundle_id: "com.apple.Safari".into(),
        name: "Safari".into(),
    });
    send(
        cx,
        &tx,
        profiles(
            vec![home, entry("work", "Work", &["Mail", "Calls"], 1)],
            "default",
        ),
    );
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.use_fixed_applications(
                vec![
                    crate::applications::InstalledApplication {
                        name: "Mail".into(),
                        bundle_identifier: "com.apple.mail".into(),
                        path: PathBuf::from("/Applications/Mail.app"),
                    },
                    crate::applications::InstalledApplication {
                        name: "Safari".into(),
                        bundle_identifier: "com.apple.Safari".into(),
                        path: PathBuf::from("/Applications/Safari.app"),
                    },
                ],
                cx,
            );
        });
    });
    open_sheet(cx, "profile-work");
    click(cx, "profile-add-application");
    let nodes = picker_labels(cx);
    assert!(
        nodes
            .iter()
            .any(|(role, label)| *role == Role::MenuItem && label == "Mail"),
        "an application with no link keeps its name: {nodes:#?}"
    );
    assert!(
        nodes.iter().any(|(role, label)| {
            *role == Role::MenuItem && label == "Safari, Linked to “Default”"
        }),
        "the row names the profile that has the link: {nodes:#?}"
    );
}

fn picker_labels(cx: &mut VisualTestContext) -> Vec<(Role, String)> {
    crate::view::controls::accessibility_log::take();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    crate::view::controls::accessibility_log::take()
}
