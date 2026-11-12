//! Tests for the kinds that ask the engine for work. No test needs a device
//! or a real wait.

use super::*;
use opencraft_engine::{Action, BindingId, EngineRequest, FeedbackState, PageTarget};

fn engine(request: EngineRequest) -> Message {
    Message::Engine(request)
}

fn nested(number: i64) -> Action {
    Action {
        kind: "test.nested".into(),
        settings: Settings::from([("number".into(), Value::Integer(number))]),
    }
}

fn invoke_nested(binding: BindingId, number: i64) -> Message {
    engine(EngineRequest::Invoke {
        binding,
        action: nested(number),
    })
}

fn state(binding: BindingId, on: bool, label: Option<&str>) -> Message {
    Message::Feedback {
        binding,
        feedback: Some(Feedback {
            label: label.map(str::to_string),
            level: None,
            state: Some(if on {
                FeedbackState::On
            } else {
                FeedbackState::Off
            }),
        }),
    }
}

impl Rig {
    fn none(&self) {
        assert!(self.messages.try_recv().is_err());
    }
}

// --- Change the page ----------------------------------------------------------------

#[test]
fn a_page_change_asks_the_engine_with_the_page_number_from_zero() {
    let mut rig = Rig::new("page", true);
    assert!(rig.initial.is_empty());
    for (settings, target) in [
        (vec![], PageTarget::Next),
        (
            vec![("target", Value::Text("next".into()))],
            PageTarget::Next,
        ),
        (
            vec![("target", Value::Text("previous".into()))],
            PageTarget::Previous,
        ),
        (
            vec![
                ("target", Value::Text("number".into())),
                ("page", Value::Integer(3)),
            ],
            PageTarget::Number(2),
        ),
        (
            vec![("target", Value::Text("number".into()))],
            PageTarget::Number(0),
        ),
        (
            vec![
                ("target", Value::Text("number".into())),
                ("page", Value::Integer(100)),
            ],
            PageTarget::Number(99),
        ),
    ] {
        rig.invoke(action("page", &settings));
        assert_eq!(rig.recv(), engine(EngineRequest::SelectPage(target)));
    }
    rig.none();
    assert!(rig.system.calls().is_empty());
}

#[test]
fn incorrect_page_settings_are_problems_without_a_request() {
    let mut rig = Rig::new("page", true);
    let number = || ("target", Value::Text("number".into()));
    for (settings, contains) in [
        (
            vec![("target", Value::Text("sideways".into()))],
            "Change the page: select a value in the field \"Page\".",
        ),
        (vec![("target", Value::Integer(1))], "the field \"Page\""),
        (
            vec![number(), ("page", Value::Integer(0))],
            "Change the page: set the field \"Page number\" to a number from 1 to 100.",
        ),
        (vec![number(), ("page", Value::Integer(101))], "1 to 100"),
        (
            vec![number(), ("page", Value::Text("2".into()))],
            "1 to 100",
        ),
    ] {
        rig.invoke(action("page", &settings));
        rig.problem(contains);
    }
    rig.none();
}

// --- Select a profile -----------------------------------------------------------------

#[test]
fn a_profile_selection_asks_the_engine_with_the_name_of_the_user() {
    let mut rig = Rig::new("profile", true);
    rig.invoke(text_action("profile", "profile", " Video calls "));
    assert_eq!(
        rig.recv(),
        engine(EngineRequest::SelectProfile {
            name: "Video calls".into()
        })
    );
    rig.invoke(action("profile", &[]));
    rig.problem("Select a profile: type a value in the field \"Profile name\".");
    for bad in [Value::Text("  ".into()), Value::Integer(1)] {
        rig.invoke(action("profile", &[("profile", bad)]));
        rig.problem("the field \"Profile name\"");
    }
    rig.none();
}

// --- Toggle -----------------------------------------------------------------------------

fn toggle(on: Option<Action>, off: Option<Action>, on_label: Option<&str>) -> Action {
    let mut fields = Vec::new();
    if let Some(on) = on {
        fields.push(("on", on.to_value()));
    }
    if let Some(off) = off {
        fields.push(("off", off.to_value()));
    }
    if let Some(label) = on_label {
        fields.push(("on_label", Value::Text(label.into())));
    }
    action("toggle", &fields)
}

fn mic() -> Action {
    toggle(Some(nested(1)), Some(nested(0)), Some("Mic on"))
}

impl Rig {
    fn appear_with(&mut self, index: u8, action: Action) {
        self.handler.handle(Request::Appear {
            binding: binding(index),
            action,
        });
    }
    fn invoke_at(&mut self, index: u8, action: Action) {
        self.handler.handle(Request::Invoke {
            binding: binding(index),
            action,
        });
    }
}

#[test]
fn a_toggle_starts_off_and_each_invoke_changes_the_state_and_does_its_action() {
    let mut rig = Rig::new("toggle", true);
    let id = binding(1);
    rig.appear_with(1, mic());
    assert_eq!(rig.recv(), state(id, false, None));

    rig.invoke_at(1, mic());
    assert_eq!(rig.recv(), invoke_nested(id, 1));
    assert_eq!(rig.recv(), state(id, true, Some("Mic on")));

    rig.invoke_at(1, mic());
    assert_eq!(rig.recv(), invoke_nested(id, 0));
    assert_eq!(rig.recv(), state(id, false, None));
    rig.none();
    assert!(rig.system.calls().is_empty());
}

#[test]
fn a_toggle_keeps_its_state_after_disappear_and_forgets_it_when_the_settings_change() {
    let mut rig = Rig::new("toggle", true);
    let id = binding(1);
    rig.invoke_at(1, mic());
    rig.recv();
    rig.recv();

    // A page change and a change back: Disappear, then Appear with the same
    // settings. The feedback shows the state from before.
    rig.disappear(1);
    rig.none();
    rig.appear_with(1, mic());
    assert_eq!(rig.recv(), state(id, true, Some("Mic on")));

    // Different settings: the state is off.
    let changed = toggle(Some(nested(1)), Some(nested(0)), Some("On"));
    rig.disappear(1);
    rig.appear_with(1, changed.clone());
    assert_eq!(rig.recv(), state(id, false, None));
    rig.invoke_at(1, changed);
    rig.recv();
    assert_eq!(rig.recv(), state(id, true, Some("On")));

    // The first settings again: the old state is gone.
    rig.disappear(1);
    rig.appear_with(1, mic());
    assert_eq!(rig.recv(), state(id, false, None));
    rig.none();
}

#[test]
fn each_binding_has_its_own_state() {
    let mut rig = Rig::new("toggle", true);
    rig.invoke_at(1, mic());
    rig.recv();
    assert_eq!(rig.recv(), state(binding(1), true, Some("Mic on")));
    rig.invoke_at(2, mic());
    assert_eq!(rig.recv(), invoke_nested(binding(2), 1));
    assert_eq!(rig.recv(), state(binding(2), true, Some("Mic on")));
}

#[test]
fn a_toggle_with_no_action_and_no_label_only_changes_its_state() {
    let mut rig = Rig::new("toggle", true);
    let empty = toggle(None, None, None);
    for on in [true, false, true] {
        rig.invoke_at(1, empty.clone());
        assert_eq!(rig.recv(), state(binding(1), on, None));
    }
    // A label of spaces is no label. An action for one state only.
    let half = toggle(Some(nested(1)), None, Some("  "));
    rig.invoke_at(2, half.clone());
    assert_eq!(rig.recv(), invoke_nested(binding(2), 1));
    assert_eq!(rig.recv(), state(binding(2), true, None));
    rig.invoke_at(2, half);
    assert_eq!(rig.recv(), state(binding(2), false, None));
    rig.none();
}

#[test]
fn a_toggle_in_a_toggle_keeps_its_own_state() {
    let mut rig = Rig::new("toggle", true);
    let id = binding(1);
    let inner = toggle(Some(nested(1)), None, None);
    let outer = toggle(Some(inner.clone()), Some(inner.clone()), None);
    rig.appear_with(1, outer.clone());
    rig.recv();
    // The engine gives the inner toggle the binding of the outer toggle.
    rig.invoke_at(1, outer.clone());
    assert_eq!(
        rig.recv(),
        engine(EngineRequest::Invoke {
            binding: id,
            action: inner.clone()
        })
    );
    assert_eq!(rig.recv(), state(id, true, None));
    rig.invoke_at(1, inner.clone());
    assert_eq!(rig.recv(), invoke_nested(id, 1));
    assert_eq!(rig.recv(), state(id, true, None));

    // After a page change and back, the two states stay.
    rig.disappear(1);
    rig.appear_with(1, outer.clone());
    assert_eq!(rig.recv(), state(id, true, None));
    rig.invoke_at(1, inner);
    assert_eq!(rig.recv(), state(id, false, None));
    rig.none();
}

#[test]
fn incorrect_toggle_settings_are_problems_and_change_nothing() {
    let mut rig = Rig::new("toggle", true);
    let kind_only = |kind: Value| Value::Table(Settings::from([("kind".into(), kind)]));
    for (key, bad, text) in [
        (
            "on",
            Value::Text("opencraft.hotkey".into()),
            "Toggle: select the action in the field \"Action for on\" again.",
        ),
        (
            "on",
            kind_only(Value::Text("".into())),
            "the field \"Action for on\"",
        ),
        (
            "off",
            kind_only(Value::Integer(3)),
            "the field \"Action for off\"",
        ),
        ("off", Value::List(vec![]), "the field \"Action for off\""),
        (
            "on_label",
            Value::Integer(1),
            "Toggle: type a value in the field \"Label for on\".",
        ),
    ] {
        rig.invoke_at(1, action("toggle", &[(key, bad)]));
        rig.problem(text);
    }
    rig.none();
    // The state did not change: the next correct Invoke turns it on.
    rig.invoke_at(1, toggle(None, None, None));
    assert_eq!(rig.recv(), state(binding(1), true, None));

    // At Appear, a bad label is no label, and no problem.
    rig.appear_with(1, action("toggle", &[("on_label", Value::Integer(1))]));
    assert_eq!(rig.recv(), state(binding(1), false, None));
    rig.none();
}

// --- Sequence ---------------------------------------------------------------------------

fn sequence(actions: &[Action], delay_ms: Option<i64>) -> Action {
    let mut fields = vec![("actions", Action::list_to_value(actions))];
    if let Some(delay) = delay_ms {
        fields.push(("delay_ms", Value::Integer(delay)));
    }
    action("sequence", &fields)
}

#[test]
fn a_sequence_asks_for_each_action_in_order_with_the_delay_between_two_actions() {
    let mut rig = Rig::new("sequence", true);
    let id = binding(1);
    rig.system.hold_sleeps();
    rig.invoke(sequence(&[nested(1), nested(2), nested(3)], Some(250)));

    // While the thread waits, only the actions before the wait are sent.
    assert_eq!(rig.recv(), invoke_nested(id, 1));
    wait_until(|| rig.system.sleeps_started.load(Ordering::SeqCst) == 1);
    rig.none();
    rig.system.release_sleep();
    assert_eq!(rig.recv(), invoke_nested(id, 2));
    wait_until(|| rig.system.sleeps_started.load(Ordering::SeqCst) == 2);
    rig.none();
    rig.system.release_sleep();
    assert_eq!(rig.recv(), invoke_nested(id, 3));
    // No wait after the last action.
    assert_eq!(rig.system.calls(), [Call::Sleep(250), Call::Sleep(250)]);
}

#[test]
fn a_sequence_waits_for_the_default_delay_and_not_for_no_delay() {
    let mut rig = Rig::new("sequence", true);
    rig.invoke(sequence(&[nested(1), nested(2)], None));
    assert_eq!(rig.recv(), invoke_nested(binding(1), 1));
    assert_eq!(rig.recv(), invoke_nested(binding(1), 2));
    assert_eq!(rig.system.calls(), [Call::Sleep(100)]);

    rig.system.clear_calls();
    rig.invoke(sequence(&[nested(3), nested(4)], Some(0)));
    assert_eq!(rig.recv(), invoke_nested(binding(1), 3));
    assert_eq!(rig.recv(), invoke_nested(binding(1), 4));
    assert!(rig.system.calls().is_empty());
}

#[test]
fn a_sequence_does_not_delay_the_next_request() {
    let mut rig = Rig::new("sequence", true);
    rig.system.hold_sleeps();
    rig.invoke(sequence(&[nested(1), nested(2)], Some(10_000)));
    assert_eq!(rig.recv(), invoke_nested(binding(1), 1));
    wait_until(|| rig.system.sleeps_started.load(Ordering::SeqCst) == 1);
    // The first sequence waits. A second one starts at once.
    rig.invoke_at(2, sequence(&[nested(5)], None));
    assert_eq!(rig.recv(), invoke_nested(binding(2), 5));
    rig.system.release_sleep();
    assert_eq!(rig.recv(), invoke_nested(binding(1), 2));
}

#[test]
fn incorrect_sequence_settings_are_problems_without_a_request() {
    let mut rig = Rig::new("sequence", true);
    let good = Action::list_to_value(&[nested(1)]);
    let delay = "Sequence of actions: set the field \"Delay between two actions, in \
                 milliseconds\" to a number from 0 to 10000.";
    for (fields, contains) in [
        (
            vec![("actions", Value::Text("x".into()))],
            "Sequence of actions: select the action in the field \"Actions\" again.",
        ),
        (
            vec![(
                "actions",
                Value::List(vec![nested(1).to_value(), Value::Integer(2)]),
            )],
            "the field \"Actions\"",
        ),
        (
            vec![("actions", good.clone()), ("delay_ms", Value::Integer(-1))],
            delay,
        ),
        (
            vec![
                ("actions", good.clone()),
                ("delay_ms", Value::Integer(10_001)),
            ],
            delay,
        ),
        (
            vec![("actions", good), ("delay_ms", Value::Text("100".into()))],
            delay,
        ),
    ] {
        rig.invoke(action("sequence", &fields));
        rig.problem(contains);
    }
    // No actions, or an empty list: nothing to do, and no problem.
    rig.invoke(action("sequence", &[]));
    rig.invoke(sequence(&[], Some(5)));
    rig.none();
    assert!(rig.system.calls().is_empty());
}
