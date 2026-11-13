//! The profiles directory: one directory for each profile, the record of the
//! active profile, and the directory of removed profiles.
//!
//! - Each directory that has a profile file is one profile. Its content is
//!   in [`crate::store`]. The identity of a profile is the name of its
//!   directory. The engine makes the name from the name that the user gives,
//!   and does not change it when the user renames the profile.
//! - The file `state.toml` holds the identity of the active profile for each
//!   device model.
//! - The engine does not delete a profile. A removal moves the directory into
//!   the directory `removed`.
//!
//! The reasons are in `docs/decisions/0018-many-profiles.md`.
//!
//! ```toml
//! # state.toml
//! [active]
//! n4-pro = "default"
//! ```

use crate::error::Result;
use crate::profile::{default_model, Color, Profile};
use crate::store::{io_error, write_and_rename, ProfileStore, PROFILE_FILE};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The name of the file with the active profile of each model.
pub const STATE_FILE: &str = "state.toml";

/// The name of the directory that holds the removed profiles.
pub const REMOVED_DIR: &str = "removed";

/// The longest identity that the engine makes, in characters.
const ID_LIMIT: usize = 64;

/// The identity for a name that has no letter and no digit.
const FALLBACK_ID: &str = "profile";

/// One profile in the list for a shell.
///
/// The pages and the first page let a shell show the profile settings of a
/// profile that is not active. For the active profile, they are the values
/// of the last [`crate::Event::Profiles`]: [`crate::Event::Pages`] gives the
/// current values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileEntry {
    /// The identity: the name of the directory. It does not change.
    pub id: String,
    /// The name for the user.
    pub name: String,
    /// The colour of the profile for the user. `None`: no colour.
    pub color: Option<Color>,
    /// The names of the pages, in their order.
    pub pages: Vec<String>,
    /// The number of the first page, from 0: see [`Profile::first_page`].
    pub first_page: usize,
}

impl ProfileEntry {
    /// The entry of `profile` with the identity `id`.
    pub fn of(id: impl Into<String>, profile: &Profile) -> Self {
        Self {
            id: id.into(),
            name: profile.name.clone(),
            color: profile.color,
            pages: profile.pages().iter().map(|p| p.name.clone()).collect(),
            first_page: profile.first_page(),
        }
    }
}

/// The profiles directory.
#[derive(Debug, Clone)]
pub struct ProfilesDir {
    dir: PathBuf,
}

impl ProfilesDir {
    /// Use the profiles directory `dir`. The directory does not have to
    /// exist: the first write makes it.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The store of the profile with the identity `id`.
    pub fn store(&self, id: &str) -> ProfileStore {
        ProfileStore::new(self.dir.join(id))
    }

    /// Each profile of the model `model`, in the order of the identities.
    ///
    /// The list reads only the top keys of each file: see
    /// [`ProfileStore::summary`]. Thus a profile with an error in its file
    /// stays in the list, and the engine does not change it until the user
    /// selects it. A file with no readable model has the model of the first
    /// descriptor, as a file of schema version 2. A file with no readable
    /// name has the identity as its name.
    pub fn list(&self, model: &str) -> Result<Vec<ProfileEntry>> {
        let mut list = Vec::new();
        for id in self.ids()? {
            let summary = self.store(&id).summary();
            if summary.model.as_deref().unwrap_or(default_model()) == model {
                let name = summary.name.unwrap_or_else(|| id.clone());
                list.push(ProfileEntry {
                    id,
                    name,
                    color: summary.color,
                    pages: summary.pages,
                    first_page: summary.first_page,
                });
            }
        }
        Ok(list)
    }

    /// True if a directory has a profile file, for any model. With no
    /// profile, the shell shows the first start (decision 0029, section
    /// 18). A directory that cannot be read has no profile.
    pub fn has_profiles(&self) -> bool {
        self.ids().is_ok_and(|ids| !ids.is_empty())
    }

    /// The identity of each directory that has a profile file, in order.
    fn ids(&self) -> Result<Vec<String>> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io_error("cannot read", &self.dir, e)),
        };
        let mut ids = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| io_error("cannot read", &self.dir, e))?;
            let Ok(id) = entry.file_name().into_string() else {
                continue;
            };
            let path = entry.path();
            if id != REMOVED_DIR
                && !id.starts_with('.')
                && path.is_dir()
                && path.join(PROFILE_FILE).exists()
            {
                ids.push(id);
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// The identity of the active profile of `model` in `state.toml`.
    /// `None` if the file is missing, cannot be read, or has no entry for the
    /// model. The caller checks that the profile exists.
    pub fn active(&self, model: &str) -> Option<String> {
        let table = self.state()?;
        let active = table.get("active")?.as_table()?;
        Some(active.get(model)?.as_str()?.to_string())
    }

    /// Record `id` as the active profile of `model` in `state.toml`. The
    /// entries of other models stay.
    pub fn set_active(&self, model: &str, id: &str) -> Result<()> {
        let mut table = self.state().unwrap_or_default();
        let active = table
            .entry("active")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if !active.is_table() {
            *active = toml::Value::Table(toml::Table::new());
        }
        if let Some(active) = active.as_table_mut() {
            active.insert(model.to_string(), toml::Value::String(id.to_string()));
        }
        let text = toml::to_string_pretty(&table)?;
        self.make_dir()?;
        write_and_rename(&self.dir.join(STATE_FILE), text.as_bytes())
    }

    /// The content of `state.toml`, or `None` if it is missing or has an
    /// error. The engine replaces a file with an error at the next write: it
    /// holds no data of the user other than the active profile.
    fn state(&self) -> Option<toml::Table> {
        fs::read_to_string(self.dir.join(STATE_FILE))
            .ok()?
            .parse()
            .ok()
    }

    /// A new identity for the name `name`: small letters, digits, and `-`,
    /// with a number at the end if the identity is in use. An identity is in
    /// use if an entry in the profiles directory has it, with no difference
    /// between small and capital letters, or if it is in `taken`.
    pub fn new_id(&self, name: &str, taken: &[&str]) -> Result<String> {
        let mut used: Vec<String> = taken.iter().map(|id| id.to_lowercase()).collect();
        used.push(REMOVED_DIR.to_string());
        match fs::read_dir(&self.dir) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry.map_err(|e| io_error("cannot read", &self.dir, e))?;
                    used.push(entry.file_name().to_string_lossy().to_lowercase());
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error("cannot read", &self.dir, e)),
        }
        Ok(free_name(&id_from_name(name), |id| {
            used.iter().any(|u| u == id)
        }))
    }

    /// Move the directory of the profile `id` into the directory `removed`.
    /// If `removed` has a directory with that name, the new name gets a
    /// number at the end. Return the new path.
    pub fn remove(&self, id: &str) -> Result<PathBuf> {
        let removed = self.dir.join(REMOVED_DIR);
        fs::create_dir_all(&removed)
            .map_err(|e| io_error("cannot make the directory", &removed, e))?;
        let name = free_name(id, |candidate| removed.join(candidate).exists());
        let from = self.dir.join(id);
        let to = removed.join(name);
        fs::rename(&from, &to).map_err(|e| io_error("cannot move", &from, e))?;
        Ok(to)
    }

    fn make_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| io_error("cannot make the directory", &self.dir, e))
    }
}

/// The identity for the name `name`: each letter and digit of ASCII, in
/// small letters, and one `-` for each group of other characters. A name
/// with no letter and no digit gives [`FALLBACK_ID`].
fn id_from_name(name: &str) -> String {
    let mut id = String::new();
    for c in name.chars() {
        if id.len() >= ID_LIMIT {
            break;
        }
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    let id = id.trim_end_matches('-');
    if id.is_empty() {
        FALLBACK_ID.to_string()
    } else {
        id.to_string()
    }
}

/// `base` if it is not in use, else `base-2`, `base-3`, and so on.
fn free_name(base: &str, in_use: impl Fn(&str) -> bool) -> String {
    if !in_use(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|candidate| !in_use(candidate))
        .expect("a name that is not in use")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn profiles() -> (TempDir, ProfilesDir) {
        let dir = TempDir::new().unwrap();
        let profiles = ProfilesDir::new(dir.path().join("profiles"));
        (dir, profiles)
    }

    fn save(profiles: &ProfilesDir, id: &str, model: &str, name: &str) {
        profiles
            .store(id)
            .save(&Profile::for_model(model, name, Vec::new()))
            .unwrap();
    }

    #[test]
    fn an_identity_comes_from_the_name_of_the_user() {
        for (name, id) in [
            ("Work", "work"),
            ("  Video Calls!  ", "video-calls"),
            ("A--b__c", "a-b-c"),
            ("Phase 4", "phase-4"),
            ("Café", "caf"),
            ("日本", FALLBACK_ID),
            ("", FALLBACK_ID),
            ("../secret", "secret"),
        ] {
            assert_eq!(id_from_name(name), id, "{name}");
        }
        assert_eq!(id_from_name(&"x".repeat(200)).len(), ID_LIMIT);
    }

    #[test]
    fn an_identity_in_use_gets_a_number_at_the_end() {
        let (_dir, profiles) = profiles();
        assert_eq!(profiles.new_id("Work", &[]).unwrap(), "work");
        save(&profiles, "work", "n4-pro", "Work");
        assert_eq!(profiles.new_id("Work", &[]).unwrap(), "work-2");
        assert_eq!(profiles.new_id("work", &["work-2"]).unwrap(), "work-3");
        // A directory with capital letters, a file, and the directory of the
        // removed profiles are in use too.
        fs::create_dir_all(profiles.dir().join("Media")).unwrap();
        assert_eq!(profiles.new_id("media", &[]).unwrap(), "media-2");
        assert_eq!(profiles.new_id("Removed", &[]).unwrap(), "removed-2");
    }

    #[test]
    fn the_list_has_the_profiles_of_one_model_in_the_order_of_the_identities() {
        let (_dir, profiles) = profiles();
        assert!(profiles.list("n4-pro").unwrap().is_empty());
        assert!(!profiles.has_profiles());
        fs::create_dir_all(profiles.dir().join("empty")).unwrap();
        assert!(!profiles.has_profiles(), "a directory with no profile file");
        save(&profiles, "work", "n4-pro", "Work");
        assert!(profiles.has_profiles());
        save(&profiles, "default", "n4-pro", "Default");
        save(&profiles, "other", "other-model", "Other");
        // Not a profile: no profile file, a file, a hidden directory, and the
        // removed profiles.
        fs::create_dir_all(profiles.dir().join("empty")).unwrap();
        fs::write(profiles.dir().join("notes.txt"), "x").unwrap();
        save(&profiles, ".hidden", "n4-pro", "Hidden");
        save(&profiles, "removed", "n4-pro", "Removed");

        let entry = |id: &str, name: &str| ProfileEntry {
            id: id.into(),
            name: name.into(),
            pages: vec!["Page 1".into()],
            ..Default::default()
        };
        assert_eq!(
            profiles.list("n4-pro").unwrap(),
            [entry("default", "Default"), entry("work", "Work")]
        );
        assert_eq!(
            profiles.list("other-model").unwrap(),
            [entry("other", "Other")]
        );
    }

    #[test]
    fn a_profile_with_an_error_stays_in_the_list_and_does_not_change() {
        let (_dir, profiles) = profiles();
        let store = profiles.store("broken");
        fs::create_dir_all(store.dir()).unwrap();
        let text = "this is not TOML [";
        fs::write(store.profile_path(), text).unwrap();

        let list = profiles.list(default_model()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(
            (list[0].id.as_str(), list[0].name.as_str()),
            ("broken", "broken")
        );
        assert!(store.read().is_err());
        assert_eq!(fs::read_to_string(store.profile_path()).unwrap(), text);
    }

    #[test]
    fn the_active_profile_of_each_model_goes_to_the_state_file_and_back() {
        let (_dir, profiles) = profiles();
        assert_eq!(profiles.active("n4-pro"), None);
        profiles.set_active("n4-pro", "work").unwrap();
        profiles.set_active("other-model", "other").unwrap();
        profiles.set_active("n4-pro", "default").unwrap();
        assert_eq!(profiles.active("n4-pro").as_deref(), Some("default"));
        assert_eq!(profiles.active("other-model").as_deref(), Some("other"));
        let text = fs::read_to_string(profiles.dir().join(STATE_FILE)).unwrap();
        assert!(text.contains("[active]"), "{text}");
        assert!(text.contains("n4-pro = \"default\""), "{text}");

        // A file with an error is no file, and the next write replaces it.
        fs::write(profiles.dir().join(STATE_FILE), "[active\n").unwrap();
        assert_eq!(profiles.active("n4-pro"), None);
        fs::write(profiles.dir().join(STATE_FILE), "active = 3\n").unwrap();
        assert_eq!(profiles.active("n4-pro"), None);
        profiles.set_active("n4-pro", "work").unwrap();
        assert_eq!(profiles.active("n4-pro").as_deref(), Some("work"));
    }

    #[test]
    fn a_removal_moves_the_directory_and_keeps_an_earlier_removal() {
        let (_dir, profiles) = profiles();
        for round in 1..=2 {
            save(&profiles, "work", "n4-pro", &format!("Work {round}"));
            fs::write(profiles.store("work").dir().join("keep.txt"), "x").unwrap();
            let to = profiles.remove("work").unwrap();
            assert!(!profiles.dir().join("work").exists());
            assert!(to.join("keep.txt").exists());
            let name = to.file_name().unwrap().to_str().unwrap().to_string();
            assert_eq!(name, if round == 1 { "work" } else { "work-2" });
        }
        let first = ProfileStore::new(profiles.dir().join(REMOVED_DIR).join("work"));
        assert_eq!(first.read().unwrap().name, "Work 1");
        assert!(profiles.list("n4-pro").unwrap().is_empty());
    }
}
