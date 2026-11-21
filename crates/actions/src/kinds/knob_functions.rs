//! Select a function with Push, then run its action with a turn (decision 0042).

use crate::descriptions::{field, setting_problem, GROUP_LOGIC};
use crate::kinds::{Kind, Work};
use opencraft_engine::{
    BindingId, ControlId, EngineRequest, Feedback, FolderId, GestureKind, InputType,
    KindDescription, Message, Settings, TurnFunction,
};
use std::collections::BTreeMap;

pub(crate) const ID: &str = "opencraft.knob-functions";
const FUNCTIONS: &str = "functions";
pub(crate) static KIND: KnobFunctionsKind = KnobFunctionsKind;

pub(crate) struct KnobFunctionsKind;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ControlKey {
    page: usize,
    folder: Option<FolderId>,
    control: ControlId,
}

impl From<BindingId> for ControlKey {
    fn from(binding: BindingId) -> Self {
        Self {
            page: binding.page,
            folder: binding.folder,
            control: binding.control,
        }
    }
}

#[derive(Default)]
pub(crate) struct Positions(BTreeMap<ControlKey, Vec<(Settings, usize)>>);

impl Positions {
    fn position(&self, key: ControlKey, settings: &Settings) -> usize {
        self.0
            .get(&key)
            .and_then(|entries| entries.iter().find(|(known, _)| known == settings))
            .map_or(0, |(_, index)| *index)
    }

    fn set(&mut self, key: ControlKey, settings: &Settings, index: usize) {
        let entries = self.0.entry(key).or_default();
        if let Some(entry) = entries.iter_mut().find(|(known, _)| known == settings) {
            entry.1 = index;
        } else {
            entries.push((settings.clone(), index));
        }
    }

    fn forget_old(&mut self, key: ControlKey, settings: &Settings) {
        if let Some(entries) = self.0.get_mut(&key) {
            entries.retain(|(known, _)| known == settings);
        }
    }
}

impl Kind for KnobFunctionsKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        KindDescription {
            kind: ID.into(),
            name: "Multi-function knob".into(),
            group: GROUP_LOGIC.into(),
            help: Some("Knob".into()),
            icon: Some("sliders-horizontal".into()),
            fields: vec![field(FUNCTIONS, "Functions", InputType::TurnList)],
            ..Default::default()
        }
        .with_steps(FUNCTIONS)
        .with_knob_group()
    }

    fn appear(&self, settings: &Settings, work: &Work<'_>) {
        let Some(binding) = work.binding() else {
            return;
        };
        if let Some(memory) = work.memory() {
            memory
                .borrow_mut()
                .knob_functions
                .forget_old(binding.into(), settings);
        }
        let functions = read(settings).unwrap_or_default();
        let active = (!functions.is_empty()).then(|| {
            let index = position(work, binding, settings) % functions.len();
            (index, &functions[index])
        });
        send_feedback(work, binding, active);
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        let functions = read(settings)?;
        let Some(binding) = work.binding().filter(|_| !functions.is_empty()) else {
            return Ok(());
        };
        let index = position(work, binding, settings) % functions.len();
        match binding.gesture {
            GestureKind::Push => {
                let next = (index + 1) % functions.len();
                if let Some(memory) = work.memory() {
                    memory
                        .borrow_mut()
                        .knob_functions
                        .set(binding.into(), settings, next);
                }
                for gesture in [
                    GestureKind::Push,
                    GestureKind::TurnLeft,
                    GestureKind::TurnRight,
                ] {
                    send_feedback(
                        work,
                        BindingId { gesture, ..binding },
                        Some((next, &functions[next])),
                    );
                }
            }
            GestureKind::TurnLeft | GestureKind::TurnRight => {
                let function = &functions[index];
                let action = if binding.gesture == GestureKind::TurnLeft {
                    &function.left
                } else {
                    &function.right
                };
                work.ask(EngineRequest::Invoke {
                    binding,
                    action: action.clone(),
                });
            }
            _ => {}
        }
        Ok(())
    }
}

fn read(settings: &Settings) -> Result<Vec<TurnFunction>, String> {
    let Some(value) = settings.get(FUNCTIONS) else {
        return Ok(Vec::new());
    };
    let mut items =
        TurnFunction::list_from_value(value).ok_or_else(|| setting_problem(ID, FUNCTIONS))?;
    for item in &mut items {
        item.label = item
            .label
            .take()
            .map(|label| label.trim().to_owned())
            .filter(|label| !label.is_empty());
    }
    Ok(items)
}

fn position(work: &Work<'_>, binding: BindingId, settings: &Settings) -> usize {
    work.memory().map_or(0, |memory| {
        memory
            .borrow()
            .knob_functions
            .position(binding.into(), settings)
    })
}

fn send_feedback(work: &Work<'_>, binding: BindingId, active: Option<(usize, &TurnFunction)>) {
    work.send(Message::Feedback {
        binding,
        feedback: active.map(|(index, function)| Feedback {
            // Push must replace the most recent visual feedback from either turn.
            label: (binding.gesture == GestureKind::Push).then(|| {
                function
                    .label
                    .clone()
                    .unwrap_or_else(|| format!("Function {}", index + 1))
            }),
            step: Some(index),
            ..Default::default()
        }),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::Memory;
    use crate::tests::support::Fake;
    use opencraft_engine::{Action, ControlKind, Value};
    use std::{cell::RefCell, sync::mpsc};

    #[test]
    fn each_gesture_shares_the_index_and_an_edit_resets_it() {
        let system = Fake::default();
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        let memory = RefCell::new(Memory::default());
        let binding = |gesture| BindingId {
            page: 0,
            folder: None,
            control: ControlId::new(ControlKind::Knob, 2),
            gesture,
        };
        let work = |gesture| {
            Work::new(&system, &sender)
                .with_binding(binding(gesture))
                .with_memory(&memory)
        };
        let push = GestureKind::Push;
        let left = GestureKind::TurnLeft;
        let right = GestureKind::TurnRight;
        let feedback = |gesture, index, title: Option<&str>| {
            assert!(matches!(messages.try_recv().unwrap(),
                Message::Feedback { binding: id, feedback: Some(feedback) }
                if id == binding(gesture) && feedback.step == Some(index) && feedback.label.as_deref() == title));
        };
        let action = |kind: &str| Action {
            kind: kind.into(),
            settings: Settings::new(),
        };
        let mut function = TurnFunction {
            label: None,
            left: action("test.left"),
            right: action("test.right"),
        };
        let settings = |items: &[TurnFunction]| {
            Settings::from([(FUNCTIONS.into(), TurnFunction::list_to_value(items))])
        };
        let original = settings(&[function.clone(), function.clone()]);
        for gesture in [left, right, push] {
            KIND.appear(&original, &work(gesture));
            feedback(gesture, 0, (gesture == push).then_some("Function 1"));
        }
        for (index, label) in [(1, "Function 2"), (0, "Function 1"), (1, "Function 2")] {
            KIND.invoke(&original, &work(push)).unwrap();
            feedback(push, index, Some(label));
            feedback(left, index, None);
            feedback(right, index, None);
            for (gesture, expected) in [(left, &function.left), (right, &function.right)] {
                KIND.invoke(&original, &work(gesture)).unwrap();
                assert!(matches!(messages.try_recv().unwrap(),
                    Message::Engine(EngineRequest::Invoke { binding: id, action }) if id == binding(gesture) && &action == expected));
            }
        }
        function.label = Some("  Volume  ".into());
        KIND.appear(&settings(&[function]), &work(push));
        feedback(push, 0, Some("Volume"));
        KIND.appear(&original, &work(push));
        feedback(push, 0, Some("Function 1"));

        let empty = Settings::new();
        KIND.appear(&empty, &work(push));
        assert!(matches!(
            messages.try_recv().unwrap(),
            Message::Feedback { feedback: None, .. }
        ));
        KIND.invoke(&empty, &work(push)).unwrap();
        assert!(messages.try_recv().is_err());
        let malformed = Settings::from([(FUNCTIONS.into(), Value::Integer(1))]);
        assert_eq!(
            KIND.invoke(&malformed, &work(push)),
            Err(setting_problem(ID, FUNCTIONS))
        );
    }
}
