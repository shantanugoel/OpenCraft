//! Controls that more than one part of the window uses: buttons, a popup
//! button and its menu, a switch, a segmented control, a slider, a text
//! field, and a colour sample.
//!
//! Read `docs/DESIGN.md` before you change the window. Its section 3 says
//! when to use each control. Use a control of this module before you make a
//! new one, and put a new shared control here, not a copy at a call site.
//!
//! Each control that the user can operate calls [`keyboard_focus`]: Tab
//! reaches it, Space and Return operate it, and it shows the focus ring. A
//! new control gets the same with one call.

use super::{
    focus_return::FocusReturn,
    menu::{MenuKeys, MenuRow},
    theme,
};
use gpui_kit::{
    actions, anchored,
    assets::IconName,
    base::FocusTrapElement,
    component::{
        input::{AnyInputState, Input, InputState, Textarea},
        slider::{Slider, SliderState},
        tooltip::Tooltip,
        Icon, Sizable,
    },
    deferred, div, point,
    prelude::*,
    px, Anchor, AnyElement, App, Div, Entity, FocusHandle, MouseButton, MouseDownEvent, Pixels,
    Rgba, Role, SharedString, Stateful, Toggled, Window,
};

/// The accessible role and label of an element (`docs/DESIGN.md`, section
/// 3.4). VoiceOver reads only an element that has an ID and a role: plain
/// text and an element with no role are not in the accessibility tree.
/// The label is the visible name or the tooltip of the element, not a second
/// text. Each shared control of this module sets it.
pub(crate) trait Accessible: StatefulInteractiveElement + Sized {
    /// Give the element the role `role` and the accessible label `label`.
    fn accessible(self, role: Role, label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        #[cfg(test)]
        accessibility_log::record(role, &label);
        self.role(role).aria_label(label)
    }

    /// Make the element a text that VoiceOver reads, for example the page
    /// name of the toolbar. GPUI gives a text element the role `Label` and
    /// the text as its value; this does the same for a `div`.
    fn readable_text(self, text: impl Into<SharedString>) -> Self {
        let text: SharedString = text.into();
        #[cfg(test)]
        accessibility_log::record(Role::Label, &text);
        self.role(Role::Label).aria_value(text)
    }

    /// The state of a control that is on or off: a switch, a toggle button,
    /// a segment, a chip, or a colour sample.
    fn checked(self, on: bool) -> Self {
        self.aria_toggled(if on { Toggled::True } else { Toggled::False })
    }
}

impl<E: StatefulInteractiveElement> Accessible for E {}

/// The roles and labels that the window made, for the window tests. The
/// test platform of GPUI does not make an accessibility tree, thus
/// [`Accessible`] writes each role and label here when it makes an element.
#[cfg(test)]
pub(crate) mod accessibility_log {
    use gpui_kit::Role;
    use std::cell::RefCell;

    thread_local! {
        static NODES: RefCell<Vec<(Role, String)>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record(role: Role, label: &str) {
        NODES.with(|nodes| nodes.borrow_mut().push((role, label.to_string())));
    }

    /// The roles and labels since the last call, in the order of the
    /// render.
    pub(crate) fn take() -> Vec<(Role, String)> {
        NODES.with(|nodes| std::mem::take(&mut *nodes.borrow_mut()))
    }
}

actions!(
    controls,
    [
        /// A click on a control gives the focus to the owner of the control:
        /// the root element of its window, or the card of a sheet. The owner
        /// handles this action. In a popover, the focus stays where it is
        /// ([`popover_focus`]).
        FocusOwner
    ]
);

/// Where the focus ring of a control shows (see [`keyboard_focus`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ring {
    /// Outside the edge: a control with a fill, for example a button.
    Outside,
    /// Inside the edge: a control with no fill, for example an icon button,
    /// a row, or a card.
    Inside,
}

/// Make `control` a control that the keyboard operates (`docs/DESIGN.md`,
/// section 5):
///
/// - Tab and Shift-Tab move the focus to it, in the order of the window.
/// - Space and Return operate it: GPUI sends a keyboard click to its click
///   handler. In a sheet, Return does the main action (`modal`).
/// - A click does not give it the focus. The focus goes to the owner of the
///   control ([`FocusOwner`]), as before the control had a focus: the key
///   commands of the window and the hotkey recorder keep the focus that
///   they need. A control does not take the focus after a click, as on
///   macOS.
/// - It shows the focus ring ([`theme::FOCUS_RING`] in `accent-line`) while
///   it has the focus. Only the keyboard gives a control the focus, thus
///   the ring shows only for the keyboard focus. It stays when the pointer
///   moves (GPUI `focus_visible` would remove it at each move of the
///   pointer).
///
/// Call it only for an enabled control: a disabled control is not in the
/// order. The element must have a stable ID, because GPUI keeps the focus
/// handle in the state of the element.
pub(crate) fn keyboard_focus(control: Stateful<Div>, ring: Ring) -> Stateful<Div> {
    focus_look(control.tab_index(0), ring)
}

/// A [`keyboard_focus`] with a focus handle of the owner, for a group in
/// which the owner moves the focus, for example the cards of the first
/// start window: the arrow keys move the selection and the focus. Set
/// `tab_stop` on the handle for the one item of the group that Tab reaches.
pub(crate) fn keyboard_focus_tracked(
    control: Stateful<Div>,
    focus: &FocusHandle,
    ring: Ring,
) -> Stateful<Div> {
    focus_look(control.track_focus(focus), ring)
}

/// The focus ring and the click of [`keyboard_focus`].
fn focus_look(control: Stateful<Div>, ring: Ring) -> Stateful<Div> {
    #[cfg(test)]
    let control = {
        let mut control = control;
        let id = control.interactivity().element_id.clone();
        control.on_action(move |_: &tests::WhoHasFocus, _, _| {
            tests::FOCUSED.with(|focused| *focused.borrow_mut() = id.clone());
        })
    };
    control
        .focus(move |style| match ring {
            Ring::Outside => style.shadow(theme::focus_ring_outside()),
            Ring::Inside => style
                .shadow(theme::focus_ring_inside())
                .border_color(theme::ACCENT_LINE),
        })
        // This listener runs before the listener that GPUI adds for the
        // focus of the element, and stops it and the one of each parent.
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            // A control or a field in this control had the click first.
            if window.default_prevented() {
                return;
            }
            window.prevent_default();
            window.dispatch_action(Box::new(FocusOwner), cx);
        })
}

/// A button with a small padding. A button that is not enabled shows dim
/// text; the caller adds no click handler to it.
pub(crate) fn small_button(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    let text: SharedString = text.into();
    small_button_frame(id, enabled)
        .accessible(Role::Button, text.clone())
        .child(label(text))
}

/// A small button with an icon before its text, for example "Test the
/// action": the look of [`small_button`], the icon 11, gap 6.
pub(super) fn small_icon_button(
    id: impl Into<SharedString>,
    icon: IconName,
    text: impl Into<SharedString>,
) -> Stateful<Div> {
    let text: SharedString = text.into();
    small_button_frame(id, true)
        .accessible(Role::Button, text.clone())
        .gap(theme::SPACE_6)
        .child(Icon::new(icon).size(theme::ICON_PLAY))
        .child(label(text))
}

/// The frame of a small button, with no content.
fn small_button_frame(id: impl Into<SharedString>, enabled: bool) -> Stateful<Div> {
    let id: SharedString = id.into();
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
            keyboard_focus(d, Ring::Outside)
                .cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_CONTROL))
                .active(|d| d.bg(theme::PRESSED))
        })
        .when(!enabled, |d| d.text_color(theme::TEXT_DISABLED))
}

/// A normal button.
pub(crate) fn button(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Stateful<Div> {
    button_if(id, text, true)
}

/// A normal button that can be disabled. A button that is not enabled
/// shows dim text; the caller adds no click handler to it.
pub(crate) fn button_if(
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
            keyboard_focus(d, Ring::Outside)
                .cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_CONTROL))
                .active(|d| d.bg(theme::PRESSED))
        })
        .when(!enabled, |d| d.text_color(theme::TEXT_DISABLED))
        .accessible(Role::Button, text.clone())
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
    keyboard_focus(div().id(id), Ring::Inside)
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
        .accessible(Role::Button, label.clone())
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        .child(Icon::new(icon).size(theme::ICON_PANE))
}

/// A transparent toolbar button with one icon: 30 × 28, icon 16, for
/// example Undo (decision 0029, section 5). `label` is the tooltip. A button
/// that is not enabled shows a dim icon, with no hover and no pointer; the
/// caller adds no click handler to it. The element of the tooltip has an ID
/// that contains the label: a tooltip that shows does not change its text,
/// thus a new label must make a new element. The button itself keeps its
/// ID, so that it keeps the keyboard focus when the label changes.
pub(super) fn toolbar_button(
    id: &'static str,
    icon: IconName,
    label: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .when(enabled, |d| keyboard_focus(d, Ring::Inside))
        .flex_shrink_0()
        .w(theme::WIDTH_ICON_BUTTON)
        .h(theme::HEIGHT_CONTROL)
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
        .accessible(Role::Button, label.clone())
        .child(
            div()
                .id(SharedString::from(format!("{id}:{label}")))
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
                .child(Icon::new(icon).size(theme::ICON_TOOLBAR)),
        )
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
    label: impl Into<SharedString>,
    enabled: bool,
    height: Pixels,
) -> Stateful<Div> {
    square_icon_button(id, icon, theme::ICON_BUTTON, label, enabled, height)
}

/// A [`square_button`] with the icon size `icon_size`, for example the "⋯"
/// button of the inspector header (icon 16).
pub(super) fn square_icon_button(
    id: &'static str,
    icon: IconName,
    icon_size: Pixels,
    label: impl Into<SharedString>,
    enabled: bool,
    height: Pixels,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .accessible(Role::Button, label.clone())
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
            keyboard_focus(d, Ring::Outside)
                .cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_CONTROL))
                .active(|d| d.bg(theme::PRESSED))
        })
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        .child(Icon::new(icon).size(icon_size))
}

/// A popup button (section 10.3): `content` at the start, and an up and
/// down chevron at the end. `height` is 30 for the action popup of a
/// gesture card and 28 in a form. While its menu is open, the button has
/// the focus border. `label` is the name of the choice (the label of its
/// row) and `value` is the text of the current choice, which `content`
/// shows: VoiceOver reads both.
pub(super) fn popup_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    height: Pixels,
    open: bool,
    content: impl IntoElement,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    keyboard_focus(div().id(id.clone()), Ring::Outside)
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
        .accessible(Role::ComboBox, label)
        .aria_value(value)
        .aria_expanded(open)
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
/// [`theme::SWITCH_INSPECTOR`] or [`theme::SWITCH_SETTINGS`]. `label` is the
/// name of its row, the accessible label. The caller adds the click handler,
/// and keeps the value.
pub(crate) fn switch(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    on: bool,
    size: theme::SwitchSize,
) -> Stateful<Div> {
    switch_if(id, label, on, true, size)
}

/// A [`switch`] that can be disabled. A disabled switch has no focus, and
/// the caller adds no click handler. The default profile uses this: the
/// switch is on, and the user cannot turn it off (decision 0035).
pub(crate) fn switch_if(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    on: bool,
    enabled: bool,
    size: theme::SwitchSize,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let label: SharedString = label.into();
    let inset = (size.height - size.knob) / 2.;
    let control = div()
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
        .accessible(Role::Switch, label)
        .checked(on);
    let control = if enabled {
        keyboard_focus(control, Ring::Outside).cursor_pointer()
    } else {
        control
    };
    control.child(
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

/// A segmented control (section 3.7): a track of `bg-field` with the
/// `segments` in one row. Make a segment with [`segment`] and add its click
/// handler.
pub(crate) fn segmented(
    id: impl Into<SharedString>,
    segments: Vec<Stateful<Div>>,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .flex()
        .p(theme::SPACE_2)
        .rounded(theme::RADIUS_TRACK)
        .bg(theme::BG_FIELD)
        .role(Role::RadioGroup)
        .children(segments)
}

/// A segment of a [`segmented`] control: it shares the free width with the
/// other segments. The selected segment is `bg-segment-on` with
/// `text-on-accent`; the other segments are transparent with
/// `text-secondary`. `height` is [`theme::HEIGHT_SEGMENT`] or
/// [`theme::HEIGHT_SEGMENT_INSPECTOR`].
pub(crate) fn segment(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
    selected: bool,
    height: Pixels,
) -> Stateful<Div> {
    segment_if(id, text, selected, true, height)
}

/// A [`segment`] that can be disabled: a disabled segment has
/// `text-disabled`, no hover, and the caller adds no click handler to it.
pub(crate) fn segment_if(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
    selected: bool,
    enabled: bool,
    height: Pixels,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let text: SharedString = text.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_1()
        .h(height)
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::RADIUS_SEGMENT)
        .text_size(theme::TEXT_LABEL)
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .when(selected, |d| {
            d.bg(theme::BG_SEGMENT_ON).text_color(theme::TEXT_ON_ACCENT)
        })
        .when(enabled, |d| keyboard_focus(d, Ring::Inside))
        .when(!selected && enabled, |d| {
            d.text_color(theme::TEXT_SECONDARY)
                .cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_ROW))
        })
        .when(!selected && !enabled, |d| {
            d.text_color(theme::TEXT_DISABLED)
        })
        .accessible(Role::RadioButton, text.clone())
        .checked(selected)
        .child(text)
}

/// A slider (section 10.7): the filled part of the track is `accent`, and
/// the thumb is white. A disabled slider has no thumb, and the filled part
/// is `bg-segment-on`. `label` is the accessible label. An enabled slider
/// takes the keyboard focus ([`keyboard_focus`]): Left and Down move the
/// thumb one step down, Right and Up one step up.
pub(super) fn slider(
    state: &Entity<SliderState>,
    label: &'static str,
    disabled: bool,
) -> Stateful<Div> {
    let slider = Slider::new(state)
        .accessibility_label(label)
        .disabled(disabled)
        .bg(if disabled {
            theme::BG_SEGMENT_ON
        } else {
            theme::ACCENT
        })
        .text_color(theme::SWITCH_KNOB_COLOUR);
    let keys = state.clone();
    div()
        .id(("slider-keys", state.entity_id()))
        // The ring needs space around the track and the thumb. The negative
        // margin keeps the slider at its place.
        .px(theme::SLIDER_FOCUS_PADDING_X)
        .mx(-theme::SLIDER_FOCUS_PADDING_X)
        .py(theme::SLIDER_FOCUS_PADDING_Y)
        .my(-theme::SLIDER_FOCUS_PADDING_Y)
        .rounded(theme::RADIUS_CONTROL)
        .when(!disabled, |d| {
            keyboard_focus(d, Ring::Inside).on_key_down(move |event, window, cx| {
                let keystroke = &event.keystroke;
                // macOS marks an arrow key with the function modifier.
                let modifiers = keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.shift || modifiers.platform {
                    return;
                }
                let direction = match keystroke.key.as_str() {
                    "left" | "down" => -1.,
                    "right" | "up" => 1.,
                    _ => return,
                };
                cx.stop_propagation();
                keys.update(cx, |state, cx| {
                    let step = state.accessibility_step();
                    state.step_thumb(false, direction * step, window, cx);
                });
            })
        })
        .child(slider)
}

/// A text field (section 10.2): height 28, radius 6, `body` text, the
/// border `border-field`, and `accent-line` when it has the focus. A
/// multi-line field is at least 60 high. `background` is `bg-field`, or
/// `bg-bar` in a gesture card. `label` is the label of its row, the
/// accessible label.
pub(super) fn text_field(
    input: &AnyInputState,
    label: impl Into<SharedString>,
    background: Rgba,
    window: &Window,
    cx: &App,
) -> Div {
    text_field_sized(input, label, background, FieldSize::default(), window, cx)
}

/// The height, the corner radius, and the side padding of a one-line
/// [`text_field_sized`].
#[derive(Clone, Copy)]
pub(super) struct FieldSize {
    pub(super) height: Pixels,
    pub(super) radius: Pixels,
    pub(super) padding: Pixels,
}

impl Default for FieldSize {
    /// A field of the inspector: height 28, radius 6, padding 8.
    fn default() -> Self {
        Self {
            height: theme::HEIGHT_CONTROL,
            radius: theme::RADIUS_CONTROL,
            padding: theme::SPACE_8,
        }
    }
}

impl FieldSize {
    /// A field of a sheet (section 16): height 30, radius 7, padding 10.
    pub(super) fn sheet() -> Self {
        Self {
            height: theme::HEIGHT_SHEET_CONTROL,
            radius: theme::RADIUS_TRACK,
            padding: theme::SPACE_10,
        }
    }
}

/// A [`text_field`] of the size `size`. A multi-line field keeps the size of
/// [`text_field`].
pub(super) fn text_field_sized(
    input: &AnyInputState,
    label: impl Into<SharedString>,
    background: Rgba,
    size: FieldSize,
    window: &Window,
    cx: &App,
) -> Div {
    let focused = input.focus_handle(cx).is_focused(window);
    let label: SharedString = label.into();
    #[cfg(test)]
    accessibility_log::record(Role::TextInput, &label);
    let frame = div()
        .w_full()
        .flex_1()
        .min_w_0()
        .rounded(size.radius)
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
        AnyInputState::Input(state) => frame.h(size.height).flex().items_center().child(
            Input::new(state)
                .aria_label(label)
                .appearance(false)
                .h(size.height - theme::BORDER * 2.)
                .px(size.padding)
                .py_0()
                .text_size(theme::TEXT_BODY),
        ),
        AnyInputState::Textarea(state) => frame.min_h(theme::HEIGHT_LONG_TEXT).child(
            // The small size of the kit has the horizontal padding 8.
            Textarea::new(state)
                .aria_label(label)
                .small()
                .appearance(false)
                .min_h(theme::HEIGHT_LONG_TEXT - theme::BORDER * 2.)
                .py(theme::SPACE_4)
                .text_size(theme::TEXT_BODY),
        ),
        _ => frame,
    }
}

/// A fill sample, 22 × 22, radius 5 (section 10.2). `fill` `None` is the
/// sample "No fill". A colour has a light border, so that a dark colour
/// shows on the panel. A selected sample has an outline of 2 in
/// `accent-line`, 2 outside the sample. `label` is the tooltip and the
/// accessible label. A sample with an open popover has no tooltip
/// (`tooltip` false).
pub(super) fn colour_sample(
    id: impl Into<SharedString>,
    fill: Option<Rgba>,
    selected: bool,
    label: impl Into<SharedString>,
    tooltip: bool,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let label: SharedString = label.into();
    let outside = theme::SAMPLE_OUTLINE + theme::SAMPLE_OUTLINE_GAP;
    keyboard_focus(div().id(id.clone()), Ring::Outside)
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
        .accessible(Role::RadioButton, label.clone())
        .checked(selected)
        .when(tooltip, |d| {
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

/// The help text below a field for a colour value, in each colour popover.
pub(super) const HEX_HELP: &str = "Type #RRGGBB and push Return.";

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
/// the button, it shows above the button. If it fits at neither place, it
/// moves up until it fits in the window, and covers the button, as a menu
/// of macOS does. A layer below the menu covers the
/// window: a click on it calls `close` and does nothing else. Make the rows
/// with [`menu_row`] and [`menu_separator`]. The menu has the keys of
/// `view::menu` (arrows, Home, End, Return, and letters) for its
/// [`menu_row`] items: the call site does nothing for them. Escape is the
/// work of the owner of the menu.
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
    let keys_id = id.clone();
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
                    .child(MenuKeys::new(
                        SharedString::from(format!("{keys_id}-keys")),
                        move |scroll, focus| {
                            div()
                                .py(theme::MENU_OFFSET)
                                .child(
                                    div()
                                        .id(id.clone())
                                        .debug_selector(move || menu_id.to_string())
                                        .role(Role::Menu)
                                        .occlude()
                                        .w(place.width)
                                        .max_h(theme::MENU_MAX_HEIGHT.min(
                                            viewport.height
                                                - (theme::MENU_OFFSET + theme::MENU_WINDOW_MARGIN)
                                                    * 2.,
                                        ))
                                        .overflow_y_scroll()
                                        .track_scroll(scroll)
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
                                        .children(rows)
                                        .map(|menu| {
                                            popover_focus(
                                                menu,
                                                SharedString::from(format!("{id}-trap")),
                                                focus,
                                            )
                                        }),
                                )
                                .into_any_element()
                        },
                    )),
            )
            .with_priority(2),
        )
        .into_any_element()
}

/// The focus of a popover: a drawn menu, a colour popover, the icon
/// picker, or the application picker.
///
/// - The popover is a focus trap: with the focus in it, Tab and Shift-Tab
///   move the focus between its controls only.
/// - A click in the popover does not move the focus: the field of a
///   popover keeps it while the user clicks a sample, and the popover does
///   not take it.
/// - When the popover closes, the focus goes back to the control that
///   opened it, if a key opened it (`focus_return`).
pub(crate) fn popover_focus(
    popover: Stateful<Div>,
    id: SharedString,
    focus: &FocusHandle,
) -> impl IntoElement + use<> {
    let trap = popover
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .on_action(|_: &FocusOwner, _, _| {})
        .focus_trap(id.clone(), focus);
    FocusReturn::new(format!("{id}-return"), focus, trap)
}

/// A row of a menu: height 28, radius 6. The menu paints the highlight
/// under the row that a key or the pointer selected
/// ([`MenuRow`]). The caller adds the content and the click handler. The
/// text of the row is its accessible label ([`MenuRow::child`]).
pub(super) fn menu_row(id: impl Into<SharedString>) -> MenuRow {
    let id: SharedString = id.into();
    MenuRow::new(
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
            .active(|d| d.bg(theme::PRESSED)),
    )
}

/// The text of a section header or of a group title (section 3.1): `section`
/// weight 600, `text-tertiary`, in capital letters, for example "PROFILES".
/// Each section header and group title of the window uses it; the caller
/// adds the padding of its place.
pub(crate) fn section_title(title: impl Into<SharedString>) -> Div {
    let title: SharedString = title.into();
    div()
        .flex_shrink_0()
        .text_size(theme::TEXT_SECTION)
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .text_color(theme::TEXT_TERTIARY)
        .child(title.to_uppercase())
}

/// The title of a group of rows, in the library and in a menu (decision
/// 0029, section 11): a [`section_title`] with padding 4 6.
pub(super) fn group_title(title: impl Into<SharedString>) -> Div {
    section_title(title).py(theme::SPACE_4).px(theme::SPACE_6)
}

/// A search field of a panel (decision 0029, section 11): `height`, padding
/// 0 8, radius 7, `bg-field`, the border `border-field` and `accent-line`
/// with the focus, a search icon 14 in `text-tertiary`, and the text in
/// `control`. The placeholder is the placeholder of `input`, and `label` is
/// the accessible label, for example "Search actions".
pub(super) fn search_field(
    id: &'static str,
    label: impl Into<SharedString>,
    input: &gpui_kit::Entity<InputState>,
    height: Pixels,
    window: &Window,
    cx: &App,
) -> Stateful<Div> {
    let focused = gpui_kit::Focusable::focus_handle(input.read(cx), cx).is_focused(window);
    let label: SharedString = label.into();
    #[cfg(test)]
    accessibility_log::record(Role::SearchInput, &label);
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .w_full()
        .h(height)
        .px(theme::SPACE_8)
        .flex()
        .items_center()
        .gap(theme::SPACE_8)
        .rounded(theme::RADIUS_TRACK)
        .bg(theme::BG_FIELD)
        .border_1()
        .border_color(if focused {
            theme::ACCENT_LINE
        } else {
            theme::BORDER_FIELD
        })
        .text_size(theme::TEXT_CONTROL)
        .text_color(theme::TEXT_PRIMARY)
        .child(
            Icon::new(IconName::Search)
                .size(theme::ICON_SEARCH)
                .text_color(theme::TEXT_TERTIARY),
        )
        .child(
            Input::new(input)
                .aria_label(label)
                .appearance(false)
                .flex_1()
                .h(height - theme::BORDER * 2.)
                .px_0()
                .py_0()
                .text_size(theme::TEXT_CONTROL),
        )
}

/// A square toggle button of 28 × 28 with a short text, for example "B" for
/// bold (section 10.2). On: `bg-segment-on`, border `#5A5C63`,
/// `text-on-accent`. Off: the look of a button, with `text-secondary`.
/// `tooltip` is the accessible label. The caller adds the click handler.
pub(super) fn toggle_button(
    id: impl Into<SharedString>,
    content: impl IntoElement,
    on: bool,
    tooltip: impl Into<SharedString>,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let tooltip: SharedString = tooltip.into();
    keyboard_focus(div().id(id.clone()), Ring::Outside)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .size(theme::WIDTH_SQUARE_BUTTON)
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::RADIUS_CONTROL)
        .border_1()
        .cursor_pointer()
        .map(|d| {
            if on {
                d.bg(theme::BG_SEGMENT_ON)
                    .border_color(theme::BORDER_TOGGLE_ON)
                    .text_color(theme::TEXT_ON_ACCENT)
            } else {
                d.bg(theme::BG_CONTROL)
                    .border_color(theme::BORDER_CONTROL)
                    .text_color(theme::TEXT_SECONDARY)
                    .hover(|d| d.bg(theme::HOVER_CONTROL))
            }
        })
        .active(|d| d.bg(theme::PRESSED))
        .accessible(Role::Button, tooltip.clone())
        .checked(on)
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .child(content)
}

/// A square colour button of 28 × 28 that shows `colour` (section 10.2).
/// While its popover is open, the button has the focus border, and no
/// tooltip. The caller adds the click handler.
pub(super) fn colour_button(
    id: impl Into<SharedString>,
    colour: Rgba,
    open: bool,
    tooltip: impl Into<SharedString>,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let tooltip: SharedString = tooltip.into();
    keyboard_focus(div().id(id.clone()), Ring::Outside)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .size(theme::WIDTH_SQUARE_BUTTON)
        .rounded(theme::RADIUS_CONTROL)
        .bg(colour)
        .border_1()
        .border_color(if open {
            theme::ACCENT_LINE
        } else {
            theme::BORDER_CONTROL
        })
        .cursor_pointer()
        .accessible(Role::Button, tooltip.clone())
        .aria_expanded(open)
        .when(!open, |d| {
            d.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        })
}

/// A filter chip (section 3.7): height 24, radius 12, padding 0 10, `label`
/// weight 500. Selected: border `accent-line`, `accent-tint` 25 %,
/// `text-on-accent`. Else: border `border-field`, transparent,
/// `text-secondary`, and a light background under the pointer. The caller
/// adds the click handler. The tabs of the icon picker use it (section 14).
pub(super) fn chip(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
    selected: bool,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let text: SharedString = text.into();
    keyboard_focus(div().id(id.clone()), Ring::Inside)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .h(theme::HEIGHT_CHIP)
        .px(theme::SPACE_10)
        .flex()
        .items_center()
        .rounded(theme::RADIUS_CHIP)
        .border_1()
        .text_size(theme::TEXT_LABEL)
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .map(|d| {
            if selected {
                d.border_color(theme::ACCENT_LINE)
                    .bg(theme::ACCENT_TINT_CHIP)
                    .text_color(theme::TEXT_ON_ACCENT)
            } else {
                d.border_color(theme::BORDER_FIELD)
                    .text_color(theme::TEXT_SECONDARY)
                    .cursor_pointer()
                    .hover(|d| d.bg(theme::HOVER_ROW))
            }
        })
        .accessible(Role::RadioButton, text.clone())
        .checked(selected)
        .child(text)
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

/// The box of a key hint, for example "⌘K" in the search field of the
/// toolbar (sections 3.4 and 5): `caption`, padding 1 5, border 1
/// `border-overlay`, radius 4. The text colour is the colour of the parent.
pub(super) fn key_hint(keys: impl Into<SharedString>) -> Div {
    let keys: SharedString = keys.into();
    div()
        .flex_shrink_0()
        .py(theme::SPACE_1)
        .px(theme::KEY_HINT_PADDING_X)
        .rounded(theme::RADIUS_BADGE)
        .border_1()
        .border_color(theme::BORDER_OVERLAY)
        .text_size(theme::TEXT_CAPTION)
        .child(keys)
}

/// A link (section 3.7): `label`, `accent-text`, no underline, and
/// `accent-text-hover` under the pointer. A link does the work at once, for
/// example "Set left and right apart" in the "Turn" card of a knob.
pub(crate) fn link(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Stateful<Div> {
    let id: SharedString = id.into();
    let text: SharedString = text.into();
    keyboard_focus(div().id(id.clone()), Ring::Inside)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        // The ring needs space around the text. The negative margin keeps
        // the text at its place.
        .px(theme::LINK_FOCUS_PADDING)
        .mx(-theme::LINK_FOCUS_PADDING)
        .py(theme::LINK_FOCUS_PADDING)
        .my(-theme::LINK_FOCUS_PADDING)
        .rounded(theme::RADIUS_BADGE)
        .text_size(theme::TEXT_LABEL)
        .text_color(theme::ACCENT_TEXT)
        .cursor_pointer()
        .hover(|d| d.text_color(theme::ACCENT_TEXT_HOVER))
        .accessible(Role::Link, text.clone())
        .child(text)
}

/// A colour circle of 20 (sections 14 and 16). `fill` `None` is the circle
/// "None": transparent with a grey border. A selected circle has an outline
/// of 2 in `accent-line`, 2 outside the circle. `label` is the tooltip. The
/// caller adds the click handler.
pub(super) fn colour_circle(
    id: impl Into<SharedString>,
    fill: Option<Rgba>,
    selected: bool,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let label: SharedString = label.into();
    let outside = theme::SAMPLE_OUTLINE + theme::SAMPLE_OUTLINE_GAP;
    // The ring is inside: a ring of a shadow outside keeps the corner radius
    // of the circle, and looks square (`docs/kb/gpui.md`).
    keyboard_focus(div().id(id.clone()), Ring::Inside)
        .debug_selector(move || id.to_string())
        .relative()
        .flex_shrink_0()
        .size(theme::COLOUR_CIRCLE)
        .rounded_full()
        .cursor_pointer()
        .map(|d| match fill {
            Some(fill) => d.bg(fill),
            None => d.border_1().border_color(theme::BORDER_NO_COLOUR),
        })
        .accessible(Role::RadioButton, label.clone())
        .checked(selected)
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        .when(selected, |d| {
            d.child(
                div()
                    .absolute()
                    .top(-outside)
                    .left(-outside)
                    .size(theme::COLOUR_CIRCLE + outside * 2.)
                    .rounded_full()
                    .border(theme::SAMPLE_OUTLINE)
                    .border_color(theme::ACCENT_LINE),
            )
        })
}

/// The main button of a sheet or a window (section 3.7): height 30, radius
/// 7, `accent`, no border, `text-on-accent`, `body` weight 600. `padding` is
/// the padding at the sides, for example 18 for "Done". The caller adds the
/// click handler.
pub(crate) fn main_button(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
    padding: Pixels,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let text: SharedString = text.into();
    keyboard_focus(div().id(id.clone()), Ring::Outside)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .h(theme::HEIGHT_SHEET_CONTROL)
        .px(padding)
        .flex()
        .items_center()
        .rounded(theme::RADIUS_TRACK)
        .bg(theme::ACCENT)
        .text_color(theme::TEXT_ON_ACCENT)
        .text_size(theme::TEXT_BODY)
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(|d| d.opacity(theme::MAIN_HOVER_OPACITY))
        .active(|d| d.opacity(theme::MAIN_PRESSED_OPACITY))
        .accessible(Role::Button, text.clone())
        .child(label(text))
}

/// A transparent button with a border of the button row of a sheet: height
/// 30, padding 0 12, radius 7, `control` weight 500. A destructive button
/// has `destructive-text`, for example "Remove the profile…". A button that
/// is not enabled shows dim text; the caller adds no click handler to it.
pub(crate) fn sheet_button(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
    destructive: bool,
    enabled: bool,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let text: SharedString = text.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .h(theme::HEIGHT_SHEET_CONTROL)
        .px(theme::SPACE_12)
        .flex()
        .items_center()
        .rounded(theme::RADIUS_TRACK)
        .border_1()
        .border_color(theme::BORDER_CONTROL)
        .text_size(theme::TEXT_CONTROL)
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .text_color(match (enabled, destructive) {
            (false, _) => theme::TEXT_DISABLED,
            (true, true) => theme::DESTRUCTIVE_TEXT,
            (true, false) => theme::TEXT_PRIMARY,
        })
        .when(enabled, |d| {
            keyboard_focus(d, Ring::Inside)
                .cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_ICON))
                .active(|d| d.bg(theme::PRESSED))
        })
        .accessible(Role::Button, text.clone())
        .child(label(text))
}

/// A window test asks which control has the keyboard focus: GPUI does not
/// show the focus handle that it keeps for an element.
#[cfg(test)]
pub(crate) mod tests {
    use gpui_kit::{actions, ElementId, VisualTestContext};
    use std::cell::RefCell;

    actions!(controls_test, [WhoHasFocus]);

    thread_local! {
        /// The ID of the control that answered [`WhoHasFocus`] last.
        pub(super) static FOCUSED: RefCell<Option<ElementId>> = const { RefCell::new(None) };
    }

    /// The ID of the control with the keyboard focus, or `None` when no
    /// control of [`super::keyboard_focus`] has it (for example the root
    /// element or a text field).
    pub(crate) fn focused(cx: &mut VisualTestContext) -> Option<String> {
        FOCUSED.with(|focused| focused.borrow_mut().take());
        cx.dispatch_action(WhoHasFocus);
        cx.run_until_parked();
        FOCUSED
            .with(|focused| focused.borrow_mut().take())
            .map(|id| id.to_string())
    }
}
