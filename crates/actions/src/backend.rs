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
    fn mac_function(&self, _function: crate::mac_function::MacFunction) -> Result<()> {
        Err(crate::system::Error::NotSupported)
    }
    fn type_text(&self, text: &str) -> Result<()>;
    fn paste_text(&self, text: &str) -> Result<()>;
    fn media_key(&self, key: MediaKey) -> Result<()>;
    fn volume(&self) -> Result<Volume>;
    fn set_volume_level(&self, level: u8) -> Result<()>;
    fn set_muted(&self, muted: bool) -> Result<()>;
    /// None means that the default input has no writable master mute.
    fn microphone_muted(&self) -> Result<Option<bool>> {
        Err(system::Error::NotSupported)
    }
    fn set_microphone_muted(&self, _muted: bool) -> Result<()> {
        Err(system::Error::NotSupported)
    }
    /// Notify only on property changes. Drop the watch to remove listeners.
    fn watch_microphone(
        &self,
        _changed: std::sync::mpsc::Sender<()>,
    ) -> Result<Box<dyn system::AudioWatch>> {
        Err(system::Error::NotSupported)
    }
    fn output_devices(&self) -> Result<Vec<system::OutputDevice>> {
        Err(system::Error::NotSupported)
    }
    fn default_output(&self) -> Result<system::OutputDevice> {
        Err(system::Error::NotSupported)
    }
    fn set_output(&self, _id: &str) -> Result<()> {
        Err(system::Error::NotSupported)
    }
    fn watch_output(
        &self,
        _changed: std::sync::mpsc::Sender<()>,
    ) -> Result<Box<dyn system::AudioWatch>> {
        Err(system::Error::NotSupported)
    }
    fn open_application(&self, application: &str, arguments: &[String]) -> Result<()>;
    fn open_file(&self, path: &str) -> Result<()>;
    fn quit_application(&self, application: &str, force: bool) -> Result<()>;
    fn open_url(&self, url: &str, application: Option<&str>) -> Result<()>;
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
    fn mac_function(&self, function: crate::mac_function::MacFunction) -> Result<()> {
        system::mac_function(function)
    }
    fn paste_text(&self, text: &str) -> Result<()> {
        system::paste_text(text)
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
    fn microphone_muted(&self) -> Result<Option<bool>> {
        system::microphone_muted()
    }
    fn set_microphone_muted(&self, muted: bool) -> Result<()> {
        system::set_microphone_muted(muted)
    }
    fn watch_microphone(
        &self,
        changed: std::sync::mpsc::Sender<()>,
    ) -> Result<Box<dyn system::AudioWatch>> {
        system::watch_microphone(changed)
    }
    fn output_devices(&self) -> Result<Vec<system::OutputDevice>> {
        system::output_devices()
    }
    fn default_output(&self) -> Result<system::OutputDevice> {
        system::default_output()
    }
    fn set_output(&self, id: &str) -> Result<()> {
        system::set_output(id)
    }
    fn watch_output(
        &self,
        changed: std::sync::mpsc::Sender<()>,
    ) -> Result<Box<dyn system::AudioWatch>> {
        system::watch_output(changed)
    }
    fn open_application(&self, application: &str, arguments: &[String]) -> Result<()> {
        system::open_application(application, arguments)
    }
    fn open_file(&self, path: &str) -> Result<()> {
        system::open_file(path)
    }
    fn quit_application(&self, application: &str, force: bool) -> Result<()> {
        system::quit_application(application, force)
    }
    fn open_url(&self, url: &str, application: Option<&str>) -> Result<()> {
        system::open_url(url, application)
    }
    fn start_shell_command(&self, command: &str) -> Result<Box<dyn ShellChild>> {
        Ok(Box::new(system::start_shell_command(command)?))
    }
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
