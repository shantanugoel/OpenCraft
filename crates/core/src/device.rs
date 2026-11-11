//! Device descriptors.
//!
//! One table entry per VID:PID. Everything above this layer works in terms of
//! *slots* (addressable displays) and *codes* (raw input bytes from the device).

/// What a display slot physically is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    /// A key face (112x112 on the N4 Pro family).
    Key,
    /// The small display beside a knob (176x112 on the N4 Pro family).
    Knob,
}

#[derive(Debug, Clone, Copy)]
pub struct Panel {
    /// Slot number as understood by the firmware (1-based, sent as-is).
    pub slot: u8,
    pub width: u16,
    pub height: u16,
    pub kind: PanelKind,
}

/// The background: one image behind the key faces. The area where it shows
/// is not measured.
///
/// The SDK sends the background as a JPEG, turned 180° as the other surfaces.
/// See `docs/kb/n4-pro-hardware.md`, section "Surfaces".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Background {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct Descriptor {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// Output report payload size. 1024 = protocol v3 (N4 Pro family).
    pub packet_size: usize,
    pub panels: &'static [Panel],
    /// The background, if the model has one.
    pub background: Option<Background>,
    /// Some rebrands need an explicit mode switch before they accept commands.
    pub required_mode: Option<u8>,
}

impl Descriptor {
    pub fn panel(&self, slot: u8) -> Option<&'static Panel> {
        self.panels.iter().find(|p| p.slot == slot)
    }
}

const fn key(slot: u8) -> Panel {
    Panel {
        slot,
        width: 112,
        height: 112,
        kind: PanelKind::Key,
    }
}

const fn knob(slot: u8) -> Panel {
    Panel {
        slot,
        width: 176,
        height: 112,
        kind: PanelKind::Knob,
    }
}

/// N4 Pro: 10 keys (slots 6..=15) + 4 knob displays (slots 1..=4).
/// Slot 5 is deliberately unused by the vendor firmware.
static N4PRO_PANELS: &[Panel] = &[
    knob(1),
    knob(2),
    knob(3),
    knob(4),
    key(6),
    key(7),
    key(8),
    key(9),
    key(10),
    key(11),
    key(12),
    key(13),
    key(14),
    key(15),
];

/// N4 Pro: the 800 x 480 background (Source: SDK, not measured).
const N4PRO_BACKGROUND: Option<Background> = Some(Background {
    width: 800,
    height: 480,
});

pub static DESCRIPTORS: &[Descriptor] = &[
    Descriptor {
        name: "VSD StreamDock N4 Pro",
        vid: 0x5548,
        pid: 0x1023,
        packet_size: 1024,
        panels: N4PRO_PANELS,
        background: N4PRO_BACKGROUND,
        required_mode: None,
    },
    Descriptor {
        name: "StreamDock N4 Pro (0x1008)",
        vid: 0x5548,
        pid: 0x1008,
        packet_size: 1024,
        panels: N4PRO_PANELS,
        background: N4PRO_BACKGROUND,
        required_mode: None,
    },
    Descriptor {
        name: "StreamDock N4 Pro (0x1021)",
        vid: 0x5548,
        pid: 0x1021,
        packet_size: 1024,
        panels: N4PRO_PANELS,
        background: N4PRO_BACKGROUND,
        required_mode: None,
    },
];

/// Vendors known to ship StreamDock-compatible hardware.
pub const KNOWN_VIDS: &[u16] = &[
    0x5548, 0x6602, 0x6603, 0x1500, 0x0300, 0xEEEF, 0x0B00, 0x0200,
];

/// Usage pages the vendor control interface can advertise.
///
/// Measured on the N4 Pro: 0xFFA0. Others are seen across the family, so accept
/// the known set rather than a single value.
pub const VENDOR_USAGE_PAGES: &[u16] = &[0xFFA0, 0xFF00, 0xFF60];

pub fn find(vid: u16, pid: u16) -> Option<&'static Descriptor> {
    DESCRIPTORS.iter().find(|d| d.vid == vid && d.pid == pid)
}
