//! The Accessibility request (decision 0029, section 18): a sheet on the
//! main window that asks for the Accessibility permission.
//!
//! The first start does not ask for the permission. The sheet opens when the
//! first binding with a kind that needs the permission gets on a control,
//! and the permission is not given:
//!
//! - when the window sets such a binding, and the engine sends the new
//!   binding back. The window notes each binding that it sends, so that the
//!   bindings of a page change do not open the sheet;
//! - when the user starts with a starter profile that has such an action
//!   (`Shell::start_with`). Then the main window opens with the sheet.
//!
//! The kinds that need the permission come from their descriptions
//! (`needs_accessibility`, section 20); the window has no list of kinds
//! (rule 8). The sheet shows one time in each run of the application: the
//! shell keeps that state. "Open System Settings" is the command of the
//! Settings window (`ShellCommand::OpenAccessibilitySettings`). The frame is
//! the shared modal layer of `modal`.

use super::{
    modal::{button_row, Modal, ModalKey},
    *,
};
use crate::{settings_window::kind_waiting_for_permission, shell::ShellCommand, starters};
use gpui_kit::{assets::IconName, component::Icon, relative, FontWeight};
use opencraft_engine::{BindingId, BindingState};
use std::{cell::RefCell, collections::BTreeSet};

/// The bindings that the window sent with an action, until the engine sends
/// them back.
pub(super) type SentBindings = Rc<RefCell<BTreeSet<BindingId>>>;

/// The state of the sheet. The kind name of the open sheet is
/// `ViewState::permission_request`.
pub(super) struct PermissionSheet {
    /// The focus of the card of the sheet.
    pub(super) focus: FocusHandle,
    /// See [`SentBindings`].
    pub(super) sent: SentBindings,
}

impl PermissionSheet {
    pub(super) fn new(cx: &mut Context<DeviceView>) -> Self {
        Self {
            focus: cx.focus_handle(),
            sent: SentBindings::default(),
        }
    }
}

/// `send`, with a note of each binding with an action in `command` in
/// `sent`. A group can contain bindings.
pub(super) fn noting_send(send: Rc<dyn Fn(Command)>, sent: SentBindings) -> Rc<dyn Fn(Command)> {
    Rc::new(move |command| {
        note(&command, &mut sent.borrow_mut());
        send(command)
    })
}

fn note(command: &Command, sent: &mut BTreeSet<BindingId>) {
    match command {
        Command::SetBinding {
            binding,
            action: Some(_),
        } => {
            sent.insert(*binding);
        }
        Command::Group { commands, .. } => {
            for command in commands {
                note(command, sent);
            }
        }
        _ => {}
    }
}

/// The text of the sheet for the kind `kind`.
pub(super) fn request_text(kind: &str) -> String {
    format!(
        "“{kind}” sends keys to other apps. macOS lets OpenCraft do this only with the \
         Accessibility permission. The binding stays in the profile. The action operates after \
         you give the permission."
    )
}

/// The title of the sheet.
pub(super) const REQUEST_TITLE: &str = "OpenCraft needs the Accessibility permission";

impl DeviceView {
    /// A binding from the engine. If the window sent it, and it has an
    /// action that needs the permission, which is not given, the sheet
    /// opens: one time in each run.
    pub(super) fn permission_event(
        &mut self,
        binding: &BindingState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.permission.sent.borrow_mut().remove(&binding.binding) {
            return;
        }
        let Some(action) = &binding.action else {
            return;
        };
        let mut actions = Vec::new();
        starters::add_with_contained(action, &mut actions);
        let Some(kind) = kind_waiting_for_permission(&actions, &self.state) else {
            return;
        };
        let shows = self
            .shell
            .update(cx, |shell, _| shell.request_permission(kind.clone()));
        if shows {
            self.open_permission_request(kind, window, cx);
        }
    }

    /// Show the sheet for the kind with the name `kind`.
    pub(super) fn open_permission_request(
        &mut self,
        kind: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.end_text_edit(cx);
        self.state.permission_request = Some(kind);
        window.focus(&self.permission.focus, cx);
        cx.notify();
    }

    /// Close the sheet. With `open_settings`, open the Accessibility pane
    /// of System Settings first, as the Settings window does.
    fn close_permission_request(
        &mut self,
        open_settings: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.permission_request = None;
        self.shell.update(cx, |shell, cx| {
            shell.close_permission_request();
            if open_settings {
                shell.command(ShellCommand::OpenAccessibilitySettings, cx);
            }
        });
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// The sheet, if it is open.
    pub(super) fn permission_sheet(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let kind = self.state.permission_request.clone()?;
        let view = cx.entity();
        let modal = Modal::new("permission-sheet", &self.permission.focus, REQUEST_TITLE)
            .width(theme::PERMISSION_SHEET_WIDTH)
            .on_key(move |key, window, cx| {
                view.update(cx, |view, cx| {
                    view.close_permission_request(key == ModalKey::Return, window, cx)
                })
            });
        let not_now = controls::sheet_button("permission-not-now", "Not now", false, true)
            .on_click(
                cx.listener(|view, _, window, cx| view.close_permission_request(false, window, cx)),
            )
            .into_any_element();
        let open = controls::main_button(
            "permission-open-settings",
            "Open System Settings",
            theme::SPACE_16,
        )
        .on_click(
            cx.listener(|view, _, window, cx| view.close_permission_request(true, window, cx)),
        )
        .into_any_element();
        let content = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .pt(theme::SPACE_20)
                    .px(theme::SPACE_24)
                    .pb(theme::SPACE_20)
                    .flex()
                    .items_start()
                    .gap(theme::SPACE_14)
                    .child(
                        div()
                            .flex_shrink_0()
                            .size(theme::PERMISSION_TILE)
                            .rounded(theme::RADIUS_CARD)
                            .bg(theme::BG_TILE)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(IconName::ShieldAlert)
                                    .size(theme::ICON_PERMISSION)
                                    .text_color(theme::WARNING),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(theme::SPACE_8)
                            .child(
                                div()
                                    .id("permission-title")
                                    .debug_selector(|| "permission-title".into())
                                    .text_size(theme::TEXT_SHEET_TITLE)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(REQUEST_TITLE),
                            )
                            .child(
                                div()
                                    .id("permission-text")
                                    .debug_selector(|| "permission-text".into())
                                    .text_color(theme::TEXT_SECONDARY)
                                    .line_height(relative(theme::LINE_HEIGHT_TEXT))
                                    .child(request_text(&kind)),
                            ),
                    ),
            )
            .child(button_row(None, vec![not_now, open]));
        Some(modal.render(content, window))
    }
}
