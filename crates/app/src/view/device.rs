//! The "Device settings" area of the window: the display brightness, the
//! haptic feedback, and the knob lights (decision 0022).
//!
//! The area shows only the settings that the layout gives for the model. It
//! changes the settings only with [`Command::SetDeviceSettings`], with all
//! settings. The engine keeps and applies them, also with no device and no
//! window.

use super::*;
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::Focusable;
use opencraft_engine::{DeviceSettings, KnobLightMode, SettingsSupport};
use std::time::Duration;

/// The shortest time between two settings commands while the user moves a
/// control. One change is 1 to 2 reports of 1025 bytes, and a report takes
/// approximately 0.4 ms to write. A full repaint of 14 images takes 22 to
/// 50 ms (`docs/kb/streamdock-protocol.md`, "Repaint time"). Thus 20
/// changes each second are much less than the device takes. The last value
/// always goes to the engine.
pub(super) const SETTINGS_INTERVAL: Duration = Duration::from_millis(50);

/// The highest value of the two brightness controls, in percent.
const MAX_PERCENT: u8 = 100;

/// The lowest value of the knob light brightness control, in percent.
const LOWEST_KNOB_PERCENT: u8 = 1;

/// The colours for a knob light. A knob light shows a pure colour best.
pub(super) const KNOB_COLOURS: [u32; 9] = [
    0xffffff, 0xff0000, 0xff8000, 0xffff00, 0x00ff00, 0x00ffff, 0x0000ff, 0xff00ff, 0x000000,
];

/// The controls of the area, and the throttle of the commands.
pub(super) struct DevicePanel {
    /// True while the window shows the area in place of the surface editor.
    pub open: bool,
    pub(super) brightness: Option<Entity<SliderState>>,
    knob_brightness: Option<Entity<SliderState>>,
    /// One text field for each knob light, with the colour as `#RRGGBB`.
    pub(super) colours: Vec<Entity<InputState>>,
    /// The settings support that the controls are for.
    support: Option<SettingsSupport>,
    /// The last settings that the user chose. They show until the throttle
    /// time ends, so that an older reply of the engine does not move a
    /// control back.
    chosen: Option<DeviceSettings>,
    /// Settings that wait for the end of the throttle time.
    pending: Option<DeviceSettings>,
    throttle: Option<Task<()>>,
    subscriptions: Vec<Subscription>,
}

impl DevicePanel {
    pub fn new() -> Self {
        Self {
            open: false,
            brightness: None,
            knob_brightness: None,
            colours: Vec::new(),
            support: None,
            chosen: None,
            pending: None,
            throttle: None,
            subscriptions: Vec::new(),
        }
    }
}

fn slider(min: u8, value: u8, cx: &mut Context<DeviceView>) -> Entity<SliderState> {
    cx.new(|_| {
        SliderState::new()
            .min(f32::from(min))
            .max(f32::from(MAX_PERCENT))
            .step(1.)
            .default_value(f32::from(value))
    })
}

fn percent(value: f32) -> u8 {
    value.round().clamp(0., f32::from(MAX_PERCENT)) as u8
}

impl DeviceView {
    /// The settings that the area shows: the last choice of the user while
    /// the throttle time runs, else the settings of the engine.
    pub(super) fn shown_settings(&self) -> Option<&DeviceSettings> {
        self.device
            .chosen
            .as_ref()
            .or(self.state.device_settings.as_ref())
    }

    pub(super) fn toggle_device_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.device.open = !self.device.open;
        if self.device.open {
            self.close_kinds();
            self.sync_device_panel(window, cx);
        }
        window.blur(cx);
        cx.notify();
    }

    /// Make the controls for the settings support of the layout, and give
    /// them the values of the engine. A control that the user moves now
    /// keeps its value.
    pub(super) fn sync_device_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(support) = self.state.layout.as_ref().map(|l| l.settings) else {
            return;
        };
        let Some(settings) = self.state.device_settings.clone() else {
            return;
        };
        if self.device.support != Some(support) {
            self.make_device_controls(support, &settings, window, cx);
        }
        if self.device.throttle.is_some() {
            return;
        }
        if let Some(slider) = &self.device.brightness {
            slider.update(cx, |s, cx| {
                if percent(s.value().end()) != settings.brightness {
                    s.set_value(f32::from(settings.brightness), window, cx);
                }
            });
        }
        if let Some(slider) = &self.device.knob_brightness {
            let value = settings.knob_lights.brightness;
            slider.update(cx, |s, cx| {
                if percent(s.value().end()) != value {
                    s.set_value(f32::from(value), window, cx);
                }
            });
        }
        for (input, colour) in self.device.colours.iter().zip(&settings.knob_lights.colors) {
            let focused = input.focus_handle(cx).is_focused(window);
            let same = input.read(cx).value().parse::<Color>().ok() == Some(*colour);
            if !focused && !same {
                input.update(cx, |input, cx| {
                    input.set_value(colour.to_string(), window, cx)
                });
            }
        }
    }

    fn make_device_controls(
        &mut self,
        support: SettingsSupport,
        settings: &DeviceSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel = &mut self.device;
        panel.support = Some(support);
        panel.subscriptions.clear();
        let brightness = slider(support.lowest_brightness, settings.brightness, cx);
        let knob = slider(LOWEST_KNOB_PERCENT, settings.knob_lights.brightness, cx);
        let mut subscriptions = vec![
            cx.subscribe_in(&brightness, window, |view, _, event, window, cx| {
                let (SliderEvent::Change(value) | SliderEvent::Release(value)) = event;
                let value = percent(value.end());
                view.change_settings(|s| s.brightness = value, window, cx);
            }),
            cx.subscribe_in(&knob, window, |view, _, event, window, cx| {
                let (SliderEvent::Change(value) | SliderEvent::Release(value)) = event;
                let value = percent(value.end());
                view.change_settings(|s| s.knob_lights.brightness = value, window, cx);
            }),
        ];
        let mut colours = Vec::new();
        for index in 0..usize::from(support.knob_lights) {
            let text = settings
                .knob_lights
                .colors
                .get(index)
                .map(Color::to_string)
                .unwrap_or_default();
            let input = cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder("#RRGGBB");
                input.set_value(text, window, cx);
                input
            });
            subscriptions.push(cx.subscribe_in(
                &input,
                window,
                move |view, input, event, window, cx| {
                    if !matches!(event, TextEvent::Change) {
                        return;
                    }
                    // A text that is not a colour yet sends nothing.
                    if let Ok(colour) = input.read(cx).value().trim().parse::<Color>() {
                        view.set_knob_colour(index, colour, false, window, cx);
                    }
                },
            ));
            colours.push(input);
        }
        let panel = &mut self.device;
        panel.brightness = Some(brightness);
        panel.knob_brightness = Some(knob);
        panel.colours = colours;
        panel.subscriptions = subscriptions;
    }

    /// Set the colour of knob light `index`. With `show`, the text field
    /// shows the new colour.
    pub(super) fn set_knob_colour(
        &mut self,
        index: usize,
        colour: Color,
        show: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if show {
            if let Some(input) = self.device.colours.get(index) {
                input.update(cx, |input, cx| {
                    input.set_value(colour.to_string(), window, cx)
                });
            }
        }
        self.change_settings(
            |s| {
                if let Some(slot) = s.knob_lights.colors.get_mut(index) {
                    *slot = colour;
                }
            },
            window,
            cx,
        );
    }

    /// Change the settings that the area shows, and send all settings to the
    /// engine. While the user moves a control, at most one command goes to
    /// the engine in each [`SETTINGS_INTERVAL`], and the last value goes at
    /// the end of the interval.
    pub(super) fn change_settings(
        &mut self,
        change: impl FnOnce(&mut DeviceSettings),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut settings) = self.shown_settings().cloned() else {
            return;
        };
        change(&mut settings);
        if self.shown_settings() == Some(&settings) {
            return;
        }
        self.device.chosen = Some(settings.clone());
        if self.device.throttle.is_some() {
            self.device.pending = Some(settings);
        } else {
            self.send_settings(settings, window, cx);
        }
        cx.notify();
    }

    fn send_settings(
        &mut self,
        settings: DeviceSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        (self.send)(Command::SetDeviceSettings(settings));
        self.device.throttle = Some(cx.spawn_in(window, async move |view, cx| {
            cx.background_executor().timer(SETTINGS_INTERVAL).await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.device.throttle = None;
                match view.device.pending.take() {
                    Some(next) => view.send_settings(next, window, cx),
                    None => {
                        view.device.chosen = None;
                        view.sync_device_panel(window, cx);
                    }
                }
                cx.notify();
            });
        }));
    }

    /// The area in place of the surface editor.
    pub(super) fn device_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut editor = div()
            .id("device-settings")
            .debug_selector(|| "device-settings".into())
            .w(px(272.))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .rounded_lg()
            .bg(rgb(PANEL))
            .child(div().text_lg().child("Device settings"));
        let support = self.state.layout.as_ref().map(|l| l.settings);
        let (Some(support), Some(settings)) = (support, self.shown_settings()) else {
            return editor
                .child(
                    div()
                        .text_color(rgb(MUTED))
                        .child("The device settings are not available yet."),
                )
                .into_any_element();
        };
        if let Some(slider) = &self.device.brightness {
            editor = editor.child(slider_row(
                "brightness",
                "Display brightness",
                settings.brightness,
                slider,
            ));
        }
        if support.haptic {
            editor = editor.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child("Haptic feedback")
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(choice("haptic-on", "On", settings.haptic).on_click(
                                cx.listener(|view, _, window, cx| {
                                    view.change_settings(|s| s.haptic = true, window, cx)
                                }),
                            ))
                            .child(choice("haptic-off", "Off", !settings.haptic).on_click(
                                cx.listener(|view, _, window, cx| {
                                    view.change_settings(|s| s.haptic = false, window, cx)
                                }),
                            )),
                    ),
            );
        }
        if support.knob_lights > 0 {
            editor = editor.child(self.knob_lights(settings, cx));
        }
        editor
            .child(div().text_xs().text_color(rgb(MUTED)).child(
                "The engine keeps these settings for the device model, and sends them to the \
                 device at each connection. You can change them with no device.",
            ))
            .into_any_element()
    }

    fn knob_lights(&self, settings: &DeviceSettings, cx: &mut Context<Self>) -> AnyElement {
        let lights = &settings.knob_lights;
        let mode = lights.mode;
        let mode_button = |id: &'static str, text: &'static str, value: KnobLightMode| {
            choice(id, text, mode == value).on_click(cx.listener(move |view, _, window, cx| {
                view.change_settings(|s| s.knob_lights.mode = value, window, cx)
            }))
        };
        let mut area = div().flex().flex_col().gap_4().child(
            div().flex().flex_col().gap_2().child("Knob lights").child(
                div()
                    .flex()
                    .gap_2()
                    .child(mode_button("knob-effect", "Effect", KnobLightMode::Effect))
                    .child(mode_button(
                        "knob-colours",
                        "Colours",
                        KnobLightMode::Colors,
                    ))
                    .child(mode_button("knob-off", "Off", KnobLightMode::Off)),
            ),
        );
        if let Some(slider) = &self.device.knob_brightness {
            area = area.child(slider_row(
                "knob-brightness",
                "Knob light brightness",
                lights.brightness,
                slider,
            ));
        }
        if mode == KnobLightMode::Colors {
            for (index, input) in self.device.colours.iter().enumerate() {
                let current = lights.colors.get(index).copied();
                area = area.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .size(px(20.))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(rgb(BORDER))
                                        .bg(rgb(current.map_or(0, |c| {
                                            u32::from(c.0[0]) << 16
                                                | u32::from(c.0[1]) << 8
                                                | u32::from(c.0[2])
                                        }))),
                                )
                                .child(div().w(px(56.)).child(format!("Knob {}", index + 1)))
                                .child(div().flex_1().child(Input::new(input))),
                        )
                        .child(div().flex().flex_wrap().gap_1().children(
                            KNOB_COLOURS.into_iter().enumerate().map(|(n, value)| {
                                let id = format!("knob-{}-colour-{n}", index + 1);
                                div()
                                    .id(SharedString::from(id.clone()))
                                    .debug_selector(move || id.clone())
                                    .size(px(22.))
                                    .rounded_md()
                                    .bg(rgb(value))
                                    .border_2()
                                    .border_color(rgb(if current == Some(colour(value)) {
                                        SELECTED
                                    } else {
                                        BORDER
                                    }))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |view, _, window, cx| {
                                        view.set_knob_colour(index, colour(value), true, window, cx)
                                    }))
                            }),
                        )),
                );
            }
            area =
                area.child(div().text_xs().text_color(rgb(MUTED)).child(
                    "A black knob is off. If all knobs are black, the knob lights are off.",
                ));
        }
        area.into_any_element()
    }
}

/// A label with the value in percent, and the slider below it.
fn slider_row(
    id: &'static str,
    text: &'static str,
    value: u8,
    slider: &Entity<SliderState>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div().flex().justify_between().child(text).child(
                div()
                    .id(id)
                    .debug_selector(move || format!("{id}-value"))
                    .text_color(rgb(MUTED))
                    .child(format!("{value} %")),
            ),
        )
        .child(
            div()
                .id(SharedString::from(format!("{id}-slider")))
                .debug_selector(move || format!("{id}-slider"))
                .py_1()
                .child(Slider::new(slider).accessibility_label(text)),
        )
        .into_any_element()
}

/// One button of a group of choices. The chosen button has the selected
/// border.
fn choice(id: &'static str, text: &'static str, chosen: bool) -> Stateful<Div> {
    small_button(id, text, true).when(chosen, |d| {
        d.border_color(rgb(SELECTED)).text_color(rgb(SELECTED))
    })
}
