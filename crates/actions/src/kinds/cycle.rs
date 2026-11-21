//! Run one step per invocation. Remember the next index for each binding
//! and settings value while the handler runs (decision 0042).

use crate::descriptions::{field, setting_problem, GROUP_LOGIC};
use crate::kinds::{Kind, Work};
use opencraft_engine::{
    Action, BindingId, EngineRequest, Feedback, InputType, KindDescription, Message, Settings,
};
use std::collections::BTreeMap;

pub(crate) const ID: &str = "opencraft.cycle";
const NAME: &str = "Action cycle";
const STEPS: &str = "steps";

pub(crate) static KIND: CycleKind = CycleKind;

pub(crate) struct CycleKind;

impl Kind for CycleKind {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        KindDescription {
            kind: ID.into(),
            name: NAME.into(),
            group: GROUP_LOGIC.into(),
            icon: Some("repeat".into()),
            fields: vec![field(STEPS, "Actions", InputType::ActionList)],
            ..KindDescription::default()
        }
        .with_steps(STEPS)
    }

    fn appear(&self, settings: &Settings, work: &Work<'_>) {
        let (Some(binding), Some(memory)) = (work.binding(), work.memory()) else {
            return;
        };
        let mut memory = memory.borrow_mut();
        memory.cycles.forget_old(binding, settings);
        let count = read(settings).map_or(0, |steps| steps.len());
        let step = (count > 0).then(|| memory.cycles.position(binding, settings) % count);
        send_step(work, binding, step);
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(read(settings)?, settings, work)
    }
}

pub(crate) fn read(settings: &Settings) -> Result<Vec<Action>, String> {
    match settings.get(STEPS) {
        None => Ok(Vec::new()),
        Some(value) => Action::list_from_value(value).ok_or_else(|| setting_problem(ID, STEPS)),
    }
}

fn send_step(work: &Work<'_>, binding: BindingId, step: Option<usize>) {
    work.send(Message::Feedback {
        binding,
        feedback: step.map(|step| Feedback {
            step: Some(step),
            ..Feedback::default()
        }),
    });
}

fn perform(steps: Vec<Action>, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
    if steps.is_empty() {
        return Ok(());
    }
    let (Some(binding), Some(memory)) = (work.binding(), work.memory()) else {
        return Err(
            "An action cycle needs the binding of a control and the memory of its handler.".into(),
        );
    };
    let (current, next) = {
        let mut memory = memory.borrow_mut();
        let current = memory.cycles.position(binding, settings) % steps.len();
        let next = (current + 1) % steps.len();
        memory.cycles.set(binding, settings, next);
        (current, next)
    };
    work.ask(EngineRequest::Invoke {
        binding,
        action: steps[current].clone(),
    });
    send_step(work, binding, Some(next));
    Ok(())
}

#[derive(Default)]
pub(crate) struct Positions {
    by_binding: BTreeMap<BindingId, Vec<(Settings, usize)>>,
}

impl Positions {
    fn position(&self, binding: BindingId, settings: &Settings) -> usize {
        self.by_binding
            .get(&binding)
            .and_then(|entries| entries.iter().find(|(known, _)| known == settings))
            .map_or(0, |(_, index)| *index)
    }

    fn set(&mut self, binding: BindingId, settings: &Settings, next: usize) {
        let entries = self.by_binding.entry(binding).or_default();
        if let Some(entry) = entries.iter_mut().find(|(known, _)| known == settings) {
            entry.1 = next;
        } else {
            entries.push((settings.clone(), next));
        }
    }

    fn forget_old(&mut self, binding: BindingId, settings: &Settings) {
        if let Some(entries) = self.by_binding.get_mut(&binding) {
            entries
                .retain(|(known, _)| known == settings || super::toggle::contains(settings, known));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::Memory;
    use crate::tests::support::Fake;
    use opencraft_engine::{ControlId, ControlKind, GestureKind, Value};
    use std::cell::RefCell;
    use std::sync::mpsc;

    fn settings(steps: &[Action]) -> Settings {
        Settings::from([(STEPS.into(), Action::list_to_value(steps))])
    }

    fn action(kind: &str) -> Action {
        Action {
            kind: kind.into(),
            settings: Settings::new(),
        }
    }

    #[test]
    fn read_accepts_a_missing_or_empty_list_and_reads_the_steps() {
        assert!(read(&Settings::new()).unwrap().is_empty());
        assert!(read(&settings(&[])).unwrap().is_empty());
        let steps = [action("test.first"), action("test.second")];
        assert_eq!(read(&settings(&steps)).unwrap(), steps);
    }

    #[test]
    fn read_rejects_a_bad_list() {
        for bad in [Value::Integer(1), Value::List(vec![Value::Integer(1)])] {
            assert_eq!(
                read(&Settings::from([(STEPS.into(), bad)])),
                Err(setting_problem(ID, STEPS))
            );
        }
    }

    fn with_work(test: impl FnOnce(&Work<'_>, BindingId, mpsc::Receiver<Message>)) {
        let (sender, messages) = mpsc::channel();
        let sender = Some(sender);
        let memory = RefCell::new(Memory::default());
        let system = Fake::default();
        let binding = BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Key, 1),
            gesture: GestureKind::Press,
            folder: None,
        };
        let work = Work::new(&system, &sender)
            .with_binding(binding)
            .with_memory(&memory);
        test(&work, binding, messages);
    }

    fn expect_step(messages: &mpsc::Receiver<Message>, binding: BindingId, next: usize) {
        assert!(matches!(
            messages.try_recv().unwrap(),
            Message::Feedback { binding: id, feedback: Some(Feedback { step: Some(step), .. }) }
                if id == binding && step == next
        ));
    }

    #[test]
    fn two_equal_steps_advance_by_index_and_wrap_around() {
        with_work(|work, binding, messages| {
            let step = action("test.repeat");
            let settings = settings(&[step.clone(), step.clone()]);
            KIND.appear(&settings, work);
            expect_step(&messages, binding, 0);
            for next in [1, 0] {
                KIND.invoke(&settings, work).unwrap();
                assert!(matches!(
                    messages.try_recv().unwrap(),
                    Message::Engine(EngineRequest::Invoke { binding: id, action })
                        if id == binding && action == step
                ));
                expect_step(&messages, binding, next);
                assert_eq!(
                    work.memory()
                        .unwrap()
                        .borrow()
                        .cycles
                        .position(binding, &settings),
                    next
                );
            }
        });
    }

    #[test]
    fn changing_settings_resets_the_position_to_zero_on_appear() {
        with_work(|work, binding, messages| {
            let original = settings(&[action("test.first"), action("test.second")]);
            KIND.invoke(&original, work).unwrap();
            messages.try_recv().unwrap();
            expect_step(&messages, binding, 1);

            let edited = settings(&[action("test.second"), action("test.first")]);
            KIND.appear(&edited, work);
            expect_step(&messages, binding, 0);
            // A later edit back to the original settings starts at 0 too.
            KIND.appear(&original, work);
            expect_step(&messages, binding, 0);
        });
    }
}
