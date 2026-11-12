//! Binding and handler tests. Only the runtime test starts handler threads.

use super::*;

#[test]
fn a_new_kind_dispatches_settings_without_engine_changes() {
    let kind = "test.a-new-kind";
    let (port, requests) = port(kind);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    let events = rig.take_events();
    assert_eq!(events[1], Event::Kinds(vec![description(kind)]));
    let id = binding(ControlKind::Key, 3, GestureKind::Press);
    let action = action(kind);
    set_binding(&mut rig, id, Some(action.clone()));
    assert_eq!(
        requests.try_recv().unwrap(),
        Request::Appear {
            binding: id,
            action: action.clone()
        }
    );
    let device = rig.connect();
    rig.take_events();
    for (code, state) in [(3, 1), (3, 0), (4, 1)] {
        device.borrow_mut().push_input(control_report(code, state));
        rig.engine.poll(POLL_INTERVAL);
    }
    assert_eq!(
        requests.try_recv().unwrap(),
        Request::Invoke {
            binding: id,
            action
        }
    );
    assert!(requests.try_recv().is_err());
    assert_eq!(
        rig.take_events()
            .iter()
            .filter(|e| matches!(e, Event::Input(_)))
            .count(),
        3
    );
    assert_eq!(
        rig.engine.counters(),
        Counters {
            reports_received: 3,
            input_events: 3,
            invokes_sent: 1
        }
    );
}

#[test]
fn each_phase_three_gesture_dispatches_once() {
    let kind = "test.all-gestures";
    let (port, requests) = port(kind);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    let cases = [
        (binding(ControlKind::Key, 3, GestureKind::Press), 3, 1),
        (binding(ControlKind::Key, 3, GestureKind::Release), 3, 0),
        (
            binding(ControlKind::Knob, 2, GestureKind::TurnLeft),
            0x50,
            0,
        ),
        (
            binding(ControlKind::Knob, 2, GestureKind::TurnRight),
            0x51,
            0,
        ),
        (binding(ControlKind::Knob, 2, GestureKind::Push), 0x35, 1),
    ];
    for (id, _, _) in cases {
        set_binding(&mut rig, id, Some(action(kind)));
    }
    assert_eq!(requests.try_iter().count(), 5);
    let device = rig.connect();
    for (_, code, state) in cases {
        device.borrow_mut().push_input(control_report(code, state));
    }
    for (id, _, _) in cases {
        rig.engine.poll(POLL_INTERVAL);
        assert_eq!(
            requests.try_recv().unwrap(),
            Request::Invoke {
                binding: id,
                action: action(kind)
            }
        );
    }
    assert!(requests.try_recv().is_err());
}

/// The reports of a recording of the real device. The recordings are the
/// fixtures of the input decoder in `crates/core`.
fn recorded(name: &str) -> Vec<Vec<u8>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/input")
        .join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    opencraft_core::recording::parse(&text)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
        .reports
        .into_iter()
        .map(|r| r.bytes)
        .collect()
}

#[test]
fn each_recorded_tap_and_swipe_invokes_its_binding_once() {
    let kind = "test.touch";
    let (port, requests) = port(kind);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    let tap = |number| binding(ControlKind::BarPanel, number, GestureKind::Tap);
    let swipe = |gesture| binding(ControlKind::TouchBar, 1, gesture);
    let (left, right) = (
        swipe(GestureKind::SwipeLeft),
        swipe(GestureKind::SwipeRight),
    );
    // The recording, the binding that it invokes, and the number of invokes.
    let cases = [
        ("26-bar-tap-1.txt", tap(1), 1),
        ("27-bar-tap-2.txt", tap(2), 1),
        ("28-bar-tap-3.txt", tap(3), 1),
        ("29-bar-tap-4.txt", tap(4), 1),
        ("35-swipe-left-20.txt", left, 20),
        ("36-swipe-right-20.txt", right, 20),
        ("34-swipe-right-1.txt", right, 1),
        // The device did not identify this swipe: the touch ends with no
        // gesture, and nothing is invoked.
        ("33-swipe-left-1.txt", left, 0),
    ];
    for id in [tap(1), tap(2), tap(3), tap(4), left, right] {
        set_binding(&mut rig, id, Some(action(kind)));
    }
    assert_eq!(requests.try_iter().count(), 6, "one Appear each");
    let device = rig.connect();
    for (name, id, invokes) in cases {
        // The touch start and the touch points of the recording invoke
        // nothing. Only the gesture that the device identifies does.
        let reports = recorded(name);
        let count = reports.len();
        for report in reports {
            device.borrow_mut().push_input(report);
        }
        for _ in 0..count {
            rig.engine.poll(POLL_INTERVAL);
        }
        let want = Request::Invoke {
            binding: id,
            action: action(kind),
        };
        assert_eq!(
            requests.try_iter().collect::<Vec<_>>(),
            vec![want; invokes],
            "{name}"
        );
    }
    assert_eq!(rig.engine.counters().invokes_sent, 45);
}

#[test]
fn five_hundred_knob_reports_keep_every_input_and_invoke_in_order() {
    let kind = "test.knob";
    let (port, requests) = port(kind);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    let left = binding(ControlKind::Knob, 2, GestureKind::TurnLeft);
    let right = binding(ControlKind::Knob, 2, GestureKind::TurnRight);
    for id in [left, right] {
        set_binding(&mut rig, id, Some(action(kind)));
    }
    assert_eq!(requests.try_iter().count(), 2);
    let device = rig.connect();
    rig.take_events();
    let expected: Vec<_> = (0..500)
        .map(|i| if i % 2 == 0 { left } else { right })
        .collect();
    for id in &expected {
        device
            .borrow_mut()
            .push_input(control_report(if *id == left { 0x50 } else { 0x51 }, 0));
    }
    for _ in &expected {
        rig.engine.poll(POLL_INTERVAL);
    }
    let inputs: Vec<_> = rig
        .take_events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Input(event) => Some(BindingId {
                page: 0,
                control: event.control,
                gesture: GestureKind::from_gesture(event.gesture).unwrap(),
            }),
            _ => None,
        })
        .collect();
    assert_eq!(inputs, expected);
    let invoked: Vec<_> = requests
        .try_iter()
        .map(|request| match request {
            Request::Invoke {
                binding,
                action: settings,
            } => {
                assert_eq!(settings, action(kind));
                binding
            }
            other => panic!("unexpected request: {other:?}"),
        })
        .collect();
    assert_eq!(invoked, expected);
    assert_eq!(
        rig.engine.counters(),
        Counters {
            reports_received: 500,
            input_events: 500,
            invokes_sent: 500
        }
    );
    rig.engine.command(Command::ReadCounters);
    assert_eq!(rig.take_events(), [Event::Counters(rig.engine.counters())]);
}

#[test]
fn an_unavailable_kind_stays_in_the_file_and_reports_its_state() {
    let mut rig = Rig::new();
    rig.take_events();
    let id = binding(ControlKind::Key, 3, GestureKind::Press);
    let action = action("missing.handler");
    set_binding(&mut rig, id, Some(action.clone()));
    let expected = Event::Binding(BindingState {
        binding: id,
        action: Some(action.clone()),
        available: false,
    });
    assert_eq!(
        rig.take_events().as_slice(),
        std::slice::from_ref(&expected)
    );
    rig.engine.finish();
    let mut rig = rig.restart();
    assert_eq!(
        rig.engine.profile().pages()[0].binding(id.control, id.gesture),
        Some(&action)
    );
    assert!(rig.take_events().contains(&expected));
    let device = rig.connect();
    device.borrow_mut().push_input(control_report(3, 1));
    rig.engine.poll(POLL_INTERVAL);
    assert_eq!(rig.engine.counters().invokes_sent, 0);
}

#[test]
fn appear_and_disappear_follow_start_set_change_and_remove() {
    let kind = "test.lifecycle";
    let dir = TempDir::new().unwrap();
    let id = binding(ControlKind::Key, 3, GestureKind::Press);
    let first = action(kind);
    let mut profile = Profile::default();
    profile
        .page_mut(0)
        .unwrap()
        .set_binding(id.control, id.gesture, Some(first.clone()));
    store_in(dir.path()).save(&profile).unwrap();
    let (port, requests) = port(kind);
    let mut rig = Rig::with_ports(dir, vec![port]);
    assert_eq!(
        requests.try_recv().unwrap(),
        Request::Appear {
            binding: id,
            action: first.clone()
        }
    );
    assert!(rig.take_events().contains(&Event::Binding(BindingState {
        binding: id,
        action: Some(first.clone()),
        available: true
    })));
    let mut second = first.clone();
    second
        .settings
        .insert("changed".into(), Value::Boolean(true));
    set_binding(&mut rig, id, Some(second.clone()));
    assert_eq!(
        requests.try_recv().unwrap(),
        Request::Disappear { binding: id }
    );
    assert_eq!(
        requests.try_recv().unwrap(),
        Request::Appear {
            binding: id,
            action: second.clone()
        }
    );
    set_binding(&mut rig, id, Some(second));
    assert!(requests.try_recv().is_err());
    set_binding(&mut rig, id, None);
    assert_eq!(
        requests.try_recv().unwrap(),
        Request::Disappear { binding: id }
    );
    assert!(rig.take_events().contains(&Event::Binding(BindingState {
        binding: id,
        action: None,
        available: false
    })));
    set_binding(&mut rig, id, Some(first.clone()));
    assert_eq!(
        requests.try_recv().unwrap(),
        Request::Appear {
            binding: id,
            action: first
        }
    );
    assert!(requests.try_recv().is_err());
}

#[test]
fn status_resolve_and_problem_use_the_same_interface() {
    let kind = "test.status";
    let (port, requests) = port(kind);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    rig.take_events();
    let status = Status::NeedsUser {
        text: "Permit access.".into(),
        button: "Continue".into(),
    };
    rig.engine.handler_message(Message::Status {
        kind: kind.into(),
        status: status.clone(),
    });
    rig.engine
        .handler_message(Message::Problem("Cannot use the settings.".into()));
    assert_eq!(
        rig.take_events(),
        [
            Event::Status {
                kind: kind.into(),
                status
            },
            Event::Problem("Cannot use the settings.".into())
        ]
    );
    rig.engine.command(Command::Resolve { kind: kind.into() });
    assert_eq!(
        requests.try_recv().unwrap(),
        Request::Resolve { kind: kind.into() }
    );
    rig.engine.command(Command::Resolve {
        kind: "missing.kind".into(),
    });
    assert!(requests.try_recv().is_err());
    rig.engine.handler_message(Message::Status {
        kind: kind.into(),
        status: Status::Ready,
    });
    assert_eq!(
        rig.take_events(),
        [Event::Status {
            kind: kind.into(),
            status: Status::Ready
        }]
    );
}

fn feedback(rig: &mut Rig, binding: BindingId, label: &str, level: u8) {
    rig.engine.handler_message(Message::Feedback {
        binding,
        feedback: Some(Feedback {
            label: Some(label.into()),
            level: Some(level),
            state: None,
        }),
    });
}

#[test]
fn knob_feedback_uses_its_bar_panel_and_survives_a_connection() {
    let kind = "test.feedback";
    let (port, _requests) = port(kind);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    let id = binding(ControlKind::Knob, 2, GestureKind::TurnRight);
    set_binding(&mut rig, id, Some(action(kind)));
    rig.engine.command(Command::SetLabel {
        surface: BAR_2,
        label: Some("Original".into()),
    });
    let original = last_state(&rig.take_events(), BAR_2).image.clone();
    feedback(&mut rig, id, "Volume", 50);
    let events = rig.take_events();
    assert_eq!(events.len(), 1);
    let state = last_state(&events, BAR_2);
    assert_ne!(state.image, original);
    assert_eq!(state.content.label.as_deref(), Some("Original"));
    let device = rig.connect();
    assert_same_picture(&shown_at(&sent(&device), 2), &state.image);
    rig.take_events();
    feedback(&mut rig, id, "Louder", 80);
    let image = last_state(&rig.take_events(), BAR_2).image.clone();
    let writes = sent(&device);
    assert_eq!(writes.len(), 2);
    assert_same_picture(&shown_at(&writes, 2), &image);
    device.borrow_mut().disconnect();
    rig.poll_for(Duration::from_secs(1));
    let device = rig.connect();
    assert_same_picture(&shown_at(&sent(&device), 2), &image);
    rig.take_events();
    set_binding(&mut rig, id, None);
    assert_eq!(last_state(&rig.take_events(), BAR_2).image, original);
    assert_same_picture(&shown_at(&sent(&device), 2), &original);
    rig.engine.handler_message(Message::Feedback {
        binding: id,
        feedback: Some(Feedback::default()),
    });
    assert!(rig.take_events().is_empty());
}

#[test]
fn newest_feedback_wins_and_removal_restores_the_previous_binding() {
    let kind = "test.shared-surface";
    let (port, _requests) = port(kind);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    let left = binding(ControlKind::Knob, 2, GestureKind::TurnLeft);
    let right = binding(ControlKind::Knob, 2, GestureKind::TurnRight);
    for id in [left, right] {
        set_binding(&mut rig, id, Some(action(kind)));
    }
    rig.take_events();
    feedback(&mut rig, left, "Left", 20);
    let first = last_state(&rig.take_events(), BAR_2).image.clone();
    feedback(&mut rig, right, "Right", 90);
    let second = last_state(&rig.take_events(), BAR_2).image.clone();
    assert_ne!(first, second);
    rig.engine.handler_message(Message::Feedback {
        binding: right,
        feedback: None,
    });
    assert_eq!(last_state(&rig.take_events(), BAR_2).image, first);
    let mut changed = action(kind);
    changed.settings.clear();
    set_binding(&mut rig, left, Some(changed));
    assert!(last_state(&rig.take_events(), BAR_2)
        .image
        .pixels()
        .all(|p| *p == Rgb([0, 0, 0])));
    rig.engine.finish();
    assert!(!rig.profile_text().unwrap().contains("Left"));
}

#[test]
fn tap_feedback_shows_on_the_bar_panel_of_the_control() {
    let kind = "test.tap-feedback";
    let (port, _requests) = port(kind);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    let id = binding(ControlKind::BarPanel, 2, GestureKind::Tap);
    set_binding(&mut rig, id, Some(action(kind)));
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    feedback(&mut rig, id, "Tapped", 60);
    let events = rig.take_events();
    assert_eq!(events.len(), 1, "{events:?}");
    let state = last_state(&events, BAR_2);
    assert!(state.image.pixels().any(|p| *p != Rgb([0, 0, 0])));
    let writes = sent(&device);
    assert_eq!(writes.len(), 2, "one image and the commit");
    assert_same_picture(&shown_at(&writes, 2), &state.image);
}

#[test]
fn feedback_ignores_an_inactive_page_and_a_control_with_no_surface() {
    let mut rig = Rig::new();
    let mut id = binding(ControlKind::TouchBar, 1, GestureKind::SwipeLeft);
    set_binding(&mut rig, id, Some(action("test.no-surface")));
    rig.take_events();
    feedback(&mut rig, id, "No display", 20);
    assert!(rig.take_events().is_empty());
    id = binding(ControlKind::Knob, 2, GestureKind::TurnRight);
    set_binding(&mut rig, id, Some(action("test.inactive")));
    rig.take_events();
    id.page = 1;
    feedback(&mut rig, id, "Inactive", 20);
    assert!(rig.take_events().is_empty());
}

#[test]
fn a_version_one_file_waits_for_a_change_then_writes_the_current_version() {
    let dir = TempDir::new().unwrap();
    let store = store_in(dir.path());
    fs::create_dir_all(store.dir()).unwrap();
    let old = "schema_version = 1\nname = \"Old\"\n[[pages]]\nname = \"Page\"\n[pages.surfaces.key-face-3]\nlabel = \"Keep\"\n";
    fs::write(store.profile_path(), old).unwrap();
    let mut rig = Rig::in_dir(dir);
    rig.poll_for(Duration::from_secs(2));
    assert_eq!(rig.profile_text().unwrap(), old);
    assert_eq!(
        rig.engine.profile().pages()[0]
            .content(KEY_3)
            .label
            .as_deref(),
        Some("Keep")
    );
    let id = binding(ControlKind::Key, 3, GestureKind::Press);
    let mut action = action("test.settings");
    action.settings.extend([
        ("empty-list".into(), Value::List(vec![])),
        ("empty-table".into(), Value::Table(Settings::new())),
        ("boolean".into(), Value::Boolean(false)),
        ("integer-min".into(), Value::Integer(i64::MIN)),
        ("integer-max".into(), Value::Integer(i64::MAX)),
        (
            "list".into(),
            Value::List(vec![
                Value::Text("text\nwith quotes \"".into()),
                Value::Integer(-7),
                Value::Boolean(true),
                Value::Table(Settings::from([(
                    "nested".into(),
                    Value::List(vec![Value::Integer(3)]),
                )])),
            ]),
        ),
        (
            "table".into(),
            Value::Table(Settings::from([(
                "key".into(),
                Value::Text("value".into()),
            )])),
        ),
    ]);
    set_binding(&mut rig, id, Some(action.clone()));
    rig.engine.finish();
    let text = rig.profile_text().unwrap();
    assert!(text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")));
    assert!(text.contains("[pages.bindings.key-3.press]"));
    let rig = rig.restart();
    assert_eq!(
        rig.engine.profile().pages()[0].binding(id.control, id.gesture),
        Some(&action)
    );
}

#[test]
fn a_kind_change_routes_lifecycle_requests_to_the_correct_handlers() {
    let (first, first_requests) = port("test.first");
    let (second, second_requests) = port("test.second");
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![first, second]);
    let id = binding(ControlKind::Key, 3, GestureKind::Press);
    set_binding(&mut rig, id, Some(action("test.first")));
    assert_eq!(
        first_requests.try_recv().unwrap(),
        Request::Appear {
            binding: id,
            action: action("test.first")
        }
    );
    set_binding(&mut rig, id, Some(action("test.second")));
    assert_eq!(
        first_requests.try_recv().unwrap(),
        Request::Disappear { binding: id }
    );
    assert_eq!(
        second_requests.try_recv().unwrap(),
        Request::Appear {
            binding: id,
            action: action("test.second")
        }
    );
    assert!(first_requests.try_recv().is_err());
    set_binding(&mut rig, id, Some(action("missing.kind")));
    assert_eq!(
        second_requests.try_recv().unwrap(),
        Request::Disappear { binding: id }
    );
    assert!(second_requests.try_recv().is_err());
}

#[test]
fn bindings_of_an_inactive_page_have_no_state_and_no_requests() {
    let kind = "test.inactive-page";
    let dir = TempDir::new().unwrap();
    let mut inactive = opencraft_engine::Page::new("Inactive");
    let mut id = binding(ControlKind::Key, 3, GestureKind::Press);
    id.page = 1;
    inactive.set_binding(id.control, id.gesture, Some(action(kind)));
    store_in(dir.path())
        .save(&Profile::new(
            "Two pages",
            vec![opencraft_engine::Page::new("Active"), inactive],
        ))
        .unwrap();
    let (port, requests) = port(kind);
    let mut rig = Rig::with_ports(dir, vec![port]);
    assert!(!rig
        .take_events()
        .iter()
        .any(|e| matches!(e, Event::Binding(_))));
    assert!(requests.try_recv().is_err());

    // A command for a binding of an inactive page changes nothing.
    set_binding(&mut rig, id, None);
    assert_eq!(problems(&rig.take_events()).len(), 1);
    assert!(requests.try_recv().is_err());
    assert_eq!(
        rig.engine.profile().pages()[1].binding(id.control, id.gesture),
        Some(&action(kind))
    );
    id.page = 10;
    set_binding(&mut rig, id, Some(action(kind)));
    assert_eq!(problems(&rig.take_events()).len(), 1);
}

#[test]
fn a_busy_message_queue_does_not_stop_device_polls() {
    let mut rig = Rig::new();
    let device = rig.plug();
    rig.take_events();
    device.borrow_mut().push_input(control_report(3, 1));
    let (commands, receiver) = mpsc::channel();
    let (sender, messages) = mpsc::channel();
    for _ in 0..1000 {
        sender
            .send(Message::Status {
                kind: "test.busy".into(),
                status: Status::Ready,
            })
            .unwrap();
    }
    let events = rig.events.clone();
    // This fake clock runs without a wait. Its next poll gives the input.
    // The sink asks the runtime to stop as soon as it sees that input.
    let link = Link::with_clock(
        FakeConnector {
            queue: rig.queue.clone(),
            refusals: rig.refusals.clone(),
        },
        FakeClock {
            world: rig.world.clone(),
        },
    );
    rig.engine = Engine::new(
        link,
        ProfilesDir::new(rig.dir.path()),
        &DESCRIPTORS[0],
        Box::new(move |event| {
            if matches!(event, Event::Input(_)) {
                commands.send(Command::Stop).unwrap();
            }
            events.borrow_mut().push(event);
        }),
    );
    runtime::run_with_handlers(&mut rig.engine, &receiver, &messages);
    assert!(
        messages.try_recv().is_ok(),
        "the input arrives before the queue is empty"
    );
    assert_eq!(rig.engine.counters().input_events, 1);
}

struct BlockingHandler {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    stopped: mpsc::Sender<()>,
    starts: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl Handler for BlockingHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![description("test.blocking")]
    }
    fn start(&mut self, sender: MessageSender) {
        self.starts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        sender
            .clone()
            .send(Message::Problem("Handler starts.".into()))
            .unwrap();
    }
    fn handle(&mut self, _: Request) {
        self.entered.send(()).unwrap();
        self.release.recv_timeout(Duration::from_secs(2)).unwrap();
    }
}

impl Drop for BlockingHandler {
    fn drop(&mut self) {
        let _ = self.stopped.send(());
    }
}

struct QuickHandler(Option<MessageSender>);

impl Handler for QuickHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![description("test.quick")]
    }
    fn start(&mut self, sender: MessageSender) {
        self.0 = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        if let Request::Resolve { kind } = request {
            self.0
                .as_ref()
                .unwrap()
                .send(Message::Status {
                    kind,
                    status: Status::Ready,
                })
                .unwrap();
        }
    }
}

#[test]
fn duplicate_kind_registration_fails_before_handlers_start() {
    let error = runtime::start_handlers(vec![
        Box::new(QuickHandler(None)),
        Box::new(QuickHandler(None)),
    ])
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn a_blocked_handler_does_not_stop_the_runtime_or_another_handler() {
    let (entered, entry) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let (stopped, stop) = mpsc::channel();
    let starts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let handler = BlockingHandler {
        entered,
        release: wait,
        stopped,
        starts: starts.clone(),
    };
    let (ports, messages) =
        runtime::start_handlers(vec![Box::new(handler), Box::new(QuickHandler(None))]).unwrap();
    let mut rig = Rig::new();
    let (events, output) = mpsc::channel();
    rig.engine = Engine::with_handlers(
        Link::with_clock(
            FakeConnector {
                queue: rig.queue.clone(),
                refusals: rig.refusals.clone(),
            },
            FakeClock {
                world: rig.world.clone(),
            },
        ),
        ProfilesDir::new(rig.dir.path()),
        &DESCRIPTORS[0],
        ports,
        Box::new(move |event| {
            let _ = events.send(event);
        }),
    );
    let id = binding(ControlKind::Key, 3, GestureKind::Press);
    set_binding(&mut rig, id, Some(action("test.blocking")));
    entry.recv_timeout(Duration::from_secs(1)).unwrap();
    let (commands, receiver) = mpsc::channel();
    commands
        .send(Command::Resolve {
            kind: "test.quick".into(),
        })
        .unwrap();
    commands.send(Command::ReadCounters).unwrap();
    // The fake engine stays on this test thread. Only the handler blocks.
    let watchdog = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(1);
        let (mut counters, mut problem, mut status) = (false, false, false);
        while !(counters && problem && status) {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            match output.recv_timeout(left) {
                Ok(Event::Counters(_)) => counters = true,
                Ok(Event::Problem(text)) if text == "Handler starts." => problem = true,
                Ok(Event::Status {
                    kind,
                    status: Status::Ready,
                }) if kind == "test.quick" => status = true,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = commands.send(Command::Stop);
        let _ = release.send(());
        counters && problem && status
    });
    runtime::run_with_handlers(&mut rig.engine, &receiver, &messages);
    assert!(watchdog.join().unwrap());
    assert_eq!(starts.load(std::sync::atomic::Ordering::SeqCst), 1);
    drop(rig);
    stop.recv_timeout(Duration::from_secs(1)).unwrap();
}

// --- Requests of a handler for an action, and the state feedback --------------

#[test]
fn a_handler_asks_the_engine_to_do_an_action_with_its_own_binding() {
    let (outer, outer_requests) = port("test.outer");
    let (inner, inner_requests) = port("test.inner");
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![outer, inner]);
    let id = binding(ControlKind::Key, 3, GestureKind::Press);
    set_binding(&mut rig, id, Some(action("test.outer")));
    outer_requests.try_iter().count();
    rig.take_events();

    let request = |action| {
        Message::Engine(opencraft_engine::EngineRequest::Invoke {
            binding: id,
            action,
        })
    };
    rig.engine.handler_message(request(action("test.inner")));
    assert_eq!(
        inner_requests.try_iter().collect::<Vec<_>>(),
        [Request::Invoke {
            binding: id,
            action: action("test.inner")
        }]
    );
    assert!(outer_requests.try_recv().is_err());
    assert!(rig.take_events().is_empty());
    // It is not input: the counters do not change.
    assert_eq!(rig.engine.counters(), Counters::default());

    // A kind with no handler is a problem for the shell.
    rig.engine.handler_message(request(action("missing.kind")));
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 1, "{events:?}");
    assert!(problems(&events)[0].contains("missing.kind"));
    assert!(!problems(&events)[0].contains("handler"));
    assert!(inner_requests.try_recv().is_err());
}

#[test]
fn feedback_with_the_state_on_draws_a_frame_and_off_removes_it() {
    let kind = "test.state";
    let (port, _requests) = port(kind);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    let id = binding(ControlKind::Key, 3, GestureKind::Press);
    set_binding(&mut rig, id, Some(action(kind)));
    rig.set_color(KEY_3, RED);
    let device = rig.connect();
    sent(&device);
    rig.take_events();
    let state = |rig: &mut Rig, state| {
        rig.engine.handler_message(Message::Feedback {
            binding: id,
            feedback: Some(Feedback {
                label: None,
                level: None,
                state: Some(state),
            }),
        });
    };

    state(&mut rig, opencraft_engine::FeedbackState::On);
    let image = last_state(&rig.take_events(), KEY_3).image.clone();
    // The frame has the colour of the label: white on red.
    for (x, y) in [(1, 1), (56, 2), (110, 56), (56, 110)] {
        assert_color(&image, x, y, Color::WHITE);
    }
    assert_color(&image, 56, 56, RED);
    let writes = sent(&device);
    assert_eq!(writes.len(), 2, "one image and the commit");
    assert_same_picture(&shown_at(&writes, KEY_3_SLOT), &image);

    state(&mut rig, opencraft_engine::FeedbackState::Off);
    let image = last_state(&rig.take_events(), KEY_3).image.clone();
    assert_color(&image, 1, 1, RED);
    assert_color(&shown_at(&sent(&device), KEY_3_SLOT), 1, 1, RED);
}
