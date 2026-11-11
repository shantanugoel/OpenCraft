//! Input report decoding.
//!
//! The decoder knows the report format of the protocol. It does not know the
//! input codes of a model: it gets them from the [`Descriptor`]. The facts are
//! in `docs/kb/streamdock-protocol.md` (section "Input reports").
//!
//! A control report has `"ACK"` at bytes 0 to 2, `"OK"` at bytes 5 to 6, the
//! input code at byte 9, the state at byte 10, and the x coordinate of the
//! start of a touch (big-endian) at bytes 11 and 12. A touch report has `"ACK"`
//! at bytes 0 to 2 and `"ARX"` at bytes 4 to 6. It has x at bytes 10 and 11
//! and y at bytes 12 and 13 (big-endian).
//!
//! The state byte of a control report is `0x01` when a key or a finger goes
//! down, and `0x00` when it goes up. A knob turn has the state `0x00`, and a
//! knob push has the state `0x01`. The decoder accepts only these states. A
//! report with a different state is not known.

use crate::device::{ControlId, Descriptor, InputCodes};

/// The state of a control report when a key or a finger goes down.
const STATE_DOWN: u8 = 0x01;
/// The state of a control report when a key or a finger goes up.
const STATE_UP: u8 = 0x00;
/// The input code of an acknowledgement. It is not user input.
const ACK_CODE: u8 = 0xFF;

/// One thing that the user does to a control.
///
/// A gesture has the coordinates that the device gives with it. The
/// coordinates are in touch bar pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gesture {
    /// A key goes down.
    Press,
    /// A key goes up.
    Release,
    /// A knob turns one click to the left. A turn has no release.
    TurnLeft,
    /// A knob turns one click to the right.
    TurnRight,
    /// A knob is pushed. The device sends no report for the release.
    Push,
    /// A finger lifts from a bar panel with no move. `x` is the x coordinate
    /// of the start of the touch.
    Tap { x: u16 },
    /// A finger touches the touch bar at `x`.
    TouchStart { x: u16 },
    /// The point of a finger on the touch bar. The device sends one each 35 ms
    /// while a finger is on the touch bar.
    TouchPoint { x: u16, y: u16 },
    /// A finger lifts after a move to the left.
    SwipeLeft,
    /// A finger lifts after a move to the right.
    SwipeRight,
    /// A finger lifts, and the device did not identify the gesture.
    TouchEnd,
}

impl Gesture {
    /// The name of the gesture, with no coordinates.
    pub fn name(&self) -> &'static str {
        match self {
            Gesture::Press => "press",
            Gesture::Release => "release",
            Gesture::TurnLeft => "turn left",
            Gesture::TurnRight => "turn right",
            Gesture::Push => "push",
            Gesture::Tap { .. } => "tap",
            Gesture::TouchStart { .. } => "touch start",
            Gesture::TouchPoint { .. } => "touch point",
            Gesture::SwipeLeft => "swipe left",
            Gesture::SwipeRight => "swipe right",
            Gesture::TouchEnd => "touch end",
        }
    }
}

/// One gesture on one control, decoded from a device report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputEvent {
    pub control: ControlId,
    pub gesture: Gesture,
}

/// The result of the decoder for one report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoded {
    /// User input.
    Input(InputEvent),
    /// An acknowledgement of a command. It is not user input.
    Ack,
    /// A report that the decoder does not know, with all its bytes.
    Unknown(Vec<u8>),
}

/// Decode one input report of a device of the model `descriptor`.
pub fn decode(descriptor: &Descriptor, report: &[u8]) -> Decoded {
    match decode_input(descriptor, report) {
        Ok(Some(event)) => Decoded::Input(event),
        Ok(None) => Decoded::Ack,
        Err(()) => Decoded::Unknown(report.to_vec()),
    }
}

/// `Ok(None)` is an acknowledgement. `Err(())` is a report that is not known.
fn decode_input(descriptor: &Descriptor, report: &[u8]) -> Result<Option<InputEvent>, ()> {
    if report.len() < 11 || &report[0..3] != b"ACK" {
        return Err(());
    }

    // A touch report has "ARX" at bytes 4 to 6, not "OK" at bytes 5 to 6.
    if report.len() >= 14 && &report[4..7] == b"ARX" {
        let touch_bar = descriptor
            .controls
            .iter()
            .find(|c| matches!(c.codes, InputCodes::TouchBar { .. }))
            .ok_or(())?;
        return Ok(Some(InputEvent {
            control: touch_bar.id(),
            gesture: Gesture::TouchPoint {
                x: u16::from_be_bytes([report[10], report[11]]),
                y: u16::from_be_bytes([report[12], report[13]]),
            },
        }));
    }

    if &report[5..7] != b"OK" {
        return Err(());
    }
    let code = report[9];
    if code == ACK_CODE {
        return Ok(None);
    }
    let state = report[10];
    // The x coordinate of the start of a touch. A report that is too short
    // does not have it.
    let x = report.get(11..13).map(|b| u16::from_be_bytes([b[0], b[1]]));

    // The descriptor has each code one time only, so the first control with
    // the code is the only one.
    let control = descriptor
        .controls
        .iter()
        .find(|c| c.codes.has(code))
        .ok_or(())?;
    let gesture = gesture(&control.codes, code, state, x).ok_or(())?;
    Ok(Some(InputEvent {
        control: control.id(),
        gesture,
    }))
}

/// The gesture of a control report with `code`, `state`, and `x`, for a
/// control with the codes `codes`. `None` if the report does not agree with
/// the protocol.
fn gesture(codes: &InputCodes, code: u8, state: u8, x: Option<u16>) -> Option<Gesture> {
    let gesture = match *codes {
        InputCodes::Key { .. } => match state {
            STATE_DOWN => Gesture::Press,
            STATE_UP => Gesture::Release,
            _ => return None,
        },
        InputCodes::Knob {
            turn_left,
            turn_right,
            push,
        } => match state {
            STATE_UP if code == turn_left => Gesture::TurnLeft,
            STATE_UP if code == turn_right => Gesture::TurnRight,
            STATE_DOWN if code == push => Gesture::Push,
            _ => return None,
        },
        InputCodes::BarPanel { .. } if state == STATE_UP => Gesture::Tap { x: x? },
        InputCodes::BarPanel { .. } => return None,
        InputCodes::TouchBar {
            touch_start,
            swipe_right,
            swipe_left,
            touch_end,
        } => match state {
            STATE_DOWN if code == touch_start => Gesture::TouchStart { x: x? },
            STATE_UP if code == swipe_right => Gesture::SwipeRight,
            STATE_UP if code == swipe_left => Gesture::SwipeLeft,
            STATE_UP if code == touch_end => Gesture::TouchEnd,
            _ => return None,
        },
    };
    Some(gesture)
}
