#![allow(dead_code)]

use opencraft_actions::backend::{ShellChild, System};
use opencraft_actions::hotkey::Hotkey;
use opencraft_actions::system::{Error, MediaKey, Result, Volume};
use opencraft_engine::{Action, BindingId, ControlId, ControlKind, GestureKind, Request, Value};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Hotkey(String),
    Mouse(opencraft_actions::mouse::MouseEvent),
    MacFunction(opencraft_actions::mac_function::MacFunction),
    Text(String),
    Media(MediaKey),
    Level(u8),
    Muted(bool),
    Application(String),
    Url(String),
    Shell(String),
    /// A wait of a sequence, in milliseconds.
    Sleep(u64),
}

#[derive(Clone, Copy)]
pub enum Exit {
    Code(i32),
    Signal,
    Error,
}

type Gate = Arc<(Mutex<Option<Exit>>, Condvar)>;

pub struct Fake {
    pub permission: AtomicBool,
    pub permission_reads: AtomicUsize,
    pub asks: AtomicUsize,
    pub volume_reads: AtomicUsize,
    pub waiting: Arc<AtomicBool>,
    pub volume: Mutex<Volume>,
    pub failure: Mutex<Option<&'static str>>,
    calls: Mutex<Vec<Call>>,
    gate: Gate,
    /// The waits that started, and the waits that may end. See
    /// [`Fake::hold_sleeps`].
    pub sleeps_started: AtomicUsize,
    sleep_gate: Arc<(Mutex<Option<usize>>, Condvar)>,
}

impl Default for Fake {
    fn default() -> Self {
        Self {
            permission: AtomicBool::new(true),
            permission_reads: AtomicUsize::new(0),
            asks: AtomicUsize::new(0),
            volume_reads: AtomicUsize::new(0),
            waiting: Arc::new(AtomicBool::new(false)),
            volume: Mutex::new(Volume {
                level: 50,
                muted: false,
            }),
            failure: Mutex::new(None),
            calls: Mutex::new(vec![]),
            gate: Arc::new((Mutex::new(Some(Exit::Code(0))), Condvar::new())),
            sleeps_started: AtomicUsize::new(0),
            sleep_gate: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }
}

impl Fake {
    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
    pub fn clear_calls(&self) {
        self.calls.lock().unwrap().clear();
    }
    pub fn set_volume(&self, level: u8, muted: bool) {
        *self.volume.lock().unwrap() = Volume { level, muted };
    }
    pub fn fail(&self, operation: Option<&'static str>) {
        *self.failure.lock().unwrap() = operation;
    }
    fn check(&self, operation: &str) -> Result<()> {
        if *self.failure.lock().unwrap() == Some(operation) {
            Err(Error::System(42))
        } else {
            Ok(())
        }
    }
    fn record(&self, call: Call, operation: &str) -> Result<()> {
        self.calls.lock().unwrap().push(call);
        self.check(operation)
    }
    pub fn block_shell(&self) {
        *self.gate.0.lock().unwrap() = None;
    }
    /// Make each wait of a sequence stop until the test calls
    /// [`Fake::release_sleep`]. With no hold, a wait ends at once.
    pub fn hold_sleeps(&self) {
        *self.sleep_gate.0.lock().unwrap() = Some(0);
    }
    /// Let one held wait end.
    pub fn release_sleep(&self) {
        let (lock, wake) = &*self.sleep_gate;
        if let Some(permits) = lock.lock().unwrap().as_mut() {
            *permits += 1;
        }
        wake.notify_all();
    }
    pub fn finish_shell(&self, exit: Exit) {
        *self.gate.0.lock().unwrap() = Some(exit);
        self.gate.1.notify_all();
    }
}

struct FakeChild {
    gate: Gate,
    waiting: Arc<AtomicBool>,
}

impl ShellChild for FakeChild {
    fn wait(self: Box<Self>) -> Result<Option<i32>> {
        self.waiting.store(true, Ordering::SeqCst);
        let (lock, wake) = &*self.gate;
        // A timeout also releases a child if a test assertion fails.
        let (exit, _) = wake
            .wait_timeout_while(lock.lock().unwrap(), Duration::from_millis(800), |exit| {
                exit.is_none()
            })
            .unwrap();
        match exit.unwrap_or(Exit::Error) {
            Exit::Code(code) => Ok(Some(code)),
            Exit::Signal => Ok(None),
            Exit::Error => Err(Error::System(42)),
        }
    }
}

impl System for Fake {
    fn can_send_keys(&self) -> bool {
        self.permission_reads.fetch_add(1, Ordering::SeqCst);
        self.permission.load(Ordering::SeqCst)
    }
    fn ask_to_send_keys(&self) {
        self.asks.fetch_add(1, Ordering::SeqCst);
    }
    fn send_hotkey(&self, hotkey: &Hotkey) -> Result<()> {
        self.record(Call::Hotkey(hotkey.to_string()), "hotkey")
    }
    fn mouse_event(&self, event: opencraft_actions::mouse::MouseEvent) -> Result<()> {
        self.record(Call::Mouse(event), "mouse")
    }
    fn mac_function(&self, function: opencraft_actions::mac_function::MacFunction) -> Result<()> {
        self.record(Call::MacFunction(function), "mac-function")
    }
    fn type_text(&self, text: &str) -> Result<()> {
        self.record(Call::Text(text.into()), "text")
    }
    fn media_key(&self, key: MediaKey) -> Result<()> {
        self.record(Call::Media(key), "media")
    }
    fn volume(&self) -> Result<Volume> {
        self.volume_reads.fetch_add(1, Ordering::SeqCst);
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
    fn open_application(&self, application: &str) -> Result<()> {
        self.record(Call::Application(application.into()), "application")
    }
    fn open_url(&self, url: &str) -> Result<()> {
        self.record(Call::Url(url.into()), "url")
    }
    fn sleep(&self, duration: Duration) {
        self.calls
            .lock()
            .unwrap()
            .push(Call::Sleep(duration.as_millis() as u64));
        self.sleeps_started.fetch_add(1, Ordering::SeqCst);
        let (lock, wake) = &*self.sleep_gate;
        // A timeout also releases a wait if a test assertion fails.
        let (mut permits, _) = wake
            .wait_timeout_while(lock.lock().unwrap(), Duration::from_millis(800), |p| {
                *p == Some(0)
            })
            .unwrap();
        if let Some(permits) = permits.as_mut() {
            *permits = permits.saturating_sub(1);
        }
    }
    fn start_shell_command(&self, command: &str) -> Result<Box<dyn ShellChild>> {
        self.record(Call::Shell(command.into()), "shell")?;
        Ok(Box::new(FakeChild {
            gate: self.gate.clone(),
            waiting: self.waiting.clone(),
        }))
    }
}

pub fn binding(index: u8) -> BindingId {
    BindingId {
        page: 0,
        control: ControlId {
            kind: ControlKind::Knob,
            number: index,
        },
        gesture: GestureKind::TurnRight,
    }
}

pub fn action(kind: &str, fields: &[(&str, Value)]) -> Action {
    Action {
        kind: format!("opencraft.{kind}"),
        settings: fields
            .iter()
            .map(|(key, value)| ((*key).into(), value.clone()))
            .collect(),
    }
}

pub fn text_action(kind: &str, key: &str, value: &str) -> Action {
    action(kind, &[(key, Value::Text(value.into()))])
}

pub fn invoke(action: Action) -> Request {
    Request::Invoke {
        binding: binding(1),
        action,
    }
}

pub fn wait_until(mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_millis(500);
    while !check() {
        assert!(
            Instant::now() < deadline,
            "the fake operation did not finish"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
