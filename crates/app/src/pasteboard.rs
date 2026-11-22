//! The system pasteboard and the temporary image file of a paste
//! (decision 0043, section 5).
//!
//! The window reads the pasteboard itself. GPUI reads a string before an
//! image (`gpui-pre-macos-0.3.12/src/pasteboard.rs`), thus a copy of an
//! image from a browser can give its text. The change count tells if the
//! pasteboard item is newer than a copy of a control in the engine.
//!
//! The engine decodes the image and copies it into the profile (rule 9).
//! Thus the window reads the bytes only when the user pastes, writes them to
//! a temporary file, and sends the existing `Command::SetImage` with that
//! path. A frame and a menu read only the change count and the kinds of the
//! item.

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
    /// The path of a file on the system pasteboard.
    File(PathBuf),
}

/// What the system pasteboard has, without the image bytes. A frame reads
/// this: the bytes come only at a paste.
#[cfg_attr(not(any(test, target_os = "macos")), expect(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ClipboardItem {
    /// The path of an image file or of an application bundle (KEY-13).
    File(PathBuf),
    /// Image data on the pasteboard, in the format of `extension`.
    Data { extension: &'static str },
}

/// The system pasteboard as the window last read it.
#[derive(Clone, Debug, Default)]
pub(crate) struct SystemClipboard {
    /// The count of the pasteboard at the read. A copy of a control keeps
    /// the count of its time. A different count means that the pasteboard
    /// item is newer.
    pub change_count: i64,
    /// What the pasteboard has, without the image bytes. A frame and a menu
    /// read this.
    pub item: Option<ClipboardItem>,
    /// The image with its bytes, when the user pasted it. A frame does not
    /// read it. A test sets it.
    pub image: Option<ClipboardImage>,
}

impl SystemClipboard {
    /// True if the pasteboard holds an image, an application bundle, or the
    /// data of an image.
    pub(crate) fn has_image(&self) -> bool {
        self.item.is_some() || self.image.is_some()
    }

    /// The image of the pasteboard, with its bytes. On macOS, this reads the
    /// bytes now, when the user pastes. A test gives `image` itself.
    pub(crate) fn image(&self) -> Option<ClipboardImage> {
        self.image
            .clone()
            .or_else(|| read_image(self.item.as_ref()))
    }
}

/// The change count of the system pasteboard. It changes at each write.
#[cfg(all(target_os = "macos", not(test)))]
pub(crate) fn change_count() -> i64 {
    objc2_app_kit::NSPasteboard::generalPasteboard().changeCount() as i64
}

/// The change count of the system pasteboard now. A test and a different
/// platform use the count of `system`.
pub(crate) fn current_change_count(system: &SystemClipboard) -> i64 {
    #[cfg(all(target_os = "macos", not(test)))]
    {
        let _ = system;
        change_count()
    }
    #[cfg(not(all(target_os = "macos", not(test))))]
    {
        system.change_count
    }
}

/// The pasteboard at `change_count`, without the image bytes. A file path
/// that is an image or an application bundle is first, as in GPUI. Else the
/// image data, because a copy of a file can also hold a picture. A file path
/// that is not an image or a bundle gives no item: it must not hide a copy
/// of a control or use the icon that Finder puts beside the path (review 1,
/// H2).
#[cfg(all(target_os = "macos", not(test)))]
pub(crate) fn probe(change_count: i64) -> SystemClipboard {
    use objc2_app_kit::NSPasteboard;

    let pasteboard = NSPasteboard::generalPasteboard();
    let file = file_path(&pasteboard);
    // SAFETY: The two extern statics are the constants of AppKit.
    let has_png = has_type(&pasteboard, unsafe { objc2_app_kit::NSPasteboardTypePNG });
    // SAFETY: The extern static is the constant of AppKit.
    let has_tiff = has_type(&pasteboard, unsafe { objc2_app_kit::NSPasteboardTypeTIFF });
    SystemClipboard {
        change_count,
        item: available_item(file, has_png, has_tiff),
        image: None,
    }
}

/// The item of a pasteboard with the file URL `file` and the image data
/// types. A file URL is first, as in GPUI. A file that is not an image and
/// not an application bundle is not an item, also when the pasteboard holds
/// a TIFF icon of the file. Image data is used only when there is no file
/// URL.
#[cfg(any(test, all(target_os = "macos", not(test))))]
fn available_item(file: Option<PathBuf>, has_png: bool, has_tiff: bool) -> Option<ClipboardItem> {
    match file {
        Some(path) if is_image(&path) || crate::applications::is_application_bundle(&path) => {
            Some(ClipboardItem::File(path))
        }
        Some(_) => None,
        None if has_png => Some(ClipboardItem::Data { extension: "png" }),
        None if has_tiff => Some(ClipboardItem::Data { extension: "tiff" }),
        None => None,
    }
}

/// The image of `item`, with its bytes, or `None`. A file gives its path;
/// the engine reads the file. Image data gives its bytes.
#[cfg(all(target_os = "macos", not(test)))]
pub(crate) fn read_image(item: Option<&ClipboardItem>) -> Option<ClipboardImage> {
    match item? {
        ClipboardItem::File(path) => Some(ClipboardImage::File(path.clone())),
        ClipboardItem::Data { extension } => {
            let pasteboard = objc2_app_kit::NSPasteboard::generalPasteboard();
            // SAFETY: The two extern statics are the constants of AppKit.
            let kind = match *extension {
                "png" => unsafe { objc2_app_kit::NSPasteboardTypePNG },
                _ => unsafe { objc2_app_kit::NSPasteboardTypeTIFF },
            };
            image_bytes(&pasteboard, kind, extension)
        }
    }
}

/// A different platform and a test have no system image data. A test gives
/// [`SystemClipboard::image`] itself.
#[cfg(not(all(target_os = "macos", not(test))))]
pub(crate) fn read_image(item: Option<&ClipboardItem>) -> Option<ClipboardImage> {
    let _ = item;
    None
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

/// True if the pasteboard can give the type `kind`, without reading its
/// data.
#[cfg(all(target_os = "macos", not(test)))]
fn has_type(
    pasteboard: &objc2_app_kit::NSPasteboard,
    kind: &objc2_app_kit::NSPasteboardType,
) -> bool {
    use objc2_foundation::NSArray;

    pasteboard
        .availableTypeFromArray(&NSArray::from_slice(&[kind]))
        .is_some()
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
/// pastes do not use one file. The shell removes the file after the engine
/// has read it (see `Shell::remove_when_read`).
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

    /// The bytes of a small PNG.
    fn png() -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([4, 5, 6, 255]));
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    /// A file of `name` in `directory`, with the bytes of a PNG when
    /// `image` is true, and text otherwise.
    fn file(directory: &std::path::Path, name: &str, image: bool) -> PathBuf {
        let path = directory.join(name);
        if image {
            fs::write(&path, png()).unwrap();
        } else {
            fs::write(&path, "not an image").unwrap();
        }
        path
    }

    #[test]
    fn an_image_file_and_an_application_bundle_are_items() {
        let directory = tempfile::tempdir().unwrap();
        let image = file(directory.path(), "picture.png", true);
        let bundle = directory.path().join("Demo.app");
        fs::create_dir_all(&bundle).unwrap();
        for path in [image, bundle] {
            assert_eq!(
                available_item(Some(path.clone()), true, true),
                Some(ClipboardItem::File(path))
            );
        }
    }

    #[test]
    fn a_file_that_is_not_an_image_or_a_bundle_is_not_an_item() {
        let directory = tempfile::tempdir().unwrap();
        let text = file(directory.path(), "notes.txt", false);
        let pdf = file(directory.path(), "paper.pdf", false);
        let folder = directory.path().join("Pictures");
        fs::create_dir_all(&folder).unwrap();
        for path in [text, pdf, folder] {
            // Finder puts a TIFF icon beside the path. It must not become
            // the image (review 1, H2).
            assert_eq!(available_item(Some(path), true, true), None);
        }
    }

    #[test]
    fn image_data_with_no_file_url_is_an_item() {
        assert_eq!(
            available_item(None, true, true),
            Some(ClipboardItem::Data { extension: "png" })
        );
        assert_eq!(
            available_item(None, false, true),
            Some(ClipboardItem::Data { extension: "tiff" })
        );
        assert_eq!(available_item(None, false, false), None);
    }

    #[test]
    fn a_probe_holds_the_item_and_no_image_bytes() {
        // A frame sees an image on the pasteboard without reading its bytes
        // (review 1, M3). The bytes come from `image()` at the paste.
        let clipboard = SystemClipboard {
            change_count: 5,
            item: Some(ClipboardItem::Data { extension: "png" }),
            image: None,
        };
        assert!(clipboard.has_image());
        assert!(clipboard.image.is_none());
    }
}
