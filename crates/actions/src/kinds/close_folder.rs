//! The action "Back". It closes the open folder.
//!
//! The action has no settings. A stored setting is ignored. The action asks
//! the engine to close the open folder (decision 0036). The window shows the
//! kind as the folder back action: see `KindDescription::folder_back`.

use crate::descriptions::GROUP_NAVIGATION;
use crate::kinds::{Kind, Work};
use opencraft_engine::{EngineRequest, KindDescription, Settings};

/// The kind name.
pub(crate) const ID: &str = "opencraft.close-folder";

const NAME: &str = "Back";

/// The registry value for this kind.
pub(crate) static KIND: CloseFolderKind = CloseFolderKind;

pub(crate) struct CloseFolderKind;

impl Kind for CloseFolderKind {
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
        group: GROUP_NAVIGATION.into(),
        icon: Some("corner-up-left".into()),
        folder_back: true,
        ..KindDescription::default()
    }
}

/// Ask the engine to close the folder.
pub(crate) fn perform(work: &Work<'_>) {
    work.ask(EngineRequest::CloseFolder);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::values;
    use crate::tests::support::Fake;
    use opencraft_engine::{Message, Value};
    use std::sync::mpsc;

    #[test]
    fn the_description_is_the_folder_back_action_with_no_field() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_NAVIGATION);
        assert_eq!(description.icon.as_deref(), Some("corner-up-left"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        assert!(description.fields.is_empty());
        assert!(description.shown_when.is_empty());
        assert!(description.direction.is_none());
        assert!(description.states.is_none());
        assert!(description.folder_back);
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn a_push_asks_the_engine_to_close_the_folder_and_ignores_a_stored_setting() {
        let system = Fake::default();
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        let work = Work::new(&system, &sender);
        KIND.invoke(&Settings::new(), &work).unwrap();
        KIND.invoke(&values(&[("folder", Value::Integer(1))]), &work)
            .unwrap();
        for _ in 0..2 {
            assert_eq!(
                messages.try_recv().unwrap(),
                Message::Engine(EngineRequest::CloseFolder)
            );
        }
        assert!(messages.try_recv().is_err());
        assert!(system.calls().is_empty());
    }
}
