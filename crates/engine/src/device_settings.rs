//! The device settings: the display brightness, the haptic feedback, and the
//! knob lights. The settings are for each device model, and they are not in
//! the profile.
//!
//! - [`DeviceSettings`] holds the settings, with the defaults and the ranges.
//! - [`SettingCommands`] gives the device commands for the settings.
//! - [`load`] and [`save`] read and write the file `device-settings.toml` in
//!   the profiles directory. It has one table for each model identifier.
//!
//! ```toml
//! schema_version = 1
//!
//! [n4-pro]
//! brightness = 80
//! haptic = true
//!
//! [n4-pro.knob_lights]
//! mode = "effect"   # "effect", "colors", or "off"
//! brightness = 100
//! colors = ["#FFFFFF", "#FFFFFF", "#FFFFFF", "#FFFFFF"]
//! ```
//!
//! The reasons are in `docs/decisions/0022-device-settings-and-stop-state.md`.

use crate::error::Result;
use crate::profile::Color;
use crate::store::{io_error, write_and_rename};
use opencraft_core::protocol;
pub use opencraft_core::SettingsSupport;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The name of the settings file in the profiles directory.
pub const SETTINGS_FILE: &str = "device-settings.toml";

/// The schema version that this engine writes.
pub const SETTINGS_SCHEMA_VERSION: i64 = 1;

/// The display brightness of a model with no settings file, in percent. It
/// is the brightness of the earlier phases.
pub const DEFAULT_BRIGHTNESS: u8 = 80;

/// The highest display brightness and knob light brightness, in percent.
pub const MAX_BRIGHTNESS: u8 = 100;

/// The lowest knob light brightness of the setting, in percent. The mode
/// "off" gives 0.
pub const LOWEST_KNOB_BRIGHTNESS: u8 = 1;

/// The knob light brightness of a model with no settings file, in percent.
pub const DEFAULT_KNOB_BRIGHTNESS: u8 = 100;

/// The knob light colour of a model with no settings file.
pub const DEFAULT_KNOB_COLOR: Color = Color::WHITE;

/// What the knob lights show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KnobLightMode {
    /// The default effect of the device: one colour for all knobs that
    /// changes slowly.
    #[default]
    Effect,
    /// One colour for each knob.
    Colors,
    /// The knob lights are off.
    Off,
}

impl KnobLightMode {
    /// The text form in the settings file.
    pub fn name(self) -> &'static str {
        match self {
            KnobLightMode::Effect => "effect",
            KnobLightMode::Colors => "colors",
            KnobLightMode::Off => "off",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        [Self::Effect, Self::Colors, Self::Off]
            .into_iter()
            .find(|m| m.name() == name)
    }
}

/// The settings of the knob lights.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KnobLights {
    pub mode: KnobLightMode,
    /// The brightness, from [`LOWEST_KNOB_BRIGHTNESS`] to [`MAX_BRIGHTNESS`].
    /// The mode "off" keeps it, so that "off" and back gives the last
    /// brightness.
    pub brightness: u8,
    /// One colour for each knob light, knob 1 first. Only the mode "colors"
    /// shows them.
    pub colors: Vec<Color>,
}

/// The device settings of one device model.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceSettings {
    /// The display brightness, from
    /// [`SettingsSupport::lowest_brightness`] to [`MAX_BRIGHTNESS`].
    pub brightness: u8,
    /// Haptic feedback on or off.
    pub haptic: bool,
    pub knob_lights: KnobLights,
}

impl DeviceSettings {
    /// The default settings for a model with `support`. They give the state
    /// of the device of the earlier phases.
    pub fn defaults(support: SettingsSupport) -> Self {
        Self {
            brightness: DEFAULT_BRIGHTNESS,
            haptic: true,
            knob_lights: KnobLights {
                mode: KnobLightMode::Effect,
                brightness: DEFAULT_KNOB_BRIGHTNESS,
                colors: Vec::new(),
            },
        }
        .fit(support)
    }

    /// The settings with each value in its range for a model with
    /// `support`: a value out of range goes to the nearest value in the
    /// range. The list of colours gets one colour for each knob light: a
    /// missing colour is [`DEFAULT_KNOB_COLOR`], and the colours after the
    /// last knob light are removed.
    pub fn fit(mut self, support: SettingsSupport) -> Self {
        let lowest = support.lowest_brightness.min(MAX_BRIGHTNESS);
        self.brightness = self.brightness.clamp(lowest, MAX_BRIGHTNESS);
        let lights = &mut self.knob_lights;
        lights.brightness = lights
            .brightness
            .clamp(LOWEST_KNOB_BRIGHTNESS, MAX_BRIGHTNESS);
        lights
            .colors
            .resize(usize::from(support.knob_lights), DEFAULT_KNOB_COLOR);
        self
    }
}

/// The device commands for a set of settings, in groups. The engine sends
/// all groups after each start sequence, in the order of the fields. At a
/// change of a setting, it sends only the groups that are different. A
/// group is empty for a setting that the model does not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingCommands {
    /// `LIG`.
    pub brightness: Vec<u8>,
    /// `SETLB` and then `LBLIG`, or `LBLIG 0` only for the mode "off".
    pub knob_lights: Vec<Vec<u8>>,
    /// `QUCMD`.
    pub haptic: Vec<Vec<u8>>,
}

impl SettingCommands {
    /// The commands for `settings` on a model with `support`.
    ///
    /// The mode "colors" with all colours black gives the commands of the
    /// mode "off", because the device shows the effect for all colours black
    /// (measured).
    pub fn new(settings: &DeviceSettings, support: SettingsSupport) -> Self {
        let lights = &settings.knob_lights;
        let count = usize::from(support.knob_lights);
        let all_black = lights.colors.iter().all(|c| *c == Color::BLACK);
        let mode = match lights.mode {
            KnobLightMode::Colors if all_black => KnobLightMode::Off,
            mode => mode,
        };
        let knob_lights = if count == 0 {
            Vec::new()
        } else {
            match mode {
                KnobLightMode::Effect => vec![
                    protocol::set_led_colors(&vec![[0, 0, 0]; count]),
                    protocol::set_led_brightness(lights.brightness),
                ],
                KnobLightMode::Colors => {
                    let colors: Vec<[u8; 3]> = lights.colors.iter().map(|c| c.0).collect();
                    vec![
                        protocol::set_led_colors(&colors),
                        protocol::set_led_brightness(lights.brightness),
                    ]
                }
                KnobLightMode::Off => vec![protocol::set_led_brightness(0)],
            }
        };
        let haptic = if support.haptic {
            vec![protocol::set_vibration(settings.haptic)]
        } else {
            Vec::new()
        };
        Self {
            brightness: protocol::set_brightness(settings.brightness),
            knob_lights,
            haptic,
        }
    }

    /// The commands after the start sequence: the knob lights, then the
    /// haptic feedback. The start sequence has the brightness command.
    pub fn after_start(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.knob_lights.iter().chain(&self.haptic)
    }

    /// The commands of the groups that are different in `new`.
    pub fn changes<'a>(&self, new: &'a SettingCommands) -> Vec<&'a Vec<u8>> {
        let mut out = Vec::new();
        if new.brightness != self.brightness {
            out.push(&new.brightness);
        }
        if new.knob_lights != self.knob_lights {
            out.extend(&new.knob_lights);
        }
        if new.haptic != self.haptic {
            out.extend(&new.haptic);
        }
        out
    }
}

/// Why the engine did not use the settings file.
#[derive(Debug)]
pub enum FileProblem {
    /// The file could not be read.
    Read(io::Error),
    /// The file is not valid, for example not TOML or a value of the wrong
    /// type.
    Invalid(String),
    /// The file has a schema version that is newer than this engine knows.
    Newer(i64),
}

impl fmt::Display for FileProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileProblem::Read(e) => write!(f, "it cannot be read: {e}"),
            FileProblem::Invalid(reason) => write!(f, "it is not valid: {reason}"),
            FileProblem::Newer(found) => {
                write!(
                    f,
                    "it has the schema version {found}, from a newer OpenCraft"
                )
            }
        }
    }
}

/// The path of the settings file in the profiles directory `dir`.
pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join(SETTINGS_FILE)
}

/// Read the settings of `model` from the settings file in the profiles
/// directory `dir`.
///
/// A missing file, table, or value gives the default. A value out of range
/// goes into the range: see [`DeviceSettings::fit`]. If the file has an
/// error or a newer schema version, the result is the problem: the caller
/// uses the defaults and does not write the file until the user changes a
/// setting.
pub fn load(
    dir: &Path,
    model: &str,
    support: SettingsSupport,
) -> std::result::Result<DeviceSettings, FileProblem> {
    let Some(file) = read_file(dir)? else {
        return Ok(DeviceSettings::defaults(support));
    };
    let mut settings = DeviceSettings::defaults(support);
    let Some(table) = file.get(model) else {
        return Ok(settings);
    };
    let table = table
        .as_table()
        .ok_or_else(|| invalid(format!("'{model}' is not a table")))?;
    if let Some(value) = integer(table, model, "brightness")? {
        settings.brightness = to_percent(value);
    }
    if let Some(value) = table.get("haptic") {
        settings.haptic = value
            .as_bool()
            .ok_or_else(|| invalid(format!("'{model}.haptic' is not true or false")))?;
    }
    if let Some(lights) = table.get("knob_lights") {
        let path = format!("{model}.knob_lights");
        let lights = lights
            .as_table()
            .ok_or_else(|| invalid(format!("'{path}' is not a table")))?;
        let knob = &mut settings.knob_lights;
        if let Some(mode) = lights.get("mode") {
            knob.mode = mode
                .as_str()
                .and_then(KnobLightMode::from_name)
                .ok_or_else(|| {
                    invalid(format!(
                        "'{path}.mode' is not \"effect\", \"colors\", or \"off\""
                    ))
                })?;
        }
        if let Some(value) = integer(lights, &path, "brightness")? {
            knob.brightness = to_percent(value);
        }
        if let Some(colors) = lights.get("colors") {
            let colors = colors
                .as_array()
                .ok_or_else(|| invalid(format!("'{path}.colors' is not a list")))?;
            knob.colors = colors
                .iter()
                .map(|c| c.as_str().and_then(|text| text.parse().ok()))
                .collect::<Option<Vec<Color>>>()
                .ok_or_else(|| {
                    invalid(format!(
                        "'{path}.colors' has a value that is not a colour #RRGGBB"
                    ))
                })?;
        }
    }
    Ok(settings.fit(support))
}

/// Write the settings of `model` to the settings file in the profiles
/// directory `dir`. The tables of other models stay. A file that has an
/// error, or a newer schema version, is replaced.
pub fn save(dir: &Path, model: &str, settings: &DeviceSettings) -> Result<()> {
    let mut file = read_file(dir).ok().flatten().unwrap_or_default();
    file.insert(
        "schema_version".to_string(),
        toml::Value::Integer(SETTINGS_SCHEMA_VERSION),
    );
    let lights = &settings.knob_lights;
    let mut knob = toml::Table::new();
    knob.insert("mode".into(), lights.mode.name().into());
    knob.insert(
        "brightness".into(),
        toml::Value::Integer(lights.brightness.into()),
    );
    knob.insert(
        "colors".into(),
        toml::Value::Array(lights.colors.iter().map(|c| c.to_string().into()).collect()),
    );
    let mut table = toml::Table::new();
    table.insert(
        "brightness".into(),
        toml::Value::Integer(settings.brightness.into()),
    );
    table.insert("haptic".into(), settings.haptic.into());
    table.insert("knob_lights".into(), toml::Value::Table(knob));
    file.insert(model.to_string(), toml::Value::Table(table));
    let text = toml::to_string_pretty(&file)?;
    fs::create_dir_all(dir).map_err(|e| io_error("cannot make the directory", dir, e))?;
    write_and_rename(&settings_path(dir), text.as_bytes())
}

/// The content of the settings file, with a schema version that this engine
/// knows. `None` if the file is missing.
fn read_file(dir: &Path) -> std::result::Result<Option<toml::Table>, FileProblem> {
    let text = match fs::read_to_string(settings_path(dir)) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(FileProblem::Read(e)),
    };
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| invalid(e.message().to_string()))?;
    match table.get("schema_version").map(toml::Value::as_integer) {
        Some(Some(SETTINGS_SCHEMA_VERSION)) => Ok(Some(table)),
        Some(Some(found)) if found > SETTINGS_SCHEMA_VERSION => Err(FileProblem::Newer(found)),
        Some(Some(found)) => Err(invalid(format!("schema version {found} does not exist"))),
        Some(None) => Err(invalid("the schema version is not a number".to_string())),
        None => Err(invalid("it has no schema version".to_string())),
    }
}

fn invalid(reason: String) -> FileProblem {
    FileProblem::Invalid(reason)
}

/// The integer `key` of `table`, if the table has it.
fn integer(
    table: &toml::Table,
    path: &str,
    key: &str,
) -> std::result::Result<Option<i64>, FileProblem> {
    table
        .get(key)
        .map(|v| {
            v.as_integer()
                .ok_or_else(|| invalid(format!("'{path}.{key}' is not a whole number")))
        })
        .transpose()
}

/// A number of the file as a percent. A value below 0 or above 100 goes to
/// the nearest end; [`DeviceSettings::fit`] puts it in the range of the
/// setting.
fn to_percent(value: i64) -> u8 {
    value.clamp(0, i64::from(MAX_BRIGHTNESS)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencraft_core::device::DESCRIPTORS;
    use tempfile::TempDir;

    const N4: &str = "n4-pro";

    fn n4() -> SettingsSupport {
        DESCRIPTORS[0].settings
    }

    const NONE: SettingsSupport = SettingsSupport {
        lowest_brightness: 0,
        knob_lights: 0,
        haptic: false,
    };

    fn custom() -> DeviceSettings {
        DeviceSettings {
            brightness: 42,
            haptic: false,
            knob_lights: KnobLights {
                mode: KnobLightMode::Colors,
                brightness: 30,
                colors: vec![
                    Color::rgb(255, 0, 0),
                    Color::rgb(0, 255, 0),
                    Color::rgb(0, 0, 255),
                    Color::WHITE,
                ],
            },
        }
    }

    fn write(dir: &Path, text: &str) {
        fs::write(settings_path(dir), text).unwrap();
    }

    #[test]
    fn the_defaults_give_the_device_of_the_earlier_phases() {
        let d = DeviceSettings::defaults(n4());
        assert_eq!(d.brightness, 80);
        assert!(d.haptic);
        assert_eq!(d.knob_lights.mode, KnobLightMode::Effect);
        assert_eq!(d.knob_lights.brightness, 100);
        assert_eq!(d.knob_lights.colors, vec![Color::WHITE; 4]);
        assert!(DeviceSettings::defaults(NONE).knob_lights.colors.is_empty());
    }

    #[test]
    fn a_value_out_of_range_goes_into_the_range() {
        let mut s = custom();
        s.brightness = 3;
        s.knob_lights.brightness = 0;
        s.knob_lights.colors.truncate(2);
        let fitted = s.clone().fit(n4());
        assert_eq!(fitted.brightness, 15);
        assert_eq!(fitted.knob_lights.brightness, 1);
        assert_eq!(fitted.knob_lights.colors.len(), 4);
        assert_eq!(fitted.knob_lights.colors[3], DEFAULT_KNOB_COLOR);
        s.brightness = 200;
        s.knob_lights.brightness = 255;
        s.knob_lights.colors = vec![Color::BLACK; 9];
        let fitted = s.fit(n4());
        assert_eq!(
            (fitted.brightness, fitted.knob_lights.brightness),
            (100, 100)
        );
        assert_eq!(fitted.knob_lights.colors.len(), 4);
    }

    fn cmd(rest: &[u8]) -> Vec<u8> {
        let mut v = b"\x00CRT\x00\x00".to_vec();
        v.extend_from_slice(rest);
        v
    }

    #[test]
    fn each_mode_gives_the_commands_of_the_decision() {
        let mut s = custom();
        let c = SettingCommands::new(&s, n4());
        assert_eq!(c.brightness, cmd(b"LIG\x00\x00\x2A"));
        assert_eq!(
            c.knob_lights,
            [
                cmd(b"SETLB\xFF\x00\x00\x00\xFF\x00\x00\x00\xFF\xFF\xFF\xFF"),
                cmd(b"LBLIG\x1E"),
            ]
        );
        assert_eq!(c.haptic, [cmd(b"QUCMD\x1F\x11\x00\xFF\x00\x11")]);

        s.knob_lights.mode = KnobLightMode::Effect;
        s.haptic = true;
        let c = SettingCommands::new(&s, n4());
        assert_eq!(
            c.knob_lights,
            [
                cmd(&[b"SETLB".as_slice(), &[0; 12]].concat()),
                cmd(b"LBLIG\x1E")
            ]
        );
        assert_eq!(c.haptic, [cmd(b"QUCMD\x1F\x11\x00\x11\x00\x11")]);

        s.knob_lights.mode = KnobLightMode::Off;
        assert_eq!(
            SettingCommands::new(&s, n4()).knob_lights,
            [cmd(b"LBLIG\x00")]
        );

        // The mode "colors" with all colours black is the mode "off".
        s.knob_lights.mode = KnobLightMode::Colors;
        s.knob_lights.colors = vec![Color::BLACK; 4];
        assert_eq!(
            SettingCommands::new(&s, n4()).knob_lights,
            [cmd(b"LBLIG\x00")]
        );
        // One black knob among colours is a black colour.
        s.knob_lights.colors[2] = Color::rgb(0, 0, 9);
        assert_eq!(SettingCommands::new(&s, n4()).knob_lights.len(), 2);
    }

    #[test]
    fn a_model_with_no_knob_lights_or_haptic_feedback_gets_no_such_command() {
        let c = SettingCommands::new(&DeviceSettings::defaults(NONE), NONE);
        assert!(c.knob_lights.is_empty());
        assert!(c.haptic.is_empty());
        assert_eq!(c.after_start().count(), 0);
        assert_eq!(c.brightness, cmd(b"LIG\x00\x00\x50"));
    }

    #[test]
    fn a_change_gives_only_the_commands_of_that_setting() {
        let old = custom();
        let before = SettingCommands::new(&old, n4());
        let changed = |f: fn(&mut DeviceSettings)| {
            let mut new = old.clone();
            f(&mut new);
            let after = SettingCommands::new(&new, n4());
            before
                .changes(&after)
                .into_iter()
                .map(|c| String::from_utf8_lossy(&c[6..11]).to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(changed(|s| s.brightness = 50), ["LIG\0\0"]);
        assert_eq!(changed(|s| s.haptic = true), ["QUCMD"]);
        assert_eq!(
            changed(|s| s.knob_lights.brightness = 9),
            ["SETLB", "LBLIG"]
        );
        assert_eq!(
            changed(|s| s.knob_lights.colors[0] = Color::BLACK),
            ["SETLB", "LBLIG"]
        );
        assert_eq!(
            changed(|s| s.knob_lights.mode = KnobLightMode::Off),
            ["LBLIG"]
        );
        assert!(changed(|_| {}).is_empty());
        // A colour change in the mode "effect" changes no command.
        let mut effect = old.clone();
        effect.knob_lights.mode = KnobLightMode::Effect;
        let a = SettingCommands::new(&effect, n4());
        effect.knob_lights.colors[1] = Color::rgb(1, 2, 3);
        assert!(a.changes(&SettingCommands::new(&effect, n4())).is_empty());
    }

    #[test]
    fn with_no_file_the_settings_are_the_defaults_and_nothing_is_written() {
        let dir = TempDir::new().unwrap();
        let s = load(dir.path(), N4, n4()).unwrap();
        assert_eq!(s, DeviceSettings::defaults(n4()));
        assert!(!settings_path(dir.path()).exists());
    }

    #[test]
    fn the_settings_go_to_the_file_and_back_and_the_file_is_readable() {
        let dir = TempDir::new().unwrap();
        save(dir.path(), N4, &custom()).unwrap();
        assert_eq!(load(dir.path(), N4, n4()).unwrap(), custom());
        let text = fs::read_to_string(settings_path(dir.path())).unwrap();
        let want = r##"schema_version = 1

[n4-pro]
brightness = 42
haptic = false

[n4-pro.knob_lights]
brightness = 30
colors = [
    "#FF0000",
    "#00FF00",
    "#0000FF",
    "#FFFFFF",
]
mode = "colors"
"##;
        assert_eq!(text, want);
    }

    #[test]
    fn a_write_keeps_the_tables_of_other_models() {
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "schema_version = 1\n[other-model]\nbrightness = 33\nfuture = \"kept\"\n",
        );
        save(dir.path(), N4, &custom()).unwrap();
        let other = load(dir.path(), "other-model", n4()).unwrap();
        assert_eq!(other.brightness, 33);
        let text = fs::read_to_string(settings_path(dir.path())).unwrap();
        assert!(text.contains("future = \"kept\""), "{text}");
        assert_eq!(load(dir.path(), N4, n4()).unwrap(), custom());
    }

    #[test]
    fn a_missing_value_gives_the_default_and_a_value_out_of_range_goes_into_the_range() {
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "schema_version = 1\n[n4-pro]\nbrightness = 5\n[n4-pro.knob_lights]\n\
             brightness = 900\ncolors = [\"#ff0000\"]\n",
        );
        let s = load(dir.path(), N4, n4()).unwrap();
        assert_eq!(s.brightness, 15);
        assert!(s.haptic);
        assert_eq!(s.knob_lights.mode, KnobLightMode::Effect);
        assert_eq!(s.knob_lights.brightness, 100);
        assert_eq!(
            s.knob_lights.colors,
            [
                Color::rgb(255, 0, 0),
                Color::WHITE,
                Color::WHITE,
                Color::WHITE
            ]
        );
        write(
            dir.path(),
            "schema_version = 1\n[n4-pro]\nbrightness = -20\n[n4-pro.knob_lights]\nbrightness = 0\n",
        );
        let s = load(dir.path(), N4, n4()).unwrap();
        assert_eq!((s.brightness, s.knob_lights.brightness), (15, 1));
        // A table of a different model only.
        write(dir.path(), "schema_version = 1\n[other]\nbrightness = 20\n");
        assert_eq!(
            load(dir.path(), N4, n4()).unwrap(),
            DeviceSettings::defaults(n4())
        );
    }

    #[test]
    fn a_file_with_an_error_or_a_newer_schema_is_a_problem() {
        let dir = TempDir::new().unwrap();
        for text in [
            "this is not TOML [",
            "[n4-pro]\nbrightness = 50\n",
            "schema_version = 0\n",
            "schema_version = \"1\"\n",
            "schema_version = 1\nn4-pro = 3\n",
            "schema_version = 1\n[n4-pro]\nbrightness = \"high\"\n",
            "schema_version = 1\n[n4-pro]\nhaptic = 1\n",
            "schema_version = 1\n[n4-pro]\nknob_lights = 1\n",
            "schema_version = 1\n[n4-pro.knob_lights]\nmode = \"rainbow\"\n",
            "schema_version = 1\n[n4-pro.knob_lights]\ncolors = \"#FFFFFF\"\n",
            "schema_version = 1\n[n4-pro.knob_lights]\ncolors = [\"red\"]\n",
            "schema_version = 1\n[n4-pro.knob_lights]\nbrightness = 1.5\n",
        ] {
            write(dir.path(), text);
            assert!(
                matches!(load(dir.path(), N4, n4()), Err(FileProblem::Invalid(_))),
                "{text}"
            );
        }
        write(
            dir.path(),
            "schema_version = 2\n[n4-pro]\nbrightness = 50\n",
        );
        let problem = load(dir.path(), N4, n4()).unwrap_err();
        assert!(matches!(problem, FileProblem::Newer(2)));
        assert!(problem.to_string().contains("newer"), "{problem}");
    }
}
