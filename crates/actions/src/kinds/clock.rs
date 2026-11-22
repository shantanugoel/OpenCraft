//! The clock and date action kind: its settings and the pure formatter.
//!
//! The handler in [`crate::clock`] reads the wall time and sends the text.
//! This module only makes the text from a [`LocalTime`] value, so a test
//! needs no computer clock. Decision 0043, section 6 has the formats.

use super::{Kind, Work};
use crate::clock::LocalTime;
use crate::descriptions::{choice, field_with_default, setting_problem, GROUP_LOGIC};
use opencraft_engine::{KindDescription, Settings, Value};

pub(crate) const ID: &str = "opencraft.clock";

/// The default format, as the field value and the choice key.
const DEFAULT_FORMAT: &str = "24h";

pub(crate) static KIND: ClockKind = ClockKind;

pub(crate) struct ClockKind;

impl Kind for ClockKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        KindDescription {
            kind: ID.into(),
            name: "Clock and date".into(),
            group: GROUP_LOGIC.into(),
            icon: Some("clock".into()),
            fields: vec![field_with_default(
                "format",
                "Format",
                choice(&[
                    ("24h", "Time, 24-hour"),
                    ("12h", "Time, 12-hour"),
                    ("date", "Date"),
                    ("weekday-date", "Weekday and date"),
                    ("time-date", "Time and date"),
                ]),
                Value::Text(DEFAULT_FORMAT.into()),
            )],
            ..KindDescription::default()
        }
    }

    /// The handler shows the text. A direct call only checks the settings.
    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        let _ = work;
        read(settings).map(|_| ())
    }
}

/// The text that a clock key shows (decision 0043, section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Format {
    /// `HH:MM`, for example `14:05`.
    #[default]
    Hours24,
    /// `h:mm AM/PM`, for example `2:05 PM`.
    Hours12,
    /// `MMM d`, for example `Nov 22`.
    Date,
    /// `EEE\nMMM d`, for example `Sat\nNov 22`.
    WeekdayDate,
    /// `HH:MM\nMMM d`, for example `14:05\nNov 22`.
    TimeDate,
}

impl Format {
    fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "24h" => Self::Hours24,
            "12h" => Self::Hours12,
            "date" => Self::Date,
            "weekday-date" => Self::WeekdayDate,
            "time-date" => Self::TimeDate,
            _ => return None,
        })
    }

    /// The text of `time` in this format. The month and weekday names are
    /// English in 0.7.0, as the interface language is English.
    pub(crate) fn text(self, time: LocalTime) -> String {
        let month = month_name(time.month);
        let date = format!("{month} {}", time.day);
        match self {
            Self::Hours24 => format!("{:02}:{:02}", time.hour, time.minute),
            Self::Hours12 => {
                let hour = match time.hour % 12 {
                    0 => 12,
                    hour => hour,
                };
                let half = if time.hour < 12 { "AM" } else { "PM" };
                format!("{hour}:{:02} {half}", time.minute)
            }
            Self::Date => date,
            Self::WeekdayDate => format!("{}\n{date}", weekday_name(time.weekday)),
            Self::TimeDate => format!("{:02}:{:02}\n{date}", time.hour, time.minute),
        }
    }
}

pub(crate) fn read(settings: &Settings) -> Result<Format, String> {
    match settings.get("format") {
        None => Ok(Format::default()),
        Some(Value::Text(key)) => {
            Format::from_key(key).ok_or_else(|| setting_problem(ID, "format"))
        }
        Some(_) => Err(setting_problem(ID, "format")),
    }
}

/// The short English name of a month. `month` is 1 for January.
fn month_name(month: u8) -> &'static str {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    NAMES
        .get(usize::from(month).wrapping_sub(1))
        .copied()
        .unwrap_or("Jan")
}

/// The short English name of a weekday. `weekday` is 0 for Sunday, as
/// `tm_wday` is.
fn weekday_name(weekday: u8) -> &'static str {
    const NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    NAMES.get(usize::from(weekday)).copied().unwrap_or("Sun")
}
