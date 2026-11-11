//! Input report decoding.
//!
//! The report format and the input codes are measured. See
//! `docs/kb/streamdock-protocol.md` (section "Input reports") and
//! `docs/kb/n4-pro-hardware.md` (section "Controls and input codes").
//!
//! A control report has `"ACK"` at bytes 0 to 2, `"OK"` at bytes 5 to 6, the
//! input code at byte 9, the state at byte 10, and the x coordinate of the
//! start of a touch (big-endian) at bytes 11 and 12. A touch report has `"ACK"`
//! at bytes 0 to 2 and `"ARX"` at bytes 4 to 6. It has x at bytes 10 and 11
//! and y at bytes 12 and 13 (big-endian).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A key. The device sends one report for the press (`pressed` is true)
    /// and one report for the release.
    Key { code: u8, pressed: bool },
    /// A knob was turned by one click. A turn has no release report.
    KnobRotate { knob: u8, dir: Direction },
    /// A knob was pushed. The device sends one report, with the state `0x01`,
    /// for the push. It sends no report for the release. Thus `pressed` is
    /// true for each report that was measured.
    KnobPress { knob: u8, pressed: bool },
    /// A finger touched the touch bar. `x` is the x coordinate of the start of
    /// the touch, in touch-bar pixels.
    TouchStart { x: u16 },
    /// A touch ended with no move. `panel` is the bar panel (1 to 4, from left
    /// to right) where the touch started. `x` is the x coordinate of the start
    /// of the touch.
    Tap { panel: u8, x: u16 },
    /// A touch ended after a move across the touch bar. `dir` is the direction
    /// of the move of the finger.
    Swipe { dir: Direction },
    /// A touch ended and the device did not identify the gesture. The meaning
    /// of this report is not known.
    TouchEnd,
    /// A point on the touch bar, in touch-bar pixel space. The device sends one
    /// each 35 ms while a finger is on the touch bar.
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
        if data[9] == 0xFF {
            return Event::Ack;
        }
        return decode_code(data);
    }

    Event::Unknown(data.to_vec())
}

/// Decode a control report. The caller checked that `data` has at least 11
/// bytes and that bytes 5 to 6 are `"OK"`.
fn decode_code(data: &[u8]) -> Event {
    let code = data[9];
    let pressed = data[10] == 0x01;
    // The x coordinate of the start of a touch. A report that is too short
    // does not have it.
    let x = data.get(11..13).map(|b| u16::from_be_bytes([b[0], b[1]]));

    match code {
        // A knob turn sends one report for each click, with the state 0x00.
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

        // Finger down on the touch bar.
        0x00 if pressed => match x {
            Some(x) => Event::TouchStart { x },
            None => Event::Unknown(data.to_vec()),
        },
        // Finger up with no move. The code gives the bar panel where the touch
        // started.
        0x40..=0x43 if !pressed => match x {
            Some(x) => Event::Tap {
                panel: code - 0x40 + 1,
                x,
            },
            None => Event::Unknown(data.to_vec()),
        },
        // Finger up after a move. The code gives the direction of the move of
        // the finger.
        0x38 => Event::Swipe {
            dir: Direction::Right,
        },
        0x39 => Event::Swipe {
            dir: Direction::Left,
        },
        // Finger up, gesture not identified.
        0xB2 => Event::TouchEnd,

        // The device sends code 0x00 with the state 0x01 only, and the codes
        // 0x40 to 0x43 with the state 0x00 only. Any other state is not known.
        0x00 | 0x40..=0x43 => Event::Unknown(data.to_vec()),

        _ => Event::Key { code, pressed },
    }
}

/// Human-readable label for a raw key code. The label is the slot number of
/// the key. Codes `0x01` to `0x05` are the top row, from left to right (slots
/// 11 to 15). Codes `0x06` to `0x0A` are the second row, from left to right
/// (slots 6 to 10).
pub fn key_label(code: u8) -> String {
    match code {
        0x01..=0x05 => format!("key {}", code + 10),
        0x06..=0x0A => format!("key {}", code),
        other => format!("code 0x{other:02X}"),
    }
}
