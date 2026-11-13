//! The command palette (decision 0029, section 15): how it opens and
//! closes, each group of results, the search, each key, and the main
//! operation of each kind of row.

use super::pages::{pages, send};
use super::*;
use crate::app_menu::Search;
use crate::settings::RightPanelTab;
use crate::view::library::{assigned_text, SELECT_FIRST};
use crate::view::palette::{Group, Target, GROUP_LIMIT};
use gpui_kit::{Bounds, Pixels};
use opencraft_engine::ProfileEntry;

fn kind(id: &str, name: &str, group: &str) -> KindDescription {
    KindDescription {
        kind: id.into(),
        name: name.into(),
        group: group.into(),
        ..Default::default()
    }
}

/// Six kinds in two groups: one more than a group of the palette shows.
fn kinds() -> Event {
    Event::Kinds(vec![
        kind("test.open", "Open an application", "System"),
        kind("test.media", "Media key", "Media"),
        kind("test.url", "Open a URL", "System"),
        kind("test.shell", "Run a shell command", "System"),
        kind("test.hotkey", "Send a hotkey", "System"),
        kind("test.text", "Type text", "System"),
    ])
}

fn entry(id: &str, name: &str, pages: &[&str]) -> ProfileEntry {
    ProfileEntry {
        id: id.into(),
        name: name.into(),
        pages: pages.iter().map(|p| p.to_string()).collect(),
        ..Default::default()
    }
}

/// The window with the kinds, the active profile `default` (three pages),
/// and the profile `work` (two pages).
fn palette_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, kinds());
    send(
        cx,
        &tx,
        Event::Profiles {
            profiles: vec![
                entry("default", "Default", &["Main", "Meetings", "Media"]),
                entry("work", "Work", &["Mail", "Calls"]),
            ],
            active: "default".into(),
        },
    );
    send(cx, &tx, pages(&["Main", "Meetings", "Media"], 0));
    (view, cx, tx, sent)
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

fn is_open(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> bool {
    let open = cx.update(|_, cx| view.read(cx).palette.open);
    assert_eq!(open, cx.debug_bounds("palette").is_some());
    open
}

/// Open the palette with ⌘K, and type `text`.
fn search(view: &Entity<DeviceView>, cx: &mut VisualTestContext, text: &str) {
    keys(cx, "cmd-k");
    assert!(is_open(view, cx));
    if !text.is_empty() {
        cx.simulate_input(text);
        cx.run_until_parked();
    }
}

/// The group, the name, and the target of each row.
fn rows(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> Vec<(Group, String, Target)> {
    cx.update(|_, cx| {
        view.read(cx)
            .palette_items(cx)
            .into_iter()
            .map(|item| (item.group, item.name.to_string(), item.target))
            .collect()
    })
}

fn selected(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> usize {
    cx.update(|_, cx| view.read(cx).palette.selected)
}

fn bounds(cx: &mut VisualTestContext, id: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"))
}

/// True if the element `id` is fully inside the visible list.
fn visible(cx: &mut VisualTestContext, id: &'static str) -> bool {
    let list = bounds(cx, "palette-results");
    let row = bounds(cx, id);
    row.top() >= list.top() && row.bottom() <= list.bottom()
}

fn message(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| view.read(cx).status_message.as_ref().map(|m| m.to_string()))
}

fn workspace_has_focus(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| view.read(cx).focus.is_focused(window))
}

#[gpui_kit::test]
fn the_palette_opens_with_the_key_the_menu_and_the_toolbar_field(cx: &mut TestAppContext) {
    let (view, cx, _, _) = palette_window(cx);
    // The size and the place of the record.
    keys(cx, "cmd-k");
    assert!(is_open(&view, cx));
    let card = bounds(cx, "palette");
    assert_eq!(card.size.width, theme::PALETTE_WIDTH);
    assert_eq!(card.size.height, theme::PALETTE_HEIGHT);
    assert_eq!(card.top(), theme::PALETTE_TOP);
    // The search field has the focus.
    cx.update(|window, cx| {
        let query = view.read(cx).palette.query.clone();
        assert!(Focusable::focus_handle(query.read(cx), cx).is_focused(window));
    });
    // The workspace key commands do not operate while the palette is open.
    keys(cx, "ctrl-cmd-s");
    cx.update(|_, cx| assert!(view.read(cx).panes.sidebar_visible));
    keys(cx, "escape");
    assert!(!is_open(&view, cx));
    assert!(workspace_has_focus(&view, cx));

    // "Search…" of the Edit menu sends the same action.
    cx.dispatch_action(Search);
    cx.run_until_parked();
    assert!(is_open(&view, cx));
    // A click out of the card closes the palette.
    let layer = bounds(cx, "palette-layer");
    let outside = layer.origin + gpui_kit::point(px(20.), px(20.));
    cx.simulate_click(outside, Modifiers::default());
    cx.run_until_parked();
    assert!(!is_open(&view, cx));

    // The search field of the toolbar is 240 × 28, at the end of the
    // toolbar, between the pill and the inspector button.
    let field = bounds(cx, "toolbar-search");
    assert_eq!(field.size.width, theme::WIDTH_TOOLBAR_SEARCH);
    assert_eq!(field.size.height, theme::HEIGHT_CONTROL);
    let pill = bounds(cx, "connection");
    assert_eq!(field.left() - pill.right(), theme::SPACE_8);
    assert!(field.right() < bounds(cx, "toggle-inspector").left());
    click(cx, "toolbar-search");
    assert!(is_open(&view, cx));
}

#[gpui_kit::test]
fn cmd_k_opens_the_palette_also_from_a_text_field(cx: &mut TestAppContext) {
    let (view, cx, _, _) = palette_window(cx);
    click(cx, "surface-key face 2");
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        window.focus(&input.focus_handle(cx), cx);
    });
    cx.run_until_parked();
    keys(cx, "cmd-k");
    assert!(is_open(&view, cx));
    // The letters go to the palette, not to the label field.
    cx.simulate_input("Knob");
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "Mail");
        assert_eq!(
            view.read(cx).palette.query.read(cx).value().as_ref(),
            "Knob"
        );
    });
    keys(cx, "escape");
    assert!(workspace_has_focus(&view, cx));
    // A new opening has an empty field.
    search(&view, cx, "");
    cx.update(|_, cx| assert!(view.read(cx).palette.query.read(cx).value().is_empty()));
}

#[gpui_kit::test]
fn an_empty_field_shows_each_group_with_its_first_rows(cx: &mut TestAppContext) {
    let (view, cx, _, _) = palette_window(cx);
    search(&view, cx, "");
    let rows = rows(&view, cx);
    let groups: Vec<Group> = rows
        .iter()
        .map(|(g, _, _)| *g)
        .fold(Vec::new(), |mut v, g| {
            if v.last() != Some(&g) {
                v.push(g);
            }
            v
        });
    assert_eq!(
        groups,
        [
            Group::Actions,
            Group::Controls,
            Group::Pages,
            Group::Profiles,
            Group::Commands
        ]
    );
    let names = |group: Group| -> Vec<String> {
        rows.iter()
            .filter(|(g, _, _)| *g == group)
            .map(|(_, n, _)| n.clone())
            .collect()
    };
    // Actions: the order of the library (the groups in the order of their
    // first kind), 5 rows at the most.
    assert_eq!(
        names(Group::Actions),
        [
            "Open an application",
            "Open a URL",
            "Run a shell command",
            "Send a hotkey",
            "Type text"
        ]
    );
    // Controls: the controls of the layout, with no touch bar.
    assert_eq!(names(Group::Controls), ["Key 2", "Knob 2", "Bar panel 2"]);
    // Pages: the active profile first.
    assert_eq!(
        names(Group::Pages),
        ["Main", "Meetings", "Media", "Mail", "Calls"]
    );
    assert_eq!(names(Group::Profiles), ["Default", "Work"]);
    // Commands: the commands of section 15 first. "Search…" is not there.
    assert_eq!(
        names(Group::Commands),
        [
            "Add a page",
            "Page settings",
            "Device settings",
            "Add a profile",
            "Settings…"
        ]
    );
    assert!(rows
        .iter()
        .all(|(g, _, _)| rows.iter().filter(|(h, _, _)| h == g).count() <= GROUP_LIMIT));
    for group in [
        "palette-group-Actions",
        "palette-group-Controls",
        "palette-group-Pages",
        "palette-group-Profiles",
        "palette-group-Commands",
    ] {
        assert!(cx.debug_bounds(group).is_some(), "{group} does not show");
    }
    // The first row is selected, and its hint is at the end of the row.
    assert_eq!(selected(&view, cx), 0);
    let row = bounds(cx, "palette-row-0");
    assert_eq!(row.size.height, theme::HEIGHT_PALETTE_ROW);
}

#[gpui_kit::test]
fn the_text_filters_each_group(cx: &mut TestAppContext) {
    let (view, cx, _, _) = palette_window(cx);
    // A control by its label, and a page with the same name.
    search(&view, cx, "MAIL");
    assert_eq!(
        rows(&view, cx),
        [
            (
                Group::Controls,
                "Key 2".to_string(),
                Target::Control(ControlId::new(ControlKind::Key, 2))
            ),
            (
                Group::Pages,
                "Mail".to_string(),
                Target::Page {
                    profile: "work".into(),
                    page: 0
                }
            ),
        ]
    );
    keys(cx, "escape");

    // A control by its name.
    search(&view, cx, "knob");
    assert_eq!(
        rows(&view, cx),
        [(
            Group::Controls,
            "Knob 2".to_string(),
            Target::Control(ControlId::new(ControlKind::Knob, 2))
        )]
    );
    keys(cx, "escape");

    // Actions of all library groups, in the order of the library.
    search(&view, cx, "key");
    let names: Vec<_> = rows(&view, cx)
        .into_iter()
        .map(|(g, n, _)| (g, n))
        .collect();
    assert_eq!(
        names,
        [
            (Group::Actions, "Send a hotkey".to_string()),
            (Group::Actions, "Media key".to_string()),
            (Group::Controls, "Key 2".to_string()),
        ]
    );
    keys(cx, "escape");

    // A profile, and a command that is not in the first five.
    search(&view, cx, "work");
    assert_eq!(
        rows(&view, cx),
        [(
            Group::Profiles,
            "Work".to_string(),
            Target::Profile("work".into())
        )]
    );
    keys(cx, "escape");
    search(&view, cx, "inspector");
    let names: Vec<_> = rows(&view, cx).into_iter().map(|(_, n, _)| n).collect();
    assert_eq!(names, ["Hide Inspector"]);
    keys(cx, "escape");

    // No result.
    search(&view, cx, "zzz");
    assert!(rows(&view, cx).is_empty());
    assert!(cx.debug_bounds("palette-no-result").is_some());
    assert!(cx.debug_bounds("palette-main").is_none());
    keys(cx, "enter");
    assert!(is_open(&view, cx), "Return with no result does nothing");
}

#[gpui_kit::test]
fn the_commands_are_the_commands_that_the_window_can_do_now(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = palette_window(cx);
    // With an empty history, Undo is not there.
    search(&view, cx, "undo");
    assert!(rows(&view, cx).is_empty());
    keys(cx, "escape");
    send(
        cx,
        &tx,
        Event::History {
            undo: Some("Set Title".into()),
            redo: None,
        },
    );
    search(&view, cx, "undo");
    let names: Vec<_> = rows(&view, cx).into_iter().map(|(_, n, _)| n).collect();
    assert_eq!(names, ["Undo Set Title"]);
    keys(cx, "enter");
    assert!(!is_open(&view, cx));
    assert_eq!(sent.borrow().last(), Some(&Command::Undo));

    // "Hide Sidebar" does the command, and its title follows the state.
    search(&view, cx, "sidebar");
    let names: Vec<_> = rows(&view, cx).into_iter().map(|(_, n, _)| n).collect();
    assert_eq!(names, ["Hide Sidebar"]);
    keys(cx, "enter");
    cx.update(|_, cx| assert!(!view.read(cx).panes.sidebar_visible));
    assert!(workspace_has_focus(&view, cx));
    search(&view, cx, "sidebar");
    let names: Vec<_> = rows(&view, cx).into_iter().map(|(_, n, _)| n).collect();
    assert_eq!(names, ["Show Sidebar"]);
    // A click on a row does the main operation also.
    click(cx, "palette-row-0");
    cx.update(|_, cx| assert!(view.read(cx).panes.sidebar_visible));

    // The commands that only the palette has.
    search(&view, cx, "add a page");
    keys(cx, "enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::AddPage { copy_of: None })
    );
    search(&view, cx, "device settings");
    keys(cx, "enter");
    cx.update(|_, cx| {
        assert!(view.read(cx).device.open);
        assert_eq!(
            view.read(cx).panes.right_panel_tab,
            RightPanelTab::Inspector
        );
    });
    search(&view, cx, "page settings");
    keys(cx, "enter");
    cx.update(|_, cx| {
        assert!(!view.read(cx).device.open);
        assert!(view.read(cx).state.page_selected);
    });
    // "Add a profile" shows the sidebar, with the field of the new name.
    keys(cx, "ctrl-cmd-s");
    search(&view, cx, "add a profile");
    keys(cx, "enter");
    cx.update(|_, cx| {
        assert!(view.read(cx).panes.sidebar_visible);
        assert_eq!(
            view.read(cx).lists.edit,
            Some(crate::view::pages::NameEdit::NewProfile)
        );
    });
}

#[gpui_kit::test]
fn the_palette_import_command_uses_the_profile_file_dialog(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = palette_window(cx);
    search(&view, cx, "import a profile");
    let names: Vec<_> = rows(&view, cx).into_iter().map(|(_, n, _)| n).collect();
    assert_eq!(names, ["Import a profile…"]);
    keys(cx, "enter");
    assert!(!is_open(&view, cx));
    assert!(cx.did_prompt_for_paths());
    let path = std::path::PathBuf::from("/imports/Checks.opencraft");
    let chosen = path.clone();
    cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
    cx.run_until_parked();
    assert_eq!(sent.borrow().last(), Some(&Command::ImportProfile { path }));
    assert!(workspace_has_focus(&view, cx));
}

#[gpui_kit::test]
fn the_arrow_keys_move_the_selection_and_keep_it_visible(cx: &mut TestAppContext) {
    let (view, cx, _, _) = palette_window(cx);
    search(&view, cx, "");
    let count = rows(&view, cx).len();
    assert!(count > 15, "The list is longer than the palette");
    // Up at the first row stays there.
    keys(cx, "up");
    assert_eq!(selected(&view, cx), 0);
    // Down moves across the groups.
    for n in 1..count {
        keys(cx, "down");
        assert_eq!(selected(&view, cx), n);
    }
    keys(cx, "down");
    assert_eq!(selected(&view, cx), count - 1);
    // The list scrolled: the last row shows, the first row does not.
    let last: &'static str = Box::leak(format!("palette-row-{}", count - 1).into_boxed_str());
    assert!(visible(cx, last), "The selected row is not visible");
    assert!(!visible(cx, "palette-row-0"));
    // Up moves back, and the list scrolls back to the top with the header.
    for _ in 0..count - 1 {
        keys(cx, "up");
    }
    assert_eq!(selected(&view, cx), 0);
    assert!(visible(cx, "palette-row-0"));
    assert!(visible(cx, "palette-group-Actions"));
    // ⌃N and ⌃P also move the selection.
    keys(cx, "ctrl-n");
    assert_eq!(selected(&view, cx), 1);
    keys(cx, "ctrl-p");
    assert_eq!(selected(&view, cx), 0);
    // A new text selects the first row again.
    keys(cx, "down");
    cx.simulate_input("e");
    cx.run_until_parked();
    assert_eq!(selected(&view, cx), 0);
    // The text of the field did not change with the arrow keys.
    cx.update(|_, cx| assert_eq!(view.read(cx).palette.query.read(cx).value().as_ref(), "e"));
}

#[gpui_kit::test]
fn return_assigns_an_action_to_the_selected_control(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = palette_window(cx);
    click(cx, "surface-key face 2");
    search(&view, cx, "url");
    // The chip names the selected control.
    assert!(cx.debug_bounds("palette-context").is_some());
    keys(cx, "enter");
    assert!(!is_open(&view, cx));
    let set = sent.borrow().iter().rev().find_map(|c| match c {
        Command::SetBinding { binding, action } => Some((*binding, action.clone())),
        _ => None,
    });
    let (binding, action) = set.expect("a binding command");
    assert_eq!(binding.control, ControlId::new(ControlKind::Key, 2));
    assert_eq!(binding.gesture, GestureKind::Press);
    assert_eq!(action.unwrap().kind, "test.url");
    assert_eq!(
        message(&view, cx),
        Some(assigned_text(
            ControlId::new(ControlKind::Key, 2),
            "Open a URL",
            "Press"
        ))
    );

    // ⌘↵ also shows the Inspector tab.
    click(cx, "tab-library");
    search(&view, cx, "type text");
    keys(cx, "cmd-enter");
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).panes.right_panel_tab,
            RightPanelTab::Inspector
        )
    });

    // With the page as the selection, there is no control for the action.
    cx.update(|window, cx| view.update(cx, |view, cx| view.select_page(window, cx)));
    let count = sent.borrow().len();
    search(&view, cx, "url");
    assert!(cx.debug_bounds("palette-context").is_none());
    assert!(cx.debug_bounds("palette-inspector").is_none());
    keys(cx, "enter");
    assert_eq!(sent.borrow().len(), count);
    assert_eq!(message(&view, cx).as_deref(), Some(SELECT_FIRST));
}

#[gpui_kit::test]
fn return_selects_a_control_a_page_and_a_profile(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = palette_window(cx);
    // A control.
    search(&view, cx, "knob");
    keys(cx, "enter");
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.selected_control,
            Some(ControlId::new(ControlKind::Knob, 2))
        )
    });
    // ⌘↵ on a control shows it in the Inspector tab.
    click(cx, "tab-library");
    search(&view, cx, "bar panel");
    keys(cx, "cmd-enter");
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.selected_control,
            Some(ControlId::new(ControlKind::BarPanel, 2))
        );
        assert_eq!(
            view.read(cx).panes.right_panel_tab,
            RightPanelTab::Inspector
        );
    });

    // A page of the active profile.
    search(&view, cx, "meetings");
    keys(cx, "enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&super::pages::on(
            "default",
            Command::SelectPage { page: 1 }
        ))
    );
    // A page of a different profile: the profile, then the page.
    search(&view, cx, "calls");
    keys(cx, "enter");
    let sent_now = sent.borrow();
    assert_eq!(
        &sent_now[sent_now.len() - 2..],
        &[
            Command::SelectProfile {
                profile: "work".into()
            },
            Command::SelectPage { page: 1 },
        ]
    );
    drop(sent_now);
    // ⌘↵ on a page makes the page the selection.
    search(&view, cx, "main");
    keys(cx, "cmd-enter");
    cx.update(|_, cx| assert!(view.read(cx).state.page_selected));

    // A profile.
    search(&view, cx, "work");
    keys(cx, "enter");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SelectProfile {
            profile: "work".into()
        })
    );
    // The active profile: no command.
    let count = sent.borrow().len();
    search(&view, cx, "default");
    keys(cx, "enter");
    assert_eq!(sent.borrow().len(), count);
}
