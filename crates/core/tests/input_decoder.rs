//! Tests for the input decoder, with the recorded reports of the VSD N4 Pro.
//!
//! The fixtures are in `tests/fixtures/input/`. Each file holds the reports of
//! one gesture. The facts that the tests check are in
//! `docs/kb/n4-pro-hardware.md` (section "Controls and input codes") and in
//! `docs/kb/streamdock-protocol.md` (section "Input reports"). The tests need
//! no device.
//!
//! The decoder gets the input codes from the descriptor. The tests at the end
//! use an invented descriptor with different codes.

use opencraft_core::recording::{self, Recording};
use opencraft_core::{
    decode as decode_with, device, Control, ControlId, ControlKind, Decoded, Descriptor, Gesture,
    InputCodes, InputEvent, Position,
};
use std::path::PathBuf;

/// The direction of a knob turn or of a swipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Left,
    Right,
}
use Direction::{Left, Right};

/// The descriptor of the device that made the recordings.
fn n4pro() -> &'static Descriptor {
    device::find(0x5548, 0x1023).unwrap()
}

/// Decode one report with the descriptor of the N4 Pro.
fn decode(report: &[u8]) -> Decoded {
    decode_with(n4pro(), report)
}

fn input(control: ControlId, gesture: Gesture) -> Decoded {
    Decoded::Input(InputEvent { control, gesture })
}

fn key(number: u8) -> ControlId {
    ControlId::new(ControlKind::Key, number)
}

fn knob(number: u8) -> ControlId {
    ControlId::new(ControlKind::Knob, number)
}

fn bar_panel(number: u8) -> ControlId {
    ControlId::new(ControlKind::BarPanel, number)
}

fn touch_bar() -> ControlId {
    ControlId::new(ControlKind::TouchBar, 1)
}

fn key_event(number: u8, pressed: bool) -> Decoded {
    input(
        key(number),
        if pressed {
            Gesture::Press
        } else {
            Gesture::Release
        },
    )
}

fn turn(number: u8, dir: Direction) -> Decoded {
    let gesture = match dir {
        Left => Gesture::TurnLeft,
        Right => Gesture::TurnRight,
    };
    input(knob(number), gesture)
}

fn push(number: u8) -> Decoded {
    input(knob(number), Gesture::Push)
}

fn tap(panel: u8, x: u16) -> Decoded {
    input(bar_panel(panel), Gesture::Tap { x })
}

fn touch_start(x: u16) -> Decoded {
    input(touch_bar(), Gesture::TouchStart { x })
}

fn touch_point(x: u16, y: u16) -> Decoded {
    input(touch_bar(), Gesture::TouchPoint { x, y })
}

fn swipe(dir: Direction) -> Decoded {
    let gesture = match dir {
        Left => Gesture::SwipeLeft,
        Right => Gesture::SwipeRight,
    };
    input(touch_bar(), gesture)
}

fn touch_end() -> Decoded {
    input(touch_bar(), Gesture::TouchEnd)
}

fn is_touch_start(e: &Decoded) -> bool {
    matches!(
        e,
        Decoded::Input(InputEvent {
            gesture: Gesture::TouchStart { .. },
            ..
        })
    )
}

fn is_key(e: &Decoded) -> bool {
    matches!(e, Decoded::Input(InputEvent { control, .. }) if control.kind == ControlKind::Key)
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/input")
}

/// The file names of all recordings (not `layout.txt`), sorted.
fn recording_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixture_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".txt") && n.chars().next().unwrap().is_ascii_digit())
        .collect();
    names.sort();
    names
}

/// Read the recording whose name starts with `step`, for example `"13"`.
fn load(step: &str) -> Recording {
    let prefix = format!("{step}-");
    let name = recording_names()
        .into_iter()
        .find(|n| n.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no fixture for step {step}"));
    let text = std::fs::read_to_string(fixture_dir().join(&name)).unwrap();
    recording::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The events of all reports of a recording, in order.
fn events(step: &str) -> Vec<Decoded> {
    load(step)
        .reports
        .iter()
        .map(|r| decode(&r.bytes))
        .collect()
}

/// A report with the given bytes at the start and zero bytes up to 512.
fn report(start: &[u8]) -> Vec<u8> {
    let mut v = start.to_vec();
    v.resize(512, 0);
    v
}

/// A control report: "ACK", `0x00 0x00`, "OK", `0x00 0x00`, code, state, then
/// the x coordinate (big-endian) at bytes 11 and 12.
fn control_report(code: u8, state: u8, x: u16) -> Vec<u8> {
    let mut v = report(b"ACK\x00\x00OK\x00\x00");
    v[9] = code;
    v[10] = state;
    v[11..13].copy_from_slice(&x.to_be_bytes());
    v
}

/// A touch report: "ACK", `0x00`, "ARX", x at bytes 10 and 11, y at bytes 12
/// and 13 (big-endian).
fn touch_report(x: u16, y: u16) -> Vec<u8> {
    let mut v = report(b"ACK\x00ARX");
    v[10..12].copy_from_slice(&x.to_be_bytes());
    v[12..14].copy_from_slice(&y.to_be_bytes());
    v
}

fn touch_points(events: &[Decoded]) -> Vec<(u16, u16)> {
    events
        .iter()
        .filter_map(|e| match e {
            Decoded::Input(InputEvent {
                control,
                gesture: Gesture::TouchPoint { x, y },
            }) if *control == touch_bar() => Some((*x, *y)),
            _ => None,
        })
        .collect()
}

/// Check that the events of one touch are: a touch start with `start_x`, one
/// or more touch points, and `end`. Return the touch points.
fn assert_one_touch(events: &[Decoded], start_x: u16, end: &Decoded) -> Vec<(u16, u16)> {
    assert_eq!(events.first(), Some(&touch_start(start_x)));
    assert_eq!(events.last(), Some(end));
    let middle = &events[1..events.len() - 1];
    assert!(!middle.is_empty(), "no touch report");
    let points = touch_points(middle);
    assert_eq!(
        points.len(),
        middle.len(),
        "a report other than a touch report between the start and the end: {middle:?}"
    );
    points
}

// --- The fixtures ----------------------------------------------------------

#[test]
fn there_are_36_recordings_and_each_parses_with_its_header() {
    let names = recording_names();
    assert_eq!(names.len(), 36, "{names:?}");
    for (i, name) in names.iter().enumerate() {
        assert!(
            name.starts_with(&format!("{:02}-", i + 1)),
            "step {} is missing or out of order: {name}",
            i + 1
        );
        let text = std::fs::read_to_string(fixture_dir().join(name)).unwrap();
        let rec = recording::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!rec.note.is_empty(), "{name}: no note line");
        assert!(!rec.device.is_empty(), "{name}: no device line");
        assert!(!rec.firmware.is_empty(), "{name}: no firmware line");
        assert!(!rec.reports.is_empty(), "{name}: no report");
        assert_eq!(rec.reports[0].ms, 0, "{name}: the first time is not 0");
        assert!(
            rec.reports.windows(2).all(|w| w[0].ms <= w[1].ms),
            "{name}: the times go back"
        );
    }
}

#[test]
fn the_recordings_come_from_one_device_and_one_firmware() {
    let first = load("01");
    assert_eq!(first.device, "VSD StreamDock N4 Pro");
    assert_eq!(first.firmware, "V4.VSD N4 Pro.02.010");
    for name in recording_names() {
        let step = &name[..2];
        let rec = load(step);
        assert_eq!(rec.device, first.device, "{name}");
        assert_eq!(rec.firmware, first.firmware, "{name}");
    }
}

#[test]
fn each_recorded_report_has_512_bytes_and_the_known_header() {
    for name in recording_names() {
        for r in &load(&name[..2]).reports {
            assert_eq!(r.bytes.len(), 512, "{name}");
            assert_eq!(&r.bytes[0..3], b"ACK", "{name}");
            // Control report: "OK" at bytes 5 to 6. Touch report: "ARX" at
            // bytes 4 to 6.
            assert!(
                &r.bytes[5..7] == b"OK" || &r.bytes[4..7] == b"ARX",
                "{name}"
            );
            // All bytes after byte 13 are zero.
            assert!(r.bytes[14..].iter().all(|&b| b == 0), "{name}");
        }
    }
}

#[test]
fn no_recorded_report_decodes_to_unknown_or_ack() {
    for name in recording_names() {
        for (i, ev) in events(&name[..2]).iter().enumerate() {
            assert!(
                matches!(ev, Decoded::Input(_)),
                "{name}, report {i}: {ev:?}"
            );
        }
    }
}

// --- Keys ------------------------------------------------------------------

/// Read a row of `layout.txt`: the slot numbers, from left to right.
fn layout_row(name: &str) -> Vec<u8> {
    let text = std::fs::read_to_string(fixture_dir().join("layout.txt")).unwrap();
    let prefix = format!("{name}:");
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("no {name} in layout.txt"));
    line.split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect()
}

/// The key control of the N4 Pro with the input code `code`.
fn key_with_code(code: u8) -> &'static Control {
    n4pro()
        .controls
        .iter()
        .find(|c| c.codes == InputCodes::Key { code })
        .unwrap_or_else(|| panic!("no key with code 0x{code:02X}"))
}

/// The slot of the key face of the key that sends `code`.
fn key_slot(code: u8) -> u8 {
    let d = n4pro();
    let face = key_with_code(code).surface.expect("a key has a key face");
    d.surface(face).unwrap().slot.unwrap()
}

#[test]
fn key_press_and_release_top_row() {
    // Codes 0x01 to 0x05, from left to right. Steps 01 to 05.
    for (i, step) in ["01", "02", "03", "04", "05"].iter().enumerate() {
        let code = i as u8 + 1;
        let control = key_with_code(code);
        assert_eq!(
            control.position,
            Position {
                row: 1,
                column: i as u8 + 1
            }
        );
        let n = control.number;
        assert_eq!(
            events(step),
            vec![key_event(n, true), key_event(n, false)],
            "step {step}"
        );
    }
}

#[test]
fn key_press_and_release_second_row() {
    // Codes 0x06 to 0x0A, from left to right. Steps 06 to 10.
    for (i, step) in ["06", "07", "08", "09", "10"].iter().enumerate() {
        let code = i as u8 + 6;
        let control = key_with_code(code);
        assert_eq!(
            control.position,
            Position {
                row: 2,
                column: i as u8 + 1
            }
        );
        let n = control.number;
        assert_eq!(
            events(step),
            vec![key_event(n, true), key_event(n, false)],
            "step {step}"
        );
    }
}

#[test]
fn key_slots_are_the_slot_numbers_of_the_measured_layout() {
    let row1 = layout_row("key_row_1");
    let row2 = layout_row("key_row_2");
    assert_eq!(row1, [11, 12, 13, 14, 15]);
    assert_eq!(row2, [6, 7, 8, 9, 10]);
    for (i, slot) in row1.iter().enumerate() {
        assert_eq!(key_slot(i as u8 + 0x01), *slot);
    }
    for (i, slot) in row2.iter().enumerate() {
        assert_eq!(key_slot(i as u8 + 0x06), *slot);
    }
}

#[test]
fn a_code_that_is_not_a_key_does_not_give_a_key_event() {
    // The codes 0x40 to 0x43 are taps on the bar panels, not keys. 0x00 is the
    // touch start. 0x0B is not a code of the N4 Pro.
    for code in [0x40, 0x43, 0x00, 0x0B] {
        assert!(
            !n4pro()
                .controls
                .iter()
                .any(|c| c.codes == InputCodes::Key { code }),
            "code 0x{code:02X}"
        );
        for state in [0x00, 0x01] {
            let ev = decode(&control_report(code, state, 0));
            assert!(!is_key(&ev), "code 0x{code:02X}: {ev:?}");
        }
    }
}

#[test]
fn two_keys_together_give_the_press_and_release_of_one_key() {
    // Measured: the device sent the reports of one key only.
    let rec = load("11");
    let n = key_with_code(0x06).number;
    assert_eq!(events("11"), vec![key_event(n, true), key_event(n, false)]);
    assert!(rec.reports[1].ms < 100);
}

#[test]
fn a_held_key_gives_one_press_and_one_release_and_no_repeat() {
    let rec = load("12");
    let n = key_with_code(0x06).number;
    assert_eq!(events("12"), vec![key_event(n, true), key_event(n, false)]);
    assert!(rec.reports[1].ms >= 3000, "held for 3 seconds");
}

// --- Knobs -----------------------------------------------------------------

/// Knob 1 to 4, from left to right: the steps for turn left, turn right and
/// push.
const KNOB_STEPS: [(u8, &str, &str, &str); 4] = [
    (1, "13", "14", "15"),
    (2, "16", "17", "18"),
    (3, "19", "20", "21"),
    (4, "22", "23", "24"),
];

#[test]
fn knob_turn_left_gives_one_event_for_each_click() {
    for (knob, left, _, _) in KNOB_STEPS {
        let expected = turn(knob, Left);
        assert_eq!(events(left), vec![expected; 5], "knob {knob}");
    }
}

#[test]
fn knob_turn_right_gives_one_event_for_each_click() {
    for (knob, _, right, _) in KNOB_STEPS {
        let expected = turn(knob, Right);
        assert_eq!(events(right), vec![expected; 5], "knob {knob}");
    }
}

#[test]
fn knob_push_gives_one_event_and_no_release() {
    for (knob, _, _, push) in KNOB_STEPS {
        assert_eq!(events(push), vec![self::push(knob)], "knob {knob}");
    }
}

#[test]
fn knob_codes_are_the_measured_codes() {
    // The codes of the "Controls and input codes" table, in the order
    // left, right for knobs 1 to 4, and the push codes.
    let turns = [
        (0xA0, 1, Left),
        (0xA1, 1, Right),
        (0x50, 2, Left),
        (0x51, 2, Right),
        (0x90, 3, Left),
        (0x91, 3, Right),
        (0x70, 4, Left),
        (0x71, 4, Right),
    ];
    for (code, knob, dir) in turns {
        assert_eq!(
            decode(&control_report(code, 0x00, 0)),
            turn(knob, dir),
            "code 0x{code:02X}"
        );
    }
    for (code, knob) in [(0x37, 1), (0x35, 2), (0x33, 3), (0x36, 4)] {
        assert_eq!(
            decode(&control_report(code, 0x01, 0)),
            push(knob),
            "code 0x{code:02X}"
        );
    }
}

#[test]
fn a_fast_turn_in_two_directions_gives_398_events() {
    let rec = load("25");
    let events = events("25");
    assert_eq!(events.len(), 398);
    let left = turn(1, Left);
    let right = turn(1, Right);
    assert!(events.iter().all(|e| *e == left || *e == right));
    assert!(events.contains(&left) && events.contains(&right));
    // The turn took about 9.5 seconds.
    let last = rec.reports.last().unwrap().ms;
    assert!((9000..10000).contains(&last), "last time: {last}");
}

// --- Touch bar -------------------------------------------------------------

#[test]
fn touch_bar_tap_on_each_bar_panel() {
    // Steps 26 to 29: a tap on bar panel 1 to 4. The x coordinate of the start
    // of the touch is in the "finger down" report and in the tap report.
    for (step, panel, x) in [
        ("26", 1, 101),
        ("27", 2, 290),
        ("28", 3, 510),
        ("29", 4, 679),
    ] {
        let events = events(step);
        let points = assert_one_touch(&events, x, &tap(panel, x));
        // The finger does not move: each touch report has the same point.
        assert!(
            points.iter().all(|p| p.0 == x && *p == points[0]),
            "step {step}: {points:?}"
        );
    }
}

#[test]
fn a_finger_that_stays_for_2_seconds_gives_a_tap() {
    let rec = load("32");
    let events = events("32");
    let points = assert_one_touch(&events, 8, &tap(1, 8));
    // A touch report each 35 ms for about 2 seconds.
    assert_eq!(points.len(), 89);
    assert!(points.iter().all(|p| *p == points[0]));
    assert!(rec.reports.last().unwrap().ms >= 2000);
}

#[test]
fn touch_report_has_x_in_bytes_10_and_11_and_y_in_bytes_12_and_13() {
    let r = &load("26").reports[1].bytes;
    assert_eq!(&r[4..7], b"ARX");
    assert_eq!(decode(r), touch_point(101, 60));
    assert_eq!(u16::from_be_bytes([r[10], r[11]]), 101);
    assert_eq!(u16::from_be_bytes([r[12], r[13]]), 60);
}

#[test]
fn touch_report_coordinates_are_big_endian() {
    // 800 = 0x0320 and 112 = 0x0070, 0x0102 = 258 and 0x0304 = 772.
    assert_eq!(decode(&touch_report(800, 112)), touch_point(800, 112));
    assert_eq!(decode(&touch_report(0x0102, 0x0304)), touch_point(258, 772));
    assert_eq!(decode(&touch_report(0, 0)), touch_point(0, 0));
    assert_eq!(
        decode(&touch_report(u16::MAX, u16::MAX)),
        touch_point(u16::MAX, u16::MAX)
    );
}

#[test]
fn a_slide_from_left_to_right_makes_x_grow() {
    let events = events("30");
    let points = assert_one_touch(&events, 26, &swipe(Right));
    assert_eq!(points.len(), 51);
    let xs: Vec<u16> = points.iter().map(|p| p.0).collect();
    assert!(
        xs.windows(2).all(|w| w[0] <= w[1]),
        "x does not grow: {xs:?}"
    );
    assert_eq!(xs.first(), Some(&26));
    assert_eq!(xs.last(), Some(&782));
    // The coordinates that were seen are in the touch area.
    assert!(points.iter().all(|p| p.0 < 800 && p.1 < 112));
}

#[test]
fn a_slide_from_top_to_bottom_makes_y_grow() {
    let events = events("31");
    let points = assert_one_touch(&events, 403, &touch_end());
    assert_eq!(points.len(), 17);
    let ys: Vec<u16> = points.iter().map(|p| p.1).collect();
    assert!(
        ys.windows(2).all(|w| w[0] <= w[1]),
        "y does not grow: {ys:?}"
    );
    assert_eq!(ys.first(), Some(&6));
    assert_eq!(ys.last(), Some(&109));
    assert!(points.iter().all(|p| p.0 == 403));
}

#[test]
fn swipe_right_is_code_0x38_and_the_finger_moves_to_the_right() {
    // Step 34: one swipe to the right.
    let rec = load("34");
    assert_eq!(rec.reports.last().unwrap().bytes[9], 0x38);
    let events = events("34");
    let points = assert_one_touch(&events, 51, &swipe(Right));
    assert!(points.last().unwrap().0 > points.first().unwrap().0);
}

#[test]
fn swipe_left_is_code_0x39() {
    // The 20 swipes to the left of step 35 gave 20 times 0x39. The one swipe of
    // step 33 gave 0xB2 (see the next test).
    let rec = load("35");
    let codes: Vec<u8> = rec
        .reports
        .iter()
        .filter(|r| &r.bytes[5..7] == b"OK" && r.bytes[9] != 0x00)
        .map(|r| r.bytes[9])
        .collect();
    assert_eq!(codes, vec![0x39; 20]);
}

#[test]
fn twenty_swipes_to_the_left_give_20_touches_that_end_in_a_swipe_left() {
    let events = events("35");
    assert_twenty_touches(&events, Left);
}

#[test]
fn twenty_swipes_to_the_right_give_20_touches_that_end_in_a_swipe_right() {
    let events = events("36");
    assert_twenty_touches(&events, Right);
}

/// Split the events at each touch start. Each part must be a touch that ends
/// in a swipe in the direction `dir`, and the finger must move in the
/// direction `dir`.
fn assert_twenty_touches(events: &[Decoded], dir: Direction) {
    let starts: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| is_touch_start(e))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(starts.len(), 20);
    let mut ends = starts[1..].to_vec();
    ends.push(events.len());
    for (n, (start, end)) in starts.iter().zip(ends).enumerate() {
        let touch = &events[*start..end];
        let Decoded::Input(InputEvent {
            gesture: Gesture::TouchStart { x },
            ..
        }) = touch[0]
        else {
            unreachable!()
        };
        let points = assert_one_touch(touch, x, &swipe(dir));
        // In each swipe, the last point is on the side of `dir` of the first.
        let (first, last) = (points[0].0, points.last().unwrap().0);
        match dir {
            Right => assert!(last > first, "swipe {n}: {first} to {last}"),
            Left => assert!(last < first, "swipe {n}: {first} to {last}"),
        }
    }
}

#[test]
fn a_finger_up_that_the_device_did_not_identify() {
    // Code 0xB2 came after one quick swipe to the left (step 33) and after a
    // move from the top edge to the bottom edge (step 31).
    for step in ["31", "33"] {
        let rec = load(step);
        let last = rec.reports.last().unwrap();
        assert_eq!(last.bytes[9], 0xB2, "step {step}");
        assert_eq!(decode(&last.bytes), touch_end(), "step {step}");
    }
    let events = events("33");
    let points = assert_one_touch(&events, 606, &touch_end());
    assert!(points.last().unwrap().0 < points.first().unwrap().0);
}

#[test]
fn the_touch_codes_of_the_table_decode_from_hand_made_reports() {
    // Finger down.
    assert_eq!(decode(&control_report(0x00, 0x01, 679)), touch_start(679));
    // Tap on bar panel 1 to 4.
    for (code, panel) in [(0x40, 1), (0x41, 2), (0x42, 3), (0x43, 4)] {
        assert_eq!(
            decode(&control_report(code, 0x00, 0x0123)),
            tap(panel, 0x0123)
        );
    }
    // Finger up after a move.
    assert_eq!(decode(&control_report(0x38, 0x00, 0)), swipe(Right));
    assert_eq!(decode(&control_report(0x39, 0x00, 0)), swipe(Left));
    assert_eq!(decode(&control_report(0xB2, 0x00, 0)), touch_end());
}

#[test]
fn a_touch_start_is_not_a_key_and_a_tap_is_not_a_key() {
    for step in [
        "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36",
    ] {
        assert!(!events(step).iter().any(is_key), "step {step}");
    }
}

#[test]
fn touch_start_and_tap_with_the_other_state_are_unknown() {
    // The device sends code 0x00 with the state 0x01 only, and the codes
    // 0x40 to 0x43 with the state 0x00 only.
    let r = control_report(0x00, 0x00, 5);
    assert_eq!(decode(&r), Decoded::Unknown(r));
    let r = control_report(0x41, 0x01, 5);
    assert_eq!(decode(&r), Decoded::Unknown(r));
}

#[test]
fn touch_start_and_tap_need_the_x_coordinate() {
    // A report that ends before byte 12 has no x coordinate.
    for (code, state) in [(0x00, 0x01), (0x42, 0x00)] {
        let full = control_report(code, state, 300);
        for len in [11, 12] {
            let short = full[..len].to_vec();
            assert_eq!(decode(&short), Decoded::Unknown(short.clone()), "len {len}");
        }
        assert_ne!(decode(&full[..13]), Decoded::Unknown(full[..13].to_vec()));
    }
}

// --- Acknowledgement, short report, unknown report --------------------------

#[test]
fn an_acknowledgement_is_code_0xff() {
    // No fixture has an acknowledgement. The report is made by hand: "ACK",
    // "OK", and the input code 0xFF.
    for state in [0x00, 0x01] {
        assert_eq!(decode(&control_report(0xFF, state, 0)), Decoded::Ack);
    }
    // The shortest report that the decoder accepts.
    let mut short = control_report(0xFF, 0x00, 0);
    short.truncate(11);
    assert_eq!(decode(&short), Decoded::Ack);
}

#[test]
fn a_short_report_is_unknown() {
    assert_eq!(decode(&[]), Decoded::Unknown(vec![]));
    assert_eq!(decode(b"ACK"), Decoded::Unknown(b"ACK".to_vec()));
    // One byte short of the 11 bytes that a control report needs.
    let ten = control_report(0x01, 0x01, 0)[..10].to_vec();
    assert_eq!(decode(&ten), Decoded::Unknown(ten.clone()));
    // A touch report needs 14 bytes.
    let thirteen = touch_report(100, 50)[..13].to_vec();
    assert_eq!(decode(&thirteen), Decoded::Unknown(thirteen.clone()));
    // A short report must not panic, for any length.
    let full = control_report(0x00, 0x01, 100);
    for len in 0..full.len() {
        let _ = decode(&full[..len]);
    }
}

#[test]
fn a_report_that_does_not_start_with_ack_is_unknown() {
    let mut r = control_report(0x01, 0x01, 0);
    r[0] = b'N';
    assert_eq!(decode(&r), Decoded::Unknown(r));
    // A report of zero bytes.
    let zeros = vec![0u8; 512];
    assert_eq!(decode(&zeros), Decoded::Unknown(zeros));
    // "ACK" in the wrong place.
    let mut r = report(&[0x00]);
    r[1..4].copy_from_slice(b"ACK");
    assert_eq!(decode(&r), Decoded::Unknown(r));
}

#[test]
fn a_report_with_neither_ok_nor_arx_is_unknown() {
    let mut r = control_report(0x01, 0x01, 0);
    r[5..7].copy_from_slice(b"NO");
    assert_eq!(decode(&r), Decoded::Unknown(r));
}

#[test]
fn a_report_with_an_unknown_code_is_kept_with_all_its_bytes() {
    // The decoder does not know the code, so it keeps the report. The probe
    // uses this to show codes that nobody measured yet.
    for state in [0x01, 0x00] {
        let r = control_report(0x7E, state, 0);
        assert_eq!(decode(&r), Decoded::Unknown(r));
    }
    // The device sends a key code with the state 0x01 or 0x00 only. A
    // different state is not known.
    let r = control_report(0x02, 0x05, 0);
    assert_eq!(decode(&r), Decoded::Unknown(r));
}

// --- An invented model -------------------------------------------------------
//
// The decoder gets all input codes from the descriptor. These tests use a
// model that does not exist: 6 keys in 2 rows of 3, 2 knobs, and no touch bar.
// Its key faces are JPEG images that are not turned.
// Some of its codes are codes of a different control on the N4 Pro.

const fn invented_key(number: u8, code: u8) -> Control {
    Control {
        number,
        codes: InputCodes::Key { code },
        position: Position {
            row: (number - 1) / 3 + 1,
            column: (number - 1) % 3 + 1,
        },
        surface: Some(opencraft_core::SurfaceId::new(
            opencraft_core::SurfaceKind::KeyFace,
            number,
        )),
    }
}

const fn invented_knob(number: u8, turn_left: u8, turn_right: u8, push: u8) -> Control {
    Control {
        number,
        codes: InputCodes::Knob {
            turn_left,
            turn_right,
            push,
        },
        position: Position {
            row: 1,
            column: number,
        },
        surface: None,
    }
}

static INVENTED_CONTROLS: &[Control] = &[
    // 0xA0 is "knob 1, turn left" on the N4 Pro.
    invented_key(1, 0xA0),
    invented_key(2, 0x21),
    invented_key(3, 0x22),
    invented_key(4, 0x23),
    invented_key(5, 0x24),
    // 0x40 is "tap on bar panel 1" on the N4 Pro.
    invented_key(6, 0x40),
    // 0x01 and 0x02 are keys on the N4 Pro.
    invented_knob(1, 0x01, 0x02, 0x03),
    invented_knob(2, 0x11, 0x12, 0x13),
];

/// Key face `number`: 64 x 64, JPEG, not turned.
const fn invented_face(number: u8) -> opencraft_core::Surface {
    opencraft_core::Surface {
        id: opencraft_core::SurfaceId::new(opencraft_core::SurfaceKind::KeyFace, number),
        slot: Some(number),
        width: 64,
        height: 64,
        format: opencraft_core::ImageFormat::Jpeg,
        rotation: opencraft_core::Rotation::Deg0,
    }
}

static INVENTED_SURFACES: &[opencraft_core::Surface] = &[
    invented_face(1),
    invented_face(2),
    invented_face(3),
    invented_face(4),
    invented_face(5),
    invented_face(6),
];

static INVENTED: Descriptor = Descriptor {
    name: "Invented model",
    vid: 0x1234,
    pid: 0x5678,
    packet_size: 512,
    surfaces: INVENTED_SURFACES,
    controls: INVENTED_CONTROLS,
    required_mode: None,
};

fn invented(report: &[u8]) -> Decoded {
    decode_with(&INVENTED, report)
}

#[test]
fn an_invented_model_gets_the_events_of_its_own_codes() {
    // The keys: state 0x01 is press, 0x00 is release.
    for (code, number) in [
        (0xA0, 1),
        (0x21, 2),
        (0x22, 3),
        (0x23, 4),
        (0x24, 5),
        (0x40, 6),
    ] {
        assert_eq!(
            invented(&control_report(code, 0x01, 0)),
            key_event(number, true),
            "code 0x{code:02X}"
        );
        assert_eq!(
            invented(&control_report(code, 0x00, 0)),
            key_event(number, false),
            "code 0x{code:02X}"
        );
    }
    // The knobs.
    assert_eq!(invented(&control_report(0x01, 0x00, 0)), turn(1, Left));
    assert_eq!(invented(&control_report(0x02, 0x00, 0)), turn(1, Right));
    assert_eq!(invented(&control_report(0x03, 0x01, 0)), push(1));
    assert_eq!(invented(&control_report(0x11, 0x00, 0)), turn(2, Left));
    assert_eq!(invented(&control_report(0x12, 0x00, 0)), turn(2, Right));
    assert_eq!(invented(&control_report(0x13, 0x01, 0)), push(2));
    // The acknowledgement is the protocol, not the model.
    assert_eq!(invented(&control_report(0xFF, 0x00, 0)), Decoded::Ack);
}

#[test]
fn an_invented_model_does_not_know_the_codes_of_the_n4_pro() {
    // Codes of the N4 Pro that the invented model does not have: key 10,
    // knob 4, bar panel 2, and the codes of the touch bar.
    for (code, state) in [
        (0x0A, 0x01),
        (0x70, 0x00),
        (0x36, 0x01),
        (0x41, 0x00),
        (0x00, 0x01),
        (0x38, 0x00),
        (0x39, 0x00),
        (0xB2, 0x00),
    ] {
        let r = control_report(code, state, 100);
        assert_eq!(invented(&r), Decoded::Unknown(r), "code 0x{code:02X}");
    }
    // The model has no touch bar, so a touch report is not known.
    let r = touch_report(100, 50);
    assert_eq!(invented(&r), Decoded::Unknown(r));
}

#[test]
fn the_recorded_reports_mean_different_things_for_an_invented_model() {
    // Step 13: knob 1 of the N4 Pro, turn left (code 0xA0, state 0x00). On the
    // invented model, 0xA0 is key 1, and state 0x00 is the release.
    let reports = load("13").reports;
    assert!(!reports.is_empty());
    for r in &reports {
        assert_eq!(invented(&r.bytes), key_event(1, false));
    }
    // Step 01: key 1 of the N4 Pro (code 0x01). On the invented model, 0x01 is
    // knob 1, turn left. A knob turn has the state 0x00, so the press report
    // (state 0x01) is not known and the release report is a turn.
    let events: Vec<Decoded> = load("01")
        .reports
        .iter()
        .map(|r| invented(&r.bytes))
        .collect();
    assert!(matches!(events[0], Decoded::Unknown(_)), "{events:?}");
    assert_eq!(events[1], turn(1, Left));
}

#[test]
fn an_invented_touch_bar_gets_the_touch_events_of_its_own_codes() {
    static CONTROLS: &[Control] = &[
        Control {
            number: 1,
            codes: InputCodes::BarPanel { tap: 0x61 },
            position: Position { row: 1, column: 1 },
            surface: None,
        },
        Control {
            number: 2,
            codes: InputCodes::BarPanel { tap: 0x62 },
            position: Position { row: 1, column: 2 },
            surface: None,
        },
        Control {
            number: 1,
            codes: InputCodes::TouchBar {
                touch_start: 0x70,
                swipe_right: 0x71,
                swipe_left: 0x72,
                touch_end: 0x73,
            },
            position: Position { row: 1, column: 1 },
            surface: None,
        },
    ];
    let d = Descriptor {
        controls: CONTROLS,
        ..INVENTED
    };
    let dec = |r: &[u8]| decode_with(&d, r);
    assert_eq!(dec(&control_report(0x70, 0x01, 300)), touch_start(300));
    assert_eq!(dec(&control_report(0x61, 0x00, 20)), tap(1, 20));
    assert_eq!(dec(&control_report(0x62, 0x00, 420)), tap(2, 420));
    assert_eq!(dec(&control_report(0x71, 0x00, 0)), swipe(Right));
    assert_eq!(dec(&control_report(0x72, 0x00, 0)), swipe(Left));
    assert_eq!(dec(&control_report(0x73, 0x00, 0)), touch_end());
    assert_eq!(dec(&touch_report(7, 8)), touch_point(7, 8));
    // The codes of the touch bar of the N4 Pro are not known here.
    for (code, state) in [(0x00, 0x01), (0x40, 0x00), (0x38, 0x00), (0xB2, 0x00)] {
        let r = control_report(code, state, 5);
        assert_eq!(dec(&r), Decoded::Unknown(r), "code 0x{code:02X}");
    }
}
