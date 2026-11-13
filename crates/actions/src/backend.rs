//! The system interface of the handlers. Tests give it a fake implementation.

use crate::hotkey::Hotkey;
use crate::system::{self, MediaKey, Result, Volume};
use std::process::Child;
use std::time::Duration;

/// A shell command that the handler waits for on a separate thread.
pub trait ShellChild: Send + 'static {
    /// The exit code, or `None` if a signal stopped the command.
    fn wait(self: Box<Self>) -> Result<Option<i32>>;
}

impl ShellChild for Child {
    fn wait(mut self: Box<Self>) -> Result<Option<i32>> {
        Ok(Child::wait(&mut self)?.code())
    }
}

/// The operations that a built-in handler can do on the computer.
pub trait System: Send + Sync + 'static {
    fn can_send_keys(&self) -> bool;
    fn ask_to_send_keys(&self);
    fn send_hotkey(&self, hotkey: &Hotkey) -> Result<()>;
    fn mouse_event(&self, event: crate::mouse::MouseEvent) -> Result<()> {
        let _ = event;
        Err(crate::system::Error::NotSupported)
    }
    fn type_text(&self, text: &str) -> Result<()>;
    fn media_key(&self, key: MediaKey) -> Result<()>;
    fn volume(&self) -> Result<Volume>;
    fn set_volume_level(&self, level: u8) -> Result<()>;
    fn set_muted(&self, muted: bool) -> Result<()>;
    fn open_application(&self, application: &str) -> Result<()>;
    fn open_url(&self, url: &str) -> Result<()>;
    fn start_shell_command(&self, command: &str) -> Result<Box<dyn ShellChild>>;
    /// Wait for `duration`. A sequence waits between two actions with this
    /// function, so that a test can control the wait.
    fn sleep(&self, duration: Duration);
}

pub(crate) struct RealSystem;

impl System for RealSystem {
    fn can_send_keys(&self) -> bool {
        system::can_send_keys()
    }
    fn ask_to_send_keys(&self) {
        system::ask_to_send_keys();
    }
    fn send_hotkey(&self, hotkey: &Hotkey) -> Result<()> {
        system::send_hotkey(hotkey)
    }
    fn type_text(&self, text: &str) -> Result<()> {
        system::type_text(text)
    }
    fn mouse_event(&self, event: crate::mouse::MouseEvent) -> Result<()> {
        system::mouse_event(event)
    }
    fn media_key(&self, key: MediaKey) -> Result<()> {
        system::media_key(key)
    }
    fn volume(&self) -> Result<Volume> {
        system::volume()
    }
    fn set_volume_level(&self, level: u8) -> Result<()> {
        system::set_volume_level(level)
    }
    fn set_muted(&self, muted: bool) -> Result<()> {
        system::set_muted(muted)
    }
    fn open_application(&self, application: &str) -> Result<()> {
        system::open_application(application)
    }
    fn open_url(&self, url: &str) -> Result<()> {
        system::open_url(url)
    }
    fn start_shell_command(&self, command: &str) -> Result<Box<dyn ShellChild>> {
        Ok(Box::new(system::start_shell_command(command)?))
    }
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
