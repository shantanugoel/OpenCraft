//! Input report decoding.
//!
//! Input packets are `"ACK" ... "OK" <code> <state>`. Touch reports use
//! `"ACK" ? "ARX"` with big-endian x/y at bytes 10..14.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A key or knob-adjacent button.
    Key { code: u8, pressed: bool },
    /// A knob was turned.
    KnobRotate { knob: u8, dir: Direction },
    /// A knob was pushed.
    KnobPress { knob: u8, pressed: bool },
    /// A swipe across the touch bar.
    Swipe { dir: Direction },
    /// A point on the touch bar, in touch-bar pixel space.
    Touch { x: u16, y: u16 },
    /// Write acknowledgement. Not user input.
    Ack,
    /// Something we do not recognise yet.
    Unknown(Vec<u8>),
}

pub fn decode(data: &[u8]) -> Event {
    if data.len() < 11 || &data[0..3] != b"ACK" {
        return Event::Unknown(data.to_vec());
    }

    // Touch reports must be checked before the generic event path: they carry
    // "ARX" at 4..7 instead of "OK" at 5..7.
    if data.len() >= 14 && &data[4..7] == b"ARX" {
        return Event::Touch {
            x: u16::from_be_bytes([data[10], data[11]]),
            y: u16::from_be_bytes([data[12], data[13]]),
        };
    }

    if &data[5..7] == b"OK" {
        let code = data[9];
        let pressed = data[10] == 0x01;
        if code == 0xFF {
            return Event::Ack;
        }
        return decode_code(code, pressed);
    }

    Event::Unknown(data.to_vec())
}

fn decode_code(code: u8, pressed: bool) -> Event {
    match code {
        // Knob rotation reports on release only.
        0xA0 => Event::KnobRotate {
            knob: 1,
            dir: Direction::Left,
        },
        0xA1 => Event::KnobRotate {
            knob: 1,
            dir: Direction::Right,
        },
        0x50 => Event::KnobRotate {
            knob: 2,
            dir: Direction::Left,
        },
        0x51 => Event::KnobRotate {
            knob: 2,
            dir: Direction::Right,
        },
        0x90 => Event::KnobRotate {
            knob: 3,
            dir: Direction::Left,
        },
        0x91 => Event::KnobRotate {
            knob: 3,
            dir: Direction::Right,
        },
        0x70 => Event::KnobRotate {
            knob: 4,
            dir: Direction::Left,
        },
        0x71 => Event::KnobRotate {
            knob: 4,
            dir: Direction::Right,
        },

        0x37 => Event::KnobPress { knob: 1, pressed },
        0x35 => Event::KnobPress { knob: 2, pressed },
        0x33 => Event::KnobPress { knob: 3, pressed },
        0x36 => Event::KnobPress { knob: 4, pressed },

        0x38 => Event::Swipe {
            dir: Direction::Left,
        },
        0x39 => Event::Swipe {
            dir: Direction::Right,
        },

        _ => Event::Key { code, pressed },
    }
}

/// Human-readable label for a raw key code, using the vendor's numbering.
pub fn key_label(code: u8) -> String {
    match code {
        0x40..=0x43 => format!("key {}", code - 0x40 + 1),
        0x01..=0x05 => format!("key {}", code + 10),
        0x06..=0x0A => format!("key {}", code),
        other => format!("code 0x{other:02X}"),
    }
}
