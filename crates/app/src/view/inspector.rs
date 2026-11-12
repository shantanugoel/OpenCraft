//! The "Inspector" tab of the right panel (decision 0029, section 10): the
//! inspector of the selected control, or the device settings.
//!
//! The inspector of a key face or a bar panel has the header, the
//! "Appearance" section, and the "Gestures" section with one gesture card
//! for each gesture (`actions`). A knob and the touch bar have the header
//! and the "Gestures" section. The parts are in `form` and `gesture_card`.

use super::*;
use gpui_kit::{component::input::AnyInputState, rgb, Rgba};
use opencraft_engine::{ControlLayout, DeviceLayout};

/// The state of the inspector that is not in the form of the cards.
pub(super) struct InspectorState {
    /// The popover of the fill sample "Other colour" is open.
    pub(super) colour_open: bool,
    /// The field for `#RRGGBB` in that popover.
    colour_input: Entity<InputState>,
    _colour_subscription: Subscription,
}

impl InspectorState {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let colour_input = cx.new(|cx| InputState::new(window, cx).placeholder("#RRGGBB"));
        let subscription =
            cx.subscribe_in(&colour_input, window, |view, input, event, window, cx| {
                if matches!(event, TextEvent::PressEnter { .. }) {
                    if let Some(colour) = parse_colour(&input.read(cx).value()) {
                        view.set_colour(Some(colour));
                        view.inspector.colour_open = false;
                        view.focus_workspace(window, cx);
                        cx.notify();
                    }
                }
            });
        Self {
            colour_open: false,
            colour_input,
            _colour_subscription: subscription,
        }
    }
}

/// A colour from the text `#RRGGBB` or `RRGGBB`.
fn parse_colour(text: &str) -> Option<Color> {
    let digits = text.trim().trim_start_matches('#');
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok().map(colour)
}

/// The text `#RRGGBB` of a colour.
fn colour_text(colour: Color) -> String {
    let [r, g, b] = colour.0;
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// The colour of a fill sample.
fn sample_fill(colour: Color) -> Rgba {
    let [r, g, b] = colour.0;
    rgb((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b))
}

/// "first" to "fifth" for 1 to 5 (decision 0029, section 10.1).
fn ordinal(number: u8) -> Option<&'static str> {
    ["first", "second", "third", "fourth", "fifth"]
        .get(usize::from(number).checked_sub(1)?)
        .copied()
}

/// The second line of the header of a control: its position, or the bar
/// panel of the feedback of a knob (section 10.1).
fn position_text(control: &ControlLayout, layout: &DeviceLayout) -> String {
    match control.id.kind {
        ControlKind::Key => {
            let row = match control.row {
                1 => "Top row".to_string(),
                n => ordinal(n).map_or_else(
                    || format!("Row {n}"),
                    |o| {
                        let mut o = o.to_string();
                        o[..1].make_ascii_uppercase();
                        format!("{o} row")
                    },
                ),
            };
            let key = ordinal(control.column)
                .map_or_else(|| format!("key {}", control.column), |o| format!("{o} key"));
            format!("{row}, {key}")
        }
        ControlKind::BarPanel => ordinal(control.column).map_or_else(
            || format!("Touch bar, panel {}", control.column),
            |o| format!("Touch bar, {o} panel"),
        ),
        ControlKind::Knob => control
            .feedback_surface
            .and_then(|surface| {
                layout
                    .controls
                    .iter()
                    .find(|c| c.id.kind == ControlKind::BarPanel && c.surface == Some(surface))
            })
            .map_or_else(
                || "No bar panel".to_string(),
                |panel| format!("Feedback on {}", control_name(panel.id).to_lowercase()),
            ),
        ControlKind::TouchBar => "Swipes on this page".to_string(),
    }
}

/// The hint of the "Gestures" section.
fn gestures_hint(kind: ControlKind) -> &'static str {
    match kind {
        ControlKind::Key => "What the key does",
        ControlKind::BarPanel => "What the bar panel does",
        ControlKind::Knob => "What the knob does",
        ControlKind::TouchBar => "What the touch bar does",
    }
}

impl DeviceView {
    /// The content of the tab. It scrolls, and a scrollbar shows when it is
    /// taller than the panel. Thus the user sees that more settings are
    /// below.
    pub(super) fn inspector(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let content = if self.device.open {
            self.device_editor(cx)
                .gap(theme::SPACE_16)
                .pt(theme::SECTION_PADDING_TOP)
                .px(theme::SECTION_PADDING_SIDE)
                .pb(theme::SECTION_PADDING_BOTTOM)
        } else {
            self.control_inspector(window, cx)
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
                    .flex_col(),
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

    /// The inspector of the selected control (section 10.1).
    fn control_inspector(&self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let editor = div().id("editor").debug_selector(|| "editor".into());
        // Until #181, the inspector shows nothing for the page.
        if self.state.page_selected {
            return editor;
        }
        let control = self.state.selected_control.and_then(|id| {
            let layout = self.state.layout.as_ref()?;
            layout.controls.iter().find(|c| c.id == id).cloned()
        });
        let (Some(control), Some(layout)) = (control, self.state.layout.as_ref()) else {
            return editor.child(
                div()
                    .pt(theme::EMPTY_STATE_TOP)
                    .px(theme::SECTION_PADDING_SIDE)
                    .text_center()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_TERTIARY)
                    .child("Select a control, the page, or the device."),
            );
        };
        let name = control_name(control.id);
        editor
            .child(form::header(
                self.header_picture(&control, layout),
                name.clone(),
                position_text(&control, layout),
                (control.id.kind != ControlKind::TouchBar).then(|| {
                    form::more_button(format!("More for {name}"), false).into_any_element()
                }),
            ))
            .children(
                self.state
                    .selected
                    .map(|surface| self.appearance(surface, window, cx)),
            )
            .child(
                form::section("gestures", "Gestures", Some(gestures_hint(control.id.kind)))
                    .children(self.gesture_cards(control.id, window, cx)),
            )
    }

    /// The picture of the header: the image of the surface from the engine,
    /// or a knob with the ring of the canvas.
    fn header_picture(&self, control: &ControlLayout, layout: &DeviceLayout) -> Option<AnyElement> {
        if control.id.kind == ControlKind::Knob {
            return Some(
                div()
                    .flex_shrink_0()
                    .size(theme::HEADER_PICTURE)
                    .rounded_full()
                    .bg(theme::DEVICE_KNOB)
                    .border(theme::HEADER_KNOB_RING)
                    // The same ring colour as the knob on the canvas.
                    .border_color(super::canvas::knob_ring(
                        self.state.device_settings.as_ref(),
                        control.id,
                    ))
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .mt(theme::SPACE_4)
                            .size(theme::HEADER_KNOB_MARK)
                            .rounded_full()
                            .bg(theme::DEVICE_KNOB_DOT),
                    )
                    .into_any_element(),
            );
        }
        let surface = layout.surface(control.surface?)?;
        let width = theme::HEADER_PICTURE;
        let (height, radius) = if surface.id.kind == SurfaceKind::KeyFace {
            (width, theme::RADIUS_POPOVER)
        } else {
            (
                width * (f32::from(surface.height) / f32::from(surface.width.max(1))),
                theme::RADIUS_CONTROL,
            )
        };
        Some(
            div()
                .flex_shrink_0()
                .w(width)
                .h(height)
                .rounded(radius)
                .overflow_hidden()
                .bg(theme::DEVICE_DISPLAY)
                .border_1()
                .border_color(theme::DEVICE_SURFACE_BORDER)
                .children(
                    self.images
                        .get(&surface.id)
                        .cloned()
                        .map(|image| img(image).size_full().rounded(radius)),
                )
                .into_any_element(),
        )
    }

    /// The "Appearance" section of a key face or a bar panel (section
    /// 10.2). #189 adds the label options, and #190 changes "Image" to
    /// "Icon".
    fn appearance(&self, id: SurfaceId, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let content = self.state.surfaces.get(&id).map(|s| &s.content);
        let has_image = content.is_some_and(|c| c.image.is_some());
        let row = |label: &str, value: AnyElement| {
            form::row(label.to_string(), theme::FORM_LABEL, theme::FORM_GAP, value)
        };
        form::section("appearance", "Appearance", None)
            .child(
                form::form(theme::FORM_GAP)
                    .child(row(
                        "Image",
                        div()
                            .flex()
                            .gap(theme::GAP_ITEMS)
                            .child(button("choose-image", "Image file…").on_click(
                                cx.listener(|view, _, window, cx| view.dialog(window, cx)),
                            ))
                            .child(button_if("remove-image", "Remove", has_image).when(
                                has_image,
                                |d| {
                                    d.on_click(cx.listener(move |view, _, _, _| {
                                        (view.send)(Command::SetImage {
                                            surface: id,
                                            path: None,
                                        })
                                    }))
                                },
                            ))
                            .into_any_element(),
                    ))
                    .child(row(
                        "Title",
                        controls::text_field(
                            &AnyInputState::Input(self.input.clone()),
                            theme::BG_FIELD,
                            window,
                            cx,
                        )
                        .id("title-field")
                        .debug_selector(|| "title-field".into())
                        .into_any_element(),
                    ))
                    .child(row("Fill", self.fill_samples(id, window, cx))),
            )
            .into_any_element()
    }

    /// The fill samples: "No fill", the colours of [`COLOURS`], and "Other
    /// colour", which opens a popover with a field for `#RRGGBB`.
    fn fill_samples(&self, id: SurfaceId, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let current = self.state.surfaces.get(&id).and_then(|s| s.content.color);
        let custom = current.filter(|c| !COLOURS.iter().any(|&v| colour(v) == *c));
        let other = controls::colour_sample(
            "other-colour",
            custom.map(sample_fill),
            custom.is_some(),
            (!self.inspector.colour_open).then(|| "Other colour".into()),
        )
        .when(custom.is_none(), |d| {
            d.flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_dashed()
                .border_color(theme::BORDER_SAMPLE_OTHER)
                .text_size(theme::TEXT_BODY)
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .text_color(theme::TEXT_SECONDARY)
                .child("+")
        })
        .on_click(cx.listener(move |view, _, window, cx| {
            let open = !view.inspector.colour_open;
            view.close_kinds();
            if open {
                view.inspector.colour_open = true;
                let text = custom.map(colour_text).unwrap_or_default();
                let input = view.inspector.colour_input.clone();
                input.update(cx, |input, cx| {
                    input.set_value(text, window, cx);
                    input.focus(window, cx);
                });
            } else {
                view.focus_workspace(window, cx);
            }
            cx.notify();
        }))
        .children(self.inspector.colour_open.then(|| {
            controls::popup_menu(
                "other-colour-popover",
                controls::MenuPlace {
                    button_height: theme::SAMPLE,
                    width: theme::WIDTH_COLOUR_POPOVER,
                    align_end: true,
                },
                vec![div()
                    .capture_key_down(cx.listener(
                        |view, event: &gpui_kit::KeyDownEvent, window, cx| {
                            // Escape in the field closes the popover with no change.
                            if event.keystroke.key == "escape" {
                                cx.stop_propagation();
                                view.close_kinds();
                                view.focus_workspace(window, cx);
                                cx.notify();
                            }
                        },
                    ))
                    .p(theme::SPACE_6)
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_8)
                    .child(
                        div()
                            .text_size(theme::TEXT_LABEL)
                            .text_color(theme::TEXT_SECONDARY)
                            .child("Other colour"),
                    )
                    .child(
                        controls::text_field(
                            &AnyInputState::Input(self.inspector.colour_input.clone()),
                            theme::BG_FIELD,
                            window,
                            cx,
                        )
                        .id("other-colour-field")
                        .debug_selector(|| "other-colour-field".into()),
                    )
                    .child(form::help("Type #RRGGBB and push Return."))
                    .into_any_element()],
                window,
                cx.listener(|view, _, window, cx| {
                    view.close_kinds();
                    view.focus_workspace(window, cx);
                    cx.notify();
                }),
            )
        }));
        div()
            .flex()
            .flex_wrap()
            .gap(theme::SPACE_6)
            .child(
                controls::colour_sample(
                    "no-colour",
                    None,
                    current.is_none(),
                    Some("No fill".into()),
                )
                .on_click(cx.listener(|view, _, _, cx| {
                    view.set_colour(None);
                    cx.notify();
                })),
            )
            .children(COLOURS.into_iter().enumerate().map(|(index, value)| {
                controls::colour_sample(
                    format!("colour-{index}"),
                    Some(rgb(value)),
                    current == Some(colour(value)),
                    Some(format!("Fill #{value:06X}").into()),
                )
                .on_click(cx.listener(move |view, _, _, _| view.set_colour(Some(colour(value)))))
            }))
            .child(other)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_text_has_six_hex_digits() {
        assert_eq!(parse_colour("#1A6FD8"), Some(colour(0x1a6fd8)));
        assert_eq!(parse_colour(" 1a6fd8 "), Some(colour(0x1a6fd8)));
        for text in ["", "#12345", "#1234567", "#GGGGGG", "+12345"] {
            assert_eq!(parse_colour(text), None, "{text}");
        }
        assert_eq!(colour_text(colour(0x0a0b0c)), "#0A0B0C");
    }

    #[test]
    fn the_position_text_follows_the_layout() {
        let Event::Layout(layout) = crate::state::tests::layout() else {
            panic!()
        };
        let text = |kind| {
            let control = layout.controls.iter().find(|c| c.id.kind == kind).unwrap();
            position_text(control, &layout)
        };
        assert_eq!(text(ControlKind::Key), "Top row, first key");
        assert_eq!(text(ControlKind::BarPanel), "Touch bar, first panel");
        assert_eq!(text(ControlKind::Knob), "Feedback on bar panel 2");
        assert_eq!(ordinal(6), None);
        assert_eq!(ordinal(0), None);
    }
}
