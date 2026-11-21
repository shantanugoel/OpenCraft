//! The system interface of the handlers. Tests give it a fake implementation.

use crate::hotkey::Hotkey;
use crate::system::{self, platform, MediaKey, Result, Volume};
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
    /// Press modifiers first, then the key if any.
    fn hotkey_down(&self, hotkey: &Hotkey) -> Result<()>;
    /// Release the key first, then the modifiers in reverse order.
    fn hotkey_up(&self, hotkey: &Hotkey) -> Result<()>;
    fn mouse_event(&self, event: crate::system::MouseEvent) -> Result<()> {
        let _ = event;
        Err(crate::system::Error::NotSupported)
    }
    fn mac_function(&self, _function: crate::system::MacFunction) -> Result<()> {
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
    /// Start to play a sound file and return at once. The volume is a percent
    /// of the system volume. With no device, the sound uses the default output.
    fn play_sound(
        &self,
        _file: &std::path::Path,
        _volume: u8,
        _device: Option<&str>,
    ) -> Result<()> {
        Err(system::Error::NotSupported)
    }
    /// Stop each sound that [`System::play_sound`] started.
    fn stop_sounds(&self) -> Result<()> {
        Err(system::Error::NotSupported)
    }
    /// The paths of the applications registered for `bundle_id`. An empty
    /// list means the caller opens the stored path. The default is empty,
    /// so a test system does not need this operation.
    fn application_copies(&self, bundle_id: &str) -> Vec<String> {
        let _ = bundle_id;
        Vec::new()
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

/// The system of the computer that runs the program. Each call goes to the
/// platform code.
pub struct RealSystem;

impl System for RealSystem {
    fn can_send_keys(&self) -> bool {
        platform::can_send_keys()
    }
    fn ask_to_send_keys(&self) {
        platform::ask_to_send_keys();
    }
    fn send_hotkey(&self, hotkey: &Hotkey) -> Result<()> {
        platform::send_hotkey(hotkey)
    }
    fn hotkey_down(&self, hotkey: &Hotkey) -> Result<()> {
        platform::hotkey_down(hotkey)
    }
    fn hotkey_up(&self, hotkey: &Hotkey) -> Result<()> {
        platform::hotkey_up(hotkey)
    }
    fn type_text(&self, text: &str) -> Result<()> {
        platform::type_text(text)
    }
    fn mouse_event(&self, event: crate::system::MouseEvent) -> Result<()> {
        platform::mouse_event(event)
    }
    fn mac_function(&self, function: crate::system::MacFunction) -> Result<()> {
        platform::mac_function(function)
    }
    fn paste_text(&self, text: &str) -> Result<()> {
        platform::paste_text(text)
    }
    fn media_key(&self, key: MediaKey) -> Result<()> {
        platform::media_key(key)
    }
    fn volume(&self) -> Result<Volume> {
        platform::volume()
    }
    fn set_volume_level(&self, level: u8) -> Result<()> {
        platform::set_volume_level(level.min(100))
    }
    fn set_muted(&self, muted: bool) -> Result<()> {
        platform::set_muted(muted)
    }
    fn microphone_muted(&self) -> Result<Option<bool>> {
        platform::microphone_muted()
    }
    fn set_microphone_muted(&self, muted: bool) -> Result<()> {
        platform::set_microphone_muted(muted)
    }
    fn watch_microphone(
        &self,
        changed: std::sync::mpsc::Sender<()>,
    ) -> Result<Box<dyn system::AudioWatch>> {
        platform::watch_microphone(changed)
    }
    fn output_devices(&self) -> Result<Vec<system::OutputDevice>> {
        platform::output_devices()
    }
    fn default_output(&self) -> Result<system::OutputDevice> {
        platform::default_output()
    }
    fn set_output(&self, id: &str) -> Result<()> {
        platform::set_output(id)
    }
    fn watch_output(
        &self,
        changed: std::sync::mpsc::Sender<()>,
    ) -> Result<Box<dyn system::AudioWatch>> {
        platform::watch_output(changed)
    }
    fn play_sound(&self, file: &std::path::Path, volume: u8, device: Option<&str>) -> Result<()> {
        platform::play_sound(file, volume, device)
    }
    fn stop_sounds(&self) -> Result<()> {
        platform::stop_sounds()
    }
    fn application_copies(&self, bundle_id: &str) -> Vec<String> {
        platform::application_copies(bundle_id)
    }
    fn open_application(&self, application: &str, arguments: &[String]) -> Result<()> {
        platform::open_application(application, arguments)
    }
    fn open_file(&self, path: &str) -> Result<()> {
        platform::open_file(path)
    }
    fn quit_application(&self, application: &str, force: bool) -> Result<()> {
        platform::quit_application(application, force)
    }
    fn open_url(&self, url: &str, application: Option<&str>) -> Result<()> {
        platform::open_url(url, application)
    }
    fn start_shell_command(&self, command: &str) -> Result<Box<dyn ShellChild>> {
        Ok(Box::new(system::start_shell_command(command)?))
    }
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
