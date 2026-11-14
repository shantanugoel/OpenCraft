//! One complete mouse operation.
//!
//! The kind module reads the settings. No operation leaves a button held.

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// One complete mouse operation. No operation leaves a button held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEvent {
    Click { button: Button, count: u8 },
    Move { x: i32, y: i32, relative: bool },
    Scroll { x: i32, y: i32 },
    Drag { button: Button, x: i32, y: i32 },
}
