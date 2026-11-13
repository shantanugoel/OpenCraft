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
    Paste(String),
    Media(MediaKey),
    Level(u8),
    Muted(bool),
    MicrophoneMuted(bool),
    Output(String),
    /// A sound: the file, the volume in percent, and the output.
    PlaySound(String, u8, Option<String>),
    StopSounds,
    Application(String, Vec<String>),
    File(String),
    Quit(String, bool),
    Url(String, Option<String>),
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
    pub output_reads: AtomicUsize,
    pub outputs: Mutex<Vec<opencraft_actions::system::OutputDevice>>,
    pub output: Mutex<String>,
    pub output_notifications: Mutex<Vec<std::sync::mpsc::Sender<()>>>,
    pub microphone_reads: AtomicUsize,
    pub microphone: Mutex<Option<bool>>,
    pub input_notifications: Mutex<Vec<std::sync::mpsc::Sender<()>>>,
    pub waiting: Arc<AtomicBool>,
    pub volume: Mutex<Volume>,
    pub failure: Mutex<Option<&'static str>>,
    calls: Mutex<Vec<Call>>,
    gate: Gate,
    /// The waits that started, and the waits that may end. See
    /// [`Fake::hold_sleeps`].
    pub sleeps_started: AtomicUsize,
    /// True: the Mac system function Sleep fails as macOS refuses it when
    /// a setting turns off all sleep.
    pub sleep_disabled: AtomicBool,
    sleep_gate: Arc<(Mutex<Option<usize>>, Condvar)>,
}

impl Default for Fake {
    fn default() -> Self {
        Self {
            permission: AtomicBool::new(true),
            permission_reads: AtomicUsize::new(0),
            asks: AtomicUsize::new(0),
            volume_reads: AtomicUsize::new(0),
            output_reads: AtomicUsize::new(0),
            outputs: Mutex::new(vec![
                opencraft_actions::system::OutputDevice {
                    id: "a".into(),
                    name: "Speakers".into(),
                },
                opencraft_actions::system::OutputDevice {
                    id: "b".into(),
                    name: "Headphones".into(),
                },
            ]),
            output: Mutex::new("a".into()),
            output_notifications: Mutex::new(Vec::new()),
            microphone_reads: AtomicUsize::new(0),
            microphone: Mutex::new(Some(false)),
            input_notifications: Mutex::new(Vec::new()),
            waiting: Arc::new(AtomicBool::new(false)),
            volume: Mutex::new(Volume {
                level: 50,
                muted: false,
            }),
            failure: Mutex::new(None),
            calls: Mutex::new(vec![]),
            gate: Arc::new((Mutex::new(Some(Exit::Code(0))), Condvar::new())),
            sleeps_started: AtomicUsize::new(0),
            sleep_disabled: AtomicBool::new(false),
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

struct FakeWatch;
impl opencraft_actions::system::AudioWatch for FakeWatch {}

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
        self.record(Call::MacFunction(function), "mac-function")?;
        if function == opencraft_actions::mac_function::MacFunction::Sleep
            && self.sleep_disabled.load(Ordering::SeqCst)
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
    fn microphone_muted(&self) -> Result<Option<bool>> {
        self.microphone_reads.fetch_add(1, Ordering::SeqCst);
        self.check("microphone")?;
        Ok(*self.microphone.lock().unwrap())
    }
    fn set_microphone_muted(&self, muted: bool) -> Result<()> {
        self.record(Call::MicrophoneMuted(muted), "set-microphone")?;
        *self.microphone.lock().unwrap() = Some(muted);
        Ok(())
    }
    fn watch_microphone(
        &self,
        changed: std::sync::mpsc::Sender<()>,
    ) -> Result<Box<dyn opencraft_actions::system::AudioWatch>> {
        self.input_notifications.lock().unwrap().push(changed);
        Ok(Box::new(FakeWatch))
    }
    fn output_devices(&self) -> Result<Vec<opencraft_actions::system::OutputDevice>> {
        self.check("outputs")?;
        Ok(self.outputs.lock().unwrap().clone())
    }
    fn default_output(&self) -> Result<opencraft_actions::system::OutputDevice> {
        self.output_reads.fetch_add(1, Ordering::SeqCst);
        self.check("output")?;
        let id = self.output.lock().unwrap().clone();
        self.outputs
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or(Error::NotSupported)
    }
    fn set_output(&self, id: &str) -> Result<()> {
        self.record(Call::Output(id.into()), "set-output")?;
        *self.output.lock().unwrap() = id.into();
        Ok(())
    }
    fn watch_output(
        &self,
        changed: std::sync::mpsc::Sender<()>,
    ) -> Result<Box<dyn opencraft_actions::system::AudioWatch>> {
        self.output_notifications.lock().unwrap().push(changed);
        Ok(Box::new(FakeWatch))
    }
    fn play_sound(&self, file: &std::path::Path, volume: u8, device: Option<&str>) -> Result<()> {
        self.record(
            Call::PlaySound(
                file.to_string_lossy().into_owned(),
                volume,
                device.map(Into::into),
            ),
            "play-sound",
        )
    }
    fn stop_sounds(&self) -> Result<()> {
        self.record(Call::StopSounds, "stop-sounds")
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
        folder: None,
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
