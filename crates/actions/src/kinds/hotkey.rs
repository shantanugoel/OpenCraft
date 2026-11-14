//! The action "Send a hotkey".
//!
//! The text form is the form of [`crate::hotkey`]. A bad text is a setting
//! problem. The work needs the Accessibility permission.

use crate::descriptions::{field, GROUP_SYSTEM};
use crate::hotkey::Hotkey;
use crate::kinds::{required_text, Kind, Work};
use opencraft_engine::{InputType, KindDescription, Settings};

/// The kind name.
pub(crate) const ID: &str = "opencraft.hotkey";

const NAME: &str = "Send a hotkey";

/// The registry value for this kind.
pub(crate) static KIND: HotkeyKind = HotkeyKind;

pub(crate) struct HotkeyKind;

impl Kind for HotkeyKind {
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
        fields: vec![field("hotkey", "Hotkey", InputType::Hotkey)],
        ..KindDescription::default()
    }
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
        .send_hotkey(hotkey)
        .map_err(|error| work.input_error(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptions::setting_problem;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::{values, Work};
    use opencraft_engine::{Settings, Value};

    fn run(hotkey: &Hotkey, system: &System) -> Result<(), String> {
        let sender = None;
        perform(hotkey, &Work::new(system, &sender))
    }

    #[test]
    fn the_description_is_the_system_form_that_needs_the_permission() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_SYSTEM);
        assert_eq!(description.icon.as_deref(), Some("keyboard"));
        assert_eq!(description.help.as_deref(), Some("Accessibility"));
        assert!(description.needs_accessibility);
        assert!(description.direction.is_none());
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(description.fields.len(), 1);
        assert_eq!(description.fields[0].key, "hotkey");
        assert_eq!(description.fields[0].label, "Hotkey");
        assert_eq!(description.fields[0].input, InputType::Hotkey);
        assert_eq!(description.fields[0].default, None);
    }

    #[test]
    fn read_accepts_a_chord_and_rejects_a_bad_hotkey() {
        assert_eq!(
            read(&values(&[("hotkey", Value::Text("shift+cmd+4".into()))]))
                .unwrap()
                .to_string(),
            "shift+cmd+4"
        );
        assert_eq!(
            read(&values(&[(
                "hotkey",
                Value::Text(" Shift + CMD+K ".into())
            )]))
            .unwrap()
            .to_string(),
            "shift+cmd+k"
        );
        assert_eq!(
            read(&values(&[("hotkey", Value::Text("cmd".into()))]))
                .unwrap()
                .to_string(),
            "cmd"
        );
        assert_eq!(
            read(&Settings::new()).unwrap_err(),
            setting_problem(ID, "hotkey")
        );
        assert_eq!(
            read(&values(&[("hotkey", Value::Text(String::new()))])).unwrap_err(),
            setting_problem(ID, "hotkey")
        );
        assert_eq!(
            read(&values(&[("hotkey", Value::Text("cmd+unknown".into()))])).unwrap_err(),
            setting_problem(ID, "hotkey")
        );
        assert_eq!(
            read(&values(&[("hotkey", Value::Integer(1))])).unwrap_err(),
            setting_problem(ID, "hotkey")
        );
    }

    #[test]
    fn a_bad_read_and_a_missing_permission_do_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(&Settings::new(), &Work::new(&system, &sender))
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "hotkey"));
        let allow = || true;
        let blocked = Work::new(&system, &sender).with_keys(false, &allow);
        let error = KIND
            .invoke(
                &values(&[("hotkey", Value::Text("cmd+a".into()))]),
                &blocked,
            )
            .unwrap_err();
        assert_eq!(error, crate::handlers::no_permission());
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_sends_the_hotkey_and_a_lost_permission_replaces_the_system_error() {
        let system = System::default();
        let hotkey = "cmd+a".parse().unwrap();
        run(&hotkey, &system).unwrap();
        assert_eq!(system.calls(), [Call::Hotkey("cmd+a".into())]);
        system.fail(Fault::Code("hotkey"));
        assert_eq!(
            run(&hotkey, &system).unwrap_err(),
            "Cannot send keys or mouse events: the system refused the operation (code 42)."
        );
        let sender = None;
        let gone = || false;
        let work = Work::new(&system, &sender).with_keys(true, &gone);
        assert_eq!(
            KIND.invoke(&values(&[("hotkey", Value::Text("cmd+a".into()))]), &work)
                .unwrap_err(),
            crate::handlers::no_permission()
        );
    }
}
