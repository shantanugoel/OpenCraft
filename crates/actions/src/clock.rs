//! One worker for all clock and date keys.
//!
//! The worker keeps the visible clock bindings. With no clock it blocks on
//! its request channel and wakes only for a request. With a clock it wakes at
//! least each 10 seconds and at the next minute boundary, reads the local
//! wall time, and sends a label only when the text is different. The engine
//! draws the label with the style of the surface, thus the handler sends no
//! style (architecture rule 8). Decision 0043, section 6 has the reasons.
//!
//! The 10-second cap is necessary because `recv_timeout` uses
//! [`std::time::Instant`], which does not count the time of a system sleep on
//! macOS (Inference, see `docs/kb/sleep-and-wake.md`). Without the cap a
//! clock could show a time that is wrong for 59 seconds after a sleep. Each
//! wake reads the local time again and calls `tzset`, thus a time zone change
//! shows at the next wake.

use crate::kinds::{clock, kinds_of, problem, send, Group};
use opencraft_engine::{
    BindingId, Feedback, Handler, KindDescription, Message, MessageSender, Request,
};
use std::collections::BTreeMap;
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// The longest wait while a clock is visible. About 6 wakes each minute.
const MAX_WAIT: Duration = Duration::from_secs(10);

/// The local wall time, as a clock key shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    /// The month, 1 for January.
    pub month: u8,
    /// The day of the month, 1 to 31.
    pub day: u8,
    /// The day of the week, 0 for Sunday, as `tm_wday` gives it.
    pub weekday: u8,
    /// The hour, 0 to 23.
    pub hour: u8,
    /// The minute, 0 to 59.
    pub minute: u8,
    /// The second, 0 to 59. It is not shown: it gives the wait to the next
    /// minute boundary.
    pub second: u8,
}

/// A source of the local wall time. `None` means that the source cannot read
/// the time; the clock then sends no text. A test gives a fake source.
pub type WallClock = Arc<dyn Fn() -> Option<LocalTime> + Send + Sync>;

/// The handler of the Clock and date kind.
pub struct ClockHandler {
    wall: WallClock,
    worker: Option<(mpsc::Sender<Request>, JoinHandle<()>)>,
}

impl ClockHandler {
    pub fn new() -> Self {
        Self::with_wall(Arc::new(local_time))
    }

    pub fn with_wall(wall: WallClock) -> Self {
        Self { wall, worker: None }
    }
}

impl Default for ClockHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler for ClockHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        kinds_of(Group::Clock)
            .map(|kind| kind.description())
            .collect()
    }

    fn start(&mut self, sender: MessageSender) {
        let (tx, rx) = mpsc::channel();
        let wall = self.wall.clone();
        let output = Some(sender.clone());
        match thread::Builder::new()
            .name("opencraft-clock".into())
            .spawn(move || {
                Clocks::default().run(rx, wall, &output);
            }) {
            Ok(thread) => self.worker = Some((tx, thread)),
            Err(e) => problem(
                &Some(sender),
                Err(format!("Cannot start the clock worker: {e}.")),
            ),
        }
    }

    fn handle(&mut self, request: Request) {
        if let Some((sender, _)) = &self.worker {
            let _ = sender.send(request);
        }
    }
}

impl Drop for ClockHandler {
    fn drop(&mut self) {
        if let Some((sender, thread)) = self.worker.take() {
            drop(sender);
            let _ = thread.join();
        }
    }
}

struct Entry {
    format: clock::Format,
    /// The text that the engine has. No message goes out for the same text.
    last: Option<String>,
}

/// The visible clock bindings, with the format and the text of each.
#[derive(Default)]
struct Clocks(BTreeMap<BindingId, Entry>);

impl Clocks {
    /// The wait to the next wake. `None` means that no clock is visible, and
    /// the worker blocks without a limit.
    fn wait(&self, time: Option<LocalTime>) -> Option<Duration> {
        if self.0.is_empty() {
            return None;
        }
        let Some(time) = time else {
            return Some(MAX_WAIT);
        };
        // The next minute starts after the seconds that remain. A second
        // value of 0 gives the whole minute.
        let boundary = 60u64.saturating_sub(u64::from(time.second)).max(1);
        Some(Duration::from_secs(boundary).min(MAX_WAIT))
    }

    fn handle(&mut self, request: Request, sender: &Option<MessageSender>) {
        let (binding, action) = match request {
            Request::Appear { binding, action } | Request::Invoke { binding, action } => {
                (binding, action)
            }
            Request::Disappear { binding } => {
                self.0.remove(&binding);
                return;
            }
            _ => return,
        };
        let format = match clock::read(&action.settings) {
            Ok(format) => format,
            Err(error) => {
                self.0.remove(&binding);
                problem(sender, Err(error));
                return;
            }
        };
        let entry = self
            .0
            .entry(binding)
            .or_insert(Entry { format, last: None });
        entry.format = format;
        // Appear and Invoke send the text at once.
        entry.last = None;
    }

    /// Send the text of each visible clock when it is different from the
    /// text that the engine has.
    fn wake(&mut self, time: Option<LocalTime>, sender: &Option<MessageSender>) {
        let Some(time) = time else {
            return;
        };
        for (&binding, entry) in &mut self.0 {
            let text = entry.format.text(time);
            if entry.last.as_deref() == Some(text.as_str()) {
                continue;
            }
            entry.last = Some(text.clone());
            send(
                sender,
                Message::Feedback {
                    binding,
                    feedback: Some(Feedback {
                        label: Some(text),
                        ..Feedback::default()
                    }),
                },
            );
        }
    }

    fn run(
        mut self,
        requests: mpsc::Receiver<Request>,
        wall: WallClock,
        sender: &Option<MessageSender>,
    ) {
        loop {
            let request = match self.wait(wall()) {
                Some(wait) => match requests.recv_timeout(wait) {
                    Ok(request) => Some(request),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                },
                None => match requests.recv() {
                    Ok(request) => Some(request),
                    Err(_) => break,
                },
            };
            if let Some(request) = request {
                self.handle(request, sender);
            }
            // Read the wall time again: the wait or the request took time.
            self.wake(wall(), sender);
        }
    }
}

// `tzset` reads the time zone again. The `libc` crate declares it only on
// Windows, thus this binding is necessary.
#[cfg(unix)]
extern "C" {
    fn tzset();
}

/// The local wall time of the computer. It calls `tzset` before
/// `localtime_r`, so that a change of the time zone shows at the next wake.
#[cfg(unix)]
pub(crate) fn local_time() -> Option<LocalTime> {
    // SAFETY: `tzset` has no argument, and `time` and `localtime_r` write
    // only the values that this function gives them. The pointers are valid
    // for the call.
    unsafe {
        tzset();
        let mut seconds: libc::time_t = 0;
        if libc::time(&mut seconds) == -1 {
            return None;
        }
        let mut time: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&seconds, &mut time).is_null() {
            return None;
        }
        Some(LocalTime {
            month: u8::try_from(time.tm_mon + 1).ok()?,
            day: u8::try_from(time.tm_mday).ok()?,
            weekday: u8::try_from(time.tm_wday).ok()?,
            hour: u8::try_from(time.tm_hour).ok()?,
            minute: u8::try_from(time.tm_min).ok()?,
            second: u8::try_from(time.tm_sec).ok()?,
        })
    }
}

/// A system that is not Unix has no clock text. The handler sends no text.
#[cfg(not(unix))]
pub(crate) fn local_time() -> Option<LocalTime> {
    None
}

#[cfg(test)]
#[path = "tests/clock.rs"]
mod tests;
