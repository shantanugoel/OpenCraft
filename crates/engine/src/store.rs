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
//! Version 3 added the key `model`: the model identifier of the descriptor.
//! Version 2 added bindings. A file of version 1 or 2 loads with the model of
//! the first descriptor (see [`crate::profile::default_model`]), and version 1
//! loads with no bindings. Nothing else changes, so the load loses nothing.
//! The next write writes version 4.
//!
//! A later task can add more keys to version 4 (for example the icon of a
//! surface, #190, and the background of a page, #191), if no release of
//! OpenCraft writes version 4 before that task. Else the task increases the
//! version.
//!
//! ```toml
//! schema_version = 4
//! name = "Default"
//! model = "n4-pro"
//! [[pages]]
//! name = "Page 1"
//! [pages.surfaces.key-face-3]
//! label = "Mail"
//! label_position = "top"
//! label_size = 14
//! [pages.bindings.key-3.press]
//! kind = "opencraft.hotkey"
//! [pages.bindings.key-3.press.settings]
//! hotkey = "cmd+shift+4"
//! ```
//!
//! Control keys are `key-N`, `knob-N`, `bar-panel-N`, and `touch-bar-N`.
//! Gesture keys are `press`, `release`, `turn-left`, `turn-right`, `push`,
//! `tap`, `swipe-left`, and `swipe-right`. Settings keep text, integer,
//! boolean, list, and table values.
//!
//! A file of version 2 can have the gesture keys `tap`, `swipe-left`, and
//! `swipe-right`: they came in the same phase, before version 3. An engine
//! that does not know them keeps the file as a backup, because the file has
//! an error for it (decision 0009).

use crate::action::{Action, GestureKind};
use crate::error::{Error, Result};
use crate::profile::{control_key, parse_control_key};
use crate::profile::{
    default_model, parse_surface_key, surface_key, Color, LabelPosition, LabelStyle, Page, Profile,
    SurfaceContent, MAX_LABEL_SIZE, MIN_LABEL_SIZE,
};
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};

/// The schema version that this engine writes.
pub const SCHEMA_VERSION: u32 = 4;

/// The first schema version with the label options of a surface.
const LABEL_OPTIONS_VERSION: u32 = 4;

/// The first schema version with the key `model`.
const MODEL_VERSION: u32 = 3;

/// The name of the profile file in the profile directory.
pub const PROFILE_FILE: &str = "profile.toml";

/// The name of the image directory in the profile directory.
pub const IMAGE_DIR: &str = "images";

/// The largest width and height of an image copy, in pixels. A larger image
/// becomes smaller when the user selects it.
pub const IMAGE_LIMIT: u32 = 512;

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

/// One profile directory.
#[derive(Debug, Clone)]
pub struct ProfileStore {
    dir: PathBuf,
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

    /// The name for the user and the model of the profile, from the top keys
    /// of the file only. This read does not check the rest of the file, so a
    /// profile with an error stays in the list of profiles. Each part that
    /// the read cannot find is `None`. With no readable file, both are
    /// `None`.
    pub fn summary(&self) -> (Option<String>, Option<String>) {
        let Ok(text) = fs::read_to_string(self.profile_path()) else {
            return (None, None);
        };
        let Ok(table) = text.parse::<toml::Table>() else {
            return (None, None);
        };
        let text_of = |key: &str| table.get(key).and_then(|v| v.as_str()).map(str::to_string);
        (text_of("name"), text_of("model"))
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
        let image_error = |source_error| Error::Image {
            path: source.to_path_buf(),
            source: source_error,
        };
        let image = ImageReader::open(source)
            .map_err(|e| io_error("cannot read", source, e))?
            .with_guessed_format()
            .map_err(|e| io_error("cannot read", source, e))?
            .decode()
            .map_err(image_error)?;
        let image = if image.width() > IMAGE_LIMIT || image.height() > IMAGE_LIMIT {
            image.resize(IMAGE_LIMIT, IMAGE_LIMIT, FilterType::Lanczos3)
        } else {
            image
        };
        // Keep the transparency: the colour of the surface shows through it.
        let image = DynamicImage::ImageRgba8(image.into_rgba8());
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
        let used: HashSet<&str> = profile.image_paths().filter_map(image_name).collect();
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
fn image_name(relative: &str) -> Option<&str> {
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

/// The fields of schema versions 1 to 4. Version 3 adds `model`, and
/// version 4 adds the label options of a surface.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileData {
    schema_version: u32,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(default)]
    pages: Vec<PageData>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageData {
    name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    surfaces: BTreeMap<String, SurfaceData>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    bindings: BTreeMap<String, BTreeMap<GestureKind, Action>>,
}

/// The content of one surface. The keys that start with `label_` came in
/// version 4.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<String>,
}

impl SurfaceData {
    fn has_label_options(&self) -> bool {
        self.label_position.is_some()
            || self.label_size.is_some()
            || self.label_color.is_some()
            || self.label_bold.is_some()
    }

    /// The label style of the file data. The error text names the key.
    fn label_style(&self) -> std::result::Result<LabelStyle, String> {
        let default = LabelStyle::default();
        let position = match &self.label_position {
            None => None,
            Some(text) => Some(
                LabelPosition::from_key(text)
                    .ok_or_else(|| format!("label_position '{text}' is not a position"))?,
            ),
        };
        let size = match self.label_size {
            None => default.size,
            Some(size) if (MIN_LABEL_SIZE..=MAX_LABEL_SIZE).contains(&size) => size,
            Some(size) => {
                return Err(format!(
                    "label_size {size} is not from {MIN_LABEL_SIZE} to {MAX_LABEL_SIZE}"
                ))
            }
        };
        let color = match &self.label_color {
            None => None,
            Some(text) => Some(
                text.parse::<Color>()
                    .map_err(|e| format!("label_color '{text}': {e}"))?,
            ),
        };
        Ok(LabelStyle {
            position,
            size,
            color,
            bold: self.label_bold.unwrap_or(default.bold),
        })
    }

    fn from_content(content: &SurfaceContent) -> Self {
        let style = content.label_style;
        let default = LabelStyle::default();
        Self {
            label: content.label.clone(),
            label_position: style.position.map(|p| p.key().to_string()),
            label_size: (style.size != default.size).then_some(style.size),
            label_color: style.color.map(|c| c.to_string()),
            label_bold: (style.bold != default.bold).then_some(style.bold),
            color: content.color.map(|c| c.to_string()),
            image: content.image.clone(),
        }
    }
}

/// Read the text of a profile file.
fn parse(text: &str) -> std::result::Result<Profile, BackupReason> {
    let invalid = |message: String| BackupReason::Invalid(message);
    let header: Header = toml::from_str(text).map_err(|e| invalid(e.to_string()))?;
    match header.schema_version {
        1..=SCHEMA_VERSION => {}
        0 => return Err(invalid("schema version 0 does not exist".to_string())),
        found => return Err(BackupReason::NewerSchema { found }),
    }
    let file: FileData = toml::from_str(text).map_err(|e| invalid(e.to_string()))?;
    let version = header.schema_version;
    let model = match (version >= MODEL_VERSION, file.model) {
        (true, Some(model)) if !model.is_empty() => model,
        (true, _) => return Err(invalid("the file has no model".to_string())),
        (false, None) => default_model().to_string(),
        (false, Some(_)) => return Err(invalid(format!("version {version} has no model"))),
    };

    let mut pages = Vec::new();
    for page_file in file.pages {
        let mut page = Page::new(page_file.name);
        for (key, surface) in page_file.surfaces {
            let id = parse_surface_key(&key)
                .ok_or_else(|| invalid(format!("'{key}' is not a surface")))?;
            let color = match &surface.color {
                Some(text) => Some(
                    text.parse::<Color>()
                        .map_err(|e| invalid(format!("{key}: '{text}': {e}")))?,
                ),
                None => None,
            };
            if let Some(image) = &surface.image {
                if image_name(image).is_none() {
                    return Err(invalid(format!(
                        "{key}: '{image}' is not in the image directory"
                    )));
                }
            }
            if version < LABEL_OPTIONS_VERSION && surface.has_label_options() {
                return Err(invalid(format!(
                    "{key}: version {version} has no label options"
                )));
            }
            let label_style = surface
                .label_style()
                .map_err(|e| invalid(format!("{key}: {e}")))?;
            page.update(id, |content| {
                *content = SurfaceContent {
                    label: surface.label,
                    label_style,
                    color,
                    image: surface.image,
                };
            });
        }
        if header.schema_version == 1 && !page_file.bindings.is_empty() {
            return Err(invalid("version 1 has no bindings".into()));
        }
        for (key, bindings) in page_file.bindings {
            let control = parse_control_key(&key)
                .ok_or_else(|| invalid(format!("'{key}' is not a control")))?;
            for (gesture, action) in bindings {
                page.set_binding(control, gesture, Some(action));
            }
        }
        pages.push(page);
    }
    Ok(Profile::for_model(model, file.name, pages))
}

/// The text of the profile file for `profile`.
fn to_text(profile: &Profile) -> Result<String> {
    let file = FileData {
        schema_version: SCHEMA_VERSION,
        name: profile.name.clone(),
        model: Some(profile.model.clone()),
        pages: profile
            .pages()
            .iter()
            .map(|page| PageData {
                name: page.name.clone(),
                bindings: {
                    let mut bindings: BTreeMap<String, BTreeMap<GestureKind, Action>> =
                        BTreeMap::new();
                    for ((control, gesture), action) in page.bindings() {
                        bindings
                            .entry(control_key(control))
                            .or_default()
                            .insert(gesture, action.clone());
                    }
                    bindings
                },
                surfaces: page
                    .surfaces()
                    .map(|(id, content)| (surface_key(id), SurfaceData::from_content(content)))
                    .collect(),
            })
            .collect(),
    };
    Ok(toml::to_string_pretty(&file)?)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        store.save(&sample()).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        let want = r##"schema_version = 4
name = "Default"
model = "n4-pro"

[[pages]]
name = "Page 1"

[pages.surfaces.bar-panel-1]
label = """
Volume "1"
line 2"""

[pages.surfaces.key-face-3]
label = "Mail"
color = "#1E6FD9"
image = "images/4f1c2a9be07d3356.png"
"##;
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
    fn a_file_with_a_newer_schema_version_is_kept_as_a_backup() {
        let (_dir, store) = store();
        let text = "schema_version = 5\nname = \"Later\"\nnew_field = true\n";
        let loaded = load_text(&store, text.as_bytes());

        assert_eq!(loaded.profile, Profile::default());
        let backup = loaded.backup.unwrap();
        assert_eq!(backup.reason, BackupReason::NewerSchema { found: 5 });
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

        // The next write is version 4 with the same content and the model.
        store.save(&profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(text.starts_with("schema_version = 4\nname = \"Old\"\nmodel = \"n4-pro\"\n"));
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
        // The load did not write the file. The next write is version 4, and
        // it writes no label option.
        assert_eq!(fs::read_to_string(store.profile_path()).unwrap(), old);
        store.save(&loaded.profile).unwrap();
        let text = fs::read_to_string(store.profile_path()).unwrap();
        assert!(text.starts_with("schema_version = 4\n"), "{text}");
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
                position: Some(LabelPosition::Top),
                size: 14,
                color: Some(Color::rgb(0xFF, 0xCE, 0x73)),
                bold: false,
            }
        });
        page.update(BAR_1, |c| {
            c.label_style.position = Some(LabelPosition::Hidden);
        });
        // Only a style, with no label.
        let key_1 = SurfaceId::new(SurfaceKind::KeyFace, 1);
        page.update(key_1, |c| {
            c.label_style.position = Some(LabelPosition::Bottom)
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
                c.label_style.position = Some(position);
            });
            store.save(&profile).unwrap();
            assert_eq!(store.load().unwrap().profile, profile, "{position:?}");
        }
    }

    #[test]
    fn a_profile_of_a_different_model_saves_and_loads_with_its_model() {
        let (_dir, store) = store();
        let profile = Profile::for_model("other-model", "Other", vec![Page::new("A")]);
        store.save(&profile).unwrap();
        assert_eq!(store.load().unwrap().profile, profile);
        assert_eq!(store.read().unwrap(), profile);
        assert_eq!(
            store.summary(),
            (Some("Other".to_string()), Some("other-model".to_string()))
        );
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
        assert_eq!(store.summary(), (None, None));
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

        // Only the second page uses the image.
        let mut profile = Profile::new("P", vec![Page::new("A"), Page::new("B")]);
        profile
            .page_mut(1)
            .unwrap()
            .update(KEY_3, |c| c.image = Some(used.clone()));

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
    fn the_hash_is_fnv_1a() {
        // The test vectors of the FNV specification.
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_f739_67e8);
    }
}
