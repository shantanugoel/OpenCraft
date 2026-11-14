//! The work that an action does on the computer.
//!
//! The platform code is in one module for each operating system, and
//! `platform` names the module of the system that runs the program. On a
//! system that has no code, each function gives [`Error::NotSupported`].
//! The system port ([`crate::backend::System`]) calls the platform functions
//! with no layer between. This file has the types of the port, the
//! Accessibility check that the application calls, and the start of a shell
//! command.
//!
//! The permission that each function needs is in
//! `docs/kb/macos-permissions.md`.

use std::io;
use std::process::{Child, Command, Stdio};

#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "macos")]
mod macos_functions;
#[cfg(target_os = "macos")]
mod macos_mouse;
#[cfg(target_os = "macos")]
mod macos_quit;
#[cfg(target_os = "macos")]
mod macos_sound;
#[cfg(target_os = "macos")]
pub(crate) use macos as platform;

#[cfg(not(target_os = "macos"))]
pub(crate) mod other;
#[cfg(not(target_os = "macos"))]
pub(crate) use other as platform;

/// An action could not do its work.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("this operating system does not have this action")]
    NotSupported,
    #[error("the computer has no sound output with a volume control")]
    NoVolume,
    #[error("the default sound input has no writable master mute")]
    NoInputMute,
    #[error("the application does not run")]
    NotRunning,
    #[error("the system refused the operation (code {0})")]
    System(i32),
    /// macOS refused the sleep with `kIOReturnNotPermitted`. A setting
    /// turns off all sleep. See `docs/kb/macos-system-functions.md`.
    #[error(
        "macOS does not allow sleep now. A setting turns off all sleep, for example \
         \"pmset disablesleep\" from an application that keeps the Mac awake. The Apple \
         menu cannot put the Mac to sleep either"
    )]
    SleepNotAllowed,
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
    FastForward,
    Rewind,
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

/// A sound output with its stable Core Audio UID and its display name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
}

/// A property listener. Refresh attaches to a changed default device.
pub trait AudioWatch: Send {
    fn refresh(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Start a shell command with `/bin/sh -c`. The call does not wait for the
/// command. The caller must wait for the child, so that the system can
/// remove it.
pub(crate) fn start_shell_command(command: &str) -> Result<Child> {
    Ok(Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

/// The path with a leading `~` or `~/` replaced by the home folder. An export
/// writes a path in the home folder with `~` (decision 0037), so each kind
/// that has a Path field calls this function when it reads its settings. A
/// name with no slash, a path that starts with `/`, and a text such as
/// `~other/x` do not change.
pub(crate) fn expand_home(path: &str) -> String {
    expand_with(path, std::env::var("HOME").ok().as_deref())
}

/// [`expand_home`] with the home folder as an argument. A missing or empty
/// home folder changes nothing.
pub(crate) fn expand_with(path: &str, home: Option<&str>) -> String {
    let Some(home) = home
        .map(|home| home.trim_end_matches('/'))
        .filter(|home| !home.is_empty())
    else {
        return path.to_string();
    };
    match path {
        "~" => home.to_string(),
        _ => match path.strip_prefix("~/") {
            Some(rest) => format!("{home}/{rest}"),
            None => path.to_string(),
        },
    }
}

#[cfg(test)]
mod expand_tests {
    use super::*;

    #[test]
    fn a_leading_tilde_is_the_home_folder_and_other_text_does_not_change() {
        assert_eq!(expand_with("~/Music/a.wav", Some("/h")), "/h/Music/a.wav");
        assert_eq!(expand_with("~/Music", Some("/h/")), "/h/Music");
        assert_eq!(expand_with("~", Some("/h")), "/h");
        for same in ["Safari", "/Applications/A.app", "~other/x", "a/~/b", ""] {
            assert_eq!(expand_with(same, Some("/h")), same);
        }
        assert_eq!(expand_with("~/x", None), "~/x");
        assert_eq!(expand_with("~", Some("")), "~");
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            expand_home("~/Documents"),
            format!("{}/Documents", home.trim_end_matches('/'))
        );
    }
}
