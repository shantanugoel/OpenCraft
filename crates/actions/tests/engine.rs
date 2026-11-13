//! End-to-end tests: the built-in handlers and the engine logic together,
//! with a fake device, a fake system, and a temporary profiles directory.
//!
//! The test moves each request of the engine to its handler, and each
//! message of a handler to the engine, in order. Only a sequence starts a
//! thread of its own. The kinds use the handler interface only (rule 8): the
//! engine does not know them.

mod support;

use opencraft_actions::handlers_with_system;
use opencraft_core::device::DESCRIPTORS;
use opencraft_core::{Clock, Connector, Device, FakeTransport, Link, Transport};
use opencraft_engine::{
    Action, BindingId, Command, ControlId, ControlKind, Engine, Event, GestureKind, Handler,
    HandlerPort, KindDescription, Message, MessageSender, ProfilesDir, Request, RgbImage, Settings,
    SurfaceId, SurfaceKind, Value,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use support::Fake;
use tempfile::TempDir;

const KEY_3: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 3);
const NESTED: &str = "test.nested";

// --- The fake device ------------------------------------------------------------------

/// A fake transport that the test and the engine share.
struct Shared(Rc<RefCell<FakeTransport>>);

impl Transport for Shared {
    fn write(&mut self, report: &[u8]) -> opencraft_core::Result<()> {
        self.0.borrow_mut().write(report)
    }
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> opencraft_core::Result<Option<usize>> {
        self.0.borrow_mut().read(buf, timeout)
    }
    fn get_feature_report(&mut self, buf: &mut [u8]) -> opencraft_core::Result<usize> {
        self.0.borrow_mut().get_feature_report(buf)
    }
}

/// A connector that gives one device, one time.
struct OneDevice(Option<Shared>);

impl Connector for OneDevice {
    type Transport = Shared;
    fn connect(&mut self) -> opencraft_core::Result<Option<Device<Shared>>> {
        Ok(self.0.take().map(|t| Device::new(t, &DESCRIPTORS[0])))
    }
}

/// A clock that moves only when the link sleeps. The tests need no real
/// wait.
struct StillClock(Instant);

impl Clock for StillClock {
    fn now(&self) -> Instant {
        self.0
    }
    fn sleep(&mut self, duration: Duration) {
        self.0 += duration;
    }
}

// --- The fake handler -------------------------------------------------------------

/// The handler of the kind `test.nested`: it keeps each request.
struct Recorder(Arc<Mutex<Vec<Request>>>);

impl Handler for Recorder {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![KindDescription {
            kind: NESTED.into(),
            name: "Nested test action".into(),
            fields: Vec::new(),
            ..Default::default()
        }]
    }
    fn start(&mut self, _: MessageSender) {}
    fn handle(&mut self, request: Request) {
        self.0.lock().unwrap().push(request);
    }
}

fn nested(number: i64) -> Action {
    Action {
        kind: NESTED.into(),
        settings: Settings::from([("number".into(), Value::Integer(number))]),
    }
}

fn built_in(kind: &str, fields: Vec<(&str, Value)>) -> Action {
    Action {
        kind: format!("opencraft.{kind}"),
        settings: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
    }
}

// --- The rig ---------------------------------------------------------------------------

type Sink = Box<dyn FnMut(Event)>;

struct World {
    engine: Engine<OneDevice, Sink, StillClock>,
    handlers: Vec<(Box<dyn Handler>, Receiver<Request>)>,
    messages: Receiver<Message>,
    events: Rc<RefCell<Vec<Event>>>,
    device: Rc<RefCell<FakeTransport>>,
    nested: Arc<Mutex<Vec<Request>>>,
    _dir: TempDir,
}

impl World {
    /// The engine with all built-in handlers and the recorder, on an empty
    /// profiles directory, with the device connected.
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let nested = Arc::new(Mutex::new(Vec::new()));
        let mut all = handlers_with_system(Arc::new(Fake::default()));
        all.push(Box::new(Recorder(nested.clone())));
        let (sender, messages) = mpsc::channel();
        let (mut ports, mut handlers) = (Vec::new(), Vec::new());
        for mut handler in all {
            let (requests, receiver) = mpsc::channel();
            ports.push(HandlerPort {
                kinds: handler.kinds(),
                sender: requests,
            });
            handler.start(sender.clone());
            handlers.push((handler, receiver));
        }
        let device = Rc::new(RefCell::new(FakeTransport::new()));
        let link = Link::with_clock(
            OneDevice(Some(Shared(device.clone()))),
            StillClock(Instant::now()),
        );
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink_events = events.clone();
        let sink: Sink = Box::new(move |event| sink_events.borrow_mut().push(event));
        let engine = Engine::with_handlers(
            link,
            ProfilesDir::new(dir.path()),
            &DESCRIPTORS[0],
            ports,
            sink,
        );
        let mut world = Self {
            engine,
            handlers,
            messages,
            events,
            device,
            nested,
            _dir: dir,
        };
        for _ in 0..10 {
            world.engine.poll(Duration::ZERO);
        }
        assert!(world.engine.link().is_connected());
        world.pump();
        world
    }

    /// Move each request to its handler and each message to the engine,
    /// until no request and no message waits.
    fn pump(&mut self) {
        loop {
            let mut busy = false;
            for (handler, requests) in &mut self.handlers {
                for request in requests.try_iter() {
                    handler.handle(request);
                    busy = true;
                }
            }
            for message in self.messages.try_iter() {
                self.engine.handler_message(message);
                busy = true;
            }
            if !busy {
                break;
            }
        }
    }

    /// Pump until `done` is true. A sequence sends its messages from its
    /// own thread.
    fn pump_until(&mut self, done: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(1);
        while !done(self) {
            assert!(Instant::now() < deadline, "the messages did not come");
            if let Ok(message) = self.messages.recv_timeout(Duration::from_millis(10)) {
                self.engine.handler_message(message);
            }
            self.pump();
        }
    }

    fn command(&mut self, command: Command) {
        assert!(self.engine.command(command));
        self.pump();
    }

    fn bind_key_3(&mut self, action: Action) {
        let binding = key_3(self.engine.active_page());
        self.command(Command::SetBinding {
            binding,
            action: Some(action),
        });
    }

    /// Press key 3 on the device.
    fn press_key_3(&mut self) {
        let mut report = vec![0u8; 512];
        report[..3].copy_from_slice(b"ACK");
        report[5..7].copy_from_slice(b"OK");
        report[9] = 3;
        report[10] = 1;
        self.device.borrow_mut().push_input(report);
        self.engine.poll(Duration::ZERO);
        self.pump();
    }

    fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut *self.events.borrow_mut())
    }

    /// The newest image of key face 3 that the shell got.
    fn key_3_image(&mut self) -> Arc<RgbImage> {
        self.take_events()
            .into_iter()
            .rev()
            .find_map(|e| match e {
                Event::Surface(state) if state.id == KEY_3 => Some(state.image),
                _ => None,
            })
            .expect("a new image of key face 3")
    }

    fn take_nested(&self) -> Vec<Request> {
        std::mem::take(&mut *self.nested.lock().unwrap())
    }
}

#[test]
fn device_actions_use_the_handler_path_and_do_not_change_brightness() {
    for (kind, state) in [
        ("screen-off", opencraft_engine::DisplayState::ScreenOff),
        ("device-sleep", opencraft_engine::DisplayState::DeviceSleep),
    ] {
        let mut world = World::new();
        world.bind_key_3(built_in(kind, vec![]));
        let before = world.engine.device_settings().clone();
        world.press_key_3();
        assert_eq!(world.engine.display_state(), state);
        assert_eq!(world.engine.device_settings(), &before);
        world.command(Command::WakeDevice);
        assert_eq!(
            world.engine.display_state(),
            opencraft_engine::DisplayState::Awake
        );
    }
}

#[test]
fn a_brightness_key_uses_the_same_settings_event_as_the_window() {
    let mut world = World::new();
    world.bind_key_3(built_in(
        "device-brightness",
        vec![
            ("operation", Value::Text("set".into())),
            ("value", Value::Integer(35)),
        ],
    ));
    world.take_events();
    world.press_key_3();
    assert_eq!(world.engine.device_settings().brightness, 35);
    assert!(world
        .take_events()
        .iter()
        .any(|e| matches!(e, Event::DeviceSettings(s) if s.brightness == 35)));
    world.bind_key_3(built_in(
        "device-brightness",
        vec![("operation", Value::Text("down".into()))],
    ));
    world.press_key_3();
    assert_eq!(world.engine.device_settings().brightness, 30);
}

fn key_3(page: usize) -> BindingId {
    BindingId {
        page,
        control: ControlId::new(ControlKind::Key, 3),
        gesture: GestureKind::Press,
    }
}

fn invoked(binding: BindingId, action: Action) -> Request {
    Request::Invoke { binding, action }
}

/// True if the edges of `image` have the frame of the state "on": white on a
/// surface with no colour.
fn has_frame(image: &RgbImage) -> bool {
    let (w, h) = image.dimensions();
    [
        (0, 0),
        (2, h / 2),
        (w / 2, 3),
        (w - 1, h - 1),
        (w - 3, h / 2),
    ]
    .iter()
    .all(|&(x, y)| image.get_pixel(x, y).0 == [255, 255, 255])
}

// --- The tests -----------------------------------------------------------------------

/// The engine sends each built-in kind with its group, its help text, and
/// its need for the Accessibility permission, in the order of the library
/// (decision 0029, section 11). The shell has no list of kinds (rule 8): the
/// library and the kind menu come only from this event.
#[test]
fn the_engine_sends_the_built_in_kinds_in_their_library_groups() {
    let mut world = World::new();
    let kinds = world
        .take_events()
        .into_iter()
        .find_map(|e| match e {
            Event::Kinds(kinds) => Some(kinds),
            _ => None,
        })
        .expect("the kinds");
    let library: Vec<_> = kinds
        .iter()
        .filter(|k| k.kind != NESTED)
        .map(|k| {
            (
                k.name.as_str(),
                k.group.as_str(),
                k.help.as_deref(),
                k.needs_accessibility,
            )
        })
        .collect();
    let accessibility = Some("Accessibility");
    assert_eq!(
        library,
        [
            ("Open an application", "System", None, false),
            ("Quit an application", "System", None, false),
            ("Open a URL", "System", None, false),
            ("Run a shell command", "System", None, false),
            ("Send a hotkey", "System", accessibility, true),
            ("Type text", "System", accessibility, true),
            ("Media key", "Media", accessibility, true),
            ("Mouse event", "System", accessibility, true),
            ("Mac system function", "System", accessibility, true),
            ("Sound volume", "Media", Some("Knob"), false),
            ("Screen off", "Device", Some("Computer wake"), false),
            ("Device sleep", "Device", Some("Computer wake"), false),
            ("Device brightness", "Device", Some("Knob"), false),
            ("Microphone mute", "Media", None, false),
            ("Sound output", "Media", None, false),
            ("Play a sound", "Media", None, false),
            ("Stop all sound", "Media", None, false),
            ("Change the page", "Navigation", None, false),
            ("Select a profile", "Navigation", None, false),
            ("Toggle", "Logic", None, false),
            ("Sequence of actions", "Logic", None, false),
        ]
    );
}

/// Each built-in kind has the icon of decision 0029, section 20, and the
/// icon set of the engine has it (decision 0033). The shell shows the icon
/// on the kind tile from this data only (rule 8).
#[test]
fn each_built_in_kind_has_an_icon_of_the_icon_set() {
    let mut world = World::new();
    let kinds = world
        .take_events()
        .into_iter()
        .find_map(|e| match e {
            Event::Kinds(kinds) => Some(kinds),
            _ => None,
        })
        .expect("the kinds");
    let icons: Vec<_> = kinds
        .iter()
        .filter(|k| k.kind != NESTED)
        .map(|k| (k.name.as_str(), k.icon.as_deref().unwrap_or("")))
        .collect();
    assert_eq!(
        icons,
        [
            ("Open an application", "app-window"),
            ("Quit an application", "circle-x"),
            ("Open a URL", "globe"),
            ("Run a shell command", "terminal"),
            ("Send a hotkey", "keyboard"),
            ("Type text", "type"),
            ("Media key", "play"),
            ("Mouse event", "mouse-pointer-2"),
            ("Mac system function", "monitor"),
            ("Sound volume", "volume-2"),
            ("Screen off", "monitor-off"),
            ("Device sleep", "moon"),
            ("Device brightness", "sun"),
            ("Microphone mute", "mic"),
            ("Sound output", "speaker"),
            ("Play a sound", "music"),
            ("Stop all sound", "circle-stop"),
            ("Change the page", "arrow-right"),
            ("Select a profile", "layers"),
            ("Toggle", "toggle-left"),
            ("Sequence of actions", "list"),
        ]
    );
    for (_, icon) in icons {
        assert!(opencraft_engine::icon_set().get(icon).is_some(), "{icon}");
    }
}

#[test]
fn a_toggle_on_a_key_does_its_action_and_shows_its_state_with_a_frame() {
    let mut world = World::new();
    let toggle = built_in(
        "toggle",
        vec![
            ("on", nested(1).to_value()),
            ("off", nested(0).to_value()),
            ("on_label", Value::Text("On".into())),
        ],
    );
    world.bind_key_3(toggle);
    // Appear: the toggle shows the state "off", with no frame.
    assert!(!has_frame(&world.key_3_image()));
    world.device.borrow_mut().take_writes();

    // The first press: the action for "on" reaches its handler, with the
    // binding of the toggle, and the key face gets the frame.
    world.press_key_3();
    assert_eq!(world.take_nested(), [invoked(key_3(0), nested(1))]);
    assert!(has_frame(&world.key_3_image()));
    assert!(!world.device.borrow_mut().take_writes().is_empty());

    // The second press: the action for "off", and no frame.
    world.press_key_3();
    assert_eq!(world.take_nested(), [invoked(key_3(0), nested(0))]);
    assert!(!has_frame(&world.key_3_image()));
    assert!(!world.device.borrow_mut().take_writes().is_empty());

    // Only the press of the user counts as an Invoke of the engine.
    assert_eq!(world.engine.counters().invokes_sent, 2);
}

#[test]
fn a_toggle_shows_its_state_again_after_a_page_change_and_back() {
    let mut world = World::new();
    world.bind_key_3(built_in("toggle", vec![("on", nested(1).to_value())]));
    world.press_key_3();
    assert!(has_frame(&world.key_3_image()));
    world.command(Command::AddPage { copy_of: None });
    assert!(!has_frame(&world.key_3_image()));
    world.command(Command::SelectPage { page: 0 });
    assert!(has_frame(&world.key_3_image()));
}

#[test]
fn a_sequence_on_a_key_does_its_actions_in_order() {
    let mut world = World::new();
    let sequence = built_in(
        "sequence",
        vec![
            (
                "actions",
                Action::list_to_value(&[nested(1), nested(2), nested(3)]),
            ),
            ("delay_ms", Value::Integer(0)),
        ],
    );
    world.bind_key_3(sequence);
    world.press_key_3();
    world.pump_until(|w| w.nested.lock().unwrap().len() == 3);
    assert_eq!(
        world.take_nested(),
        [1, 2, 3].map(|n| invoked(key_3(0), nested(n)))
    );
}

#[test]
fn the_page_kind_changes_the_page_through_the_engine() {
    let mut world = World::new();
    world.bind_key_3(built_in("page", vec![]));
    // The copy has the same binding. The device shows the first page again.
    world.command(Command::AddPage { copy_of: Some(0) });
    world.command(Command::SelectPage { page: 0 });

    world.press_key_3();
    assert_eq!(world.engine.active_page(), 1);
    world.press_key_3();
    assert_eq!(
        world.engine.active_page(),
        0,
        "after the last page, the first"
    );

    // A page number from 1, for the user.
    world.bind_key_3(built_in(
        "page",
        vec![
            ("target", Value::Text("number".into())),
            ("page", Value::Integer(2)),
        ],
    ));
    world.press_key_3();
    assert_eq!(world.engine.active_page(), 1);
}

// The test compares key faces with and with no label. Only macOS draws the
// text of a label (decision 0010).
#[cfg(target_os = "macos")]
#[test]
fn a_page_key_shows_its_page_number_only_when_the_setting_is_on() {
    let mut world = World::new();
    let key = |show: Option<bool>| {
        built_in(
            "page",
            show.map(|on| ("show_number", Value::Boolean(on)))
                .into_iter()
                .collect(),
        )
    };
    // No setting, as in an old profile: the key face is the same on each page.
    world.bind_key_3(key(None));
    let plain = world.key_3_image();
    world.command(Command::AddPage { copy_of: Some(0) });
    assert_eq!(world.engine.active_page(), 1);
    assert_eq!(*world.key_3_image(), *plain);
    // On: each page shows its own number, and the number comes back after a
    // change of the page.
    world.bind_key_3(key(Some(true)));
    let two = world.key_3_image();
    assert_ne!(*two, *plain);
    world.command(Command::SelectPage { page: 0 });
    world.bind_key_3(key(Some(true)));
    let one = world.key_3_image();
    assert_ne!(*one, *plain);
    assert_ne!(*one, *two);
    world.command(Command::SelectPage { page: 1 });
    assert_eq!(*world.key_3_image(), *two);
    // Off: the number goes away.
    world.bind_key_3(key(Some(false)));
    assert_eq!(*world.key_3_image(), *plain);
}

#[test]
fn the_profile_kind_selects_a_profile_through_the_engine() {
    let mut world = World::new();
    world.command(Command::AddProfile {
        name: "Video calls".into(),
        copy_of: None,
    });
    world.command(Command::SelectProfile {
        profile: "default".into(),
    });
    world.bind_key_3(built_in(
        "profile",
        vec![("profile", Value::Text("Video calls".into()))],
    ));
    world.take_events();

    world.press_key_3();
    assert_eq!(world.engine.active_profile(), "video-calls");

    // A name that no profile has is a problem for the shell.
    world.command(Command::SelectProfile {
        profile: "default".into(),
    });
    world.bind_key_3(built_in(
        "profile",
        vec![("profile", Value::Text("Missing".into()))],
    ));
    world.take_events();
    world.press_key_3();
    assert_eq!(world.engine.active_profile(), "default");
    let problems: Vec<_> = world
        .take_events()
        .into_iter()
        .filter(|e| matches!(e, Event::Problem(text) if text.contains("Missing")))
        .collect();
    assert_eq!(problems.len(), 1);
}
