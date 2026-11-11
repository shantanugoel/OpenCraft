//! The work that an action does on the computer.
//!
//! Each function does one small thing and returns. The platform code is in
//! one module for each operating system. On a system that has no code, each
//! function gives [`Error::NotSupported`].
//!
//! The permission that each function needs is in
//! `docs/kb/macos-permissions.md`.

use crate::hotkey::Hotkey;
use std::io;
use std::process::{Child, Command, Stdio};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(not(target_os = "macos"))]
mod other;
#[cfg(not(target_os = "macos"))]
use other as platform;

/// An action could not do its work.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("this operating system does not have this action")]
    NotSupported,
    #[error("the computer has no sound output with a volume control")]
    NoVolume,
    #[error("the system refused the operation (code {0})")]
    System(i32),
    #[error("{0}")]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A media key of a keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKey {
    PlayPause,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    Mute,
}

/// The state of the sound output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume {
    /// The level in percent, from 0 to 100.
    pub level: u8,
    pub muted: bool,
}

/// True if this program can send keys to other applications. On macOS, this
/// is the Accessibility permission.
pub fn can_send_keys() -> bool {
    platform::can_send_keys()
}

/// Ask the system to show its prompt for the permission to send keys. The
/// call returns at once. The user gives the permission in the settings of
/// the system.
pub fn ask_to_send_keys() {
    platform::ask_to_send_keys();
}

/// Press and release the key of `hotkey`, with its modifier keys.
pub fn send_hotkey(hotkey: &Hotkey) -> Result<()> {
    platform::send_hotkey(hotkey)
}

/// Type `text` into the application that has the keyboard focus.
pub fn type_text(text: &str) -> Result<()> {
    platform::type_text(text)
}

/// Press and release a media key.
pub fn media_key(key: MediaKey) -> Result<()> {
    platform::media_key(key)
}

/// The state of the default sound output.
pub fn volume() -> Result<Volume> {
    platform::volume()
}

/// Set the level of the default sound output, in percent. A value above 100
/// is 100.
pub fn set_volume_level(level: u8) -> Result<()> {
    platform::set_volume_level(level.min(100))
}

/// Switch the mute of the default sound output on or off.
pub fn set_muted(muted: bool) -> Result<()> {
    platform::set_muted(muted)
}

/// Open an application. `application` is a name, for example `Safari`, or
/// the path of an application.
pub fn open_application(application: &str) -> Result<()> {
    platform::open_application(application)
}

/// Open a URL in the default application for it.
pub fn open_url(url: &str) -> Result<()> {
    platform::open_url(url)
}

/// Start a shell command with `/bin/sh -c`. The call does not wait for the
/// command. The caller must wait for the child, so that the system can
/// remove it.
pub fn start_shell_command(command: &str) -> Result<Child> {
    Ok(Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}
