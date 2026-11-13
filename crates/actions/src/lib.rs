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
//! the permission again at each Invoke and Resolve. With
//! no permission, an Invoke also sends a problem, so that the user sees the
//! cause at the time of the gesture.
//!
//! Tests use [`handlers_with_system`] and a fake [`backend::System`]. No
//! handler test needs a device or changes the real system.
//!
//! - [`hotkey`]: the text form of a hotkey, for example `shift+cmd+4`.
//! - `flow`: the kinds that ask the engine for work: change the page, select
//!   a profile, toggle, and sequence. A sequence waits with
//!   [`backend::System::sleep`], so a test does not wait.
//! - [`system`]: the work that an action does on the computer: keys, text,
//!   media keys, sound volume, applications, URLs, and shell commands.
//!
//! The handlers use only the handler interface of the engine (architecture
//! rule 8, and `docs/decisions/0012-action-handler-interface.md`).

mod audio;
mod audio_output;
pub mod backend;
mod descriptions;
mod device;
mod flow;
mod handlers;
pub mod hotkey;
pub mod mac_function;
pub mod mouse;
pub mod system;

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

#[cfg(test)]
extern crate self as opencraft_actions;
#[cfg(test)]
mod tests;
