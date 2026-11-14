//! Device descriptors: the static data that describes one device model.
//!
//! The descriptor table is the only place in the device layer that knows a
//! specific model (architecture rule 4). A new model is a new table entry.
//! The procedure is in `docs/ADDING-A-DEVICE-MODEL.md`.
//!
//! A descriptor lists:
//!
//! - the **surfaces**: the areas on which the application can draw (key face,
//!   bar panel, background), with the slot, the size, the image format, and
//!   the rotation of each;
//! - the **controls**: the parts that the user operates (key, knob, bar panel,
//!   touch bar), with the input code of each gesture, the place of the control,
//!   the surface of the control, and its feedback surface.
//!
//! The format of a report is the protocol, not the model. It is in
//! [`crate::events`]. Only the codes are model data.
//!
//! The facts of the N4 Pro are in `docs/kb/n4-pro-hardware.md`.

use crate::protocol;
use std::fmt;

// --- Surfaces ----------------------------------------------------------------

/// The kind of a surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SurfaceKind {
    /// The display of a key.
    KeyFace,
    /// One part of the display of the touch bar.
    BarPanel,
    /// The full display behind the key faces and the bar panels.
    Background,
}

impl SurfaceKind {
    /// The name of the kind, in the terms of the architecture.
    pub fn name(self) -> &'static str {
        match self {
            SurfaceKind::KeyFace => "key face",
            SurfaceKind::BarPanel => "bar panel",
            SurfaceKind::Background => "background",
        }
    }
}

/// The identity of a surface: its kind and its number. The numbers of each
/// kind start at 1. The identity does not change when the table order
/// changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceId {
    pub kind: SurfaceKind,
    pub number: u8,
}

impl SurfaceId {
    pub const fn new(kind: SurfaceKind, number: u8) -> Self {
        Self { kind, number }
    }
}

impl fmt::Display for SurfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind.name(), self.number)
    }
}

/// The image format that the device layer sends to a surface. See
/// `docs/decisions/0007-jpeg-for-key-faces-and-bar-panels.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Png,
    Jpeg,
}

impl ImageFormat {
    pub fn name(self) -> &'static str {
        match self {
            ImageFormat::Png => "PNG",
            ImageFormat::Jpeg => "JPEG",
        }
    }
}

/// The turn that the device needs: the device layer turns an upright image by
/// this angle before it sends the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rotation {
    Deg0,
    Deg180,
}

impl Rotation {
    pub fn degrees(self) -> u16 {
        match self {
            Rotation::Deg0 => 0,
            Rotation::Deg180 => 180,
        }
    }
}

/// An area of the device on which the application can draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surface {
    pub id: SurfaceId,
    /// The slot number that the firmware uses for the surface. A background
    /// has no slot: it has a different command.
    pub slot: Option<u8>,
    /// The position of the top left corner of the image, in pixels of the
    /// upright display. The background is at 0, 0. See
    /// [`Descriptor::display_size`].
    pub x: u16,
    pub y: u16,
    /// The size of the image, in pixels.
    pub width: u16,
    pub height: u16,
    pub format: ImageFormat,
    pub rotation: Rotation,
}

impl Surface {
    pub fn kind(&self) -> SurfaceKind {
        self.id.kind
    }
}

// --- Controls ----------------------------------------------------------------

/// The kind of a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ControlKind {
    Key,
    Knob,
    /// The part of the touch bar that a tap identifies. A bar panel control
    /// has a bar panel surface.
    BarPanel,
    TouchBar,
}

impl ControlKind {
    /// The name of the kind, in the terms of the architecture.
    pub fn name(self) -> &'static str {
        match self {
            ControlKind::Key => "key",
            ControlKind::Knob => "knob",
            ControlKind::BarPanel => "bar panel",
            ControlKind::TouchBar => "touch bar",
        }
    }
}

/// The identity of a control: its kind and its number. The numbers of each
/// kind start at 1.
///
/// The engine finds a binding by the control identity and the gesture. The
/// identity is 2 bytes, so it is cheap to copy, to compare, and to use as a
/// key in a map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ControlId {
    pub kind: ControlKind,
    pub number: u8,
}

impl ControlId {
    pub const fn new(kind: ControlKind, number: u8) -> Self {
        Self { kind, number }
    }
}

impl fmt::Display for ControlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind.name(), self.number)
    }
}

/// The input codes of a control: one code for each gesture. The device sends
/// the code at byte 9 of a control report. The meaning of the state byte is
/// the protocol. See [`crate::events`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputCodes {
    /// One code for the press and the release.
    Key { code: u8 },
    /// One code for each direction of a turn, and one code for a push.
    Knob {
        turn_left: u8,
        turn_right: u8,
        push: u8,
    },
    /// The code of a tap on the bar panel.
    BarPanel { tap: u8 },
    /// The codes of the touch bar. The touch reports have no code: they belong
    /// to the touch bar of the descriptor.
    TouchBar {
        /// A finger touches the touch bar.
        touch_start: u8,
        /// The finger lifts after a move to the right.
        swipe_right: u8,
        /// The finger lifts after a move to the left.
        swipe_left: u8,
        /// The finger lifts, and the device did not identify the gesture.
        touch_end: u8,
    },
}

impl InputCodes {
    pub fn kind(&self) -> ControlKind {
        match self {
            InputCodes::Key { .. } => ControlKind::Key,
            InputCodes::Knob { .. } => ControlKind::Knob,
            InputCodes::BarPanel { .. } => ControlKind::BarPanel,
            InputCodes::TouchBar { .. } => ControlKind::TouchBar,
        }
    }

    /// True if `code` is one of the codes of the control.
    pub fn has(&self, code: u8) -> bool {
        match *self {
            InputCodes::Key { code: c } => c == code,
            InputCodes::Knob {
                turn_left,
                turn_right,
                push,
            } => code == turn_left || code == turn_right || code == push,
            InputCodes::BarPanel { tap } => code == tap,
            InputCodes::TouchBar {
                touch_start,
                swipe_right,
                swipe_left,
                touch_end,
            } => [touch_start, swipe_right, swipe_left, touch_end].contains(&code),
        }
    }

    /// All codes of the control.
    pub fn all(&self) -> Vec<u8> {
        match *self {
            InputCodes::Key { code } => vec![code],
            InputCodes::Knob {
                turn_left,
                turn_right,
                push,
            } => vec![turn_left, turn_right, push],
            InputCodes::BarPanel { tap } => vec![tap],
            InputCodes::TouchBar {
                touch_start,
                swipe_right,
                swipe_left,
                touch_end,
            } => vec![touch_start, swipe_right, swipe_left, touch_end],
        }
    }
}

/// The place of a control among the controls of the same kind. Row 1 is the
/// top row. Column 1 is the left column.
///
/// The descriptor holds only the places that are measured with certainty.
/// A control that has a surface is at the position of that surface: see
/// [`Surface::x`]. The place of a knob relative to the display, and the
/// pixel area of each display window, are not in the descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub row: u8,
    pub column: u8,
}

/// A part of the device that the user operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Control {
    /// The number of the control among the controls of its kind. The kind
    /// comes from the codes. See [`Control::id`].
    pub number: u8,
    pub codes: InputCodes,
    pub position: Position,
    /// The surface of the control, if the control has a display.
    pub surface: Option<SurfaceId>,
    /// The feedback surface when the control has no display.
    pub feedback_surface: Option<SurfaceId>,
}

impl Control {
    /// The surface that shows feedback for this control.
    pub fn feedback_surface(&self) -> Option<SurfaceId> {
        self.surface.or(self.feedback_surface)
    }

    pub fn id(&self) -> ControlId {
        ControlId::new(self.kind(), self.number)
    }

    pub fn kind(&self) -> ControlKind {
        self.codes.kind()
    }
}

// --- Descriptor --------------------------------------------------------------

/// The static data that describes one device model.
#[derive(Debug, Clone, Copy)]
pub struct Descriptor {
    pub name: &'static str,
    /// The stable text identifier of the model, for example `n4-pro`. A
    /// profile names its model with it. Descriptors of models with the same
    /// surfaces and controls have the same identifier, so a profile works on
    /// each of them. The form is small letters, digits, and `-`. See
    /// `docs/decisions/0018-many-profiles.md`.
    pub model: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// Output report payload size.
    pub packet_size: usize,
    pub surfaces: &'static [Surface],
    pub controls: &'static [Control],
    /// The device settings that the model has. See [`SettingsSupport`].
    pub settings: SettingsSupport,
}

/// The device settings that a model has: the capabilities for the display
/// brightness, the knob lights, and the haptic feedback. The engine sends a
/// setting command only to a model that has the setting, and the shell shows
/// only these settings. See
/// `docs/decisions/0022-device-settings-and-stop-state.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SettingsSupport {
    /// The lowest value of the brightness command that changes the light of
    /// the displays, in percent. The values below it give the same light as
    /// this value. The highest value is 100.
    pub lowest_brightness: u8,
    /// The number of knob lights. The colour command has one colour for each
    /// knob light, knob 1 first. 0: the model has no knob lights.
    pub knob_lights: u8,
    /// True if the model has haptic feedback (`QUCMD`).
    pub haptic: bool,
}

impl Descriptor {
    pub fn surface(&self, id: SurfaceId) -> Option<&'static Surface> {
        self.surfaces.iter().find(|s| s.id == id)
    }

    /// The surface that has the slot `slot`.
    pub fn surface_at_slot(&self, slot: u8) -> Option<&'static Surface> {
        self.surfaces.iter().find(|s| s.slot == Some(slot))
    }

    /// The surfaces that have a slot, in table order.
    pub fn slot_surfaces(&self) -> impl Iterator<Item = &'static Surface> {
        self.surfaces.iter().filter(|s| s.slot.is_some())
    }

    /// The background, if the model has one.
    pub fn background(&self) -> Option<&'static Surface> {
        self.surfaces
            .iter()
            .find(|s| s.kind() == SurfaceKind::Background)
    }

    /// The size of the display on which the surfaces have their positions,
    /// in pixels: the smallest area from 0, 0 that contains each surface.
    /// For a model with a background, it is the size of the background.
    pub fn display_size(&self) -> (u16, u16) {
        let width = self.surfaces.iter().map(|s| s.x + s.width).max();
        let height = self.surfaces.iter().map(|s| s.y + s.height).max();
        (width.unwrap_or(0), height.unwrap_or(0))
    }

    pub fn control(&self, id: ControlId) -> Option<&'static Control> {
        self.controls.iter().find(|c| c.id() == id)
    }

    /// The controls of one kind, in table order.
    pub fn controls_of(&self, kind: ControlKind) -> impl Iterator<Item = &'static Control> {
        self.controls.iter().filter(move |c| c.kind() == kind)
    }
}

// --- The table ---------------------------------------------------------------

const fn key_face(number: u8, slot: u8, x: u16, y: u16) -> Surface {
    Surface {
        id: SurfaceId::new(SurfaceKind::KeyFace, number),
        slot: Some(slot),
        x,
        y,
        width: 112,
        height: 112,
        format: ImageFormat::Jpeg,
        rotation: Rotation::Deg180,
    }
}

const fn bar_panel_surface(number: u8, x: u16) -> Surface {
    Surface {
        id: SurfaceId::new(SurfaceKind::BarPanel, number),
        slot: Some(number),
        x,
        y: 358,
        width: 176,
        height: 112,
        format: ImageFormat::Jpeg,
        rotation: Rotation::Deg180,
    }
}

/// N4 Pro: 4 bar panels (slots 1 to 4, from left to right), 10 key faces
/// (slots 11 to 15 in the top row and 6 to 10 in the second row, from left to
/// right), and the 800 × 480 background. Slot 5 is not used. The positions are
/// measured: see "Positions on the display" in `docs/kb/n4-pro-hardware.md`.
static N4PRO_SURFACES: &[Surface] = &[
    bar_panel_surface(1, 0),
    bar_panel_surface(2, 203),
    bar_panel_surface(3, 406),
    bar_panel_surface(4, 609),
    key_face(1, 11, 6, 8),
    key_face(2, 12, 176, 8),
    key_face(3, 13, 346, 8),
    key_face(4, 14, 514, 8),
    key_face(5, 15, 682, 8),
    key_face(6, 6, 6, 178),
    key_face(7, 7, 176, 178),
    key_face(8, 8, 346, 178),
    key_face(9, 9, 514, 178),
    key_face(10, 10, 682, 178),
    Surface {
        id: SurfaceId::new(SurfaceKind::Background, 1),
        slot: None,
        x: 0,
        y: 0,
        width: 800,
        height: 480,
        format: ImageFormat::Jpeg,
        rotation: Rotation::Deg180,
    },
];

/// Key `number` in a grid of 5 columns, with the key face of the same
/// number.
const fn n4pro_key(number: u8, code: u8) -> Control {
    Control {
        number,
        codes: InputCodes::Key { code },
        position: Position {
            row: (number - 1) / 5 + 1,
            column: (number - 1) % 5 + 1,
        },
        surface: Some(SurfaceId::new(SurfaceKind::KeyFace, number)),
        feedback_surface: None,
    }
}

const fn n4pro_knob(number: u8, turn_left: u8, turn_right: u8, push: u8) -> Control {
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
        feedback_surface: Some(SurfaceId::new(SurfaceKind::BarPanel, number)),
    }
}

const fn bar_panel(number: u8, tap: u8) -> Control {
    Control {
        number,
        codes: InputCodes::BarPanel { tap },
        position: Position {
            row: 1,
            column: number,
        },
        surface: Some(SurfaceId::new(SurfaceKind::BarPanel, number)),
        feedback_surface: None,
    }
}

/// N4 Pro: the controls and the input codes. All codes are measured. Keys 1 to
/// 5 are the top row and keys 6 to 10 are the second row, from left to right.
/// Knobs 1 to 4 and bar panels 1 to 4 are from left to right.
static N4PRO_CONTROLS: &[Control] = &[
    n4pro_key(1, 0x01),
    n4pro_key(2, 0x02),
    n4pro_key(3, 0x03),
    n4pro_key(4, 0x04),
    n4pro_key(5, 0x05),
    n4pro_key(6, 0x06),
    n4pro_key(7, 0x07),
    n4pro_key(8, 0x08),
    n4pro_key(9, 0x09),
    n4pro_key(10, 0x0A),
    n4pro_knob(1, 0xA0, 0xA1, 0x37),
    n4pro_knob(2, 0x50, 0x51, 0x35),
    n4pro_knob(3, 0x90, 0x91, 0x33),
    n4pro_knob(4, 0x70, 0x71, 0x36),
    bar_panel(1, 0x40),
    bar_panel(2, 0x41),
    bar_panel(3, 0x42),
    bar_panel(4, 0x43),
    Control {
        number: 1,
        codes: InputCodes::TouchBar {
            touch_start: 0x00,
            swipe_right: 0x38,
            swipe_left: 0x39,
            touch_end: 0xB2,
        },
        position: Position { row: 1, column: 1 },
        // The bar panels are the display of the touch bar.
        surface: None,
        feedback_surface: None,
    },
];

/// N4 Pro: the values 0 to 15 of the brightness command give the same light
/// (measured), 4 knob lights, and haptic feedback. See "Device settings" in
/// `docs/kb/n4-pro-hardware.md`.
const N4PRO_SETTINGS: SettingsSupport = SettingsSupport {
    lowest_brightness: 15,
    knob_lights: 4,
    haptic: true,
};

pub static DESCRIPTORS: &[Descriptor] = &[
    Descriptor {
        name: "VSD StreamDock N4 Pro",
        model: "n4-pro",
        vid: 0x5548,
        pid: 0x1023,
        packet_size: protocol::PACKET_SIZE,
        surfaces: N4PRO_SURFACES,
        controls: N4PRO_CONTROLS,
        settings: N4PRO_SETTINGS,
    },
    // Not tested on hardware. The same data as 0x1023.
    Descriptor {
        name: "StreamDock N4 Pro (0x1008)",
        model: "n4-pro",
        vid: 0x5548,
        pid: 0x1008,
        packet_size: protocol::PACKET_SIZE,
        surfaces: N4PRO_SURFACES,
        controls: N4PRO_CONTROLS,
        settings: N4PRO_SETTINGS,
    },
    // Not tested on hardware. The same data as 0x1023.
    Descriptor {
        name: "StreamDock N4 Pro (0x1021)",
        model: "n4-pro",
        vid: 0x5548,
        pid: 0x1021,
        packet_size: protocol::PACKET_SIZE,
        surfaces: N4PRO_SURFACES,
        controls: N4PRO_CONTROLS,
        settings: N4PRO_SETTINGS,
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

/// The descriptor of the device with the USB identity `vid`:`pid` in the table
/// of the known models, [`DESCRIPTORS`].
pub fn find(vid: u16, pid: u16) -> Option<&'static Descriptor> {
    find_in(DESCRIPTORS, vid, pid)
}

/// The descriptor of the device with the USB identity `vid`:`pid` in `table`.
///
/// [`find`] uses the table of the known models. A test gives a different
/// table, for example a table with a test model that is not a real device.
/// See `docs/ADDING-A-DEVICE-MODEL.md`.
pub fn find_in(table: &'static [Descriptor], vid: u16, pid: u16) -> Option<&'static Descriptor> {
    table.iter().find(|d| d.vid == vid && d.pid == pid)
}
