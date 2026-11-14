//! A second device model that is not a real device: the test model.
//!
//! The test model is only in the tests. It is not in the table of the real
//! models (`DESCRIPTORS` in `crates/core/src/device.rs`), and no device has
//! its USB identity. It is different from the N4 Pro in each value that a
//! model can change: the identity, the number of keys and knobs, the slots,
//! the size and the format and the rotation of the key faces, the input codes,
//! and the device settings. It has no bar panel, no touch bar, and no
//! background.
//!
//! The tests use it to show that a new model is new data only (architecture
//! rule 4). The device layer tests are in `crates/core/tests/test_model.rs`.
//! The engine tests include this file from `crates/engine/tests/engine.rs`.
//! See `docs/ADDING-A-DEVICE-MODEL.md`.

// Each test crate that includes this file uses a different part of it.
#![allow(dead_code)]

use opencraft_core::{
    protocol, Control, Descriptor, ImageFormat, InputCodes, Position, Rotation, SettingsSupport,
    Surface, SurfaceId, SurfaceKind,
};

/// The size of each key face of the test model, in pixels.
pub const FACE_SIZE: u16 = 96;

/// The slot of key face `number`. The slots of the N4 Pro are 1 to 15, so
/// these slots are different.
pub const fn slot_of(number: u8) -> u8 {
    20 + number
}

/// The input code of key `number`.
pub const fn key_code(number: u8) -> u8 {
    0x10 + number
}

/// The input codes of knob 1: turn left, turn right, push.
pub const KNOB_CODES: (u8, u8, u8) = (0x21, 0x22, 0x23);

/// Key face `number` in a grid of 3 columns. A space of 4 pixels is around
/// each key face.
const fn face(number: u8) -> Surface {
    let index = (number - 1) as u16;
    Surface {
        id: SurfaceId::new(SurfaceKind::KeyFace, number),
        slot: Some(slot_of(number)),
        x: 4 + (index % 3) * 100,
        y: 4 + (index / 3) * 100,
        width: FACE_SIZE,
        height: FACE_SIZE,
        format: ImageFormat::Png,
        rotation: Rotation::Deg0,
    }
}

/// Key `number` in a grid of 3 columns, with the key face of the same number.
const fn key(number: u8) -> Control {
    Control {
        number,
        codes: InputCodes::Key {
            code: key_code(number),
        },
        position: Position {
            row: (number - 1) / 3 + 1,
            column: (number - 1) % 3 + 1,
        },
        surface: Some(SurfaceId::new(SurfaceKind::KeyFace, number)),
        feedback_surface: None,
    }
}

static SURFACES: &[Surface] = &[face(1), face(2), face(3), face(4), face(5), face(6)];

static CONTROLS: &[Control] = &[
    key(1),
    key(2),
    key(3),
    key(4),
    key(5),
    key(6),
    Control {
        number: 1,
        codes: InputCodes::Knob {
            turn_left: KNOB_CODES.0,
            turn_right: KNOB_CODES.1,
            push: KNOB_CODES.2,
        },
        position: Position { row: 1, column: 1 },
        surface: None,
        feedback_surface: None,
    },
];

/// The test model: 6 keys with displays in 2 rows of 3, and 1 knob with 1
/// knob light. No device has the USB identity `0xFFFF:0x7E57`.
pub static TEST_MODEL: Descriptor = Descriptor {
    name: "OpenCraft test model (not a real device)",
    model: "opencraft-test-model",
    vid: 0xFFFF,
    pid: 0x7E57,
    // The protocol, version 3. It is not model data: see
    // `docs/ADDING-A-DEVICE-MODEL.md`.
    packet_size: protocol::PACKET_SIZE,
    surfaces: SURFACES,
    controls: CONTROLS,
    settings: SettingsSupport {
        lowest_brightness: 0,
        knob_lights: 1,
        haptic: false,
    },
};
