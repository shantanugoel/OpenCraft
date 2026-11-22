//! One profile directory: the profile file and the image files.
//!
//! A profile is a directory in the profiles directory (see
//! [`crate::profiles`]). The profile file is `profile.toml`, and the images
//! are in the directory `images` next to it. The reasons are in
//! `docs/decisions/0009-profile-file-format.md` and
//! `docs/decisions/0018-many-profiles.md`.
//!
//! - The engine writes the file (architecture rule 9). A write makes a
//!   temporary file and renames it, so a stop during a write does not leave
//!   a file that is not complete.
//! - The file has a schema version. The engine does not change a file that
//!   it cannot use: it renames the file to a backup name and starts with an
//!   empty profile. It does this only for the active profile. For a
//!   different profile, [`ProfileStore::read`] gives an error and changes
//!   nothing.
//! - An image of the user becomes a PNG copy in `images`. The profile does
//!   not refer to the original file.

//! ## Schema version 4
//!
//! Version 4 adds the label options of a surface (issue #189):
//!
//! | Key | Value | With no key |
//! |---|---|---|
//! | `label_position` | `"top"`, `"middle"`, `"bottom"`, or `"none"` | The middle with no image, the bottom with an image |
//! | `label_size` | The font size in pixels, from 10 to 40 | 20 |
//! | `label_color` | `#RRGGBB` | Black or white, for the fill |
//! | `label_bold` | `true` or `false` | `true` |
//!
//! The engine writes a key only if its value is not the value with no key.
//! A file of version 1 to 3 has no label options, and loads with the look of
//! the earlier versions.
//!
//! Version 4 also adds two top keys for the profile settings (issue #195):
//!
//! | Key | Value | With no key |
//! |---|---|---|
//! | `color` | The colour of the profile for the user, `#RRGGBB` | No colour |
//! | `first_page` | The number of the page that the device shows when the profile becomes active, from 1 | The page at the start of the list |
//!
//! Version 3 added the key `model`: the model identifier of the descriptor.
//! Version 2 added bindings. A file of version 1 or 2 loads with the model of
//! the first descriptor (see [`crate::profile::default_model`]), and version 1
//! loads with no bindings. Nothing else changes, so the load loses nothing.
//! The next write writes version 4.
//!
//! Version 4 also adds the background of a page (issue #191): the table
//! `background` of a page, with one of these keys. Decision 0032 gives the
//! reasons.
//!
//! | Key | Value | With no key |
//! |---|---|---|
//! | `color` | One colour on all of the display, `#RRGGBB` | — |
//! | `image` | An image path in `images`, as for a surface | — |
//! | `fit` | With `image` only: `"fill"` or `"fit"` | `"fill"` |
//!
//! A page with no table `background` has no background. A background image
//! copy is larger than a surface image copy: see
//! [`ProfileStore::import_background`].
//!
//! Version 4 also adds the icon of a surface (issue #190, decision 0033):
//!
//! | Key | Value | With no key |
//! |---|---|---|
//! | `icon` | The name of an icon of the built-in set, for example `"globe"`: small letters, digits, and hyphens | No icon |
//! | `icon_color` | `#RRGGBB`. Only with `icon`. | Black or white, for the fill |
//!
//! A surface has `image` or `icon`, not both. An icon name that the set of
//! this engine does not have loads and stays in the file, and the surface
//! shows no icon.
//!
//! Schema version 5 adds `folders` (decision 0036) and `applications`
//! (decision 0035). A file of version 4 or lower has neither key. Release
//! 0.2.0 writes version 4. This engine writes version 5. A file of version
//! 4 or lower that has `applications` or `folders` has an error: the engine
//! keeps that file as a backup.
//!
//! | Key | Value | With no key |
//! |---|---|---|
//! | `applications` | An array of tables, in the order of the links. Each table has `bundle_id` and `name`. | No links |
//!
//! Schema version 6 adds the key `id` of a page (decision 0040): 16 small
//! hexadecimal digits that are not all zero. The identity stays with the page
//! when it moves, and the pages of a profile have different identities. A
//! file of version 6 or later has the key for each page. A file of version 5
//! or lower has an error if it has the key. The engine gives the page
//! at place N of such a file the identity N+1 until it writes the file again,
//! thus an unchanged file gives the same identities at each read.
//!
//! Schema version 8 adds the label keys of decision 0043:
//!
//! | Key | Value | With no key |
//! |---|---|---|
//! | `label_position` | `"top"`, `"middle"`, `"bottom"`, `"none"`, or `"auto"` | The default title style of the profile |
//! | `label_color` | `#RRGGBB` or `"auto"` | The default title style of the profile |
//! | `label_font` | A font family, or `"system"` | The default title style of the profile |
//! | `label_size` | The font size in pixels, from 10 to 40 | The default title style of the profile |
//! | `label_bold`, `label_italic`, `label_underline` | `true` or `false` | The default title style of the profile |
//!
//! `"auto"` is the automatic position and the automatic colour, and
//! `"system"` is the system font. A surface uses an automatic value also
//! when the default title style of the profile has a value. A key of version
//! 8 in a file of version 1 to 7 has an error: the engine keeps that file as
//! a backup (decision 0009).
//!
//! Version 8 writes a label key when the surface has an own value, also when
//! the value is the engine default. Versions 4 to 7 wrote a size and a
//! weight only when it was not the default. Thus an old surface with the
//! default look has no such key and inherits, and it shows as before.
//!
//! Schema version 8 also adds the crop of an image (decision 0043, KEY-02):
//!
//! | Key | Value | With no key |
//! |---|---|---|
//! | `image_zoom` | The zoom in percent, from 100 to 400 | 100 |
//! | `image_offset_x` | The pan on the x axis, in per mille of the overflow, from -1000 to 1000 | 0 |
//! | `image_offset_y` | The pan on the y axis, in per mille of the overflow, from -1000 to 1000 | 0 |
//!
//! The keys are independent, and the fit (100, 0, 0) writes no key. A key
//! on a face with no `image` has an error. An `on` face has its own keys.
//!
//! ```toml
//! schema_version = 8
//! name = "Default"
//! model = "n4-pro"
//! color = "#3B82F6"
//! first_page = 2
//! [[pages]]
//! id = "0f3a9c27d41b8e65"
//! name = "Page 1"
//! [pages.background]
//! image = "images/0b5e2f6a91c4d873.png"
//! fit = "fit"
//! [pages.surfaces.key-face-3]
//! label = "Mail"
//! label_position = "top"
//! label_size = 14
//! icon = "mail"
//! icon_color = "#F59E0B"
//! [pages.bindings.key-3.press]
//! kind = "opencraft.hotkey"
//! [pages.bindings.key-3.press.settings]
//! hotkey = "cmd+shift+4"
//! ```
//!
//! Control keys are `key-N`, `knob-N`, `bar-panel-N`, and `touch-bar-N`.
//! Gesture keys are `press`, `release`, `turn-left`, `turn-right`, `push`,
//! `tap`, `swipe-left`, `swipe-right`, `long-press`, and `double-press`.
//! The last two require schema version 7 (decision 0042). Settings keep
//! text, integer, boolean, list, and table values.
//!
//! A file of version 2 can have the gesture keys `tap`, `swipe-left`, and
//! `swipe-right`: they came in the same phase, before version 3. An engine
//! that does not know them keeps the file as a backup, because the file has
//! an error for it (decision 0009).

use crate::action::{Action, GestureKind};
use crate::error::{Error, Result};
use crate::front::{same_bundle, valid_bundle_id, MAX_LINKS};
use crate::profile::{control_key, parse_control_key};
use crate::profile::{
    default_model, parse_surface_key, surface_key, ApplicationLink, Auto, Background,
    BackgroundFit, Color, Folder, FolderId, ImageCrop, LabelPosition, LabelStyle, Page, PageId,
    Profile, SurfaceContent, SurfaceIcon, MAX_IMAGE_PAN, MAX_IMAGE_ZOOM, MAX_LABEL_SIZE,
    MIN_IMAGE_ZOOM, MIN_LABEL_SIZE, SYSTEM_FONT_WORD,
};
use crate::saved::SavedFile;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader};
use opencraft_core::{ControlKind, SurfaceKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};

/// The schema version that this engine writes.
pub const SCHEMA_VERSION: u32 = 8;

/// The first schema version with the label keys of Phase 11 (decision 0043):
/// the font, the italic form, the underline, the automatic word of the
/// position and the colour, and the larger label size of ICON-04.
pub const FACES_VERSION: u32 = 8;

/// The largest `label_size` in a file of schema version 4 to 7. Version 8
/// has [`MAX_LABEL_SIZE`] (decision 0043).
const MAX_LABEL_SIZE_BEFORE_FACES: u8 = 40;

/// The file word of the automatic position and the automatic colour.
const AUTO_WORD: &str = "auto";

/// The first schema version with long and double presses (decision 0042).
const GESTURE_TIMERS_VERSION: u32 = 7;
/// Schema version 7 adds the On face of a surface.
const STATE_FACES_VERSION: u32 = 7;

/// The first schema version with the links of a profile (decision 0035).
const APPLICATIONS_VERSION: u32 = 5;

/// The first schema version with the label options of a surface, and with
/// the colour and the first page of the profile.
const LABEL_OPTIONS_VERSION: u32 = 4;

/// The first schema version with the key `folders` (decision 0036).
const FOLDERS_VERSION: u32 = 5;

/// The first schema version with the key `id` of a page (decision 0040).
const PAGE_IDS_VERSION: u32 = 6;

/// The first schema version with the key `model`.
const MODEL_VERSION: u32 = 3;

/// The name of the profile file in the profile directory.
pub const PROFILE_FILE: &str = "profile.toml";

/// The name of the image directory in the profile directory.
pub const IMAGE_DIR: &str = "images";

/// The largest width and height of an image copy, in pixels. A larger image
/// becomes smaller when the user selects it.
pub const IMAGE_LIMIT: u32 = 512;

/// The largest width and height of a background copy, as a multiple of the
/// size of the background. See [`ProfileStore::import_background`].
pub const BACKGROUND_LIMIT: u32 = 2;

/// The profiles directory of the user. Each profile is a directory in it.
/// `None` if the operating system gives no home directory.
///
/// On macOS it is `~/Library/Application Support/OpenCraft/profiles`. The
/// profile of the earlier phases is the directory `default` in it.
pub fn default_profiles_dir() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(var("HOME")?)
            .join("Library/Application Support")
            .join("OpenCraft")
    } else if cfg!(windows) {
        PathBuf::from(var("APPDATA")?).join("OpenCraft")
    } else {
        match var("XDG_CONFIG_HOME") {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from(var("HOME")?).join(".config"),
        }
        .join("opencraft")
    };
    Some(base.join("profiles"))
}

/// Why the engine did not use a profile file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupReason {
    /// The file has a schema version that is newer than this engine knows.
    NewerSchema { found: u32 },
    /// The file has an error. The text gives the error.
    Invalid(String),
}

impl fmt::Display for BackupReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackupReason::NewerSchema { found } => {
                write!(
                    f,
                    "it has the schema version {found}, from a newer OpenCraft"
                )
            }
            BackupReason::Invalid(error) => write!(f, "it has an error ({error})"),
        }
    }
}

/// A profile file that the engine did not use. The engine renamed it and did
/// not change its content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
    /// The new path of the file.
    pub path: PathBuf,
    pub reason: BackupReason,
}

/// The result of [`ProfileStore::load`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub profile: Profile,
    /// `Some` if the directory had a profile file that the engine could not
    /// use. Then `profile` is a new profile with no content.
    pub backup: Option<Backup>,
}

/// The parts of a profile file that the list of profiles shows: see
/// [`ProfileStore::summary`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub name: Option<String>,
    pub model: Option<String>,
    pub color: Option<Color>,
    /// The names of the pages, in their order.
    pub pages: Vec<String>,
    /// The identities of the pages, in their order. A file that has no
    /// identities, or one that is not correct, gives the same identities as a
    /// full read of a file of version 5 or lower.
    pub page_ids: Vec<PageId>,
    /// The number of the first page, from 0.
    pub first_page: usize,
    /// The links that the file names. The summary uses the same check as
    /// the full read. When that check gives an error, the list is empty.
    pub applications: Vec<ApplicationLink>,
    /// The default title style of the profile (decision 0043, section 2).
    pub default_label_style: LabelStyle,
}

/// One profile directory.
#[derive(Debug, Clone)]
pub struct ProfileStore {
    dir: PathBuf,
}

impl SavedFile<Profile> for ProfileStore {
    fn exists(&self) -> bool {
        self.profile_path().exists()
    }

    fn write(&self, profile: &Profile) -> Result<()> {
        self.save(profile)
    }
}

impl ProfileStore {
    /// Use the profile directory `dir`. The directory does not have to exist:
    /// the first write makes it.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The path of the profile file.
    pub fn profile_path(&self) -> PathBuf {
        self.dir.join(PROFILE_FILE)
    }

    /// Load the profile.
    ///
    /// - No file: the result is a new profile with no content. Nothing is
    ///   written.
    /// - A file that the engine cannot use: the engine renames it to a backup
    ///   name. The result is a new profile, with the backup.
    /// - A file that the engine cannot read or rename gives an error. The
    ///   caller must not write a profile then, because the write would
    ///   replace the file.
    pub fn load(&self) -> Result<Loaded> {
        match self.parse_file()? {
            None => Ok(Loaded {
                profile: Profile::default(),
                backup: None,
            }),
            Some(Ok(profile)) => Ok(Loaded {
                profile,
                backup: None,
            }),
            Some(Err(reason)) => self.set_aside(reason),
        }
    }

    /// Read the profile, and change nothing. Use this for a profile that is
    /// not the active profile: a missing file, or a file that the engine
    /// cannot use, gives an error, and the file stays as it is.
    pub fn read(&self) -> Result<Profile> {
        let unusable = |reason: String| Error::Unusable {
            path: self.profile_path(),
            reason,
        };
        match self.parse_file()? {
            Some(Ok(profile)) => Ok(profile),
            Some(Err(reason)) => Err(unusable(reason.to_string())),
            None => Err(unusable("the file does not exist".to_string())),
        }
    }

    /// The profile in the file, or the reason why the engine cannot use the
    /// file. `None` if there is no file.
    fn parse_file(&self) -> Result<Option<std::result::Result<Profile, BackupReason>>> {
        let path = self.profile_path();
        match fs::read_to_string(&path) {
            Ok(text) => Ok(Some(parse(&text))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            // Text that is not UTF-8 is a file with an error, not a failed read.
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                Ok(Some(Err(BackupReason::Invalid(e.to_string()))))
            }
            Err(source) => Err(io_error("cannot read", &path, source)),
        }
    }

    /// The name for the user, the model, and the profile settings, from the
    /// file with no full check. Thus a profile with an error stays in the
    /// list of profiles. Each part that the read cannot find is `None` or
    /// empty. With no readable file, all parts are `None` or empty.
    pub fn summary(&self) -> Summary {
        let Ok(text) = fs::read_to_string(self.profile_path()) else {
            return Summary::default();
        };
        let Ok(table) = text.parse::<toml::Table>() else {
            return Summary::default();
        };
        let text_of = |key: &str| table.get(key).and_then(|v| v.as_str()).map(str::to_string);
        let pages: Vec<String> = table
            .get("pages")
            .and_then(|v| v.as_array())
            .map(|pages| {
                pages
                    .iter()
                    .map(|p| {
                        p.get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or_default()
                            .to_string()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let page_ids = table
            .get("pages")
            .and_then(|v| v.as_array())
            .map(|pages| {
                pages
                    .iter()
                    .enumerate()
                    .map(|(index, page)| {
                        page.get("id")
                            .and_then(|id| id.as_str())
                            .and_then(|id| id.parse().ok())
                            .unwrap_or_else(|| PageId::of_old_page(index))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let first_page = table
            .get("first_page")
            .and_then(|v| v.as_integer())
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| (1..=pages.len()).contains(n))
            .map_or(0, |n| n - 1);
        Summary {
            name: text_of("name"),
            model: text_of("model"),
            color: text_of("color").and_then(|c| c.parse().ok()),
            pages,
            page_ids,
            first_page,
            applications: applications_of(&table).unwrap_or_default(),
            default_label_style: default_label_style_of(&table),
        }
    }

    /// Rename the profile file to a backup name that is not in use.
    fn set_aside(&self, reason: BackupReason) -> Result<Loaded> {
        let from = self.profile_path();
        let mut number = 1;
        let to = loop {
            let candidate = self.dir.join(format!("profile.backup-{number}.toml"));
            if !candidate.exists() {
                break candidate;
            }
            number += 1;
        };
        fs::rename(&from, &to).map_err(|e| io_error("cannot rename", &from, e))?;
        Ok(Loaded {
            profile: Profile::default(),
            backup: Some(Backup { path: to, reason }),
        })
    }

    /// Write the profile file.
    pub fn save(&self, profile: &Profile) -> Result<()> {
        let text = to_text(profile)?;
        self.make_dir(&self.dir)?;
        write_and_rename(&self.profile_path(), text.as_bytes())
    }

    /// Make a copy of the image file `source` in the image directory, and
    /// return the path of the copy relative to the profile directory. Put
    /// this path in [`SurfaceContent::image`].
    ///
    /// The copy is a PNG. An image that is larger than [`IMAGE_LIMIT`] on a
    /// side becomes smaller first. The name of the copy comes from its data,
    /// so the same image gives the same file.
    ///
    /// A file that is not an image gives an error, and nothing is written.
    pub fn import_image(&self, source: &Path) -> Result<String> {
        self.add_image(decode_file(source)?, source)
    }

    /// Make a copy of the image file `source` for the background of a page,
    /// and return its path relative to the profile directory. Put this path
    /// in [`Background::Image`]. `size` is the size of the background of the
    /// model, in pixels.
    ///
    /// The copy is a PNG with no transparency: a transparent pixel goes on
    /// black, as on the device. An image that is larger than necessary to
    /// cover `size` becomes smaller: the copy is the smallest size that
    /// covers `size`, thus each fit has all the pixels that it needs. The
    /// copy has at most [`BACKGROUND_LIMIT`] times `size` on a side. The
    /// name of the copy comes from its data, as for [`Self::import_image`].
    ///
    /// A file that is not an image gives an error, and nothing is written.
    pub fn import_background(&self, source: &Path, size: (u32, u32)) -> Result<String> {
        self.add_background(decode_file(source)?, source, size)
    }

    /// Write a background copy of `image`, as [`Self::import_background`]
    /// does. `source` names the image in an error.
    pub(crate) fn add_background(
        &self,
        image: DynamicImage,
        source: &Path,
        size: (u32, u32),
    ) -> Result<String> {
        let (width, height) = background_copy_size(image.width(), image.height(), size);
        let image = if (width, height) == (image.width(), image.height()) {
            image
        } else {
            image.resize_exact(width, height, FilterType::Lanczos3)
        };
        let rgba = image.into_rgba8();
        let rgb = image::RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
            let [r, g, b, a] = rgba.get_pixel(x, y).0;
            let on_black = |c: u8| ((u16::from(c) * u16::from(a) + 127) / 255) as u8;
            image::Rgb([on_black(r), on_black(g), on_black(b)])
        });
        self.write_copy(DynamicImage::ImageRgb8(rgb), source)
    }

    /// Write a copy of `image` in the image directory, as
    /// [`Self::import_image`] does, and return its path relative to the
    /// profile directory. `source` names the image in an error.
    pub(crate) fn add_image(&self, image: DynamicImage, source: &Path) -> Result<String> {
        let image = if image.width() > IMAGE_LIMIT || image.height() > IMAGE_LIMIT {
            image.resize(IMAGE_LIMIT, IMAGE_LIMIT, FilterType::Lanczos3)
        } else {
            image
        };
        // Keep the transparency: the colour of the surface shows through it.
        self.write_copy(DynamicImage::ImageRgba8(image.into_rgba8()), source)
    }

    /// Write `image` as a PNG in the image directory, with a name from its
    /// data, and return its path relative to the profile directory.
    fn write_copy(&self, image: DynamicImage, source: &Path) -> Result<String> {
        let image_error = |source_error| Error::Image {
            path: source.to_path_buf(),
            source: source_error,
        };
        let mut data = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut data), ImageFormat::Png)
            .map_err(image_error)?;

        let name = format!("{:016x}.png", fnv1a_64(&data));
        let dir = self.dir.join(IMAGE_DIR);
        self.make_dir(&dir)?;
        let path = dir.join(&name);
        if !path.exists() {
            write_and_rename(&path, &data)?;
        }
        Ok(format!("{IMAGE_DIR}/{name}"))
    }

    /// The data of the image copy at `relative`, a path from
    /// [`Self::import_image`], with no decode. A path that is not a file in
    /// the image directory gives [`Error::ImagePath`], and the method reads
    /// nothing.
    pub(crate) fn image_data(&self, relative: &str) -> Result<Vec<u8>> {
        let name = image_name(relative).ok_or_else(|| Error::ImagePath(relative.to_string()))?;
        let path = self.dir.join(IMAGE_DIR).join(name);
        fs::read(&path).map_err(|e| io_error("cannot read", &path, e))
    }

    /// Decode the image copy at `relative`, a path from
    /// [`Self::import_image`]. A path that is not a file in the image
    /// directory gives [`Error::ImagePath`], and the method reads nothing.
    pub fn load_image(&self, relative: &str) -> Result<DynamicImage> {
        let name = image_name(relative).ok_or_else(|| Error::ImagePath(relative.to_string()))?;
        let path = self.dir.join(IMAGE_DIR).join(name);
        ImageReader::open(&path)
            .map_err(|e| io_error("cannot read", &path, e))?
            .with_guessed_format()
            .map_err(|e| io_error("cannot read", &path, e))?
            .decode()
            .map_err(|source| Error::Image { path, source })
    }

    /// Remove each image copy that no surface of `profile` uses. Return the
    /// number of files that were removed.
    ///
    /// The method removes only files that have the name form of an image
    /// copy. It does not touch a different file in the image directory.
    pub fn remove_unused_images(&self, profile: &Profile) -> Result<usize> {
        self.remove_unused_images_but(profile, std::iter::empty())
    }

    /// [`Self::remove_unused_images`], but also keep each image copy that
    /// `also` names. The engine gives the images of the undo history that it
    /// keeps for the profile (decision 0030).
    pub fn remove_unused_images_but<'a>(
        &self,
        profile: &'a Profile,
        also: impl Iterator<Item = &'a str>,
    ) -> Result<usize> {
        let used: HashSet<&str> = profile
            .image_paths()
            .chain(also)
            .filter_map(image_name)
            .collect();
        let dir = self.dir.join(IMAGE_DIR);
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(io_error("cannot read", &dir, e)),
        };
        let mut removed = 0;
        for entry in entries {
            let entry = entry.map_err(|e| io_error("cannot read", &dir, e))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if is_copy_name(name) && !used.contains(name) {
                let path = entry.path();
                fs::remove_file(&path).map_err(|e| io_error("cannot remove", &path, e))?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Copy each image file that `profile` uses from the profile directory of
    /// `source` into this profile directory. An image that is missing in
    /// `source` is not copied: the surface shows no image there either.
    pub fn copy_images_from(&self, source: &ProfileStore, profile: &Profile) -> Result<()> {
        let names: HashSet<&str> = profile.image_paths().filter_map(image_name).collect();
        let dir = self.dir.join(IMAGE_DIR);
        for name in names {
            let from = source.dir.join(IMAGE_DIR).join(name);
            if !from.exists() {
                continue;
            }
            self.make_dir(&dir)?;
            let to = dir.join(name);
            fs::copy(&from, &to).map_err(|e| io_error("cannot copy", &from, e))?;
        }
        Ok(())
    }

    fn make_dir(&self, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir).map_err(|e| io_error("cannot make the directory", dir, e))
    }
}

/// Decode the image file `source` of the user.
fn decode_file(source: &Path) -> Result<DynamicImage> {
    ImageReader::open(source)
        .map_err(|e| io_error("cannot read", source, e))?
        .with_guessed_format()
        .map_err(|e| io_error("cannot read", source, e))?
        .decode()
        .map_err(|e| Error::Image {
            path: source.to_path_buf(),
            source: e,
        })
}

/// The size of the background copy of an image of `width` × `height`
/// pixels, for a background of the size `size`: see
/// [`ProfileStore::import_background`]. The copy has its proportion, except
/// for the change to whole pixels.
pub(crate) fn background_copy_size(width: u32, height: u32, size: (u32, u32)) -> (u32, u32) {
    let (target_width, target_height) = (size.0.max(1), size.1.max(1));
    if width == 0 || height == 0 {
        return (width, height);
    }
    let (w, h) = (f64::from(width), f64::from(height));
    // The scale that covers the target, and no larger than the image.
    let cover = (f64::from(target_width) / w)
        .max(f64::from(target_height) / h)
        .min(1.0);
    // The limit on each side.
    let limit = (f64::from(target_width * BACKGROUND_LIMIT) / w)
        .min(f64::from(target_height * BACKGROUND_LIMIT) / h);
    let scale = cover.min(limit);
    if scale >= 1.0 {
        return (width, height);
    }
    let side = |v: f64| ((v * scale).round() as u32).max(1);
    (side(w), side(h))
}

pub(crate) fn io_error(action: &'static str, path: &Path, source: io::Error) -> Error {
    Error::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// Write `data` to a temporary file next to `path`, then rename it to `path`.
/// A reader sees the old file or the new file, and not a part of a file.
pub(crate) fn write_and_rename(path: &Path, data: &[u8]) -> Result<()> {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let temporary = path.with_file_name(name);
    let write = || -> io::Result<()> {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    };
    write().map_err(|e| {
        // The temporary file has no use after an error.
        let _ = fs::remove_file(&temporary);
        io_error("cannot write", path, e)
    })
}

/// The file name of an image path of the profile: `images/<name>`, where the
/// name has the form of an image copy. `None` for each other path, for
/// example a path that goes out of the image directory.
pub(crate) fn image_name(relative: &str) -> Option<&str> {
    let name = relative.strip_prefix(IMAGE_DIR)?.strip_prefix('/')?;
    is_copy_name(name).then_some(name)
}

/// True if `name` has the form that [`ProfileStore::import_image`] makes: 16
/// hexadecimal digits and `.png`.
fn is_copy_name(name: &str) -> bool {
    name.strip_suffix(".png").is_some_and(|stem| {
        stem.len() == 16
            && stem
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// The FNV-1a hash, 64 bits. The name of an image copy comes from it. The
/// value does not change with the version of the compiler.
fn fnv1a_64(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

// --- The file format -----------------------------------------------------------

/// The part of each schema version that tells the version.
#[derive(Deserialize)]
struct Header {
    schema_version: u32,
}

/// The fields of schema versions 1 to 5. Version 3 adds `model`, version 4
/// adds the label options of a surface, and version 5 adds `folders` and
/// the links.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileData {
    schema_version: u32,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    /// The colour of the profile. It came in version 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    /// The number of the first page, from 1. It came in version 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    first_page: Option<usize>,
    /// The links. The key came in version 5. `None`: the file has no key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    applications: Option<Vec<ApplicationData>>,
    /// The default title style of the profile. It came in version 8. The
    /// field is before `pages`, thus the serializer writes the table before
    /// the array of the page tables (decision 0043, section 7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_label_style: Option<DefaultLabelStyleData>,
    #[serde(default)]
    pages: Vec<PageData>,
    /// The folders. The key came in version 5. `None` means the key is not
    /// in the file. An empty array is `Some` and is an error in an older file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    folders: Option<Vec<FolderData>>,
}

/// One folder in the profile file. It has key faces and keys only
/// (decision 0036).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FolderData {
    id: String,
    name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    surfaces: BTreeMap<String, SurfaceData>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    bindings: BTreeMap<String, BTreeMap<GestureKind, Action>>,
}

/// One link in the profile file.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplicationData {
    bundle_id: String,
    name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageData {
    /// The identity of the page. The key came in version 6. `None` means the
    /// key is not in the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    name: String,
    /// The background of the page. It came in version 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    background: Option<BackgroundData>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    surfaces: BTreeMap<String, SurfaceData>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    bindings: BTreeMap<String, BTreeMap<GestureKind, Action>>,
}

/// The background of one page: a colour, or an image with its fit.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackgroundData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fit: Option<String>,
}

impl BackgroundData {
    /// The background of the file data. The error text names the key.
    fn background(&self) -> std::result::Result<Background, String> {
        match (&self.color, &self.image) {
            (Some(text), None) => {
                if self.fit.is_some() {
                    return Err("a colour has no fit".to_string());
                }
                text.parse::<Color>()
                    .map(Background::Color)
                    .map_err(|e| format!("color '{text}': {e}"))
            }
            (None, Some(image)) => {
                if image_name(image).is_none() {
                    return Err(format!("'{image}' is not in the image directory"));
                }
                let fit = match &self.fit {
                    None => BackgroundFit::default(),
                    Some(text) => BackgroundFit::from_key(text)
                        .ok_or_else(|| format!("fit '{text}' is not a fit"))?,
                };
                Ok(Background::Image {
                    image: image.clone(),
                    fit,
                })
            }
            _ => Err("a background has a colour or an image".to_string()),
        }
    }

    fn from_background(background: &Background) -> Self {
        match background {
            Background::Color(color) => Self {
                color: Some(color.to_string()),
                image: None,
                fit: None,
            },
            Background::Image { image, fit } => Self {
                color: None,
                image: Some(image.clone()),
                fit: (*fit != BackgroundFit::default()).then(|| fit.key().to_string()),
            },
        }
    }
}

/// The table `[default_label_style]` of a profile file. The keys have no
/// `label_` prefix (decision 0043, section 2). An absent key is the engine
/// default, and an absent table is a profile with no own value.
#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct DefaultLabelStyleData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    position: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    size: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    underline: Option<bool>,
}

impl DefaultLabelStyleData {
    /// The style of the table. An empty table is the empty style.
    fn style(&self) -> std::result::Result<LabelStyle, String> {
        let font = match &self.font {
            None => None,
            Some(name) if name == SYSTEM_FONT_WORD => Some(Auto::Auto),
            Some(name) if name.trim().is_empty() => {
                return Err("default_label_style.font is empty".to_string())
            }
            Some(name) => Some(Auto::Value(name.clone())),
        };
        let position = match &self.position {
            None => None,
            Some(text) if text == AUTO_WORD => Some(Auto::Auto),
            Some(text) => Some(Auto::Value(LabelPosition::from_key(text).ok_or_else(
                || format!("default_label_style.position '{text}' is not a position"),
            )?)),
        };
        let size = match self.size {
            None => None,
            Some(size) if (MIN_LABEL_SIZE..=MAX_LABEL_SIZE).contains(&size) => Some(size),
            Some(size) => {
                return Err(format!(
                "default_label_style.size {size} is not from {MIN_LABEL_SIZE} to {MAX_LABEL_SIZE}"
            ))
            }
        };
        let color = match &self.color {
            None => None,
            Some(text) if text == AUTO_WORD => Some(Auto::Auto),
            Some(text) => {
                Some(Auto::Value(text.parse::<Color>().map_err(|e| {
                    format!("default_label_style.color '{text}': {e}")
                })?))
            }
        };
        Ok(LabelStyle {
            font,
            position,
            size,
            color,
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
        })
    }

    /// The table of `style`. The caller writes no table for the empty style.
    fn from_style(style: &LabelStyle) -> Self {
        Self {
            font: style.font.as_ref().map(|font| match font {
                Auto::Auto => SYSTEM_FONT_WORD.to_string(),
                Auto::Value(name) => name.clone(),
            }),
            position: style.position.as_ref().map(|position| match position {
                Auto::Auto => AUTO_WORD.to_string(),
                Auto::Value(position) => position.key().to_string(),
            }),
            size: style.size,
            color: style.color.as_ref().map(|color| match color {
                Auto::Auto => AUTO_WORD.to_string(),
                Auto::Value(color) => color.to_string(),
            }),
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
        }
    }
}

/// The content of one surface. The keys that start with `label_` and the
/// keys `icon` and `icon_color` came in version 4.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label_position: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label_size: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label_bold: Option<bool>,
    /// The font family, or `"system"`. It came in version 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label_font: Option<String>,
    /// It came in version 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label_italic: Option<bool>,
    /// It came in version 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label_underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon_color: Option<String>,
    /// The zoom of the image in percent. It came in version 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image_zoom: Option<u16>,
    /// The pan of the image on the x axis, in per mille of the overflow. It
    /// came in version 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image_offset_x: Option<i16>,
    /// The pan of the image on the y axis, in per mille of the overflow. It
    /// came in version 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image_offset_y: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    on: Option<Box<SurfaceData>>,
}

impl SurfaceData {
    /// True if the data has a key of version 4.
    fn has_version_4_keys(&self) -> bool {
        self.label_position.is_some()
            || self.label_size.is_some()
            || self.label_color.is_some()
            || self.label_bold.is_some()
            || self.icon.is_some()
            || self.icon_color.is_some()
    }

    /// True if the data has a key of schema version 8: a label key, or the
    /// crop of the image.
    fn has_version_8_keys(&self) -> bool {
        self.label_font.is_some()
            || self.label_italic.is_some()
            || self.label_underline.is_some()
            || self.image_zoom.is_some()
            || self.image_offset_x.is_some()
            || self.image_offset_y.is_some()
    }

    /// The first key of the crop of the image in the data, for the error
    /// text. The caller checked that the data has one.
    fn first_image_crop_key(&self) -> &'static str {
        if self.image_zoom.is_some() {
            "image_zoom"
        } else if self.image_offset_x.is_some() {
            "image_offset_x"
        } else {
            "image_offset_y"
        }
    }

    /// The crop of the image of the file data, if the data has a key of it.
    /// An absent key is the value of the fit: zoom 100 and no pan. The
    /// error text names the key (decision 0043).
    fn image_crop(&self, version: u32) -> std::result::Result<Option<ImageCrop>, String> {
        let has_key = self.image_zoom.is_some()
            || self.image_offset_x.is_some()
            || self.image_offset_y.is_some();
        if !has_key {
            return Ok(None);
        }
        let key = self.first_image_crop_key();
        if self.image.is_none() {
            return Err(format!("{key} needs an image"));
        }
        faces_key(version, key)?;
        let zoom = self.image_zoom.unwrap_or(MIN_IMAGE_ZOOM);
        if !(MIN_IMAGE_ZOOM..=MAX_IMAGE_ZOOM).contains(&zoom) {
            return Err(format!(
                "image_zoom {zoom} is not from {MIN_IMAGE_ZOOM} to {MAX_IMAGE_ZOOM}"
            ));
        }
        let x = self.image_offset_x.unwrap_or(0);
        let y = self.image_offset_y.unwrap_or(0);
        for (name, value) in [("image_offset_x", x), ("image_offset_y", y)] {
            if !(-MAX_IMAGE_PAN..=MAX_IMAGE_PAN).contains(&value) {
                return Err(format!(
                    "{name} {value} is not from {} to {MAX_IMAGE_PAN}",
                    -MAX_IMAGE_PAN
                ));
            }
        }
        let crop = ImageCrop { zoom, x, y };
        // The fit is no crop: the file then has no keys.
        Ok((!crop.is_fit()).then_some(crop))
    }

    /// The icon of the file data. The error text names the key.
    fn icon(&self) -> std::result::Result<Option<SurfaceIcon>, String> {
        let color = match &self.icon_color {
            None => None,
            Some(text) => Some(
                text.parse::<Color>()
                    .map_err(|e| format!("icon_color '{text}': {e}"))?,
            ),
        };
        match &self.icon {
            None if color.is_some() => Err("icon_color with no icon".to_string()),
            None => Ok(None),
            Some(name) if !SurfaceIcon::is_name(name) => {
                Err(format!("icon '{name}' is not an icon name"))
            }
            Some(_) if self.image.is_some() => Err("an image and an icon".to_string()),
            Some(name) => Ok(Some(SurfaceIcon {
                name: name.clone(),
                color,
            })),
        }
    }

    /// The label style of the file data. The error text names the key. A
    /// label key of schema version 8 in a file of version 1 to 7 is an
    /// error (decision 0043).
    fn label_style(&self, version: u32) -> std::result::Result<LabelStyle, String> {
        let position = match &self.label_position {
            None => None,
            Some(text) if text == AUTO_WORD => {
                faces_key(version, "label_position")?;
                Some(Auto::Auto)
            }
            Some(text) => Some(Auto::Value(
                LabelPosition::from_key(text)
                    .ok_or_else(|| format!("label_position '{text}' is not a position"))?,
            )),
        };
        let max = if version < FACES_VERSION {
            MAX_LABEL_SIZE_BEFORE_FACES
        } else {
            MAX_LABEL_SIZE
        };
        let size = match self.label_size {
            None => None,
            Some(size) if (MIN_LABEL_SIZE..=max).contains(&size) => Some(size),
            Some(size) => {
                return Err(format!(
                    "label_size {size} is not from {MIN_LABEL_SIZE} to {max}"
                ))
            }
        };
        let color = match &self.label_color {
            None => None,
            Some(text) if text == AUTO_WORD => {
                faces_key(version, "label_color")?;
                Some(Auto::Auto)
            }
            Some(text) => Some(Auto::Value(
                text.parse::<Color>()
                    .map_err(|e| format!("label_color '{text}': {e}"))?,
            )),
        };
        let font = match &self.label_font {
            None => None,
            Some(name) => {
                faces_key(version, "label_font")?;
                if name == SYSTEM_FONT_WORD {
                    Some(Auto::Auto)
                } else if name.trim().is_empty() {
                    return Err("label_font is empty".to_string());
                } else {
                    Some(Auto::Value(name.clone()))
                }
            }
        };
        let italic = self.label_italic;
        if italic.is_some() {
            faces_key(version, "label_italic")?;
        }
        let underline = self.label_underline;
        if underline.is_some() {
            faces_key(version, "label_underline")?;
        }
        Ok(LabelStyle {
            font,
            position,
            size,
            color,
            bold: self.label_bold,
            italic,
            underline,
        })
    }

    fn from_content(content: &SurfaceContent) -> Self {
        Self {
            on: content
                .on
                .as_deref()
                .map(|face| Box::new(Self::from_face(face))),
            ..Self::from_face(content)
        }
    }

    /// The data of one face. The engine writes a key when the face has an
    /// own value, also when the value is the engine default (decision 0043).
    fn from_face(content: &SurfaceContent) -> Self {
        let style = &content.label_style;
        // The fit writes no keys, as a face with no image has no crop.
        let crop = content.image_crop.filter(|crop| !crop.is_fit());
        Self {
            label: content.label.clone(),
            label_position: style.position.as_ref().map(|position| match position {
                Auto::Auto => AUTO_WORD.to_string(),
                Auto::Value(position) => position.key().to_string(),
            }),
            label_size: style.size,
            label_color: style.color.as_ref().map(|color| match color {
                Auto::Auto => AUTO_WORD.to_string(),
                Auto::Value(color) => color.to_string(),
            }),
            label_bold: style.bold,
            label_font: style.font.as_ref().map(|font| match font {
                Auto::Auto => SYSTEM_FONT_WORD.to_string(),
                Auto::Value(name) => name.clone(),
            }),
            label_italic: style.italic,
            label_underline: style.underline,
            color: content.color.map(|c| c.to_string()),
            image: content.image.clone(),
            icon: content.icon.as_ref().map(|i| i.name.clone()),
            icon_color: content
                .icon
                .as_ref()
                .and_then(|i| i.color)
                .map(|c| c.to_string()),
            image_zoom: crop.map(|crop| crop.zoom),
            // An absent pan is 0: it writes no key.
            image_offset_x: crop.filter(|crop| crop.x != 0).map(|crop| crop.x),
            image_offset_y: crop.filter(|crop| crop.y != 0).map(|crop| crop.y),
            on: None,
        }
    }
}

/// The content of one surface in the file. The error text names the key.
fn content_of(
    key: &str,
    surface: SurfaceData,
    version: u32,
) -> std::result::Result<(opencraft_core::SurfaceId, SurfaceContent), String> {
    let id = parse_surface_key(key).ok_or_else(|| format!("'{key}' is not a surface"))?;
    let on = if let Some(face) = surface.on {
        if version < STATE_FACES_VERSION {
            return Err(format!("{key}: version {version} has no on face"));
        }
        if face.on.is_some() {
            return Err(format!("{key}.on: an on face cannot have an on face"));
        }
        Some(Box::new(face_of(&format!("{key}.on"), *face, version)?))
    } else {
        None
    };
    let mut content = face_of(
        key,
        SurfaceData {
            on: None,
            ..surface
        },
        version,
    )?;
    content.on = on;
    Ok((id, content))
}

/// An error if `version` has no label key of schema version 8. `key` names
/// the key in the error text (decision 0043).
fn faces_key(version: u32, key: &str) -> std::result::Result<(), String> {
    if version < FACES_VERSION {
        return Err(format!("{key} requires schema version {FACES_VERSION}"));
    }
    Ok(())
}

fn face_of(
    key: &str,
    surface: SurfaceData,
    version: u32,
) -> std::result::Result<SurfaceContent, String> {
    let color = match &surface.color {
        Some(text) => Some(
            text.parse::<Color>()
                .map_err(|e| format!("{key}: '{text}': {e}"))?,
        ),
        None => None,
    };
    if let Some(image) = &surface.image {
        if image_name(image).is_none() {
            return Err(format!("{key}: '{image}' is not in the image directory"));
        }
    }
    if version < LABEL_OPTIONS_VERSION
        && (surface.has_version_4_keys() || surface.has_version_8_keys())
    {
        return Err(format!(
            "{key}: version {version} has no label options and no icon"
        ));
    }
    let label_style = surface
        .label_style(version)
        .map_err(|e| format!("{key}: {e}"))?;
    let icon = surface.icon().map_err(|e| format!("{key}: {e}"))?;
    let image_crop = surface
        .image_crop(version)
        .map_err(|e| format!("{key}: {e}"))?;
    Ok(SurfaceContent {
        label: surface.label.filter(|s| !s.is_empty()),
        label_style,
        color,
        image: surface.image,
        icon,
        image_crop,
        on: None,
    })
}

/// Put the bindings of the file on `page`. With `keys_only`, a binding of a
/// different kind is an error: a folder has keys only.
fn apply_bindings(
    page: &mut Page,
    bindings: BTreeMap<String, BTreeMap<GestureKind, Action>>,
    keys_only: bool,
    version: u32,
) -> std::result::Result<(), String> {
    for (key, gestures) in bindings {
        let control = parse_control_key(&key).ok_or_else(|| format!("'{key}' is not a control"))?;
        if keys_only && control.kind != ControlKind::Key {
            return Err(format!("'{key}' is not a key"));
        }
        for (gesture, action) in gestures {
            if version < GESTURE_TIMERS_VERSION {
                let name = match gesture {
                    GestureKind::LongPress => Some("long-press"),
                    GestureKind::DoublePress => Some("double-press"),
                    _ => None,
                };
                if let Some(name) = name {
                    return Err(format!(
                        "gesture '{name}' requires schema version {GESTURE_TIMERS_VERSION}"
                    ));
                }
            }
            page.set_binding(control, gesture, Some(action));
        }
    }
    Ok(())
}

/// The bindings of `page` in the form of the file.
fn binding_data(page: &Page) -> BTreeMap<String, BTreeMap<GestureKind, Action>> {
    let mut bindings: BTreeMap<String, BTreeMap<GestureKind, Action>> = BTreeMap::new();
    for ((control, gesture), action) in page.bindings() {
        bindings
            .entry(control_key(control))
            .or_default()
            .insert(gesture, action.clone());
    }
    bindings
}

/// Read a profile from the text of a profile file, for example a starter
/// profile of a shell (decision 0029, section 18). The text has the format
/// of this module. Image paths are not checked against a directory.
pub fn profile_from_text(text: &str) -> std::result::Result<Profile, BackupReason> {
    parse(text)
}

/// Read the text of a profile file.
pub(crate) fn parse(text: &str) -> std::result::Result<Profile, BackupReason> {
    let invalid = |message: String| BackupReason::Invalid(message);
    let header: Header = toml::from_str(text).map_err(|e| invalid(e.to_string()))?;
    match header.schema_version {
        1..=SCHEMA_VERSION => {}
        0 => return Err(invalid("schema version 0 does not exist".to_string())),
        found => return Err(BackupReason::NewerSchema { found }),
    }
    let version = header.schema_version;
    let file: FileData = toml::from_str(text).map_err(|e| invalid(e.to_string()))?;
    let model = match (version >= MODEL_VERSION, file.model) {
        (true, Some(model)) if !model.is_empty() => model,
        (true, _) => return Err(invalid("the file has no model".to_string())),
        (false, None) => default_model().to_string(),
        (false, Some(_)) => return Err(invalid(format!("version {version} has no model"))),
    };
    if version < LABEL_OPTIONS_VERSION && (file.color.is_some() || file.first_page.is_some()) {
        return Err(invalid(format!(
            "version {version} has no colour and no first page"
        )));
    }
    let color = match &file.color {
        Some(text) => Some(
            text.parse::<Color>()
                .map_err(|e| invalid(format!("color '{text}': {e}")))?,
        ),
        None => None,
    };
    let page_count = file.pages.len().max(1);
    let first_page = match file.first_page {
        None => None,
        Some(n) if (1..=page_count).contains(&n) => Some(n - 1),
        Some(n) => {
            return Err(invalid(format!(
                "first_page {n} is not a page from 1 to {page_count}"
            )))
        }
    };

    let mut pages = Vec::new();
    let mut seen_pages = HashSet::new();
    for (index, page_file) in file.pages.into_iter().enumerate() {
        let id = match (version >= PAGE_IDS_VERSION, &page_file.id) {
            (true, Some(text)) => text.parse::<PageId>().map_err(|_| {
                invalid(format!(
                    "page identity '{text}' is not 16 hexadecimal digits"
                ))
            })?,
            (true, None) => {
                return Err(invalid(format!(
                    "page '{}' has no identity",
                    page_file.name
                )))
            }
            (false, None) => PageId::of_old_page(index),
            (false, Some(_)) => {
                return Err(invalid(format!(
                    "page '{}': version {version} has no page identity",
                    page_file.name
                )))
            }
        };
        if !seen_pages.insert(id) {
            return Err(invalid(format!("the page '{id}' is in the file two times")));
        }
        let mut page = Page::new(page_file.name).with_id(id);
        if let Some(background) = &page_file.background {
            if version < LABEL_OPTIONS_VERSION {
                return Err(invalid(format!(
                    "page '{}': version {version} has no background",
                    page.name
                )));
            }
            page.background = Some(
                background
                    .background()
                    .map_err(|e| invalid(format!("page '{}': background: {e}", page.name)))?,
            );
        }
        for (key, surface) in page_file.surfaces {
            let (id, content) = content_of(&key, surface, version).map_err(invalid)?;
            page.update(id, |slot| *slot = content);
        }
        if header.schema_version == 1 && !page_file.bindings.is_empty() {
            return Err(invalid("version 1 has no bindings".into()));
        }
        apply_bindings(&mut page, page_file.bindings, false, version).map_err(invalid)?;
        pages.push(page);
    }
    if version < FOLDERS_VERSION && file.folders.is_some() {
        return Err(invalid(format!("version {version} has no folders")));
    }
    let mut folders = Vec::new();
    let mut seen = HashSet::new();
    for folder_file in file.folders.unwrap_or_default() {
        let id = folder_file.id.parse::<FolderId>().map_err(|_| {
            invalid(format!(
                "folder identity '{}' is not 16 hexadecimal digits",
                folder_file.id
            ))
        })?;
        if !seen.insert(id) {
            return Err(invalid(format!(
                "the folder '{}' is in the file two times",
                folder_file.id
            )));
        }
        if folder_file.name.trim().is_empty() {
            return Err(invalid("a folder must have a name".into()));
        }
        let mut page = Page::new(folder_file.name);
        for (key, surface) in folder_file.surfaces {
            let (id, content) = content_of(&key, surface, version).map_err(invalid)?;
            if id.kind != SurfaceKind::KeyFace {
                return Err(invalid(format!(
                    "folder '{}': '{key}' is not a key face",
                    page.name
                )));
            }
            page.update(id, |slot| *slot = content);
        }
        apply_bindings(&mut page, folder_file.bindings, true, version)
            .map_err(|e| invalid(format!("folder '{}': {e}", page.name)))?;
        folders.push(Folder { id, page });
    }
    let mut profile = Profile::for_model(model, file.name, pages);
    profile.color = color;
    profile.applications = check_links(version, file.applications).map_err(invalid)?;
    if let Some(table) = file.default_label_style {
        if version < FACES_VERSION {
            return Err(invalid(format!(
                "version {version} has no default title style"
            )));
        }
        profile.default_label_style = table.style().map_err(invalid)?;
    }
    if let Some(first) = first_page {
        profile.mark_first_page(first);
    }
    profile.replace_folders(folders);
    Ok(profile)
}

/// The links of the file, after one check.
///
/// `links` is `None` when the file has no `applications` key. A file of a
/// version before the links then has none. A key in an older file, a link
/// that is not correct, a link that is two times in the file, or more than
/// the maximum count gives an error. The full read and the summary both
/// use this function.
fn check_links(
    version: u32,
    links: Option<Vec<ApplicationData>>,
) -> std::result::Result<Vec<ApplicationLink>, String> {
    let Some(links) = links else {
        return Ok(Vec::new());
    };
    if version < APPLICATIONS_VERSION {
        return Err(format!("version {version} has no application links"));
    }
    if links.len() > MAX_LINKS {
        return Err(format!("a profile can have {MAX_LINKS} links"));
    }
    let mut seen: Vec<String> = Vec::new();
    let mut applications = Vec::new();
    for link in links {
        if !valid_bundle_id(&link.bundle_id) || link.name.is_empty() {
            return Err(format!("the link '{}' is not correct", link.bundle_id));
        }
        if seen.iter().any(|id| same_bundle(id, &link.bundle_id)) {
            return Err(format!(
                "the link '{}' is in the profile two times",
                link.bundle_id
            ));
        }
        seen.push(link.bundle_id.clone());
        applications.push(ApplicationLink {
            bundle_id: link.bundle_id,
            name: link.name,
        });
    }
    Ok(applications)
}

/// The links of a summary table. This uses [`check_links`], the same check
/// as the full read.
fn applications_of(table: &toml::Table) -> std::result::Result<Vec<ApplicationLink>, String> {
    let Some(value) = table.get("applications") else {
        return Ok(Vec::new());
    };
    let version = table
        .get("schema_version")
        .and_then(toml::Value::as_integer)
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| "the schema version is not correct".to_string())?;
    let links =
        Vec::<ApplicationData>::deserialize(value.clone()).map_err(|error| error.to_string())?;
    check_links(version, Some(links))
}

/// The default title style of a summary table. A schema version before 8
/// has no table, and an invalid table gives the empty style: the full read
/// reports the error.
fn default_label_style_of(table: &toml::Table) -> LabelStyle {
    let version = table
        .get("schema_version")
        .and_then(toml::Value::as_integer)
        .and_then(|number| u32::try_from(number).ok())
        .unwrap_or_default();
    if version < FACES_VERSION {
        return LabelStyle::default();
    }
    table
        .get("default_label_style")
        .and_then(|value| DefaultLabelStyleData::deserialize(value.clone()).ok())
        .and_then(|table| table.style().ok())
        .unwrap_or_default()
}

/// The text of the profile file for `profile`.
pub(crate) fn to_text(profile: &Profile) -> Result<String> {
    let file = FileData {
        schema_version: SCHEMA_VERSION,
        name: profile.name.clone(),
        model: Some(profile.model.clone()),
        color: profile.color.map(|c| c.to_string()),
        first_page: profile.has_first_page().then(|| profile.first_page() + 1),
        applications: (!profile.applications.is_empty()).then(|| {
            profile
                .applications
                .iter()
                .map(|link| ApplicationData {
                    bundle_id: link.bundle_id.clone(),
                    name: link.name.clone(),
                })
                .collect()
        }),
        default_label_style: (profile.default_label_style != LabelStyle::default())
            .then(|| DefaultLabelStyleData::from_style(&profile.default_label_style)),
        pages: profile
            .pages()
            .iter()
            .map(|page| PageData {
                id: Some(page.id().to_string()),
                name: page.name.clone(),
                background: page
                    .background
                    .as_ref()
                    .map(BackgroundData::from_background),
                bindings: binding_data(page),
                surfaces: page
                    .surfaces()
                    .map(|(id, content)| (surface_key(id), SurfaceData::from_content(content)))
                    .collect(),
            })
            .collect(),
        folders: {
            let folders: Vec<FolderData> = profile
                .folders()
                .iter()
                .map(|folder| FolderData {
                    id: folder.id.to_string(),
                    name: folder.page.name.clone(),
                    surfaces: folder
                        .page
                        .surfaces()
                        .map(|(id, content)| (surface_key(id), SurfaceData::from_content(content)))
                        .collect(),
                    bindings: binding_data(&folder.page),
                })
                .collect();
            (!folders.is_empty()).then_some(folders)
        },
    };
    Ok(toml::to_string_pretty(&file)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{LabelChange, DEFAULT_LABEL_SIZE};
    use image::{Rgba, RgbaImage};
    use opencraft_core::{SurfaceId, SurfaceKind};
    use tempfile::TempDir;

    const KEY_3: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 3);
    const BAR_1: SurfaceId = SurfaceId::new(SurfaceKind::BarPanel, 1);

    fn store() -> (TempDir, ProfileStore) {
        let dir = TempDir::new().unwrap();
        let store = ProfileStore::new(dir.path().join("profile"));
        (dir, store)
    }

    /// A profile with each part of the content in use.
    fn sample() -> Profile {
        let mut profile = Profile::default();
        let page = profile.page_mut(0).unwrap();
        page.update(KEY_3, |c| {
            c.label = Some("Mail".into());
            c.color = Some(Color::rgb(0x1E, 0x6F, 0xD9));
            c.image = Some("images/4f1c2a9be07d3356.png".into());
        });
        page.update(BAR_1, |c| c.label = Some("Volume \"1\"\nline 2".into()));
        profile
    }

    /// Write an image file of `width` × `height` and return its path.
    fn image_file(dir: &Path, name: &str, width: u32, height: u32) -> PathBuf {
        let image = RgbaImage::from_fn(width, height, |x, y| {
            Rgba([
                (x % 256) as u8,
                (y % 256) as u8,
                7,
                if x < 2 { 0 } else { 255 },
            ])
        });
        let path = dir.join(name);
        image.save(&path).unwrap();
        path
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn with_no_file_the_profile_is_new_and_nothing_is_written() {
        let (_dir, store) = store();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.profile, Profile::default());
        assert_eq!(loaded.backup, None);
        assert!(!store.dir().exists());
    }

    #[test]
    fn a_saved_profile_loads_again_with_the_same_content() {
        let (_dir, store) = store();
        store.save(&sample()).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.profile, sample());
        assert_eq!(loaded.backup, None);
        // The write leaves no temporary file.
        assert_eq!(files(store.dir()), [PROFILE_FILE]);
    }

    #[test]
    fn the_file_is_readable_text_with_the_schema_version() {
        let (_dir, store) = store();
        let profile = sample();
        let id = profile.pages()[0].id();
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        let want = r##"schema_version = 8
name = "Default"
model = "n4-pro"

[[pages]]
id = "ID"
name = "Page 1"

[pages.surfaces.bar-panel-1]
label = """
Volume "1"
line 2"""

[pages.surfaces.key-face-3]
label = "Mail"
color = "#1E6FD9"
image = "images/4f1c2a9be07d3356.png"
"##
        .replace("ID", &id.to_string());
        assert_eq!(text, want);
    }

    #[test]
    fn touch_gestures_save_with_their_text_form_and_load_again() {
        use opencraft_core::{ControlId, ControlKind};
        let (_dir, store) = store();
        let mut profile = Profile::default();
        let page = profile.page_mut(0).unwrap();
        let bar = ControlId::new(ControlKind::BarPanel, 2);
        let touch_bar = ControlId::new(ControlKind::TouchBar, 1);
        for (control, gesture) in [
            (bar, GestureKind::Tap),
            (touch_bar, GestureKind::SwipeLeft),
            (touch_bar, GestureKind::SwipeRight),
        ] {
            let action = Action {
                kind: format!("test.{gesture:?}"),
                settings: Default::default(),
            };
            page.set_binding(control, gesture, Some(action));
        }
        store.save(&profile).unwrap();

        let text = fs::read_to_string(store.profile_path()).unwrap();
        for table in [
            "[pages.bindings.bar-panel-2.tap]",
            "[pages.bindings.touch-bar-1.swipe-left]",
            "[pages.bindings.touch-bar-1.swipe-right]",
        ] {
            assert!(text.contains(table), "{table} in {text}");
        }
        assert_eq!(store.load().unwrap().profile, profile);
    }

    #[test]
    fn many_pages_save_and_load_in_their_order() {
        let (_dir, store) = store();
        let mut profile = sample();
        let mut second = Page::new("Media");
        second.update(BAR_1, |c| c.color = Some(Color::rgb(1, 2, 3)));
        profile.add_page(second);
        profile.add_page(Page::new("Empty"));
        store.save(&profile).unwrap();
        let loaded = store.load().unwrap().profile;
        assert_eq!(loaded, profile);
        let names: Vec<_> = loaded.pages().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Page 1", "Media", "Empty"]);
    }

    #[test]
    fn a_new_profile_saves_and_loads() {
        let (_dir, store) = store();
        store.save(&Profile::default()).unwrap();
        assert_eq!(store.load().unwrap().profile, Profile::default());
    }

    #[test]
    fn a_save_replaces_the_file_and_keeps_other_files() {
        let (_dir, store) = store();
        store.save(&sample()).unwrap();
        fs::write(store.dir().join("note.txt"), "keep").unwrap();
        store.save(&Profile::default()).unwrap();
        assert_eq!(store.load().unwrap().profile, Profile::default());
        assert_eq!(files(store.dir()), ["note.txt", PROFILE_FILE]);
    }

    /// Put `text` in the profile file, load, and return the result.
    fn load_text(store: &ProfileStore, text: &[u8]) -> Loaded {
        fs::create_dir_all(store.dir()).unwrap();
        fs::write(store.profile_path(), text).unwrap();
        store.load().unwrap()
    }

    #[test]
    fn an_icon_saves_and_loads_and_an_unknown_icon_name_stays() {
        let (_dir, store) = store();
        let mut profile = sample();
        profile.page_mut(0).unwrap().update(BAR_1, |c| {
            c.icon = Some(SurfaceIcon {
                name: "volume-2".into(),
                color: Some(Color::rgb(0xF5, 0x9E, 0x0B)),
            })
        });
        // A later icon set can have a name that this set does not have.
        profile
            .page_mut(0)
            .unwrap()
            .update(SurfaceId::new(SurfaceKind::KeyFace, 1), |c| {
                c.icon = Some(SurfaceIcon::new("icon-of-a-later-set"))
            });
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            text.contains("icon = \"volume-2\"\nicon_color = \"#F59E0B\"\n"),
            "{text}"
        );
        assert!(text.contains("icon = \"icon-of-a-later-set\"\n"), "{text}");
        assert_eq!(store.load().unwrap().profile, profile);
    }

    fn old_profile_text(version: u32) -> String {
        let model = if version >= 3 {
            "model = \"n4-pro\"\n"
        } else {
            ""
        };
        let id = if version >= 6 {
            "id = \"000000000000002a\"\n"
        } else {
            ""
        };
        let binding = if version >= 2 {
            "[pages.bindings.key-3.press]\nkind = \"test.press\"\n"
        } else {
            ""
        };
        format!(
            "schema_version = {version}\nname = \"Old\"\n{model}[[pages]]\n{id}name = \"Main\"\n\
            [pages.surfaces.key-face-3]\nlabel = \"Mail\"\n{binding}"
        )
    }

    #[test]
    fn each_old_version_keeps_its_content_when_saved_again() {
        let key = opencraft_core::ControlId::new(ControlKind::Key, 3);
        for version in 1..=6 {
            let (_dir, store) = store();
            let mut text = old_profile_text(version);
            if version >= 5 {
                text.push_str("[[folders]]\nid = \"0000000000000002\"\nname = \"More\"\n");
            }
            let loaded = load_text(&store, text.as_bytes());
            assert!(
                loaded.backup.is_none(),
                "version {version}: {:?}",
                loaded.backup
            );
            let profile = loaded.profile;
            let page = &profile.pages()[0];
            assert_eq!(page.name, "Main");
            assert_eq!(
                page.id().to_string(),
                if version >= 6 {
                    "000000000000002a"
                } else {
                    "0000000000000001"
                }
            );
            assert_eq!(page.content(KEY_3).label.as_deref(), Some("Mail"));
            assert_eq!(
                page.binding(key, GestureKind::Press)
                    .map(|action| action.kind.as_str()),
                (version >= 2).then_some("test.press")
            );
            assert_eq!(profile.model, "n4-pro");
            assert_eq!(profile.folders().len(), usize::from(version >= 5));
            store.save(&profile).unwrap();
            assert_eq!(store.load().unwrap().profile, profile);
        }
    }

    #[test]
    fn gestures_and_on_faces_round_trip_on_pages_and_folders_in_version_seven() {
        let (_dir, store) = store();
        let mut text = old_profile_text(7);
        for holder in ["pages", "folders"] {
            if holder == "folders" {
                text.push_str("[[folders]]\nid = \"0000000000000002\"\nname = \"More\"\n");
            }
            text.push_str(&format!(
                r##"
[{holder}.surfaces.key-face-1.on]
label = "Live"
label_position = "top"
label_size = 24
label_color = "#FF0000"
label_bold = false
color = "#223344"
"##
            ));
            if holder == "pages" {
                text.push_str("image = \"images/0123456789abcdef.png\"\n");
            } else {
                text.push_str("icon = \"star\"\nicon_color = \"#AABBCC\"\n");
            }
            for gesture in ["long-press", "double-press"] {
                text.push_str(&format!(
                    "[{holder}.bindings.key-3.{gesture}]\nkind = \"test.{gesture}\"\n"
                ));
            }
        }
        let loaded = load_text(&store, text.as_bytes());
        assert!(loaded.backup.is_none(), "{:?}", loaded.backup);
        store.save(&loaded.profile).unwrap();
        let written = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            written.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
            "{written}"
        );
        assert_eq!(written.matches(".key-3.long-press]").count(), 2);
        assert_eq!(written.matches(".key-3.double-press]").count(), 2);
        assert_eq!(written.matches(".key-face-1.on]").count(), 2);
        let key = opencraft_core::SurfaceId::new(opencraft_core::SurfaceKind::KeyFace, 1);
        let face = loaded.profile.pages()[0].content(key).on.unwrap();
        assert_eq!(face.label.as_deref(), Some("Live"));
        assert_eq!(face.label_style.size, Some(24));
        assert_eq!(face.color, Some(Color::rgb(0x22, 0x33, 0x44)));
        assert_eq!(face.image.as_deref(), Some("images/0123456789abcdef.png"));
        let folder_face = loaded.profile.folders()[0].page.content(key).on.unwrap();
        assert_eq!(folder_face.label_style, face.label_style);
        assert_eq!(folder_face.icon.unwrap().name, "star");
        assert_eq!(store.load().unwrap().profile, loaded.profile);
    }

    #[test]
    fn old_or_nested_on_faces_are_rejected() {
        for (version, holder, suffix, expected) in [
            (6, "pages", "on", "version 6 has no on face"),
            (6, "folders", "on", "version 6 has no on face"),
            (7, "pages", "on.on", "an on face cannot have an on face"),
        ] {
            let (_dir, store) = store();
            let mut text = old_profile_text(version);
            if holder == "folders" {
                text.push_str("[[folders]]\nid = \"0000000000000002\"\nname = \"More\"\n");
            }
            text.push_str(&format!("[{holder}.surfaces.key-face-1.{suffix}]\n"));
            let backup = load_text(&store, text.as_bytes()).backup.unwrap();
            assert!(
                matches!(backup.reason, BackupReason::Invalid(ref reason) if reason.contains(expected)),
                "{:?}",
                backup.reason
            );
            assert_eq!(fs::read_to_string(backup.path).unwrap(), text);
        }
    }

    #[test]
    fn older_files_with_new_gestures_are_kept_as_backups() {
        for version in 1..=6 {
            for holder in ["pages", "folders"] {
                if holder == "folders" && version < 5 {
                    continue;
                }
                for gesture in ["long-press", "double-press"] {
                    let (_dir, store) = store();
                    let mut text = old_profile_text(version);
                    if holder == "folders" {
                        text.push_str("[[folders]]\nid = \"0000000000000002\"\nname = \"More\"\n");
                    }
                    text.push_str(&format!(
                        "[{holder}.bindings.key-3.{gesture}]\nkind = \"test.action\"\n"
                    ));
                    let backup = load_text(&store, text.as_bytes()).backup.unwrap();
                    let expected = if version == 1 {
                        "version 1 has no bindings"
                    } else {
                        "requires schema version 7"
                    };
                    assert!(
                        matches!(backup.reason, BackupReason::Invalid(ref reason) if reason.contains(expected)),
                        "{:?}",
                        backup.reason
                    );
                    assert_eq!(fs::read_to_string(backup.path).unwrap(), text);
                }
            }
        }
    }

    #[test]
    fn a_file_with_a_newer_schema_version_is_kept_as_a_backup() {
        let (_dir, store) = store();
        let text = "schema_version = 9\nname = \"Later\"\nnew_field = true\n";
        let loaded = load_text(&store, text.as_bytes());

        assert_eq!(loaded.profile, Profile::default());
        let backup = loaded.backup.unwrap();
        assert_eq!(backup.reason, BackupReason::NewerSchema { found: 9 });
        assert_eq!(backup.path, store.dir().join("profile.backup-1.toml"));
        // The content of the file did not change, and no profile file is
        // there until the next save.
        assert_eq!(fs::read_to_string(&backup.path).unwrap(), text);
        assert!(!store.profile_path().exists());
    }

    #[test]
    fn a_file_with_an_error_is_kept_as_a_backup() {
        let cases: &[&[u8]] = &[
            b"",
            b"this is not TOML [",
            b"name = \"No version\"\n",
            b"schema_version = 0\nname = \"x\"\n",
            b"schema_version = \"1\"\nname = \"x\"\n",
            b"schema_version = 1\n",
            b"schema_version = 1\nname = \"x\"\nunknown = 1\n",
            b"schema_version = 1\nname = \"x\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.knob-1]\nlabel = \"x\"\n",
            b"schema_version = 1\nname = \"x\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\ncolor = \"red\"\n",
            b"schema_version = 1\nname = \"x\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nimage = \"../secret.png\"\n",
            b"schema_version = 1\nname = \"x\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nimage = \"/tmp/a.png\"\n",
            b"\xFF\xFE not UTF-8",
            // Version 3 needs a model, and versions 1 and 2 have none.
            b"schema_version = 3\nname = \"x\"\n",
            b"schema_version = 3\nname = \"x\"\nmodel = \"\"\n",
            b"schema_version = 3\nname = \"x\"\nmodel = 4\n",
            b"schema_version = 2\nname = \"x\"\nmodel = \"n4-pro\"\n",
            b"schema_version = 4\nname = \"x\"\n",
            // Versions 1 to 3 have no label options, and version 4 checks
            // each value.
            b"schema_version = 3\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_bold = false\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_position = \"centre\"\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_size = 9\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_size = 41\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_size = 300\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_color = \"white\"\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_bold = \"yes\"\n",
            // Version 4 adds the icon. Versions 1 to 3 have none, an icon
            // name has only small letters, digits, and hyphens, a colour
            // needs an icon, and a surface has an image or an icon.
            b"schema_version = 3\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nicon = \"globe\"\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nicon = \"Globe\"\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nicon = \"\"\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nicon = \"../globe\"\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nicon_color = \"#FFFFFF\"\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nicon = \"globe\"\nicon_color = \"amber\"\n",
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nicon = \"globe\"\nimage = \"images/4f1c2a9be07d3356.png\"\n",
            // Version 4 has no folders. A folder has a real identity, a name,
            // and only key faces and keys.
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[folders]]\nid = \"0000000000000001\"\nname = \"Clips\"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[folders]]\nid = \"0000000000000000\"\nname = \"Clips\"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[folders]]\nid = \"0000000000000001\"\nname = \"  \"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[folders]]\nid = \"ZZ\"\nname = \"Clips\"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[folders]]\nid = \"0000000000000001\"\nname = \"Clips\"\n[[folders]]\nid = \"0000000000000001\"\nname = \"Again\"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[folders]]\nid = \"0000000000000001\"\nname = \"Clips\"\n[folders.surfaces.bar-panel-1]\nlabel = \"x\"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[folders]]\nid = \"0000000000000001\"\nname = \"Clips\"\n[folders.bindings.knob-1.press]\nkind = \"opencraft.hotkey\"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[folders]]\nid = \"0000000000000001\"\nname = \"Clips\"\nbackground = \"#000000\"\n",
            // Version 4 and lower have no links.
            b"schema_version = 4\nname = \"x\"\nmodel = \"m\"\napplications = []\n",
            b"schema_version = 3\nname = \"x\"\nmodel = \"m\"\nfolders = []\n",
            b"schema_version = 5\nname = \"x\"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[applications]]\nbundle_id = \"not valid\"\nname = \"X\"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[applications]]\nbundle_id = \"com.apple.Safari\"\nname = \"\"\n",
            b"schema_version = 5\nname = \"x\"\nmodel = \"m\"\n[[applications]]\nbundle_id = \"com.apple.Safari\"\nname = \"Safari\"\n[[applications]]\nbundle_id = \"COM.APPLE.SAFARI\"\nname = \"Safari\"\n",
            // Version 8 adds the font, the italic form, the underline, the
            // automatic word of the position and the colour, and a label
            // size above 40. A file of version 7 with one has an error.
            b"schema_version = 7\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_font = \"Avenir Next\"\n",
            b"schema_version = 7\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_italic = true\n",
            b"schema_version = 7\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_underline = true\n",
            b"schema_version = 7\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_position = \"auto\"\n",
            b"schema_version = 7\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_color = \"auto\"\n",
            b"schema_version = 7\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_size = 41\n",
            // An old file of version 1 to 3 has no label options at all.
            b"schema_version = 3\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_font = \"Avenir Next\"\n",
            b"schema_version = 3\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_position = \"auto\"\n",
            // A family name is not empty.
            b"schema_version = 8\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel_font = \"\"\n",
            // Version 8 adds the crop of an image. A file of version 7 with
            // one has an error, a key needs an image, and each value has a
            // range (decision 0043).
            b"schema_version = 7\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nimage = \"images/0123456789abcdef.png\"\nimage_zoom = 200\n",
            b"schema_version = 7\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nimage = \"images/0123456789abcdef.png\"\nimage_offset_x = -500\n",
            b"schema_version = 7\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1.on]\nimage = \"images/0123456789abcdef.png\"\nimage_offset_y = 500\n",
            b"schema_version = 8\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nimage_zoom = 200\n",
            b"schema_version = 8\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nlabel = \"Mail\"\nimage_offset_x = 100\n",
            b"schema_version = 8\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nimage = \"images/0123456789abcdef.png\"\nimage_zoom = 99\n",
            b"schema_version = 8\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nimage = \"images/0123456789abcdef.png\"\nimage_zoom = 401\n",
            b"schema_version = 8\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nimage = \"images/0123456789abcdef.png\"\nimage_offset_x = 1001\n",
            b"schema_version = 8\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nid = \"0000000000000001\"\nname = \"p\"\n[pages.surfaces.key-face-1]\nimage = \"images/0123456789abcdef.png\"\nimage_offset_y = -1001\n",
        ];
        for (i, text) in cases.iter().enumerate() {
            let (_dir, store) = store();
            let loaded = load_text(&store, text);
            assert_eq!(loaded.profile, Profile::default(), "case {i}");
            let backup = loaded
                .backup
                .unwrap_or_else(|| panic!("case {i}: no backup"));
            assert!(
                matches!(backup.reason, BackupReason::Invalid(_)),
                "case {i}: {:?}",
                backup.reason
            );
            assert_eq!(&fs::read(&backup.path).unwrap(), text, "case {i}");
        }
    }

    /// `FileData.folders` is `Some` when the key is in the file, including an
    /// empty array. One check after that read is enough. An older file does
    /// not need a second parse of the text.
    #[test]
    fn an_old_file_with_folders_fails_on_the_one_check() {
        let cases = [
            (
                4,
                "schema_version = 4\nname = \"x\"\nmodel = \"m\"\nfolders = []\n",
            ),
            (
                4,
                "schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[folders]]\nid = \"0000000000000001\"\nname = \"Clips\"\n",
            ),
            (1, "schema_version = 1\nname = \"x\"\nfolders = []\n"),
        ];
        for (version, text) in cases {
            assert_eq!(
                parse(text),
                Err(BackupReason::Invalid(format!(
                    "version {version} has no folders"
                ))),
                "{text}"
            );
        }
    }

    #[test]
    fn a_version_two_file_loads_with_no_loss_and_the_model_of_the_first_descriptor() {
        let (_dir, store) = store();
        let old = r##"schema_version = 2
name = "Old"

[[pages]]
name = "Main"

[pages.surfaces.key-face-3]
label = "Mail"
color = "#1E6FD9"

[pages.bindings.key-3.press]
kind = "opencraft.hotkey"

[pages.bindings.key-3.press.settings]
hotkey = "cmd+shift+4"

[pages.bindings.bar-panel-2.tap]
kind = "test.tap"

[pages.bindings.touch-bar-1.swipe-left]
kind = "test.left"

[pages.bindings.touch-bar-1.swipe-right]
kind = "test.right"

[[pages]]
name = "Second"
"##;
        let loaded = load_text(&store, old.as_bytes());
        assert_eq!(loaded.backup, None);
        let profile = loaded.profile;
        assert_eq!(profile.model, default_model());
        assert_eq!(profile.name, "Old");
        assert_eq!(profile.pages().len(), 2);
        assert_eq!(profile.pages()[0].bindings().count(), 4);
        // The load did not write the file.
        assert_eq!(fs::read_to_string(store.profile_path()).unwrap(), old);

        // The next write is the current version with the same content and
        // the model.
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(text.starts_with(&format!(
            "schema_version = {SCHEMA_VERSION}\nname = \"Old\"\nmodel = \"n4-pro\"\n"
        )));
        assert!(text.contains("hotkey = \"cmd+shift+4\""), "{text}");
        assert!(!text.contains("label_"), "{text}");
        assert_eq!(store.load().unwrap().profile, profile);
    }

    #[test]
    fn a_version_three_file_loads_with_the_default_label_style() {
        let (_dir, store) = store();
        let old = r##"schema_version = 3
name = "Old"
model = "n4-pro"

[[pages]]
name = "Main"

[pages.surfaces.key-face-3]
label = "Mail"
color = "#1E6FD9"
image = "images/4f1c2a9be07d3356.png"

[pages.surfaces.bar-panel-1]
label = "Sound"
"##;
        let loaded = load_text(&store, old.as_bytes());
        assert_eq!(loaded.backup, None);
        let page = &loaded.profile.pages()[0];
        for id in [KEY_3, BAR_1] {
            assert_eq!(page.content(id).label_style, LabelStyle::default(), "{id}");
        }
        assert_eq!(page.content(KEY_3).label.as_deref(), Some("Mail"));
        // The load did not write the file. The next write is the current
        // version, and it writes no label option.
        assert_eq!(fs::read_to_string(store.profile_path()).unwrap(), old);
        store.save(&loaded.profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
            "{text}"
        );
        assert!(!text.contains("label_"), "{text}");
        assert_eq!(store.load().unwrap().profile, loaded.profile);
    }

    #[test]
    fn the_label_options_save_with_their_text_form_and_load_again() {
        let (_dir, store) = store();
        let mut profile = sample();
        let page = profile.page_mut(0).unwrap();
        page.update(KEY_3, |c| {
            c.label_style = LabelStyle {
                position: Some(Auto::Value(LabelPosition::Top)),
                size: Some(14),
                color: Some(Auto::Value(Color::rgb(0xFF, 0xCE, 0x73))),
                bold: Some(false),
                ..LabelStyle::default()
            }
        });
        page.update(BAR_1, |c| {
            c.label_style.position = Some(Auto::Value(LabelPosition::Hidden));
        });
        // Only a style, with no label.
        let key_1 = SurfaceId::new(SurfaceKind::KeyFace, 1);
        page.update(key_1, |c| {
            c.label_style.position = Some(Auto::Value(LabelPosition::Bottom))
        });
        store.save(&profile).unwrap();

        let text = fs::read_to_string(store.profile_path()).unwrap();
        let key_3 = r##"[pages.surfaces.key-face-3]
label = "Mail"
label_position = "top"
label_size = 14
label_color = "#FFCE73"
label_bold = false
color = "#1E6FD9"
"##;
        assert!(text.contains(key_3), "{text}");
        assert!(text.contains("label_position = \"none\""), "{text}");
        assert!(
            text.contains("[pages.surfaces.key-face-1]\nlabel_position = \"bottom\"\n"),
            "{text}"
        );
        assert_eq!(store.load().unwrap().profile, profile);
        for position in LabelPosition::ALL {
            let mut profile = Profile::default();
            profile.page_mut(0).unwrap().update(KEY_3, |c| {
                c.label = Some("x".into());
                c.label_style.position = Some(Auto::Value(position));
            });
            store.save(&profile).unwrap();
            assert_eq!(store.load().unwrap().profile, profile, "{position:?}");
        }
    }

    #[test]
    fn the_label_keys_of_version_eight_have_their_text_form_and_load_again() {
        let (_dir, store) = store();
        let mut profile = Profile::new("P", vec![Page::new("Main")]);
        let page = profile.page_mut(0).unwrap();
        page.update(KEY_3, |c| {
            c.label = Some("Mail".into());
            c.label_style = LabelStyle {
                font: Some(Auto::Value("Avenir Next".into())),
                position: Some(Auto::Auto),
                size: Some(24),
                color: Some(Auto::Auto),
                bold: Some(true),
                italic: Some(true),
                underline: Some(true),
            };
        });
        // An own value that is equal to the engine default is written, and
        // the automatic font is the word "system".
        page.update(BAR_1, |c| {
            c.label_style = LabelStyle {
                font: Some(Auto::Auto),
                size: Some(DEFAULT_LABEL_SIZE),
                ..LabelStyle::default()
            };
        });
        store.save(&profile).unwrap();

        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
            "{text}"
        );
        for line in [
            "label_font = \"Avenir Next\"",
            "label_position = \"auto\"",
            "label_size = 24",
            "label_color = \"auto\"",
            "label_bold = true",
            "label_italic = true",
            "label_underline = true",
        ] {
            assert!(text.contains(line), "{line} in {text}");
        }
        assert!(
            text.contains(
                "[pages.surfaces.bar-panel-1]\nlabel_size = 20\nlabel_font = \"system\"\n"
            ),
            "{text}"
        );
        assert_eq!(store.load().unwrap().profile, profile);
    }

    #[test]
    fn a_font_change_cannot_write_a_file_that_does_not_load() {
        let (_dir, store) = store();
        let mut profile = Profile::new("P", vec![Page::new("Main")]);
        profile.page_mut(0).unwrap().update(KEY_3, |c| {
            c.label = Some("Mail".into());
            c.label_style
                .apply(LabelChange::Font(Some(Auto::Value("   ".into()))));
            c.label_style.apply(LabelChange::Italic(Some(true)));
        });
        assert_eq!(
            profile.pages()[0].content(KEY_3).label_style.font,
            None,
            "An empty family is no own value."
        );
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(!text.contains("label_font"), "{text}");
        assert_eq!(store.load().unwrap().profile, profile);

        // The reserved word is the automatic font, and it loads again.
        profile.page_mut(0).unwrap().update(KEY_3, |c| {
            c.label_style
                .apply(LabelChange::Font(Some(Auto::Value(" system ".into()))));
        });
        assert_eq!(
            profile.pages()[0].content(KEY_3).label_style.font,
            Some(Auto::Auto)
        );
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(text.contains("label_font = \"system\""), "{text}");
        assert_eq!(store.load().unwrap().profile, profile);
    }

    #[test]
    fn the_default_title_style_saves_before_the_pages_and_loads_again() {
        let (_dir, store) = store();
        let mut profile = Profile::new("P", vec![Page::new("Main")]);
        profile.default_label_style = LabelStyle {
            font: Some(Auto::Value("Avenir Next".into())),
            position: Some(Auto::Value(LabelPosition::Top)),
            size: Some(30),
            color: Some(Auto::Value(Color::rgb(0x12, 0x34, 0x56))),
            bold: Some(false),
            italic: Some(true),
            underline: Some(true),
        };
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        let table = text.find("[default_label_style]").unwrap();
        let pages = text.find("[[pages]]").unwrap();
        assert!(table < pages, "{text}");
        for line in [
            "font = \"Avenir Next\"",
            "position = \"top\"",
            "size = 30",
            "color = \"#123456\"",
            "bold = false",
            "italic = true",
            "underline = true",
        ] {
            assert!(text.contains(line), "{line} in {text}");
        }
        assert_eq!(
            store.load().unwrap().profile.default_label_style,
            profile.default_label_style
        );

        // The empty style writes no table.
        profile.default_label_style = LabelStyle::default();
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(!text.contains("[default_label_style]"), "{text}");
        assert_eq!(store.load().unwrap().profile, profile);
    }

    #[test]
    fn an_old_schema_refuses_the_default_title_style() {
        for version in 1..FACES_VERSION {
            let model = if version >= 3 {
                "model = \"n4-pro\"\n"
            } else {
                ""
            };
            let id = if version >= 6 {
                "id = \"0000000000000001\"\n"
            } else {
                ""
            };
            let text = format!(
                "schema_version = {version}\nname = \"Old\"\n{model}\
                 [default_label_style]\nfont = \"Menlo\"\n\n\
                 [[pages]]\n{id}name = \"Main\"\n"
            );
            let error = parse(&text).unwrap_err().to_string();
            assert!(
                error.contains("default title style"),
                "version {version}: {error}"
            );
        }
        let text = format!(
            "schema_version = {FACES_VERSION}\nname = \"New\"\nmodel = \"n4-pro\"\n\
             [default_label_style]\nfont = \"Menlo\"\n\n\
             [[pages]]\nid = \"0000000000000001\"\nname = \"Main\"\n"
        );
        assert_eq!(
            parse(&text).unwrap().default_label_style.font,
            Some(Auto::Value("Menlo".into()))
        );
    }

    #[test]
    fn schema_eight_accepts_a_label_size_of_eighty() {
        let text = format!(
            "schema_version = {SCHEMA_VERSION}\nname = \"P\"\nmodel = \"n4-pro\"\n\
             [[pages]]\nid = \"0000000000000001\"\nname = \"Main\"\n\
             [pages.surfaces.key-face-3]\nlabel_size = 80\n"
        );
        let profile = parse(&text).unwrap();
        assert_eq!(profile.pages()[0].content(KEY_3).label_style.size, Some(80));
        // Version 7 refuses the same size (decision 0043, section 3).
        let old = "schema_version = 7\nname = \"P\"\nmodel = \"n4-pro\"\n\
                   [[pages]]\nid = \"0000000000000001\"\nname = \"Main\"\n\
                   [pages.surfaces.key-face-3]\nlabel_size = 80\n";
        assert!(parse(old).is_err());
    }

    #[test]
    fn the_default_title_style_refuses_a_bad_value() {
        for (key, value, text) in [
            ("font", "\"\"", "font is empty"),
            ("size", "4", "size 4 is not from"),
            ("size", "81", "size 81 is not from"),
            ("position", "\"centre\"", "is not a position"),
            ("color", "\"red\"", "color 'red'"),
        ] {
            let file = format!(
                "schema_version = {FACES_VERSION}\nname = \"P\"\nmodel = \"n4-pro\"\n\
                 [default_label_style]\n{key} = {value}\n\n\
                 [[pages]]\nid = \"0000000000000001\"\nname = \"Main\"\n"
            );
            let error = parse(&file).unwrap_err().to_string();
            assert!(error.contains(text), "{key}: {error}");
        }
    }

    #[test]
    fn the_crop_of_an_image_saves_with_its_keys_and_loads_again() {
        let (_dir, store) = store();
        let mut profile = sample();
        let page = profile.page_mut(0).unwrap();
        page.update(KEY_3, |c| {
            c.image_crop = Some(ImageCrop {
                zoom: 200,
                x: -500,
                y: 1000,
            })
        });
        // An On face has its own keys.
        page.update(BAR_1, |c| {
            c.image = Some("images/4f1c2a9be07d3356.png".into());
        });
        page.update(BAR_1, |c| {
            let face = c.face_mut(true);
            face.label = None;
            face.image_crop = Some(ImageCrop {
                zoom: 400,
                x: 0,
                y: 0,
            });
        });
        // The fit writes no key, and a face with no image has no crop.
        let key_1 = SurfaceId::new(SurfaceKind::KeyFace, 1);
        page.update(key_1, |c| {
            c.image = Some("images/4f1c2a9be07d3356.png".into());
            c.image_crop = Some(ImageCrop::default());
        });
        store.save(&profile).unwrap();

        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
            "{text}"
        );
        assert!(
            text.contains(
                "[pages.surfaces.key-face-3]\nlabel = \"Mail\"\ncolor = \"#1E6FD9\"\nimage = \"images/4f1c2a9be07d3356.png\"\nimage_zoom = 200\nimage_offset_x = -500\nimage_offset_y = 1000\n"
            ),
            "{text}"
        );
        assert!(
            text.contains(
                "[pages.surfaces.bar-panel-1.on]\nimage = \"images/4f1c2a9be07d3356.png\"\nimage_zoom = 400\n"
            ),
            "{text}"
        );
        assert!(
            text.contains("[pages.surfaces.key-face-1]\nimage = \"images/4f1c2a9be07d3356.png\"\n"),
            "{text}"
        );
        assert!(!text.contains("image_offset_x = 0"), "{text}");
        // The crop stays after a load: a restart of the engine reads it.
        assert_eq!(store.load().unwrap().profile, profile);
        assert_eq!(store.read().unwrap(), profile);
    }

    #[test]
    fn a_profile_of_a_different_model_saves_and_loads_with_its_model() {
        let (_dir, store) = store();
        let profile = Profile::for_model("other-model", "Other", vec![Page::new("A")]);
        store.save(&profile).unwrap();
        assert_eq!(store.load().unwrap().profile, profile);
        assert_eq!(store.read().unwrap(), profile);
        let summary = store.summary();
        assert_eq!(
            (summary.name.as_deref(), summary.model.as_deref()),
            (Some("Other"), Some("other-model"))
        );
        assert_eq!((summary.pages, summary.first_page), (vec!["A".into()], 0));
    }

    #[test]
    fn the_colour_and_the_first_page_save_and_load_again() {
        let (_dir, store) = store();
        let mut profile = Profile::new("P", vec![Page::new("A"), Page::new("B")]);
        profile.color = Some(Color::rgb(0x3B, 0x82, 0xF6));
        assert!(profile.set_first_page(1));
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            text.starts_with(&format!(
                "schema_version = {SCHEMA_VERSION}\nname = \"P\"\nmodel = \"n4-pro\"\ncolor = \"#3B82F6\"\nfirst_page = 2\n"
            )),
            "{text}"
        );
        assert_eq!(store.load().unwrap().profile, profile);
        let summary = store.summary();
        assert_eq!(summary.color, profile.color);
        assert_eq!(summary.pages, ["A", "B"]);
        assert_eq!(summary.first_page, 1);

        // With no choice, the file has no key.
        store.save(&Profile::default()).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            !text.contains("first_page") && !text.contains("\ncolor"),
            "{text}"
        );

        for bad in [
            "schema_version = 4\nname = \"x\"\nmodel = \"m\"\nfirst_page = 2\n",
            "schema_version = 4\nname = \"x\"\nmodel = \"m\"\nfirst_page = 0\n",
            "schema_version = 4\nname = \"x\"\nmodel = \"m\"\ncolor = \"blue\"\n",
            "schema_version = 3\nname = \"x\"\nmodel = \"m\"\ncolor = \"#000000\"\n",
        ] {
            let backup = load_text(&store, bad.as_bytes()).backup;
            assert!(backup.is_some(), "{bad}");
        }
    }

    #[test]
    fn the_links_save_and_load_again_and_an_old_file_keeps_its_pages() {
        let (_dir, store) = store();
        let old = "schema_version = 4\nname = \"Calls\"\nmodel = \"n4-pro\"\n\n\
                   [[pages]]\nname = \"Main\"\n\n[[pages]]\nname = \"Dial\"\n";
        let loaded = load_text(&store, old.as_bytes());
        assert_eq!(loaded.backup, None);
        assert!(loaded.profile.applications.is_empty());
        assert_eq!(loaded.profile.pages().len(), 2);
        let mut profile = loaded.profile;
        profile.applications.push(ApplicationLink {
            bundle_id: "us.zoom.xos".into(),
            name: "zoom.us".into(),
        });
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
            "{text}"
        );
        assert!(text.contains("name = \"Dial\""), "{text}");
        assert!(
            text.contains("bundle_id = \"us.zoom.xos\"\nname = \"zoom.us\""),
            "{text}"
        );
        assert_eq!(store.load().unwrap().profile, profile);
        assert_eq!(store.summary().applications, profile.applications);

        let version_3 = "schema_version = 3\nname = \"Old\"\nmodel = \"n4-pro\"\n\n\
                         [[pages]]\nname = \"Main\"\n";
        let loaded = load_text(&store, version_3.as_bytes());
        assert_eq!(loaded.backup, None);
        assert_eq!(loaded.profile.pages().len(), 1);
        store.save(&loaded.profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
            "{text}"
        );
        assert!(text.contains("name = \"Main\""), "{text}");
        assert!(!text.contains("applications"), "{text}");
        assert!(store.summary().applications.is_empty());
    }

    #[test]
    fn the_summary_and_the_full_read_agree_on_links_that_are_not_correct() {
        let (_dir, store) = store();
        fs::create_dir_all(store.dir()).unwrap();
        let head = "schema_version = 5\nname = \"Calls\"\nmodel = \"n4-pro\"\n\n";
        // One more than the maximum count.
        let mut too_many = String::from(head);
        for number in 0..MAX_LINKS + 1 {
            too_many.push_str(&format!(
                "[[applications]]\nbundle_id = \"com.example.app{number}\"\nname = \"App {number}\"\n"
            ));
        }
        let repeated = format!(
            "{head}[[applications]]\nbundle_id = \"us.zoom.xos\"\nname = \"zoom.us\"\n\
             [[applications]]\nbundle_id = \"US.ZOOM.XOS\"\nname = \"Zoom\"\n"
        );
        // A link that is not correct, then a link that is correct. The
        // summary must not keep the correct link.
        let incorrect = format!(
            "{head}[[applications]]\nbundle_id = \"not valid\"\nname = \"Bad\"\n\
             [[applications]]\nbundle_id = \"com.apple.Safari\"\nname = \"Safari\"\n"
        );
        let old = "schema_version = 4\nname = \"x\"\nmodel = \"m\"\n\
                   [[applications]]\nbundle_id = \"com.apple.Safari\"\nname = \"Safari\"\n";
        for (label, text) in [
            ("too many", too_many),
            ("repeated", repeated),
            ("not correct", incorrect),
            ("version 4", old.to_string()),
        ] {
            let from_read = parse(&text)
                .map(|profile| profile.applications)
                .map_err(|reason| match reason {
                    BackupReason::Invalid(message) => message,
                    other => other.to_string(),
                });
            let from_summary = applications_of(&text.parse().unwrap());
            assert_eq!(from_summary, from_read, "{label}");
            assert!(from_read.is_err(), "{label}");
            fs::write(store.profile_path(), &text).unwrap();
            assert!(store.summary().applications.is_empty(), "{label}");
            assert!(
                matches!(store.read(), Err(Error::Unusable { .. })),
                "{label}"
            );
        }
    }

    #[test]
    fn read_changes_no_file_that_the_engine_cannot_use() {
        for text in ["schema_version = 99\nname = \"x\"\n", "not TOML ["] {
            let (_dir, store) = store();
            fs::create_dir_all(store.dir()).unwrap();
            fs::write(store.profile_path(), text).unwrap();
            assert!(
                matches!(store.read(), Err(Error::Unusable { .. })),
                "{text}"
            );
            assert_eq!(files(store.dir()), [PROFILE_FILE]);
            assert_eq!(fs::read_to_string(store.profile_path()).unwrap(), text);
        }
        let (_dir, store) = store();
        assert!(matches!(store.read(), Err(Error::Unusable { .. })));
        assert_eq!(store.summary(), Summary::default());
    }

    #[test]
    fn a_copy_of_the_images_has_only_the_images_that_the_profile_uses() {
        let (dir, store) = store();
        let used = store
            .import_image(&image_file(dir.path(), "a.png", 8, 8))
            .unwrap();
        store
            .import_image(&image_file(dir.path(), "b.png", 9, 9))
            .unwrap();
        let mut profile = Profile::default();
        let page = profile.page_mut(0).unwrap();
        page.update(KEY_3, |c| c.image = Some(used.clone()));
        page.update(BAR_1, |c| {
            c.image = Some("images/0123456789abcdef.png".into())
        });

        let copy = ProfileStore::new(dir.path().join("copy"));
        copy.copy_images_from(&store, &profile).unwrap();
        assert_eq!(
            files(&copy.dir().join(IMAGE_DIR)),
            [image_name(&used).unwrap()]
        );
        assert!(copy.load_image(&used).is_ok());
    }

    #[test]
    fn a_second_backup_does_not_replace_the_first() {
        let (_dir, store) = store();
        let first = load_text(&store, b"first error [").backup.unwrap();
        let second = load_text(&store, b"second error [").backup.unwrap();
        assert_ne!(first.path, second.path);
        assert_eq!(fs::read(&first.path).unwrap(), b"first error [");
        assert_eq!(fs::read(&second.path).unwrap(), b"second error [");
    }

    #[test]
    fn a_file_with_no_page_gets_one_page() {
        let (_dir, store) = store();
        let loaded = load_text(&store, b"schema_version = 1\nname = \"Empty\"\n");
        assert_eq!(loaded.backup, None);
        assert_eq!(loaded.profile.name, "Empty");
        assert_eq!(loaded.profile.pages().len(), 1);
    }

    #[test]
    fn a_temporary_file_from_a_stopped_write_does_not_prevent_a_load_or_a_save() {
        let (_dir, store) = store();
        store.save(&sample()).unwrap();
        fs::write(store.dir().join("profile.toml.tmp"), "schema_ver").unwrap();
        assert_eq!(store.load().unwrap().profile, sample());
        store.save(&Profile::default()).unwrap();
        assert_eq!(files(store.dir()), [PROFILE_FILE]);
    }

    #[test]
    fn an_image_becomes_a_png_copy_with_a_name_from_its_data() {
        let (dir, store) = store();
        let source = image_file(dir.path(), "icon.png", 64, 48);

        let path = store.import_image(&source).unwrap();
        let name = image_name(&path).expect("the path is in the image directory");
        assert_eq!(files(&store.dir().join(IMAGE_DIR)), [name]);

        let copy = store.load_image(&path).unwrap();
        assert_eq!((copy.width(), copy.height()), (64, 48));
        // The transparency of the source stays.
        assert_eq!(copy.to_rgba8().get_pixel(0, 0)[3], 0);
        assert_eq!(copy.to_rgba8().get_pixel(10, 5), &Rgba([10, 5, 7, 255]));

        // The same image gives the same file, also from a different format.
        assert_eq!(store.import_image(&source).unwrap(), path);
        assert_eq!(files(&store.dir().join(IMAGE_DIR)).len(), 1);
    }

    #[test]
    fn a_large_image_becomes_smaller_and_keeps_its_proportion() {
        let (dir, store) = store();
        let source = image_file(dir.path(), "large.png", 2048, 1024);
        let copy = store
            .load_image(&store.import_image(&source).unwrap())
            .unwrap();
        assert_eq!((copy.width(), copy.height()), (512, 256));

        // An image at the limit keeps its size.
        let source = image_file(dir.path(), "limit.png", 512, 512);
        let copy = store
            .load_image(&store.import_image(&source).unwrap())
            .unwrap();
        assert_eq!((copy.width(), copy.height()), (512, 512));
    }

    #[test]
    fn a_jpeg_is_accepted() {
        let (dir, store) = store();
        let path = dir.path().join("photo.jpg");
        image::RgbImage::from_pixel(40, 30, image::Rgb([200, 30, 30]))
            .save(&path)
            .unwrap();
        let copy = store
            .load_image(&store.import_image(&path).unwrap())
            .unwrap();
        assert_eq!((copy.width(), copy.height()), (40, 30));
    }

    /// Write the bytes of a new image of `format` to `name`, and return the
    /// path. The test does not use the file extension: the decoders guess
    /// the format from the bytes, as they do for a paste.
    fn image_bytes_file(dir: &Path, name: &str, format: image::ImageFormat) -> PathBuf {
        let image = image::RgbImage::from_fn(7, 5, |x, y| image::Rgb([x as u8, y as u8, 9]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut bytes, format)
            .unwrap();
        let path = dir.join(name);
        fs::write(&path, bytes.into_inner()).unwrap();
        path
    }

    #[test]
    fn an_image_of_each_format_becomes_a_png_copy_from_a_file_and_from_bytes() {
        let (dir, store) = store();
        for (name, format) in [
            ("a.png", image::ImageFormat::Png),
            ("b.jpg", image::ImageFormat::Jpeg),
            ("c.bmp", image::ImageFormat::Bmp),
            ("d.webp", image::ImageFormat::WebP),
        ] {
            let source = image_bytes_file(dir.path(), name, format);
            let path = store.import_image(&source).unwrap();
            assert!(path.ends_with(".png"), "{name} gives {path}");
            let copy = store.load_image(&path).unwrap();
            assert_eq!((copy.width(), copy.height()), (7, 5), "{name}");
        }
    }

    /// A small TIFF file: 2 × 2 pixels, three samples of 8 bits, one strip
    /// that is not compressed. A copy from Preview is often a TIFF.
    fn tiff_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"II\x2a\x00");
        bytes.extend_from_slice(&8u32.to_le_bytes());
        bytes.extend_from_slice(&10u16.to_le_bytes());
        {
            let mut entry = |tag: u16, kind: u16, count: u32, value: [u8; 4]| {
                bytes.extend_from_slice(&tag.to_le_bytes());
                bytes.extend_from_slice(&kind.to_le_bytes());
                bytes.extend_from_slice(&count.to_le_bytes());
                bytes.extend_from_slice(&value);
            };
            entry(256, 4, 1, 2u32.to_le_bytes()); // ImageWidth
            entry(257, 4, 1, 2u32.to_le_bytes()); // ImageLength
            entry(258, 3, 3, 134u32.to_le_bytes()); // BitsPerSample
            entry(259, 3, 1, [1, 0, 0, 0]); // Compression: none
            entry(262, 3, 1, [2, 0, 0, 0]); // Photometric: RGB
            entry(273, 4, 1, 140u32.to_le_bytes()); // StripOffsets
            entry(277, 3, 1, [3, 0, 0, 0]); // SamplesPerPixel
            entry(278, 4, 1, 2u32.to_le_bytes()); // RowsPerStrip
            entry(279, 4, 1, 12u32.to_le_bytes()); // StripByteCounts
            entry(284, 3, 1, [1, 0, 0, 0]); // PlanarConfiguration
        }
        bytes.extend_from_slice(&0u32.to_le_bytes());
        for sample in [8u16, 8, 8] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]);
        bytes
    }

    #[test]
    fn a_tiff_is_accepted() {
        let (dir, store) = store();
        let source = dir.path().join("copy.tiff");
        fs::write(&source, tiff_bytes()).unwrap();
        let copy = store
            .load_image(&store.import_image(&source).unwrap())
            .unwrap();
        assert_eq!((copy.width(), copy.height()), (2, 2));
        assert_eq!(copy.to_rgba8().get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn a_file_that_is_not_an_image_gives_an_error_and_writes_nothing() {
        let (dir, store) = store();
        let source = dir.path().join("notes.png");
        fs::write(&source, "this is text").unwrap();
        assert!(matches!(
            store.import_image(&source),
            Err(Error::Image { .. })
        ));
        assert!(matches!(
            store.import_image(&dir.path().join("missing.png")),
            Err(Error::Io { .. })
        ));
        assert!(!store.dir().exists());
    }

    #[test]
    fn load_image_refuses_a_path_outside_the_image_directory() {
        let (dir, store) = store();
        // A real image outside the image directory.
        fs::create_dir_all(store.dir()).unwrap();
        image_file(store.dir(), "0123456789abcdef.png", 4, 4);
        image_file(dir.path(), "0123456789abcdef.png", 4, 4);
        for path in [
            "0123456789abcdef.png",
            "images/../0123456789abcdef.png",
            "images/../../0123456789abcdef.png",
            "/images/0123456789abcdef.png",
            "images/sub/0123456789abcdef.png",
            "images/0123456789ABCDEF.png",
            "images/0123456789abcdef.jpg",
            "images/",
            "",
        ] {
            assert!(
                matches!(store.load_image(path), Err(Error::ImagePath(_))),
                "{path}"
            );
        }
    }

    #[test]
    fn unused_image_copies_are_removed_and_other_files_stay() {
        let (dir, store) = store();
        let used = store
            .import_image(&image_file(dir.path(), "a.png", 8, 8))
            .unwrap();
        let unused = store
            .import_image(&image_file(dir.path(), "b.png", 9, 9))
            .unwrap();
        let images = store.dir().join(IMAGE_DIR);
        fs::write(images.join("readme.txt"), "not a copy").unwrap();

        // Only the On face on the second page uses the image.
        let mut profile = Profile::new("P", vec![Page::new("A"), Page::new("B")]);
        profile
            .page_mut(1)
            .unwrap()
            .update(KEY_3, |c| c.face_mut(true).image = Some(used.clone()));

        assert_eq!(store.remove_unused_images(&profile).unwrap(), 1);
        assert_eq!(
            files(&images),
            [image_name(&used).unwrap(), "readme.txt"],
            "{unused} is gone"
        );
        assert_eq!(store.remove_unused_images(&profile).unwrap(), 0);
    }

    #[test]
    fn remove_unused_images_does_nothing_with_no_image_directory() {
        let (_dir, store) = store();
        assert_eq!(store.remove_unused_images(&Profile::default()).unwrap(), 0);
    }

    #[test]
    fn a_page_background_saves_with_its_text_form_and_loads_again() {
        let (dir, store) = store();
        let image = store
            .import_background(&image_file(dir.path(), "a.png", 900, 600), (800, 480))
            .unwrap();
        let mut profile = Profile::new("P", vec![Page::new("A"), Page::new("B"), Page::new("C")]);
        profile.page_mut(0).unwrap().background = Some(Background::Image {
            image: image.clone(),
            fit: BackgroundFit::Fill,
        });
        profile.page_mut(1).unwrap().background = Some(Background::Image {
            image: image.clone(),
            fit: BackgroundFit::Fit,
        });
        profile.page_mut(2).unwrap().background = Some(Background::Color(Color::rgb(1, 2, 3)));
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(
            text.contains(&format!("[pages.background]\nimage = \"{image}\"\n\n")),
            "the default fit has no key: {text}"
        );
        assert!(
            text.contains(&format!("image = \"{image}\"\nfit = \"fit\"\n")),
            "{text}"
        );
        assert!(
            text.contains("[pages.background]\ncolor = \"#010203\"\n"),
            "{text}"
        );
        assert_eq!(store.load().unwrap().profile, profile);
        // The background image is in use: it stays.
        assert_eq!(store.remove_unused_images(&profile).unwrap(), 0);
        assert_eq!(
            profile.image_paths().collect::<Vec<_>>(),
            [image.as_str(), image.as_str()]
        );

        let page = "schema_version = 4\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"A\"\n";
        for bad in [
            "[pages.background]\n",
            "[pages.background]\ncolor = \"#000000\"\nimage = \"images/0123456789abcdef.png\"\n",
            "[pages.background]\ncolor = \"blue\"\n",
            "[pages.background]\ncolor = \"#000000\"\nfit = \"fit\"\n",
            "[pages.background]\nimage = \"../x.png\"\n",
            "[pages.background]\nimage = \"images/0123456789abcdef.png\"\nfit = \"stretch\"\n",
            "[pages.background]\nvideo = \"x\"\n",
        ] {
            let text = format!("{page}{bad}");
            assert!(load_text(&store, text.as_bytes()).backup.is_some(), "{bad}");
        }
        let old = "schema_version = 3\nname = \"x\"\nmodel = \"m\"\n[[pages]]\nname = \"A\"\n\
                   [pages.background]\ncolor = \"#000000\"\n";
        assert!(load_text(&store, old.as_bytes()).backup.is_some());
    }

    #[test]
    fn a_background_copy_covers_the_background_and_has_no_transparency() {
        let size = (800, 480);
        // Larger: the smallest size that covers the background.
        assert_eq!(background_copy_size(1600, 960, size), (800, 480));
        assert_eq!(background_copy_size(4000, 3000, size), (800, 600));
        // A high image: the limit of the height applies.
        assert_eq!(background_copy_size(3000, 4000, size), (720, 960));
        // Smaller, or covers in one direction only: no change.
        assert_eq!(background_copy_size(400, 240, size), (400, 240));
        assert_eq!(background_copy_size(1000, 300, size), (1000, 300));
        // A very long image keeps at most two times the background on a side.
        assert_eq!(background_copy_size(8000, 500, size), (1600, 100));
        assert_eq!(background_copy_size(500, 8000, size), (60, 960));
        assert_eq!(background_copy_size(0, 10, size), (0, 10));

        let (dir, store) = store();
        let source = image_file(dir.path(), "large.png", 2000, 1200);
        let copy = store
            .load_image(&store.import_background(&source, size).unwrap())
            .unwrap();
        assert_eq!((copy.width(), copy.height()), size);
        assert!(
            !copy.color().has_alpha(),
            "a background has no transparency"
        );
        // A transparent pixel is black.
        let source = image_file(dir.path(), "small.png", 20, 10);
        let copy = store
            .load_image(&store.import_background(&source, size).unwrap())
            .unwrap()
            .into_rgb8();
        assert_eq!(copy.get_pixel(0, 0), &image::Rgb([0, 0, 0]));
        assert_eq!(copy.get_pixel(10, 5), &image::Rgb([10, 5, 7]));
        let source = dir.path().join("not.png");
        fs::write(&source, b"not an image").unwrap();
        assert!(store.import_background(&source, size).is_err());
    }

    #[test]
    fn the_hash_is_fnv_1a() {
        // The test vectors of the FNV specification.
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_f739_67e8);
    }
}
