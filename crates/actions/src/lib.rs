//! The built-in action handlers of OpenCraft.
//!
//! [`handlers`] gives the built-in handlers to the program that starts the
//! engine. Each handler checks its settings and sends problems as messages.
//! A problem with a setting names the action and the field as the editor
//! shows them: see `descriptions::setting_problem`.
//! Long shell commands and sequences do not delay the next request. Helper
//! threads stop when their handler stops. A sequence ends after its last
//! action. The volume helper also stops when no binding
//! remains. A shell command can continue after its handler stops.
//! The volume handler uses field defaults for missing settings.
//!
//! The key handler sends Ready at the start if it has the permission. It
//! sends NeedsUser otherwise. One sleeping helper reads the permission every
//! five seconds and reports only changes, including a revocation. It checks
//! the permission again at each Invoke and Resolve. With no permission, an
//! Invoke of an action that sends keys or mouse events also sends a problem,
//! so that the user sees the cause at the time of the gesture. A Mac system
//! function that sends no key does the work and sends no problem.
//!
//! Tests use [`handlers_with_system`] and a fake [`backend::System`]. No
//! handler test needs a device or changes the real system.
//!
//! - [`hotkey`]: the text form of a hotkey, for example `shift+cmd+4`.
//! - `flow`: the handler of the kinds that ask the engine for work: change
//!   the page, select a profile, open and close a folder, toggle, sequence,
//!   screen off, and device brightness. A sequence waits with
//!   [`backend::System::sleep`], so a test does not wait.
//! - [`system`]: the types of the system port, the Accessibility check, and
//!   the platform code of the work that an action does on the computer: keys,
//!   text, media keys, sound volume, applications, URLs, and shell commands.
//!   [`backend::RealSystem`] calls the platform code.
//! - `kinds`: one module for each built-in kind, and the
//!   registry that lists those modules. A handler runs the kinds of its
//!   group and does not read their settings. The key handler keeps one
//!   poll of the Accessibility permission. The volume handler keeps its poll.
//!   The microphone and sound output handlers keep their listeners and their
//!   feedback. The sound handler runs "Play a sound" and "Stop all sound" in
//!   one queue.
//!
//! The handlers use only the handler interface of the engine (architecture
//! rule 8, and `docs/decisions/0012-action-handler-interface.md`).

mod audio;
mod audio_output;
pub mod backend;
mod clock;
mod descriptions;
mod flow;
mod handlers;
pub mod hotkey;
mod kinds;
mod sound;
pub mod system;
mod timer;

pub use clock::{ClockHandler, LocalTime, WallClock};
pub use timer::{Clock, TimerHandler};

use opencraft_engine::Handler;
use std::sync::Arc;

/// The built-in handlers. A program gives this list to the engine when it
/// starts the engine.
pub fn handlers() -> Vec<Box<dyn Handler>> {
    handlers::real_handlers()
}

/// The built-in handlers with a system implementation that the caller gives.
/// Tests use a fake system. This function does not start a handler thread.
pub fn handlers_with_system(system: Arc<dyn backend::System>) -> Vec<Box<dyn Handler>> {
    handlers::build(system, handlers::Intervals::default())
}

/// The built-in handlers with a caller-supplied monotonic timer clock.
pub fn handlers_with_clock(
    system: Arc<dyn backend::System>,
    clock: Clock,
) -> Vec<Box<dyn Handler>> {
    handlers::build_with_clock(system, handlers::Intervals::default(), clock)
}

/// The built-in handlers with a caller-supplied monotonic timer clock and a
/// caller-supplied wall clock for the clock kind. The tests use this function,
/// so that a clock key needs no real time.
pub fn handlers_with_clocks(
    system: Arc<dyn backend::System>,
    clock: Clock,
    wall: WallClock,
) -> Vec<Box<dyn Handler>> {
    handlers::build_with_clocks(system, handlers::Intervals::default(), clock, wall)
}

#[cfg(test)]
extern crate self as opencraft_actions;
#[cfg(test)]
mod tests;
