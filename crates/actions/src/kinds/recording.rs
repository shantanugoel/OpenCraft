//! A fake [`crate::backend::System`] for the tests of a kind module.
//!
//! It records each call that a kind module makes. A call that no kind uses
//! panics, so a test sees the mistake.

use crate::backend::{ShellChild, System as SystemPort};
use crate::system::{Error, MediaKey, Result, Volume};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Call {
    Application(String, Vec<String>),
    File(String),
    Quit(String, bool),
    Url(String, Option<String>),
    Shell(String),
    Hotkey(String),
    Text(String),
    Paste(String),
    Media(MediaKey),
    Mouse(crate::mouse::MouseEvent),
    MacFunction(crate::mac_function::MacFunction),
    Level(u8),
    Muted(bool),
}

/// A failure of one operation. `Code` is [`Error::System`] with code 42.
/// `operation` names the call: `application`, `file`, `quit`, `url`,
/// `shell`, `hotkey`, `text`, `paste`, `media`, `mouse`, `mac-function`,
/// `volume`, `level`, or `muted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fault {
    Code(&'static str),
    NotRunning,
    /// Sleep fails because macOS does not allow it.
    SleepNotAllowed,
}

/// The result of [`crate::backend::ShellChild::wait`] for the next command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellEnd {
    Code(i32),
    Signal,
    WaitFailed,
}

pub(crate) struct System {
    calls: Mutex<Vec<Call>>,
    fault: Mutex<Option<Fault>>,
    shell_end: Mutex<ShellEnd>,
    waited: Arc<AtomicBool>,
    volume: Mutex<Volume>,
    /// Paths that [`SystemPort::application_copies`] returns.
    copies: Mutex<Vec<String>>,
}

impl Default for System {
    fn default() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            fault: Mutex::new(None),
            shell_end: Mutex::new(ShellEnd::Code(0)),
            waited: Arc::new(AtomicBool::new(false)),
            volume: Mutex::new(Volume {
                level: 50,
                muted: false,
            }),
            copies: Mutex::new(Vec::new()),
        }
    }
}

impl System {
    pub(crate) fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    pub(crate) fn fail(&self, fault: Fault) {
        *self.fault.lock().unwrap() = Some(fault);
    }

    /// Set the level and the mute that [`SystemPort::volume`] returns.
    pub(crate) fn set_volume(&self, level: u8, muted: bool) {
        *self.volume.lock().unwrap() = Volume { level, muted };
    }

    pub(crate) fn set_end(&self, end: ShellEnd) {
        *self.shell_end.lock().unwrap() = end;
    }

    /// The registered copies of a bundle identifier, for the next open.
    pub(crate) fn set_copies(&self, copies: Vec<String>) {
        *self.copies.lock().unwrap() = copies;
    }

    /// True after the waiter of the last command has returned from `wait`.
    pub(crate) fn wait_until_finished(&self) -> bool {
        let start = Instant::now();
        while !self.waited.load(Ordering::SeqCst) {
            if start.elapsed() > Duration::from_secs(2) {
                return false;
            }
            thread::sleep(Duration::from_millis(5));
        }
        true
    }

    fn record(&self, call: Call, operation: &str) -> Result<()> {
        self.calls.lock().unwrap().push(call);
        self.check(operation)
    }

    fn check(&self, operation: &str) -> Result<()> {
        match *self.fault.lock().unwrap() {
            Some(Fault::Code(name)) if name == operation => Err(Error::System(42)),
            Some(Fault::NotRunning) if operation == "quit" => Err(Error::NotRunning),
            _ => Ok(()),
        }
    }
}

fn unused() -> ! {
    panic!("this kind test system does not implement this operation");
}

struct ScriptedChild {
    end: ShellEnd,
    waited: Arc<AtomicBool>,
}

impl ShellChild for ScriptedChild {
    fn wait(self: Box<Self>) -> Result<Option<i32>> {
        let result = match self.end {
            ShellEnd::Code(code) => Ok(Some(code)),
            ShellEnd::Signal => Ok(None),
            ShellEnd::WaitFailed => Err(Error::System(42)),
        };
        self.waited.store(true, Ordering::SeqCst);
        result
    }
}

impl SystemPort for System {
    fn can_send_keys(&self) -> bool {
        unused()
    }
    fn ask_to_send_keys(&self) {
        unused()
    }
    fn send_hotkey(&self, hotkey: &crate::hotkey::Hotkey) -> Result<()> {
        self.record(Call::Hotkey(hotkey.to_string()), "hotkey")
    }
    fn mouse_event(&self, event: crate::mouse::MouseEvent) -> Result<()> {
        self.record(Call::Mouse(event), "mouse")
    }
    fn mac_function(&self, function: crate::mac_function::MacFunction) -> Result<()> {
        self.record(Call::MacFunction(function), "mac-function")?;
        if function == crate::mac_function::MacFunction::Sleep
            && *self.fault.lock().unwrap() == Some(Fault::SleepNotAllowed)
        {
            return Err(Error::SleepNotAllowed);
        }
        Ok(())
    }
    fn type_text(&self, text: &str) -> Result<()> {
        self.record(Call::Text(text.into()), "text")
    }
    fn paste_text(&self, text: &str) -> Result<()> {
        self.record(Call::Paste(text.into()), "paste")
    }
    fn media_key(&self, key: MediaKey) -> Result<()> {
        self.record(Call::Media(key), "media")
    }
    fn volume(&self) -> Result<Volume> {
        self.check("volume")?;
        Ok(*self.volume.lock().unwrap())
    }
    fn set_volume_level(&self, level: u8) -> Result<()> {
        self.record(Call::Level(level), "level")?;
        self.volume.lock().unwrap().level = level;
        Ok(())
    }
    fn set_muted(&self, muted: bool) -> Result<()> {
        self.record(Call::Muted(muted), "muted")?;
        self.volume.lock().unwrap().muted = muted;
        Ok(())
    }
    fn application_copies(&self, _bundle_id: &str) -> Vec<String> {
        self.copies.lock().unwrap().clone()
    }
    fn open_application(&self, application: &str, arguments: &[String]) -> Result<()> {
        self.record(
            Call::Application(application.into(), arguments.to_vec()),
            "application",
        )
    }
    fn open_file(&self, path: &str) -> Result<()> {
        self.record(Call::File(path.into()), "file")
    }
    fn quit_application(&self, application: &str, force: bool) -> Result<()> {
        self.record(Call::Quit(application.into(), force), "quit")
    }
    fn open_url(&self, url: &str, application: Option<&str>) -> Result<()> {
        self.record(
            Call::Url(url.into(), application.map(str::to_string)),
            "url",
        )
    }
    fn start_shell_command(&self, command: &str) -> Result<Box<dyn ShellChild>> {
        self.record(Call::Shell(command.into()), "shell")?;
        self.waited.store(false, Ordering::SeqCst);
        Ok(Box::new(ScriptedChild {
            end: *self.shell_end.lock().unwrap(),
            waited: Arc::clone(&self.waited),
        }))
    }
    fn sleep(&self, _duration: Duration) {
        unused()
    }
}
