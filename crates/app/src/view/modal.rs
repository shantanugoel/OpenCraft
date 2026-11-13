//! The modal layer of the main window, and the parts of a sheet (decision
//! 0029, sections 15, 16, and 18).
//!
//! A modal layer covers the window with [`theme::MODAL_DIM`] and shows one
//! card above it: a sheet, or the command palette. The parts:
//!
//! - **The dim layer.** It covers the full window and takes each click, so
//!   that no element below it gets the click. A sheet stays open on a click
//!   out of it, as a sheet of macOS does. The command palette closes then:
//!   see [`Modal::close_on_click_out`].
//! - **The focus.** The card has its own focus handle and the key context
//!   [`MODAL`]. The layer is not in the element of the key context
//!   `"Workspace"`, thus the key commands and the menu items of the window
//!   do not operate while the layer is open, as with the dialog of the kit
//!   (`docs/kb/gpui.md`, "Popover and modal layer"). The owner focuses the
//!   card when the layer opens, and gives the focus back to the window when
//!   it closes.
//! - **Escape and Return.** The card gets the two keys before a text field
//!   in it does, and calls the key function of the owner.
//! - **The button row.** [`button_row`] is the footer of a sheet: a button
//!   at the start, for example "Remove the profile…", and the buttons at the
//!   end, with the main button last.
//!
//! The owner keeps the state: if the layer is open, and what it shows. Put
//! the result of [`Modal::render`] after the other content of the root
//! element of the window, outside the element of the key context
//! `"Workspace"`.

use super::theme;
use gpui_kit::{
    div, prelude::*, AnyElement, App, Div, FocusHandle, KeyDownEvent, MouseButton, Pixels, Rgba,
    Role, SharedString, Window,
};
use std::rc::Rc;

/// The key context of the card of a modal layer.
pub(crate) const MODAL: &str = "Modal";

/// A key that the card of a modal layer handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModalKey {
    /// Escape: close, or cancel.
    Escape,
    /// Return: the main button.
    Return,
}

type KeyHandler = Rc<dyn Fn(ModalKey, &mut Window, &mut App)>;
type ClickOutHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// A modal layer with one card. See the module documentation.
pub(crate) struct Modal {
    id: SharedString,
    focus: FocusHandle,
    label: SharedString,
    width: Pixels,
    top: Pixels,
    background: Rgba,
    on_key: Option<KeyHandler>,
    on_click_out: Option<ClickOutHandler>,
}

impl Modal {
    /// A layer with the card `id`. `focus` is the focus handle of the card,
    /// and `label` is its accessible label, for example "Profile settings
    /// for Work". The card is a sheet: 600 wide, [`theme::SHEET_TOP`] below
    /// the top of the window, with `bg-card`.
    pub(crate) fn new(
        id: impl Into<SharedString>,
        focus: &FocusHandle,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            focus: focus.clone(),
            label: label.into(),
            width: theme::PROFILE_SHEET_WIDTH,
            top: theme::SHEET_TOP,
            background: theme::BG_CARD,
            on_key: None,
            on_click_out: None,
        }
    }

    /// The width of the card.
    pub(crate) fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// The space from the top of the window to the card, for example 120
    /// for the command palette.
    pub(crate) fn top(mut self, top: Pixels) -> Self {
        self.top = top;
        self
    }

    /// The background of the card, for example `bg-field` for the command
    /// palette.
    pub(crate) fn background(mut self, background: Rgba) -> Self {
        self.background = background;
        self
    }

    /// The function for Escape and Return in the card.
    pub(crate) fn on_key(mut self, f: impl Fn(ModalKey, &mut Window, &mut App) + 'static) -> Self {
        self.on_key = Some(Rc::new(f));
        self
    }

    /// Close the layer on a click out of the card, as the command palette
    /// does. A sheet does not use this.
    pub(crate) fn close_on_click_out(
        mut self,
        f: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click_out = Some(Rc::new(f));
        self
    }

    /// The layer with `content` in the card.
    pub(crate) fn render(self, content: impl IntoElement, window: &Window) -> AnyElement {
        let id = self.id.clone();
        let layer_id = SharedString::from(format!("{}-layer", self.id));
        let layer_selector = layer_id.clone();
        let max_height = window.viewport_size().height - self.top - theme::SHEET_BOTTOM;
        let on_key = self.on_key.clone();
        let on_click_out = self.on_click_out.clone();
        div()
            .id(layer_id)
            .debug_selector(move || layer_selector.to_string())
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .bg(theme::MODAL_DIM)
            .flex()
            .flex_col()
            .items_center()
            .pt(self.top)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                if let Some(f) = &on_click_out {
                    f(window, cx);
                }
            })
            .child(
                div()
                    .id(self.id.clone())
                    .debug_selector(move || id.to_string())
                    .role(Role::Dialog)
                    .aria_label(self.label)
                    .key_context(MODAL)
                    .track_focus(&self.focus)
                    .capture_key_down(move |event: &KeyDownEvent, window, cx| {
                        let key = match event.keystroke.key.as_str() {
                            "escape" => ModalKey::Escape,
                            "enter" => ModalKey::Return,
                            _ => return,
                        };
                        let modifiers = event.keystroke.modifiers;
                        if modifiers.platform || modifiers.control || modifiers.alt {
                            return;
                        }
                        if let Some(f) = &on_key {
                            cx.stop_propagation();
                            f(key, window, cx);
                        }
                    })
                    // A click in the card does not get to the dim layer.
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .w(self.width)
                    .max_h(max_height)
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(theme::RADIUS_SHEET)
                    .bg(self.background)
                    .border_1()
                    .border_color(theme::BORDER_OVERLAY)
                    .shadow(theme::sheet_shadow())
                    .text_size(theme::TEXT_BODY)
                    .text_color(theme::TEXT_PRIMARY)
                    .cursor_default()
                    .child(content),
            )
            .into_any_element()
    }
}

/// The title of a sheet (section 16): padding 20 24 14, the title in
/// `sheet-title`, and below it, 4 lower, the subtitle in `label`,
/// `text-secondary`.
pub(crate) fn sheet_title(
    title: impl Into<SharedString>,
    subtitle: impl Into<SharedString>,
) -> Div {
    let title: SharedString = title.into();
    let subtitle: SharedString = subtitle.into();
    div()
        .flex_shrink_0()
        .pt(theme::SPACE_20)
        .px(theme::SPACE_24)
        .pb(theme::SPACE_14)
        .flex()
        .flex_col()
        .gap(theme::SPACE_4)
        .child(
            div()
                .text_size(theme::TEXT_SHEET_TITLE)
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .debug_selector(|| "sheet-subtitle".into())
                .text_size(theme::TEXT_LABEL)
                .text_color(theme::TEXT_SECONDARY)
                .child(subtitle),
        )
}

/// A row of the form of a sheet: the label in a column of
/// [`theme::SHEET_LABEL`], 14 to the control.
pub(crate) fn sheet_row(label: impl Into<SharedString>, control: impl IntoElement) -> Div {
    let label: SharedString = label.into();
    div()
        .flex()
        .items_center()
        .gap(theme::SPACE_14)
        .child(
            div()
                .flex_shrink_0()
                .w(theme::SHEET_LABEL)
                .text_size(theme::TEXT_CONTROL)
                .text_color(theme::TEXT_SECONDARY)
                .child(label),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .child(control),
        )
}

/// The header of a section of a sheet, for example "SHARE": `section`,
/// `text-tertiary`, in capital letters.
pub(crate) fn sheet_section_header(title: impl Into<SharedString>) -> Div {
    super::controls::section_title(title)
}

/// The button row at the bottom of a sheet (section 16): padding 14 24, a
/// line `line-pane` at the top, `bg-sidebar`. `start` is at the leading
/// side; `end` is at the trailing side, gap 8, with the main button last.
pub(crate) fn button_row(start: Option<AnyElement>, end: Vec<AnyElement>) -> Div {
    div()
        .flex_shrink_0()
        .py(theme::SPACE_14)
        .px(theme::SPACE_24)
        .flex()
        .items_center()
        .gap(theme::SPACE_8)
        .border_t_1()
        .border_color(theme::LINE_PANE)
        .bg(theme::BG_SIDEBAR)
        .children(start)
        .child(div().flex_1())
        .children(end)
}
