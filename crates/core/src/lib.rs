//! Core device layer for OpenCraft: descriptors, transport, protocol, input
//! decoding, and images for surfaces.
//!
//! Deliberately GUI-free and blocking, so it can be driven from a CLI today and
//! from a worker thread behind GPUI later.

pub mod connection;
pub mod device;
pub mod error;
pub mod events;
pub mod image;
pub mod link;
pub mod protocol;
pub mod recording;
pub mod transport;

pub use connection::Device;
pub use device::{
    Control, ControlId, ControlKind, Descriptor, ImageFormat, InputCodes, Position, Rotation,
    SettingsSupport, Surface, SurfaceId, SurfaceKind,
};
pub use error::{Error, Result};
pub use events::{decode, Decoded, Gesture, InputEvent};
pub use link::{Clock, Connector, HidConnector, Link, LinkEvent, SystemClock};
pub use transport::{list_devices, scan, select, FakeTransport, Found, HidTransport, Transport};
