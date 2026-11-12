//! The device layer operates a second model that is data only.
//!
//! The test model is in `tests/support/test_model.rs`. It is not a real
//! device, and it is not in the table of the real models. The tests find it
//! in a table, select its vendor interface, decode its input with its own
//! codes, and send images to its own surfaces through the fake transport. No
//! code of the device layer changes for it (architecture rule 4). See
//! `docs/ADDING-A-DEVICE-MODEL.md`.

#[path = "support/test_model.rs"]
mod test_model;

use image::{Rgb, RgbImage};
use opencraft_core::device::{self, DESCRIPTORS};
use opencraft_core::recording;
use opencraft_core::{
    decode, image as surface_image, select, ControlId, ControlKind, Decoded, Descriptor, Device,
    Error, FakeTransport, Found, Gesture, InputEvent, SurfaceKind,
};
use std::ffi::CString;
use test_model::{key_code, slot_of, FACE_SIZE, KNOB_CODES, TEST_MODEL};

/// A table with the first real model and the test model.
static TABLE: [Descriptor; 2] = [DESCRIPTORS[0], TEST_MODEL];

/// A control report with `code` and `state`, in the format of the protocol.
fn control_report(code: u8, state: u8) -> Vec<u8> {
    let mut r = vec![0u8; 512];
    r[..3].copy_from_slice(b"ACK");
    r[5..7].copy_from_slice(b"OK");
    r[9] = code;
    r[10] = state;
    r
}

fn input(kind: ControlKind, number: u8, gesture: Gesture) -> Decoded {
    Decoded::Input(InputEvent {
        control: ControlId::new(kind, number),
        gesture,
    })
}

/// An entry of the enumeration for the test model. The path does not exist.
fn entry(usage_page: u16, interface: i32) -> Found {
    Found {
        vid: TEST_MODEL.vid,
        pid: TEST_MODEL.pid,
        product: "Test device".to_string(),
        serial: None,
        usage_page,
        interface,
        known: device::find_in(&TABLE, TEST_MODEL.vid, TEST_MODEL.pid),
        path: CString::new(format!("test-model-path-{interface}")).unwrap(),
    }
}

// --- Identity --------------------------------------------------------------------

#[test]
fn the_test_model_is_not_in_the_table_of_real_models() {
    assert!(device::find(TEST_MODEL.vid, TEST_MODEL.pid).is_none());
    for d in DESCRIPTORS {
        assert_ne!(
            (d.vid, d.pid),
            (TEST_MODEL.vid, TEST_MODEL.pid),
            "{}",
            d.name
        );
        assert_ne!(d.model, TEST_MODEL.model, "{}", d.name);
        assert!(!d.name.contains("test model"), "{}", d.name);
    }
    assert!(!device::KNOWN_VIDS.contains(&TEST_MODEL.vid));
}

#[test]
fn the_lookup_finds_each_model_of_a_table_by_its_usb_identity() {
    let found = device::find_in(&TABLE, TEST_MODEL.vid, TEST_MODEL.pid).unwrap();
    assert!(std::ptr::eq(found, &TABLE[1]));
    assert_eq!(found.model, "opencraft-test-model");
    let n4 = device::find_in(&TABLE, DESCRIPTORS[0].vid, DESCRIPTORS[0].pid).unwrap();
    assert_eq!(n4.model, "n4-pro");
    assert!(device::find_in(&TABLE, TEST_MODEL.vid, 0x1023).is_none());
    assert!(device::find_in(&TABLE, 0x5548, TEST_MODEL.pid).is_none());
}

#[test]
fn only_the_vendor_interface_of_the_test_model_is_selected() {
    let vendor = entry(0xFFA0, 0);
    let keyboard = entry(0x0001, 1);
    for list in [
        vec![keyboard.clone(), vendor.clone()],
        vec![vendor.clone(), keyboard.clone()],
    ] {
        let chosen = select(&list).unwrap();
        assert_eq!(chosen.path, vendor.path);
        assert!(std::ptr::eq(chosen.openable().unwrap(), &TABLE[1]));
        assert_eq!(chosen.label(), TEST_MODEL.name);
    }
    assert!(matches!(
        select(std::slice::from_ref(&keyboard)),
        Err(Error::NoVendorInterface)
    ));
    // Architecture rule 6: the check comes before any `hidapi` call.
    assert!(matches!(
        Device::open(&keyboard),
        Err(Error::NoVendorInterface)
    ));
}

// --- Input -----------------------------------------------------------------------

#[test]
fn the_decoder_gives_the_events_of_the_codes_of_the_test_model() {
    for number in 1..=6 {
        assert_eq!(
            decode(&TEST_MODEL, &control_report(key_code(number), 0x01)),
            input(ControlKind::Key, number, Gesture::Press)
        );
        assert_eq!(
            decode(&TEST_MODEL, &control_report(key_code(number), 0x00)),
            input(ControlKind::Key, number, Gesture::Release)
        );
    }
    let (left, right, push) = KNOB_CODES;
    let knob = |gesture| input(ControlKind::Knob, 1, gesture);
    assert_eq!(
        decode(&TEST_MODEL, &control_report(left, 0x00)),
        knob(Gesture::TurnLeft)
    );
    assert_eq!(
        decode(&TEST_MODEL, &control_report(right, 0x00)),
        knob(Gesture::TurnRight)
    );
    assert_eq!(
        decode(&TEST_MODEL, &control_report(push, 0x01)),
        knob(Gesture::Push)
    );
    // The acknowledgement is the protocol, not the model.
    assert_eq!(
        decode(&TEST_MODEL, &control_report(0xFF, 0x00)),
        Decoded::Ack
    );
}

#[test]
fn the_test_model_does_not_know_the_codes_of_the_n4_pro() {
    // Key 1 and knob 1 of the N4 Pro, a tap on bar panel 1, and the start of
    // a touch.
    for (code, state) in [(0x01, 0x01), (0xA0, 0x00), (0x40, 0x00), (0x00, 0x01)] {
        let r = control_report(code, state);
        assert_eq!(decode(&TEST_MODEL, &r), Decoded::Unknown(r));
    }
    // The test model has no touch bar, so a touch report is not known.
    let mut touch = vec![0u8; 512];
    touch[..3].copy_from_slice(b"ACK");
    touch[4..7].copy_from_slice(b"ARX");
    assert_eq!(decode(&TEST_MODEL, &touch), Decoded::Unknown(touch));
}

#[test]
fn a_recording_in_the_fixture_format_decodes_with_the_test_model() {
    // A recording of a new model has this format: the header, then one line
    // for each report. The bytes here are invented, not recorded.
    let mut text = recording::header(
        "Key, top row, 1st from the left: press it 1 time",
        TEST_MODEL.name,
        "(unavailable)",
    );
    for (ms, report) in [
        (0, control_report(key_code(1), 0x01)),
        (90, control_report(key_code(1), 0x00)),
    ] {
        text += &recording::format_report_line(ms, &report);
        text.push('\n');
    }
    let rec = recording::parse(&text).unwrap();
    assert_eq!(rec.device, TEST_MODEL.name);
    let events: Vec<Decoded> = rec
        .reports
        .iter()
        .map(|r| decode(&TEST_MODEL, &r.bytes))
        .collect();
    assert_eq!(
        events,
        [
            input(ControlKind::Key, 1, Gesture::Press),
            input(ControlKind::Key, 1, Gesture::Release),
        ]
    );
}

// --- Output ----------------------------------------------------------------------

/// The writes of the fake transport, with the report ID and the padding.
fn writes(device: Device<FakeTransport>) -> Vec<Vec<u8>> {
    device.into_transport().take_writes()
}

#[test]
fn each_key_face_gets_an_upright_png_of_its_size_in_its_own_slot() {
    let mut dev = Device::new(FakeTransport::new(), &TEST_MODEL);
    let faces: Vec<_> = TEST_MODEL.slot_surfaces().collect();
    assert_eq!(faces.len(), 6);
    let mut sent = Vec::new();
    for surface in &faces {
        assert_eq!(surface.kind(), SurfaceKind::KeyFace);
        // A red mark at the top left corner of an upright blue image.
        let size = u32::from(FACE_SIZE);
        let mut img = RgbImage::from_pixel(size, size, Rgb([0, 0, 200]));
        for x in 0..8 {
            for y in 0..8 {
                img.put_pixel(x, y, Rgb([255, 0, 0]));
            }
        }
        let data = surface_image::encode_for(surface, &img, 90).unwrap();
        dev.send_image(surface.slot.unwrap(), &data).unwrap();
        sent.push((surface.slot.unwrap(), data));
    }
    dev.commit().unwrap();

    let writes = writes(dev);
    let mut i = 0;
    for (slot, data) in &sent {
        let header = &writes[i];
        assert_eq!(header.len(), 1 + TEST_MODEL.packet_size);
        assert_eq!(&header[..10], b"\x00CRT\x00\x00BAT\x00");
        let len = u32::from_be_bytes(header[9..13].try_into().unwrap()) as usize;
        assert_eq!(len, data.len());
        assert_eq!(header[13], *slot);
        let parts = data.len().div_ceil(TEST_MODEL.packet_size);
        let mut joined = Vec::new();
        for part in &writes[i + 1..i + 1 + parts] {
            joined.extend_from_slice(&part[1..]);
        }
        joined.truncate(len);
        // The PNG of the test model is not turned: the mark stays at the top
        // left corner.
        let shown = image::load_from_memory(&joined).unwrap().into_rgb8();
        assert_eq!(shown.dimensions(), (96, 96));
        assert_eq!(*shown.get_pixel(1, 1), Rgb([255, 0, 0]));
        assert_eq!(*shown.get_pixel(94, 94), Rgb([0, 0, 200]));
        i += 1 + parts;
    }
    assert_eq!(&writes[i][..10], b"\x00CRT\x00\x00STP\x00");
    assert_eq!(writes.len(), i + 1);
    let slots: Vec<u8> = sent.iter().map(|(slot, _)| *slot).collect();
    assert_eq!(slots, (1..=6).map(slot_of).collect::<Vec<_>>());
}

#[test]
fn the_device_layer_refuses_the_surfaces_that_the_test_model_does_not_have() {
    let mut fake = FakeTransport::new();
    let mut dev = Device::new(&mut fake, &TEST_MODEL);
    // Slot 6 is key face 6 of the N4 Pro.
    assert!(matches!(
        dev.send_image(6, &[1, 2, 3]),
        Err(Error::NoSuchSlot(6))
    ));
    assert!(matches!(
        dev.send_background(&[1, 2, 3]),
        Err(Error::NoBackground)
    ));
    assert!(matches!(dev.clear_background(), Err(Error::NoBackground)));
    assert!(fake.writes().is_empty());
}

#[test]
fn the_start_and_the_stop_are_the_commands_of_the_protocol() {
    let mut dev = Device::new(FakeTransport::new(), &TEST_MODEL);
    dev.start(80).unwrap();
    dev.stop().unwrap();
    let ops: Vec<Vec<u8>> = writes(dev)
        .iter()
        .map(|w| {
            assert_eq!(w.len(), 1 + TEST_MODEL.packet_size);
            w[6..]
                .iter()
                .copied()
                .take_while(u8::is_ascii_uppercase)
                .collect()
        })
        .collect();
    assert_eq!(ops, [&b"DIS"[..], b"LIG", b"CLE", b"STP", b"CLE", b"HAN"]);
}
