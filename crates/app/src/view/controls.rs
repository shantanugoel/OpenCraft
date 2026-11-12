//! Buttons that more than one part of the window uses (decision 0029,
//! sections 3.5 and 3.7).

use super::theme;
use gpui_kit::{
    assets::IconName,
    component::{tooltip::Tooltip, Icon},
    div,
    prelude::*,
    Div, SharedString, Stateful,
};

/// A button with a small padding. A button that is not enabled shows dim
/// text; the caller adds no click handler to it.
pub(super) fn small_button(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let text: SharedString = text.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .h(theme::HEIGHT_SEGMENT)
        .px(theme::SPACE_10)
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::RADIUS_CONTROL)
        .bg(theme::BG_CONTROL)
        .border_1()
        .border_color(theme::BORDER_CONTROL)
        .text_size(theme::TEXT_LABEL)
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .when(enabled, |d| {
            d.cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_CONTROL))
                .active(|d| d.bg(theme::PRESSED))
        })
        .when(!enabled, |d| d.text_color(theme::TEXT_DISABLED))
        .child(label(text))
}

/// A button for a change that removes something.
pub(super) fn danger_button(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
) -> Stateful<Div> {
    button(id, text).text_color(theme::DESTRUCTIVE_TEXT)
}

/// A normal button.
pub(super) fn button(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Stateful<Div> {
    let id: SharedString = id.into();
    let text: SharedString = text.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .h(theme::HEIGHT_CONTROL)
        .px(theme::SPACE_12)
        .flex()
        .items_center()
        .rounded(theme::RADIUS_CONTROL)
        .bg(theme::BG_CONTROL)
        .border_1()
        .border_color(theme::BORDER_CONTROL)
        .text_size(theme::TEXT_LABEL)
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(|d| d.bg(theme::HOVER_CONTROL))
        .active(|d| d.bg(theme::PRESSED))
        .child(label(text))
}

/// The text of a button. A long text ends with "…" when the caller limits
/// the width of the button.
fn label(text: SharedString) -> Div {
    div().min_w_0().truncate().child(text)
}

/// A transparent button with one icon: 30 × 28, icon 17. `label` is the
/// tooltip, the accessible label of the record.
pub(super) fn icon_button(
    id: &'static str,
    icon: IconName,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .w(theme::WIDTH_ICON_BUTTON)
        .h(theme::HEIGHT_CONTROL)
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::RADIUS_CONTROL)
        .text_color(theme::TEXT_SECONDARY)
        .cursor_pointer()
        .hover(|d| d.bg(theme::HOVER_ICON))
        .active(|d| d.bg(theme::PRESSED))
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        .child(Icon::new(icon).size(theme::ICON_PANE))
}

/// A dot of 6 for a state, for example the connection.
pub(super) fn dot(colour: gpui_kit::Rgba) -> Div {
    div()
        .flex_shrink_0()
        .size(theme::DOT)
        .rounded_full()
        .bg(colour)
}

/// A square button with one icon, 28 × 28. `label` is the tooltip. A
/// button that is not enabled shows a dim icon; the caller adds no click
/// handler to it.
pub(super) fn square_button(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    enabled: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .size(theme::HEIGHT_CONTROL)
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::RADIUS_CONTROL)
        .bg(theme::BG_CONTROL)
        .border_1()
        .border_color(theme::BORDER_CONTROL)
        .text_color(if enabled {
            theme::TEXT_SECONDARY
        } else {
            theme::TEXT_DISABLED
        })
        .when(enabled, |d| {
            d.cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_CONTROL))
                .active(|d| d.bg(theme::PRESSED))
        })
        .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
        .child(Icon::new(icon).size(theme::ICON_BUTTON))
}
