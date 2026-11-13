//! The sidebar: the device card, the profiles, the pages, their menus, and
//! the edit of a name in a row (decision 0029, section 7).
//!
//! The right-click menu is a native menu. The test platform shows no native
//! menu, thus a test does the right click and then sends the action of the
//! menu item, as the menu does.

use super::pages::{pages, profiles, send};
use super::*;
use crate::view::pages::{MenuItem, Row};
use crate::view::sidebar::{
    DuplicateRow, MoveRowDown, MoveRowUp, RemoveRow, RenameRow, SelectDevice,
};
use gpui_kit::{Action, MouseButton, MouseDownEvent, MouseUpEvent, Pixels};

const LIST: [(&str, &str); 2] = [("default", "Default"), ("work", "Work")];

/// A window with two profiles, and three pages in the active profile.
fn sidebar_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, profiles(&LIST, "default"));
    send(cx, &tx, pages(&["Main", "Meetings", "Media"], 0));
    (view, cx, tx, sent)
}

fn centre(cx: &mut VisualTestContext, id: &'static str) -> gpui_kit::Point<Pixels> {
    cx.debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"))
        .center()
}

fn double_click(cx: &mut VisualTestContext, id: &'static str) {
    let position = centre(cx, id);
    for click_count in [1, 2] {
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position,
            modifiers: Modifiers::default(),
            click_count,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position,
            modifiers: Modifiers::default(),
            click_count,
        });
    }
    cx.run_until_parked();
}

/// A right click on a row, and then a choice of a menu item.
fn menu(cx: &mut VisualTestContext, id: &'static str, item: impl Action) {
    let position = centre(cx, id);
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Right,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.run_until_parked();
    cx.dispatch_action(item);
    cx.run_until_parked();
}

fn name_text(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, cx| view.read(cx).lists.name.read(cx).value().to_string())
}

fn last(sent: &Commands) -> Option<Command> {
    sent.borrow().last().cloned()
}

#[gpui_kit::test]
fn the_sidebar_shows_the_profiles_and_the_pages_of_the_active_profile(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = sidebar_window(cx);
    for id in [
        "device-card",
        "profile-add",
        "profile-default",
        "profile-work",
        "page-0",
        "page-1",
        "page-2",
        "page-add",
        "sidebar-settings",
    ] {
        assert!(cx.debug_bounds(id).is_some(), "{id}");
    }
    // The pages are below the active profile, and above the next profile.
    let default = cx.debug_bounds("profile-default").unwrap();
    let add = cx.debug_bounds("page-add").unwrap();
    let work = cx.debug_bounds("profile-work").unwrap();
    assert!(default.bottom() <= cx.debug_bounds("page-0").unwrap().top());
    assert!(add.bottom() < work.top());
    // The page rows have the indent of the record.
    let sidebar = cx.debug_bounds("sidebar").unwrap();
    assert_eq!(default.left() - sidebar.left(), theme::SPACE_10);

    // A click on a page sends the command. The sidebar shows the page of
    // the engine, not the click.
    click(cx, "page-1");
    assert_eq!(last(&sent), Some(Command::SelectPage { page: 1 }));
    cx.update(|_, cx| assert_eq!(view.read(cx).state.page, 0));
    // An action on the device changes the page: the sidebar follows.
    send(cx, &tx, pages(&["Main", "Meetings", "Media"], 1));
    cx.update(|_, cx| assert_eq!(view.read(cx).state.page, 1));
    let count = sent.borrow().len();
    click(cx, "page-1");
    assert_eq!(sent.borrow().len(), count, "The active page is selected");

    click(cx, "page-add");
    assert_eq!(last(&sent), Some(Command::AddPage { copy_of: None }));

    // A click on a different profile selects it. Only the active profile
    // has page rows.
    click(cx, "profile-work");
    assert_eq!(
        last(&sent),
        Some(Command::SelectProfile {
            profile: "work".into()
        })
    );
    let count = sent.borrow().len();
    click(cx, "profile-default");
    assert_eq!(sent.borrow().len(), count, "The profile is active");
    send(cx, &tx, profiles(&LIST, "work"));
    assert!(cx.debug_bounds("page-0").is_none());
    send(cx, &tx, pages(&["Work"], 0));
    let work = cx.debug_bounds("profile-work").unwrap();
    let page = cx.debug_bounds("page-0").unwrap();
    assert!(work.bottom() <= page.top());
    assert!(cx.debug_bounds("page-1").is_none());
}

#[gpui_kit::test]
fn the_triangle_alone_hides_and_shows_the_pages(cx: &mut TestAppContext) {
    let (_, cx, tx, sent) = sidebar_window(cx);
    click(cx, "disclosure-default");
    assert!(cx.debug_bounds("page-0").is_none());
    assert!(cx.debug_bounds("page-add").is_none());
    click(cx, "disclosure-default");
    assert!(cx.debug_bounds("page-0").is_some());
    assert!(sent.borrow().is_empty(), "The triangle sends nothing");
    // The triangle of a different profile selects that profile.
    click(cx, "disclosure-work");
    assert_eq!(
        last(&sent),
        Some(Command::SelectProfile {
            profile: "work".into()
        })
    );
    // A new active profile shows its pages.
    click(cx, "disclosure-default");
    send(cx, &tx, profiles(&LIST, "work"));
    send(cx, &tx, pages(&["Work"], 0));
    assert!(cx.debug_bounds("page-0").is_some());
}

#[gpui_kit::test]
fn the_page_menu_sends_the_page_commands(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = sidebar_window(cx);
    menu(cx, "page-1", DuplicateRow);
    assert_eq!(last(&sent), Some(Command::AddPage { copy_of: Some(1) }));
    menu(cx, "page-1", MoveRowUp);
    assert_eq!(last(&sent), Some(Command::MovePage { page: 1, to: 0 }));
    menu(cx, "page-1", MoveRowDown);
    assert_eq!(last(&sent), Some(Command::MovePage { page: 1, to: 2 }));

    // "Move Up" is disabled on the first page, and "Move Down" on the last.
    let enabled = |view: &Entity<DeviceView>, cx: &mut VisualTestContext, page, item| {
        cx.update(|_, cx| {
            view.read(cx)
                .menu_items(&Row::Page(page))
                .contains(&(item, true))
        })
    };
    assert!(!enabled(&view, cx, 0, MenuItem::MoveUp));
    assert!(enabled(&view, cx, 0, MenuItem::MoveDown));
    assert!(!enabled(&view, cx, 2, MenuItem::MoveDown));
    let count = sent.borrow().len();
    menu(cx, "page-0", MoveRowUp);
    menu(cx, "page-2", MoveRowDown);
    assert_eq!(sent.borrow().len(), count);

    // "Remove…" asks first. The question names the page.
    menu(cx, "page-2", RemoveRow);
    let (message, _) = cx.pending_prompt().expect("a question");
    assert_eq!(message, "Remove the page “Media”?");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), count);
    menu(cx, "page-2", RemoveRow);
    cx.simulate_prompt_answer("Remove");
    cx.run_until_parked();
    assert_eq!(last(&sent), Some(Command::RemovePage { page: 2 }));

    // A change of the pages during the question can give the number to a
    // different page: the answer then does nothing.
    let count = sent.borrow().len();
    menu(cx, "page-1", RemoveRow);
    send(cx, &tx, pages(&["Main", "Media"], 0));
    cx.simulate_prompt_answer("Remove");
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), count);

    // The last page cannot go.
    send(cx, &tx, pages(&["Main"], 0));
    menu(cx, "page-0", RemoveRow);
    assert!(!cx.has_pending_prompt());
    assert_eq!(sent.borrow().len(), count);
}

#[gpui_kit::test]
fn a_page_name_is_edited_in_the_row(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = sidebar_window(cx);
    cx.update(|window, _| window.activate_window());

    // A double click selects the page and starts the edit.
    double_click(cx, "page-1");
    assert_eq!(last(&sent), Some(Command::SelectPage { page: 1 }));
    assert!(cx.debug_bounds("row-name").is_some());
    assert_eq!(name_text(&view, cx), "Meetings");
    // The engine makes the page active: the edit stays.
    send(cx, &tx, pages(&["Main", "Meetings", "Media"], 1));
    assert!(cx.debug_bounds("row-name").is_some());
    // Typed text replaces the old name. Return keeps the name.
    cx.simulate_input("Calls");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        last(&sent),
        Some(Command::RenamePage {
            page: 1,
            name: "Calls".into()
        })
    );
    assert!(cx.debug_bounds("row-name").is_none());
    cx.update(|window, cx| {
        assert!(
            view.read(cx).focus.is_focused(window),
            "The root has the focus"
        )
    });

    // Escape cancels.
    let count = sent.borrow().len();
    menu(cx, "page-2", RenameRow);
    assert_eq!(name_text(&view, cx), "Media");
    cx.simulate_input("Other");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("row-name").is_none());
    assert_eq!(sent.borrow().len(), count, "A cancelled edit sends nothing");

    // An empty name is not possible: Return then does nothing.
    menu(cx, "page-2", RenameRow);
    cx.simulate_keystrokes("backspace enter");
    cx.run_until_parked();
    assert!(cx.debug_bounds("row-name").is_some());
    assert_eq!(sent.borrow().len(), count);
    // A click outside keeps the name.
    cx.simulate_input("Music");
    click(cx, "canvas");
    assert!(cx.debug_bounds("row-name").is_none());
    assert_eq!(
        last(&sent),
        Some(Command::RenamePage {
            page: 2,
            name: "Music".into()
        })
    );

    // The same name sends nothing.
    let count = sent.borrow().len();
    double_click(cx, "page-0");
    cx.simulate_keystrokes("enter");

    cx.run_until_parked();
    assert!(cx.debug_bounds("row-name").is_none());
    assert_eq!(
        sent.borrow().len(),
        count + 1,
        "Only the selection of the page"
    );

    // A change of the names of the pages stops the edit: the number can be
    // a different page now.
    menu(cx, "page-1", RenameRow);
    send(cx, &tx, pages(&["Main", "Music"], 0));
    assert!(cx.debug_bounds("row-name").is_none());
    cx.update(|window, cx| {
        assert!(
            view.read(cx).focus.is_focused(window),
            "The root has the focus"
        )
    });
}

#[gpui_kit::test]
fn the_profile_menu_and_the_add_button_send_the_profile_commands(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = sidebar_window(cx);
    cx.update(|window, _| window.activate_window());

    // "+" adds a row with a name field. Return adds the profile.
    click(cx, "profile-add");
    assert!(cx.debug_bounds("profile-new").is_some());
    assert_eq!(name_text(&view, cx), "New profile");
    cx.simulate_input("Streaming");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        last(&sent),
        Some(Command::AddProfile {
            name: "Streaming".into(),
            copy_of: None
        })
    );
    assert!(cx.debug_bounds("profile-new").is_none());
    // Escape removes the new row, and sends nothing.
    let count = sent.borrow().len();
    click(cx, "profile-add");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("profile-new").is_none());
    assert_eq!(sent.borrow().len(), count);
    // A click outside with an empty name removes the row also.
    click(cx, "profile-add");
    cx.simulate_keystrokes("backspace enter");
    cx.run_until_parked();
    assert!(cx.debug_bounds("profile-new").is_some());
    click(cx, "canvas");
    assert!(cx.debug_bounds("profile-new").is_none());
    assert_eq!(sent.borrow().len(), count);

    // "Duplicate" adds a copy at once, with a free name.
    send(
        cx,
        &tx,
        profiles(
            &[
                ("default", "Default"),
                ("work", "Work"),
                ("copy", "Work copy"),
            ],
            "default",
        ),
    );
    menu(cx, "profile-work", DuplicateRow);
    assert_eq!(
        last(&sent),
        Some(Command::AddProfile {
            name: "Work copy 2".into(),
            copy_of: Some("work".into())
        })
    );

    // A profile that is not active can get a new name.
    menu(cx, "profile-work", RenameRow);
    assert_eq!(name_text(&view, cx), "Work");
    cx.simulate_input("Office");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        last(&sent),
        Some(Command::RenameProfile {
            profile: "work".into(),
            name: "Office".into()
        })
    );
    // A double click on the active profile also starts the edit.
    double_click(cx, "profile-default");
    assert_eq!(name_text(&view, cx), "Default");
    cx.simulate_input("Home");
    click(cx, "canvas");
    assert_eq!(
        last(&sent),
        Some(Command::RenameProfile {
            profile: "default".into(),
            name: "Home".into()
        })
    );

    // "Remove…" asks first. The question names the profile.
    let count = sent.borrow().len();
    menu(cx, "profile-work", RemoveRow);
    let (message, _) = cx.pending_prompt().expect("a question");
    assert_eq!(message, "Remove the profile “Work”?");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), count);
    menu(cx, "profile-work", RemoveRow);
    cx.simulate_prompt_answer("Remove");
    cx.run_until_parked();
    assert_eq!(
        last(&sent),
        Some(Command::RemoveProfile {
            profile: "work".into()
        })
    );

    // The edit of a profile stops when the profile goes.
    menu(cx, "profile-copy", RenameRow);
    send(cx, &tx, profiles(&LIST, "default"));
    assert!(cx.debug_bounds("row-name").is_none());

    // The last profile cannot go.
    let count = sent.borrow().len();
    send(cx, &tx, profiles(&[("default", "Default")], "default"));
    menu(cx, "profile-default", RemoveRow);
    assert!(!cx.has_pending_prompt());
    assert_eq!(sent.borrow().len(), count);
}

#[gpui_kit::test]
fn removal_of_the_active_larger_profile_replaces_its_pages(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = sidebar_window(cx);
    send(cx, &tx, pages(&["Main", "Meetings", "Media"], 2));
    menu(cx, "profile-default", RemoveRow);
    cx.simulate_prompt_answer("Remove");
    cx.run_until_parked();
    assert_eq!(
        last(&sent),
        Some(Command::RemoveProfile {
            profile: "default".into()
        })
    );
    // A profile change sends the surfaces, the profile list, then the pages.
    send(cx, &tx, surface(key(), "Work"));
    send(cx, &tx, profiles(&[("work", "Work")], "work"));
    send(cx, &tx, pages(&["Work"], 0));
    assert!(cx.debug_bounds("profile-default").is_none());
    assert!(cx.debug_bounds("page-1").is_none());
    assert!(cx.debug_bounds("page-2").is_none());
    cx.update(|_, cx| {
        let state = &view.read(cx).state;
        assert_eq!(state.profile.as_deref(), Some("work"));
        assert_eq!(state.pages, ["Work"]);
        assert_eq!(state.page, 0);
    });
    let count = sent.borrow().len();
    click(cx, "page-0");
    click(cx, "surface-key face 2");
    assert_eq!(sent.borrow().len(), count);
}

#[gpui_kit::test]
fn the_device_card_selects_the_device(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = sidebar_window(cx);
    click(cx, "device-card");
    cx.update(|_, cx| assert!(view.read(cx).device.open));
    // A second click keeps the device as the selection.
    click(cx, "device-card");
    cx.update(|_, cx| assert!(view.read(cx).device.open));
    // The menu of the device switcher has the one device.
    click(cx, "surface-key face 2");
    cx.update(|_, cx| assert!(!view.read(cx).device.open));
    cx.dispatch_action(SelectDevice);
    cx.run_until_parked();
    cx.update(|_, cx| assert!(view.read(cx).device.open));
    // A click on the active page makes the page the selection again.
    click(cx, "page-0");
    cx.update(|_, cx| assert!(!view.read(cx).device.open));
    assert!(sent.borrow().is_empty());
}

#[gpui_kit::test]
fn the_settings_row_opens_the_settings_window(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = sidebar_window(cx);
    let windows = |cx: &mut VisualTestContext| cx.update(|_, cx| cx.windows().len());
    assert_eq!(windows(cx), 1);
    click(cx, "sidebar-settings");
    assert_eq!(windows(cx), 2, "The Settings window opened");
    // A second click brings the same window to the front.
    click(cx, "sidebar-settings");
    assert_eq!(windows(cx), 2);
    // The row sends no command to the engine.
    assert!(sent.borrow().is_empty(), "{:?}", sent.borrow());
}
