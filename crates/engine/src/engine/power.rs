use super::*;
use crate::power::{DisplayState, PowerNotice};
use opencraft_core::link::KEEP_ALIVE_INTERVAL;

impl<C: Connector, S: FnMut(Event), K: Clock> Engine<C, S, K> {
    /// Restart one deadline. With Never, add no timer, thread, or processor wake-up.
    pub(super) fn restart_idle(&mut self) {
        self.power.idle_deadline = if self.settings.screen_off_minutes == 0
            || self.power.state != DisplayState::Awake
        {
            None
        } else {
            Some(self.now() + Duration::from_secs(u64::from(self.settings.screen_off_minutes) * 60))
        };
    }

    pub(super) fn check_idle(&mut self) {
        if self.power.idle_deadline.is_some_and(|at| self.now() >= at) {
            self.set_display_state(DisplayState::ScreenOff);
        }
    }

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
                // A wake of the computer is not a wake of its displays: a
                // maintenance wake leaves them dark. `DisplaysWake`, an
                // unlock, or a device input lights the device then.
                if self.power.locked || self.power.displays_asleep {
                    self.set_display_state(DisplayState::ScreenOff);
                } else {
                    // The device can have started again during the sleep,
                    // also when no notice came before it. Draw it again.
                    self.wake(true);
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

    /// An explicit wake: a device input, the window, or the menu bar item.
    /// It never overrides a lock or system sleep. A device that is awake
    /// stays as it is: only the idle time starts again.
    pub(super) fn wake_device(&mut self) {
        self.wake(false);
    }

    /// `redraw_if_awake` is for a wake of the computer, where the device
    /// can have lost its picture although the engine state is awake.
    fn wake(&mut self, redraw_if_awake: bool) {
        if self.power.locked || self.power.sleeping {
            return;
        }
        if self.power.state == DisplayState::Awake && !redraw_if_awake {
            self.restart_idle();
            return;
        }
        // The user is at the computer or at the device.
        self.power.displays_asleep = false;
        if self.power.state == DisplayState::DeviceSleep {
            self.link.set_keep_alive_interval(Some(KEEP_ALIVE_INTERVAL));
        }
        self.power.state = DisplayState::Awake;
        self.restart_idle();
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
        self.power.idle_deadline = None;
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
        let was_down = self.power.down_key == Some(event.control);
        match event.gesture {
            Gesture::Press => self.power.down_key = Some(event.control),
            Gesture::Release if was_down => self.power.down_key = None,
            _ => {}
        }
        if self.power.locked || self.power.sleeping {
            if event.gesture == Gesture::Press {
                self.power.consumed_key = Some(event.control);
            }
            if matches!(
                event.gesture,
                Gesture::TouchStart { .. } | Gesture::TouchPoint { .. }
            ) {
                self.power.consumed_touch = true;
            }
            return true;
        }
        if self.power.state != DisplayState::Awake {
            // The release of the key that requested screen off is not a new
            // gesture and must not wake the device at once.
            if event.gesture == Gesture::Release && was_down {
                return true;
            }
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
                self.restart_idle();
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
        self.restart_idle();
        false
    }
}
