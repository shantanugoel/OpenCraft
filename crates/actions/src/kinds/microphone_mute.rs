//! The action "Microphone mute".
//!
//! The action has no settings. A push switches the mute of the default sound
//! input. A stored setting is ignored. The microphone handler keeps the
//! bindings, the listener, and the feedback. This module gives the
//! description, the work, and the text of the feedback.

use crate::descriptions::GROUP_MEDIA;
use crate::kinds::{Kind, Work};
use opencraft_engine::{Feedback, FeedbackState, KindDescription, Settings, TwoStates};

/// The kind name.
pub(crate) const ID: &str = "opencraft.microphone-mute";

const NAME: &str = "Microphone mute";

/// The registry value for this kind.
pub(crate) static KIND: MicrophoneMuteKind = MicrophoneMuteKind;

pub(crate) struct MicrophoneMuteKind;

impl Kind for MicrophoneMuteKind {
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
        icon: Some("mic".into()),
        states: Some(TwoStates { on_label: None }),
        ..KindDescription::default()
    }
}

/// Switch the mute of the default input. An input with no master mute is a
/// problem.
pub(crate) fn perform(work: &Work<'_>) -> Result<(), String> {
    work.system
        .microphone_muted()
        .and_then(|muted| match muted {
            Some(muted) => work.system.set_microphone_muted(!muted),
            None => Err(crate::system::Error::NoInputMute),
        })
        .map_err(|error| format!("Cannot set the microphone mute: {error}."))
}

/// The feedback for the mute that the system reports. `None` means that the
/// default input has no master mute, or that the system cannot read it.
pub(crate) fn feedback(muted: Option<bool>) -> Feedback {
    Feedback {
        label: Some(
            match muted {
                Some(true) => "Microphone muted",
                Some(false) => "Microphone on",
                None => "Not supported",
            }
            .into(),
        ),
        state: muted.map(|muted| {
            if muted {
                FeedbackState::On
            } else {
                FeedbackState::Off
            }
        }),
        ..Feedback::default()
    }
}

/// The text of a failed read of the mute.
pub(crate) fn read_problem(error: impl std::fmt::Display) -> String {
    format!("Cannot read the microphone mute: {error}.")
}

/// The text of a failed follow of the changes of the mute.
pub(crate) fn follow_problem(error: impl std::fmt::Display) -> String {
    format!("Cannot follow the microphone mute: {error}.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::values;
    use crate::tests::support::{Call, Fake};
    use opencraft_engine::Value;

    fn run(system: &Fake) -> Result<(), String> {
        let sender = None;
        KIND.invoke(&Settings::new(), &Work::new(system, &sender))
    }

    #[test]
    fn the_description_has_the_mic_icon_and_no_field() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_MEDIA);
        assert_eq!(description.icon.as_deref(), Some("mic"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        assert!(description.fields.is_empty());
        assert!(description.shown_when.is_empty());
        assert!(description.direction.is_none());
        assert_eq!(description.states, Some(TwoStates { on_label: None }));
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn a_stored_setting_does_not_change_the_work() {
        let system = Fake::default();
        let sender = None;
        KIND.invoke(
            &values(&[("anything", Value::Integer(1))]),
            &Work::new(&system, &sender),
        )
        .unwrap();
        assert_eq!(system.calls(), [Call::MicrophoneMuted(true)]);
    }

    #[test]
    fn each_push_switches_the_mute() {
        let system = Fake::default();
        run(&system).unwrap();
        run(&system).unwrap();
        assert_eq!(
            system.calls(),
            [Call::MicrophoneMuted(true), Call::MicrophoneMuted(false)]
        );
    }

    #[test]
    fn an_input_with_no_master_mute_is_a_problem_and_changes_nothing() {
        let system = Fake::default();
        system.set_microphone(None);
        assert_eq!(
            run(&system).unwrap_err(),
            "Cannot set the microphone mute: the default sound input has no writable master mute."
        );
        assert!(system.calls().is_empty());
    }

    #[test]
    fn a_failed_read_or_write_is_a_problem() {
        let system = Fake::default();
        system.fail(Some("microphone"));
        assert_eq!(
            run(&system).unwrap_err(),
            "Cannot set the microphone mute: the system refused the operation (code 42)."
        );
        assert!(system.calls().is_empty());
        system.fail(Some("set-microphone"));
        assert_eq!(
            run(&system).unwrap_err(),
            "Cannot set the microphone mute: the system refused the operation (code 42)."
        );
    }

    #[test]
    fn the_feedback_shows_the_mute_or_that_it_is_not_supported() {
        assert_eq!(
            feedback(Some(true)).label.as_deref(),
            Some("Microphone muted")
        );
        assert_eq!(feedback(Some(true)).state, Some(FeedbackState::On));
        assert_eq!(
            feedback(Some(false)).label.as_deref(),
            Some("Microphone on")
        );
        assert_eq!(feedback(Some(false)).state, Some(FeedbackState::Off));
        assert_eq!(feedback(None).label.as_deref(), Some("Not supported"));
        assert_eq!(feedback(None).state, None);
    }
}
