//! One worker for all countdowns and stopwatches.
//!
//! A timer that a key shows wakes the worker each second, because its text
//! changes each second. A timer that no key shows draws no text: a countdown
//! wakes the worker one time only, at zero, for its bell, and a stopwatch
//! wakes it never. The elapsed time of a hidden stopwatch comes from the
//! clock when a key shows the timer again. Thus only a timer that a key shows
//! wakes the worker each second, and a timer that no key shows causes no
//! endless wake.
//!
//! The engine sends `Invoke` to the handler of the kind of the action, with
//! the binding of the handler that asked. An action inside a Sequence, a
//! Toggle, an action cycle, or the multi-function knob thus sends `Invoke`
//! with the binding of the outer action, and the engine sends no `Appear` and
//! no `Disappear` to this worker for that binding. Such a timer stays hidden:
//! its countdown rings one time at zero and keeps no wake after that, and its
//! stopwatch has no effect. Decision 0017 has the reasons. A timer that a key
//! showed keeps its state when the key goes away, because a countdown sounds
//! its bell also on another page (`docs/user/actions.md`).

use crate::backend::System;
use crate::kinds::{kinds_of, problem, send, timer, Group};
use opencraft_engine::{
    BindingId, Feedback, FeedbackState, Handler, KindDescription, Message, MessageSender, Request,
};
use std::collections::BTreeMap;
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use timer::{Mode, TimerSettings};

/// A clock that gives the current monotonic instant.
pub type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

/// The handler of the Timer kind.
pub struct TimerHandler {
    system: Arc<dyn System>,
    clock: Clock,
    worker: Option<(mpsc::Sender<Request>, JoinHandle<()>)>,
}

impl TimerHandler {
    pub fn new(system: Arc<dyn System>) -> Self {
        Self::with_clock(system, Arc::new(Instant::now))
    }

    pub fn with_clock(system: Arc<dyn System>, clock: Clock) -> Self {
        Self {
            system,
            clock,
            worker: None,
        }
    }
}

impl Handler for TimerHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        kinds_of(Group::Timer)
            .map(|kind| kind.description())
            .collect()
    }

    fn start(&mut self, sender: MessageSender) {
        let (tx, rx) = mpsc::channel();
        let system = self.system.clone();
        let clock = self.clock.clone();
        let output = Some(sender.clone());
        match thread::Builder::new()
            .name("opencraft-timer".into())
            .spawn(move || {
                Timers::default().run(rx, clock, system.as_ref(), &output);
            }) {
            Ok(thread) => self.worker = Some((tx, thread)),
            Err(e) => problem(
                &Some(sender),
                Err(format!("Cannot start the timer worker: {e}.")),
            ),
        }
    }

    fn handle(&mut self, request: Request) {
        if let Some((sender, _)) = &self.worker {
            let _ = sender.send(request);
        }
    }
}

impl Drop for TimerHandler {
    fn drop(&mut self) {
        if let Some((sender, thread)) = self.worker.take() {
            drop(sender);
            let _ = thread.join();
        }
    }
}

#[derive(Clone, Copy, Default)]
enum Phase {
    #[default]
    Idle,
    Running {
        started_at: Instant,
        elapsed_before: Duration,
    },
    Paused(Duration),
    Finished,
}

impl Phase {
    fn elapsed(self, now: Instant) -> Duration {
        match self {
            Self::Running {
                started_at,
                elapsed_before,
            } => elapsed_before + now.saturating_duration_since(started_at),
            Self::Paused(elapsed) => elapsed,
            _ => Duration::ZERO,
        }
    }
}

struct Entry {
    settings: TimerSettings,
    phase: Phase,
    visible: bool,
    last_shown: Option<(u64, FeedbackState)>,
}

impl Entry {
    fn complete(
        &mut self,
        now: Instant,
        system: &dyn System,
        sender: &Option<MessageSender>,
    ) -> bool {
        if matches!(self.phase, Phase::Running { .. })
            && self.settings.mode == Mode::Countdown
            && self.phase.elapsed(now).as_secs() >= self.settings.seconds
        {
            self.phase = Phase::Finished;
            problem(sender, self.settings.bell.ring(system));
            true
        } else {
            false
        }
    }

    fn show(&mut self, binding: BindingId, now: Instant, sender: &Option<MessageSender>) {
        if !self.visible {
            return;
        }
        let elapsed = self.phase.elapsed(now).as_secs();
        let seconds = match self.settings.mode {
            Mode::Countdown if matches!(self.phase, Phase::Finished) => 0,
            Mode::Countdown => self.settings.seconds.saturating_sub(elapsed),
            Mode::Stopwatch => elapsed,
        };
        let state = if matches!(self.phase, Phase::Running { .. }) {
            FeedbackState::On
        } else {
            FeedbackState::Off
        };
        if self.last_shown != Some((seconds, state)) {
            self.last_shown = Some((seconds, state));
            send(
                sender,
                Message::Feedback {
                    binding,
                    feedback: Some(Feedback {
                        label: Some(format_time(seconds)),
                        state: Some(state),
                        ..Feedback::default()
                    }),
                },
            );
        }
    }
}

fn format_time(seconds: u64) -> String {
    if seconds < 3600 {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    } else {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    }
}

#[derive(Default)]
struct Timers(BTreeMap<BindingId, Entry>);

impl Timers {
    /// The deadline of the next turn of the worker.
    ///
    /// A timer that a key shows needs a turn at its next second, because the
    /// text changes at each second. A timer that no key shows draws no text,
    /// thus only the bell of its countdown needs a turn, at zero. A hidden
    /// stopwatch has no deadline at all. Thus a timer that an action inside
    /// another action starts, or that a page change hides, causes no wake at
    /// each second, and no endless wake.
    fn next_wake(&self, now: Instant) -> Option<Instant> {
        self.0
            .values()
            .filter_map(|entry| {
                let Phase::Running {
                    started_at,
                    elapsed_before,
                } = entry.phase
                else {
                    return None;
                };
                if entry.visible {
                    let second = entry.phase.elapsed(now).as_secs() + 1;
                    return Some(started_at + (Duration::from_secs(second) - elapsed_before));
                }
                match entry.settings.mode {
                    // One turn for the bell at zero, and no turn for the text
                    // that no key shows.
                    Mode::Countdown => Some(
                        started_at
                            + Duration::from_secs(entry.settings.seconds)
                                .saturating_sub(elapsed_before),
                    ),
                    // A stopwatch has no bell and no text while it is hidden.
                    Mode::Stopwatch => None,
                }
            })
            .min()
    }

    fn advance(&mut self, now: Instant, system: &dyn System, sender: &Option<MessageSender>) {
        for (&binding, entry) in &mut self.0 {
            entry.complete(now, system, sender);
            entry.show(binding, now, sender);
        }
    }

    fn handle(
        &mut self,
        request: Request,
        now: Instant,
        system: &dyn System,
        sender: &Option<MessageSender>,
    ) {
        let (binding, action, appear, reset) = match request {
            Request::Appear { binding, action } => (binding, action, true, false),
            Request::Invoke { binding, action } => (binding, action, false, false),
            Request::LongPress { binding, action } => (binding, action, false, true),
            Request::Disappear { binding } => {
                if let Some(entry) = self.0.get_mut(&binding) {
                    if matches!(entry.phase, Phase::Idle) {
                        self.0.remove(&binding);
                    } else {
                        entry.visible = false;
                        entry.last_shown = None;
                    }
                }
                return;
            }
            _ => return,
        };
        let settings = match timer::read(&action.settings) {
            Ok(settings) => settings,
            Err(error) => {
                self.0.remove(&binding);
                problem(sender, Err(error));
                return;
            }
        };
        let entry = self.0.entry(binding).or_insert_with(|| Entry {
            settings: settings.clone(),
            phase: Phase::Idle,
            visible: false,
            last_shown: None,
        });
        if entry.settings != settings {
            entry.settings = settings;
            entry.phase = Phase::Idle;
        }
        if appear {
            entry.visible = true;
            entry.last_shown = None;
        } else if reset {
            entry.phase = Phase::Idle;
            if !entry.visible {
                self.0.remove(&binding);
            }
        } else if !entry.complete(now, system, sender) {
            entry.phase = match entry.phase {
                Phase::Running { .. } => Phase::Paused(entry.phase.elapsed(now)),
                Phase::Paused(elapsed_before) => Phase::Running {
                    started_at: now,
                    elapsed_before,
                },
                Phase::Idle | Phase::Finished => Phase::Running {
                    started_at: now,
                    elapsed_before: Duration::ZERO,
                },
            };
        }
    }

    fn run(
        mut self,
        requests: mpsc::Receiver<Request>,
        clock: Clock,
        system: &dyn System,
        sender: &Option<MessageSender>,
    ) {
        let mut now = clock();
        loop {
            let request = match self.next_wake(now) {
                Some(deadline) => {
                    match requests.recv_timeout(deadline.saturating_duration_since(clock())) {
                        Ok(request) => Some(request),
                        Err(mpsc::RecvTimeoutError::Timeout) => None,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                None => match requests.recv() {
                    Ok(request) => Some(request),
                    Err(_) => break,
                },
            };
            now = clock();
            if let Some(request) = request {
                self.handle(request, now, system, sender);
            }
            self.advance(now, system, sender);
        }
    }
}

#[cfg(test)]
#[path = "tests/timer.rs"]
mod tests;
