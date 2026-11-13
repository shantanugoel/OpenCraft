//! The engine of OpenCraft.
//!
//! The engine owns the device, the profile, and the images for the surfaces.
//! A shell sends commands to the engine and gets events from it. The engine
//! does not know the GUI toolkit, and it can operate with no window
//! (architecture rules 1, 2, 3, and 12).
//!
//! - [`action`]: actions, bindings, and the handler interface.
//! - [`profile`]: the profile, its pages, and the content of each surface.
//! - [`profiles`]: the profiles directory: one directory for each profile,
//!   and the active profile of each device model.
//! - [`store`]: one profile directory: the profile file and the image files.
//! - [`share`]: the profile file for export and import, with the images.
//! - [`render`] and [`label`]: the image of a surface, made from its content.
//! - [`device_settings`]: the device settings of each model, and their file.
//! - [`layout`]: the layout of the device, as a message for a shell.
//! - [`engine`]: the engine logic, with the commands and the events.
//! - [`history`]: the undo and redo history of the active profile.
//! - [`runtime`]: the engine thread.
//!
//! A shell uses only this crate, and not the device layer. Thus the crate
//! gives the shell the types of the device layer that are in the messages.

pub mod action;
pub mod clipboard;
pub mod device_settings;
pub mod engine;
pub mod error;
pub mod history;
pub mod label;
pub mod layout;
pub mod profile;
pub mod profiles;
pub mod render;
pub mod runtime;
pub mod share;
pub mod store;

pub use action::*;
pub use device_settings::{DeviceSettings, KnobLightMode, KnobLights, SettingsSupport};
pub use engine::{Command, Counters, Engine, Event, SurfaceState};
/// The image type of the messages.
pub use image::RgbImage;
pub use layout::{ControlLayout, DeviceLayout, SurfaceLayout};
/// The types of the device layer that are in the messages.
pub use opencraft_core::{ControlId, ControlKind, Gesture, InputEvent, SurfaceId, SurfaceKind};
pub use runtime::{spawn, Config, EngineHandle};

pub use error::{Error, Result};
pub use profile::{
    Color, LabelChange, LabelPosition, LabelStyle, Page, Profile, SurfaceContent, LABEL_SIZES,
};
pub use profiles::{ProfileEntry, ProfilesDir};
pub use render::label_color;
pub use store::{default_profiles_dir, Backup, BackupReason, Loaded, ProfileStore};
