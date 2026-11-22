//! The clock kind and its worker with a fake wall clock. No test waits.

use super::*;
use crate::kinds::clock::{self, Format};
use opencraft_engine::{Action, ControlId, ControlKind, GestureKind, Settings, Value};

/// A fixed local time: Saturday 22 November, 14:05 and 30 seconds.
fn time(hour: u8, minute: u8, second: u8) -> LocalTime {
    LocalTime {
        month: 11,
        day: 22,
        weekday: 6,
        hour,
        minute,
        second,
    }
}

fn settings(format: Option<&str>) -> Settings {
    match format {
        Some(format) => Settings::from([("format".into(), Value::Text(format.into()))]),
        None => Settings::new(),
    }
}

struct Rig {
    clocks: Clocks,
    binding: BindingId,
    now: Option<LocalTime>,
    sender: Option<MessageSender>,
    messages: mpsc::Receiver<Message>,
}

impl Rig {
    fn new() -> Self {
        Self::at(time(14, 5, 30))
    }

    fn at(now: LocalTime) -> Self {
        let (sender, messages) = mpsc::channel();
        Self {
            clocks: Clocks::default(),
            binding: BindingId {
                page: 0,
                folder: None,
                control: ControlId::new(ControlKind::Key, 1),
                gesture: GestureKind::Press,
            },
            now: Some(now),
            sender: Some(sender),
            messages,
        }
    }

    fn appear(&mut self, format: Option<&str>) {
        self.send(Request::Appear {
            binding: self.binding,
            action: Action {
                kind: clock::ID.into(),
                settings: settings(format),
            },
        });
    }

    fn invoke(&mut self, format: Option<&str>) {
        self.send(Request::Invoke {
            binding: self.binding,
            action: Action {
                kind: clock::ID.into(),
                settings: settings(format),
            },
        });
    }

    fn disappear(&mut self) {
        self.send(Request::Disappear {
            binding: self.binding,
        });
    }

    /// Do one wake of the worker with the current time.
    fn send(&mut self, request: Request) {
        self.clocks.handle(request, &self.sender);
        self.wake();
    }

    fn set(&mut self, now: LocalTime) {
        self.now = Some(now);
    }

    fn lose_the_wall_clock(&mut self) {
        self.now = None;
    }

    fn wake(&mut self) {
        self.clocks.wake(self.now, &self.sender);
    }

    /// The wait that the worker uses before its next wake.
    fn wait(&self) -> Option<Duration> {
        self.clocks.wait(self.now)
    }

    fn shows(&self, text: &str) {
        assert_eq!(
            self.messages.try_iter().collect::<Vec<_>>(),
            [Message::Feedback {
                binding: self.binding,
                feedback: Some(Feedback {
                    label: Some(text.into()),
                    ..Feedback::default()
                }),
            }]
        );
    }

    fn quiet(&self) {
        assert!(self.messages.try_recv().is_err());
    }

    fn problem(&self) {
        match self.messages.try_recv() {
            Ok(Message::Problem(text)) => assert!(text.contains("Format"), "{text}"),
            other => panic!("expected a Problem, got {other:?}"),
        }
    }
}

#[test]
fn the_five_formats_give_the_text_of_the_decision() {
    let now = time(14, 5, 32);
    assert_eq!(Format::Hours24.text(now), "14:05");
    assert_eq!(Format::Hours12.text(now), "2:05 PM");
    assert_eq!(Format::Date.text(now), "Nov 22");
    assert_eq!(Format::WeekdayDate.text(now), "Sat\nNov 22");
    assert_eq!(Format::TimeDate.text(now), "14:05\nNov 22");

    // Midnight and noon have 12 in the 12-hour form, and the day has no
    // leading zero.
    let midnight = LocalTime {
        minute: 7,
        hour: 0,
        day: 5,
        ..now
    };
    assert_eq!(Format::Hours12.text(midnight), "12:07 AM");
    assert_eq!(Format::Date.text(midnight), "Nov 5");
    let noon = LocalTime {
        hour: 12,
        minute: 0,
        ..now
    };
    assert_eq!(Format::Hours12.text(noon), "12:00 PM");
    let morning = LocalTime {
        hour: 9,
        minute: 0,
        ..now
    };
    assert_eq!(Format::Hours12.text(morning), "9:00 AM");
}

#[test]
fn the_read_gives_the_default_and_refuses_an_unknown_format() {
    assert_eq!(clock::read(&settings(None)), Ok(Format::Hours24));
    assert_eq!(clock::read(&settings(Some("12h"))), Ok(Format::Hours12));
    assert_eq!(clock::read(&settings(Some("date"))), Ok(Format::Date));
    assert_eq!(
        clock::read(&settings(Some("weekday-date"))),
        Ok(Format::WeekdayDate)
    );
    assert_eq!(
        clock::read(&settings(Some("time-date"))),
        Ok(Format::TimeDate)
    );
    assert!(clock::read(&settings(Some("tomorrow"))).is_err());
    assert!(clock::read(&Settings::from([("format".into(), Value::Integer(24))])).is_err());
}

#[test]
fn a_clock_sends_its_text_at_appear_and_only_when_the_minute_changes() {
    let mut r = Rig::new();
    assert_eq!(r.wait(), None, "no visible clock means no wake");
    r.appear(None);
    r.shows("14:05");
    assert_eq!(r.wait(), Some(MAX_WAIT));
    r.wake();
    r.quiet();

    // The same minute does not send the same text again.
    r.set(time(14, 5, 59));
    assert_eq!(r.wait(), Some(Duration::from_secs(1)));
    r.wake();
    r.quiet();

    // The next minute sends the new text one time.
    r.set(time(14, 6, 0));
    assert_eq!(r.wait(), Some(MAX_WAIT));
    r.wake();
    r.shows("14:06");
    r.wake();
    r.quiet();

    // The wait is 10 seconds or the time to the boundary, whichever is less.
    for (second, wait) in [(0, 10), (49, 10), (50, 10), (51, 9), (59, 1)] {
        r.set(time(14, 6, second));
        assert_eq!(r.wait(), Some(Duration::from_secs(wait)), "{second}");
    }
}

#[test]
fn midnight_changes_the_hour_and_the_date() {
    let mut r = Rig::at(LocalTime {
        month: 12,
        day: 31,
        weekday: 3,
        hour: 23,
        minute: 59,
        second: 59,
    });
    r.appear(Some("time-date"));
    r.shows("23:59\nDec 31");
    r.set(LocalTime {
        month: 1,
        day: 1,
        weekday: 4,
        hour: 0,
        minute: 0,
        second: 0,
    });
    r.wake();
    r.shows("00:00\nJan 1");
    r.wake();
    r.quiet();
}

#[test]
fn a_time_zone_change_shows_at_the_next_wake() {
    let mut r = Rig::new();
    r.appear(Some("24h"));
    r.shows("14:05");
    // The wall clock gives another hour for the same minute: a change of the
    // time zone, or a change of the clock by the person or the time service.
    r.set(time(15, 5, 32));
    r.wake();
    r.shows("15:05");
    r.wake();
    r.quiet();
}

#[test]
fn a_change_of_the_format_updates_the_text() {
    let mut r = Rig::new();
    r.appear(Some("24h"));
    r.shows("14:05");
    r.appear(Some("12h"));
    r.shows("2:05 PM");
    r.appear(Some("weekday-date"));
    r.shows("Sat\nNov 22");
}

#[test]
fn an_invoke_sends_the_text_again_at_once() {
    let mut r = Rig::new();
    r.appear(Some("date"));
    r.shows("Nov 22");
    r.wake();
    r.quiet();
    r.invoke(Some("date"));
    r.shows("Nov 22");
}

#[test]
fn a_hidden_clock_sends_no_text_and_has_no_wake() {
    let mut r = Rig::new();
    r.appear(Some("24h"));
    r.shows("14:05");
    r.disappear();
    assert_eq!(r.wait(), None);
    r.wake();
    r.quiet();
    r.set(LocalTime {
        month: 12,
        day: 25,
        weekday: 5,
        hour: 8,
        minute: 0,
        second: 0,
    });
    r.wake();
    r.quiet();
    // A second disappear of a binding that is not visible is safe.
    r.disappear();
    r.quiet();
}

#[test]
fn a_bad_format_is_a_problem_and_does_not_show() {
    let mut r = Rig::new();
    r.appear(Some("tomorrow"));
    r.problem();
    r.quiet();
    assert_eq!(r.wait(), None);
}

#[test]
fn a_wall_clock_that_cannot_read_sends_no_text_and_keeps_the_cap() {
    let mut r = Rig::new();
    r.lose_the_wall_clock();
    r.appear(Some("24h"));
    r.quiet();
    assert_eq!(r.wait(), Some(MAX_WAIT));
    r.wake();
    r.quiet();
}
