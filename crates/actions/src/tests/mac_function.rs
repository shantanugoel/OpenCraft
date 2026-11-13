use super::*;
use crate::mac_function::MacFunction;

#[test]
fn every_mac_function_uses_the_fake_including_sleep_lock_and_screen_saver() {
    let mut rig = Rig::new("mac-function", true);
    for function in MacFunction::ALL {
        assert_eq!(MacFunction::from_name(function.name()), Some(function));
        rig.invoke(text_action("mac-function", "function", function.name()));
    }
    assert_eq!(rig.system.calls(), MacFunction::ALL.map(Call::MacFunction));
    rig.system.clear_calls();
    rig.invoke(action("mac-function", &[]));
    assert_eq!(
        rig.system.calls(),
        [Call::MacFunction(MacFunction::MissionControl)]
    );
    assert!(MacFunction::from_name("do-not-disturb").is_none());
}

#[test]
fn mac_function_checks_settings_permission_and_system_errors_without_real_calls() {
    for value in [
        Value::Text("do-not-disturb".into()),
        Value::Text(String::new()),
        Value::Integer(1),
    ] {
        let mut rig = Rig::new("mac-function", true);
        rig.invoke(action("mac-function", &[("function", value)]));
        rig.problem("Function");
        assert!(rig.system.calls().is_empty());
    }
    for function in MacFunction::ALL {
        let mut rig = Rig::new("mac-function", false);
        rig.invoke(text_action("mac-function", "function", function.name()));
        statuses(
            (0..crate::descriptions::KEY_KINDS.len())
                .map(|_| rig.recv())
                .collect(),
            false,
        );
        rig.problem("Accessibility");
        assert!(rig.system.calls().is_empty());
        rig.system.permission.store(true, Ordering::SeqCst);
        rig.system.fail(Some("mac-function"));
        rig.invoke(text_action("mac-function", "function", function.name()));
        statuses(
            (0..crate::descriptions::KEY_KINDS.len())
                .map(|_| rig.recv())
                .collect(),
            true,
        );
        rig.problem("42");
    }
}

#[test]
fn a_sleep_that_macos_refuses_gives_a_message_with_the_cause() {
    let mut rig = Rig::new("mac-function", true);
    rig.system.sleep_disabled.store(true, Ordering::SeqCst);
    rig.invoke(text_action("mac-function", "function", "sleep"));
    assert_eq!(rig.system.calls(), [Call::MacFunction(MacFunction::Sleep)]);
    rig.problem(
        "Cannot do the Mac system function: macOS does not allow sleep now. A setting turns \
         off all sleep, for example \"pmset disablesleep\" from an application that keeps \
         the Mac awake. The Apple menu cannot put the Mac to sleep either.",
    );
    // The other functions do not change.
    rig.system.clear_calls();
    rig.invoke(text_action("mac-function", "function", "lock"));
    assert_eq!(rig.system.calls(), [Call::MacFunction(MacFunction::Lock)]);
}

#[test]
fn mac_function_description_has_all_choices_and_its_library_group() {
    let description = crate::descriptions::description(crate::descriptions::MAC_FUNCTION);
    assert_eq!(description.group, "System");
    assert!(description.needs_accessibility);
    assert!(description.direction.is_none());
    assert_eq!(description.fields.len(), 1);
    assert_eq!(description.fields[0].key, "function");
    let InputType::Choice { options } = &description.fields[0].input else {
        panic!("expected choices");
    };
    assert_eq!(
        options
            .iter()
            .map(|option| option.value.as_str())
            .collect::<Vec<_>>(),
        MacFunction::ALL.map(MacFunction::name)
    );
}
