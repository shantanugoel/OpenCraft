//! Settings and sound for the countdown and stopwatch action.

use super::{Kind, Work};
use crate::backend::System;
use crate::descriptions::{choice, field, field_with_default, setting_problem, GROUP_LOGIC};
use opencraft_engine::{
    InputType, KindDescription, PathKind, Settings, ShownWhen, TwoStates, Value,
};
use std::path::{Path, PathBuf};

pub(crate) const ID: &str = "opencraft.timer";
pub(crate) static KIND: TimerKind = TimerKind;
const MAX_SECONDS: i64 = 86_400;
const DEFAULT_SECONDS: i64 = 300;
const DEFAULT_VOLUME: i64 = 100;

pub(crate) struct TimerKind;

impl Kind for TimerKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        KindDescription {
            kind: ID.into(),
            name: "Timer".into(),
            group: GROUP_LOGIC.into(),
            icon: Some("timer".into()),
            states: Some(TwoStates { on_label: None }),
            fields: vec![
                field_with_default(
                    "mode",
                    "Mode",
                    choice(&[("countdown", "Countdown"), ("stopwatch", "Stopwatch")]),
                    Value::Text("countdown".into()),
                ),
                field_with_default(
                    "seconds",
                    "Duration (s)",
                    InputType::Integer {
                        min: 1,
                        max: MAX_SECONDS,
                    },
                    Value::Integer(DEFAULT_SECONDS),
                ),
                field_with_default(
                    "bell",
                    "Bell",
                    choice(&[
                        ("system", "System bell"),
                        ("file", "Sound file"),
                        ("none", "None"),
                    ]),
                    Value::Text("system".into()),
                ),
                field(
                    "file",
                    "Sound file",
                    InputType::Path {
                        kind: PathKind::File,
                    },
                ),
                field_with_default(
                    "volume",
                    "Volume (%)",
                    InputType::Integer { min: 0, max: 100 },
                    Value::Integer(DEFAULT_VOLUME),
                ),
            ],
            shown_when: vec![
                ShownWhen::new("seconds", "mode", &["countdown"]),
                ShownWhen::new("bell", "mode", &["countdown"]),
                ShownWhen::new("file", "mode", &["countdown"]),
                ShownWhen::new("file", "bell", &["file"]),
                ShownWhen::new("volume", "mode", &["countdown"]),
                ShownWhen::new("volume", "bell", &["file"]),
            ],
            ..KindDescription::default()
        }
        .with_handles_long_press()
    }

    // The timer handler owns the lifecycle. A direct kind call checks the
    // settings and can play the configured file without starting a timer.
    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        if let Bell::File { path, volume } = read(settings)?.bell {
            play_file(work.system, &path, volume)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Countdown,
    Stopwatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Bell {
    System,
    File { path: PathBuf, volume: u8 },
    None,
}

impl Bell {
    pub(crate) fn ring(&self, system: &dyn System) -> Result<(), String> {
        match self {
            Self::System => system
                .play_bell()
                .map_err(|e| format!("Cannot play the timer bell: {e}.")),
            Self::File { path, volume } => play_file(system, path, *volume),
            Self::None => Ok(()),
        }
    }
}

fn play_file(system: &dyn System, path: &Path, volume: u8) -> Result<(), String> {
    system
        .play_sound(path, volume, None)
        .map_err(|e| format!("Cannot play the timer sound file: {e}."))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimerSettings {
    pub(crate) mode: Mode,
    pub(crate) seconds: u64,
    pub(crate) bell: Bell,
}

pub(crate) fn read(settings: &Settings) -> Result<TimerSettings, String> {
    let choice = |key, default| match settings.get(key) {
        None => Ok(default),
        Some(Value::Text(text)) => Ok(text.as_str()),
        Some(_) => Err(setting_problem(ID, key)),
    };
    let integer = |key, min, max, default| match settings.get(key) {
        None => Ok(default),
        Some(Value::Integer(n)) if (min..=max).contains(n) => Ok(*n),
        Some(_) => Err(setting_problem(ID, key)),
    };
    match choice("mode", "countdown")? {
        "stopwatch" => Ok(TimerSettings {
            mode: Mode::Stopwatch,
            seconds: 0,
            bell: Bell::None,
        }),
        "countdown" => {
            let seconds = integer("seconds", 1, MAX_SECONDS, DEFAULT_SECONDS)? as u64;
            let bell = match choice("bell", "system")? {
                "none" => Bell::None,
                "system" => Bell::System,
                "file" => {
                    let volume = integer("volume", 0, 100, DEFAULT_VOLUME)? as u8;
                    match settings.get("file") {
                        None => Bell::System,
                        Some(Value::Text(path)) if path.trim().is_empty() => Bell::System,
                        Some(Value::Text(path)) => Bell::File {
                            path: crate::system::expand_home(path.trim()).into(),
                            volume,
                        },
                        Some(_) => return Err(setting_problem(ID, "file")),
                    }
                }
                _ => return Err(setting_problem(ID, "bell")),
            };
            Ok(TimerSettings {
                mode: Mode::Countdown,
                seconds,
                bell,
            })
        }
        _ => Err(setting_problem(ID, "mode")),
    }
}
