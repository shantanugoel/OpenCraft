//! The profile file for export and import: one file that holds a profile
//! and its images, so that a user can keep a profile or give it to a
//! different person.
//!
//! The file is TOML. It holds the content of the profile file of decision
//! 0009 in the table `profile`, and the data of each image in the table
//! `images`, as Base64 text. The reasons are in
//! `docs/decisions/0031-profile-export-file.md`.
//!
//! ```toml
//! format = "OpenCraft profile"
//! version = 1
//!
//! [images]
//! "images/4f1c2a9be07d3356.png" = "iVBORw0KGgo..."
//!
//! [profile]
//! schema_version = 5
//! name = "Work"
//! model = "n4-pro"
//!
//! [[profile.pages]]
//! name = "Main"
//!
//! [profile.pages.surfaces.key-face-1]
//! image = "images/4f1c2a9be07d3356.png"
//! ```
//!
//! The file holds no path of the computer: an image path is relative to the
//! profile directory, and the file does not name the profiles directory or
//! the identity of the profile. The engine does not read the settings of an
//! action (rule 8), thus a setting that the user typed, for example a
//! command, goes into the file as it is.
//!
//! An import reads a file from a source that the engine does not trust. It
//! checks the size of the file, the number and the size of the images, and
//! each image path. It decodes each image with limits, and writes a new copy
//! with a name from the data, as for an image of the user. Thus no data from
//! the file goes to a path that the file gives.

use crate::error::{Error, Result};
use crate::profile::{ImageUse, Profile};
use crate::store::{self, image_name, write_and_rename, BackupReason, ProfileStore};
use base64::Engine as _;
use image::{DynamicImage, ImageReader, Limits};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Cursor;
use std::path::Path;

/// The value of the key `format`. A file with a different value is not a
/// profile file of OpenCraft.
pub const FORMAT: &str = "OpenCraft profile";

/// The version of this file format that the engine writes and reads.
pub const VERSION: i64 = 1;

/// The extension of the file name, with no dot.
pub const EXTENSION: &str = "opencraft";

/// The largest file that an import reads, in bytes.
pub const FILE_LIMIT: u64 = 64 * 1024 * 1024;

/// The largest number of images in a file.
pub const IMAGE_COUNT_LIMIT: usize = 1024;

/// The largest data of one image, in bytes, after the Base64 decode. An
/// image copy has at most 512 pixels on a side, thus its PNG data is
/// smaller than 1.1 MB. A background copy of the N4 Pro has at most
/// 1600 × 480 or 800 × 960 pixels with no transparency, thus its PNG data
/// is smaller than 2.4 MB.
pub const IMAGE_DATA_LIMIT: usize = 4 * 1024 * 1024;

/// The largest width and height of an image that an import decodes, in
/// pixels. A larger image becomes smaller after the decode, as an image of
/// the user does.
const DECODE_SIZE_LIMIT: u32 = 4096;

/// The largest memory that the decode of one image can use, in bytes.
const DECODE_MEMORY_LIMIT: u64 = 128 * 1024 * 1024;

/// A profile from a profile file, with the data of its images. The data is
/// not decoded yet: see [`Package::write_images`].
#[derive(Debug, Clone)]
pub struct Package {
    pub profile: Profile,
    /// The data of each image, by its image path in the profile.
    images: BTreeMap<String, Vec<u8>>,
}

fn error(reason: impl Into<String>) -> Error {
    Error::ProfileFile(reason.into())
}

/// Write `profile` and its images from `source` to the file `path`. An
/// image that is missing in `source` is not in the file, and its surface has
/// no image in the file.
pub fn export(profile: &Profile, source: &ProfileStore, path: &Path) -> Result<()> {
    let mut profile = profile.clone();
    let mut images = toml::Table::new();
    let mut missing = Vec::new();
    for relative in profile.image_paths() {
        if images.contains_key(relative) {
            continue;
        }
        match source.image_data(relative) {
            Ok(data) => {
                let text = base64::engine::general_purpose::STANDARD.encode(data);
                images.insert(relative.to_string(), toml::Value::String(text));
            }
            Err(_) => missing.push(relative.to_string()),
        }
    }
    if !missing.is_empty() {
        profile.map_images(|p, _| (!missing.iter().any(|m| m == p)).then(|| p.to_string()));
    }
    let content: toml::Table = store::to_text(&profile)?
        .parse()
        .map_err(|e| error(format!("cannot read the profile as a table: {e}")))?;
    let mut file = toml::Table::new();
    file.insert("format".into(), toml::Value::String(FORMAT.into()));
    file.insert("version".into(), toml::Value::Integer(VERSION));
    file.insert("profile".into(), toml::Value::Table(content));
    file.insert("images".into(), toml::Value::Table(images));
    let text = toml::to_string_pretty(&file)?;
    write_and_rename(path, text.as_bytes())
}

/// Read the profile file `path`. Each error gives a text for the user that
/// tells what is wrong with the file.
pub fn import(path: &Path) -> Result<Package> {
    let not_profile_file = || error("the file is not a profile file of OpenCraft");
    let size = fs::metadata(path)
        .map_err(|e| store::io_error("cannot read", path, e))?
        .len();
    if size > FILE_LIMIT {
        return Err(error(format!(
            "the file is too large for a profile file ({} MB, the limit is {} MB)",
            size.div_ceil(1024 * 1024),
            FILE_LIMIT / 1024 / 1024
        )));
    }
    let data = fs::read(path).map_err(|e| store::io_error("cannot read", path, e))?;
    let text = String::from_utf8(data).map_err(|_| not_profile_file())?;
    parse(&text).map_err(|e| match e {
        Error::ProfileFile(_) => e,
        _ => not_profile_file(),
    })
}

/// Read the text of a profile file. See [`import`].
fn parse(text: &str) -> Result<Package> {
    let not_profile_file = || error("the file is not a profile file of OpenCraft");
    let mut file: toml::Table = text.parse().map_err(|_| not_profile_file())?;
    if file.get("format").and_then(|v| v.as_str()) != Some(FORMAT) {
        return Err(not_profile_file());
    }
    match file.get("version").and_then(|v| v.as_integer()) {
        Some(VERSION) => {}
        Some(found) if found > VERSION => {
            return Err(error(
                "the file is from a newer version of OpenCraft. Update OpenCraft, and then \
                 import the file again",
            ))
        }
        _ => return Err(not_profile_file()),
    }
    let Some(toml::Value::Table(content)) = file.remove("profile") else {
        return Err(error("the file has no profile"));
    };
    let content = toml::to_string(&content)?;
    let profile = store::parse(&content).map_err(|reason| match reason {
        BackupReason::NewerSchema { .. } => error(
            "the profile in the file is from a newer version of OpenCraft. Update \
             OpenCraft, and then import the file again",
        ),
        BackupReason::Invalid(detail) => {
            error(format!("the profile in the file has an error ({detail})"))
        }
    })?;

    let table = match file.remove("images") {
        None => toml::Table::new(),
        Some(toml::Value::Table(table)) => table,
        Some(_) => return Err(error("the images in the file have an error")),
    };
    if table.len() > IMAGE_COUNT_LIMIT {
        return Err(error(format!(
            "the file has too many images ({}, the limit is {IMAGE_COUNT_LIMIT})",
            table.len()
        )));
    }
    let mut images = BTreeMap::new();
    for (key, value) in table {
        if image_name(&key).is_none() {
            return Err(error(format!(
                "the file has an image with the path '{key}'"
            )));
        }
        let Some(text) = value.as_str() else {
            return Err(error(format!("the image '{key}' has an error")));
        };
        // Four Base64 characters give three bytes.
        if text.len() / 4 * 3 > IMAGE_DATA_LIMIT + 3 {
            return Err(error(format!("the image '{key}' is too large")));
        }
        let data = base64::engine::general_purpose::STANDARD
            .decode(text)
            .map_err(|_| error(format!("the image '{key}' has an error")))?;
        if data.len() > IMAGE_DATA_LIMIT {
            return Err(error(format!("the image '{key}' is too large")));
        }
        images.insert(key, data);
    }
    if let Some(missing) = profile.image_paths().find(|p| !images.contains_key(*p)) {
        return Err(error(format!(
            "the file is not complete: the image '{missing}' is not in it"
        )));
    }
    Ok(Package { profile, images })
}

impl Package {
    /// The number of images in the file that the profile uses.
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Decode each image with limits, write a new copy of it into `store`,
    /// and put the paths of the copies into the profile. Then return the
    /// profile. An image that the engine cannot decode gives an error.
    ///
    /// The copy of an image has the limits of its use: a surface image, or
    /// a page background of the size `background` (see
    /// [`ProfileStore::import_background`]). An image with the two uses
    /// gets one copy for each use.
    pub fn write_images(mut self, store: &ProfileStore, background: (u32, u32)) -> Result<Profile> {
        let uses: BTreeSet<(String, ImageUse)> = self
            .profile
            .image_uses()
            .map(|(path, use_)| (path.to_string(), use_))
            .collect();
        let mut paths = BTreeMap::new();
        for (relative, use_) in uses {
            let Some(data) = self.images.get(&relative) else {
                continue;
            };
            let image = decode(data).map_err(|reason| {
                error(format!("the image '{relative}' has an error ({reason})"))
            })?;
            let source = Path::new(&relative);
            let copy = match use_ {
                ImageUse::Surface => store.add_image(image, source)?,
                ImageUse::Background => store.add_background(image, source, background)?,
            };
            paths.insert((relative, use_), copy);
        }
        self.profile
            .map_images(|p, use_| paths.get(&(p.to_string(), use_)).cloned());
        Ok(self.profile)
    }
}

/// Decode the image `data` with the limits of an import.
fn decode(data: &[u8]) -> std::result::Result<DynamicImage, String> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(DECODE_SIZE_LIMIT);
    limits.max_image_height = Some(DECODE_SIZE_LIMIT);
    limits.max_alloc = Some(DECODE_MEMORY_LIMIT);
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    reader.limits(limits);
    reader.decode().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{Color, Page};
    use image::{Rgba, RgbaImage};
    use opencraft_core::{SurfaceId, SurfaceKind};
    use tempfile::TempDir;

    const KEY_1: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 1);
    const KEY_2: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 2);
    /// The size of the background of the N4 Pro.
    const BACKGROUND: (u32, u32) = (800, 480);

    #[test]
    fn a_background_goes_out_and_in_with_its_image_at_the_size_of_a_background() {
        use crate::profile::{Background, BackgroundFit};
        let dir = TempDir::new().unwrap();
        let store = ProfileStore::new(dir.path().join("work"));
        let source = dir.path().join("photo.png");
        RgbaImage::from_fn(1000, 600, |x, y| {
            Rgba([(x % 251) as u8, (y % 241) as u8, 9, 255])
        })
        .save(&source)
        .unwrap();
        let background = store.import_background(&source, BACKGROUND).unwrap();
        let mut profile = Profile::new(
            "Work",
            vec![Page::new("Main"), Page::new("Media"), Page::new("Plain")],
        );
        profile.page_mut(0).unwrap().background = Some(Background::Image {
            image: background.clone(),
            fit: BackgroundFit::Fit,
        });
        profile.page_mut(1).unwrap().background =
            Some(Background::Color(Color::rgb(0x10, 0x20, 0x30)));
        // The same file is also the image of a surface: it gets one copy
        // for each use at the import.
        profile
            .page_mut(2)
            .unwrap()
            .update(KEY_1, |c| c.image = Some(background.clone()));
        let file = dir.path().join("Work.opencraft");
        export(&profile, &store, &file).unwrap();

        let package = import(&file).unwrap();
        assert_eq!(package.image_count(), 1);
        let target = ProfileStore::new(dir.path().join("imported"));
        let imported = package.write_images(&target, BACKGROUND).unwrap();
        assert_eq!(
            imported.pages()[0].background,
            profile.pages()[0].background
        );
        assert_eq!(
            imported.pages()[1].background,
            profile.pages()[1].background
        );
        assert!(imported.pages()[2].background.is_none());
        let surface = imported.pages()[2].content(KEY_1).image.unwrap();
        assert_ne!(surface, background, "a surface copy is smaller");
        let image = |path: &str| target.load_image(path).unwrap();
        assert_eq!(
            (image(&background).width(), image(&background).height()),
            BACKGROUND
        );
        assert_eq!(image(&surface).width(), store::IMAGE_LIMIT);
    }

    /// A store with a profile of two pages, a colour, a first page, and two
    /// images. The same image is on two surfaces.
    fn sample(dir: &TempDir) -> (ProfileStore, Profile) {
        let store = ProfileStore::new(dir.path().join("work"));
        let source = dir.path().join("icon.png");
        RgbaImage::from_fn(32, 24, |x, y| Rgba([x as u8 * 8, y as u8 * 9, 40, 255]))
            .save(&source)
            .unwrap();
        let first = store.import_image(&source).unwrap();
        RgbaImage::from_pixel(600, 300, Rgba([1, 2, 3, 128]))
            .save(&source)
            .unwrap();
        let second = store.import_image(&source).unwrap();
        let mut profile = Profile::new("Work", vec![Page::new("Main"), Page::new("Media")]);
        profile.color = Some(Color::rgb(0x14, 0xB8, 0xA6));
        profile.set_first_page(1);
        let page = profile.page_mut(0).unwrap();
        page.update(KEY_1, |c| {
            c.image = Some(first.clone());
            c.label = Some("Mail".into());
        });
        page.update(KEY_2, |c| c.image = Some(first.clone()));
        profile
            .page_mut(1)
            .unwrap()
            .update(KEY_1, |c| c.image = Some(second.clone()));
        store.save(&profile).unwrap();
        (store, profile)
    }

    #[test]
    fn an_export_and_an_import_give_the_same_profile_with_the_images() {
        let dir = TempDir::new().unwrap();
        let (store, profile) = sample(&dir);
        let file = dir.path().join("Work.opencraft");
        export(&profile, &store, &file).unwrap();

        let package = import(&file).unwrap();
        assert_eq!(package.image_count(), 2, "each image one time");
        let target = ProfileStore::new(dir.path().join("imported"));
        let imported = package.write_images(&target, BACKGROUND).unwrap();
        assert_eq!(imported, profile);
        for path in profile.image_paths() {
            assert_eq!(
                target.image_data(path).unwrap(),
                store.image_data(path).unwrap(),
                "{path}"
            );
        }
    }

    /// The file for the checks of the owner (`docs/checks`) is a correct
    /// profile file with 3 pages, and it holds no personal path.
    #[test]
    fn the_profile_file_of_the_owner_checks_imports() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/checks/phase-7-8-checks.opencraft");
        let package = import(&path).unwrap();
        assert_eq!(package.image_count(), 0);
        assert_eq!(package.profile.pages().len(), 3);
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.replace("/Users/Shared", "").contains("/Users/"));
    }

    #[test]
    fn the_file_holds_no_path_of_the_computer() {
        let dir = TempDir::new().unwrap();
        let (store, profile) = sample(&dir);
        let file = dir.path().join("Work.opencraft");
        export(&profile, &store, &file).unwrap();
        let text = fs::read_to_string(&file).unwrap();
        assert!(
            text.starts_with("format = \"OpenCraft profile\"\nversion = 1\n"),
            "{text}"
        );
        let root = dir.path().to_string_lossy();
        assert!(!text.contains(root.as_ref()), "{text}");
        assert!(!text.contains("work"), "no identity: {text}");
        if let Some(home) = std::env::var_os("HOME") {
            assert!(!text.contains(home.to_string_lossy().as_ref()));
        }
    }

    #[test]
    fn a_missing_image_is_not_in_the_file_and_its_surface_has_no_image() {
        let dir = TempDir::new().unwrap();
        let (store, mut profile) = sample(&dir);
        profile.page_mut(1).unwrap().update(KEY_2, |c| {
            c.image = Some("images/0123456789abcdef.png".into())
        });
        let file = dir.path().join("Work.opencraft");
        export(&profile, &store, &file).unwrap();
        let imported = import(&file)
            .unwrap()
            .write_images(&ProfileStore::new(dir.path().join("i")), BACKGROUND)
            .unwrap();
        assert!(imported.pages()[1].content(KEY_2).image.is_none());
        assert!(imported.pages()[1].content(KEY_1).image.is_some());
    }

    fn import_text(text: &str) -> Result<Package> {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("x.opencraft");
        fs::write(&file, text).unwrap();
        import(&file)
    }

    fn reason(result: Result<Package>) -> String {
        match result {
            Err(Error::ProfileFile(reason)) => reason,
            other => panic!("not a file error: {other:?}"),
        }
    }

    const HEAD: &str = "format = \"OpenCraft profile\"\nversion = 1\n";
    const PROFILE: &str =
        "[profile]\nschema_version = 4\nname = \"W\"\nmodel = \"n4-pro\"\n[[profile.pages]]\nname = \"A\"\n";

    #[test]
    fn a_wrong_file_gives_a_clear_error() {
        let png = base64::engine::general_purpose::STANDARD.encode(b"not a png");
        let cases = [
            ("", "not a profile file"),
            ("not TOML [", "not a profile file"),
            ("schema_version = 4\nname = \"x\"\n", "not a profile file"),
            ("format = \"Other\"\nversion = 1\n", "not a profile file"),
            ("format = \"OpenCraft profile\"\nversion = 2\n", "newer version"),
            (HEAD, "no profile"),
            (
                &format!("{HEAD}[profile]\nschema_version = 9\nname = \"x\"\n"),
                "newer version",
            ),
            (
                &format!("{HEAD}[profile]\nschema_version = 4\nname = \"x\"\n"),
                "has an error",
            ),
            (
                &format!("{HEAD}{PROFILE}[profile.pages.surfaces.key-face-1]\nimage = \"images/0123456789abcdef.png\"\n"),
                "not complete",
            ),
            (
                &format!("{HEAD}{PROFILE}[images]\n\"../../evil.png\" = \"{png}\"\n"),
                "with the path",
            ),
            (
                &format!("{HEAD}{PROFILE}[images]\n\"images/../0123456789abcdef.png\" = \"{png}\"\n"),
                "with the path",
            ),
            (
                &format!("{HEAD}{PROFILE}[images]\n\"images/0123456789abcdef.png\" = \"%%%\"\n"),
                "has an error",
            ),
            (&format!("{HEAD}images = 3\n{PROFILE}"), "images in the file have an error"),
        ];
        for (text, want) in cases {
            let got = reason(import_text(text));
            assert!(got.contains(want), "{text:?}: {got}");
        }
        // Image data that is not an image fails at the decode, and writes
        // nothing.
        let text = format!(
            "{HEAD}{PROFILE}[profile.pages.surfaces.key-face-1]\nimage = \"images/0123456789abcdef.png\"\n[images]\n\"images/0123456789abcdef.png\" = \"{png}\"\n"
        );
        let dir = TempDir::new().unwrap();
        let target = ProfileStore::new(dir.path().join("t"));
        match import_text(&text)
            .unwrap()
            .write_images(&target, BACKGROUND)
        {
            Err(Error::ProfileFile(reason)) => assert!(reason.contains("has an error"), "{reason}"),
            other => panic!("not a file error: {other:?}"),
        }
        assert!(!target.dir().join("images").exists());
    }

    #[test]
    fn a_large_file_and_a_large_image_are_refused() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("large.opencraft");
        let f = fs::File::create(&file).unwrap();
        f.set_len(FILE_LIMIT + 1).unwrap();
        assert!(reason(import(&file)).contains("too large"));

        let data = "A".repeat((IMAGE_DATA_LIMIT / 3 + 8) * 4);
        let text =
            format!("{HEAD}{PROFILE}[images]\n\"images/0123456789abcdef.png\" = \"{data}\"\n");
        assert!(reason(import_text(&text)).contains("too large"));

        let mut text = format!("{HEAD}{PROFILE}[images]\n");
        for n in 0..=IMAGE_COUNT_LIMIT {
            text.push_str(&format!("\"images/{n:016x}.png\" = \"\"\n"));
        }
        assert!(reason(import_text(&text)).contains("too many images"));
    }

    #[test]
    fn a_very_large_image_is_refused_at_the_decode() {
        // A PNG header that gives 100000 × 100000 pixels.
        let mut data = Vec::new();
        RgbaImage::new(1, 1)
            .write_to(&mut Cursor::new(&mut data), image::ImageFormat::Png)
            .unwrap();
        data[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        data[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        assert!(decode(&data).is_err());
    }
}
