//! The parts of an inspector (decision 0029, section 10.2): the header, a
//! section with its header, and the rows of a two-column form.
//!
//! The key, bar panel, knob, page, and device inspectors use these parts,
//! and the body of a gesture card is a form of these rows. The parts are
//! layout only: the caller gives the content and the click handlers.

use super::theme;
use gpui_kit::{
    assets::IconName,
    component::{tooltip::Tooltip, Icon},
    div,
    prelude::*,
    AnyElement, Div, FontWeight, Pixels, SharedString, Stateful, TextRun, Window,
};

/// The header of an inspector: a picture of 64 at the start, the name and
/// the second line (a text, or an element such as a state with a dot), and at the end an optional button, for example the
/// "⋯" button of a control. Padding 16, gap 12.
pub(super) fn header(
    picture: Option<AnyElement>,
    name: impl Into<SharedString>,
    second_line: impl IntoElement,
    end: Option<AnyElement>,
) -> Stateful<Div> {
    div()
        .id("inspector-header")
        .debug_selector(|| "inspector-header".into())
        .flex_shrink_0()
        .p(theme::SPACE_16)
        .flex()
        .items_center()
        .gap(theme::SPACE_12)
        .children(picture)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(theme::SPACE_3)
                .child(
                    div()
                        .truncate()
                        .text_size(theme::TEXT_HEADING)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::TEXT_PRIMARY)
                        .child(name.into()),
                )
                .child(
                    div()
                        .text_size(theme::TEXT_LABEL)
                        .text_color(theme::TEXT_TERTIARY)
                        .child(second_line),
                ),
        )
        .children(end)
}

/// The "⋯" button of the header of a control, 28 × 28. `label` is the
/// tooltip, for example "More for Key 1". A button that is not enabled
/// shows a dim icon; the caller adds no click handler to it.
pub(super) fn more_button(label: impl Into<SharedString>, enabled: bool) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id("more-button")
        .debug_selector(|| "more-button".into())
        .flex_shrink_0()
        .size(theme::HEIGHT_CONTROL)
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::RADIUS_CONTROL)
        .bg(theme::BG_FIELD)
        .border_1()
        .border_color(theme::BORDER_FIELD)
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
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        .child(Icon::new(IconName::Ellipsis).size(theme::ICON_MORE))
}

/// A section of an inspector: a line at the top, the padding of section
/// 3.3, and the header text in capital letters. With a `hint`, the hint is
/// at the end of the header line, and the content has a gap of 8. Add the
/// content with `child`.
pub(super) fn section(id: &'static str, title: &str, hint: Option<&str>) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.into())
        .flex_shrink_0()
        .pt(theme::SECTION_PADDING_TOP)
        .px(theme::SECTION_PADDING_SIDE)
        .pb(theme::SECTION_PADDING_BOTTOM)
        .border_t_1()
        .border_color(theme::LINE_SECTION)
        .flex()
        .flex_col()
        .gap(if hint.is_some() {
            theme::CARD_GAP
        } else {
            theme::SECTION_HEADER_GAP
        })
        .child(
            div()
                .flex()
                .items_baseline()
                .when(hint.is_some(), |d| d.pb(theme::SECTION_HINT_GAP))
                .child(
                    div()
                        .flex_1()
                        .text_size(theme::TEXT_SECTION)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::TEXT_TERTIARY)
                        .child(title.to_uppercase()),
                )
                .children(hint.map(|hint| {
                    div()
                        .text_size(theme::TEXT_CAPTION)
                        .text_color(theme::TEXT_TERTIARY)
                        .child(hint.to_string())
                })),
        )
}

/// A form: its rows, one below the other. `gap` is
/// [`theme::FORM_GAP`], or [`theme::CARD_FORM_GAP`] in a gesture card.
pub(super) fn form(gap: Pixels) -> Div {
    div().flex().flex_col().gap(gap)
}

/// A row of a form: the label in a column of `label_width`, and the value
/// in the rest of the width. The label and the value are centred
/// vertically. `gap` is the gap between the columns, as in [`form`].
pub(super) fn row(
    label: impl Into<SharedString>,
    label_width: Pixels,
    gap: Pixels,
    value: impl IntoElement,
) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(gap)
        .child(label_cell(label, label_width))
        .child(div().flex_1().min_w_0().flex().items_center().child(value))
}

/// A row of a form for a value of more than one line, for example a text
/// field of three lines: the label is at the top, next to the first line of
/// the value.
pub(super) fn top_row(
    label: impl Into<SharedString>,
    label_width: Pixels,
    gap: Pixels,
    value: impl IntoElement,
) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .items_start()
        .gap(gap)
        .child(label_cell(label, label_width).pt(theme::SPACE_6))
        .child(div().flex_1().min_w_0().flex().child(value))
}

/// A row of a form, as [`row`]. If the label is wider than its column, the
/// label is above the value ([`stacked_row`]), so that a long label does
/// not break into short lines.
pub(super) fn fitted_row(
    label: impl Into<SharedString>,
    label_width: Pixels,
    gap: Pixels,
    value: impl IntoElement,
    window: &Window,
) -> Div {
    let label: SharedString = label.into();
    if label_fits(&label, label_width, window) {
        row(label, label_width, gap, value)
    } else {
        stacked_row(label, value)
    }
}

/// True if `label` in the size of a form label fits in `width`.
fn label_fits(label: &SharedString, width: Pixels, window: &Window) -> bool {
    let style = window.text_style();
    let run = TextRun {
        len: label.len(),
        font: style.font(),
        color: style.color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(label.clone(), theme::TEXT_LABEL, &[run], None)
        .width
        <= width
}

/// A row of a form with no label, and the value at the trailing end, for
/// example the button "Test the action" (#188).
pub(super) fn end_row(label_width: Pixels, gap: Pixels, value: impl IntoElement) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(gap)
        .child(div().flex_shrink_0().w(label_width))
        .child(div().flex_1().min_w_0().flex().justify_end().child(value))
}

/// A row with the label above the value, gap 6: for a value that needs the
/// full width, for example a nested card (section 10.5).
pub(super) fn stacked_row(label: impl Into<SharedString>, value: impl IntoElement) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(theme::SPACE_6)
        .child(
            div()
                .text_size(theme::TEXT_LABEL)
                .text_color(theme::TEXT_SECONDARY)
                .child(label.into()),
        )
        .child(value)
}

/// The label of a row: `label`, `text-secondary`. A long label wraps.
fn label_cell(label: impl Into<SharedString>, width: Pixels) -> Div {
    div()
        .flex_shrink_0()
        .w(width)
        .text_size(theme::TEXT_LABEL)
        .text_color(theme::TEXT_SECONDARY)
        .child(label.into())
}

/// A help text below a control or a form: `help`, `text-tertiary`.
pub(super) fn help(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(theme::TEXT_HELP)
        .text_color(theme::TEXT_TERTIARY)
        .child(text.into())
}
