//! A fake [`crate::backend::System`] for the tests of a kind module.
//!
//! It records the calls of the open-handler kinds. A call of another
//! operation panics, so a test sees the mistake. Add an operation here when
//! a new kind module needs it.

use crate::backend::{ShellChild, System as SystemPort};
use crate::system::{Error, Result};
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
}

/// A failure of one operation. `Code` is [`Error::System`] with code 42.
/// `operation` is `application`, `file`, `quit`, `url`, or `shell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fault {
    Code(&'static str),
    NotRunning,
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
}

impl Default for System {
    fn default() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            fault: Mutex::new(None),
            shell_end: Mutex::new(ShellEnd::Code(0)),
            waited: Arc::new(AtomicBool::new(false)),
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

    pub(crate) fn set_end(&self, end: ShellEnd) {
        *self.shell_end.lock().unwrap() = end;
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
    panic!("the open-kind test system does not implement this operation");
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
    fn send_hotkey(&self, _hotkey: &crate::hotkey::Hotkey) -> Result<()> {
        unused()
    }
    fn type_text(&self, _text: &str) -> Result<()> {
        unused()
    }
    fn paste_text(&self, _text: &str) -> Result<()> {
        unused()
    }
    fn media_key(&self, _key: crate::system::MediaKey) -> Result<()> {
        unused()
    }
    fn volume(&self) -> Result<crate::system::Volume> {
        unused()
    }
    fn set_volume_level(&self, _level: u8) -> Result<()> {
        unused()
    }
    fn set_muted(&self, _muted: bool) -> Result<()> {
        unused()
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
