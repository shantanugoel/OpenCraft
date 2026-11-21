use super::*;
use opencraft_core::recording::{parse, Report};
use opencraft_engine::{EngineRequest, FolderId, PageTarget, PowerNotice};
use GestureKind::{DoublePress, LongPress, Press, Release, TurnRight};

const KIND: &str = "test.gestures";

struct Setup {
    rig: Rig,
    device: Rc<RefCell<FakeTransport>>,
    requests: mpsc::Receiver<Request>,
}

impl Setup {
    fn new(key: u8, gestures: &[GestureKind]) -> Self {
        let (port, requests) = port(KIND);
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
