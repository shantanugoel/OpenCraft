//! The action "Stop all sound".
//!
//! The action has no settings. A push stops each sound that the action "Play
//! a sound" started. A stored setting is ignored. The sound handler runs this
//! kind and "Play a sound" on one thread and in one queue.

use crate::descriptions::GROUP_MEDIA;
use crate::kinds::{Kind, Work};
use opencraft_engine::{KindDescription, Settings};

/// The kind name.
pub(crate) const ID: &str = "opencraft.stop-sound";

const NAME: &str = "Stop all sound";

/// The registry value for this kind.
pub(crate) static KIND: StopSoundKind = StopSoundKind;

pub(crate) struct StopSoundKind;

impl Kind for StopSoundKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, _settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_MEDIA.into(),
        icon: Some("circle-stop".into()),
        ..KindDescription::default()
    }
}

/// Stop each sound that OpenCraft plays.
pub(crate) fn perform(work: &Work<'_>) -> Result<(), String> {
    work.system
        .stop_sounds()
        .map_err(|error| format!("Cannot stop the sound: {error}."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::values;
    use crate::tests::support::{Call, Fake};
    use opencraft_engine::Value;

    #[test]
    fn the_description_has_the_stop_icon_and_no_field() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_MEDIA);
        assert_eq!(description.icon.as_deref(), Some("circle-stop"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        assert!(description.fields.is_empty());
        assert!(description.shown_when.is_empty());
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn a_push_stops_the_sounds_and_ignores_a_stored_setting() {
        let system = Fake::default();
        let sender = None;
        KIND.invoke(
            &values(&[("file", Value::Integer(1))]),
            &Work::new(&system, &sender),
        )
        .unwrap();
        assert_eq!(system.calls(), [Call::StopSounds]);
    }

    #[test]
    fn a_failed_stop_is_a_problem() {
        let system = Fake::default();
        system.fail(Some("stop-sounds"));
        let sender = None;
        assert_eq!(
            KIND.invoke(&Settings::new(), &Work::new(&system, &sender))
                .unwrap_err(),
            "Cannot stop the sound: the system refused the operation (code 42)."
        );
    }
}
