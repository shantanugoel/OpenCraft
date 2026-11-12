//! Controls that more than one part of the window uses (decision 0029,
//! sections 3.5 and 3.7): buttons, a popup button and its menu, a switch,
//! a text field, and a colour sample.

use super::theme;
use gpui_kit::{
    anchored,
    assets::IconName,
    component::{
        input::{AnyInputState, Input, Textarea},
        tooltip::Tooltip,
        Icon, Sizable,
    },
    deferred, div, point,
    prelude::*,
    px, Anchor, AnyElement, App, Div, MouseButton, MouseDownEvent, Pixels, Rgba, SharedString,
    Stateful, Window,
};

/// A button with a small padding. A button that is not enabled shows dim
/// text; the caller adds no click handler to it.
pub(crate) fn small_button(
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

/// A normal button.
pub(crate) fn button(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Stateful<Div> {
    button_if(id, text, true)
}

/// A normal button that can be disabled. A button that is not enabled
/// shows dim text; the caller adds no click handler to it.
pub(super) fn button_if(
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
        .when(enabled, |d| {
            d.cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_CONTROL))
                .active(|d| d.bg(theme::PRESSED))
        })
        .when(!enabled, |d| d.text_color(theme::TEXT_DISABLED))
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

/// A transparent toolbar button with one icon: 30 × 28, icon 16, for
/// example Undo (decision 0029, section 5). `label` is the tooltip. A button
/// that is not enabled shows a dim icon, with no hover and no pointer; the
/// caller adds no click handler to it. The element ID contains the label:
/// a tooltip that shows does not change its text, thus a new label must
/// make a new element.
pub(super) fn toolbar_button(
    id: &'static str,
    icon: IconName,
    label: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(SharedString::from(format!("{id}:{label}")))
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .w(theme::WIDTH_ICON_BUTTON)
        .h(theme::HEIGHT_CONTROL)
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::RADIUS_CONTROL)
        .text_color(if enabled {
            theme::TEXT_SECONDARY
        } else {
            theme::TEXT_DISABLED
        })
        .when(enabled, |d| {
            d.cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_ICON))
                .active(|d| d.bg(theme::PRESSED))
        })
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        .child(Icon::new(icon).size(theme::ICON_TOOLBAR))
}

/// A dot of 6 for a state, for example the connection.
pub(super) fn dot(colour: gpui_kit::Rgba) -> Div {
    div()
        .flex_shrink_0()
        .size(theme::DOT)
        .rounded_full()
        .bg(colour)
}

/// A small square button with one icon (sections 3.5 and 8.1): width 28,
/// `height` [`theme::HEIGHT_CONTROL`] or [`theme::HEIGHT_SEGMENT`] (the page
/// header), radius 6, `bg-field`, border `border-field`, icon 14. `label` is
/// the tooltip. A button that is not enabled shows a dim icon; the caller
/// adds no click handler to it.
pub(super) fn square_button(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    enabled: bool,
    height: Pixels,
) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .w(theme::WIDTH_SQUARE_BUTTON)
        .h(height)
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
        .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
        .child(Icon::new(icon).size(theme::ICON_BUTTON))
}

/// A popup button (section 10.3): `content` at the start, and an up and
/// down chevron at the end. `height` is 30 for the action popup of a
/// gesture card and 28 in a form. While its menu is open, the button has
/// the focus border.
pub(super) fn popup_button(
    id: impl Into<SharedString>,
    height: Pixels,
    open: bool,
    content: impl IntoElement,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_1()
        .min_w_0()
        .h(height)
        .px(theme::SPACE_8)
        .flex()
        .items_center()
        .gap(theme::SPACE_8)
        .rounded(theme::RADIUS_CONTROL)
        .bg(theme::BG_CONTROL)
        .border_1()
        .border_color(if open {
            theme::ACCENT_LINE
        } else {
            theme::BORDER_CONTROL
        })
        .text_size(theme::TEXT_CONTROL)
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(|d| d.bg(theme::HOVER_CONTROL))
        .active(|d| d.bg(theme::PRESSED))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(theme::SPACE_8)
                .child(content),
        )
        .child(
            Icon::new(IconName::ChevronsUpDown)
                .size(theme::ICON_SMALL)
                .text_color(theme::TEXT_SECONDARY),
        )
}

/// A switch (section 3.5): on is `accent` with the knob at the end, off is
/// `bg-segment-on` with the knob at the start. The knob is white. `size` is
/// [`theme::SWITCH_INSPECTOR`] or [`theme::SWITCH_SETTINGS`]. The caller adds
/// the click handler, and keeps the value.
pub(crate) fn switch(
    id: impl Into<SharedString>,
    on: bool,
    size: theme::SwitchSize,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let inset = (size.height - size.knob) / 2.;
    div()
        .id(id.clone())
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .flex_shrink_0()
        .relative()
        .w(size.width)
        .h(size.height)
        .rounded_full()
        .bg(if on {
            theme::ACCENT
        } else {
            theme::BG_SEGMENT_ON
        })
        .cursor_pointer()
        .child(
            div()
                .debug_selector(move || format!("{id}-knob"))
                .absolute()
                .top(inset)
                .when(on, |d| d.right(inset))
                .when(!on, |d| d.left(inset))
                .size(size.knob)
                .rounded_full()
                .bg(theme::SWITCH_KNOB_COLOUR),
        )
}

/// A text field (section 10.2): height 28, radius 6, `body` text, the
/// border `border-field`, and `accent-line` when it has the focus. A
/// multi-line field is at least 60 high. `background` is `bg-field`, or
/// `bg-bar` in a gesture card.
pub(super) fn text_field(
    input: &AnyInputState,
    background: Rgba,
    window: &Window,
    cx: &App,
) -> Div {
    let focused = input.focus_handle(cx).is_focused(window);
    let frame = div()
        .w_full()
        .flex_1()
        .min_w_0()
        .rounded(theme::RADIUS_CONTROL)
        .bg(background)
        .border_1()
        .border_color(if focused {
            theme::ACCENT_LINE
        } else {
            theme::BORDER_FIELD
        })
        .text_size(theme::TEXT_BODY)
        .text_color(theme::TEXT_PRIMARY);
    // The frame draws the background and the border. The field of the kit
    // draws only the text, at the sizes of the window.
    match input {
        AnyInputState::Input(state) => frame.h(theme::HEIGHT_CONTROL).flex().items_center().child(
            Input::new(state)
                .appearance(false)
                .h(theme::HEIGHT_CONTROL - px(2.))
                .px(theme::SPACE_8)
                .py_0()
                .text_size(theme::TEXT_BODY),
        ),
        AnyInputState::Textarea(state) => frame.min_h(theme::HEIGHT_LONG_TEXT).child(
            // The small size of the kit has the horizontal padding 8.
            Textarea::new(state)
                .small()
                .appearance(false)
                .min_h(theme::HEIGHT_LONG_TEXT - px(2.))
                .py(theme::SPACE_4)
                .text_size(theme::TEXT_BODY),
        ),
        _ => frame,
    }
}

/// A fill sample, 22 × 22, radius 5 (section 10.2). `fill` `None` is the
/// sample "No fill". A colour has a light border, so that a dark colour
/// shows on the panel. A selected sample has an outline of 2 in
/// `accent-line`, 2 outside the sample. `label` is the tooltip; a sample
/// with an open popover has no tooltip.
pub(super) fn colour_sample(
    id: impl Into<SharedString>,
    fill: Option<Rgba>,
    selected: bool,
    label: Option<SharedString>,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let outside = theme::SAMPLE_OUTLINE + theme::SAMPLE_OUTLINE_GAP;
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .relative()
        .flex_shrink_0()
        .size(theme::SAMPLE)
        .rounded(theme::RADIUS_SEGMENT)
        .cursor_pointer()
        .map(|d| match fill {
            Some(fill) => d
                .bg(fill)
                .border_1()
                .border_color(theme::DEVICE_SURFACE_BORDER),
            None => d
                .bg(theme::BG_FIELD)
                .border_1()
                .border_color(theme::BORDER_OVERLAY),
        })
        .when_some(label, |d, label| {
            d.tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        })
        .when(selected, |d| {
            d.child(
                div()
                    .absolute()
                    .top(-outside)
                    .left(-outside)
                    .size(theme::SAMPLE + outside * 2.)
                    .rounded(theme::RADIUS_SEGMENT + outside)
                    .border(theme::SAMPLE_OUTLINE)
                    .border_color(theme::ACCENT_LINE),
            )
        })
}

/// Where a menu of [`popup_menu`] shows.
#[derive(Clone, Copy)]
pub(super) struct MenuPlace {
    /// The height of the button of the menu.
    pub(super) button_height: Pixels,
    /// The width of the menu.
    pub(super) width: Pixels,
    /// True: the trailing edge of the menu is on the trailing edge of the
    /// button. False: the leading edges are on one line.
    pub(super) align_end: bool,
}

impl MenuPlace {
    /// A menu of [`theme::MENU_WIDTH`] below a button of `button_height`,
    /// with the leading edges on one line.
    pub(super) fn below(button_height: Pixels) -> Self {
        Self {
            button_height,
            width: theme::MENU_WIDTH,
            align_end: false,
        }
    }
}

/// A menu that the window draws, below its popup button: put it in a
/// `relative` parent that holds the button. If the menu does not fit below
/// the button, it shows above the button. A layer below the menu covers the
/// window: a click on it calls `close` and does nothing else. Make the rows
/// with [`menu_row`] and [`menu_separator`].
pub(super) fn popup_menu(
    id: impl Into<SharedString>,
    place: MenuPlace,
    rows: Vec<AnyElement>,
    window: &Window,
    close: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let id: SharedString = id.into();
    let menu_id = id.clone();
    let viewport = window.viewport_size();
    let (anchor, button_height) = (
        if place.align_end {
            Anchor::TopRight
        } else {
            Anchor::TopLeft
        },
        place.button_height,
    );
    div()
        .absolute()
        .top_0()
        .map(|d| {
            if place.align_end {
                d.right_0()
            } else {
                d.left_0()
            }
        })
        .child(
            deferred(
                anchored().position(point(px(0.), px(0.))).child(
                    div()
                        .id(SharedString::from(format!("{id}-backdrop")))
                        .w(viewport.width)
                        .h(viewport.height)
                        .occlude()
                        .on_mouse_down(MouseButton::Left, close),
                ),
            )
            .with_priority(1),
        )
        .child(
            deferred(
                // The anchor is the top of the button, and the offset puts
                // the menu below it. A menu that does not fit there goes
                // above the anchor: GPUI switches the corner and drops the
                // offset.
                anchored()
                    .anchor(anchor)
                    .offset(point(px(0.), button_height))
                    .child(
                        div().py(theme::MENU_OFFSET).child(
                            div()
                                .id(id)
                                .debug_selector(move || menu_id.to_string())
                                .occlude()
                                .w(place.width)
                                .max_h(theme::MENU_MAX_HEIGHT)
                                .overflow_y_scroll()
                                .flex()
                                .flex_col()
                                .p(theme::SPACE_6)
                                .rounded(theme::RADIUS_POPOVER)
                                .border_1()
                                .border_color(theme::BORDER_OVERLAY)
                                .bg(theme::BG_CARD)
                                .shadow(theme::popover_shadow())
                                // A menu does not take the text style of
                                // its button.
                                .text_size(theme::TEXT_CONTROL)
                                .font_weight(gpui_kit::FontWeight::NORMAL)
                                .text_color(theme::TEXT_PRIMARY)
                                .children(rows),
                        ),
                    ),
            )
            .with_priority(2),
        )
        .into_any_element()
}

/// A row of a menu: height 28, radius 6, a light background under the
/// pointer. The caller adds the content and the click handler.
pub(super) fn menu_row(id: impl Into<SharedString>) -> Stateful<Div> {
    let id: SharedString = id.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .h(theme::HEIGHT_MENU_ROW)
        .px(theme::SPACE_8)
        .flex()
        .items_center()
        .gap(theme::SPACE_8)
        .rounded(theme::RADIUS_CONTROL)
        .cursor_pointer()
        .hover(|d| d.bg(theme::HOVER_ROW))
        .active(|d| d.bg(theme::PRESSED))
}

/// A line between two groups of rows of a menu.
pub(super) fn menu_separator() -> Div {
    div()
        .flex_shrink_0()
        .my(theme::SPACE_4)
        .mx(theme::SPACE_8)
        .h(theme::SPACE_1)
        .bg(theme::LINE_SECTION)
}
