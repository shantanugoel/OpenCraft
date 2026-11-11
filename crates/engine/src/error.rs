use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// A file operation in the profile directory failed.
    #[error("{action} {path}: {source}")]
    Io {
        /// What the engine did, for example "cannot write".
        action: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The engine could not decode or encode an image.
    #[error("cannot use the image {path}: {source}")]
    Image {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    /// The profile refers to an image with a path that is not a file in the
    /// image directory of the profile.
    #[error("the image path '{0}' is not in the image directory of the profile")]
    ImagePath(String),

    /// The profile could not be written as text.
    #[error("cannot write the profile as text: {0}")]
    Format(#[from] toml::ser::Error),

    /// An error from the device layer.
    #[error(transparent)]
    Device(#[from] opencraft_core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
