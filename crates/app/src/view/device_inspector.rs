//! The inspector of the device (decision 0029, section 10.7): the header,
//! the sections "Display", "Feedback", and "Knob lights", and the note at the
//! bottom.
//!
//! The sections show only the settings that the model has (decision 0022).
//! The state of the controls and the commands are in `device`.

use super::*;
use controls::{segment, segmented, slider, switch};
use gpui_kit::{
    assets::IconName,
    component::{input::AnyInputState, slider::SliderState, Icon},
    relative, FontWeight,
};
use opencraft_engine::{DeviceSettings, KnobLightMode};

/// The note at the bottom of the inspector.
const NOTE: &str = "These settings are for this device. A change of the profile or the page \
                    does not change them. You can change them when no device is connected.";

/// The help text of the haptic switch.
const HAPTIC_HELP: &str = "The device vibrates a little when you operate a control.";

/// The columns of the grid of the knob light buttons.
const KNOB_COLUMNS: usize = 4;

impl DeviceView {
    /// The inspector while the device is the selection (section 10.1).
    pub(super) fn device_inspector(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let connected = self.state.connected;
        let inspector = div()
            .id("device-settings")
            .debug_selector(|| "device-settings".into())
            .child(
                form::header(
                    Some(
                        div()
                            .flex_shrink_0()
                            .size(theme::DEVICE_HEADER_TILE)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(theme::RADIUS_HEADER_TILE)
                            .bg(theme::BG_TILE)
                            .text_color(theme::TEXT_PRIMARY)
                            .child(Icon::new(IconName::Grid3x2).size(theme::ICON_HEADER_DEVICE))
                            .into_any_element(),
                    ),
                    self.device_name(),
                    div()
                        .id("device-header-state")
                        .debug_selector(|| "device-header-state".into())
                        .flex()
                        .items_center()
                        .gap(theme::SPACE_6)
                        .text_color(theme::TEXT_SECONDARY)
                        .child(controls::dot(if connected {
                            theme::SUCCESS
                        } else {
                            theme::TEXT_DISABLED
                        }))
                        .child(if connected {
                            "Connected by USB"
                        } else {
                            "Not connected"
                        }),
                    None,
                )
                .pb(theme::SPACE_12),
            );
        let support = self.state.layout.as_ref().map(|l| l.settings);
        let (Some(support), Some(settings)) = (support, self.shown_settings()) else {
            return inspector.child(
                div()
                    .px(theme::SECTION_PADDING_SIDE)
                    .pt(theme::SPACE_12)
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_TERTIARY)
                    .child("The device settings are not available yet."),
            );
        };
        inspector
            .children(
                self.device
                    .brightness
                    .as_ref()
                    .map(|slider| self.display_section(settings, slider)),
            )
            .children(support.haptic.then(|| self.feedback_section(settings, cx)))
            .children(
                (support.knob_lights > 0).then(|| self.knob_lights_section(settings, window, cx)),
            )
            // The free space pushes the note to the bottom of the panel.
            .child(div().flex_1().min_h_0())
            .child(
                div()
                    .id("device-note")
                    .debug_selector(|| "device-note".into())
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(theme::LINE_SECTION)
                    .pt(theme::SPACE_12)
                    .px(theme::SECTION_PADDING_SIDE)
                    .pb(theme::SPACE_16)
                    .child(form::help(NOTE).line_height(relative(theme::LINE_HEIGHT_TEXT))),
            )
    }

    /// The section "Display": the brightness.
    fn display_section(
        &self,
        settings: &DeviceSettings,
        slider: &Entity<SliderState>,
    ) -> AnyElement {
        form::section("display", "Display", None)
            .child(slider_row(
                "brightness",
                "Brightness",
                settings.brightness,
                slider,
                false,
            ))
            .into_any_element()
    }

    /// The section "Feedback": the haptic switch, and one line of help.
    fn feedback_section(&self, settings: &DeviceSettings, cx: &mut Context<Self>) -> AnyElement {
        let on = settings.haptic;
        form::section("feedback", "Feedback", None)
            .gap(theme::SPACE_10)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(value_label("Haptic feedback"))
                    .child(
                        switch("haptic-switch", on, theme::SWITCH_INSPECTOR).on_click(cx.listener(
                            move |view, _, window, cx| {
                                view.change_settings(|s| s.haptic = !on, window, cx)
                            },
                        )),
                    ),
            )
            .child(form::help(HAPTIC_HELP))
            .into_any_element()
    }

    /// The section "Knob lights": the mode, the colours (in the mode
    /// "Colours"), and the brightness of the lights.
    fn knob_lights_section(
        &self,
        settings: &DeviceSettings,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let lights = &settings.knob_lights;
        let mode = lights.mode;
        let segment_for = |id: &'static str, text: &'static str, value: KnobLightMode| {
            segment(id, text, mode == value, theme::HEIGHT_SEGMENT_INSPECTOR).on_click(cx.listener(
                move |view, _, window, cx| {
                    view.change_settings(|s| s.knob_lights.mode = value, window, cx)
                },
            ))
        };
        let mut section = form::section("knob-lights", "Knob lights", None).child(segmented(
            "knob-mode",
            vec![
                segment_for("knob-effect", "Effect", KnobLightMode::Effect),
                segment_for("knob-colours", "Colours", KnobLightMode::Colors),
                segment_for("knob-off", "Off", KnobLightMode::Off),
            ],
        ));
        if mode == KnobLightMode::Colors && !lights.colors.is_empty() {
            section =
                section.child(
                    div()
                        .id("knob-colour-grid")
                        .debug_selector(|| "knob-colour-grid".into())
                        .grid()
                        .grid_cols(KNOB_COLUMNS as u16)
                        .gap(theme::SPACE_8)
                        .children(lights.colors.iter().enumerate().map(|(index, colour)| {
                            self.knob_light_button(index, *colour, window, cx)
                        })),
                );
        }
        section
            .children(self.device.knob_brightness.as_ref().map(|slider| {
                slider_row(
                    "knob-brightness",
                    "Light brightness",
                    lights.brightness,
                    slider,
                    mode == KnobLightMode::Off,
                )
            }))
            .into_any_element()
    }

    /// The button of one knob light: a ring in its colour, its name, and its
    /// hex value. A click opens the popover of the colour.
    fn knob_light_button(
        &self,
        index: usize,
        colour: Color,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let number = index + 1;
        let open = self.device.colour_open == Some(index);
        let hex = colour.to_string();
        let id = format!("knob-{number}-colour");
        let tooltip = format!("Knob {number} light colour {hex}");
        div()
            .relative()
            .child(
                div()
                    .id(SharedString::from(id.clone()))
                    .debug_selector({
                        let id = id.clone();
                        move || id.clone()
                    })
                    .h(theme::HEIGHT_KNOB_LIGHT_BUTTON)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(theme::SPACE_6)
                    .rounded(theme::RADIUS_CARD)
                    .bg(theme::BG_CARD)
                    .border_1()
                    .border_color(if open {
                        theme::ACCENT_LINE
                    } else {
                        theme::BORDER_FIELD
                    })
                    .cursor_pointer()
                    .hover(|d| d.bg(theme::HOVER_CONTROL))
                    // A button with an open popover has no tooltip: it
                    // would cover the popover.
                    .when(!open, |d| {
                        d.tooltip(move |window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(tooltip.clone())
                                .build(window, cx)
                        })
                    })
                    .child(
                        div()
                            .size(theme::KNOB_LIGHT_RING)
                            .rounded_full()
                            .bg(theme::DEVICE_KNOB)
                            .border(theme::KNOB_LIGHT_RING_WIDTH)
                            .border_color(theme::colour(colour)),
                    )
                    .child(
                        div()
                            .text_size(theme::TEXT_CAPTION)
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme::TEXT_PRIMARY)
                            .child(format!("Knob {number}")),
                    )
                    .child(
                        div()
                            .debug_selector({
                                let id = id.clone();
                                move || format!("{id}-hex")
                            })
                            .text_size(theme::TEXT_BADGE)
                            .font_weight(FontWeight::NORMAL)
                            .text_color(theme::TEXT_TERTIARY)
                            .child(hex),
                    )
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.toggle_knob_colour(index, window, cx)
                    })),
            )
            .children(open.then(|| {
                self.knob_colour_popover(
                    index,
                    colour,
                    index % KNOB_COLUMNS >= KNOB_COLUMNS / 2,
                    window,
                    cx,
                )
            }))
            .into_any_element()
    }

    /// The popover of a knob light colour: the samples of [`KNOB_COLOURS`]
    /// and a field for `#RRGGBB`.
    fn knob_colour_popover(
        &self,
        index: usize,
        current: Color,
        align_end: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        controls::popup_menu(
            "knob-colour-popover",
            controls::MenuPlace {
                button_height: theme::HEIGHT_KNOB_LIGHT_BUTTON,
                width: theme::WIDTH_KNOB_POPOVER,
                align_end,
            },
            vec![div()
                .capture_key_down(cx.listener(
                    |view, event: &gpui_kit::KeyDownEvent, window, cx| {
                        // Escape in the field closes the popover with no change.
                        if event.keystroke.key == "escape" {
                            cx.stop_propagation();
                            view.close_knob_colour(window, cx);
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
                        .child(format!("Knob {} light colour", index + 1)),
                )
                .child(
                    div().flex().justify_between().children(
                        device::KNOB_COLOURS
                            .into_iter()
                            .enumerate()
                            .map(|(n, value)| {
                                controls::colour_sample(
                                    format!("knob-{}-colour-{n}", index + 1),
                                    Some(theme::sample(value)),
                                    current == colour(value),
                                    Some(format!("#{value:06X}").into()),
                                )
                                .on_click(cx.listener(
                                    move |view, _, window, cx| {
                                        view.set_knob_colour(index, colour(value), window, cx);
                                        view.show_knob_colour_text(colour(value), window, cx);
                                    },
                                ))
                            }),
                    ),
                )
                .child(
                    controls::text_field(
                        &AnyInputState::Input(self.device.colour_input.clone()),
                        theme::BG_FIELD,
                        window,
                        cx,
                    )
                    .id("knob-colour-field")
                    .debug_selector(|| "knob-colour-field".into()),
                )
                .child(form::help(controls::HEX_HELP))
                .into_any_element()],
            window,
            cx.listener(|view, _, window, cx| view.close_knob_colour(window, cx)),
        )
    }

    /// Show `colour` in the field of the knob colour popover.
    fn show_knob_colour_text(
        &mut self,
        colour: Color,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.device.colour_input.update(cx, |input, cx| {
            input.set_value(colour.to_string(), window, cx)
        });
    }
}

/// The name of a setting: `control`, `text-primary`.
fn value_label(text: &'static str) -> Div {
    div()
        .text_size(theme::TEXT_CONTROL)
        .text_color(theme::TEXT_PRIMARY)
        .child(text)
}

/// A line with the name of a setting and its value in percent at the end,
/// and the slider below it (gap 8). A disabled slider keeps its value.
fn slider_row(
    id: &'static str,
    text: &'static str,
    value: u8,
    state: &Entity<SliderState>,
    disabled: bool,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(theme::SPACE_8)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(value_label(text).when(disabled, |d| d.text_color(theme::TEXT_DISABLED)))
                .child(
                    div()
                        .id(id)
                        .debug_selector(move || format!("{id}-value"))
                        .text_size(theme::TEXT_CONTROL)
                        .text_color(if disabled {
                            theme::TEXT_DISABLED
                        } else {
                            theme::TEXT_SECONDARY
                        })
                        .child(format!("{value} %")),
                ),
        )
        // The thumb is wider than the track, and it is centred on the end
        // of the track. The inset keeps the thumb inside the section.
        .child(
            div().px(theme::SPACE_8).child(
                div()
                    .id(SharedString::from(format!("{id}-slider")))
                    .debug_selector(move || format!("{id}-slider"))
                    .child(slider(state, text, disabled)),
            ),
        )
        .into_any_element()
}
