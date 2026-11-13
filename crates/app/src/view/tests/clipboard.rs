//! Copy, Paste, Clear, and Copy to All Pages of a control: the key
//! commands, the Edit menu, the "⋯" menu, and the right-click menu
//! (decision 0029, sections 8.3, 10.2, and 13). The engine does the work:
//! these tests check the commands that the window sends.

use super::*;
use crate::app_menu::{Clear, Copy, CopyToAllPages, Paste};
use gpui_kit::{MouseButton, MouseDownEvent};

const KEY: ControlId = ControlId::new(ControlKind::Key, 2);
const KNOB: ControlId = ControlId::new(ControlKind::Knob, 2);

/// The clipboard commands in `sent`: all others are not of this test.
fn clipboard_commands(sent: &Commands) -> Vec<Command> {
    sent.borrow()
        .iter()
        .filter(|c| {
            matches!(
                c,
                Command::CopyControl { .. }
                    | Command::PasteControl { .. }
                    | Command::ClearControl { .. }
                    | Command::CopyToAllPages { .. }
            )
        })
        .cloned()
        .collect()
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

fn available(cx: &mut VisualTestContext, action: &dyn gpui_kit::Action) -> bool {
    cx.update(|window, cx| window.is_action_available(action, cx))
}

fn message(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| view.read(cx).status_message.as_ref().map(|m| m.to_string()))
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

fn right_click(cx: &mut VisualTestContext, id: &'static str) {
    let position = cx.debug_bounds(id).expect(id).center();
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Right,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
fn the_edit_menu_has_copy_paste_and_clear_after_the_history(cx: &mut TestAppContext) {
    let (_, cx, _, _) = window(cx);
    assert_eq!(
        super::history::edit_menu_items(cx),
        ["Undo", "Redo", "Copy", "Paste", "Clear"]
    );
}

#[gpui_kit::test]
fn a_key_is_copied_and_pasted_with_the_keys(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    assert!(available(cx, &Copy));
    assert!(!available(cx, &Paste), "the clipboard is empty");
    keys(cx, "cmd-c");
    assert_eq!(
        clipboard_commands(&sent),
        [Command::CopyControl { control: KEY }]
    );
    assert_eq!(message(&view, cx).as_deref(), Some("Key 2 is copied"));

    // The engine tells the kind of the copy. Then Paste is possible.
    send(cx, &tx, Event::Clipboard(ControlKind::Key));
    assert!(available(cx, &Paste));
    keys(cx, "cmd-v");
    assert_eq!(
        clipboard_commands(&sent).last(),
        Some(&Command::PasteControl { control: KEY })
    );
}

#[gpui_kit::test]
fn a_paste_on_a_control_of_a_different_kind_is_not_possible(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, Event::Clipboard(ControlKind::Key));
    click(cx, "control-knob 2");
    assert!(!available(cx, &Paste), "the Edit menu item is disabled");
    keys(cx, "cmd-v");
    assert!(clipboard_commands(&sent).is_empty());
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("You can paste a key only on a key")
    );
    // A knob copies to a knob.
    keys(cx, "cmd-c");
    assert_eq!(
        clipboard_commands(&sent),
        [Command::CopyControl { control: KNOB }]
    );
}

#[gpui_kit::test]
fn delete_clears_a_control_with_content(cx: &mut TestAppContext) {
    let (_, cx, _, sent) = window(cx);
    click(cx, "surface-key face 2");
    assert!(available(cx, &Clear), "the key has the label Mail");
    keys(cx, "backspace");
    assert_eq!(
        clipboard_commands(&sent),
        [Command::ClearControl { control: KEY }]
    );
    // The page has no clipboard commands.
    click(cx, "help-line");
    assert!(!available(cx, &Clear));
    assert!(!available(cx, &Copy));
}

#[gpui_kit::test]
fn copy_to_all_pages_and_the_menus_send_the_command_for_the_selection(cx: &mut TestAppContext) {
    let (_, cx, _, sent) = window(cx);
    click(cx, "surface-key face 2");
    // The "⋯" button is enabled for a control. The native menu shows
    // nothing on the test platform: the test sends the action of the item.
    click(cx, "more-button");
    cx.dispatch_action(CopyToAllPages);
    cx.run_until_parked();
    assert_eq!(
        clipboard_commands(&sent),
        [Command::CopyToAllPages { control: KEY }]
    );

    // A right click selects the control, and its menu is for it.
    right_click(cx, "control-knob 2");
    cx.dispatch_action(Copy);
    cx.run_until_parked();
    assert_eq!(
        clipboard_commands(&sent).last(),
        Some(&Command::CopyControl { control: KNOB })
    );
}

#[gpui_kit::test]
fn a_focused_text_field_keeps_its_copy_paste_and_delete(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = window(cx);
    cx.update(|window, _| window.activate_window());
    click(cx, "surface-key face 2");
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        window.focus(&input.focus_handle(cx), cx);
    });
    cx.run_until_parked();
    keys(cx, "cmd-a cmd-c backspace");
    cx.update(|_, cx| assert_eq!(view.read(cx).input.read(cx).value().as_ref(), ""));
    keys(cx, "cmd-v");
    cx.update(|_, cx| assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "Mail"));
    // The Edit menu item goes to the field too.
    cx.update(|window, cx| window.dispatch_action(Box::new(Clear), cx));
    cx.run_until_parked();
    assert!(clipboard_commands(&sent).is_empty(), "{:?}", sent.borrow());
}
