use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    #[error("no matching device found")]
    DeviceNotFound,

    #[error("device returned an unexpected report size: {0}")]
    ShortReport(usize),

    #[error("image encoding failed: {0}")]
    Image(#[from] image::ImageError),

    #[error("panel {0} does not exist on this device")]
    NoSuchPanel(u8),

    #[error(
        "macOS denied access to the device (kIOReturnNotPermitted).\n\n           Fix: System Settings -> Privacy & Security -> Input Monitoring,\n           enable the app you are running this from (Terminal / iTerm / your editor),\n           then quit and reopen that app.\n\n           Also make sure VSD Craft is not running: it may hold the device exclusively."
    )]
    PermissionDenied,
}

pub type Result<T> = std::result::Result<T, Error>;
