//! The action "Media key".
//!
//! The stored value is the name of the key. The names of an old profile keep
//! their meaning. The work needs the Accessibility permission.

use crate::descriptions::{choice, field, GROUP_MEDIA};
use crate::kinds::{required_text, Kind, Work};
use crate::system::MediaKey;
use opencraft_engine::{KindDescription, Settings};

/// The kind name.
pub(crate) const ID: &str = "opencraft.media-key";

const NAME: &str = "Media key";

/// The registry value for this kind.
pub(crate) static KIND: MediaKeyKind = MediaKeyKind;

pub(crate) struct MediaKeyKind;

impl Kind for MediaKeyKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        let key = read(settings)?;
        if !work.keys_allowed() {
            return Err(crate::handlers::no_permission());
        }
        perform(key, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_MEDIA.into(),
        icon: Some("play".into()),
        help: Some("Accessibility".into()),
        needs_accessibility: true,
        fields: vec![field(
            "key",
            "Key",
            choice(&[
                ("play-pause", "Play or pause"),
                ("next", "Next"),
                ("previous", "Previous"),
                ("fast-forward", "Fast forward"),
                ("rewind", "Rewind"),
                ("volume-up", "Volume up"),
                ("volume-down", "Volume down"),
                ("mute", "Mute"),
            ]),
        )],
        ..KindDescription::default()
    }
}

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<MediaKey, String> {
    let key = match required_text(ID, settings, "key")? {
        "play-pause" => MediaKey::PlayPause,
        "next" => MediaKey::Next,
        "previous" => MediaKey::Previous,
        "fast-forward" => MediaKey::FastForward,
        "rewind" => MediaKey::Rewind,
        "volume-up" => MediaKey::VolumeUp,
        "volume-down" => MediaKey::VolumeDown,
        "mute" => MediaKey::Mute,
        _ => return Err(crate::descriptions::setting_problem(ID, "key")),
    };
    Ok(key)
}

/// Press and release the media key.
pub(crate) fn perform(key: MediaKey, work: &Work<'_>) -> Result<(), String> {
    work.system
        .media_key(key)
        .map_err(|error| work.input_error(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptions::setting_problem;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::{values, Work};
    use opencraft_engine::{InputType, Settings, Value};

    fn run(key: MediaKey, system: &System) -> Result<(), String> {
        let sender = None;
        perform(key, &Work::new(system, &sender))
    }

    #[test]
    fn the_description_lists_the_keys_in_library_order() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_MEDIA);
        assert_eq!(description.icon.as_deref(), Some("play"));
        assert_eq!(description.help.as_deref(), Some("Accessibility"));
        assert!(description.needs_accessibility);
        assert!(description.direction.is_none());
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(description.fields.len(), 1);
        assert_eq!(description.fields[0].key, "key");
        assert_eq!(description.fields[0].label, "Key");
        assert_eq!(description.fields[0].default, None);
        let InputType::Choice { options } = &description.fields[0].input else {
            panic!("expected a choice");
        };
        assert_eq!(
            options
                .iter()
                .map(|option| (option.value.as_str(), option.label.as_str()))
                .collect::<Vec<_>>(),
            [
                ("play-pause", "Play or pause"),
                ("next", "Next"),
                ("previous", "Previous"),
                ("fast-forward", "Fast forward"),
                ("rewind", "Rewind"),
                ("volume-up", "Volume up"),
                ("volume-down", "Volume down"),
                ("mute", "Mute"),
            ]
        );
    }

    #[test]
    fn read_keeps_the_names_of_an_old_profile_and_rejects_an_unknown_key() {
        for (name, key) in [
            ("play-pause", MediaKey::PlayPause),
            ("next", MediaKey::Next),
            ("previous", MediaKey::Previous),
            ("fast-forward", MediaKey::FastForward),
            ("rewind", MediaKey::Rewind),
            ("volume-up", MediaKey::VolumeUp),
            ("volume-down", MediaKey::VolumeDown),
            ("mute", MediaKey::Mute),
        ] {
            assert_eq!(
                read(&values(&[("key", Value::Text(name.into()))])).unwrap(),
                key
            );
        }
        assert_eq!(
            read(&Settings::new()).unwrap_err(),
            setting_problem(ID, "key")
        );
        assert_eq!(
            read(&values(&[("key", Value::Text("play".into()))])).unwrap_err(),
            setting_problem(ID, "key")
        );
        assert_eq!(
            read(&values(&[("key", Value::Integer(1))])).unwrap_err(),
            setting_problem(ID, "key")
        );
    }

    #[test]
    fn a_bad_read_and_a_missing_permission_do_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(&Settings::new(), &Work::new(&system, &sender))
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "key"));
        let allow = || true;
        let blocked = Work::new(&system, &sender).with_keys(false, &allow);
        let error = KIND
            .invoke(&values(&[("key", Value::Text("mute".into()))]), &blocked)
            .unwrap_err();
        assert_eq!(error, crate::handlers::no_permission());
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_sends_the_key_and_reports_a_system_error() {
        let system = System::default();
        run(MediaKey::Mute, &system).unwrap();
        run(MediaKey::FastForward, &system).unwrap();
        assert_eq!(
            system.calls(),
            [
                Call::Media(MediaKey::Mute),
                Call::Media(MediaKey::FastForward)
            ]
        );
        system.fail(Fault::Code("media"));
        assert_eq!(
            run(MediaKey::Mute, &system).unwrap_err(),
            "Cannot send keys or mouse events: the system refused the operation (code 42)."
        );
    }
}
