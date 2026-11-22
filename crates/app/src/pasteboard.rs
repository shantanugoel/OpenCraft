//! The system pasteboard and the temporary image file of a paste
//! (decision 0043, section 5).
//!
//! The window reads the pasteboard itself. GPUI reads a string before an
//! image (`gpui-pre-macos-0.3.12/src/pasteboard.rs`), thus a copy of an
//! image from a browser can give its text. The change count tells if the
//! pasteboard item is newer than a copy of a control in the engine.
//!
//! The engine decodes the image and copies it into the profile (rule 9).
//! Thus the window reads the bytes only and writes them to a temporary
//! file. The engine gets the existing `Command::SetImage` with that path.

use std::{fs, io::Write, path::PathBuf};

/// An image on the system pasteboard. A different platform has no
/// pasteboard that makes one, thus the variants have no constructor there.
#[cfg_attr(not(any(test, target_os = "macos")), expect(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ClipboardImage {
    /// The bytes of an image, and the file extension of their format.
    Bytes {
        bytes: Vec<u8>,
        extension: &'static str,
    },
    /// The path of a file on the system pasteboard. The engine checks if it
    /// is an image, and refuses it with the usual problem if it is not.
    File(PathBuf),
}

/// The system pasteboard as the window last read it.
#[derive(Clone, Debug, Default)]
pub(crate) struct SystemClipboard {
    /// The count of the pasteboard at the read. A copy of a control keeps
    /// the count of its time. A different count means that the pasteboard
    /// item is newer.
    pub change_count: i64,
    pub image: Option<ClipboardImage>,
}

impl SystemClipboard {
    /// True if the pasteboard holds an image or a file path.
    pub(crate) fn has_image(&self) -> bool {
        self.image.is_some()
    }
}

/// The change count of the system pasteboard. It changes at each write.
#[cfg(all(target_os = "macos", not(test)))]
pub(crate) fn change_count() -> i64 {
    objc2_app_kit::NSPasteboard::generalPasteboard().changeCount() as i64
}

/// The pasteboard at `change_count`. A file path to an image is first, as
/// in GPUI. Else the image data, because a copy of a file can also hold a
/// picture. A file path that is not an image stays last: the engine tells
/// the user that it is not an image.
#[cfg(all(target_os = "macos", not(test)))]
pub(crate) fn read(change_count: i64) -> SystemClipboard {
    use objc2_app_kit::NSPasteboard;

    let pasteboard = NSPasteboard::generalPasteboard();
    let file = file_path(&pasteboard);
    let image = file
        .clone()
        .filter(|path| is_image(path))
        .map(ClipboardImage::File)
        .or_else(|| {
            // SAFETY: The two extern statics are the constants of AppKit.
            let png = unsafe { objc2_app_kit::NSPasteboardTypePNG };
            image_bytes(&pasteboard, png, "png")
        })
        .or_else(|| {
            // SAFETY: The two extern statics are the constants of AppKit.
            let tiff = unsafe { objc2_app_kit::NSPasteboardTypeTIFF };
            image_bytes(&pasteboard, tiff, "tiff")
        })
        .or_else(|| file.map(ClipboardImage::File));
    SystemClipboard {
        change_count,
        image,
    }
}

/// The path of a file on the pasteboard: a URL string, or the first of an
/// array of them.
#[cfg(all(target_os = "macos", not(test)))]
fn file_path(pasteboard: &objc2_app_kit::NSPasteboard) -> Option<PathBuf> {
    use objc2_foundation::{NSArray, NSString, NSURL};

    // SAFETY: The extern static is the constant of AppKit.
    let list = pasteboard.propertyListForType(unsafe { objc2_app_kit::NSPasteboardTypeFileURL })?;
    let url = if let Some(url) = list.downcast_ref::<NSString>() {
        NSURL::URLWithString(url)
    } else {
        list.downcast_ref::<NSArray>().and_then(|array| {
            array.iter().find_map(|item| {
                item.downcast_ref::<NSString>()
                    .and_then(NSURL::URLWithString)
            })
        })
    }?;
    url.path().map(|path| PathBuf::from(path.to_string()))
}

/// The image data of `kind`, with the file extension of its format.
#[cfg(all(target_os = "macos", not(test)))]
fn image_bytes(
    pasteboard: &objc2_app_kit::NSPasteboard,
    kind: &objc2_app_kit::NSPasteboardType,
    extension: &'static str,
) -> Option<ClipboardImage> {
    let data = pasteboard.dataForType(kind)?;
    let bytes = data.to_vec();
    (!bytes.is_empty()).then_some(ClipboardImage::Bytes { bytes, extension })
}

/// True if `path` is a file that the image decoders can read. A directory
/// (an application bundle) is not an image.
#[cfg(any(test, all(target_os = "macos", not(test))))]
pub(crate) fn is_image(path: &std::path::Path) -> bool {
    image::ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .is_ok_and(|reader| reader.format().is_some())
}

/// Write `bytes` to a new file in the system temporary directory, and
/// return its path. The name has random hexadecimal digits, so that two
/// pastes do not use one file. The caller removes the file.
pub(crate) fn temporary_image(bytes: &[u8], extension: &str) -> std::io::Result<PathBuf> {
    let directory = std::env::temp_dir();
    for _ in 0..32 {
        let path = directory.join(format!("opencraft-{}.{extension}", random_hex()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(bytes) {
                    let _ = fs::remove_file(&path);
                    return Err(error);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "no temporary file name is free",
    ))
}

/// Sixteen random hexadecimal digits. The hash of the system random state
/// is different for each call, and the count makes two calls of one moment
/// different.
fn random_hex() -> String {
    use std::{
        collections::hash_map::RandomState,
        hash::{BuildHasher, Hasher},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static COUNT: AtomicU64 = AtomicU64::new(0);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(u64::from(std::process::id()));
    hasher.write_u64(COUNT.fetch_add(1, Ordering::Relaxed));
    if let Ok(elapsed) = SystemTime::now().duration_since(UNIX_EPOCH) {
        hasher.write_u128(elapsed.as_nanos());
    }
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_temporary_file_holds_the_bytes_and_has_a_new_name() {
        let first = temporary_image(b"one", "png").unwrap();
        let second = temporary_image(b"two", "webp").unwrap();
        assert_ne!(first, second);
        assert_eq!(fs::read(&first).unwrap(), b"one");
        assert_eq!(fs::read(&second).unwrap(), b"two");
        assert_eq!(first.extension().unwrap(), "png");
        assert_eq!(second.extension().unwrap(), "webp");
        let name = first.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("opencraft-"));
        assert_eq!(name.len(), "opencraft-".len() + 16 + ".png".len());
        fs::remove_file(first).unwrap();
        fs::remove_file(second).unwrap();
    }

    #[test]
    fn only_an_image_file_is_an_image() {
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join("picture.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]))
            .save(&image)
            .unwrap();
        let text = directory.path().join("notes.txt");
        fs::write(&text, "not an image").unwrap();
        assert!(is_image(&image));
        assert!(!is_image(&text));
        assert!(!is_image(directory.path()));
        assert!(!is_image(&directory.path().join("missing.png")));
    }
}
