//! The settings of the shell, in the file `shell.toml` (decision 0020).
//!
//! These settings are not engine state: a different shell has different
//! settings (rule 12). Thus the shell reads and writes this file itself. The
//! file is in the parent directory of the profiles directory, next to the
//! lock file of decision 0019.

use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};

pub const FILE_NAME: &str = "shell.toml";

/// The settings of the shell. A missing file or a missing value gives the
/// default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Show the Dock icon. If false, the application uses the accessory
    /// activation policy.
    pub show_dock_icon: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_dock_icon: true,
        }
    }
}

/// The directory of the files of the shell: the parent directory of the
/// profiles directory. A profiles directory with no parent gives the
/// current directory.
pub fn config_dir(profiles_dir: &Path) -> PathBuf {
    match profiles_dir.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// Read the settings from `dir`. If the file has an error, the result is the
/// default settings and a text for the user.
pub fn load(dir: &Path) -> (Settings, Option<String>) {
    let path = dir.join(FILE_NAME);
    let problem = |error: &dyn std::fmt::Display| {
        Some(format!(
            "OpenCraft cannot read {}, thus it uses the default settings of the shell: {error}",
            path.display()
        ))
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => match toml::from_str(&text) {
            Ok(settings) => (settings, None),
            Err(error) => (Settings::default(), problem(&error.message())),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => (Settings::default(), None),
        Err(error) => (Settings::default(), problem(&error)),
    }
}

/// Write the settings to `dir`. The function writes a temporary file and
/// then renames it, so that a stop during the write does not damage the
/// file.
pub fn save(dir: &Path, settings: &Settings) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let text = toml::to_string(settings).map_err(io::Error::other)?;
    let temporary = dir.join(format!("{FILE_NAME}.tmp"));
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, dir.join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_gives_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let (settings, problem) = load(dir.path());
        assert_eq!(settings, Settings::default());
        assert!(settings.show_dock_icon);
        assert_eq!(problem, None);
    }

    #[test]
    fn save_then_load_gives_the_same_settings() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("new");
        let hidden = Settings {
            show_dock_icon: false,
        };
        save(&config, &hidden).unwrap();
        assert_eq!(
            std::fs::read_to_string(config.join(FILE_NAME)).unwrap(),
            "show_dock_icon = false\n"
        );
        assert_eq!(load(&config), (hidden, None));
        save(&config, &Settings::default()).unwrap();
        assert_eq!(load(&config), (Settings::default(), None));
        assert!(!config.join("shell.toml.tmp").exists());
    }

    #[test]
    fn a_missing_value_gives_the_default_and_an_unknown_key_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_NAME), "later_setting = 3\n").unwrap();
        assert_eq!(load(dir.path()), (Settings::default(), None));
    }

    #[test]
    fn a_file_with_an_error_gives_the_default_and_a_problem() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_NAME), "show_dock_icon = \"no\"\n").unwrap();
        let (settings, problem) = load(dir.path());
        assert_eq!(settings, Settings::default());
        assert!(problem.unwrap().contains("cannot read"));
    }

    #[test]
    fn the_files_of_the_shell_are_next_to_the_profiles_directory() {
        assert_eq!(
            config_dir(Path::new("/a/OpenCraft/profiles")),
            Path::new("/a/OpenCraft")
        );
        assert_eq!(config_dir(Path::new("profiles")), Path::new("."));
    }
}
