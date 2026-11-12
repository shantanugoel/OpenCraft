//! The state of the device inspector: the display brightness, the haptic
//! feedback, and the knob lights (decision 0022). The inspector itself is
//! in `device_inspector`.
//!
//! The inspector shows only the settings that the layout gives for the
//! model. It changes the settings only with [`Command::SetDeviceSettings`],
//! with all settings. The engine keeps and applies them, also with no device
//! and no window.

use super::*;
use gpui_kit::component::slider::{SliderEvent, SliderState};
use gpui_kit::Focusable;
use opencraft_engine::{DeviceSettings, SettingsSupport};
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

/// The controls of the inspector, and the throttle of the commands.
pub(super) struct DevicePanel {
    /// True while the device is the selection: the inspector then shows the
    /// device settings, and no control has the frame of the selection.
    pub open: bool,
    pub(super) brightness: Option<Entity<SliderState>>,
    pub(super) knob_brightness: Option<Entity<SliderState>>,
    /// The knob light whose colour popover is open.
    pub(super) colour_open: Option<usize>,
    /// The field for `#RRGGBB` in that popover.
    pub(super) colour_input: Entity<InputState>,
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
    _colour_subscription: Subscription,
}

impl DevicePanel {
    pub fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let colour_input = cx.new(|cx| InputState::new(window, cx).placeholder("#RRGGBB"));
        let subscription =
            cx.subscribe_in(&colour_input, window, |view, input, event, window, cx| {
                match event {
                    // A text that is not a colour yet sends nothing.
                    TextEvent::Change => {
                        if let (Some(index), Some(colour)) = (
                            view.device.colour_open,
                            super::inspector::parse_colour(&input.read(cx).value()),
                        ) {
                            view.set_knob_colour(index, colour, window, cx);
                        }
                    }
                    TextEvent::PressEnter { .. } => view.close_knob_colour(window, cx),
                    _ => {}
                }
            });
        Self {
            open: false,
            brightness: None,
            knob_brightness: None,
            colour_open: None,
            colour_input,
            support: None,
            chosen: None,
            pending: None,
            throttle: None,
            subscriptions: Vec::new(),
            _colour_subscription: subscription,
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
    /// The settings that the inspector shows: the last choice of the user while
    /// the throttle time runs, else the settings of the engine.
    pub(super) fn shown_settings(&self) -> Option<&DeviceSettings> {
        self.device
            .chosen
            .as_ref()
            .or(self.state.device_settings.as_ref())
    }

    /// Make the device the selection. The inspector then shows the device
    /// settings (decision 0029, sections 8.3 and 10.7).
    pub(super) fn select_device(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_kinds();
        self.device.open = true;
        self.sync_device_panel(window, cx);
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// Open the popover of the colour of knob light `index`, or close it if
    /// it is open.
    pub(super) fn toggle_knob_colour(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let open = self.device.colour_open != Some(index);
        self.close_kinds();
        if open {
            self.device.colour_open = Some(index);
            let text = self
                .shown_settings()
                .and_then(|s| s.knob_lights.colors.get(index))
                .map(Color::to_string)
                .unwrap_or_default();
            let input = self.device.colour_input.clone();
            input.update(cx, |input, cx| {
                input.set_value(text, window, cx);
                input.focus(window, cx);
            });
        } else {
            self.focus_workspace(window, cx);
        }
        cx.notify();
    }

    /// Close the popover of a knob light colour. The colour stays.
    pub(super) fn close_knob_colour(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_kinds();
        self.focus_workspace(window, cx);
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
        if let Some(index) = self.device.colour_open {
            let input = self.device.colour_input.clone();
            let colour = settings.knob_lights.colors.get(index).copied();
            let focused = input.focus_handle(cx).is_focused(window);
            let same = super::inspector::parse_colour(&input.read(cx).value()) == colour;
            if let (false, false, Some(colour)) = (focused, same, colour) {
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
        let subscriptions = vec![
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
        let panel = &mut self.device;
        panel.brightness = Some(brightness);
        panel.knob_brightness = Some(knob);
        panel.subscriptions = subscriptions;
    }

    /// Set the colour of knob light `index`.
    pub(super) fn set_knob_colour(
        &mut self,
        index: usize,
        colour: Color,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    /// Change the settings that the inspector shows, and send all settings to the
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
}
