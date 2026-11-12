//! The "Inspector" tab of the right panel (decision 0029, section 10): the
//! editor of the selected control, or the device settings.
//!
//! Until #179, the content is the editor of Phase 6: the action of the
//! selected gesture, the label, the colour, and the image.

use super::*;
use gpui_kit::FontWeight;

/// The name of the selection, at the top of the inspector.
fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(theme::TEXT_HEADING)
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}

/// A help text below a name or a control.
pub(super) fn help(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(theme::TEXT_HELP)
        .text_color(theme::TEXT_TERTIARY)
        .child(text.into())
}

/// The label above a group of controls.
pub(super) fn label(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(theme::TEXT_LABEL)
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme::TEXT_SECONDARY)
        .child(text.into())
}

/// A label and its controls, one below the other.
fn group() -> Div {
    div().flex().flex_col().gap(theme::SPACE_8)
}

impl DeviceView {
    /// The content of the tab. It scrolls, and a scrollbar shows when it is
    /// taller than the panel. Thus the user sees that more settings are
    /// below.
    pub(super) fn inspector(&self, cx: &mut Context<Self>) -> AnyElement {
        let content = if self.device.open {
            self.device_editor(cx)
        } else {
            self.surface_editor(cx)
        };
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                content
                    .size_full()
                    .track_scroll(&self.inspector_scroll)
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_16)
                    .pt(theme::SECTION_PADDING_TOP)
                    .px(theme::SECTION_PADDING_SIDE)
                    .pb(theme::SECTION_PADDING_BOTTOM),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(Scrollbar::vertical(&self.inspector_scroll).mode(ScrollbarMode::Always)),
            )
            .into_any_element()
    }

    fn surface_editor(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut editor = div().id("editor").debug_selector(|| "editor".into());
        if let Some(control) = self.state.selected_control {
            // A control with no surface has an action only.
            if self.state.selected.is_none() {
                return editor
                    .child(heading(control_name(control)))
                    .when(control.kind == ControlKind::TouchBar, |d| {
                        d.child(help(
                            "A swipe has no position. It is the same on all bar panels.",
                        ))
                    })
                    .child(self.action_editor(cx));
            }
        }
        let Some(id) = self.state.selected else {
            return editor.child(help(
                "Click a key, a knob, a bar panel, or the touch bar in the device picture.",
            ));
        };
        let content = self.state.surfaces.get(&id).map(|s| &s.content);
        let current_colour = content.and_then(|c| c.color);
        let has_image = content.is_some_and(|c| c.image.is_some());
        editor = editor
            .child(heading(surface_name(id)))
            .child(self.action_editor(cx))
            .child(
                group().child(label("Label")).child(
                    div()
                        .text_color(theme::TEXT_PRIMARY)
                        .child(Input::new(&self.input)),
                ),
            )
            .child(
                group()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(label("Colour"))
                            .child(
                                small_button(
                                    "no-colour",
                                    if current_colour.is_none() {
                                        "No colour ✓"
                                    } else {
                                        "No colour"
                                    },
                                    true,
                                )
                                .on_click(cx.listener(|view, _, _, _| view.set_colour(None))),
                            ),
                    )
                    .child(div().flex().flex_wrap().gap(theme::SPACE_6).children(
                        COLOURS.into_iter().enumerate().map(|(index, value)| {
                            div()
                                .id(format!("colour-{index}"))
                                .debug_selector(move || format!("colour-{index}"))
                                .size(theme::SAMPLE)
                                .rounded(theme::RADIUS_SEGMENT)
                                .bg(gpui_kit::rgb(value))
                                .border_2()
                                .border_color(if current_colour == Some(colour(value)) {
                                    theme::ACCENT_LINE
                                } else {
                                    theme::BORDER_FIELD
                                })
                                .cursor_pointer()
                                .on_click(cx.listener(move |view, _, _, _| {
                                    view.set_colour(Some(colour(value)))
                                }))
                        }),
                    )),
            )
            .child(
                group()
                    .child(label("Image"))
                    .child(help(format!(
                        "This {} has {} image.",
                        if id.kind == SurfaceKind::KeyFace {
                            "key face"
                        } else {
                            "bar panel"
                        },
                        if has_image { "an" } else { "no" },
                    )))
                    .child(
                        div()
                            .flex()
                            .gap(theme::GAP_ITEMS)
                            .child(button("choose-image", "Select image…").on_click(
                                cx.listener(|view, _, window, cx| view.dialog(window, cx)),
                            ))
                            .when(has_image, |d| {
                                d.child(button("remove-image", "Remove image").on_click(
                                    cx.listener(move |view, _, _, _| {
                                        (view.send)(Command::SetImage {
                                            surface: id,
                                            path: None,
                                        })
                                    }),
                                ))
                            }),
                    ),
            );
        editor
    }
}
