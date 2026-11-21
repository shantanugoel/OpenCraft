//! The action "Hold a hotkey". An Invoke taps the chord instead.

use crate::descriptions::{field, GROUP_SYSTEM};
use crate::hotkey::Hotkey;
use crate::kinds::{required_text, Kind, Work};
use opencraft_engine::{InputType, KindDescription, Settings, TwoStates};

/// The kind name.
pub(crate) const ID: &str = "opencraft.hold-hotkey";

const NAME: &str = "Hold a hotkey";

/// The registry value for this kind.
pub(crate) static KIND: HoldHotkeyKind = HoldHotkeyKind;

pub(crate) struct HoldHotkeyKind;

impl Kind for HoldHotkeyKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        let hotkey = read(settings)?;
        if !work.keys_allowed() {
            return Err(crate::handlers::no_permission());
        }
        perform(&hotkey, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_SYSTEM.into(),
        icon: Some("keyboard".into()),
        help: Some("Accessibility".into()),
        needs_accessibility: true,
        states: Some(TwoStates { on_label: None }),
        fields: vec![field("hotkey", "Hotkey", InputType::Hotkey)],
        ..KindDescription::default()
    }
    .with_receives_release()
}

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<Hotkey, String> {
    required_text(ID, settings, "hotkey")?
        .parse()
        .map_err(|_| crate::descriptions::setting_problem(ID, "hotkey"))
}

/// Press and release the hotkey.
pub(crate) fn perform(hotkey: &Hotkey, work: &Work<'_>) -> Result<(), String> {
    work.system
        .hotkey_down(hotkey)
        .map_err(|error| work.input_error(error))?;
    work.system
        .hotkey_up(hotkey)
        .map_err(|error| work.input_error(error))
}
