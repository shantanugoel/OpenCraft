//! One worker for all countdowns and stopwatches.

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
                let second = entry.phase.elapsed(now).as_secs() + 1;
                Some(started_at + (Duration::from_secs(second) - elapsed_before))
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
