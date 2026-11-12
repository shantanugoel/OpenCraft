//! Undo and redo: the toolbar buttons, the key commands, the Edit menu, and
//! the text edits (decision 0029, sections 5 and 13; decision 0030).

use super::*;
use crate::app_menu::{Redo, Undo};
use gpui_kit::OwnedMenuItem;

fn history(undo: Option<&str>, redo: Option<&str>) -> Event {
    Event::History {
        undo: undo.map(str::to_string),
        redo: redo.map(str::to_string),
    }
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

/// The commands of the history in `sent`: all others are not of this test.
fn history_commands(sent: &Commands) -> Vec<Command> {
    sent.borrow()
        .iter()
        .filter(|c| {
            matches!(
                c,
                Command::Undo | Command::Redo | Command::StartTextEdit | Command::EndTextEdit
            )
        })
        .cloned()
        .collect()
}

/// The titles of the items of the Edit menu.
fn edit_menu(cx: &mut VisualTestContext) -> Vec<String> {
    let menus = cx.update(|_, cx| cx.get_menus()).unwrap_or_default();
    menus
        .into_iter()
        .find(|m| m.name.as_ref() == "Edit")
        .expect("an Edit menu")
        .items
        .into_iter()
        .filter_map(|item| match item {
            OwnedMenuItem::Action { name, .. } => Some(name.to_string()),
            _ => None,
        })
        .collect()
}

/// True if the window has a handler for `action` now. macOS enables the
/// menu item of the action only then.
fn available(cx: &mut VisualTestContext, action: &dyn gpui_kit::Action) -> bool {
    cx.update(|window, cx| window.is_action_available(action, cx))
}

#[gpui_kit::test]
fn the_toolbar_buttons_send_undo_and_redo_only_with_a_step(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    // The engine sends the empty history at the start.
    send(cx, &tx, history(None, None));
    click(cx, "undo");
    click(cx, "redo");
    assert!(history_commands(&sent).is_empty(), "both are disabled");

    send(cx, &tx, history(Some("Set Title"), None));
    click(cx, "undo");
    click(cx, "redo");
    assert_eq!(history_commands(&sent), [Command::Undo]);

    send(cx, &tx, history(None, Some("Set Title")));
    click(cx, "undo");
    click(cx, "redo");
    assert_eq!(history_commands(&sent), [Command::Undo, Command::Redo]);
    cx.update(|_, cx| {
        let state = &view.read(cx).state;
        assert_eq!(
            (state.undo.as_deref(), state.redo.as_deref()),
            (None, Some("Set Title"))
        );
    });
}

#[gpui_kit::test]
fn the_toolbar_buttons_are_after_the_title_block(cx: &mut TestAppContext) {
    let (_, cx, _, _) = window(cx);
    let title = cx.debug_bounds("toolbar-title").unwrap();
    let undo = cx.debug_bounds("undo").unwrap();
    let redo = cx.debug_bounds("redo").unwrap();
    assert!(undo.left() >= title.right());
    assert!(redo.left() >= undo.right());
    assert_eq!(
        (undo.size.width, undo.size.height),
        (theme::WIDTH_ICON_BUTTON, theme::HEIGHT_CONTROL)
    );
}

#[gpui_kit::test]
fn the_keys_send_undo_and_redo_from_the_workspace(cx: &mut TestAppContext) {
    let (_, cx, tx, sent) = window(cx);
    send(cx, &tx, history(None, None));
    cx.simulate_keystrokes("cmd-z");
    cx.simulate_keystrokes("cmd-shift-z");
    cx.run_until_parked();
    assert!(history_commands(&sent).is_empty(), "the lists are empty");
    assert!(!available(cx, &Undo));
    assert!(!available(cx, &Redo));

    send(cx, &tx, history(Some("Set Fill"), Some("Add Page")));
    assert!(available(cx, &Undo));
    assert!(available(cx, &Redo));
    cx.simulate_keystrokes("cmd-z");
    cx.simulate_keystrokes("cmd-shift-z");
    cx.run_until_parked();
    assert_eq!(history_commands(&sent), [Command::Undo, Command::Redo]);
}

#[gpui_kit::test]
fn the_edit_menu_names_the_steps(cx: &mut TestAppContext) {
    let (_, cx, tx, _) = window(cx);
    assert_eq!(edit_menu(cx), ["Undo", "Redo"]);
    send(cx, &tx, history(Some("Set Title"), Some("Remove Page")));
    assert_eq!(edit_menu(cx), ["Undo Set Title", "Redo Remove Page"]);
    send(cx, &tx, history(None, None));
    assert_eq!(edit_menu(cx), ["Undo", "Redo"]);
}

#[gpui_kit::test]
fn a_focused_label_field_keeps_its_keys_and_its_edit_is_one_text_edit(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    // The test window must be active, else no loss of the focus comes
    // (`docs/kb/gpui.md`).
    cx.update(|window, _| window.activate_window());
    send(cx, &tx, history(Some("Set Fill"), None));
    click(cx, "surface-key face 2");
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        window.focus(&input.focus_handle(cx), cx);
    });
    cx.run_until_parked();
    assert_eq!(history_commands(&sent), [Command::StartTextEdit]);
    // While the field has the focus, the Edit menu names no step.
    assert_eq!(edit_menu(cx), ["Undo", "Redo"]);

    cx.simulate_input("X");
    cx.simulate_keystrokes("cmd-z");
    cx.run_until_parked();
    // The field undid the letter, and the profile got no undo.
    assert_eq!(history_commands(&sent), [Command::StartTextEdit]);
    cx.update(|_, cx| assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "Mail"));

    // Return ends the step of the edit. The field keeps the focus.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        history_commands(&sent),
        [
            Command::StartTextEdit,
            Command::EndTextEdit,
            Command::StartTextEdit
        ]
    );

    // An Edit menu item sends the workspace action: it goes to the field.
    cx.simulate_input("Y");
    cx.update(|window, cx| window.dispatch_action(Box::new(Undo), cx));
    cx.run_until_parked();
    assert_eq!(history_commands(&sent).len(), 3);
    cx.update(|_, cx| assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "Mail"));

    // The toolbar button ends the edit and undoes the step of the profile.
    click(cx, "undo");
    assert_eq!(
        history_commands(&sent)[3..],
        [Command::EndTextEdit, Command::Undo]
    );
    assert_eq!(edit_menu(cx), ["Undo Set Fill", "Redo"]);
}

#[gpui_kit::test]
fn a_click_on_the_canvas_ends_the_text_edit(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = window(cx);
    cx.update(|window, _| window.activate_window());
    click(cx, "surface-key face 2");
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        window.focus(&input.focus_handle(cx), cx);
    });
    cx.run_until_parked();
    click(cx, "surface-bar panel 3");
    assert_eq!(
        history_commands(&sent),
        [Command::StartTextEdit, Command::EndTextEdit]
    );
    cx.update(|_, cx| assert_eq!(view.read(cx).text_edit, None));
}

#[test]
fn the_titles_name_the_step() {
    use crate::app_menu::{redo_title, undo_title};
    assert_eq!(undo_title(Some("Set Title")), "Undo Set Title");
    assert_eq!(redo_title(Some("Move Page")), "Redo Move Page");
    assert_eq!(undo_title(None), "Undo");
    assert_eq!(redo_title(None), "Redo");
}
