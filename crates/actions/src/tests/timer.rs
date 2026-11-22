use super::*;
use crate::tests::support::{Call, Fake};
use opencraft_engine::{Action, ControlId, ControlKind, GestureKind, Settings, Value};
use FeedbackState::{Off, On};

struct Rig {
    timers: Timers,
    binding: BindingId,
    action: Action,
    now: Instant,
    system: Fake,
    sender: Option<MessageSender>,
    messages: mpsc::Receiver<Message>,
}

impl Rig {
    fn countdown(seconds: i64) -> Self {
        Self::new(Settings::from([(
            "seconds".into(),
            Value::Integer(seconds),
        )]))
    }

    fn new(settings: Settings) -> Self {
        let (sender, messages) = mpsc::channel();
        Self {
            timers: Timers::default(),
            binding: BindingId {
                page: 0,
                folder: None,
                control: ControlId::new(ControlKind::Key, 1),
                gesture: GestureKind::Press,
            },
            action: Action {
                kind: timer::ID.into(),
                settings,
            },
            now: Instant::now(),
            system: Fake::default(),
            sender: Some(sender),
            messages,
        }
    }

    fn send(&mut self, request: Request) {
        self.timers
            .handle(request, self.now, &self.system, &self.sender);
        self.advance(Duration::ZERO);
    }

    fn appear(&mut self) {
        self.send(Request::Appear {
            binding: self.binding,
            action: self.action.clone(),
        });
    }

    fn press(&mut self) {
        self.send(Request::Invoke {
            binding: self.binding,
            action: self.action.clone(),
        });
    }

    fn reset(&mut self) {
        self.send(Request::LongPress {
            binding: self.binding,
            action: self.action.clone(),
        });
    }

    fn disappear(&mut self) {
        self.send(Request::Disappear {
            binding: self.binding,
        });
    }

    fn advance(&mut self, duration: Duration) {
        self.now += duration;
        self.timers.advance(self.now, &self.system, &self.sender);
    }

    fn seconds(&mut self, seconds: u64) {
        self.advance(Duration::from_secs(seconds));
    }

    fn shows(&self, label: &str, state: FeedbackState) {
        assert_eq!(
            self.messages.try_iter().collect::<Vec<_>>(),
            [Message::Feedback {
                binding: self.binding,
                feedback: Some(Feedback {
                    label: Some(label.into()),
                    state: Some(state),
                    ..Feedback::default()
                }),
            }]
        );
    }

    fn quiet(&self) {
        assert!(self.messages.try_recv().is_err());
    }
}

#[test]
fn countdown_ticks_once_per_second_rings_once_and_can_restart() {
    let mut r = Rig::countdown(5);
    assert_eq!(r.timers.next_wake(r.now), None);
    r.appear();
    r.shows("00:05", Off);
    assert_eq!(r.timers.next_wake(r.now), None);
    r.press();
    r.shows("00:05", On);
    for seconds in (1..=4).rev() {
        assert_eq!(
            r.timers.next_wake(r.now),
            Some(r.now + Duration::from_secs(1))
        );
        r.seconds(1);
        r.shows(&format!("00:0{seconds}"), On);
    }
    r.seconds(1);
    r.shows("00:00", Off);
    assert_eq!(r.timers.next_wake(r.now), None);
    r.seconds(2);
    r.quiet();
    assert_eq!(r.system.calls(), [Call::Bell]);
    r.press();
    r.shows("00:05", On);
}

#[test]
fn pause_and_resume_preserve_fractional_seconds_and_remove_the_idle_deadline() {
    let mut r = Rig::countdown(10);
    r.appear();
    r.shows("00:10", Off);
    r.press();
    r.shows("00:10", On);
    r.advance(Duration::from_millis(3250));
    r.shows("00:07", On);
    r.press();
    r.shows("00:07", Off);
    assert_eq!(r.timers.next_wake(r.now), None);
    r.seconds(10);
    r.quiet();
    assert!(r.system.calls().is_empty());
    r.press();
    r.shows("00:07", On);
    assert_eq!(
        r.timers.next_wake(r.now),
        Some(r.now + Duration::from_millis(750))
    );
    r.advance(Duration::from_millis(6749));
    r.shows("00:01", On);
    r.advance(Duration::from_millis(1));
    r.shows("00:00", Off);
    assert_eq!(r.system.calls(), [Call::Bell]);
}

#[test]
fn long_press_resets_running_paused_and_finished_without_a_bell() {
    let mut r = Rig::countdown(10);
    r.appear();
    r.shows("00:10", Off);
    for pause in [false, true] {
        r.press();
        r.shows("00:10", On);
        r.seconds(4);
        r.shows("00:06", On);
        if pause {
            r.press();
            r.shows("00:06", Off);
        }
        r.reset();
        r.shows("00:10", Off);
        assert_eq!(r.timers.next_wake(r.now), None);
        assert!(r.system.calls().is_empty());
    }
    r.press();
    r.shows("00:10", On);
    r.seconds(10);
    r.shows("00:00", Off);
    r.reset();
    r.shows("00:10", Off);
    assert_eq!(r.system.calls(), [Call::Bell]);
}

#[test]
fn stopwatch_counts_up_and_never_rings() {
    let mut r = Rig::new(Settings::from([(
        "mode".into(),
        Value::Text("stopwatch".into()),
    )]));
    r.appear();
    r.shows("00:00", Off);
    r.press();
    r.shows("00:00", On);
    r.seconds(1);
    r.shows("00:01", On);
    r.seconds(1);
    r.shows("00:02", On);
    r.seconds(3598);
    r.shows("1:00:00", On);
    r.press();
    r.shows("1:00:00", Off);
    r.reset();
    r.shows("00:00", Off);
    assert!(r.system.calls().is_empty());
}

#[test]
fn navigation_hides_feedback_but_preserves_a_running_paused_or_finished_timer() {
    let mut r = Rig::countdown(10);
    r.appear();
    r.shows("00:10", Off);
    r.press();
    r.shows("00:10", On);
    r.seconds(3);
    r.shows("00:07", On);
    r.disappear();
    r.seconds(4);
    r.quiet();
    r.appear();
    r.shows("00:03", On);
    r.press();
    r.shows("00:03", Off);
    r.disappear();
    r.seconds(10);
    r.quiet();
    r.appear();
    r.shows("00:03", Off);
    r.press();
    r.shows("00:03", On);
    r.disappear();
    r.seconds(3);
    r.quiet();
    assert_eq!(r.system.calls(), [Call::Bell]);
    r.appear();
    r.shows("00:00", Off);
}

#[test]
fn an_invoke_of_a_binding_that_no_key_shows_keeps_no_wake() {
    let mut r = Rig::new(Settings::from([(
        "mode".into(),
        Value::Text("stopwatch".into()),
    )]));
    // The engine sends `Invoke` with the binding of the outer action for an
    // action inside a Sequence, a Toggle, an action cycle, or the
    // multi-function knob. It sends no `Appear` and no `Disappear` to this
    // worker for it, thus the action must keep no wake.
    r.press();
    r.quiet();
    assert_eq!(
        r.timers.next_wake(r.now),
        None,
        "a hidden stopwatch has no wake"
    );
    r.seconds(3600);
    r.quiet();
    assert!(r.system.calls().is_empty(), "a stopwatch never rings");
    assert_eq!(
        r.timers.next_wake(r.now),
        None,
        "no wake stays after the wait"
    );
}

#[test]
fn a_hidden_timer_wakes_only_for_the_bell_of_its_countdown() {
    let mut r = Rig::countdown(10);
    r.appear();
    r.shows("00:10", Off);
    r.press();
    r.shows("00:10", On);
    r.seconds(3);
    r.shows("00:07", On);
    r.disappear();
    // One turn at zero, and no turn for each second of text that no key shows.
    assert_eq!(
        r.timers.next_wake(r.now),
        Some(r.now + Duration::from_secs(7))
    );
    r.seconds(6);
    r.quiet();
    assert_eq!(
        r.timers.next_wake(r.now),
        Some(r.now + Duration::from_secs(1))
    );
    r.seconds(1);
    assert_eq!(r.system.calls(), [Call::Bell]);
    r.seconds(60);
    r.quiet();
    assert_eq!(
        r.timers.next_wake(r.now),
        None,
        "a finished timer has no wake"
    );
}

#[test]
fn a_hidden_stopwatch_keeps_its_time_with_no_wake() {
    let mut r = Rig::new(Settings::from([(
        "mode".into(),
        Value::Text("stopwatch".into()),
    )]));
    r.appear();
    r.shows("00:00", Off);
    r.press();
    r.shows("00:00", On);
    r.seconds(2);
    r.shows("00:02", On);
    r.disappear();
    assert_eq!(
        r.timers.next_wake(r.now),
        None,
        "no turn for a hidden stopwatch"
    );
    r.seconds(3598);
    r.quiet();
    // The elapsed time comes from the clock, thus the timer kept its time.
    r.appear();
    r.shows("1:00:00", On);
}

#[test]
fn editing_a_running_timer_resets_it_and_bell_choice_controls_playback() {
    for (bell, expected) in [
        (
            "file",
            vec![Call::PlaySound("/sounds/bell.wav".into(), 80, None)],
        ),
        ("none", vec![]),
    ] {
        let mut r = Rig::countdown(10);
        r.appear();
        r.shows("00:10", Off);
        r.press();
        r.shows("00:10", On);
        r.disappear();
        r.action.settings.extend([
            ("seconds".into(), Value::Integer(2)),
            ("bell".into(), Value::Text(bell.into())),
            ("file".into(), Value::Text("/sounds/bell.wav".into())),
            ("volume".into(), Value::Integer(80)),
        ]);
        r.appear();
        r.shows("00:02", Off);
        r.seconds(10);
        r.quiet();
        assert!(r.system.calls().is_empty());
        r.press();
        r.shows("00:02", On);
        r.seconds(2);
        r.shows("00:00", Off);
        assert_eq!(r.system.calls(), expected);
    }
}
