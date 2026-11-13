use super::*;
use crate::power::{DisplayState, PowerNotice};
use opencraft_core::link::KEEP_ALIVE_INTERVAL;

impl<C: Connector, S: FnMut(Event), K: Clock> Engine<C, S, K> {
    /// Current requested device light state.
    pub fn display_state(&self) -> DisplayState {
        self.power.state
    }

    pub(super) fn power_notice(&mut self, notice: PowerNotice) {
        match notice {
            PowerNotice::WillSleep => {
                self.power.sleeping = true;
                if let Err(e) = self.write_now() {
                    self.problem(format!("OpenCraft could not save the profile: {e}"));
                }
                self.save_settings(self.now());
                self.set_display_state(DisplayState::DeviceSleep);
            }
            PowerNotice::DidWake => {
                self.power.sleeping = false;
                self.power.displays_asleep = false;
                if self.power.locked {
                    self.set_display_state(DisplayState::ScreenOff);
                } else {
                    self.wake_device();
                }
            }
            PowerNotice::Locked => {
                self.power.locked = true;
                if !self.power.sleeping {
                    self.set_display_state(DisplayState::ScreenOff);
                }
            }
            PowerNotice::Unlocked => {
                self.power.locked = false;
                self.power.displays_asleep = false;
                self.wake_device();
            }
            PowerNotice::DisplaysSleep => {
                self.power.displays_asleep = true;
                if !self.power.sleeping {
                    self.set_display_state(DisplayState::ScreenOff);
                }
            }
            PowerNotice::DisplaysWake => {
                self.power.displays_asleep = false;
                self.wake_device();
            }
        }
    }

    /// An explicit computer-side wake. It never overrides a lock or system sleep.
    pub(super) fn wake_device(&mut self) {
        if self.power.locked || self.power.sleeping || self.power.displays_asleep {
            return;
        }
        if self.power.state == DisplayState::DeviceSleep {
            self.link.set_keep_alive_interval(Some(KEEP_ALIVE_INTERVAL));
        }
        self.power.state = DisplayState::Awake;
        self.needs_draw = true;
        if self.link.is_connected() {
            (self.sink)(Event::Disconnected);
            self.draw_all();
            self.announce_connected();
        }
    }

    pub(super) fn set_display_state(&mut self, state: DisplayState) {
        if state == DisplayState::Awake {
            self.wake_device();
            return;
        }
        if self.power.state == state {
            return;
        }
        self.power.state = state;
        self.needs_draw = true;
        self.apply_dark();
    }

    pub(super) fn apply_dark(&mut self) {
        let sleep = self.power.state == DisplayState::DeviceSleep;
        self.link
            .set_keep_alive_interval((!sleep).then_some(KEEP_ALIVE_INTERVAL));
        let result = self.link.with_device(|device| {
            if sleep {
                device.stop()
            } else {
                device.send(opencraft_core::protocol::sleep())
            }
        });
        // The firmware needs a pause between DC and a later start, even when
        // a wake follows at once. Polls, not a blocking real sleep, enforce it.
        if sleep {
            self.power.ready_after = Some(self.now() + Duration::from_millis(100));
        }
        self.needs_draw = match result {
            Ok(()) | Err(DeviceError::Disconnected) => false,
            Err(e) => {
                self.link_error(&e);
                true
            }
        };
    }

    /// Consume the entire first gesture, including a later key release or
    /// touch completion. Acknowledgements never call this method.
    pub(super) fn consume_dark_input(&mut self, event: InputEvent) -> bool {
        use opencraft_core::Gesture;
        if self.power.locked || self.power.sleeping {
            return true;
        }
        if self.power.state != DisplayState::Awake {
            if event.gesture == Gesture::Press {
                self.power.consumed_key = Some(event.control);
            }
            self.power.consumed_touch = matches!(
                event.gesture,
                Gesture::TouchStart { .. } | Gesture::TouchPoint { .. }
            );
            self.wake_device();
            return true;
        }
        if self.power.consumed_key == Some(event.control) {
            if event.gesture == Gesture::Release {
                self.power.consumed_key = None;
            } else if event.gesture == Gesture::Press {
                // A lost release must not swallow the next press.
                self.power.consumed_key = None;
                return false;
            }
            return true;
        }
        if self.power.consumed_touch {
            match event.gesture {
                Gesture::TouchPoint { .. } => return true,
                Gesture::Tap { .. }
                | Gesture::SwipeLeft
                | Gesture::SwipeRight
                | Gesture::TouchEnd => {
                    self.power.consumed_touch = false;
                    return true;
                }
                Gesture::TouchStart { .. } => self.power.consumed_touch = false,
                _ => {}
            }
        }
        false
    }
}
