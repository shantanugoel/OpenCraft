//! Recognize key gestures from reports and monotonic time (decision 0042).

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use opencraft_core::{ControlId, ControlKind, Gesture, InputEvent};

use crate::GestureKind;

const LONG_PRESS_THRESHOLD: Duration = Duration::from_millis(500);
const DOUBLE_PRESS_WINDOW: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct KeyFacts {
    pub long_press: bool,
    pub double_press: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dispatch {
    Invoke {
        control: ControlId,
        gesture: GestureKind,
    },
}

#[derive(Clone, Copy)]
enum KeyState {
    DownImmediate,
    DownWaitingLong {
        deadline: Instant,
        double_press: bool,
    },
    DownWaitingDouble,
    WaitingDouble {
        deadline: Instant,
    },
    /// Still down, but the gesture has resolved or was cancelled.
    Ignored,
}

impl KeyState {
    fn is_down(self) -> bool {
        !matches!(self, Self::WaitingDouble { .. })
    }

    fn deadline(self) -> Option<Instant> {
        match self {
            Self::DownWaitingLong { deadline, .. } | Self::WaitingDouble { deadline } => {
                Some(deadline)
            }
            _ => None,
        }
    }
}

#[derive(Default)]
pub(crate) struct Gestures {
    // An absent key is idle. Non-key controls never enter the map.
    states: BTreeMap<ControlId, KeyState>,
}

fn short(control: ControlId) -> [Dispatch; 2] {
    [GestureKind::Press, GestureKind::Release].map(|gesture| Dispatch::Invoke { control, gesture })
}

impl Gestures {
    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.states.values().filter_map(|s| s.deadline()).min()
    }

    pub(crate) fn any_key_down(&self) -> bool {
        self.states.values().any(|s| s.is_down())
    }

    pub(crate) fn tick(&mut self, now: Instant) -> Vec<Dispatch> {
        let mut out = Vec::new();
        self.states.retain(|&control, state| {
            if state.deadline().is_none_or(|at| at > now) {
                return true;
            }
            if state.is_down() {
                *state = KeyState::Ignored;
                out.push(Dispatch::Invoke {
                    control,
                    gesture: GestureKind::LongPress,
                });
                true
            } else {
                out.extend(short(control));
                false
            }
        });
        out
    }

    pub(crate) fn input(
        &mut self,
        event: InputEvent,
        now: Instant,
        facts: impl FnOnce(ControlId) -> KeyFacts,
    ) -> Vec<Dispatch> {
        let mut out = self.tick(now);
        let control = event.control;
        let invoke = |gesture| Dispatch::Invoke { control, gesture };
        self.states.retain(|&other, state| {
            if other != control && !state.is_down() {
                out.extend(short(other));
                false
            } else {
                true
            }
        });
        if control.kind != ControlKind::Key {
            out.extend(GestureKind::from_gesture(event.gesture).map(invoke));
            return out;
        }
        match event.gesture {
            Gesture::Press => {
                // A different key press means the preceding release was lost.
                self.states.retain(|&other, _| other == control);
                let state = match self.states.get(&control) {
                    Some(state) if state.is_down() => return out,
                    Some(_) => {
                        out.push(invoke(GestureKind::DoublePress));
                        KeyState::Ignored
                    }
                    None => match facts(control) {
                        KeyFacts {
                            long_press: true,
                            double_press,
                        } => KeyState::DownWaitingLong {
                            deadline: now + LONG_PRESS_THRESHOLD,
                            double_press,
                        },
                        KeyFacts {
                            double_press: true, ..
                        } => KeyState::DownWaitingDouble,
                        _ => {
                            out.push(invoke(GestureKind::Press));
                            KeyState::DownImmediate
                        }
                    },
                };
                self.states.insert(control, state);
            }
            Gesture::Release => match self.states.remove(&control) {
                Some(KeyState::DownImmediate) => out.push(invoke(GestureKind::Release)),
                Some(KeyState::DownWaitingLong {
                    double_press: false,
                    ..
                }) => out.extend(short(control)),
                Some(
                    KeyState::DownWaitingLong {
                        double_press: true, ..
                    }
                    | KeyState::DownWaitingDouble,
                ) => {
                    self.states.insert(
                        control,
                        KeyState::WaitingDouble {
                            deadline: now + DOUBLE_PRESS_WINDOW,
                        },
                    );
                }
                Some(state @ KeyState::WaitingDouble { .. }) => {
                    self.states.insert(control, state);
                }
                Some(KeyState::Ignored) | None => {}
            },
            _ => {}
        }
        out
    }

    pub(crate) fn cancel(&mut self, control: ControlId) {
        if self.states.remove(&control).is_some_and(KeyState::is_down) {
            self.states.insert(control, KeyState::Ignored);
        }
    }

    /// A power or link change can swallow releases. Forget those keys too.
    pub(crate) fn cancel_all(&mut self) {
        self.states.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use GestureKind::{DoublePress, LongPress, Press, Release};

    const A: ControlId = ControlId::new(ControlKind::Key, 1);
    const B: ControlId = ControlId::new(ControlKind::Key, 2);
    const KNOB: ControlId = ControlId::new(ControlKind::Knob, 1);
    const BAR: ControlId = ControlId::new(ControlKind::BarPanel, 1);
    const TOUCH: ControlId = ControlId::new(ControlKind::TouchBar, 1);

    fn at(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    fn key(g: &mut Gestures, time: Instant, edge: Gesture, l: bool, d: bool) -> Vec<Dispatch> {
        g.input(
            InputEvent {
                control: A,
                gesture: edge,
            },
            time,
            |_| KeyFacts {
                long_press: l,
                double_press: d,
            },
        )
    }

    fn invoke(control: ControlId, gesture: GestureKind) -> Dispatch {
        Dispatch::Invoke { control, gesture }
    }

    #[test]
    fn no_timer_and_no_release_without_a_press() {
        let mut g = Gestures::default();
        let t = Instant::now();
        assert!(key(&mut g, t, Gesture::Release, false, false).is_empty());
        assert_eq!(
            key(&mut g, t, Gesture::Press, false, false),
            [invoke(A, Press)]
        );
        assert!(g.any_key_down());
        assert_eq!(g.deadline(), None);
        assert!(key(&mut g, t, Gesture::Press, false, false).is_empty());
        assert_eq!(
            key(&mut g, t, Gesture::Release, false, false),
            [invoke(A, Release)]
        );
        assert!(!g.any_key_down());
    }

    #[test]
    fn the_long_timer_wins_at_the_boundary_without_repeating() {
        for double in [false, true] {
            let t = Instant::now();
            let mut g = Gestures::default();
            assert!(key(&mut g, t, Gesture::Press, true, double).is_empty());
            assert!(key(&mut g, at(t, 499), Gesture::Press, false, false).is_empty());
            assert_eq!(g.deadline(), Some(at(t, 500)));
            assert!(g.tick(at(t, 499)).is_empty());
            assert_eq!(g.tick(at(t, 500)), [invoke(A, LongPress)]);
            assert!(g.any_key_down());
            assert!(key(&mut g, at(t, 501), Gesture::Press, true, double).is_empty());
            assert!(key(&mut g, at(t, 502), Gesture::Release, true, double).is_empty());
            assert_eq!(g.deadline(), None);
            assert!(key(&mut g, at(t, 600), Gesture::Press, true, double).is_empty());
            assert_eq!(
                key(&mut g, at(t, 1100), Gesture::Release, true, double),
                [invoke(A, LongPress)]
            );
        }
        let t = Instant::now();
        let mut g = Gestures::default();
        assert!(key(&mut g, t, Gesture::Press, true, false).is_empty());
        assert_eq!(
            key(&mut g, at(t, 499), Gesture::Release, false, true),
            short(A)
        );
    }

    #[test]
    fn the_double_window_starts_at_release_and_excludes_its_end() {
        for long in [false, true] {
            for delay in [299, 300] {
                let t = Instant::now();
                let mut g = Gestures::default();
                assert!(key(&mut g, t, Gesture::Press, long, true).is_empty());
                assert!(key(&mut g, at(t, 130), Gesture::Press, long, true).is_empty());
                assert!(key(&mut g, at(t, 200), Gesture::Release, false, false).is_empty());
                assert!(key(&mut g, at(t, 201), Gesture::Release, false, false).is_empty());
                assert!(!g.any_key_down());
                assert_eq!(g.deadline(), Some(at(t, 500)));
                let out = key(&mut g, at(t, 200 + delay), Gesture::Press, long, true);
                if delay == 299 {
                    assert_eq!(out, [invoke(A, DoublePress)]);
                    assert!(g.tick(at(t, 1100)).is_empty());
                    assert!(key(&mut g, at(t, 1101), Gesture::Release, long, true).is_empty());
                } else {
                    assert_eq!(out, short(A));
                    assert!(key(&mut g, at(t, 530), Gesture::Release, long, true).is_empty());
                    assert!(g.tick(at(t, 829)).is_empty());
                    assert_eq!(g.tick(at(t, 830)), short(A));
                }
                assert_eq!(g.deadline(), None);
            }
        }
    }

    #[test]
    fn any_other_control_flushes_the_window_before_its_own_action() {
        for (control, gesture, action) in [
            (B, Gesture::Press, Some(Press)),
            (B, Gesture::Release, None),
            (KNOB, Gesture::TurnLeft, Some(GestureKind::TurnLeft)),
            (BAR, Gesture::Tap { x: 0 }, Some(GestureKind::Tap)),
            (TOUCH, Gesture::TouchStart { x: 0 }, None),
            (TOUCH, Gesture::SwipeLeft, Some(GestureKind::SwipeLeft)),
        ] {
            let t = Instant::now();
            let mut g = Gestures::default();
            assert!(key(&mut g, t, Gesture::Press, true, true).is_empty());
            assert!(key(&mut g, at(t, 30), Gesture::Release, true, true).is_empty());
            let mut expected = short(A).to_vec();
            expected.extend(action.map(|gesture| invoke(control, gesture)));
            assert_eq!(
                g.input(InputEvent { control, gesture }, at(t, 31), |_| {
                    KeyFacts::default()
                }),
                expected
            );
            assert_eq!(g.deadline(), None);
        }
    }

    #[test]
    fn a_key_press_drops_a_lost_release_but_a_knob_turn_does_not() {
        let t = Instant::now();
        let mut g = Gestures::default();
        assert!(key(&mut g, t, Gesture::Press, true, true).is_empty());
        assert_eq!(
            g.input(
                InputEvent {
                    control: KNOB,
                    gesture: Gesture::TurnLeft
                },
                at(t, 1),
                |_| unreachable!()
            ),
            [invoke(KNOB, GestureKind::TurnLeft)]
        );
        assert_eq!(g.deadline(), Some(at(t, 500)));
        assert_eq!(
            g.input(
                InputEvent {
                    control: B,
                    gesture: Gesture::Press
                },
                at(t, 2),
                |_| KeyFacts::default()
            ),
            [invoke(B, Press)]
        );
        assert!(key(&mut g, at(t, 500), Gesture::Release, true, true).is_empty());
        assert_eq!(
            g.input(
                InputEvent {
                    control: B,
                    gesture: Gesture::Release
                },
                at(t, 501),
                |_| unreachable!()
            ),
            [invoke(B, Release)]
        );
        assert!(key(&mut g, at(t, 502), Gesture::Press, true, true).is_empty());
        assert!(key(&mut g, at(t, 532), Gesture::Release, true, true).is_empty());
        assert_eq!(
            key(&mut g, at(t, 533), Gesture::Press, true, true),
            [invoke(A, DoublePress)]
        );
    }

    #[test]
    fn cancel_stops_timers_but_only_cancel_all_forgets_a_down_key() {
        for released in [false, true] {
            for all in [false, true] {
                let t = Instant::now();
                let mut g = Gestures::default();
                assert!(key(&mut g, t, Gesture::Press, true, true).is_empty());
                if released {
                    assert!(key(&mut g, at(t, 1), Gesture::Release, true, true).is_empty());
                }
                g.cancel(B);
                assert!(g.deadline().is_some());
                if all {
                    g.cancel_all();
                } else {
                    g.cancel(A);
                }
                assert_eq!(g.deadline(), None);
                assert_eq!(g.any_key_down(), !released && !all);
                assert!(g.tick(at(t, 500)).is_empty());
                if !released && !all {
                    assert!(key(&mut g, at(t, 501), Gesture::Press, false, false).is_empty());
                }
                assert!(key(&mut g, at(t, 502), Gesture::Release, false, false).is_empty());
                assert_eq!(
                    key(&mut g, at(t, 503), Gesture::Press, false, false),
                    [invoke(A, Press)]
                );
            }
        }
    }
}
