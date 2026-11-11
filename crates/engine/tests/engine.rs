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
use opencraft_engine::{Color, Command, Engine, Event, Profile, ProfileStore, SurfaceState};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const KEY_3: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 3);
const BAR_2: SurfaceId = SurfaceId::new(SurfaceKind::BarPanel, 2);
/// The slot of key face 3 on the N4 Pro.
const KEY_3_SLOT: u8 = 13;
const RED: Color = Color::rgb(200, 30, 30);
const BLUE: Color = Color::rgb(20, 40, 220);

// --- The fake device -----------------------------------------------------------

/// The fake time. A read with no report, and a sleep, move it forward.
struct World {
    now: Cell<Instant>,
}

/// A fake transport that the test and the engine share. The test looks at
/// the writes and puts input in the queue while the engine owns the device.
#[derive(Clone)]
struct Shared {
    fake: Rc<RefCell<FakeTransport>>,
    world: Rc<World>,
}

impl Transport for Shared {
    fn write(&mut self, report: &[u8]) -> opencraft_core::Result<()> {
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
/// queue means "the device is not present".
struct FakeConnector {
    queue: Rc<RefCell<VecDeque<Shared>>>,
}

impl Connector for FakeConnector {
    type Transport = Shared;

    fn connect(&mut self) -> opencraft_core::Result<Option<Device<Shared>>> {
        Ok(self
            .queue
            .borrow_mut()
            .pop_front()
            .map(|t| Device::new(t, &DESCRIPTORS[0])))
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
}

type Sink = Box<dyn FnMut(Event)>;
type TestEngine = Engine<FakeConnector, Sink, FakeClock>;

/// One engine with its fake device, its events, and its profile directory.
struct Rig {
    engine: TestEngine,
    events: Rc<RefCell<Vec<Event>>>,
    queue: Rc<RefCell<VecDeque<Shared>>>,
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
        let world = Rc::new(World {
            now: Cell::new(Instant::now()),
        });
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let events = Rc::new(RefCell::new(Vec::new()));
        let link = Link::with_clock(
            FakeConnector {
                queue: Rc::clone(&queue),
            },
            FakeClock {
                world: Rc::clone(&world),
            },
        );
        let sink_events = Rc::clone(&events);
        let sink: Sink = Box::new(move |event| sink_events.borrow_mut().push(event));
        let engine = Engine::new(link, store_in(dir.path()), &DESCRIPTORS[0], sink);
        Self {
            engine,
            events,
            queue,
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
        let fake = Rc::new(RefCell::new(FakeTransport::new()));
        self.queue.borrow_mut().push_back(Shared {
            fake: Rc::clone(&fake),
            world: Rc::clone(&self.world),
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

fn store_in(dir: &Path) -> ProfileStore {
    ProfileStore::new(dir.join("profile"))
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

/// The full draw after a connection: the start sequence, 14 images, and the
/// commit.
fn assert_full_draw(sent: &[Sent]) {
    assert_eq!(
        &sent[..4],
        [op("DIS"), op("LIG"), op("CLE"), op("STP")],
        "the start sequence"
    );
    let slots: Vec<u8> = sent[4..sent.len() - 1]
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
    // image of the size of the surface.
    let states = surfaces(&events);
    assert_eq!(states.len(), 14);
    assert_eq!(events.len(), 15, "no other event: {events:?}");
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

    // The shell gets the new state.
    let events = rig.take_events();
    assert_eq!(events.len(), 1);
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
    assert_eq!(
        rig.engine.profile().active_page().content(KEY_3).image,
        None
    );
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
