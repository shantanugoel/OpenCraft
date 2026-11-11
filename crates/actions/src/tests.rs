use crate::handlers::{build, Intervals};
use crate::{handlers, handlers_with_system};
use opencraft_engine::{
    Feedback, Handler, InputType, Message, PathKind, Request, Settings, Status, Value,
};
use std::collections::BTreeSet;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[path = "../tests/support/mod.rs"]
mod support;
use support::*;

struct Rig {
    system: Arc<Fake>,
    handler: Box<dyn Handler>,
    messages: Receiver<Message>,
    initial: Vec<Message>,
}

impl Rig {
    fn new(kind: &str, permission: bool) -> Self {
        Self::with_volume_interval(kind, permission, Duration::from_secs(60))
    }
    fn polling() -> Self {
        Self::with_volume_interval("volume", true, Duration::from_millis(10))
    }
    fn with_volume_interval(kind: &str, permission: bool, volume: Duration) -> Self {
        let system = Arc::new(Fake::default());
        system.permission.store(permission, Ordering::SeqCst);
        let mut handler = build(
            system.clone(),
            Intervals {
                volume,
                permission: Duration::from_millis(10),
                permission_limit: Duration::from_millis(100),
            },
        )
        .into_iter()
        .find(|h| {
            h.kinds()
                .iter()
                .any(|k| k.kind == format!("opencraft.{kind}"))
        })
        .unwrap();
        let (sender, messages) = mpsc::channel();
        handler.start(sender);
        let initial = messages.try_iter().collect();
        Self {
            system,
            handler,
            messages,
            initial,
        }
    }
    fn invoke(&mut self, action: opencraft_engine::Action) {
        self.handler.handle(invoke(action));
    }
    fn recv(&self) -> Message {
        self.messages
            .recv_timeout(Duration::from_millis(500))
            .unwrap()
    }
    fn problem(&self, contains: &str) {
        match self.recv() {
            Message::Problem(text) => assert!(text.contains(contains), "{text}"),
            other => panic!("expected a Problem, got {other:?}"),
        }
    }
    fn feedback(&self, index: u8, level: u8, muted: bool) {
        assert_eq!(
            self.recv(),
            Message::Feedback {
                binding: binding(index),
                feedback: Some(Feedback {
                    level: Some(level),
                    label: muted.then(|| "Muted".into()),
                }),
            }
        );
    }
    fn appear(&mut self, index: u8) {
        self.handler.handle(Request::Appear {
            binding: binding(index),
            action: action("volume", &[]),
        });
    }
    fn disappear(&mut self, index: u8) {
        self.handler.handle(Request::Disappear {
            binding: binding(index),
        });
    }
}

fn statuses(messages: Vec<Message>, ready: bool) {
    assert_eq!(messages.len(), 3, "{messages:?}");
    let mut kinds = BTreeSet::new();
    for message in messages {
        match message {
            Message::Status { kind, status } => {
                assert!(kinds.insert(kind));
                if ready {
                    assert_eq!(status, Status::Ready);
                } else {
                    let Status::NeedsUser { text, button } = status else {
                        panic!("expected NeedsUser")
                    };
                    assert!(text.contains("Accessibility"));
                    assert!(!button.is_empty());
                }
            }
            other => panic!("expected a status, got {other:?}"),
        }
    }
    assert_eq!(
        kinds,
        [
            "opencraft.hotkey",
            "opencraft.type-text",
            "opencraft.media-key"
        ]
        .map(str::to_string)
        .into()
    );
}

#[test]
fn each_text_kind_does_its_work() {
    for (kind, key, value, expected) in [
        (
            "open-application",
            "application",
            "/Applications/Test.app",
            Call::Application("/Applications/Test.app".into()),
        ),
        (
            "open-url",
            "url",
            "https://example.test",
            Call::Url("https://example.test".into()),
        ),
        (
            "hotkey",
            "hotkey",
            "shift+cmd+4",
            Call::Hotkey("shift+cmd+4".into()),
        ),
        (
            "type-text",
            "text",
            "A line\nA second line 🦀",
            Call::Text("A line\nA second line 🦀".into()),
        ),
        (
            "shell-command",
            "command",
            "exit 0",
            Call::Shell("exit 0".into()),
        ),
    ] {
        let mut rig = Rig::new(kind, true);
        rig.invoke(text_action(kind, key, value));
        wait_until(|| !rig.system.calls().is_empty());
        assert_eq!(rig.system.calls(), [expected]);
        assert!(rig.messages.try_recv().is_err());
    }
}

#[test]
fn each_media_key_has_the_correct_system_key() {
    let mut rig = Rig::new("media-key", true);
    for (value, key) in [
        ("play-pause", crate::system::MediaKey::PlayPause),
        ("next", crate::system::MediaKey::Next),
        ("previous", crate::system::MediaKey::Previous),
        ("volume-up", crate::system::MediaKey::VolumeUp),
        ("volume-down", crate::system::MediaKey::VolumeDown),
        ("mute", crate::system::MediaKey::Mute),
    ] {
        rig.invoke(text_action("media-key", "key", value));
        assert_eq!(rig.system.calls().last(), Some(&Call::Media(key)));
    }
    assert!(rig.messages.try_recv().is_err());
}

#[test]
fn missing_empty_and_wrong_type_text_settings_are_problems_without_work() {
    for (kind, key) in [
        ("open-application", "application"),
        ("open-url", "url"),
        ("hotkey", "hotkey"),
        ("type-text", "text"),
        ("media-key", "key"),
        ("shell-command", "command"),
    ] {
        let mut rig = Rig::new(kind, true);
        rig.invoke(action(kind, &[]));
        rig.problem(key);
        for bad in [
            Value::Text("".into()),
            Value::Integer(3),
            Value::Boolean(true),
            Value::List(vec![]),
            Value::Table(Settings::new()),
        ] {
            rig.invoke(action(kind, &[(key, bad)]));
            rig.problem(key);
        }
        assert!(rig.system.calls().is_empty(), "{kind}");
    }
}

#[test]
fn incorrect_choices_hotkeys_and_blank_open_or_command_settings_do_no_work() {
    for (kind, key, bad) in [
        ("media-key", "key", "play"),
        ("hotkey", "hotkey", "cmd+unknown"),
        ("hotkey", "hotkey", "cmd"),
        ("open-application", "application", "  "),
        ("open-url", "url", "\n"),
        ("shell-command", "command", " \n "),
    ] {
        let mut rig = Rig::new(kind, true);
        rig.invoke(text_action(kind, key, bad));
        rig.problem("");
        assert!(rig.system.calls().is_empty());
    }
}

#[test]
fn text_keeps_whitespace_and_extra_settings_do_not_break_an_action() {
    let mut rig = Rig::new("type-text", true);
    rig.invoke(action(
        "type-text",
        &[
            ("text", Value::Text(" \n ".into())),
            ("extra", Value::Boolean(false)),
        ],
    ));
    assert_eq!(rig.system.calls(), [Call::Text(" \n ".into())]);
}

#[test]
fn system_errors_become_problems_for_each_kind() {
    for (kind, key, value, failure) in [
        ("open-application", "application", "Test", "application"),
        ("open-url", "url", "https://example.test", "url"),
        ("hotkey", "hotkey", "cmd+a", "hotkey"),
        ("type-text", "text", "Test", "text"),
        ("media-key", "key", "mute", "media"),
        ("shell-command", "command", "exit 0", "shell"),
    ] {
        let mut rig = Rig::new(kind, true);
        rig.system.fail(Some(failure));
        rig.invoke(text_action(kind, key, value));
        rig.problem("42");
    }
    for failure in ["volume", "level", "muted"] {
        let mut rig = Rig::new("volume", true);
        rig.system.fail(Some(failure));
        let operation = if failure == "muted" { "mute" } else { "up" };
        rig.invoke(text_action("volume", "operation", operation));
        rig.problem("42");
    }
}

#[test]
fn volume_defaults_are_up_and_five_and_invoke_sends_feedback_without_appear() {
    let mut rig = Rig::new("volume", true);
    rig.invoke(action("volume", &[]));
    assert_eq!(rig.system.calls(), [Call::Level(55)]);
    rig.feedback(1, 55, false);
    // Invoke alone must not start periodic reads.
    let reads = rig.system.volume_reads.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(rig.system.volume_reads.load(Ordering::SeqCst), reads);
}

#[test]
fn incorrect_volume_settings_are_problems_without_changes() {
    let mut rig = Rig::new("volume", true);
    for (key, bad) in [
        ("operation", Value::Text("sideways".into())),
        ("operation", Value::Text("".into())),
        ("operation", Value::Integer(1)),
        ("step", Value::Text("5".into())),
        ("step", Value::Boolean(true)),
        ("step", Value::Integer(0)),
        ("step", Value::Integer(26)),
        ("step", Value::Integer(i64::MIN)),
        ("step", Value::Integer(i64::MAX)),
    ] {
        rig.invoke(action("volume", &[(key, bad)]));
        rig.problem("volume");
        rig.feedback(1, 50, false);
        assert!(rig.system.calls().is_empty());
    }
}

#[test]
fn volume_up_down_clamp_and_unmute_and_mute_toggles() {
    let mut rig = Rig::new("volume", true);
    for (operation, initial, step, expected) in [("up", 98, 25, 100), ("down", 2, 5, 0)] {
        rig.system.set_volume(initial, true);
        rig.system.clear_calls();
        rig.invoke(action(
            "volume",
            &[
                ("operation", Value::Text(operation.into())),
                ("step", Value::Integer(step)),
            ],
        ));
        assert_eq!(
            rig.system.calls(),
            [Call::Level(expected), Call::Muted(false)]
        );
        rig.feedback(1, expected, false);
    }
    rig.system.set_volume(61, false);
    rig.system.clear_calls();
    rig.invoke(text_action("volume", "operation", "mute"));
    rig.feedback(1, 0, true);
    rig.invoke(text_action("volume", "operation", "mute"));
    rig.feedback(1, 61, false);
    assert_eq!(rig.system.calls(), [Call::Muted(true), Call::Muted(false)]);
}

#[test]
fn volume_appear_polls_only_changes_for_each_binding_and_stops_after_last_disappear() {
    let mut rig = Rig::polling();
    assert_eq!(rig.system.volume_reads.load(Ordering::SeqCst), 0);
    rig.appear(1);
    rig.feedback(1, 50, false);
    rig.appear(2);
    rig.feedback(2, 50, false);
    let reads = rig.system.volume_reads.load(Ordering::SeqCst);
    wait_until(|| rig.system.volume_reads.load(Ordering::SeqCst) >= reads + 3);
    assert!(
        rig.messages.try_recv().is_err(),
        "unchanged reads send no feedback"
    );
    rig.system.set_volume(73, false);
    rig.feedback(1, 73, false);
    rig.feedback(2, 73, false);
    rig.disappear(1);
    rig.system.set_volume(73, true);
    rig.feedback(2, 0, true);
    rig.disappear(2);
    let reads = rig.system.volume_reads.load(Ordering::SeqCst);
    rig.system.set_volume(20, false);
    std::thread::sleep(Duration::from_millis(35));
    assert_eq!(rig.system.volume_reads.load(Ordering::SeqCst), reads);
    assert!(rig.messages.try_recv().is_err());
    rig.appear(1);
    rig.feedback(1, 20, false);
    rig.disappear(1);
}

#[test]
fn a_volume_invoke_updates_all_active_bindings_and_always_its_own_binding() {
    let mut rig = Rig::new("volume", true);
    rig.appear(1);
    rig.feedback(1, 50, false);
    rig.appear(2);
    rig.feedback(2, 50, false);
    rig.invoke(action("volume", &[]));
    rig.feedback(1, 55, false);
    rig.feedback(2, 55, false);
    rig.system.set_volume(100, false);
    rig.invoke(action("volume", &[]));
    rig.feedback(1, 100, false);
    rig.feedback(2, 100, false);
    rig.invoke(action("volume", &[]));
    rig.feedback(1, 100, false);
    assert!(rig.messages.try_recv().is_err());
}

#[test]
fn a_volume_read_error_is_not_repeated_at_each_poll_and_recovers() {
    let mut rig = Rig::polling();
    rig.system.fail(Some("volume"));
    rig.appear(1);
    rig.problem("42");
    let reads = rig.system.volume_reads.load(Ordering::SeqCst);
    wait_until(|| rig.system.volume_reads.load(Ordering::SeqCst) >= reads + 3);
    assert!(rig.messages.try_recv().is_err());
    rig.system.fail(None);
    rig.feedback(1, 50, false);
}

#[test]
fn permissions_at_start_are_explicit_for_all_three_kinds() {
    for ready in [true, false] {
        let rig = Rig::new("hotkey", ready);
        statuses(rig.initial, ready);
        assert_eq!(rig.system.asks.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn permission_is_checked_at_each_invoke_and_missing_permission_sends_no_keys() {
    let mut rig = Rig::new("hotkey", false);
    for (kind, key, value) in [
        ("hotkey", "hotkey", "cmd+a"),
        ("type-text", "text", "Test"),
        ("media-key", "key", "mute"),
    ] {
        rig.invoke(text_action(kind, key, value));
        statuses((0..3).map(|_| rig.recv()).collect(), false);
    }
    assert!(rig.system.calls().is_empty());
    rig.system.permission.store(true, Ordering::SeqCst);
    rig.invoke(text_action("hotkey", "hotkey", "cmd+a"));
    statuses((0..3).map(|_| rig.recv()).collect(), true);
    assert_eq!(rig.system.calls(), [Call::Hotkey("cmd+a".into())]);
    rig.system.permission.store(false, Ordering::SeqCst);
    rig.invoke(text_action("hotkey", "hotkey", "cmd+a"));
    statuses((0..3).map(|_| rig.recv()).collect(), false);
    assert_eq!(rig.system.calls().len(), 1);
    assert!(rig.messages.try_recv().is_err());
}

#[test]
fn resolve_asks_and_the_helper_sends_ready_when_permission_arrives() {
    let mut rig = Rig::new("hotkey", false);
    rig.handler.handle(Request::Resolve {
        kind: "opencraft.type-text".into(),
    });
    statuses((0..3).map(|_| rig.recv()).collect(), false);
    assert_eq!(rig.system.asks.load(Ordering::SeqCst), 1);
    let reads = rig.system.permission_reads.load(Ordering::SeqCst);
    wait_until(|| rig.system.permission_reads.load(Ordering::SeqCst) > reads);
    rig.system.permission.store(true, Ordering::SeqCst);
    statuses((0..3).map(|_| rig.recv()).collect(), true);
    let reads = rig.system.permission_reads.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(rig.system.permission_reads.load(Ordering::SeqCst), reads);
    assert!(rig.system.calls().is_empty());
}

#[test]
fn resolve_when_permission_is_already_present_does_no_periodic_work() {
    let mut rig = Rig::new("hotkey", false);
    rig.system.permission.store(true, Ordering::SeqCst);
    rig.handler.handle(Request::Resolve {
        kind: "opencraft.hotkey".into(),
    });
    statuses((0..3).map(|_| rig.recv()).collect(), true);
    let reads = rig.system.permission_reads.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(rig.system.permission_reads.load(Ordering::SeqCst), reads);
}

#[test]
fn the_permission_helper_stops_at_its_deadline_and_resolve_can_start_it_again() {
    let mut rig = Rig::new("hotkey", false);
    let resolve = Request::Resolve {
        kind: "opencraft.media-key".into(),
    };
    rig.handler.handle(resolve.clone());
    statuses((0..3).map(|_| rig.recv()).collect(), false);
    std::thread::sleep(Duration::from_millis(140));
    let reads = rig.system.permission_reads.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(rig.system.permission_reads.load(Ordering::SeqCst), reads);
    rig.handler.handle(resolve);
    statuses((0..3).map(|_| rig.recv()).collect(), false);
    rig.system.permission.store(true, Ordering::SeqCst);
    statuses((0..3).map(|_| rig.recv()).collect(), true);
    assert_eq!(rig.system.asks.load(Ordering::SeqCst), 2);
}

#[test]
fn dropping_handlers_stops_helpers() {
    for kind in ["volume", "hotkey"] {
        let mut rig = Rig::new(kind, false);
        if kind == "volume" {
            rig.appear(1);
            rig.feedback(1, 50, false);
        } else {
            rig.handler.handle(Request::Resolve {
                kind: "opencraft.hotkey".into(),
            });
        }
        let system = rig.system.clone();
        drop(rig);
        let reads = (
            system.volume_reads.load(Ordering::SeqCst),
            system.permission_reads.load(Ordering::SeqCst),
        );
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(
            reads,
            (
                system.volume_reads.load(Ordering::SeqCst),
                system.permission_reads.load(Ordering::SeqCst)
            )
        );
    }
}

#[test]
fn shell_wait_does_not_delay_the_next_request_and_nonzero_exit_is_a_problem() {
    let mut rig = Rig::new("shell-command", true);
    rig.system.block_shell();
    let start = Instant::now();
    rig.invoke(text_action("shell-command", "command", "wait in the fake"));
    assert!(start.elapsed() < Duration::from_millis(100));
    wait_until(|| rig.system.waiting.load(Ordering::SeqCst));
    rig.invoke(text_action("open-url", "url", "https://example.test"));
    assert_eq!(
        rig.system.calls(),
        [
            Call::Shell("wait in the fake".into()),
            Call::Url("https://example.test".into())
        ]
    );
    assert!(rig.messages.try_recv().is_err());
    rig.system.finish_shell(Exit::Code(7));
    rig.problem("code 7");
}

#[test]
fn shell_wait_errors_and_signals_are_problems() {
    for (exit, contains) in [(Exit::Error, "42"), (Exit::Signal, "signal")] {
        let mut rig = Rig::new("shell-command", true);
        rig.system.finish_shell(exit);
        rig.invoke(text_action("shell-command", "command", "fake"));
        rig.problem(contains);
    }
}

#[test]
fn non_volume_lifecycle_and_unknown_resolve_requests_do_no_work() {
    for kind in ["open-url", "hotkey"] {
        let mut rig = Rig::new(kind, true);
        rig.handler.handle(Request::Appear {
            binding: binding(1),
            action: action(kind, &[]),
        });
        rig.disappear(1);
        rig.handler.handle(Request::Resolve {
            kind: "other.unknown".into(),
        });
        assert!(rig.system.calls().is_empty());
        assert!(rig.messages.try_recv().is_err());
        assert_eq!(rig.system.asks.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn a_closed_message_channel_does_not_panic() {
    let mut rig = Rig::new("volume", true);
    let (_, replacement) = mpsc::channel();
    drop(std::mem::replace(&mut rig.messages, replacement));
    rig.handler.handle(invoke(action("volume", &[])));
    rig.handler.handle(Request::Appear {
        binding: binding(1),
        action: action("volume", &[]),
    });
    rig.disappear(1);
}

#[test]
fn the_factory_has_exactly_seven_unique_kinds_with_the_required_fields() {
    let real: Vec<_> = handlers().iter().flat_map(|h| h.kinds()).collect();
    let fake: Vec<_> = handlers_with_system(Arc::new(Fake::default()))
        .iter()
        .flat_map(|h| h.kinds())
        .collect();
    assert_eq!(real, fake);
    assert_eq!(real.len(), 7);
    let kinds: BTreeSet<_> = real.iter().map(|k| k.kind.as_str()).collect();
    assert_eq!(kinds.len(), 7);
    for (kind, name, key, input) in [
        (
            "open-application",
            "Open an application",
            "application",
            InputType::Path {
                kind: PathKind::Application,
            },
        ),
        ("open-url", "Open a URL", "url", InputType::Text),
        ("hotkey", "Send a hotkey", "hotkey", InputType::Hotkey),
        ("type-text", "Type text", "text", InputType::LongText),
        (
            "shell-command",
            "Run a shell command",
            "command",
            InputType::LongText,
        ),
    ] {
        let description = real
            .iter()
            .find(|d| d.kind == format!("opencraft.{kind}"))
            .unwrap();
        assert_eq!(description.name, name);
        assert_eq!(description.fields.len(), 1);
        assert_eq!(description.fields[0].key, key);
        assert!(!description.fields[0].label.is_empty());
        assert_eq!(description.fields[0].input, input);
        assert_eq!(description.fields[0].default, None);
    }
    for (kind, name, key, expected) in [
        (
            "media-key",
            "Media key",
            "key",
            vec![
                "play-pause",
                "next",
                "previous",
                "volume-up",
                "volume-down",
                "mute",
            ],
        ),
        (
            "volume",
            "Sound volume",
            "operation",
            vec!["up", "down", "mute"],
        ),
    ] {
        let description = real
            .iter()
            .find(|d| d.kind == format!("opencraft.{kind}"))
            .unwrap();
        assert_eq!(description.name, name);
        assert_eq!(description.fields[0].key, key);
        let InputType::Choice { options } = &description.fields[0].input else {
            panic!("expected a choice")
        };
        assert_eq!(
            options.iter().map(|o| o.value.as_str()).collect::<Vec<_>>(),
            expected
        );
        assert!(options.iter().all(|o| !o.label.is_empty()));
        if kind == "volume" {
            assert_eq!(description.fields.len(), 2);
            assert_eq!(
                description.fields[0].default,
                Some(Value::Text("up".into()))
            );
            assert_eq!(description.fields[1].key, "step");
            assert_eq!(
                description.fields[1].input,
                InputType::Integer { min: 1, max: 25 }
            );
            assert_eq!(description.fields[1].default, Some(Value::Integer(5)));
        } else {
            assert_eq!(description.fields.len(), 1);
            assert_eq!(description.fields[0].default, None);
        }
    }
}

#[test]
fn production_intervals_match_the_specification() {
    let intervals = handlers::Intervals::default();
    assert_eq!(intervals.volume, Duration::from_secs(2));
    assert_eq!(intervals.permission, Duration::from_secs(1));
    assert_eq!(intervals.permission_limit, Duration::from_secs(600));
}
