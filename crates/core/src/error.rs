use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    /// A transport failed with no `hidapi` error, for example the fake
    /// transport on request.
    #[error("transport error: {0}")]
    Transport(String),

    #[error("no matching device found")]
    DeviceNotFound,

    #[error("device returned an unexpected report size: {0}")]
    ShortReport(usize),

    #[error("image encoding failed: {0}")]
    Image(#[from] image::ImageError),

    #[error("panel {0} does not exist on this device")]
    NoSuchPanel(u8),

    /// A known device has no vendor interface in the list. OpenCraft opens only
    /// the vendor interface (architecture rule 6).
    #[error(
        "the device is connected, but it shows no vendor interface.\n\n\
         OpenCraft opens only the vendor interface. It does not open the keyboard interface.\n\
         Connect the device again. If the error stays, run 'opencraft-probe list' and look at the interfaces of the device."
    )]
    NoVendorInterface,

    /// The operating system refused to open the vendor interface.
    #[error(
        "the operating system refused access to the vendor interface of the device (kIOReturnNotPermitted).\n\n\
         The Input Monitoring permission is not necessary. The vendor interface needs no privacy permission.\n\
         The cause is not known. Possible causes: a different application holds the device (for example VSD Craft), \
         or the cable or the device changed state.\n\
         Quit VSD Craft, connect the device again, and try again."
    )]
    PermissionDenied,
}

pub type Result<T> = std::result::Result<T, Error>;
