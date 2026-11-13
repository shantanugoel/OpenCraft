//! The settings of the shell, in the file `shell.toml` (decision 0020).
//!
//! These settings are not engine state: a different shell has different
//! settings (rule 12). Thus the shell reads and writes this file itself. The
//! file is in the parent directory of the profiles directory, next to the
//! lock file of decision 0019.
//!
//! The file also keeps the panes of the window (decision 0029, section 4),
//! the options of the Settings window, the pane of that window that was
//! visible last (decision 0029, section 17), and the icons that the user
//! chose last in the icon picker (section 14).

use crate::view::theme;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};

pub const FILE_NAME: &str = "shell.toml";

/// The settings of the shell. A missing file or a missing value gives the
/// default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Show the Dock icon. If false, the application uses the accessory
    /// activation policy.
    pub show_dock_icon: bool,
    /// Open the window at a start that is not a start at login (decision
    /// 0029, section 17). A start at login never opens the window (decision
    /// 0021).
    pub show_window_at_start: bool,
    /// The pane of the Settings window that was visible last.
    #[serde(deserialize_with = "settings_pane")]
    pub settings_pane: SettingsPane,
    #[serde(flatten)]
    pub panes: Panes,
    /// The icons that the user chose last in the icon picker, the newest
    /// first: names of the icon set of the engine. The tab "Recent" shows
    /// them (decision 0029, section 14).
    #[serde(
        deserialize_with = "recent_icons",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub recent_icons: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_dock_icon: true,
            show_window_at_start: true,
            settings_pane: SettingsPane::default(),
            panes: Panes::default(),
            recent_icons: Vec::new(),
        }
    }
}

impl Settings {
    /// Put the icon `name` at the start of [`Settings::recent_icons`]. The
    /// list has each name one time, and keeps [`theme::RECENT_ICONS`] names
    /// at the most. Return true if the list changes.
    pub fn add_recent_icon(&mut self, name: &str) -> bool {
        if self.recent_icons.first().map(String::as_str) == Some(name) {
            return false;
        }
        self.recent_icons.retain(|n| n != name);
        self.recent_icons.insert(0, name.to_string());
        self.recent_icons.truncate(theme::RECENT_ICONS);
        true
    }
}

/// A pane of the Settings window, in the order of its tabs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingsPane {
    #[default]
    General,
    Shortcuts,
    About,
}

impl SettingsPane {
    /// The panes, in the order of the tabs. The tabs "Plug-ins" and "Updates"
    /// are places for later (decision 0029, section 21): they are not here.
    pub const ALL: [SettingsPane; 3] = [Self::General, Self::Shortcuts, Self::About];

    /// The name of the pane, for its tab and for the title of the window.
    pub fn name(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Shortcuts => "Shortcuts",
            Self::About => "About",
        }
    }
}

/// The tab that the right panel shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RightPanelTab {
    #[default]
    Inspector,
    Library,
}

/// The panes of the window: which side panes show, their widths in points,
/// and the tab of the right panel. A missing or incorrect value gives the
/// default, and the other values stay. A width out of the range of its pane
/// is incorrect.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Panes {
    #[serde(default = "yes", deserialize_with = "visible")]
    pub sidebar_visible: bool,
    #[serde(default = "yes", deserialize_with = "visible")]
    pub inspector_visible: bool,
    #[serde(default = "sidebar_default", deserialize_with = "sidebar_width")]
    pub sidebar_width: f32,
    #[serde(default = "inspector_default", deserialize_with = "inspector_width")]
    pub inspector_width: f32,
    #[serde(default, deserialize_with = "tab")]
    pub right_panel_tab: RightPanelTab,
}

impl Default for Panes {
    fn default() -> Self {
        Self {
            sidebar_visible: true,
            inspector_visible: true,
            sidebar_width: theme::SIDEBAR_WIDTH,
            inspector_width: theme::INSPECTOR_WIDTH,
            right_panel_tab: RightPanelTab::Inspector,
        }
    }
}

fn yes() -> bool {
    true
}

fn sidebar_default() -> f32 {
    theme::SIDEBAR_WIDTH
}

fn inspector_default() -> f32 {
    theme::INSPECTOR_WIDTH
}

/// Read a value of type `T`. A value of a different type, or a value that
/// `valid` refuses, gives `default`.
fn lenient<'de, D: Deserializer<'de>, T: DeserializeOwned>(
    deserializer: D,
    default: T,
    valid: impl Fn(&T) -> bool,
) -> Result<T, D::Error> {
    let value = toml::Value::deserialize(deserializer)?;
    Ok(value.try_into().ok().filter(valid).unwrap_or(default))
}

fn visible<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    lenient(deserializer, true, |_| true)
}

fn sidebar_width<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    lenient(deserializer, theme::SIDEBAR_WIDTH, |w| {
        (theme::SIDEBAR_MIN..=theme::SIDEBAR_MAX).contains(w)
    })
}

fn inspector_width<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    lenient(deserializer, theme::INSPECTOR_WIDTH, |w| {
        (theme::INSPECTOR_MIN..=theme::INSPECTOR_MAX).contains(w)
    })
}

fn tab<'de, D: Deserializer<'de>>(deserializer: D) -> Result<RightPanelTab, D::Error> {
    lenient(deserializer, RightPanelTab::Inspector, |_| true)
}

fn settings_pane<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SettingsPane, D::Error> {
    lenient(deserializer, SettingsPane::default(), |_| true)
}

/// A list of names. An incorrect value gives an empty list, and the list
/// keeps [`theme::RECENT_ICONS`] names at the most.
fn recent_icons<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let mut names: Vec<String> = lenient(deserializer, Vec::new(), |_| true)?;
    names.truncate(theme::RECENT_ICONS);
    Ok(names)
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
            ..Settings::default()
        };
        save(&config, &hidden).unwrap();
        assert_eq!(
            std::fs::read_to_string(config.join(FILE_NAME)).unwrap(),
            "show_dock_icon = false\nshow_window_at_start = true\nsettings_pane = \"general\"\n\
             sidebar_visible = true\ninspector_visible = true\n\
             sidebar_width = 240.0\ninspector_width = 360.0\nright_panel_tab = \"inspector\"\n"
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
    fn the_options_of_the_settings_window_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        // The first start: the window shows at the start, at the first pane.
        assert!(Settings::default().show_window_at_start);
        assert_eq!(Settings::default().settings_pane, SettingsPane::General);
        let settings = Settings {
            show_window_at_start: false,
            settings_pane: SettingsPane::About,
            ..Settings::default()
        };
        save(dir.path(), &settings).unwrap();
        assert_eq!(load(dir.path()), (settings, None));
        // An unknown pane, for example a pane of a later version, gives the
        // first pane. The other values stay, and the user sees no problem.
        std::fs::write(
            dir.path().join(FILE_NAME),
            "show_window_at_start = false\nsettings_pane = \"plug-ins\"\n",
        )
        .unwrap();
        let (loaded, problem) = load(dir.path());
        assert_eq!(problem, None);
        assert!(!loaded.show_window_at_start);
        assert_eq!(loaded.settings_pane, SettingsPane::General);
        // The panes are the ones of the tabs, in order.
        let names: Vec<_> = SettingsPane::ALL.iter().map(|p| p.name()).collect();
        assert_eq!(names, ["General", "Shortcuts", "About"]);
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
    fn the_panes_are_kept_and_an_incorrect_value_gives_its_default() {
        let dir = tempfile::tempdir().unwrap();
        let panes = Panes {
            sidebar_visible: false,
            inspector_visible: false,
            sidebar_width: 300.,
            inspector_width: 420.,
            right_panel_tab: RightPanelTab::Library,
        };
        let settings = Settings {
            panes,
            ..Settings::default()
        };
        save(dir.path(), &settings).unwrap();
        assert_eq!(load(dir.path()), (settings, None));

        // Each incorrect value gives its default. The correct values stay,
        // and the user sees no problem.
        std::fs::write(
            dir.path().join(FILE_NAME),
            "show_dock_icon = false\nsidebar_visible = \"no\"\ninspector_visible = false\n\
             sidebar_width = 1000\ninspector_width = 400\nright_panel_tab = \"plug-ins\"\n",
        )
        .unwrap();
        assert_eq!(
            load(dir.path()),
            (
                Settings {
                    show_dock_icon: false,
                    panes: Panes {
                        sidebar_visible: true,
                        inspector_visible: false,
                        sidebar_width: theme::SIDEBAR_WIDTH,
                        inspector_width: 400.,
                        right_panel_tab: RightPanelTab::Inspector,
                    },
                    ..Settings::default()
                },
                None
            )
        );
        std::fs::write(dir.path().join(FILE_NAME), "sidebar_width = 120.5\n").unwrap();
        assert_eq!(load(dir.path()).0.panes, Panes::default());
    }

    #[test]
    fn the_recent_icons_are_kept_newest_first_with_no_name_two_times() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        assert!(settings.add_recent_icon("globe"));
        assert!(settings.add_recent_icon("mail"));
        assert!(!settings.add_recent_icon("mail"));
        assert!(settings.add_recent_icon("globe"));
        assert_eq!(settings.recent_icons, ["globe", "mail"]);
        for n in 0..30 {
            settings.add_recent_icon(&format!("icon-{n}"));
        }
        assert_eq!(settings.recent_icons.len(), theme::RECENT_ICONS);
        assert_eq!(settings.recent_icons[0], "icon-29");
        save(dir.path(), &settings).unwrap();
        assert_eq!(load(dir.path()), (settings, None));
        // An incorrect value gives an empty list, and the other values stay.
        std::fs::write(
            dir.path().join(FILE_NAME),
            "show_dock_icon = false\nrecent_icons = \"globe\"\n",
        )
        .unwrap();
        let (loaded, problem) = load(dir.path());
        assert_eq!(problem, None);
        assert!(!loaded.show_dock_icon);
        assert!(loaded.recent_icons.is_empty());
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
