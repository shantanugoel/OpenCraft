//! Tests for the descriptor table.
//!
//! The first tests check the rules that each descriptor must obey. The other
//! tests check the N4 Pro entry against the measured facts in
//! `docs/kb/n4-pro-hardware.md` and in `tests/fixtures/input/layout.txt`.

use opencraft_core::device::{self, DESCRIPTORS};
use opencraft_core::{ControlKind, Descriptor, ImageFormat, Rotation, SurfaceKind};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

fn n4pro() -> &'static Descriptor {
    device::find(0x5548, 0x1023).unwrap()
}

// --- Rules for each descriptor ------------------------------------------------

#[test]
fn each_usb_identity_selects_one_descriptor() {
    let mut identities = HashSet::new();
    for d in DESCRIPTORS {
        assert!(
            identities.insert((d.vid, d.pid)),
            "duplicate USB identity: {}",
            d.name
        );
        assert!(
            std::ptr::eq(device::find(d.vid, d.pid).unwrap(), d),
            "{}",
            d.name
        );
        assert!(
            device::find(0xFFFF, d.pid).is_none(),
            "vendor is part of the identity"
        );
        assert!(
            device::find(d.vid, 0xFFFF).is_none(),
            "product is part of the identity"
        );
    }
}

#[test]
fn no_two_controls_share_an_input_code() {
    // All codes are in control reports. The touch reports have no code.
    for d in DESCRIPTORS {
        let mut owner = HashMap::new();
        for c in d.controls {
            for code in c.codes.all() {
                if let Some(other) = owner.insert(code, c.id()) {
                    panic!(
                        "{}: code 0x{code:02X} is used by {other} and by {}",
                        d.name,
                        c.id()
                    );
                }
            }
        }
        // 0xFF is the acknowledgement of the protocol.
        assert!(!owner.contains_key(&0xFF), "{}", d.name);
    }
}

#[test]
fn each_control_and_each_surface_has_one_identity() {
    for d in DESCRIPTORS {
        let controls: HashSet<_> = d.controls.iter().map(|c| c.id()).collect();
        assert_eq!(controls.len(), d.controls.len(), "{}", d.name);
        let surfaces: HashSet<_> = d.surfaces.iter().map(|s| s.id).collect();
        assert_eq!(surfaces.len(), d.surfaces.len(), "{}", d.name);

        // The numbers of each kind are 1, 2, 3, and so on.
        for kind in [
            ControlKind::Key,
            ControlKind::Knob,
            ControlKind::BarPanel,
            ControlKind::TouchBar,
        ] {
            let mut numbers: Vec<u8> = d.controls_of(kind).map(|c| c.number).collect();
            numbers.sort();
            let expected: Vec<u8> = (1..=numbers.len() as u8).collect();
            assert_eq!(numbers, expected, "{}: {kind:?}", d.name);
        }
        for kind in [
            SurfaceKind::KeyFace,
            SurfaceKind::BarPanel,
            SurfaceKind::Background,
        ] {
            let mut numbers: Vec<u8> = d
                .surfaces
                .iter()
                .filter(|s| s.kind() == kind)
                .map(|s| s.id.number)
                .collect();
            numbers.sort();
            let expected: Vec<u8> = (1..=numbers.len() as u8).collect();
            assert_eq!(numbers, expected, "{}: {kind:?}", d.name);
        }
    }
}

#[test]
fn each_surface_of_a_control_exists_and_has_the_correct_kind() {
    for d in DESCRIPTORS {
        let mut used = HashSet::new();
        for c in d.controls {
            let expected = match c.kind() {
                ControlKind::Key => Some(SurfaceKind::KeyFace),
                ControlKind::BarPanel => Some(SurfaceKind::BarPanel),
                ControlKind::Knob | ControlKind::TouchBar => None,
            };
            let Some(id) = c.surface else {
                continue;
            };
            let surface = d.surface(id).unwrap_or_else(|| {
                panic!("{}: {} refers to {id}, which is missing", d.name, c.id())
            });
            assert_eq!(Some(surface.kind()), expected, "{}: {}", d.name, c.id());
            assert!(used.insert(id), "{}: two controls refer to {id}", d.name);
        }
    }
}

#[test]
fn each_slot_is_used_one_time_and_only_the_background_has_no_slot() {
    for d in DESCRIPTORS {
        let mut slots = HashSet::new();
        for s in d.surfaces {
            assert!(s.width > 0 && s.height > 0, "{}: {}", d.name, s.id);
            match s.slot {
                Some(slot) => {
                    assert!(
                        slots.insert(slot),
                        "{}: slot {slot} is used 2 times",
                        d.name
                    );
                    // The clear command uses 0xFF for all slots.
                    assert_ne!(slot, 0xFF, "{}: {}", d.name, s.id);
                    assert_ne!(s.kind(), SurfaceKind::Background, "{}", d.name);
                }
                None => assert_eq!(s.kind(), SurfaceKind::Background, "{}: {}", d.name, s.id),
            }
        }
        let backgrounds = d
            .surfaces
            .iter()
            .filter(|s| s.kind() == SurfaceKind::Background)
            .count();
        assert!(backgrounds <= 1, "{}", d.name);
    }
}

#[test]
fn each_surface_is_on_the_display_and_no_two_slot_surfaces_overlap() {
    for d in DESCRIPTORS {
        let (width, height) = d.display_size();
        if let Some(bg) = d.background() {
            assert_eq!((bg.x, bg.y), (0, 0), "{}", d.name);
            assert_eq!((bg.width, bg.height), (width, height), "{}", d.name);
        }
        let slots: Vec<_> = d.slot_surfaces().collect();
        for (i, a) in slots.iter().enumerate() {
            assert!(
                a.x + a.width <= width && a.y + a.height <= height,
                "{}: {} is not on the display",
                d.name,
                a.id
            );
            for b in &slots[i + 1..] {
                let apart = a.x + a.width <= b.x
                    || b.x + b.width <= a.x
                    || a.y + a.height <= b.y
                    || b.y + b.height <= a.y;
                assert!(apart, "{}: {} and {} overlap", d.name, a.id, b.id);
            }
        }
    }
}

#[test]
fn the_position_of_a_surface_agrees_with_the_position_of_its_control() {
    // A control more to the right has a surface more to the right. A control
    // in a lower row has a surface that is lower.
    for d in DESCRIPTORS {
        let with_surface: Vec<_> = d
            .controls
            .iter()
            .filter_map(|c| Some((c, d.surface(c.surface?).unwrap())))
            .collect();
        for (a, sa) in &with_surface {
            for (b, sb) in &with_surface {
                if a.kind() != b.kind() {
                    continue;
                }
                if a.position.row == b.position.row && a.position.column < b.position.column {
                    assert!(sa.x < sb.x, "{}: {} and {}", d.name, a.id(), b.id());
                    assert_eq!(sa.y, sb.y, "{}: {} and {}", d.name, a.id(), b.id());
                }
                if a.position.row < b.position.row {
                    assert!(sa.y < sb.y, "{}: {} and {}", d.name, a.id(), b.id());
                }
            }
        }
    }
}

#[test]
fn the_n4_pro_surfaces_are_at_the_measured_positions() {
    // "Positions on the display" in docs/kb/n4-pro-hardware.md.
    let d = n4pro();
    assert_eq!(d.display_size(), (800, 480));
    let at = |slot: u8| {
        let s = d.surface_at_slot(slot).unwrap();
        (s.x, s.y)
    };
    let key_x = [6, 176, 346, 514, 682];
    for (i, x) in key_x.into_iter().enumerate() {
        assert_eq!(at(11 + i as u8), (x, 8), "top row");
        assert_eq!(at(6 + i as u8), (x, 178), "second row");
    }
    for (i, x) in [0, 203, 406, 609].into_iter().enumerate() {
        assert_eq!(at(1 + i as u8), (x, 358), "bar panel");
    }
}

#[test]
fn no_two_controls_of_one_kind_have_the_same_position() {
    for d in DESCRIPTORS {
        let mut positions = HashSet::new();
        for c in d.controls {
            assert!(c.position.row >= 1 && c.position.column >= 1, "{}", c.id());
            assert!(
                positions.insert((c.kind(), c.position)),
                "{}: {} has the position of a different control",
                d.name,
                c.id()
            );
        }
    }
}

// --- The N4 Pro ---------------------------------------------------------------

/// Read a row of `layout.txt`: the slot numbers, from left to right.
fn layout_row(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/input/layout.txt");
    let text = std::fs::read_to_string(path).unwrap();
    let prefix = format!("{name}:");
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("no {name} in layout.txt"));
    line.split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect()
}

/// The slots of the surfaces of the controls of `kind` in `row`, from left to
/// right.
fn slots_in_row(d: &Descriptor, kind: ControlKind, row: u8) -> Vec<u8> {
    let mut controls: Vec<_> = d
        .controls_of(kind)
        .filter(|c| c.position.row == row)
        .collect();
    controls.sort_by_key(|c| c.position.column);
    controls
        .iter()
        .map(|c| d.surface(c.surface.unwrap()).unwrap().slot.unwrap())
        .collect()
}

#[test]
fn the_n4_pro_layout_agrees_with_layout_txt() {
    let d = n4pro();
    assert_eq!(
        slots_in_row(d, ControlKind::Key, 1),
        layout_row("key_row_1")
    );
    assert_eq!(
        slots_in_row(d, ControlKind::Key, 2),
        layout_row("key_row_2")
    );
    assert_eq!(
        slots_in_row(d, ControlKind::BarPanel, 1),
        layout_row("bar_panels")
    );
    // 2 rows of 5 keys. Knob 1 to 4 are the knobs from left to right.
    assert!(d.controls_of(ControlKind::Key).all(|c| c.position.row <= 2));
    for c in d.controls_of(ControlKind::Knob) {
        assert_eq!((c.position.row, c.position.column), (1, c.number));
    }
}

#[test]
fn the_n4_pro_has_the_measured_controls_and_surfaces() {
    let d = n4pro();
    let count = |kind| d.controls_of(kind).count();
    assert_eq!(count(ControlKind::Key), 10);
    assert_eq!(count(ControlKind::Knob), 4);
    assert_eq!(count(ControlKind::BarPanel), 4);
    assert_eq!(count(ControlKind::TouchBar), 1);
    assert_eq!(d.controls.len(), 19);

    // Key face: 112 x 112. Bar panel: 176 x 112. Background: 800 x 480. All
    // are JPEG (decision 0007), and all are turned 180°.
    let surfaces = |kind| d.surfaces.iter().filter(move |s| s.kind() == kind);
    for (kind, n, size, format) in [
        (SurfaceKind::KeyFace, 10, (112, 112), ImageFormat::Jpeg),
        (SurfaceKind::BarPanel, 4, (176, 112), ImageFormat::Jpeg),
        (SurfaceKind::Background, 1, (800, 480), ImageFormat::Jpeg),
    ] {
        assert_eq!(surfaces(kind).count(), n, "{kind:?}");
        for s in surfaces(kind) {
            assert_eq!((s.width, s.height), size, "{}", s.id);
            assert_eq!(s.format, format, "{}", s.id);
            assert_eq!(s.rotation, Rotation::Deg180, "{}", s.id);
        }
    }
    // Slot 5 is not used.
    assert!(d.surface_at_slot(5).is_none());
    let mut slots: Vec<u8> = d.slot_surfaces().map(|s| s.slot.unwrap()).collect();
    slots.sort();
    assert_eq!(slots, [1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    assert!(d.background().unwrap().slot.is_none());
}

#[test]
fn the_three_n4_pro_entries_have_the_same_controls_and_surfaces() {
    let d = n4pro();
    for pid in [0x1008, 0x1021] {
        let other = device::find(0x5548, pid).unwrap();
        assert_eq!(other.controls, d.controls);
        assert_eq!(other.surfaces, d.surfaces);
    }
}

/// The decoder must not contain an input code of a model. The codes are in
/// the descriptor table only (architecture rule 4).
#[test]
fn the_decoder_source_has_no_input_code_of_the_n4_pro() {
    let source = include_str!("../src/events.rs").to_lowercase();
    for c in n4pro().controls {
        for code in c.codes.all() {
            // 0x00 and 0x01 are also the state values of the protocol.
            if code <= 0x01 {
                continue;
            }
            let literal = format!("0x{code:02x}");
            assert!(
                !source.contains(&literal),
                "events.rs contains {literal}, a code of {}",
                c.id()
            );
        }
    }
}

/// The device shows no slot image that is too large, and it gives no error.
/// The largest slot image that our device showed had 58 187 bytes, and an
/// image of 60 969 bytes did not show (`docs/kb/streamdock-protocol.md`,
/// "Image transfer"). Decision 0007 uses a JPEG quality that keeps hard images
/// below that size. The test encodes the two hardest images that we found for
/// each key face and each bar panel: noise, and noise of saturated colours. It
/// is not a proof for all images. It fails if the quality goes to 100.
#[test]
fn hard_slot_images_stay_below_the_largest_size_that_the_device_showed() {
    const LARGEST_SHOWN: usize = 58_187;
    let d = device::find(0x5548, 0x1023).unwrap();
    for s in d.slot_surfaces() {
        for saturated in [false, true] {
            // Noise from a fixed linear congruential generator.
            let mut state = 0x2545_F491u32;
            let mut img = image::RgbImage::new(s.width as u32, s.height as u32);
            for p in img.pixels_mut() {
                for c in p.0.iter_mut() {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let value = (state >> 24) as u8;
                    *c = match saturated {
                        true if value >= 128 => 255,
                        true => 0,
                        false => value,
                    };
                }
            }
            let quality = opencraft_core::image::jpeg_quality_for(s);
            let data = opencraft_core::image::encode_for(s, &img, quality).unwrap();
            assert!(
                data.len() < LARGEST_SHOWN,
                "{}: {} bytes at quality {quality} (saturated: {saturated})",
                s.id,
                data.len()
            );
            // The check has a meaning only if the image is a hard one.
            assert!(data.len() > 20_000, "{}: {} bytes", s.id, data.len());
        }
    }
}
