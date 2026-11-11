//! The built-in action handlers of OpenCraft.
//!
//! - [`hotkey`]: the text form of a hotkey, for example `shift+cmd+4`.
//! - [`system`]: the work that an action does on the computer: keys, text,
//!   media keys, sound volume, applications, URLs, and shell commands.
//!
//! The handlers use only the handler interface of the engine (architecture
//! rule 8, and `docs/decisions/0012-action-handler-interface.md`).

pub mod hotkey;
pub mod system;
