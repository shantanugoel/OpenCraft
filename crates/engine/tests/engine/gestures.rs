use super::*;
use opencraft_core::recording::{parse, Report};
use opencraft_engine::{EngineRequest, FolderId, PageTarget, PowerNotice};
use GestureKind::{DoublePress, LongPress, Press, Release, TurnRight};

const KIND: &str = "test.gestures";
const HOLD: &str = "test.hold";
const TIMER: &str = "test.timer";

struct Setup {
    rig: Rig,
    device: Rc<RefCell<FakeTransport>>,
    requests: mpsc::Receiver<Request>,
}

impl Setup {
    fn new(key: u8, gestures: &[GestureKind]) -> Self {
        let (mut port, requests) = port(KIND);
        port.kinds.push(description(HOLD).with_receives_release());
        port.kinds
            .push(description(TIMER).with_handles_long_press());
        let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
        for &gesture in gestures {
            set_binding(
                &mut rig,
                binding(ControlKind::Key, key, gesture),
                Some(action(KIND)),
            );
        }
        let device = rig.connect();
        rig.take_events();
        Self {
            rig,
            device,
            requests,
        }
    }

    fn with_hold() -> Self {
        let mut s = Self::new(1, &[Release, LongPress, DoublePress]);
        s.bind(1, Press, Some(HOLD));
        s.requests.try_iter().for_each(drop);
        s
    }

    fn bind(&mut self, key: u8, gesture: GestureKind, kind: Option<&str>) {
        let binding = BindingId {
            page: self.rig.engine.active_page(),
            folder: self.rig.engine.open_folders().last().copied(),
            ..binding(ControlKind::Key, key, gesture)
        };
        set_binding(&mut self.rig, binding, kind.map(action));
    }

    fn input(&mut self, bytes: Vec<u8>) {
        self.device.borrow_mut().push_input(bytes);
        self.rig.engine.poll(Duration::ZERO);
    }

    fn edge(&mut self, code: u8, state: u8) {
        self.input(control_report(code, state));
    }

    fn advance(&mut self, ms: u64) {
        let now = self.rig.world.now.get();
        self.rig.world.now.set(now + Duration::from_millis(ms));
    }

    fn invokes(&self) -> Vec<GestureKind> {
        self.requests
            .try_iter()
            .filter_map(|request| match request {
                Request::Invoke { binding, .. } => Some(binding.gesture),
                _ => None,
            })
            .collect()
    }
}

#[test]
fn hold_press_and_release_sends_hold_start_then_hold_end_and_runs_release_binding() {
    let mut s = Setup::with_hold();
    let press = binding(ControlKind::Key, 1, Press);
    s.edge(1, 1);
    assert_eq!(
        s.requests.try_iter().collect::<Vec<_>>(),
        [Request::HoldStart {
            binding: press,
            action: action(HOLD)
        }]
    );
    s.edge(1, 1); // Duplicate while the key is down.
    s.advance(1_000);
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.requests.try_iter().next().is_none());
    s.edge(1, 0);
    assert_eq!(
        s.requests.try_iter().collect::<Vec<_>>(),
        [
            Request::HoldEnd { binding: press },
            Request::Invoke {
                binding: BindingId {
                    gesture: Release,
                    ..press
                },
                action: action(KIND)
            },
        ]
    );
    s.edge(1, 0);
    assert!(s.requests.try_iter().next().is_none());
}

#[test]
fn a_hotkey_hold_keeps_the_display_awake_past_the_idle_deadline() {
    use opencraft_engine::DisplayState;

    let mut s = Setup::with_hold();
    let mut settings = s.rig.engine.device_settings().clone();
    settings.screen_off_minutes = 1;
    s.rig.engine.command(Command::SetDeviceSettings(settings));
    s.edge(1, 1);
    assert!(matches!(
        s.requests.try_recv(),
        Ok(Request::HoldStart { .. })
    ));
    s.advance(120_000);
    s.rig.engine.poll(Duration::ZERO);
    assert_eq!(s.rig.engine.display_state(), DisplayState::Awake);
    assert!(s.requests.try_recv().is_err());
    s.edge(1, 0);
    assert!(matches!(s.requests.try_recv(), Ok(Request::HoldEnd { .. })));
    s.advance(60_000);
    s.rig.engine.poll(Duration::ZERO);
    assert_eq!(s.rig.engine.display_state(), DisplayState::ScreenOff);
}

#[test]
fn hold_on_press_wins_over_long_and_double_press_and_marks_them_inert() {
    let mut s = Setup::with_hold();
    let states = |s: &mut Setup| {
        s.rig
            .take_events()
            .into_iter()
            .filter_map(|event| {
                if let Event::Binding(state) = event {
                    Some((state.binding.gesture, state.inert_by))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        states(&mut s),
        [
            (Press, None),
            (LongPress, Some(Press)),
            (DoublePress, Some(Press))
        ]
    );
    for _ in 0..2 {
        s.edge(1, 1);
        assert!(matches!(
            s.requests.try_recv(),
            Ok(Request::HoldStart { .. })
        ));
        s.edge(1, 0);
        assert!(matches!(s.requests.try_recv(), Ok(Request::HoldEnd { .. })));
        assert_eq!(s.invokes(), [Release]);
    }
    s.advance(1000);
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.requests.try_iter().next().is_none());
    s.bind(1, Press, Some(KIND));
    assert_eq!(
        states(&mut s),
        [(Press, None), (LongPress, None), (DoublePress, None)]
    );
    s.bind(1, Release, Some(HOLD));
    assert_eq!(states(&mut s), [(Release, None)]);
}

#[test]
fn hold_kind_on_non_press_gesture_or_test_action_or_engine_request_sends_invoke() {
    let hold_requests = |s: &Setup| {
        s.requests
            .try_iter()
            .filter(|request| {
                !matches!(request, Request::Appear { .. } | Request::Disappear { .. })
            })
            .collect::<Vec<_>>()
    };

    for gesture in [Release, LongPress, DoublePress] {
        let mut s = Setup::new(1, &[]);
        s.bind(1, gesture, Some(HOLD));
        let id = binding(ControlKind::Key, 1, gesture);
        s.edge(1, 1);
        match gesture {
            Release => s.edge(1, 0),
            LongPress => {
                s.advance(500);
                s.rig.engine.poll(Duration::ZERO);
            }
            DoublePress => {
                s.edge(1, 0);
                s.edge(1, 1);
            }
            _ => unreachable!(),
        }
        assert_eq!(
            hold_requests(&s),
            [Request::Invoke {
                binding: id,
                action: action(HOLD)
            }]
        );
    }
    let mut s = Setup::with_hold();
    let id = binding(ControlKind::Key, 1, Press);
    s.rig.engine.command(Command::TestAction { binding: id });
    s.rig
        .engine
        .handler_message(Message::Engine(EngineRequest::Invoke {
            binding: id,
            action: action(HOLD),
        }));
    assert_eq!(
        hold_requests(&s),
        vec![
            Request::Invoke {
                binding: id,
                action: action(HOLD)
            };
            2
        ]
    );
}

#[test]
fn every_cancel_path_sends_hold_end_before_disappear_and_ignores_late_release() {
    let folder = FolderId::from_raw(1).unwrap();
    for cancel in [
        "page",
        "folder open",
        "folder close",
        "profile",
        "edit",
        "remove",
        "paste",
        "copy to all",
        "dark",
        "lock",
        "sleep",
        "wake",
        "lost release",
        "disconnect",
        "stop",
    ] {
        let mut s = Setup::with_hold();
        s.rig.engine.command(Command::AddPage {
            copy_of: if cancel == "copy to all" {
                None
            } else {
                Some(0)
            },
        });
        s.rig.engine.command(Command::AddFolder {
            id: folder,
            name: Some("More".into()),
            copy_of: None,
        });
        s.rig.engine.select_page(0);
        let original = s.rig.engine.active_profile().to_owned();
        s.rig.engine.command(Command::AddProfile {
            name: "Other".into(),
            copy_of: None,
        });
        let other = s.rig.engine.active_profile().to_owned();
        s.rig.engine.select_profile(&original);
        if cancel == "folder close" {
            s.rig.engine.command(Command::OpenFolder { folder });
            s.bind(1, Press, Some(HOLD));
        }
        if cancel == "paste" {
            s.rig.engine.command(Command::CopyControl {
                control: ControlId::new(ControlKind::Key, 2),
            });
        }
        s.requests.try_iter().for_each(drop);
        s.edge(1, 1);
        let Ok(Request::HoldStart { binding, .. }) = s.requests.try_recv() else {
            panic!("{cancel}: no start")
        };
        match cancel {
            "page" => s.rig.engine.select_page(1),
            "folder open" => {
                s.rig.engine.command(Command::OpenFolder { folder });
            }
            "folder close" => {
                s.rig.engine.command(Command::CloseFolder);
            }
            "profile" => s.rig.engine.select_profile(&other),
            "edit" => s.bind(1, Press, Some(KIND)),
            "remove" => s.bind(1, Press, None),
            "paste" => {
                s.rig.engine.command(Command::PasteControl {
                    control: binding.control,
                });
            }
            "copy to all" => {
                s.rig.engine.command(Command::CopyToAllPages {
                    control: binding.control,
                });
            }
            "dark" | "lock" | "sleep" | "wake" => {
                let notice = match cancel {
                    "dark" => PowerNotice::DisplaysSleep,
                    "lock" => PowerNotice::Locked,
                    "sleep" => PowerNotice::WillSleep,
                    _ => PowerNotice::DidWake,
                };
                s.rig.engine.command(Command::PowerNotice(notice));
            }
            "lost release" => s.edge(2, 1),
            "disconnect" => {
                s.device.borrow_mut().disconnect();
                s.rig.engine.poll(Duration::ZERO);
            }
            "stop" => s.rig.engine.finish(),
            _ => unreachable!(),
        }
        let mut requests = s.requests.try_iter();
        assert_eq!(
            requests.next(),
            Some(Request::HoldEnd { binding }),
            "{cancel}"
        );
        assert!(
            requests.all(|r| !matches!(r, Request::HoldEnd { .. } | Request::Invoke { .. })),
            "{cancel}"
        );
        if cancel == "disconnect" {
            s.device = s.rig.connect();
        }
        if cancel != "stop" {
            s.edge(1, 0);
            assert!(
                s.requests
                    .try_iter()
                    .all(|r| !matches!(r, Request::HoldEnd { .. } | Request::Invoke { .. })),
                "{cancel}"
            );
        }
    }
}

#[test]
fn randomized_sequence_balances_every_hold_start_with_one_hold_end() {
    let mut s = Setup::with_hold();
    for key in [2, 3] {
        s.bind(key, Press, Some(HOLD));
    }
    s.rig.engine.command(Command::AddPage { copy_of: Some(0) });
    let folder = FolderId::from_raw(1).unwrap();
    s.rig.engine.command(Command::AddFolder {
        id: folder,
        name: Some("More".into()),
        copy_of: None,
    });
    s.rig.engine.command(Command::OpenFolder { folder });
    s.bind(1, Press, Some(HOLD));
    s.rig.engine.command(Command::CloseFolder);

    let mut seed = 42_u32;
    let mut active = std::collections::BTreeSet::new();
    let mut started = 0;
    let mut check = |request| match request {
        Request::HoldStart { binding, .. } => {
            started += 1;
            assert!(active.insert(binding), "duplicate HoldStart: {binding:?}");
        }
        Request::HoldEnd { binding } => {
            assert!(active.remove(&binding), "unmatched HoldEnd: {binding:?}")
        }
        Request::Disappear { binding } => assert!(
            !active.contains(&binding),
            "Disappear before HoldEnd: {binding:?}"
        ),
        _ => {}
    };
    for _ in 0..512 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let key = 1 + (seed % 3) as u8;
        match (seed >> 16) % 16 {
            0..=4 => s.edge(key, 1),
            5..=7 => s.edge(key, 0),
            8 => {
                s.advance(u64::from(seed % 600));
                s.rig.engine.poll(Duration::ZERO);
            }
            9 => s.rig.engine.select_page(1 - s.rig.engine.active_page()),
            10 => {
                s.rig.engine.command(Command::OpenFolder { folder });
            }
            11 => {
                s.rig.engine.command(Command::CloseFolder);
            }
            12 => s.bind(
                key,
                Press,
                [Some(HOLD), Some(KIND), None][((seed >> 8) % 3) as usize],
            ),
            13 => {
                s.device.borrow_mut().disconnect();
                s.rig.engine.poll(Duration::ZERO);
                s.device = s.rig.connect();
            }
            14 => {
                s.rig
                    .engine
                    .command(Command::PowerNotice(PowerNotice::DisplaysSleep));
            }
            _ => {
                s.rig.engine.command(Command::WakeDevice);
            }
        }
        s.requests.try_iter().for_each(&mut check);
    }
    s.rig.engine.finish();
    s.requests.try_iter().for_each(check);
    assert!(active.is_empty());
    assert!(started > 10, "only {started} holds started");
}

#[test]
fn press_kind_with_handles_long_press_sends_invoke_on_short_press_and_long_press_on_hold() {
    let mut s = Setup::new(1, &[]);
    s.bind(1, Press, Some(TIMER));
    s.requests.try_iter().for_each(drop);
    let binding = binding(ControlKind::Key, 1, Press);
    s.edge(1, 1);
    assert!(s.requests.try_recv().is_err());
    s.advance(100);
    s.edge(1, 0);
    assert_eq!(
        s.requests.try_recv(),
        Ok(Request::Invoke {
            binding,
            action: action(TIMER)
        })
    );
    s.advance(100);
    s.edge(1, 1);
    s.advance(499);
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.requests.try_recv().is_err());
    s.advance(1);
    s.rig.engine.poll(Duration::ZERO);
    assert_eq!(
        s.requests.try_recv(),
        Ok(Request::LongPress {
            binding,
            action: action(TIMER)
        })
    );
    s.advance(100);
    s.edge(1, 0);
    assert!(s.requests.try_recv().is_err());
}

#[test]
fn explicit_long_press_binding_overrides_handles_long_press_on_press_binding() {
    let mut s = Setup::new(1, &[LongPress]);
    s.bind(1, Press, Some(TIMER));
    s.requests.try_iter().for_each(drop);
    s.edge(1, 1);
    s.advance(500);
    s.rig.engine.poll(Duration::ZERO);
    s.edge(1, 0);
    assert_eq!(
        s.requests.try_iter().collect::<Vec<_>>(),
        [Request::Invoke {
            binding: binding(ControlKind::Key, 1, LongPress),
            action: action(KIND)
        }]
    );
}

fn reports(text: &str) -> [Report; 2] {
    parse(text).unwrap().reports.try_into().unwrap()
}

#[test]
fn a_recorded_short_press_runs_at_the_expected_time_for_each_binding_set() {
    let text = include_str!("../../../core/tests/fixtures/input/01-key-row1-col1.txt");
    for (gestures, at_press, at_release, at_timeout) in [
        (&[Press, Release][..], &[Press][..], &[Release][..], &[][..]),
        (&[Press, Release, LongPress], &[], &[Press, Release], &[]),
        (&[Press, Release, DoublePress], &[], &[], &[Press, Release]),
        (
            &[Press, Release, LongPress, DoublePress],
            &[],
            &[],
            &[Press, Release],
        ),
    ] {
        let mut s = Setup::new(1, gestures);
        let [down, up] = reports(text);
        s.advance(down.ms);
        s.input(down.bytes);
        assert_eq!(s.invokes(), at_press);
        assert!(s
            .rig
            .take_events()
            .iter()
            .any(|e| matches!(e, Event::Input(_))));
        s.advance(up.ms - down.ms);
        s.input(up.bytes);
        assert_eq!(s.invokes(), at_release);
        if !at_timeout.is_empty() {
            let start = s.rig.world.now.get();
            s.rig.engine.poll(Duration::from_secs(10));
            assert_eq!(s.rig.world.now.get() - start, Duration::from_millis(300));
        }
        assert_eq!(s.invokes(), at_timeout);
    }
}

#[test]
fn the_long_timer_shortens_the_poll_and_fires_before_the_recorded_release() {
    let mut s = Setup::new(6, &[Press, Release, LongPress, DoublePress]);
    let [down, up] = reports(include_str!(
        "../../../core/tests/fixtures/input/12-key-hold.txt"
    ));
    s.advance(down.ms);
    s.input(down.bytes);
    assert!(s.invokes().is_empty());
    let start = s.rig.world.now.get();
    s.rig.engine.poll(Duration::from_secs(10));
    assert_eq!(s.rig.world.now.get() - start, Duration::from_millis(500));
    assert_eq!(s.invokes(), [LongPress]);
    s.advance(up.ms - down.ms - 500);
    s.input(up.bytes);
    assert!(s.invokes().is_empty());
}

#[test]
fn a_recorded_adjacent_key_pair_is_a_short_tap_that_can_start_a_double_press() {
    let text = include_str!("../../../core/tests/fixtures/input/11-keys-two.txt");
    for timer in [false, true] {
        let mut s = Setup::new(
            6,
            if timer {
                &[Press, Release, LongPress, DoublePress]
            } else {
                &[Press, Release, DoublePress]
            },
        );
        let [down, up] = reports(text);
        s.advance(down.ms);
        s.input(down.bytes.clone());
        s.input(down.bytes.clone()); // Duplicate while down.
        s.advance(up.ms - down.ms);
        s.input(up.bytes.clone());
        assert!(s.invokes().is_empty());
        s.advance(299);
        s.input(down.bytes);
        assert_eq!(s.invokes(), [DoublePress]);
        s.input(up.bytes);
        assert!(s.invokes().is_empty());
    }
}

#[test]
fn another_control_flushes_a_window_but_another_key_cancels_a_lost_release() {
    let mut s = Setup::new(3, &[Press, Release, LongPress, DoublePress]);
    set_binding(
        &mut s.rig,
        binding(ControlKind::Knob, 2, TurnRight),
        Some(action(KIND)),
    );
    set_binding(
        &mut s.rig,
        binding(ControlKind::Key, 4, Press),
        Some(action(KIND)),
    );
    s.edge(3, 1);
    s.edge(3, 0);
    s.edge(0x51, 0);
    assert_eq!(s.invokes(), [Press, Release, TurnRight]);
    s.edge(3, 1);
    s.edge(4, 1);
    s.advance(500);
    s.edge(3, 0);
    assert_eq!(s.invokes(), [Press]);
}

#[test]
fn a_binding_edit_or_page_change_cancels_down_keys_and_open_windows() {
    for released in [false, true] {
        for page_change in [false, true] {
            let mut s = Setup::new(3, &[Press, Release, LongPress, DoublePress]);
            s.rig.engine.command(Command::AddPage { copy_of: Some(0) });
            s.rig.engine.select_page(0);
            s.edge(3, 1);
            if released {
                s.edge(3, 0);
            }
            if page_change {
                s.rig.engine.select_page(1);
            } else {
                set_binding(&mut s.rig, binding(ControlKind::Key, 3, Press), None);
            }
            s.advance(500);
            s.edge(3, 0);
            assert!(s.invokes().is_empty());
        }
    }
}

#[test]
fn a_page_change_drops_the_release_of_the_press_that_requested_it() {
    let mut s = Setup::new(3, &[Press, Release]);
    s.rig.engine.command(Command::AddPage { copy_of: Some(0) });
    s.rig.engine.select_page(0);
    s.edge(3, 1);
    assert_eq!(s.invokes(), [Press]);
    s.rig
        .engine
        .handler_message(Message::Engine(EngineRequest::SelectPage(PageTarget::Next)));
    assert_eq!(s.rig.engine.active_page(), 1);
    s.edge(3, 0);
    assert!(s.invokes().is_empty());
    s.edge(3, 1);
    s.edge(3, 0);
    assert_eq!(s.invokes(), [Press, Release]);
}

#[test]
fn folder_and_profile_changes_cancel_pending_gestures() {
    let mut s = Setup::new(3, &[Press, Release, LongPress, DoublePress]);
    let original = s.rig.engine.active_profile().to_owned();
    let folder = FolderId::from_raw(1).unwrap();
    s.rig.engine.command(Command::AddFolder {
        id: folder,
        name: Some("More".into()),
        copy_of: None,
    });
    set_binding(
        &mut s.rig,
        BindingId {
            folder: Some(folder),
            ..binding(ControlKind::Key, 3, LongPress)
        },
        Some(action(KIND)),
    );
    s.rig.engine.command(Command::CloseFolder);
    s.edge(3, 1);
    s.edge(3, 0);
    s.rig.engine.command(Command::OpenFolder { folder });
    assert_eq!(s.rig.engine.open_folders(), &[folder]);
    s.advance(300);
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.invokes().is_empty());
    s.edge(3, 1);
    s.rig.engine.command(Command::CloseFolder);
    s.advance(500);
    s.edge(3, 0);
    assert!(s.invokes().is_empty());
    s.edge(3, 1);
    s.edge(3, 0);
    s.rig.engine.command(Command::AddProfile {
        name: "Other".into(),
        copy_of: None,
    });
    assert_ne!(s.rig.engine.active_profile(), original);
    s.rig.engine.select_profile(&original);
    s.advance(300);
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.invokes().is_empty());
}

#[test]
fn paste_and_copy_to_all_pages_cancel_pending_gestures() {
    let mut s = Setup::new(3, &[Press, Release, LongPress, DoublePress]);
    let key = ControlId::new(ControlKind::Key, 3);
    s.rig.engine.command(Command::AddPage { copy_of: None });
    s.rig.engine.select_page(0);
    s.edge(3, 1);
    s.edge(3, 0);
    s.rig
        .engine
        .command(Command::CopyToAllPages { control: key });
    s.advance(300);
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.invokes().is_empty());
    for gesture in [Press, Release, LongPress, DoublePress] {
        set_binding(
            &mut s.rig,
            binding(ControlKind::Key, 4, gesture),
            Some(action(KIND)),
        );
    }
    s.rig
        .set_color(SurfaceId::new(SurfaceKind::KeyFace, 4), RED);
    s.rig.engine.command(Command::CopyControl {
        control: ControlId::new(ControlKind::Key, 4),
    });
    // Only the face changes on paste. The timer must still be cancelled.
    s.edge(3, 1);
    s.rig.engine.command(Command::PasteControl { control: key });
    s.advance(500);
    s.edge(3, 0);
    assert!(s.invokes().is_empty());
}

#[test]
fn dark_lock_and_sleep_cancel_timers_without_sticking_after_wake() {
    for (dark, wake) in [
        (
            Command::PowerNotice(PowerNotice::DisplaysSleep),
            Command::WakeDevice,
        ),
        (
            Command::PowerNotice(PowerNotice::Locked),
            Command::PowerNotice(PowerNotice::Unlocked),
        ),
        (
            Command::PowerNotice(PowerNotice::WillSleep),
            Command::PowerNotice(PowerNotice::DidWake),
        ),
    ] {
        let mut s = Setup::new(3, &[Press, Release, LongPress]);
        s.edge(3, 1);
        s.rig.engine.command(dark);
        s.advance(500);
        s.edge(3, 0);
        assert!(s.invokes().is_empty());
        s.rig.engine.command(wake);
        s.edge(3, 1);
        s.edge(3, 0);
        assert_eq!(s.invokes(), [Press, Release]);
    }
}

#[test]
fn disconnect_cancels_an_open_double_press_window() {
    let mut s = Setup::new(3, &[Press, Release, DoublePress]);
    s.edge(3, 1);
    s.edge(3, 0);
    s.device.borrow_mut().disconnect();
    s.rig.engine.poll(Duration::ZERO);
    s.advance(300);
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.invokes().is_empty());
}

#[test]
fn disconnect_or_clock_wake_at_due_deadline_cancels_without_invoking() {
    let mut s = Setup::new(3, &[Press, Release, DoublePress]);
    s.edge(3, 1);
    s.edge(3, 0);
    s.advance(300);
    s.device.borrow_mut().disconnect();
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.invokes().is_empty());

    let mut s = Setup::new(3, &[Press, Release, LongPress]);
    s.edge(3, 1);
    s.advance(500);
    let asleep = s.rig.world.asleep.get();
    s.rig.world.asleep.set(asleep + Duration::from_secs(70));
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.invokes().is_empty());
    s.edge(3, 0);
    assert!(s.invokes().is_empty());
}

#[test]
fn recorded_n4_pro_presses_match_expected_single_press_latency() {
    let fixtures: [(u8, &str); 10] = [
        (1, "01-key-row1-col1.txt"),
        (2, "02-key-row1-col2.txt"),
        (3, "03-key-row1-col3.txt"),
        (4, "04-key-row1-col4.txt"),
        (5, "05-key-row1-col5.txt"),
        (6, "06-key-row2-col1.txt"),
        (7, "07-key-row2-col2.txt"),
        (8, "08-key-row2-col3.txt"),
        (9, "09-key-row2-col4.txt"),
        (10, "10-key-row2-col5.txt"),
    ];
    let fixture_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/tests/fixtures/input");
    let parsed: Vec<(u8, Vec<Report>, u64)> = fixtures
        .into_iter()
        .map(|(key, name)| {
            let text = std::fs::read_to_string(fixture_dir.join(name)).unwrap();
            let reports = parse(&text).unwrap().reports;
            assert_eq!(reports.len(), 2, "{name}");
            let release_ms = reports[1].ms - reports[0].ms;
            (key, reports, release_ms)
        })
        .collect();

    let mut durations: Vec<u64> = parsed.iter().map(|&(_, _, d)| d).collect();
    durations.sort_unstable();
    assert_eq!(durations.first().copied(), Some(106));
    assert_eq!(durations[4], 159);
    assert_eq!(durations[5], 159);
    assert_eq!(durations.last().copied(), Some(162));

    let is_press_dispatch = |req: &Request| {
        matches!(req, Request::Invoke { binding, .. } if binding.gesture == Press)
            || matches!(req, Request::HoldStart { .. })
    };

    // One Setup per binding configuration, binding all 10 keys once, and using
    // engine deadline wakeup (`poll(10s)`) for exact timing and fast execution.
    enum Mode {
        PressOnly,
        HoldWithTimers,
        PressAndLong,
        TimerPress,
        PressAndDouble,
        PressLongAndDouble,
    }
    for mode in [
        Mode::PressOnly,
        Mode::HoldWithTimers,
        Mode::PressAndLong,
        Mode::TimerPress,
        Mode::PressAndDouble,
        Mode::PressLongAndDouble,
    ] {
        let mut s = Setup::new(1, &[]);
        for key in 1..=10u8 {
            match mode {
                Mode::PressOnly => s.bind(key, Press, Some(KIND)),
                Mode::HoldWithTimers => {
                    s.bind(key, Press, Some(HOLD));
                    s.bind(key, Release, Some(KIND));
                    s.bind(key, LongPress, Some(KIND));
                    s.bind(key, DoublePress, Some(KIND));
                }
                Mode::PressAndLong => {
                    s.bind(key, Press, Some(KIND));
                    s.bind(key, LongPress, Some(KIND));
                }
                Mode::TimerPress => s.bind(key, Press, Some(TIMER)),
                Mode::PressAndDouble => {
                    s.bind(key, Press, Some(KIND));
                    s.bind(key, DoublePress, Some(KIND));
                }
                Mode::PressLongAndDouble => {
                    s.bind(key, Press, Some(KIND));
                    s.bind(key, LongPress, Some(KIND));
                    s.bind(key, DoublePress, Some(KIND));
                }
            }
        }
        s.requests.try_iter().for_each(drop);

        for (key, reports, release_ms) in &parsed {
            let start = s.rig.world.now.get();
            s.input(reports[0].bytes.clone());
            let mut dispatched_at = s
                .requests
                .try_iter()
                .any(|req| is_press_dispatch(&req))
                .then_some(Duration::ZERO);
            s.advance(*release_ms);
            s.input(reports[1].bytes.clone());
            if dispatched_at.is_none() && s.requests.try_iter().any(|req| is_press_dispatch(&req)) {
                dispatched_at = Some(s.rig.world.now.get() - start);
            }
            if dispatched_at.is_none() {
                s.rig.engine.poll(Duration::from_secs(10));
                assert!(
                    s.requests.try_iter().any(|req| is_press_dispatch(&req)),
                    "Press never dispatched for key {key}"
                );
                dispatched_at = Some(s.rig.world.now.get() - start);
            }
            // Drain any remaining requests (e.g. HoldEnd / Release) before the next key.
            s.requests.try_iter().for_each(drop);

            let expected = match mode {
                Mode::PressOnly | Mode::HoldWithTimers => 0,
                Mode::PressAndLong | Mode::TimerPress => *release_ms,
                Mode::PressAndDouble | Mode::PressLongAndDouble => *release_ms + 300,
            };
            assert_eq!(dispatched_at.unwrap(), Duration::from_millis(expected));
        }
    }

    // Recorded 3.37 s hold triggers LongPress at exactly 500 ms while still down.
    let hold_text = std::fs::read_to_string(fixture_dir.join("12-key-hold.txt")).unwrap();
    let hold_reports = parse(&hold_text).unwrap().reports;
    let mut s = Setup::new(6, &[Press, Release, LongPress]);
    s.requests.try_iter().for_each(drop);
    let start = s.rig.world.now.get();
    s.input(hold_reports[0].bytes.clone());
    assert!(s.invokes().is_empty());
    s.advance(499);
    s.rig.engine.poll(Duration::ZERO);
    assert!(s.invokes().is_empty());
    s.rig.engine.poll(Duration::from_secs(10));
    assert_eq!(s.rig.world.now.get() - start, Duration::from_millis(500));
    assert_eq!(s.invokes(), [LongPress]);
    s.advance(hold_reports[1].ms - 500);
    s.input(hold_reports[1].bytes.clone());
    assert!(s.invokes().is_empty());
}
