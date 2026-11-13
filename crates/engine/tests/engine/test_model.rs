//! The engine operates a second model that is data only.
//!
//! The test model is in `crates/core/tests/support/test_model.rs`: 6 keys in
//! 2 rows of 3 with key faces of 96 × 96 PNG that are not turned, 1 knob with
//! 1 knob light, no haptic feedback, no bar panel, no touch bar, and no
//! background. It is not a real device. The tests show that the engine makes
//! a profile for it, draws all its surfaces, and dispatches its bindings with
//! no code for a specific model (architecture rule 4). See
//! `docs/ADDING-A-DEVICE-MODEL.md`.

use super::test_model_data::{key_code, slot_of, FACE_SIZE, KNOB_CODES, TEST_MODEL};
use super::*;
use opencraft_core::protocol;
use opencraft_engine::SettingsSupport;

const KIND: &str = "test.test-model";

fn face(number: u8) -> SurfaceId {
    SurfaceId::new(SurfaceKind::KeyFace, number)
}

/// The image that the device shows for the PNG `data`. The test model does
/// not turn its images.
fn upright(data: &[u8]) -> RgbImage {
    image::load_from_memory(data).unwrap().into_rgb8()
}

/// The text of the profile file of the profile `id`.
fn profile_file(rig: &Rig, id: &str) -> Option<String> {
    fs::read_to_string(ProfilesDir::new(rig.dir.path()).store(id).profile_path()).ok()
}

/// An engine that has the test model as its first model, with one handler.
fn test_model_rig() -> (Rig, mpsc::Receiver<Request>) {
    let (port, requests) = port(KIND);
    let rig = Rig::with_model(TempDir::new().unwrap(), vec![port], &TEST_MODEL);
    (rig, requests)
}

/// Make a device of the test model present, and let the engine connect and
/// draw.
fn connect_test_model(rig: &mut Rig) -> Rc<RefCell<FakeTransport>> {
    let device = rig.plug_model(&TEST_MODEL);
    rig.poll_for(Duration::from_secs(2));
    assert!(rig.engine.link().is_connected());
    device
}

#[test]
fn the_engine_shows_the_layout_and_the_surfaces_of_the_test_model() {
    let (mut rig, _requests) = test_model_rig();
    let events = rig.take_events();

    let Event::Layout(layout) = &events[0] else {
        panic!("the first event is {:?}", events[0]);
    };
    assert_eq!(layout.name, TEST_MODEL.name);
    assert_eq!((layout.display_width, layout.display_height), (300, 200));
    assert_eq!(layout.surfaces.len(), 6);
    assert_eq!(layout.controls.len(), 7);
    assert_eq!(layout.settings, TEST_MODEL.settings);
    let knob = layout
        .controls
        .iter()
        .find(|c| c.id.kind == ControlKind::Knob)
        .unwrap();
    assert_eq!((knob.surface, knob.feedback_surface), (None, None));
    assert!(layout
        .controls
        .iter()
        .all(|c| matches!(c.id.kind, ControlKind::Key | ControlKind::Knob)));

    // One state for each key face, black, at the size of the key face.
    let states = surfaces(&events);
    assert_eq!(states.len(), 6);
    for (state, number) in states.iter().zip(1..) {
        assert_eq!(state.id, face(number));
        assert_eq!(
            state.image.dimensions(),
            (u32::from(FACE_SIZE), u32::from(FACE_SIZE))
        );
        assert!(state.image.pixels().all(|p| *p == Rgb([0, 0, 0])));
    }

    // A new profile for the model of the test model, and its settings: one
    // knob light, and no haptic feedback.
    assert_eq!(rig.engine.profile().model, TEST_MODEL.model);
    assert_eq!(rig.engine.active_profile(), "default");
    let settings = DeviceSettings::defaults(TEST_MODEL.settings);
    assert_eq!(settings.knob_lights.colors.len(), 1);
    assert_eq!(events.last(), Some(&Event::DeviceSettings(settings)));
}

#[test]
fn the_full_draw_has_the_settings_and_the_slots_of_the_test_model() {
    let (mut rig, _requests) = test_model_rig();
    rig.set_color(face(2), RED);
    rig.take_events();
    let device = connect_test_model(&mut rig);
    assert_eq!(rig.take_events(), [Event::Firmware(None), Event::Connected]);

    let writes = device.borrow().writes().to_vec();
    assert!(writes.iter().all(|w| w.len() == 1 + TEST_MODEL.packet_size));
    // The start sequence, then the knob light of the one knob, in the mode
    // "effect" at full brightness. The model has no haptic feedback, so no
    // `QUCMD`.
    let start: Vec<Vec<u8>> = [
        protocol::init(),
        protocol::set_brightness(80),
        protocol::clear_slot(0xFF),
        protocol::commit(),
        protocol::set_led_colors(&[[0, 0, 0]]),
        protocol::set_led_brightness(100),
    ]
    .into_iter()
    .map(full_report)
    .collect();
    assert_eq!(&writes[..6], start.as_slice());

    // Then one PNG for each key face, in its own slot, and the commit.
    let sent = sent(&device);
    let images: Vec<(u8, &Vec<u8>)> = sent[6..sent.len() - 1]
        .iter()
        .map(|s| match s {
            Sent::Image { slot, data } => (*slot, data),
            other => panic!("not an image: {other:?}"),
        })
        .collect();
    assert_eq!(
        images.iter().map(|(slot, _)| *slot).collect::<Vec<_>>(),
        (1..=6).map(slot_of).collect::<Vec<_>>()
    );
    assert_eq!(sent.last(), Some(&op("STP")));
    for (slot, data) in images {
        assert_eq!(&data[1..4], b"PNG", "slot {slot}");
        let shown = upright(data);
        assert_eq!(shown.dimensions(), (96, 96));
        let want = if slot == slot_of(2) {
            RED
        } else {
            Color::BLACK
        };
        assert_color(&shown, 5, 5, want);
    }
}

#[test]
fn a_change_draws_one_png_and_the_device_shows_the_image_of_the_shell() {
    let (mut rig, _requests) = test_model_rig();
    let device = connect_test_model(&mut rig);
    sent(&device);
    rig.take_events();

    rig.set_color(face(5), BLUE);
    rig.engine.command(Command::SetLabel {
        surface: face(5),
        label: Some("Mail".into()),
    });
    let events = rig.take_events();
    let state = last_state(&events, face(5));
    assert_eq!(state.content.label.as_deref(), Some("Mail"));

    let sent = sent(&device);
    assert_eq!(sent.len(), 4, "two images and two commits: {sent:?}");
    // PNG keeps each pixel, so the device gets the same image as the shell
    // (architecture rule 11).
    assert_eq!(
        upright(match &sent[2] {
            Sent::Image { slot, data } if *slot == slot_of(5) => data,
            other => panic!("not the image of key face 5: {other:?}"),
        }),
        *state.image
    );
    assert_eq!(sent[3], op("STP"));
}

#[test]
fn the_bindings_of_the_test_model_use_its_own_input_codes() {
    let (mut rig, requests) = test_model_rig();
    let key_6 = binding(ControlKind::Key, 6, GestureKind::Press);
    let turn = binding(ControlKind::Knob, 1, GestureKind::TurnRight);
    let push = binding(ControlKind::Knob, 1, GestureKind::Push);
    for id in [key_6, turn, push] {
        set_binding(&mut rig, id, Some(action(KIND)));
    }
    assert_eq!(requests.try_iter().count(), 3, "one Appear each");
    let device = connect_test_model(&mut rig);
    rig.take_events();

    let (_, right, push_code) = KNOB_CODES;
    for (code, state) in [
        (key_code(6), 1),
        (key_code(6), 0),
        (right, 0),
        (push_code, 1),
        // Key 6 and knob 2 of the N4 Pro: not codes of the test model.
        (0x06, 1),
        (0x51, 0),
    ] {
        device.borrow_mut().push_input(control_report(code, state));
        rig.engine.poll(POLL_INTERVAL);
    }
    let invoked: Vec<BindingId> = requests
        .try_iter()
        .map(|r| match r {
            Request::Invoke { binding, action } => {
                assert_eq!(action, super::action(KIND));
                binding
            }
            other => panic!("not an invoke: {other:?}"),
        })
        .collect();
    assert_eq!(invoked, [key_6, turn, push]);
    let inputs = rig
        .take_events()
        .into_iter()
        .filter(|e| matches!(e, Event::Input(_)))
        .count();
    assert_eq!(inputs, 4, "the two codes of the N4 Pro give no input event");

    // The binding is in the profile file of the test model.
    rig.poll_for(Duration::from_secs(1));
    let text = profile_file(&rig, "default").unwrap();
    assert!(text.contains("model = \"opencraft-test-model\""), "{text}");
    assert!(text.contains("key-6"), "{text}");
    assert!(text.contains("knob-1"), "{text}");
}

#[test]
fn the_engine_refuses_the_surfaces_that_the_test_model_does_not_have() {
    let (mut rig, _requests) = test_model_rig();
    rig.take_events();
    for surface in [
        face(7),
        SurfaceId::new(SurfaceKind::BarPanel, 1),
        SurfaceId::new(SurfaceKind::Background, 1),
    ] {
        rig.set_color(surface, RED);
        let events = rig.take_events();
        assert_eq!(problems(&events).len(), 1, "{surface}");
        assert_eq!(events.len(), 1, "{surface}");
    }
}

#[test]
fn a_device_of_the_test_model_after_the_n4_pro_gets_its_own_profile_and_layout() {
    // The engine of the application starts with the layout of the first
    // descriptor of the table.
    let mut rig = Rig::new();
    rig.set_color(KEY_3, RED);
    rig.take_events();

    let device = connect_test_model(&mut rig);
    let events = rig.take_events();
    assert!(matches!(&events[0], Event::Layout(l) if l.name == TEST_MODEL.name));
    assert_eq!(surfaces(&events).len(), 6);
    assert_eq!(rig.engine.profile().model, TEST_MODEL.model);
    assert_ne!(rig.engine.active_profile(), "default");
    assert_eq!(
        events
            .iter()
            .find(|e| matches!(e, Event::DeviceSettings(_))),
        Some(&Event::DeviceSettings(DeviceSettings::defaults(
            TEST_MODEL.settings
        )))
    );
    assert_eq!(events.last(), Some(&Event::Connected));
    // The change on the N4 Pro is in its file.
    assert!(profile_file(&rig, "default").unwrap().contains("#C81E1E"));
    // Key face 3 of the test model has no content.
    let drawn = sent(&device);
    let key_3 = drawn
        .iter()
        .find_map(|s| match s {
            Sent::Image { slot, data } if *slot == slot_of(3) => Some(data),
            _ => None,
        })
        .unwrap();
    assert_color(&upright(key_3), 5, 5, Color::BLACK);

    // The N4 Pro gets its profile and its layout again.
    device.borrow_mut().disconnect();
    rig.poll_for(Duration::from_secs(1));
    let n4 = rig.connect();
    assert_eq!(rig.engine.active_profile(), "default");
    let events = rig.take_events();
    assert_eq!(events[0], Event::Disconnected);
    assert!(matches!(&events[1], Event::Layout(l) if l.name == DESCRIPTORS[0].name));
    assert_color(&last_state(&events, KEY_3).image, 2, 2, RED);
    assert_full_draw(&sent(&n4));
}

#[test]
fn the_test_model_settings_differ_from_the_n4_pro_in_each_capability() {
    // The test has a meaning only if the test model is different.
    let n4: SettingsSupport = DESCRIPTORS[0].settings;
    let test = TEST_MODEL.settings;
    assert_ne!(n4.lowest_brightness, test.lowest_brightness);
    assert_ne!(n4.knob_lights, test.knob_lights);
    assert_ne!(n4.haptic, test.haptic);
}
