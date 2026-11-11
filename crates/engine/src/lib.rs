//! The engine of OpenCraft.
//!
//! The engine owns the device, the profile, and the images for the surfaces.
//! A shell sends commands to the engine and gets events from it. The engine
//! does not know the GUI toolkit, and it can operate with no window
//! (architecture rules 1, 2, 3, and 12).
//!
//! - [`profile`]: the profile, its pages, and the content of each surface.
//! - [`store`]: the profile directory: the profile file and the image files.
//! - [`render`] and [`label`]: the image of a surface, made from its content.

pub mod error;
pub mod label;
pub mod profile;
pub mod render;
pub mod store;

pub use error::{Error, Result};
pub use profile::{Color, Page, Profile, SurfaceContent};
pub use store::{default_profile_dir, Backup, BackupReason, Loaded, ProfileStore};
