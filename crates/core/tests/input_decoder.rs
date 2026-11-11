//! Tests for the input decoder, with the recorded reports of the VSD N4 Pro.
//!
//! The fixtures are in `tests/fixtures/input/`. Each file holds the reports of
//! one gesture. The facts that the tests check are in
//! `docs/kb/n4-pro-hardware.md` (section "Controls and input codes") and in
//! `docs/kb/streamdock-protocol.md` (section "Input reports"). The tests need
//! no device.

use opencraft_core::events::key_label;
use opencraft_core::recording::{self, Recording};
use opencraft_core::{decode, Direction, Event};
use std::path::PathBuf;

use Direction::{Left, Right};

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
fn events(step: &str) -> Vec<Event> {
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

fn touch_points(events: &[Event]) -> Vec<(u16, u16)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Touch { x, y } => Some((*x, *y)),
            _ => None,
        })
        .collect()
}

/// Check that the events of one touch are: a `TouchStart` with `start_x`, one
/// or more touch points, and `end`. Return the touch points.
fn assert_one_touch(events: &[Event], start_x: u16, end: &Event) -> Vec<(u16, u16)> {
    assert_eq!(events.first(), Some(&Event::TouchStart { x: start_x }));
    assert_eq!(events.last(), Some(end));
    let middle = &events[1..events.len() - 1];
    assert!(!middle.is_empty(), "no touch report");
    assert!(
        middle.iter().all(|e| matches!(e, Event::Touch { .. })),
        "a report other than a touch report between the start and the end: {middle:?}"
    );
    touch_points(middle)
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
                !matches!(ev, Event::Unknown(_) | Event::Ack),
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

#[test]
fn key_press_and_release_top_row() {
    // Codes 0x01 to 0x05, from left to right. Steps 01 to 05.
    for (i, step) in ["01", "02", "03", "04", "05"].iter().enumerate() {
        let code = i as u8 + 1;
        assert_eq!(
            events(step),
            vec![
                Event::Key {
                    code,
                    pressed: true
                },
                Event::Key {
                    code,
                    pressed: false
                },
            ],
            "step {step}"
        );
    }
}

#[test]
fn key_press_and_release_second_row() {
    // Codes 0x06 to 0x0A, from left to right. Steps 06 to 10.
    for (i, step) in ["06", "07", "08", "09", "10"].iter().enumerate() {
        let code = i as u8 + 6;
        assert_eq!(
            events(step),
            vec![
                Event::Key {
                    code,
                    pressed: true
                },
                Event::Key {
                    code,
                    pressed: false
                },
            ],
            "step {step}"
        );
    }
}

#[test]
fn key_labels_are_the_slot_numbers_of_the_measured_layout() {
    let row1 = layout_row("key_row_1");
    let row2 = layout_row("key_row_2");
    assert_eq!(row1, [11, 12, 13, 14, 15]);
    assert_eq!(row2, [6, 7, 8, 9, 10]);
    for (i, slot) in row1.iter().enumerate() {
        assert_eq!(key_label(i as u8 + 0x01), format!("key {slot}"));
    }
    for (i, slot) in row2.iter().enumerate() {
        assert_eq!(key_label(i as u8 + 0x06), format!("key {slot}"));
    }
}

#[test]
fn key_label_for_a_code_that_is_not_a_key() {
    // The codes 0x40 to 0x43 are taps on the bar panels, not keys.
    assert_eq!(key_label(0x40), "code 0x40");
    assert_eq!(key_label(0x43), "code 0x43");
    assert_eq!(key_label(0x00), "code 0x00");
    assert_eq!(key_label(0x0B), "code 0x0B");
}

#[test]
fn two_keys_together_give_the_press_and_release_of_one_key() {
    // Measured: the device sent the reports of one key only.
    let rec = load("11");
    assert_eq!(
        events("11"),
        vec![
            Event::Key {
                code: 0x06,
                pressed: true
            },
            Event::Key {
                code: 0x06,
                pressed: false
            },
        ]
    );
    assert!(rec.reports[1].ms < 100);
}

#[test]
fn a_held_key_gives_one_press_and_one_release_and_no_repeat() {
    let rec = load("12");
    assert_eq!(
        events("12"),
        vec![
            Event::Key {
                code: 0x06,
                pressed: true
            },
            Event::Key {
                code: 0x06,
                pressed: false
            },
        ]
    );
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
        let expected = Event::KnobRotate { knob, dir: Left };
        assert_eq!(events(left), vec![expected; 5], "knob {knob}");
    }
}

#[test]
fn knob_turn_right_gives_one_event_for_each_click() {
    for (knob, _, right, _) in KNOB_STEPS {
        let expected = Event::KnobRotate { knob, dir: Right };
        assert_eq!(events(right), vec![expected; 5], "knob {knob}");
    }
}

#[test]
fn knob_push_gives_one_event_and_no_release() {
    for (knob, _, _, push) in KNOB_STEPS {
        assert_eq!(
            events(push),
            vec![Event::KnobPress {
                knob,
                pressed: true
            }],
            "knob {knob}"
        );
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
            Event::KnobRotate { knob, dir },
            "code 0x{code:02X}"
        );
    }
    for (code, knob) in [(0x37, 1), (0x35, 2), (0x33, 3), (0x36, 4)] {
        assert_eq!(
            decode(&control_report(code, 0x01, 0)),
            Event::KnobPress {
                knob,
                pressed: true
            },
            "code 0x{code:02X}"
        );
    }
}

#[test]
fn a_fast_turn_in_two_directions_gives_398_events() {
    let rec = load("25");
    let events = events("25");
    assert_eq!(events.len(), 398);
    let left = Event::KnobRotate { knob: 1, dir: Left };
    let right = Event::KnobRotate {
        knob: 1,
        dir: Right,
    };
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
        let points = assert_one_touch(&events, x, &Event::Tap { panel, x });
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
    let points = assert_one_touch(&events, 8, &Event::Tap { panel: 1, x: 8 });
    // A touch report each 35 ms for about 2 seconds.
    assert_eq!(points.len(), 89);
    assert!(points.iter().all(|p| *p == points[0]));
    assert!(rec.reports.last().unwrap().ms >= 2000);
}

#[test]
fn touch_report_has_x_in_bytes_10_and_11_and_y_in_bytes_12_and_13() {
    let r = &load("26").reports[1].bytes;
    assert_eq!(&r[4..7], b"ARX");
    assert_eq!(decode(r), Event::Touch { x: 101, y: 60 });
    assert_eq!(u16::from_be_bytes([r[10], r[11]]), 101);
    assert_eq!(u16::from_be_bytes([r[12], r[13]]), 60);
}

#[test]
fn touch_report_coordinates_are_big_endian() {
    // 800 = 0x0320 and 112 = 0x0070, 0x0102 = 258 and 0x0304 = 772.
    assert_eq!(
        decode(&touch_report(800, 112)),
        Event::Touch { x: 800, y: 112 }
    );
    assert_eq!(
        decode(&touch_report(0x0102, 0x0304)),
        Event::Touch { x: 258, y: 772 }
    );
    assert_eq!(decode(&touch_report(0, 0)), Event::Touch { x: 0, y: 0 });
    assert_eq!(
        decode(&touch_report(u16::MAX, u16::MAX)),
        Event::Touch {
            x: u16::MAX,
            y: u16::MAX
        }
    );
}

#[test]
fn a_slide_from_left_to_right_makes_x_grow() {
    let events = events("30");
    let points = assert_one_touch(&events, 26, &Event::Swipe { dir: Right });
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
    let points = assert_one_touch(&events, 403, &Event::TouchEnd);
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
    let points = assert_one_touch(&events, 51, &Event::Swipe { dir: Right });
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

/// Split the events at each `TouchStart`. Each part must be a touch that ends
/// in `Swipe { dir }`, and the finger must move in the direction `dir`.
fn assert_twenty_touches(events: &[Event], dir: Direction) {
    let starts: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Event::TouchStart { .. }))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(starts.len(), 20);
    let mut ends = starts[1..].to_vec();
    ends.push(events.len());
    for (n, (start, end)) in starts.iter().zip(ends).enumerate() {
        let touch = &events[*start..end];
        let Event::TouchStart { x } = touch[0] else {
            unreachable!()
        };
        let points = assert_one_touch(touch, x, &Event::Swipe { dir });
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
        assert_eq!(decode(&last.bytes), Event::TouchEnd, "step {step}");
    }
    let events = events("33");
    let points = assert_one_touch(&events, 606, &Event::TouchEnd);
    assert!(points.last().unwrap().0 < points.first().unwrap().0);
}

#[test]
fn the_touch_codes_of_the_table_decode_from_hand_made_reports() {
    // Finger down.
    assert_eq!(
        decode(&control_report(0x00, 0x01, 679)),
        Event::TouchStart { x: 679 }
    );
    // Tap on bar panel 1 to 4.
    for (code, panel) in [(0x40, 1), (0x41, 2), (0x42, 3), (0x43, 4)] {
        assert_eq!(
            decode(&control_report(code, 0x00, 0x0123)),
            Event::Tap { panel, x: 0x0123 }
        );
    }
    // Finger up after a move.
    assert_eq!(
        decode(&control_report(0x38, 0x00, 0)),
        Event::Swipe { dir: Right }
    );
    assert_eq!(
        decode(&control_report(0x39, 0x00, 0)),
        Event::Swipe { dir: Left }
    );
    assert_eq!(decode(&control_report(0xB2, 0x00, 0)), Event::TouchEnd);
}

#[test]
fn a_touch_start_is_not_a_key_and_a_tap_is_not_a_key() {
    for step in [
        "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36",
    ] {
        assert!(
            !events(step).iter().any(|e| matches!(e, Event::Key { .. })),
            "step {step}"
        );
    }
}

#[test]
fn touch_start_and_tap_with_the_other_state_are_unknown() {
    // The device sends code 0x00 with the state 0x01 only, and the codes
    // 0x40 to 0x43 with the state 0x00 only.
    let r = control_report(0x00, 0x00, 5);
    assert_eq!(decode(&r), Event::Unknown(r));
    let r = control_report(0x41, 0x01, 5);
    assert_eq!(decode(&r), Event::Unknown(r));
}

#[test]
fn touch_start_and_tap_need_the_x_coordinate() {
    // A report that ends before byte 12 has no x coordinate.
    for (code, state) in [(0x00, 0x01), (0x42, 0x00)] {
        let full = control_report(code, state, 300);
        for len in [11, 12] {
            let short = full[..len].to_vec();
            assert_eq!(decode(&short), Event::Unknown(short.clone()), "len {len}");
        }
        assert_ne!(decode(&full[..13]), Event::Unknown(full[..13].to_vec()));
    }
}

// --- Acknowledgement, short report, unknown report --------------------------

#[test]
fn an_acknowledgement_is_code_0xff() {
    // No fixture has an acknowledgement. The report is made by hand: "ACK",
    // "OK", and the input code 0xFF.
    for state in [0x00, 0x01] {
        assert_eq!(decode(&control_report(0xFF, state, 0)), Event::Ack);
    }
    // The shortest report that the decoder accepts.
    let mut short = control_report(0xFF, 0x00, 0);
    short.truncate(11);
    assert_eq!(decode(&short), Event::Ack);
}

#[test]
fn a_short_report_is_unknown() {
    assert_eq!(decode(&[]), Event::Unknown(vec![]));
    assert_eq!(decode(b"ACK"), Event::Unknown(b"ACK".to_vec()));
    // One byte short of the 11 bytes that a control report needs.
    let ten = control_report(0x01, 0x01, 0)[..10].to_vec();
    assert_eq!(decode(&ten), Event::Unknown(ten.clone()));
    // A touch report needs 14 bytes.
    let thirteen = touch_report(100, 50)[..13].to_vec();
    assert_eq!(decode(&thirteen), Event::Unknown(thirteen.clone()));
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
    assert_eq!(decode(&r), Event::Unknown(r));
    // A report of zero bytes.
    let zeros = vec![0u8; 512];
    assert_eq!(decode(&zeros), Event::Unknown(zeros));
    // "ACK" in the wrong place.
    let mut r = report(&[0x00]);
    r[1..4].copy_from_slice(b"ACK");
    assert_eq!(decode(&r), Event::Unknown(r));
}

#[test]
fn a_report_with_neither_ok_nor_arx_is_unknown() {
    let mut r = control_report(0x01, 0x01, 0);
    r[5..7].copy_from_slice(b"NO");
    assert_eq!(decode(&r), Event::Unknown(r));
}

#[test]
fn a_report_with_an_unknown_code_is_a_key_event_with_that_code() {
    // The decoder does not know the code, so it shows the raw code. The probe
    // uses this to show codes that nobody measured yet.
    assert_eq!(
        decode(&control_report(0x7E, 0x01, 0)),
        Event::Key {
            code: 0x7E,
            pressed: true
        }
    );
    assert_eq!(
        decode(&control_report(0x7E, 0x00, 0)),
        Event::Key {
            code: 0x7E,
            pressed: false
        }
    );
    // A state other than 0x01 is "not pressed".
    assert_eq!(
        decode(&control_report(0x02, 0x05, 0)),
        Event::Key {
            code: 0x02,
            pressed: false
        }
    );
}
