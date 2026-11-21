//! The action "Toggle".
//!
//! A toggle has two states. At each push, it changes the state, asks the
//! engine to do the action of the new state, and sends the feedback, in this
//! order (decision 0017). The settings are the action for "on", the action
//! for "off", and an optional label for the state "on". Each part is
//! optional. A toggle with no action only changes its state.
//!
//! The state is in memory, for each binding and its settings. A toggle starts
//! in the state "off". The state stays after Disappear, so a page change and a
//! change back show the state from before. An Appear with different settings
//! forgets the old state, so a change of the settings gives the state "off".
//! The state of a toggle that the binding contains, for example in a
//! sequence, stays too. The handler keeps the states in [`States`]: see
//! [`crate::kinds::Memory`].

use crate::descriptions::{field, setting_problem, GROUP_LOGIC};
use crate::kinds::{Kind, Work};
use opencraft_engine::{
    Action, BindingId, EngineRequest, Feedback, FeedbackState, InputType, KindDescription, Message,
    Settings, TwoStates, Value,
};
use std::collections::BTreeMap;

/// The kind name.
pub(crate) const ID: &str = "opencraft.toggle";

const NAME: &str = "Toggle";

/// The registry value for this kind.
pub(crate) static KIND: ToggleKind = ToggleKind;

pub(crate) struct ToggleKind;

impl Kind for ToggleKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(&read(settings)?, settings, work)
    }

    fn appear(&self, settings: &Settings, work: &Work<'_>) {
        let (Some(binding), Some(memory)) = (work.binding(), work.memory()) else {
            return;
        };
        let mut memory = memory.borrow_mut();
        memory.toggles.forget_old(binding, settings);
        // The settings are checked at Invoke. Here a bad label is no label.
        let on_label = read(settings).ok().and_then(|toggle| toggle.on_label);
        let on = memory.toggles.state(binding, settings);
        work.send(Message::Feedback {
            binding,
            feedback: Some(feedback(on, on_label)),
        });
    }
}

/// The description of this kind (decision 0014). The toggle shows its state
/// on the surface (decision 0017).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_LOGIC.into(),
        icon: Some("toggle-left".into()),
        fields: vec![
            field("on", "From Off to On", InputType::Action),
            field("off", "From On to Off", InputType::Action),
            field("on_label", "Label for on", InputType::Text),
        ],
        states: Some(TwoStates {
            on_label: Some("on_label".into()),
        }),
        ..KindDescription::default()
    }
}

/// The settings of a toggle, after a successful read.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Toggle {
    pub(crate) on: Option<Action>,
    pub(crate) off: Option<Action>,
    pub(crate) on_label: Option<String>,
}

/// Read the settings. A value of a different form is an error. A label with
/// only spaces is no label. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<Toggle, String> {
    let action = |key: &str| match settings.get(key) {
        None => Ok(None),
        Some(value) => Action::from_value(value)
            .map(Some)
            .ok_or_else(|| setting_problem(ID, key)),
    };
    let on_label = match settings.get("on_label") {
        None => None,
        Some(Value::Text(label)) => Some(label.clone()).filter(|label| !label.trim().is_empty()),
        Some(_) => return Err(setting_problem(ID, "on_label")),
    };
    Ok(Toggle {
        on: action("on")?,
        off: action("off")?,
        on_label,
    })
}

/// Change the state of the binding, ask the engine to do the action of the
/// new state, and send the feedback. This is the order of decision 0017.
pub(crate) fn perform(toggle: &Toggle, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
    let (Some(binding), Some(memory)) = (work.binding(), work.memory()) else {
        return Err(
            "A toggle needs the binding of a control and the memory of its handler.".into(),
        );
    };
    let on = {
        let mut memory = memory.borrow_mut();
        let on = !memory.toggles.state(binding, settings);
        memory.toggles.set(binding, settings, on);
        on
    };
    let action = if on { &toggle.on } else { &toggle.off };
    if let Some(action) = action {
        work.ask(EngineRequest::Invoke {
            binding,
            action: action.clone(),
        });
    }
    work.send(Message::Feedback {
        binding,
        feedback: Some(feedback(on, toggle.on_label.clone())),
    });
    Ok(())
}

/// The feedback of a state: the state, and the label for "on" when the state
/// is on. The feedback of the other state lets the engine make the image of
/// both states for the shell.
pub(crate) fn feedback(on: bool, on_label: Option<String>) -> Feedback {
    let state = |on: bool| Feedback {
        label: on_label
            .clone()
            .filter(|label| on && !label.trim().is_empty()),
        state: Some(if on {
            FeedbackState::On
        } else {
            FeedbackState::Off
        }),
        ..Feedback::default()
    };
    Feedback {
        other_state: Some(Box::new(state(!on))),
        ..state(on)
    }
}

/// The states of the toggles of one handler: for each binding, the state for
/// each settings value.
#[derive(Default)]
pub(crate) struct States {
    by_binding: BTreeMap<BindingId, Vec<(Settings, bool)>>,
}

impl States {
    /// The state of the toggle with these settings. The default is off.
    fn state(&self, binding: BindingId, settings: &Settings) -> bool {
        self.by_binding
            .get(&binding)
            .and_then(|list| list.iter().find(|(known, _)| known == settings))
            .is_some_and(|(_, on)| *on)
    }

    fn set(&mut self, binding: BindingId, settings: &Settings, on: bool) {
        let list = self.by_binding.entry(binding).or_default();
        match list.iter_mut().find(|(known, _)| known == settings) {
            Some(entry) => entry.1 = on,
            None => list.push((settings.clone(), on)),
        }
    }

    /// Forget each state of the binding, except the state of these settings
    /// and the state of a toggle that these settings contain.
    fn forget_old(&mut self, binding: BindingId, settings: &Settings) {
        if let Some(list) = self.by_binding.get_mut(&binding) {
            list.retain(|(known, _)| known == settings || contains(settings, known));
        }
    }
}

/// True if a value in `settings`, at any depth, is the table `inner`: for
/// example the settings of an action that the settings contain.
fn contains(settings: &Settings, inner: &Settings) -> bool {
    fn value_contains(value: &Value, inner: &Settings) -> bool {
        match value {
            Value::Table(table) => table == inner || contains(table, inner),
            Value::List(list) => list.iter().any(|value| value_contains(value, inner)),
            _ => false,
        }
    }
    settings.values().any(|value| value_contains(value, inner))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::{values, Memory};
    use crate::tests::support::Fake;
    use opencraft_engine::{ControlId, ControlKind, GestureKind};
    use std::cell::RefCell;
    use std::sync::mpsc::{self, Receiver};

    fn binding(index: u8) -> BindingId {
        BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Key, index),
            gesture: GestureKind::Press,
            folder: None,
        }
    }

    fn nested(number: i64) -> Action {
        Action {
            kind: "test.nested".into(),
            settings: Settings::from([("number".into(), Value::Integer(number))]),
        }
    }

    fn toggle_settings(
        on: Option<Action>,
        off: Option<Action>,
        on_label: Option<&str>,
    ) -> Settings {
        let mut pairs = Vec::new();
        if let Some(on) = on {
            pairs.push(("on", on.to_value()));
        }
        if let Some(off) = off {
            pairs.push(("off", off.to_value()));
        }
        if let Some(label) = on_label {
            pairs.push(("on_label", Value::Text(label.into())));
        }
        values(&pairs)
    }

    fn mic() -> Settings {
        toggle_settings(Some(nested(1)), Some(nested(0)), Some("Mic on"))
    }

    /// The system, the memory of a handler, and the messages of its sender.
    struct Rig {
        system: Fake,
        memory: RefCell<Memory>,
        sender: Option<mpsc::Sender<Message>>,
        messages: Receiver<Message>,
    }

    impl Rig {
        fn new() -> Self {
            let (sender, messages) = mpsc::channel();
            Self {
                system: Fake::default(),
                memory: RefCell::default(),
                sender: Some(sender),
                messages,
            }
        }

        fn work(&self, index: u8) -> Work<'_> {
            Work::new(&self.system, &self.sender)
                .with_binding(binding(index))
                .with_memory(&self.memory)
        }

        fn invoke(&self, index: u8, settings: &Settings) -> Result<(), String> {
            KIND.invoke(settings, &self.work(index))
        }

        fn appear(&self, index: u8, settings: &Settings) {
            KIND.appear(settings, &self.work(index));
        }

        fn next(&self) -> Message {
            self.messages.try_recv().expect("a message")
        }

        fn none(&self) {
            assert!(self.messages.try_recv().is_err());
        }
    }

    fn run(nested: i64, index: u8) -> Message {
        Message::Engine(EngineRequest::Invoke {
            binding: binding(index),
            action: self::nested(nested),
        })
    }

    fn state(index: u8, on: bool, label: Option<&str>) -> Message {
        Message::Feedback {
            binding: binding(index),
            feedback: Some(feedback(on, label.map(str::to_string))),
        }
    }

    #[test]
    fn the_description_has_two_actions_a_label_and_two_states() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_LOGIC);
        assert_eq!(description.group, "Logic");
        assert_eq!(description.icon.as_deref(), Some("toggle-left"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        assert!(description.direction.is_none());
        assert!(!description.folder_back);
        assert!(description.shown_when.is_empty());
        assert_eq!(
            description.states,
            Some(TwoStates {
                on_label: Some("on_label".into())
            })
        );
        let keys: Vec<_> = description.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, ["on", "off", "on_label"]);
        assert_eq!(description.fields[0].input, InputType::Action);
        assert_eq!(description.fields[1].input, InputType::Action);
        assert_eq!(description.fields[2].input, InputType::Text);
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn read_gets_both_actions_and_a_label_and_a_blank_label_is_no_label() {
        assert_eq!(
            read(&Settings::new()).unwrap(),
            Toggle {
                on: None,
                off: None,
                on_label: None,
            }
        );
        assert_eq!(
            read(&mic()).unwrap(),
            Toggle {
                on: Some(nested(1)),
                off: Some(nested(0)),
                on_label: Some("Mic on".into()),
            }
        );
        assert_eq!(
            read(&toggle_settings(None, None, Some("  ")))
                .unwrap()
                .on_label,
            None
        );
    }

    #[test]
    fn read_rejects_a_value_of_a_different_form() {
        let kind_only = |kind: Value| Value::Table(Settings::from([("kind".into(), kind)]));
        for (key, bad, text) in [
            (
                "on",
                Value::Text("opencraft.hotkey".into()),
                "Toggle: select the action in the field \"From Off to On\" again.",
            ),
            (
                "on",
                kind_only(Value::Text("".into())),
                "Toggle: select the action in the field \"From Off to On\" again.",
            ),
            (
                "off",
                kind_only(Value::Integer(3)),
                "Toggle: select the action in the field \"From On to Off\" again.",
            ),
            (
                "off",
                Value::List(vec![]),
                "Toggle: select the action in the field \"From On to Off\" again.",
            ),
            (
                "on_label",
                Value::Integer(1),
                "Toggle: type a value in the field \"Label for on\".",
            ),
        ] {
            assert_eq!(read(&values(&[(key, bad)])).unwrap_err(), text);
        }
    }

    #[test]
    fn a_push_changes_the_state_then_asks_for_the_action_then_sends_the_feedback() {
        let rig = Rig::new();
        rig.appear(1, &mic());
        assert_eq!(rig.next(), state(1, false, Some("Mic on")));

        rig.invoke(1, &mic()).unwrap();
        assert_eq!(rig.next(), run(1, 1));
        assert_eq!(rig.next(), state(1, true, Some("Mic on")));

        rig.invoke(1, &mic()).unwrap();
        assert_eq!(rig.next(), run(0, 1));
        assert_eq!(rig.next(), state(1, false, Some("Mic on")));
        rig.none();
        assert!(rig.system.calls().is_empty());
    }

    #[test]
    fn the_state_is_changed_before_the_request_goes_out() {
        let rig = Rig::new();
        rig.invoke(1, &mic()).unwrap();
        // The memory has the new state when the first message is out.
        assert!(rig.memory.borrow().toggles.state(binding(1), &mic()));
        assert_eq!(rig.next(), run(1, 1));
        assert_eq!(rig.next(), state(1, true, Some("Mic on")));
    }

    #[test]
    fn the_state_stays_after_a_page_change_and_goes_when_the_settings_change() {
        let rig = Rig::new();
        rig.invoke(1, &mic()).unwrap();
        rig.next();
        rig.next();

        // Disappear sends nothing to a kind. Appear with the same settings
        // shows the state from before.
        rig.appear(1, &mic());
        assert_eq!(rig.next(), state(1, true, Some("Mic on")));

        // Different settings: the state is off.
        let changed = toggle_settings(Some(nested(1)), Some(nested(0)), Some("On"));
        rig.appear(1, &changed);
        assert_eq!(rig.next(), state(1, false, Some("On")));
        rig.invoke(1, &changed).unwrap();
        rig.next();
        assert_eq!(rig.next(), state(1, true, Some("On")));

        // The first settings again: the old state is gone.
        rig.appear(1, &mic());
        assert_eq!(rig.next(), state(1, false, Some("Mic on")));
        rig.none();
    }

    #[test]
    fn each_binding_has_its_own_state() {
        let rig = Rig::new();
        rig.invoke(1, &mic()).unwrap();
        rig.next();
        assert_eq!(rig.next(), state(1, true, Some("Mic on")));
        rig.invoke(2, &mic()).unwrap();
        assert_eq!(rig.next(), run(1, 2));
        assert_eq!(rig.next(), state(2, true, Some("Mic on")));
    }

    #[test]
    fn a_toggle_with_no_action_only_changes_its_state() {
        let rig = Rig::new();
        let empty = toggle_settings(None, None, None);
        for on in [true, false, true] {
            rig.invoke(1, &empty).unwrap();
            assert_eq!(rig.next(), state(1, on, None));
        }
        // A label of spaces is no label. An action for one state only.
        let half = toggle_settings(Some(nested(1)), None, Some("  "));
        rig.invoke(2, &half).unwrap();
        assert_eq!(rig.next(), run(1, 2));
        assert_eq!(rig.next(), state(2, true, None));
        rig.invoke(2, &half).unwrap();
        assert_eq!(rig.next(), state(2, false, None));
        rig.none();
    }

    #[test]
    fn a_toggle_in_a_toggle_keeps_its_own_state() {
        let rig = Rig::new();
        let inner = toggle_settings(Some(nested(1)), None, None);
        let inner_action = Action {
            kind: ID.into(),
            settings: inner.clone(),
        };
        let outer = toggle_settings(Some(inner_action.clone()), Some(inner_action.clone()), None);
        rig.appear(1, &outer);
        rig.next();
        // The engine gives the inner toggle the binding of the outer toggle.
        rig.invoke(1, &outer).unwrap();
        assert_eq!(
            rig.next(),
            Message::Engine(EngineRequest::Invoke {
                binding: binding(1),
                action: inner_action.clone(),
            })
        );
        assert_eq!(rig.next(), state(1, true, None));
        rig.invoke(1, &inner).unwrap();
        assert_eq!(rig.next(), run(1, 1));
        assert_eq!(rig.next(), state(1, true, None));

        // After a page change and back, the two states stay.
        rig.appear(1, &outer);
        assert_eq!(rig.next(), state(1, true, None));
        rig.invoke(1, &inner).unwrap();
        assert_eq!(rig.next(), state(1, false, None));
        rig.none();
    }

    #[test]
    fn a_bad_setting_is_a_problem_and_changes_nothing() {
        let rig = Rig::new();
        rig.invoke(1, &values(&[("on_label", Value::Integer(1))]))
            .unwrap_err();
        rig.none();
        // The state did not change: the next correct push turns it on.
        rig.invoke(1, &Settings::new()).unwrap();
        assert_eq!(rig.next(), state(1, true, None));

        // At Appear, a bad label is no label, and no problem.
        rig.appear(1, &values(&[("on_label", Value::Integer(1))]));
        assert_eq!(rig.next(), state(1, false, None));
        rig.none();
    }

    #[test]
    fn the_feedback_has_the_state_the_label_for_on_and_the_other_state() {
        let on = feedback(true, Some("Mic on".into()));
        assert_eq!(on.state, Some(FeedbackState::On));
        assert_eq!(on.label.as_deref(), Some("Mic on"));
        let other = on.other_state.expect("the other state");
        assert_eq!(other.state, Some(FeedbackState::Off));
        assert_eq!(other.label, None);
        let off = feedback(false, Some("Mic on".into()));
        assert_eq!(off.state, Some(FeedbackState::Off));
        assert_eq!(off.label, None);
        let other = off.other_state.expect("the other state");
        assert_eq!(other.state, Some(FeedbackState::On));
        assert_eq!(other.label.as_deref(), Some("Mic on"));
    }
}
