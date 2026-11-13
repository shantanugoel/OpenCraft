//! The group box and its rows (decision 0029, section 17).

use crate::view::theme;
use gpui_kit::{div, prelude::*, Div, IntoElement, SharedString};

/// A group box: radius 10, the card colour, and a border. Between two rows
/// there is a line.
pub(super) fn group(rows: impl IntoIterator<Item = Div>) -> Div {
    let rows: Vec<Div> = rows.into_iter().collect();
    let last = rows.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .rounded(theme::RADIUS_GROUP)
        .bg(theme::BG_CARD)
        .border_1()
        .border_color(theme::LINE_PANE)
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            row.when(index < last, |row| {
                row.border_b_1().border_color(theme::LINE_PANE)
            })
        }))
}

/// A row with the name and a description at the leading side, and a control
/// at the end. The name and the description have one line each, so that the
/// rows of a pane have the same height: a long text, for example a long
/// folder path, ends with "…".
pub(super) fn row(
    name: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(theme::SPACE_12)
        .px(theme::SPACE_14)
        .py(theme::SPACE_10)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(theme::SPACE_2)
                .child(div().truncate().child(name.into()))
                .child(
                    div()
                        .truncate()
                        .text_size(theme::TEXT_HELP)
                        .text_color(theme::TEXT_TERTIARY)
                        .child(description.into()),
                ),
        )
        .child(control)
}

/// A row with one line: a name at the leading side, and a value at the end.
pub(super) fn line_row(name: impl Into<SharedString>, value: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(theme::SPACE_12)
        .px(theme::SPACE_14)
        .py(theme::SPACE_10)
        .child(div().child(name.into()))
        .child(div().text_color(theme::TEXT_SECONDARY).child(value.into()))
}
