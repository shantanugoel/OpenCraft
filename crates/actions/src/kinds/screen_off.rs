//! The action "Screen off".
//!
//! The action has no settings. A stored setting is ignored. The action asks
//! the engine to turn off the light of the device screen. A push on the
//! device or a wake of the computer turns it on again. The action makes no
//! call on the system, and it has no "sleep" form: the device sleep is not a
//! kind of OpenCraft.

use crate::descriptions::GROUP_DEVICE;
use crate::kinds::{Kind, Work};
use opencraft_engine::{DisplayState, EngineRequest, KindDescription, Settings};

/// The kind name.
pub(crate) const ID: &str = "opencraft.screen-off";

const NAME: &str = "Screen off";

/// The registry value for this kind.
pub(crate) static KIND: ScreenOffKind = ScreenOffKind;

pub(crate) struct ScreenOffKind;

impl Kind for ScreenOffKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, _settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(work);
        Ok(())
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_DEVICE.into(),
        icon: Some("monitor-off".into()),
        help: Some("Computer wake".into()),
        ..KindDescription::default()
    }
}

/// Ask the engine to turn off the screen.
pub(crate) fn perform(work: &Work<'_>) {
    work.ask(EngineRequest::DeviceDisplay(DisplayState::ScreenOff));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::values;
    use crate::tests::support::Fake;
    use opencraft_engine::{Message, Value};
    use std::sync::mpsc;

    #[test]
    fn the_description_has_the_device_group_the_wake_help_and_no_field() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_DEVICE);
        assert_eq!(description.group, "Device");
        assert_eq!(description.icon.as_deref(), Some("monitor-off"));
        assert_eq!(description.help.as_deref(), Some("Computer wake"));
        assert!(!description.needs_accessibility);
        assert!(description.fields.is_empty());
        assert!(description.shown_when.is_empty());
        assert!(description.direction.is_none());
        assert!(description.states.is_none());
        assert!(!description.folder_back);
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn a_push_asks_the_engine_to_turn_off_the_screen_and_makes_no_system_call() {
        let system = Fake::default();
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        let work = Work::new(&system, &sender);
        KIND.invoke(&Settings::new(), &work).unwrap();
        KIND.invoke(&values(&[("level", Value::Integer(1))]), &work)
            .unwrap();
        for _ in 0..2 {
            assert_eq!(
                messages.try_recv().unwrap(),
                Message::Engine(EngineRequest::DeviceDisplay(DisplayState::ScreenOff))
            );
        }
        assert!(messages.try_recv().is_err());
        assert!(system.calls().is_empty());
    }

    #[test]
    fn the_registry_has_no_kind_for_the_device_sleep() {
        assert!(crate::kinds::find("opencraft.device-sleep").is_none());
    }
}
