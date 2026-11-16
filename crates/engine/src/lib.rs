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
//! - [`profile_list`]: the rules for the links, the clashes, and the default
//!   profile of the profile list.
//! - [`front_switch`]: the state machine of the automatic switch. It decides
//!   what the engine does after a front application notice.
//! - [`store`]: one profile directory: the profile file and the image files.
//! - [`share`]: the profile file for export and import, with the images.
//! - [`render`] and [`label`]: the image of a surface, made from its content.
//! - [`icons`]: the built-in icon set for surfaces.
//! - [`device_settings`]: the device settings of each model, and their file.
//! - [`layout`]: the layout of the device, as a message for a shell.
//! - [`location`]: the active page and the open folders.
//! - [`engine`]: the engine logic, with the commands and the events.
//! - [`history`]: the undo and redo history of the active profile.
//! - [`saved`]: a value that the engine writes to a file after a delay: the
//!   active profile and the device settings.
//! - [`runtime`]: the engine thread.
//!
//! A shell uses only this crate, and not the device layer. Thus the crate
//! gives the shell the types of the device layer that are in the messages.

pub mod action;
pub mod clipboard;
pub mod device_settings;
pub mod engine;
pub mod error;
pub mod front;
pub mod front_switch;
pub mod history;
pub mod icons;
pub mod label;
pub mod layout;
pub mod location;
pub mod power;
pub mod profile;
pub mod profile_list;
pub mod profiles;
pub mod render;
pub mod runtime;
pub mod saved;
pub mod share;
pub mod store;

pub use action::*;
pub use device_settings::{DeviceSettings, KnobLightMode, KnobLights, SettingsSupport};
pub use engine::{
    BackgroundState, Command, Counters, Engine, Event, NewBackground, PageRef, Place, SurfaceState,
};
/// The image type of the messages.
pub use image::RgbImage;
pub use layout::{ControlLayout, DeviceLayout, SurfaceLayout};
pub use location::{Holder, Location};
/// The types of the device layer that are in the messages.
pub use opencraft_core::{ControlId, ControlKind, Gesture, InputEvent, SurfaceId, SurfaceKind};
pub use power::{DisplayState, PowerNotice};
pub use runtime::{spawn, Config, EngineHandle, EngineSender};

pub use error::{Error, Result};
pub use front::{ActiveCause, FrontApplication};
pub use icons::{icon_set, Icon, IconCategory, IconSet};
pub use profile::{
    ApplicationLink, Background, BackgroundFit, Color, Folder, FolderEntry, FolderId, LabelChange,
    LabelPosition, LabelStyle, Page, PageId, ParseFolderIdError, ParsePageIdError, Profile,
    SurfaceContent, SurfaceIcon, LABEL_SIZES,
};
pub use profiles::{LinkOwner, ProfileEntry, ProfilesDir};
pub use render::{icon_color, label_color, page_preview};
pub use store::{
    default_profiles_dir, profile_from_text, Backup, BackupReason, Loaded, ProfileStore,
};
