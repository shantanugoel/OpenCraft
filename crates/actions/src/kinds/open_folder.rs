//! The action "Open a folder".
//!
//! The action asks the engine to open the folder with the identity in the
//! field `folder` (decision 0036). The identity is text of 16 hexadecimal
//! digits. A missing, blank, or bad identity is a problem for the user.

use crate::descriptions::{field, setting_problem, GROUP_NAVIGATION};
use crate::kinds::{nonblank, Kind, Work};
use opencraft_engine::{EngineRequest, FolderId, InputType, KindDescription, Settings};

/// The kind name.
pub(crate) const ID: &str = "opencraft.open-folder";

const NAME: &str = "Open a folder";

/// The registry value for this kind.
pub(crate) static KIND: OpenFolderKind = OpenFolderKind;

pub(crate) struct OpenFolderKind;

impl Kind for OpenFolderKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(read(settings)?, work);
        Ok(())
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_NAVIGATION.into(),
        icon: Some("folder".into()),
        fields: vec![field("folder", "Folder", InputType::Folder)],
        ..KindDescription::default()
    }
}

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<FolderId, String> {
    nonblank(ID, settings, "folder")?
        .parse::<FolderId>()
        .map_err(|_| setting_problem(ID, "folder"))
}

/// Ask the engine to open the folder.
pub(crate) fn perform(folder: FolderId, work: &Work<'_>) {
    work.ask(EngineRequest::OpenFolder(folder));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::recording::System;
    use crate::kinds::values;
    use opencraft_engine::{Message, Value};
    use std::sync::mpsc;

    const FOLDER: &str = "00000000000000ab";

    #[test]
    fn the_description_has_a_folder_field() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_NAVIGATION);
        assert_eq!(description.icon.as_deref(), Some("folder"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        assert!(description.direction.is_none());
        assert!(description.states.is_none());
        assert!(!description.folder_back);
        assert!(description.shown_when.is_empty());
        assert_eq!(description.fields.len(), 1);
        assert_eq!(description.fields[0].key, "folder");
        assert_eq!(description.fields[0].label, "Folder");
        assert_eq!(description.fields[0].input, InputType::Folder);
        assert_eq!(description.fields[0].default, None);
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn read_gets_the_identity_and_rejects_a_missing_blank_or_bad_value() {
        let folder = FOLDER.parse::<FolderId>().unwrap();
        assert_eq!(
            read(&values(&[("folder", Value::Text(FOLDER.into()))])).unwrap(),
            folder
        );
        let problem = "Open a folder: select a value in the field \"Folder\".";
        for settings in [
            Settings::new(),
            values(&[("folder", Value::Text("  ".into()))]),
            values(&[("folder", Value::Text("not a folder".into()))]),
            values(&[("folder", Value::Text("0000000000000000".into()))]),
            values(&[("folder", Value::Integer(1))]),
        ] {
            assert_eq!(read(&settings).unwrap_err(), problem);
        }
    }

    #[test]
    fn a_push_asks_the_engine_to_open_the_folder() {
        let system = System::default();
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        let work = Work::new(&system, &sender);
        KIND.invoke(&values(&[("folder", Value::Text(FOLDER.into()))]), &work)
            .unwrap();
        assert_eq!(
            messages.try_recv().unwrap(),
            Message::Engine(EngineRequest::OpenFolder(FOLDER.parse().unwrap()))
        );
        KIND.invoke(&Settings::new(), &work).unwrap_err();
        assert!(messages.try_recv().is_err());
        assert!(system.calls().is_empty());
    }
}
