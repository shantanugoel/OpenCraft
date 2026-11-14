//! The action "Play a sound".
//!
//! A push starts a sound file and does not wait for its end. A missing
//! volume is 100. A missing or empty output is the default output of the
//! Mac. A saved output that is not connected is a problem, and no sound
//! plays. The sound handler runs this kind and "Stop all sound" on one
//! thread and in one queue. This module reads the settings, lists the
//! choices, and starts the sound.

use crate::descriptions::{field, field_with_default, setting_problem, GROUP_MEDIA};
use crate::kinds::{nonblank, Choices, Kind, Work};
use opencraft_engine::{ChoiceOption, InputType, KindDescription, PathKind, Settings, Value};
use std::path::Path;

/// The kind name.
pub(crate) const ID: &str = "opencraft.play-sound";

const NAME: &str = "Play a sound";

/// The label of the empty value of the field "Output".
pub(crate) const DEFAULT_OUTPUT: &str = "Default output";

/// The registry value for this kind.
pub(crate) static KIND: PlaySoundKind = PlaySoundKind;

pub(crate) struct PlaySoundKind;

impl Kind for PlaySoundKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(&read(settings)?, work)
    }

    fn choices(&self, field: &str, work: &Work<'_>) -> Option<Choices> {
        choices(field, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_MEDIA.into(),
        icon: Some("music".into()),
        fields: vec![
            field(
                "file",
                "File",
                InputType::Path {
                    kind: PathKind::File,
                },
            ),
            field_with_default(
                "volume",
                "Volume",
                InputType::Integer { min: 0, max: 100 },
                Value::Integer(100),
            ),
            field_with_default(
                "device",
                "Output",
                InputType::RuntimeChoice,
                Value::Text(String::new()),
            ),
        ],
        ..KindDescription::default()
    }
}

/// One sound, after a successful read of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sound {
    pub(crate) file: String,
    /// A percent of the system volume.
    pub(crate) volume: u8,
    /// The UID of the output. `None` is the default output.
    pub(crate) device: Option<String>,
}

/// Read the settings. A missing volume is 100. A missing or empty output is
/// the default output. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<Sound, String> {
    // An imported profile has `~` for the home folder (decision 0037).
    let file = crate::system::expand_home(nonblank(ID, settings, "file")?);
    let volume = match settings.get("volume") {
        None => 100,
        Some(Value::Integer(volume)) => u8::try_from(*volume)
            .ok()
            .filter(|volume| *volume <= 100)
            .ok_or_else(|| setting_problem(ID, "volume"))?,
        Some(_) => return Err(setting_problem(ID, "volume")),
    };
    let device = match settings.get("device") {
        None => None,
        Some(Value::Text(device)) if device.is_empty() => None,
        Some(Value::Text(device)) => Some(device.clone()),
        Some(_) => return Err(setting_problem(ID, "device")),
    };
    Ok(Sound {
        file,
        volume,
        device,
    })
}

/// Start the sound. A chosen output that is not connected is a problem.
pub(crate) fn perform(sound: &Sound, work: &Work<'_>) -> Result<(), String> {
    if let Some(device) = &sound.device {
        let devices = work
            .system
            .output_devices()
            .map_err(crate::kinds::sound_output::list_problem)?;
        if !devices.iter().any(|d| d.id == *device) {
            return Err("The selected sound output is not available.".into());
        }
    }
    work.system
        .play_sound(
            Path::new(&sound.file),
            sound.volume,
            sound.device.as_deref(),
        )
        .map_err(|error| format!("Cannot play the sound file: {error}."))
}

/// The outputs for the field "Output". The first option is the default
/// output. A failed list keeps that option and gives a problem.
pub(crate) fn choices(field: &str, work: &Work<'_>) -> Option<Choices> {
    if field != "device" {
        return None;
    }
    let mut options = vec![ChoiceOption {
        value: String::new(),
        label: DEFAULT_OUTPUT.into(),
    }];
    let mut problem = None;
    match work.system.output_devices() {
        Ok(devices) => options.extend(devices.into_iter().map(|device| ChoiceOption {
            value: device.id,
            label: device.name,
        })),
        Err(error) => problem = Some(crate::kinds::sound_output::list_problem(error)),
    }
    Some(Choices { options, problem })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::values;

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    fn run(settings: &Settings, system: &System) -> Result<(), String> {
        let sender = None;
        KIND.invoke(settings, &Work::new(system, &sender))
    }

    fn file() -> (&'static str, Value) {
        ("file", text("/sounds/a.wav"))
    }

    #[test]
    fn the_description_is_the_form_with_a_volume_of_100_and_the_default_output() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_MEDIA);
        assert_eq!(description.icon.as_deref(), Some("music"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        let fields: Vec<_> = description
            .fields
            .iter()
            .map(|f| (f.key.as_str(), f.label.as_str(), f.default.clone()))
            .collect();
        assert_eq!(
            fields,
            [
                ("file", "File", None),
                ("volume", "Volume", Some(Value::Integer(100))),
                ("device", "Output", Some(text(""))),
            ]
        );
        assert_eq!(
            description.fields[0].input,
            InputType::Path {
                kind: PathKind::File
            }
        );
        assert_eq!(
            description.fields[1].input,
            InputType::Integer { min: 0, max: 100 }
        );
        assert_eq!(description.fields[2].input, InputType::RuntimeChoice);
        assert!(description.shown_when.is_empty());
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn read_uses_a_volume_of_100_and_the_default_output() {
        let sound = |volume, device: Option<&str>| Sound {
            file: "/sounds/a.wav".into(),
            volume,
            device: device.map(str::to_string),
        };
        assert_eq!(read(&values(&[file()])).unwrap(), sound(100, None));
        assert_eq!(
            read(&values(&[
                file(),
                ("volume", Value::Integer(0)),
                ("device", text("")),
            ]))
            .unwrap(),
            sound(0, None)
        );
        assert_eq!(
            read(&values(&[
                file(),
                ("volume", Value::Integer(25)),
                ("device", text("b")),
            ]))
            .unwrap(),
            sound(25, Some("b"))
        );
    }

    #[test]
    fn read_rejects_a_bad_file_volume_or_output() {
        for (settings, key) in [
            (Settings::new(), "file"),
            (values(&[("file", text("  "))]), "file"),
            (values(&[("file", Value::Integer(1))]), "file"),
            (values(&[file(), ("volume", Value::Integer(101))]), "volume"),
            (values(&[file(), ("volume", Value::Integer(-1))]), "volume"),
            (values(&[file(), ("volume", text("5"))]), "volume"),
            (values(&[file(), ("device", Value::Integer(1))]), "device"),
        ] {
            assert_eq!(read(&settings).unwrap_err(), setting_problem(ID, key));
        }
    }

    #[test]
    fn a_bad_read_plays_nothing() {
        let system = System::default();
        assert_eq!(
            run(&values(&[("volume", Value::Integer(5))]), &system).unwrap_err(),
            setting_problem(ID, "file")
        );
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_sends_the_file_the_volume_and_the_output_to_the_system() {
        let system = System::default();
        run(&values(&[file()]), &system).unwrap();
        run(
            &values(&[
                file(),
                ("volume", Value::Integer(25)),
                ("device", text("b")),
            ]),
            &system,
        )
        .unwrap();
        assert_eq!(
            system.calls(),
            [
                Call::PlaySound("/sounds/a.wav".into(), 100, None),
                Call::PlaySound("/sounds/a.wav".into(), 25, Some("b".into())),
            ]
        );
    }

    #[test]
    fn a_missing_output_plays_nothing() {
        let system = System::default();
        assert_eq!(
            run(&values(&[file(), ("device", text("gone"))]), &system).unwrap_err(),
            "The selected sound output is not available."
        );
        system.fail(Fault::Code("outputs"));
        assert_eq!(
            run(&values(&[file(), ("device", text("b"))]), &system).unwrap_err(),
            "Cannot list the sound outputs: the system refused the operation (code 42)."
        );
        assert!(system.calls().is_empty());
    }

    #[test]
    fn a_failed_play_is_a_problem() {
        let system = System::default();
        system.fail(Fault::Code("play-sound"));
        assert_eq!(
            run(&values(&[file()]), &system).unwrap_err(),
            "Cannot play the sound file: the system refused the operation (code 42)."
        );
    }

    #[test]
    fn the_output_choices_start_with_the_default_output() {
        let system = System::default();
        let sender = None;
        let work = Work::new(&system, &sender);
        let choices = KIND.choices("device", &work).unwrap();
        assert_eq!(choices.problem, None);
        assert_eq!(
            choices
                .options
                .iter()
                .map(|o| (o.value.as_str(), o.label.as_str()))
                .collect::<Vec<_>>(),
            [
                ("", "Default output"),
                ("a", "Speakers"),
                ("b", "Headphones")
            ]
        );
        assert!(KIND.choices("file", &work).is_none());
        assert!(system.calls().is_empty());
        system.fail(Fault::Code("outputs"));
        let choices = KIND.choices("device", &work).unwrap();
        assert_eq!(choices.options.len(), 1);
        assert_eq!(
            choices.problem.as_deref(),
            Some("Cannot list the sound outputs: the system refused the operation (code 42).")
        );
    }
}
