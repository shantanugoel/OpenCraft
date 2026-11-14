//! The action "Sequence of actions".
//!
//! The settings are a list of actions, and a delay between two actions in
//! milliseconds (decision 0017 and its amendment for #282). At each push, a
//! thread of its own asks the engine to do each action in the order of the
//! list, and waits for the delay before each action. The handler does not
//! wait for the thread, so its next request does not wait (decision 0012).
//! Each push starts a new thread: two sequences can run at the same time.
//! The thread ends after the last action, also when the handler stops.
//!
//! A missing list is no action. A step with no `delay_before_ms` waits for 0
//! if it is the first step, else for `delay_ms` (default [`DEFAULT_DELAY`]).
//! Thus an old profile keeps its timing. The step delay is not a setting of
//! the kind of the step, so it is removed before the engine gets the action.

use crate::descriptions::{field, field_with_default, setting_problem, GROUP_LOGIC};
use crate::kinds::{Kind, Work};
use opencraft_engine::{
    Action, EngineRequest, Field, InputType, KindDescription, Message, Settings, Value,
};
use std::thread;
use std::time::Duration;

/// The kind name.
pub(crate) const ID: &str = "opencraft.sequence";

const NAME: &str = "Sequence of actions";

/// The largest delay of a sequence, in milliseconds.
pub(crate) const DELAY_LIMIT: i64 = 10_000;
/// The delay of a sequence with no delay in its settings, in milliseconds.
pub(crate) const DEFAULT_DELAY: i64 = 100;

/// The registry value for this kind.
pub(crate) static KIND: SequenceKind = SequenceKind;

pub(crate) struct SequenceKind;

impl Kind for SequenceKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(read(settings)?, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_LOGIC.into(),
        icon: Some("list".into()),
        fields: vec![
            field("actions", "Actions", InputType::TimedActionList),
            field_with_default(
                "delay_ms",
                "Delay between two actions, in milliseconds",
                InputType::Integer {
                    min: 0,
                    max: DELAY_LIMIT,
                },
                Value::Integer(DEFAULT_DELAY),
            ),
        ],
        ..KindDescription::default()
    }
}

/// Each action of the sequence, with the delay before it.
pub(crate) type Steps = Vec<(Action, Duration)>;

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<Steps, String> {
    let actions = match settings.get("actions") {
        None => Vec::new(),
        Some(value) => {
            Action::list_from_value(value).ok_or_else(|| setting_problem(ID, "actions"))?
        }
    };
    let delay = match settings.get("delay_ms") {
        None => DEFAULT_DELAY,
        Some(Value::Integer(ms)) if (0..=DELAY_LIMIT).contains(ms) => *ms,
        Some(_) => return Err(setting_problem(ID, "delay_ms")),
    };
    let step = Field::step_delay(0);
    actions
        .into_iter()
        .enumerate()
        .map(|(index, mut action)| {
            let ms = match action.settings.remove(&step.key) {
                None if index == 0 => 0,
                None => delay,
                Some(Value::Integer(ms)) if (0..=DELAY_LIMIT).contains(&ms) => ms,
                Some(_) => {
                    return Err(format!(
                    "{NAME}: set the field \"{}\" of step {} to a number from 0 to {DELAY_LIMIT}.",
                    step.label,
                    index + 1
                ))
                }
            };
            Ok((action, Duration::from_millis(ms as u64)))
        })
        .collect()
}

/// Start the thread that asks the engine for each action. A sequence with no
/// action starts no thread.
pub(crate) fn perform(steps: Steps, work: &Work<'_>) -> Result<(), String> {
    if steps.is_empty() {
        return Ok(());
    }
    let Some(binding) = work.binding() else {
        return Err("A sequence needs the binding of a control.".into());
    };
    let Some(system) = work.shared_system() else {
        return Err("A sequence needs a system that a thread can share.".into());
    };
    let sender = work.sender.clone();
    thread::Builder::new()
        .name("opencraft-sequence".into())
        .spawn(move || {
            for (action, delay) in steps {
                if !delay.is_zero() {
                    system.sleep(delay);
                }
                crate::handlers::send(
                    &sender,
                    Message::Engine(EngineRequest::Invoke { binding, action }),
                );
            }
        })
        .map(|_| ())
        .map_err(|error| format!("Cannot start the sequence: {error}."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::System as SystemPort;
    use crate::kinds::recording::{Call, System};
    use crate::kinds::values;
    use opencraft_engine::{BindingId, ControlId, ControlKind, GestureKind};
    use std::sync::{mpsc, Arc};

    fn binding() -> BindingId {
        BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Key, 1),
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

    fn with_delay(mut action: Action, ms: Value) -> Action {
        action.settings.insert("delay_before_ms".into(), ms);
        action
    }

    fn sequence(actions: &[Action], delay_ms: Option<i64>) -> Settings {
        let mut pairs = vec![("actions", Action::list_to_value(actions))];
        if let Some(delay) = delay_ms {
            pairs.push(("delay_ms", Value::Integer(delay)));
        }
        values(&pairs)
    }

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    /// Run the sequence, and wait for the end of its thread: the thread
    /// drops its sender when it ends.
    fn run(settings: &Settings, system: &Arc<System>) -> Result<Vec<Message>, String> {
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        let shared: Arc<dyn SystemPort> = system.clone();
        let work = Work::new(system.as_ref(), &sender)
            .with_binding(binding())
            .with_shared_system(&shared);
        KIND.invoke(settings, &work)?;
        drop(sender);
        Ok(messages.iter().collect())
    }

    fn engine(number: i64) -> Message {
        Message::Engine(EngineRequest::Invoke {
            binding: binding(),
            action: nested(number),
        })
    }

    #[test]
    fn the_description_has_a_timed_list_and_a_default_delay() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_eq!(description.name, NAME);
        assert_eq!(description.group, GROUP_LOGIC);
        assert_eq!(description.icon.as_deref(), Some("list"));
        assert_eq!(description.help, None);
        assert!(!description.needs_accessibility);
        assert!(description.direction.is_none());
        assert!(description.states.is_none());
        assert!(!description.folder_back);
        assert!(description.shown_when.is_empty());
        let keys: Vec<_> = description.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, ["actions", "delay_ms"]);
        assert_eq!(description.fields[0].input, InputType::TimedActionList);
        assert_eq!(description.fields[0].default, None);
        assert_eq!(
            description.fields[1].input,
            InputType::Integer {
                min: 0,
                max: 10_000
            }
        );
        assert_eq!(description.fields[1].default, Some(Value::Integer(100)));
        assert_eq!(crate::descriptions::description(ID), description);
    }

    #[test]
    fn read_gives_the_delay_before_each_step_and_removes_it_from_the_action() {
        assert_eq!(read(&Settings::new()).unwrap(), []);
        assert_eq!(read(&sequence(&[], Some(5))).unwrap(), []);
        // The first step waits for 0, a later step for the delay of the list.
        assert_eq!(
            read(&sequence(&[nested(1), nested(2), nested(3)], Some(250))).unwrap(),
            [
                (nested(1), ms(0)),
                (nested(2), ms(250)),
                (nested(3), ms(250))
            ]
        );
        // The default delay is 100. A delay of 0 is a delay of 0.
        assert_eq!(
            read(&sequence(&[nested(1), nested(2)], None)).unwrap()[1].1,
            ms(100)
        );
        assert_eq!(
            read(&sequence(&[nested(1), nested(2)], Some(0))).unwrap()[1].1,
            ms(0)
        );
        // A delay of a step wins, also before the first step.
        let steps: Vec<_> = [30, 0, 90]
            .into_iter()
            .enumerate()
            .map(|(index, delay)| with_delay(nested(index as i64 + 1), Value::Integer(delay)))
            .collect();
        assert_eq!(
            read(&sequence(&steps, Some(250))).unwrap(),
            [(nested(1), ms(30)), (nested(2), ms(0)), (nested(3), ms(90))]
        );
    }

    #[test]
    fn read_rejects_a_bad_list_delay_or_step_delay() {
        let list = "Sequence of actions: select the action in the field \"Actions\" again.";
        let delay = "Sequence of actions: set the field \"Delay between two actions, in \
                     milliseconds\" to a number from 0 to 10000.";
        let good = Action::list_to_value(&[nested(1)]);
        for (pairs, expected) in [
            (vec![("actions", Value::Text("x".into()))], list),
            (
                vec![(
                    "actions",
                    Value::List(vec![nested(1).to_value(), Value::Integer(2)]),
                )],
                list,
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
            assert_eq!(read(&values(&pairs)).unwrap_err(), expected);
        }
        for value in [
            Value::Integer(-1),
            Value::Integer(10_001),
            Value::Text("100".into()),
        ] {
            let error =
                read(&sequence(&[nested(1), with_delay(nested(2), value)], None)).unwrap_err();
            assert!(
                error.contains("\"Delay before, in milliseconds\" of step 2"),
                "{error}"
            );
        }
    }

    #[test]
    fn a_push_asks_for_each_action_in_order_and_waits_before_each_later_action() {
        let system = Arc::new(System::default());
        let messages = run(
            &sequence(&[nested(1), nested(2), nested(3)], Some(250)),
            &system,
        )
        .unwrap();
        assert_eq!(messages, [engine(1), engine(2), engine(3)]);
        // No wait before the first action, and none after the last.
        assert_eq!(system.calls(), [Call::Sleep(250), Call::Sleep(250)]);
    }

    #[test]
    fn a_step_waits_before_its_request_and_the_child_gets_no_timing() {
        let system = Arc::new(System::default());
        let steps: Vec<_> = [30, 0, 90]
            .into_iter()
            .enumerate()
            .map(|(index, delay)| with_delay(nested(index as i64 + 1), Value::Integer(delay)))
            .collect();
        let messages = run(&sequence(&steps, Some(250)), &system).unwrap();
        assert_eq!(messages, [engine(1), engine(2), engine(3)]);
        assert_eq!(system.calls(), [Call::Sleep(30), Call::Sleep(90)]);
    }

    #[test]
    fn a_sequence_with_no_action_or_a_bad_setting_starts_nothing() {
        let system = Arc::new(System::default());
        assert_eq!(run(&Settings::new(), &system).unwrap(), []);
        assert_eq!(run(&sequence(&[], Some(5)), &system).unwrap(), []);
        run(&values(&[("delay_ms", Value::Integer(-1))]), &system).unwrap_err();
        assert!(system.calls().is_empty());
    }

    #[test]
    fn a_sequence_needs_a_binding_and_a_shared_system() {
        let system = Arc::new(System::default());
        let sender = None;
        let settings = sequence(&[nested(1)], None);
        KIND.invoke(&settings, &Work::new(system.as_ref(), &sender))
            .unwrap_err();
        let shared: Arc<dyn SystemPort> = system.clone();
        KIND.invoke(
            &settings,
            &Work::new(system.as_ref(), &sender).with_shared_system(&shared),
        )
        .unwrap_err();
    }
}
