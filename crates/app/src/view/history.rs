//! Undo and redo in the window (decision 0030).
//!
//! The engine keeps the history. The window sends `Command::Undo` and
//! `Command::Redo` from the toolbar buttons, from ⌘Z and ⇧⌘Z, and from the
//! Edit menu. It tells the engine when a text field that edits the profile
//! gets and loses the focus, so that one edit of the field is one step.
//!
//! The focus rule of decision 0029, section 13: a focused text field keeps
//! ⌘Z and ⇧⌘Z, because its key context is deeper than the workspace. A click
//! on an Edit menu item sends the workspace action also when a field has the
//! focus, thus the handler gives the action to the field then
//! (`docs/kb/gpui.md`, "Key commands in the window").

use super::*;
use gpui_kit::{component::WindowExt, EntityId};

impl DeviceView {
    /// An event of a text field that edits the profile, for example the
    /// label field. Focus starts a text edit, and a loss of the focus ends
    /// it. Return ends the step of the edit, and the next change starts a
    /// new step.
    pub(super) fn text_edit_event(
        &mut self,
        field: EntityId,
        event: &TextEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            TextEvent::Focus => {
                // The loss of the focus of the field before can come after
                // this event. It then does not end this edit.
                self.text_edit = Some(field);
                (self.send)(Command::StartTextEdit);
                self.shell
                    .update(cx, |shell, cx| shell.set_typing(true, cx));
                cx.notify();
            }
            TextEvent::Blur if self.text_edit == Some(field) => self.end_text_edit(cx),
            TextEvent::PressEnter { .. } if self.text_edit == Some(field) => {
                (self.send)(Command::EndTextEdit);
                (self.send)(Command::StartTextEdit);
            }
            _ => {}
        }
    }

    /// End the open text edit, if there is one. The window calls this also
    /// when the focus goes back to the workspace: a field that goes away
    /// with the focus sends no event.
    pub(super) fn end_text_edit(&mut self, cx: &mut Context<Self>) {
        if self.text_edit.take().is_some() {
            (self.send)(Command::EndTextEdit);
            self.shell
                .update(cx, |shell, cx| shell.set_typing(false, cx));
            cx.notify();
        }
    }

    /// True if the window has a handler for Undo (`undo`) or Redo. macOS
    /// enables the Edit menu item only then. A focused text field has its
    /// own history.
    pub(super) fn can_history(&self, undo: bool, window: &mut Window, cx: &mut App) -> bool {
        let step = if undo {
            &self.state.undo
        } else {
            &self.state.redo
        };
        step.is_some() || self.text_edit.is_some() || window.focused_input(cx).is_some()
    }

    /// Undo (`undo`) or Redo: of the focused text field, or else of the
    /// profile.
    pub(super) fn history(&mut self, undo: bool, window: &mut Window, cx: &mut Context<Self>) {
        if window.focused_input(cx).is_some() {
            if undo {
                window.dispatch_action(Box::new(gpui_kit::component::input::Undo), cx);
            } else {
                window.dispatch_action(Box::new(gpui_kit::component::input::Redo), cx);
            }
            return;
        }
        (self.send)(if undo { Command::Undo } else { Command::Redo });
    }
}
