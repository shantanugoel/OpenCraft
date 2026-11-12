//! Tests for the engine logic, with no window and no device.
//!
//! The engine runs on a fake transport and a fake clock (architecture rule
//! 7). The tests use no thread and no real wait.

use image::{Rgb, RgbImage};
use opencraft_core::device::DESCRIPTORS;
use opencraft_core::{
    Clock, Connector, ControlId, ControlKind, Device, FakeTransport, Gesture, InputEvent, Link,
    SurfaceId, SurfaceKind, Transport,
};
use opencraft_engine::engine::SAVE_DELAY;
use opencraft_engine::runtime::{self, POLL_INTERVAL};
use opencraft_engine::store::{PROFILE_FILE, SCHEMA_VERSION};
use opencraft_engine::{
    Action, BindingId, BindingState, Color, Command, Counters, DeviceSettings, Engine, Event,
    Feedback, GestureKind, Handler, HandlerPort, KindDescription, Message, MessageSender, Profile,
    ProfileEntry, ProfileStore, ProfilesDir, Request, Settings, Status, SurfaceState, Value,
};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[path = "engine/actions.rs"]
mod actions;
#[path = "engine/device_settings.rs"]
mod device_settings;
#[path = "engine/history.rs"]
mod history;
#[path = "engine/labels.rs"]
mod labels;
#[path = "engine/pages.rs"]
mod pages;
#[path = "engine/profiles.rs"]
mod profiles;
#[path = "engine/test_model.rs"]
mod test_model;
/// The test model of the device layer tests: a second model that is not a
/// real device.
#[path = "../../core/tests/support/test_model.rs"]
mod test_model_data;

const KEY_3: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 3);
const BAR_2: SurfaceId = SurfaceId::new(SurfaceKind::BarPanel, 2);
/// The slot of key face 3 on the N4 Pro.
const KEY_3_SLOT: u8 = 13;
const RED: Color = Color::rgb(200, 30, 30);
const BLUE: Color = Color::rgb(20, 40, 220);

// --- The fake handlers ---------------------------------------------------------

fn description(kind: &str) -> KindDescription {
    KindDescription {
        kind: kind.into(),
        name: "Test action".into(),
        fields: Vec::new(),
        ..Default::default()
    }
}

fn port(kind: &str) -> (HandlerPort, mpsc::Receiver<Request>) {
    let (sender, receiver) = mpsc::channel();
    (
        HandlerPort {
            kinds: vec![description(kind)],
            sender,
        },
        receiver,
    )
}

fn action(kind: &str) -> Action {
    Action {
        kind: kind.into(),
        settings: Settings::from([
            ("text".into(), Value::Text("a setting".into())),
            ("number".into(), Value::Integer(42)),
        ]),
    }
}

fn binding(control: ControlKind, number: u8, gesture: GestureKind) -> BindingId {
    BindingId {
        page: 0,
        control: ControlId::new(control, number),
        gesture,
    }
}

fn set_binding(rig: &mut Rig, binding: BindingId, action: Option<Action>) {
    rig.engine.command(Command::SetBinding { binding, action });
}

// --- The fake device -----------------------------------------------------------

/// The fake time. A read with no report, and a sleep, move it forward.
struct World {
    now: Cell<Instant>,
    /// The fake sleep time of the computer. A test adds to it to simulate a
    /// sleep. `now` does not move then, as on macOS.
    asleep: Cell<Duration>,
    /// A test can set this function. The fake transport calls it before each
    /// write, so that the test can see the order of writes and requests.
    before_write: RefCell<Option<Box<dyn FnMut()>>>,
}

/// A fake transport that the test and the engine share. The test looks at
/// the writes and puts input in the queue while the engine owns the device.
#[derive(Clone)]
struct Shared {
    fake: Rc<RefCell<FakeTransport>>,
    world: Rc<World>,
    /// The model of the device.
    descriptor: &'static opencraft_core::Descriptor,
}

impl Transport for Shared {
    fn write(&mut self, report: &[u8]) -> opencraft_core::Result<()> {
        if let Some(before_write) = self.world.before_write.borrow_mut().as_mut() {
            before_write();
        }
        self.fake.borrow_mut().write(report)
    }

    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> opencraft_core::Result<Option<usize>> {
        let result = self.fake.borrow_mut().read(buf, timeout);
        if matches!(result, Ok(None)) {
            self.world.now.set(self.world.now.get() + timeout);
        }
        result
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> opencraft_core::Result<usize> {
        self.fake.borrow_mut().get_feature_report(buf)
    }
}

/// A connector that gives the transports of a queue, in order. An empty
/// queue means "the device is not present". While `refusals` is not zero,
/// each try fails as when a different application holds the device, and
/// counts down.
struct FakeConnector {
    queue: Rc<RefCell<VecDeque<Shared>>>,
    refusals: Rc<Cell<u32>>,
}

impl Connector for FakeConnector {
    type Transport = Shared;

    fn connect(&mut self) -> opencraft_core::Result<Option<Device<Shared>>> {
        if self.refusals.get() > 0 {
            self.refusals.set(self.refusals.get() - 1);
            return Err(opencraft_core::Error::PermissionDenied);
        }
        Ok(self.queue.borrow_mut().pop_front().map(|t| {
            let descriptor = t.descriptor;
            Device::new(t, descriptor)
        }))
    }
}

struct FakeClock {
    world: Rc<World>,
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.world.now.get()
    }

    fn sleep(&mut self, duration: Duration) {
        self.world.now.set(self.world.now.get() + duration);
    }

    fn asleep(&self) -> Option<Duration> {
        Some(self.world.asleep.get())
    }
}

type Sink = Box<dyn FnMut(Event)>;
type TestEngine = Engine<FakeConnector, Sink, FakeClock>;

/// One engine with its fake device, its events, and its profile directory.
struct Rig {
    engine: TestEngine,
    events: Rc<RefCell<Vec<Event>>>,
    queue: Rc<RefCell<VecDeque<Shared>>>,
    refusals: Rc<Cell<u32>>,
    world: Rc<World>,
    dir: TempDir,
}

impl Rig {
    /// An engine on an empty profile directory, with no device present.
    fn new() -> Self {
        Self::in_dir(TempDir::new().unwrap())
    }

    /// An engine on the profile directory `dir`, with no device present.
    fn in_dir(dir: TempDir) -> Self {
        Self::with_ports(dir, Vec::new())
    }

    fn with_ports(dir: TempDir, ports: Vec<opencraft_engine::HandlerPort>) -> Self {
        Self::with_model(dir, ports, &DESCRIPTORS[0])
    }

    /// An engine with the layout of `model` until a device connects.
    fn with_model(
        dir: TempDir,
        ports: Vec<opencraft_engine::HandlerPort>,
        model: &'static opencraft_core::Descriptor,
    ) -> Self {
        let world = Rc::new(World {
            now: Cell::new(Instant::now()),
            asleep: Cell::new(Duration::ZERO),
            before_write: RefCell::new(None),
        });
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let events = Rc::new(RefCell::new(Vec::new()));
        let refusals = Rc::new(Cell::new(0));
        let link = Link::with_clock(
            FakeConnector {
                queue: Rc::clone(&queue),
                refusals: Rc::clone(&refusals),
            },
            FakeClock {
                world: Rc::clone(&world),
            },
        );
        let sink_events = Rc::clone(&events);
        let sink: Sink = Box::new(move |event| sink_events.borrow_mut().push(event));
        let engine = Engine::with_handlers(link, ProfilesDir::new(dir.path()), model, ports, sink);
        Self {
            engine,
            events,
            queue,
            refusals,
            world,
            dir,
        }
    }

    /// Stop the engine with no write, and start a new engine on the same
    /// profile directory.
    fn restart(self) -> Self {
        let Rig { engine, dir, .. } = self;
        drop(engine);
        Self::in_dir(dir)
    }

    /// Make a device present. The engine connects to it at the next poll.
    fn plug(&mut self) -> Rc<RefCell<FakeTransport>> {
        self.plug_model(&DESCRIPTORS[0])
    }

    /// Make a device of the model `descriptor` present.
    fn plug_model(
        &mut self,
        descriptor: &'static opencraft_core::Descriptor,
    ) -> Rc<RefCell<FakeTransport>> {
        let fake = Rc::new(RefCell::new(FakeTransport::new()));
        self.queue.borrow_mut().push_back(Shared {
            fake: Rc::clone(&fake),
            world: Rc::clone(&self.world),
            descriptor,
        });
        fake
    }

    /// Make a device present, and let the engine connect and draw.
    fn connect(&mut self) -> Rc<RefCell<FakeTransport>> {
        let fake = self.plug();
        self.poll_for(Duration::from_secs(2));
        assert!(self.engine.link().is_connected());
        fake
    }

    /// Poll until `duration` of the fake time is gone.
    fn poll_for(&mut self, duration: Duration) {
        let end = self.world.now.get() + duration;
        while self.world.now.get() < end {
            self.engine.poll(POLL_INTERVAL);
        }
    }

    /// Simulate a sleep of the computer for `duration`.
    fn sleep_computer(&mut self, duration: Duration) {
        self.world.asleep.set(self.world.asleep.get() + duration);
    }

    fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut *self.events.borrow_mut())
    }

    fn profile_text(&self) -> Option<String> {
        fs::read_to_string(store_in(self.dir.path()).profile_path()).ok()
    }

    fn set_color(&mut self, surface: SurfaceId, color: Color) {
        assert!(self.engine.command(Command::SetColor {
            surface,
            color: Some(color),
        }));
    }
}

/// The store of the profile `default` in the profiles directory `dir`: the
/// profile of the earlier phases.
fn store_in(dir: &Path) -> ProfileStore {
    ProfilesDir::new(dir).store("default")
}

/// The surface states in `events`, in order.
fn surfaces(events: &[Event]) -> Vec<&SurfaceState> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Surface(state) => Some(state),
            _ => None,
        })
        .collect()
}

fn problems(events: &[Event]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Problem(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// The last state of `id` in `events`.
fn last_state(events: &[Event], id: SurfaceId) -> &SurfaceState {
    surfaces(events)
        .into_iter()
        .rev()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("no state of {id}"))
}

// --- What the device got -------------------------------------------------------

/// One thing that the engine sent to the device.
#[derive(Debug, Clone, PartialEq)]
enum Sent {
    /// A command with no image, by its operation code, for example `DIS`.
    Op(String),
    /// An image for a slot, with the image data.
    Image { slot: u8, data: Vec<u8> },
}

/// Read the writes of the fake transport as commands and images.
fn sent(fake: &Rc<RefCell<FakeTransport>>) -> Vec<Sent> {
    let writes = fake.borrow_mut().take_writes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < writes.len() {
        let report = &writes[i];
        assert_eq!(report.len(), 1025, "each write is one full report");
        assert_eq!(&report[..6], b"\0CRT\0\0", "a command starts with CRT");
        let code: String = report[6..]
            .iter()
            .take_while(|b| b.is_ascii_uppercase())
            .map(|&b| b as char)
            .collect();
        i += 1;
        if code == "BAT" {
            let len = u32::from_be_bytes(report[9..13].try_into().unwrap()) as usize;
            let slot = report[13];
            let mut data = Vec::new();
            while data.len() < len {
                data.extend_from_slice(&writes[i][1..]);
                i += 1;
            }
            data.truncate(len);
            out.push(Sent::Image { slot, data });
        } else {
            out.push(Sent::Op(code));
        }
    }
    out
}

fn op(code: &str) -> Sent {
    Sent::Op(code.to_string())
}

/// The upright image that the device shows for the image data `data`.
fn shown(data: &[u8]) -> RgbImage {
    let image = image::load_from_memory(data).unwrap().into_rgb8();
    image::imageops::rotate180(&image)
}

/// The image of `slot` in `sent`, upright.
fn shown_at(sent: &[Sent], slot: u8) -> RgbImage {
    let data = sent
        .iter()
        .rev()
        .find_map(|s| match s {
            Sent::Image { slot: s, data } if *s == slot => Some(data),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no image for slot {slot}"));
    shown(data)
}

/// Check that `a` and `b` are the same picture. The JPEG of the device
/// changes each value a little.
fn assert_same_picture(a: &RgbImage, b: &RgbImage) {
    assert_eq!(a.dimensions(), b.dimensions());
    let total: u64 = a
        .pixels()
        .zip(b.pixels())
        .flat_map(|(p, q)| (0..3).map(move |i| u64::from(p[i].abs_diff(q[i]))))
        .sum();
    let mean = total as f64 / (a.width() * a.height() * 3) as f64;
    assert!(
        mean < 4.0,
        "the pictures are different: mean difference {mean}"
    );
}

fn assert_color(image: &RgbImage, x: u32, y: u32, want: Color) {
    let got = image.get_pixel(x, y);
    let near = (0..3).all(|i| got[i].abs_diff(want.0[i]) <= 12);
    assert!(near, "pixel {x}, {y} is {got:?}, not {want}");
}

/// The full draw after a connection: the start sequence, the device
/// settings, 14 images, and the commit.
fn assert_full_draw(sent: &[Sent]) {
    assert_eq!(
        &sent[..4],
        [op("DIS"), op("LIG"), op("CLE"), op("STP")],
        "the start sequence"
    );
    assert_eq!(
        &sent[4..7],
        [op("SETLB"), op("LBLIG"), op("QUCMD")],
        "the device settings after the start sequence"
    );
    let slots: Vec<u8> = sent[7..sent.len() - 1]
        .iter()
        .map(|s| match s {
            Sent::Image { slot, .. } => *slot,
            other => panic!("not an image: {other:?}"),
        })
        .collect();
    let mut want: Vec<u8> = DESCRIPTORS[0]
        .slot_surfaces()
        .filter_map(|s| s.slot)
        .collect();
    assert_eq!(slots.len(), 14);
    let mut sorted = slots.clone();
    sorted.sort();
    want.sort();
    assert_eq!(sorted, want, "each slot one time");
    assert_eq!(sent.last(), Some(&op("STP")), "the commit is last");
}

// --- The start -----------------------------------------------------------------

#[test]
fn the_engine_starts_with_no_device_and_sends_the_layout_and_each_surface() {
    let mut rig = Rig::new();
    let events = rig.take_events();

    let Event::Layout(layout) = &events[0] else {
        panic!("the first event is {:?}", events[0]);
    };
    assert_eq!(layout.name, DESCRIPTORS[0].name);
    assert_eq!((layout.display_width, layout.display_height), (800, 480));

    // Each key face and each bar panel has a state: no content, and a black
    // image of the size of the surface. The list of profiles, the list of
    // pages, the empty history, and the device settings come last.
    let states = surfaces(&events);
    assert_eq!(states.len(), 14);
    assert_eq!(events.len(), 20);
    assert_eq!(events[1], Event::Kinds(Vec::new()));
    assert_eq!(
        events[16],
        Event::Profiles {
            profiles: vec![ProfileEntry {
                id: "default".into(),
                name: "Default".into()
            }],
            active: "default".into()
        }
    );
    assert_eq!(
        events[17],
        Event::Pages {
            names: vec!["Page 1".into()],
            active: 0
        }
    );
    assert_eq!(
        events[18],
        Event::History {
            undo: None,
            redo: None
        }
    );
    assert_eq!(
        events[19],
        Event::DeviceSettings(DeviceSettings::defaults(DESCRIPTORS[0].settings))
    );
    for state in states {
        let surface = DESCRIPTORS[0].surface(state.id).unwrap();
        assert!(state.content.is_empty());
        assert_eq!(
            state.image.dimensions(),
            (u32::from(surface.width), u32::from(surface.height))
        );
        assert!(state.image.pixels().all(|p| *p == Rgb([0, 0, 0])));
    }

    // With no device, the engine waits and writes no file.
    rig.poll_for(Duration::from_secs(3));
    assert!(rig.take_events().is_empty());
    assert_eq!(rig.profile_text(), None);
}

#[test]
fn a_connection_gives_the_start_sequence_and_each_image() {
    let mut rig = Rig::new();
    rig.take_events();
    let device = rig.connect();

    assert_eq!(rig.take_events(), [Event::Connected]);
    assert_full_draw(&sent(&device));

    // After the draw, only the keep-alive goes to the device.
    rig.poll_for(Duration::from_secs(25));
    assert_eq!(sent(&device), [op("CONNECT"), op("CONNECT")]);
    assert!(rig.take_events().is_empty());
}

// --- Changes -------------------------------------------------------------------

#[test]
fn a_change_shows_on_the_device_and_in_the_event_as_the_same_image() {
    let mut rig = Rig::new();
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    rig.set_color(KEY_3, RED);

    // The shell gets the new state, and the name of the new step.
    let events = rig.take_events();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[1],
        Event::History {
            undo: Some("Set Fill".into()),
            redo: None
        }
    );
    let state = last_state(&events, KEY_3);
    assert_eq!(state.content.color, Some(RED));
    assert_color(&state.image, 56, 56, RED);

    // The device gets that one image and a commit, and nothing else.
    let sent = sent(&device);
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert!(matches!(
        sent[0],
        Sent::Image {
            slot: KEY_3_SLOT,
            ..
        }
    ));
    assert_eq!(sent[1], op("STP"));
    assert_same_picture(&shown_at(&sent, KEY_3_SLOT), &state.image);
}

#[test]
fn a_command_that_changes_nothing_does_nothing() {
    let mut rig = Rig::new();
    let device = rig.connect();
    rig.set_color(KEY_3, RED);
    rig.poll_for(Duration::from_secs(1));
    sent(&device);
    rig.take_events();
    let text = rig.profile_text();

    rig.set_color(KEY_3, RED);
    rig.engine.command(Command::SetLabel {
        surface: KEY_3,
        label: Some(String::new()),
    });
    rig.engine.command(Command::SetImage {
        surface: KEY_3,
        path: None,
    });
    rig.poll_for(Duration::from_secs(1));

    assert!(rig.take_events().is_empty());
    assert!(sent(&device).is_empty());
    assert_eq!(rig.profile_text(), text);
}

#[test]
fn the_profile_is_written_after_the_delay_and_one_time_for_many_changes() {
    let mut rig = Rig::new();
    for i in 0..10u8 {
        rig.set_color(KEY_3, Color::rgb(i, 0, 0));
        rig.poll_for(SAVE_DELAY / 5);
        assert_eq!(
            rig.profile_text(),
            None,
            "change {i}: no write during the changes"
        );
    }
    rig.poll_for(SAVE_DELAY);
    let text = rig.profile_text().expect("the file is written");
    assert!(text.contains(&format!("schema_version = {SCHEMA_VERSION}")));
    assert!(text.contains("[pages.surfaces.key-face-3]"));
    assert!(text.contains("color = \"#090000\""));
}

#[test]
fn the_saved_profile_shows_after_the_engine_starts_again() {
    let mut rig = Rig::new();
    rig.set_color(KEY_3, RED);
    rig.set_color(BAR_2, BLUE);
    rig.engine.command(Command::SetLabel {
        surface: KEY_3,
        label: Some("Mail".to_string()),
    });
    rig.poll_for(Duration::from_secs(1));

    let mut rig = rig.restart();
    let events = rig.take_events();
    assert!(problems(&events).is_empty());
    let key = last_state(&events, KEY_3);
    assert_eq!(key.content.color, Some(RED));
    assert_eq!(key.content.label.as_deref(), Some("Mail"));
    assert_color(&key.image, 2, 2, RED);
    assert_color(&last_state(&events, BAR_2).image, 2, 2, BLUE);

    // The device shows the saved profile after the connection.
    let device = rig.connect();
    let sent = sent(&device);
    assert_full_draw(&sent);
    assert_same_picture(&shown_at(&sent, KEY_3_SLOT), &key.image);
    assert_color(&shown_at(&sent, 2), 2, 2, BLUE);
}

#[test]
fn a_change_with_no_device_shows_at_the_connection() {
    let mut rig = Rig::new();
    rig.set_color(KEY_3, RED);
    let events = rig.take_events();
    assert_color(&last_state(&events, KEY_3).image, 56, 56, RED);

    let device = rig.connect();
    let sent = sent(&device);
    assert_full_draw(&sent);
    assert_color(&shown_at(&sent, KEY_3_SLOT), 2, 2, RED);
}

// --- Disconnection ---------------------------------------------------------------

#[test]
fn after_a_disconnection_and_a_new_connection_the_device_shows_the_profile_again() {
    let mut rig = Rig::new();
    let first = rig.connect();
    rig.set_color(KEY_3, RED);
    sent(&first);
    rig.take_events();

    first.borrow_mut().disconnect();
    rig.poll_for(Duration::from_secs(1));
    assert_eq!(rig.take_events(), [Event::Disconnected]);

    // A change while the device is gone.
    rig.set_color(BAR_2, BLUE);
    rig.poll_for(Duration::from_secs(3));
    rig.take_events();

    let second = rig.connect();
    assert_eq!(rig.take_events(), [Event::Connected]);
    let sent = sent(&second);
    assert_full_draw(&sent);
    assert_color(&shown_at(&sent, KEY_3_SLOT), 2, 2, RED);
    assert_color(&shown_at(&sent, 2), 2, 2, BLUE);
}

#[test]
fn a_draw_that_fails_is_done_again_in_full() {
    let mut rig = Rig::new();
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    device.borrow_mut().fail_writes("the write failed");
    rig.set_color(KEY_3, RED);
    rig.engine.poll(POLL_INTERVAL);
    device.borrow_mut().clear_errors();
    rig.poll_for(Duration::from_secs(1));

    // The shell gets the problem one time.
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 1, "{events:?}");
    assert!(!events.contains(&Event::Disconnected));

    let sent = sent(&device);
    assert_full_draw(&sent);
    assert_color(&shown_at(&sent, KEY_3_SLOT), 2, 2, RED);
}

// --- Input ---------------------------------------------------------------------

/// A control report with the input code `code` and the state `state`.
fn control_report(code: u8, state: u8) -> Vec<u8> {
    let mut report = vec![0u8; 512];
    report[..3].copy_from_slice(b"ACK");
    report[5..7].copy_from_slice(b"OK");
    report[9] = code;
    report[10] = state;
    report
}

#[test]
fn input_from_the_device_goes_to_the_shell() {
    let mut rig = Rig::new();
    let device = rig.connect();
    rig.take_events();

    // Key 3 down and up, knob 2 to the right, an acknowledgement, and a
    // report that is not known.
    for report in [
        control_report(0x03, 1),
        control_report(0x03, 0),
        control_report(0x51, 0),
        control_report(0xFF, 0),
        vec![1, 2, 3],
    ] {
        device.borrow_mut().push_input(report);
    }
    rig.poll_for(Duration::from_millis(200));

    let key = ControlId::new(ControlKind::Key, 3);
    let knob = ControlId::new(ControlKind::Knob, 2);
    let input = |control, gesture| Event::Input(InputEvent { control, gesture });
    assert_eq!(
        rig.take_events(),
        [
            input(key, Gesture::Press),
            input(key, Gesture::Release),
            input(knob, Gesture::TurnRight),
        ]
    );
    assert_eq!(
        rig.engine.counters(),
        Counters {
            reports_received: 5,
            input_events: 3,
            invokes_sent: 0
        }
    );
}

// --- Images --------------------------------------------------------------------

/// Write a green image file of 64 × 64 and return its path.
fn green_file(dir: &Path) -> PathBuf {
    let path = dir.join("green.png");
    RgbImage::from_pixel(64, 64, Rgb([0, 200, 0]))
        .save(&path)
        .unwrap();
    path
}

const GREEN: Color = Color::rgb(0, 200, 0);

#[test]
fn an_image_of_the_user_shows_and_stays_when_the_original_file_is_gone() {
    let mut rig = Rig::new();
    let device = rig.connect();
    sent(&device);
    rig.take_events();
    let file = green_file(rig.dir.path());

    rig.engine.command(Command::SetImage {
        surface: KEY_3,
        path: Some(file.clone()),
    });
    let events = rig.take_events();
    let state = last_state(&events, KEY_3);
    assert_color(&state.image, 56, 56, GREEN);
    assert_color(&shown_at(&sent(&device), KEY_3_SLOT), 56, 56, GREEN);
    let copy = state
        .content
        .image
        .clone()
        .expect("the content has the image");
    assert!(copy.starts_with("images/"), "{copy}");

    // The profile uses its own copy.
    fs::remove_file(&file).unwrap();
    rig.poll_for(Duration::from_secs(1));
    let mut rig = rig.restart();
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    assert_color(&last_state(&events, KEY_3).image, 56, 56, GREEN);

    // Without the image, the surface is black again.
    rig.engine.command(Command::SetImage {
        surface: KEY_3,
        path: None,
    });
    let events = rig.take_events();
    let state = last_state(&events, KEY_3);
    assert_eq!(state.content.image, None);
    assert_color(&state.image, 56, 56, Color::BLACK);

    // The next start removes the copy that no surface uses.
    rig.poll_for(Duration::from_secs(1));
    let images = store_in(rig.dir.path()).dir().join("images");
    assert_eq!(fs::read_dir(&images).unwrap().count(), 1);
    let rig = rig.restart();
    assert_eq!(fs::read_dir(&images).unwrap().count(), 0);
    drop(rig);
}

#[test]
fn a_file_that_is_not_an_image_gives_a_problem_and_no_change() {
    let mut rig = Rig::new();
    let device = rig.connect();
    rig.set_color(KEY_3, RED);
    sent(&device);
    rig.take_events();
    let file = rig.dir.path().join("notes.png");
    fs::write(&file, "this is text").unwrap();

    rig.engine.command(Command::SetImage {
        surface: KEY_3,
        path: Some(file),
    });

    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 1);
    assert_eq!(events.len(), 1, "no new state: {events:?}");
    assert!(sent(&device).is_empty());
    assert_eq!(rig.engine.profile().pages()[0].content(KEY_3).image, None);
}

#[test]
fn a_command_for_a_surface_that_the_model_does_not_have_gives_a_problem() {
    let mut rig = Rig::new();
    rig.take_events();
    for surface in [
        SurfaceId::new(SurfaceKind::KeyFace, 11),
        // Phase 2 draws no background.
        SurfaceId::new(SurfaceKind::Background, 1),
    ] {
        rig.set_color(surface, RED);
        let events = rig.take_events();
        assert_eq!(problems(&events).len(), 1, "{surface}");
        assert_eq!(events.len(), 1, "{surface}");
    }
    assert_eq!(rig.engine.profile(), &Profile::default());
}

// --- The profile file ----------------------------------------------------------

#[test]
fn a_profile_file_that_the_engine_cannot_use_stays_as_a_backup() {
    let dir = TempDir::new().unwrap();
    let store = store_in(dir.path());
    fs::create_dir_all(store.dir()).unwrap();
    let text = "schema_version = 99\nname = \"From a newer version\"\n";
    fs::write(store.profile_path(), text).unwrap();

    let mut rig = Rig::in_dir(dir);
    let events = rig.take_events();
    let problems = problems(&events);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("99"), "{}", problems[0]);
    assert_eq!(surfaces(&events).len(), 14);

    // A change makes a new profile file. The backup stays.
    rig.set_color(KEY_3, RED);
    rig.poll_for(Duration::from_secs(1));
    let mut names: Vec<String> = fs::read_dir(store.dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["profile.backup-1.toml", PROFILE_FILE]);
    assert_eq!(
        fs::read_to_string(store.dir().join("profile.backup-1.toml")).unwrap(),
        text
    );
    assert!(rig.profile_text().unwrap().contains("#C81E1E"));
}

#[test]
fn a_profile_that_cannot_be_read_is_not_replaced() {
    let dir = TempDir::new().unwrap();
    let store = store_in(dir.path());
    // A directory in the place of the file: the read fails, and it is not
    // "no file".
    fs::create_dir_all(store.profile_path()).unwrap();

    let mut rig = Rig::in_dir(dir);
    assert_eq!(problems(&rig.take_events()).len(), 1);

    rig.set_color(KEY_3, RED);
    rig.poll_for(Duration::from_secs(1));
    rig.engine.finish();
    assert!(problems(&rig.take_events()).is_empty());
    assert!(store.profile_path().is_dir(), "the engine wrote nothing");
}

#[test]
fn a_failed_write_of_the_profile_gives_one_problem_and_a_new_try() {
    let mut rig = Rig::new();
    let store = store_in(rig.dir.path());
    // A file in the place of the profile directory: the write fails.
    fs::write(store.dir(), "in the way").unwrap();

    rig.set_color(KEY_3, RED);
    rig.poll_for(Duration::from_secs(3));
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 1, "{events:?}");

    fs::remove_file(store.dir()).unwrap();
    rig.poll_for(Duration::from_secs(1));
    assert!(rig.profile_text().unwrap().contains("#C81E1E"));
    assert!(rig.take_events().is_empty());
}

// --- The loop of the engine thread ---------------------------------------------

#[test]
fn the_loop_obeys_the_commands_and_writes_the_profile_at_the_stop() {
    let mut rig = Rig::new();
    let device = rig.plug();
    let (commands, receiver) = mpsc::channel();
    commands
        .send(Command::SetColor {
            surface: KEY_3,
            color: Some(RED),
        })
        .unwrap();
    commands.send(Command::Stop).unwrap();
    // A command after the stop does nothing.
    commands
        .send(Command::SetColor {
            surface: KEY_3,
            color: Some(BLUE),
        })
        .unwrap();

    runtime::run(&mut rig.engine, &receiver);

    // The stop does not wait for the delay of the write.
    assert!(rig.profile_text().unwrap().contains("#C81E1E"));
    assert!(
        sent(&device).is_empty(),
        "the stop came before the first poll"
    );
}

#[test]
fn the_loop_stops_when_the_shell_is_gone() {
    let mut rig = Rig::new();
    let (commands, receiver) = mpsc::channel();
    commands
        .send(Command::SetColor {
            surface: KEY_3,
            color: Some(RED),
        })
        .unwrap();
    drop(commands);

    runtime::run(&mut rig.engine, &receiver);
    assert!(rig.profile_text().unwrap().contains("#C81E1E"));
}

// --- The stop --------------------------------------------------------------------

/// The full report of the command `payload`, as the device gets it.
fn full_report(payload: Vec<u8>) -> Vec<u8> {
    let mut report = payload;
    report.resize(1025, 0);
    report
}

/// Each write of the fake transport, as full reports. The record is taken.
fn raw_writes(fake: &Rc<RefCell<FakeTransport>>) -> Vec<Vec<u8>> {
    fake.borrow_mut().take_writes()
}

fn stop_sequence() -> Vec<Vec<u8>> {
    vec![
        full_report(opencraft_core::protocol::disconnected()),
        full_report(opencraft_core::protocol::sleep()),
    ]
}

#[test]
fn the_stop_writes_the_profile_then_sends_dc_and_han_and_then_nothing() {
    let mut rig = Rig::new();
    let device = rig.connect();
    raw_writes(&device);
    rig.take_events();

    // The profile file must be written before the first write of the stop.
    let profile_at_write = Rc::new(RefCell::new(Vec::new()));
    let path = store_in(rig.dir.path()).profile_path();
    let seen = Rc::clone(&profile_at_write);
    *rig.world.before_write.borrow_mut() = Some(Box::new(move || {
        seen.borrow_mut().push(fs::read_to_string(&path).ok());
    }));

    // The change comes just before the stop: the delay of the write is not
    // over.
    let (commands, receiver) = mpsc::channel();
    commands
        .send(Command::SetColor {
            surface: KEY_3,
            color: Some(RED),
        })
        .unwrap();
    commands.send(Command::Stop).unwrap();
    runtime::run(&mut rig.engine, &receiver);

    let writes = raw_writes(&device);
    // The image of the change and its commit, then the stop sequence.
    assert_eq!(&writes[writes.len() - 2..], stop_sequence().as_slice());
    let seen = profile_at_write.borrow();
    let at_dc = seen[seen.len() - 2]
        .as_deref()
        .expect("the file exists at DC");
    assert!(at_dc.contains("#C81E1E"), "the change is in the file at DC");
    assert!(!rig.engine.link().is_connected());

    // After the stop, nothing goes to the device: no keep-alive, and no new
    // connection to a device that is present.
    *rig.world.before_write.borrow_mut() = None;
    let other = rig.plug();
    rig.poll_for(Duration::from_secs(60));
    assert!(
        raw_writes(&device).is_empty(),
        "no keep-alive after the stop"
    );
    assert!(raw_writes(&other).is_empty(), "no new connection");
    assert!(!rig.engine.link().is_connected());
}

#[test]
fn the_stop_comes_also_when_each_sender_of_commands_is_gone() {
    let mut rig = Rig::new();
    let device = rig.connect();
    raw_writes(&device);
    let (commands, receiver) = mpsc::channel::<Command>();
    drop(commands);

    runtime::run(&mut rig.engine, &receiver);
    assert_eq!(raw_writes(&device), stop_sequence());
}

#[test]
fn the_stop_with_no_device_sends_nothing_and_writes_the_profile() {
    let mut rig = Rig::new();
    let device = rig.plug();
    rig.set_color(KEY_3, RED);
    rig.engine.finish();
    assert!(rig.profile_text().unwrap().contains("#C81E1E"));
    assert!(raw_writes(&device).is_empty());

    // A device that went away, and the engine did not see it yet: one try,
    // no wait.
    let mut rig = Rig::new();
    let device = rig.connect();
    raw_writes(&device);
    device.borrow_mut().disconnect();
    let before = rig.world.now.get();
    rig.engine.finish();
    assert_eq!(rig.world.now.get(), before, "the stop does not wait");
    assert!(!rig.engine.link().is_connected());
}

#[test]
fn the_stop_does_not_hang_or_retry_when_the_writes_fail() {
    let mut rig = Rig::new();
    let device = rig.connect();
    raw_writes(&device);
    rig.set_color(KEY_3, RED);
    raw_writes(&device);
    device.borrow_mut().fail_writes("the write failed");

    let before = rig.world.now.get();
    rig.engine.finish();
    assert_eq!(rig.world.now.get(), before, "the stop does not wait");
    assert!(raw_writes(&device).is_empty());
    assert!(rig.profile_text().unwrap().contains("#C81E1E"));
    assert!(!rig.engine.link().is_connected());

    device.borrow_mut().clear_errors();
    rig.poll_for(Duration::from_secs(30));
    assert!(raw_writes(&device).is_empty(), "no retry, no keep-alive");
}

// --- Sleep and wake of the computer -----------------------------------------------

#[test]
fn a_wake_gives_the_start_sequence_and_a_full_draw() {
    let mut rig = Rig::new();
    let device = rig.connect();
    rig.set_color(KEY_3, RED);
    rig.poll_for(Duration::from_secs(1));
    sent(&device);
    rig.take_events();

    rig.sleep_computer(Duration::from_secs(70));
    rig.engine.poll(POLL_INTERVAL);

    // The shell drops its input marks, and gets Connected after the draw.
    assert_eq!(rig.take_events(), [Event::Disconnected, Event::Connected]);
    let sent_after = sent(&device);
    assert_eq!(sent_after[0], op("CONNECT"), "the keep-alive first");
    assert_full_draw(&sent_after[1..]);
    assert_color(&shown_at(&sent_after, KEY_3_SLOT), 2, 2, RED);

    // Then only the keep-alive, each 10 seconds from the wake.
    rig.poll_for(Duration::from_secs(25));
    assert_eq!(sent(&device), [op("CONNECT"), op("CONNECT")]);
    assert!(rig.take_events().is_empty());
}

#[test]
fn a_normal_period_gives_nothing_extra() {
    let mut rig = Rig::new();
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    // A sleep below the limit only moves the keep-alive.
    rig.sleep_computer(Duration::from_secs(1));
    rig.poll_for(Duration::from_secs(120));
    assert!(rig.take_events().is_empty());
    assert!(sent(&device).iter().all(|s| *s == op("CONNECT")));
}

#[test]
fn a_wake_with_failing_writes_gives_a_new_connection_and_a_full_draw() {
    let mut rig = Rig::new();
    let first = rig.connect();
    sent(&first);
    rig.take_events();
    // The handle stays, but each write fails after the wake. The same
    // device opens again with a new handle.
    first
        .borrow_mut()
        .fail_writes("IOHIDDeviceSetReport failed");
    let second = rig.plug();

    rig.sleep_computer(Duration::from_secs(300));
    rig.poll_for(Duration::from_secs(3));

    let events = rig.take_events();
    assert_eq!(events.first(), Some(&Event::Disconnected));
    assert_eq!(events.last(), Some(&Event::Connected), "{events:?}");
    assert!(rig.engine.link().is_connected());
    assert!(sent(&first).is_empty());
    assert_full_draw(&sent(&second));
}

#[test]
fn a_device_restart_some_seconds_after_the_wake_also_ends_in_a_full_draw() {
    let mut rig = Rig::new();
    let first = rig.connect();
    sent(&first);
    rig.take_events();

    rig.sleep_computer(Duration::from_secs(70));
    rig.poll_for(Duration::from_secs(1));
    assert_full_draw(&sent(&first)[1..]);
    rig.take_events();

    // The keep-alive timer of the device ran out during the sleep: the
    // device stops the USB connection 4 seconds after the wake.
    rig.poll_for(Duration::from_secs(3));
    first.borrow_mut().disconnect();
    let second = rig.plug();
    rig.poll_for(Duration::from_secs(2));

    assert_eq!(rig.take_events(), [Event::Disconnected, Event::Connected]);
    assert_full_draw(&sent(&second));
}

#[test]
fn a_wake_with_no_device_sends_nothing_and_keeps_the_pending_write() {
    let mut rig = Rig::new();
    rig.take_events();
    // A change, and the computer sleeps before the delay of the write.
    rig.set_color(KEY_3, RED);
    rig.take_events();
    rig.sleep_computer(Duration::from_secs(3600));
    rig.engine.poll(POLL_INTERVAL);
    assert!(rig.take_events().is_empty());
    assert_eq!(rig.profile_text(), None, "the delay is in time awake");

    rig.poll_for(SAVE_DELAY);
    assert!(rig.profile_text().unwrap().contains("#C81E1E"));
}

#[test]
fn a_device_that_a_different_application_holds_gives_one_problem_for_the_user() {
    let mut rig = Rig::new();
    rig.take_events();
    // VSD Craft holds the device for some tries, and then quits.
    rig.refusals.set(3);
    rig.plug();
    rig.poll_for(Duration::from_secs(5));
    assert!(rig.engine.link().is_connected());
    let events = rig.take_events();
    let problems = problems(&events);
    assert_eq!(problems.len(), 1, "{events:?}");
    assert!(problems[0].contains("Quit VSD Craft"), "{}", problems[0]);
    // The text of the device layer is for the log and the probe only.
    assert!(!problems[0].contains("kIOReturn"), "{}", problems[0]);
    assert!(events.contains(&Event::Connected));
}
