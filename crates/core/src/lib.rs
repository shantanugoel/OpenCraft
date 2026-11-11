//! Core device layer for OpenCraft: transport, protocol and panel rendering.
//!
//! Deliberately GUI-free and blocking, so it can be driven from a CLI today and
//! from a worker thread behind GPUI later.

pub mod device;
pub mod error;
pub mod events;
pub mod image;
pub mod protocol;
pub mod transport;

pub use device::{Descriptor, Panel, PanelKind};
pub use error::{Error, Result};
pub use events::{decode, Direction, Event};
pub use transport::{list_devices, scan, Found, Transport};
