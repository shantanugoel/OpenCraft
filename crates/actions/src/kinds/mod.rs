//! One module for each built-in action kind, and the registry of those
//! modules. The registry is the only list of the kinds.
//!
//! A kind module holds the kind name, the description (decision 0014), a
//! typed read of the settings, and the work. The work uses the system port
//! ([`crate::backend::System`]). The handler does not read the settings.
//!
//! [`REGISTERED`] lists the modules. The order is the order of the kinds in
//! the library (decision 0029, section 11). The kinds of one [`Group`] are
//! together, and the groups are in the order of the handlers in
//! [`crate::handlers::build`]. A test checks this. Kinds with the same
//! [`Group`] share one handler, one thread, and one queue (decision 0012).
//!
//! To add a kind, copy [`open_application`]: `ID`, `description`, `read`,
//! `perform`, and a value that implements [`Kind`]. Then add one line to
//! [`REGISTERED`]. [`crate::descriptions`] has no table of kinds: each kind
//! gives its own description. A new [`Group`] also needs a handler that calls
//! [`kinds_of`] and [`find_in`].
//!
//! [`Kind::invoke`] is `perform(&read(settings)?, work)`. A kind that sends
//! keys reads the settings first. It then refuses the work when
//! [`Work::keys_allowed`] is false. The key handler keeps one thread and one
//! poll of the Accessibility permission, and it passes the result on `Work`.
//! A Mac system function chooses, after the read, whether that function needs
//! the permission. A kind that must handle Appear, Disappear, Resolve, or
//! Choices adds a method on [`Kind`] with an empty default, and its handler
//! calls that method.
//!
//! A kind that only asks the engine for work (a page, a profile, a folder,
//! the device, a toggle, a sequence) calls [`Work::ask`]. The request
//! handler of its group ([`crate::flow`]) gives it the binding with
//! [`Work::binding`], the memory of the handler with [`Work::memory`], and
//! the shared system port with [`Work::shared_system`].

mod close_folder;
mod device_brightness;
mod hotkey;
mod mac_function;
mod media_key;
pub(crate) mod microphone_mute;
mod mouse;
mod open_application;
mod open_folder;
mod open_url;
pub(crate) mod page;
mod play_sound;
mod profile;
mod quit_application;
mod screen_off;
mod sequence;
mod shell_command;
pub(crate) mod sound_output;
mod stop_sound;
mod toggle;
mod type_text;
mod volume;

/// The test of the macOS calls checks this, so a key call and the permission
/// stay together.
#[cfg(all(test, target_os = "macos"))]
pub(crate) use mac_function::needs_permission as mac_function_needs_permission;

#[cfg(test)]
mod export_round_trip;

use crate::backend::System;
use crate::descriptions::setting_problem;
use opencraft_engine::{
    BindingId, ChoiceOption, EngineRequest, KindDescription, Message, MessageSender, Settings,
    Value,
};
use std::cell::RefCell;
use std::sync::Arc;

/// One handler. Kinds with the same group share its thread and its queue.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Group {
    /// Open an application, quit an application, open a URL, and a shell command.
    Open,
    /// Send a hotkey, type text, a media key, a mouse event, and a Mac system
    /// function. These kinds share one handler and its one poll of the
    /// Accessibility permission.
    Key,
    /// Sound volume. This kind keeps the volume handler and its poll.
    Volume,
    /// Microphone mute. The handler keeps the listener and the feedback.
    Microphone,
    /// Sound output. The handler keeps the listener, the feedback, and the
    /// choices.
    Output,
    /// Play a sound and stop all sound. These kinds share one handler and
    /// one queue, so a stop does not pass a play that came before it.
    Sound,
    /// Change the page, select a profile, open a folder, and close a folder.
    /// These kinds only ask the engine for work.
    Navigation,
    /// Toggle. The handler keeps the states of the toggles.
    Toggle,
    /// Sequence of actions. A sequence waits on a thread of its own.
    Sequence,
    /// Screen off and device brightness. These kinds only ask the engine for
    /// work.
    Device,
}

/// What a handler of the engine request kinds remembers between two calls.
/// The handler owns it and gives it to a kind on [`Work`].
#[derive(Default)]
pub(crate) struct Memory {
    /// The states of the toggles (decision 0017).
    pub(crate) toggles: toggle::States,
}

/// One call of a kind: the system port, and the sender of its handler.
///
/// A kind that finishes later (a shell command) sends a Problem on `sender`.
/// The key handler also sets the Accessibility permission. A kind that sends
/// keys reads it. Other kinds ignore it.
pub(crate) struct Work<'a> {
    pub(crate) system: &'a dyn System,
    pub(crate) sender: &'a Option<MessageSender>,
    /// True when the key handler has the Accessibility permission.
    /// The default is true, so a kind test and an open kind do the work.
    keys_allowed: bool,
    /// The key handler sets this. After a failed key or mouse send, the kind
    /// calls it. It returns true when the permission is still present, and
    /// the handler sends the new status. None keeps the system error.
    recheck_keys: Option<&'a dyn Fn() -> bool>,
    /// The binding of the call. Only the request handler sets it.
    binding: Option<BindingId>,
    /// The memory of the request handler.
    memory: Option<&'a RefCell<Memory>>,
    /// The system port as a shared pointer, for a kind that works on a thread
    /// of its own. Only the request handler sets it.
    shared_system: Option<&'a Arc<dyn System>>,
}

impl<'a> Work<'a> {
    pub(crate) fn new(system: &'a dyn System, sender: &'a Option<MessageSender>) -> Self {
        Self {
            system,
            sender,
            keys_allowed: true,
            recheck_keys: None,
            binding: None,
            memory: None,
            shared_system: None,
        }
    }

    /// The binding that the engine gave to this call.
    pub(crate) fn with_binding(mut self, binding: BindingId) -> Self {
        self.binding = Some(binding);
        self
    }

    /// The memory of the handler.
    pub(crate) fn with_memory(mut self, memory: &'a RefCell<Memory>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// The system port as a shared pointer.
    pub(crate) fn with_shared_system(mut self, system: &'a Arc<dyn System>) -> Self {
        self.shared_system = Some(system);
        self
    }

    /// The binding of this call, if the handler gave one.
    pub(crate) fn binding(&self) -> Option<BindingId> {
        self.binding
    }

    /// The memory of the handler, if the handler gave it.
    pub(crate) fn memory(&self) -> Option<&'a RefCell<Memory>> {
        self.memory
    }

    /// A shared pointer to the system port, if the handler gave it.
    pub(crate) fn shared_system(&self) -> Option<Arc<dyn System>> {
        self.shared_system.cloned()
    }

    /// Send a message to the engine. A closed channel is not an error: the
    /// shell may already have stopped.
    pub(crate) fn send(&self, message: Message) {
        send(self.sender, message);
    }

    /// Ask the engine for work (decision 0016).
    pub(crate) fn ask(&self, request: EngineRequest) {
        self.send(Message::Engine(request));
    }

    /// The permission from the one check of the key handler.
    pub(crate) fn with_keys(mut self, allowed: bool, recheck: &'a dyn Fn() -> bool) -> Self {
        self.keys_allowed = allowed;
        self.recheck_keys = Some(recheck);
        self
    }

    /// True when the key handler had the Accessibility permission.
    pub(crate) fn keys_allowed(&self) -> bool {
        self.keys_allowed
    }

    /// The text of a failed key or mouse send. A lost permission replaces the
    /// system error. The handler then sends the new status.
    pub(crate) fn input_error(&self, error: impl std::fmt::Display) -> String {
        if self.recheck_keys.is_some_and(|recheck| !recheck()) {
            crate::handlers::no_permission()
        } else {
            format!("Cannot send keys or mouse events: {error}.")
        }
    }
}

/// A built-in kind that has a module. The registry holds one value.
pub(crate) trait Kind: Send + Sync {
    /// The kind name, for example `opencraft.open-application`.
    fn id(&self) -> &'static str;

    /// The description that the shell uses to make the form (decision 0014).
    fn description(&self) -> KindDescription;

    /// Do the work of one Invoke.
    ///
    /// Implement this as `perform(&read(settings)?, work)`. `read` checks
    /// the settings. `perform` uses the system port. An error is the text
    /// of a Problem for the user.
    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String>;

    /// The choices of the runtime field `field`. `None` means that this kind
    /// has no runtime choices for that field. The default is `None`.
    fn choices(&self, field: &str, work: &Work<'_>) -> Option<Choices> {
        let _ = (field, work);
        None
    }

    /// The Appear of a binding with this kind. The binding is `work.binding()`.
    /// The default does nothing.
    fn appear(&self, settings: &Settings, work: &Work<'_>) {
        let _ = (settings, work);
    }
}

/// The runtime choices of one field (decision 0014).
pub(crate) struct Choices {
    pub(crate) options: Vec<ChoiceOption>,
    /// The handler sends this text as a Problem. The options stay valid, so
    /// the form still has the options that the kind could make.
    pub(crate) problem: Option<String>,
}

struct Registered {
    group: Group,
    kind: &'static dyn Kind,
}

/// The built-in kinds that have a module, in library order. A group is one
/// block of the registry, and the groups are in the order of the handlers in
/// [`crate::handlers::build`].
const REGISTERED: &[Registered] = &[
    Registered {
        group: Group::Open,
        kind: &open_application::KIND,
    },
    Registered {
        group: Group::Open,
        kind: &quit_application::KIND,
    },
    Registered {
        group: Group::Open,
        kind: &open_url::KIND,
    },
    Registered {
        group: Group::Open,
        kind: &shell_command::KIND,
    },
    Registered {
        group: Group::Key,
        kind: &hotkey::KIND,
    },
    Registered {
        group: Group::Key,
        kind: &type_text::KIND,
    },
    Registered {
        group: Group::Key,
        kind: &media_key::KIND,
    },
    Registered {
        group: Group::Key,
        kind: &mouse::KIND,
    },
    Registered {
        group: Group::Key,
        kind: &mac_function::KIND,
    },
    Registered {
        group: Group::Volume,
        kind: &volume::KIND,
    },
    Registered {
        group: Group::Device,
        kind: &screen_off::KIND,
    },
    Registered {
        group: Group::Device,
        kind: &device_brightness::KIND,
    },
    Registered {
        group: Group::Microphone,
        kind: &microphone_mute::KIND,
    },
    Registered {
        group: Group::Output,
        kind: &sound_output::KIND,
    },
    Registered {
        group: Group::Sound,
        kind: &play_sound::KIND,
    },
    Registered {
        group: Group::Sound,
        kind: &stop_sound::KIND,
    },
    Registered {
        group: Group::Navigation,
        kind: &page::KIND,
    },
    Registered {
        group: Group::Navigation,
        kind: &profile::KIND,
    },
    Registered {
        group: Group::Navigation,
        kind: &open_folder::KIND,
    },
    Registered {
        group: Group::Navigation,
        kind: &close_folder::KIND,
    },
    Registered {
        group: Group::Toggle,
        kind: &toggle::KIND,
    },
    Registered {
        group: Group::Sequence,
        kind: &sequence::KIND,
    },
];

/// Each registered kind, in library order.
#[cfg(test)]
pub(crate) fn all() -> impl Iterator<Item = &'static dyn Kind> {
    REGISTERED.iter().map(|entry| entry.kind)
}

/// The kinds of `group`, in library order.
pub(crate) fn kinds_of(group: Group) -> impl Iterator<Item = &'static dyn Kind> {
    REGISTERED
        .iter()
        .filter(move |entry| entry.group == group)
        .map(|entry| entry.kind)
}

/// The kind with this name, if it has a module.
pub(crate) fn find(id: &str) -> Option<&'static dyn Kind> {
    REGISTERED
        .iter()
        .find(|entry| entry.kind.id() == id)
        .map(|entry| entry.kind)
}

/// The kind with this name in `group`.
pub(crate) fn find_in(group: Group, id: &str) -> Option<&'static dyn Kind> {
    kinds_of(group).find(|kind| kind.id() == id)
}

/// Send a Problem when `result` is an error. A closed channel is not an error
/// for the handler: the shell may already have stopped.
pub(crate) fn problem(sender: &Option<MessageSender>, result: Result<(), String>) {
    if let Err(text) = result {
        send(sender, Message::Problem(text));
    }
}

/// Send a message. A closed channel or no channel is not an error.
pub(crate) fn send(sender: &Option<MessageSender>, message: Message) {
    if let Some(sender) = sender {
        let _ = sender.send(message);
    }
}

/// The text of `key` when it is not empty. The returned text keeps its
/// spaces. Empty text, a missing value, and a value that is not text are
/// the same problem.
pub(crate) fn required_text<'a>(
    kind: &str,
    settings: &'a Settings,
    key: &str,
) -> Result<&'a str, String> {
    match settings.get(key) {
        Some(Value::Text(value)) if !value.is_empty() => Ok(value),
        _ => Err(setting_problem(kind, key)),
    }
}

/// The text of `key` when it has a character that is not a space.
/// The returned text keeps its spaces. Empty text, text with only spaces,
/// a missing value, and a value that is not text are the same problem.
pub(crate) fn nonblank<'a>(
    kind: &str,
    settings: &'a Settings,
    key: &str,
) -> Result<&'a str, String> {
    match settings.get(key) {
        Some(Value::Text(value)) if !value.trim().is_empty() => Ok(value),
        _ => Err(setting_problem(kind, key)),
    }
}

/// The text of `key`, trimmed. A missing value and text with only spaces are
/// `None`. A value that is not text is a problem.
pub(crate) fn optional_text(
    kind: &str,
    settings: &Settings,
    key: &str,
) -> Result<Option<String>, String> {
    match settings.get(key) {
        None => Ok(None),
        Some(Value::Text(text)) => Ok(match text.trim() {
            "" => None,
            trimmed => Some(trimmed.to_string()),
        }),
        Some(_) => Err(setting_problem(kind, key)),
    }
}

/// The toggle `key`. A missing value is Off.
pub(crate) fn toggle(kind: &str, settings: &Settings, key: &str) -> Result<bool, String> {
    match settings.get(key) {
        None => Ok(false),
        Some(Value::Boolean(value)) => Ok(*value),
        Some(_) => Err(setting_problem(kind, key)),
    }
}

#[cfg(test)]
pub(crate) fn values(pairs: &[(&str, Value)]) -> Settings {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

#[cfg(test)]
pub(crate) fn assert_system_kind(description: &KindDescription, name: &str, icon: &str) {
    assert_eq!(description.name, name);
    assert_eq!(description.group, crate::descriptions::GROUP_SYSTEM);
    assert_eq!(description.icon.as_deref(), Some(icon));
    assert_eq!(description.help, None);
    assert!(!description.needs_accessibility);
    assert!(description.direction.is_none());
    assert!(description.states.is_none());
    assert!(!description.folder_back);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::{self, Intervals};
    use opencraft_engine::{InputType, Message};
    use std::sync::Arc;

    #[test]
    fn the_open_group_lists_its_kinds_in_library_order() {
        let ids: Vec<_> = kinds_of(Group::Open).map(|kind| kind.id()).collect();
        assert_eq!(
            ids,
            [
                open_application::ID,
                quit_application::ID,
                open_url::ID,
                shell_command::ID,
            ]
        );
        assert!(find("opencraft.not-a-kind").is_none());
        for id in ids {
            assert!(id.starts_with("opencraft."), "{id}");
            let kind = find(id).expect(id);
            assert_eq!(crate::descriptions::description(id), kind.description());
            assert!(find_in(Group::Open, id).is_some());
        }
        let open = handlers::build(
            Arc::new(crate::tests::support::Fake::default()),
            Intervals::default(),
        )
        .into_iter()
        .find(|handler| {
            handler
                .kinds()
                .iter()
                .any(|kind| kind.kind == open_application::ID)
        })
        .expect("the open handler");
        let published = open.kinds();
        let registered: Vec<_> = kinds_of(Group::Open)
            .map(|kind| kind.description())
            .collect();
        assert_eq!(published, registered);
    }

    #[test]
    fn the_other_groups_list_their_kinds_in_library_order() {
        let key_ids: Vec<_> = kinds_of(Group::Key).map(|kind| kind.id()).collect();
        assert_eq!(
            key_ids,
            [
                hotkey::ID,
                type_text::ID,
                media_key::ID,
                mouse::ID,
                mac_function::ID,
            ]
        );
        let ids = |group| kinds_of(group).map(|kind| kind.id()).collect::<Vec<_>>();
        assert_eq!(ids(Group::Volume), [volume::ID]);
        assert_eq!(ids(Group::Microphone), [microphone_mute::ID]);
        assert_eq!(ids(Group::Output), [sound_output::ID]);
        assert_eq!(ids(Group::Sound), [play_sound::ID, stop_sound::ID]);
        assert_eq!(
            ids(Group::Navigation),
            [page::ID, profile::ID, open_folder::ID, close_folder::ID]
        );
        assert_eq!(ids(Group::Toggle), [toggle::ID]);
        assert_eq!(ids(Group::Sequence), [sequence::ID]);
        assert_eq!(ids(Group::Device), [screen_off::ID, device_brightness::ID]);
        let handlers = handlers::build(
            Arc::new(crate::tests::support::Fake::default()),
            Intervals::default(),
        );
        for (id, group) in [
            (hotkey::ID, Group::Key),
            (volume::ID, Group::Volume),
            (microphone_mute::ID, Group::Microphone),
            (sound_output::ID, Group::Output),
            (play_sound::ID, Group::Sound),
            (page::ID, Group::Navigation),
            (toggle::ID, Group::Toggle),
            (sequence::ID, Group::Sequence),
            (screen_off::ID, Group::Device),
        ] {
            let handler = handlers
                .iter()
                .find(|handler| handler.kinds().iter().any(|kind| kind.kind == id))
                .expect(id);
            let registered: Vec<_> = kinds_of(group).map(|kind| kind.description()).collect();
            assert_eq!(handler.kinds(), registered);
            for id in kinds_of(group).map(|kind| kind.id()) {
                assert_eq!(
                    crate::descriptions::description(id),
                    find_in(group, id).expect(id).description()
                );
            }
        }
    }

    /// The library shows the kinds in the order of the handlers, and then in
    /// the order of each handler. Thus the registry is the one list of the
    /// order only if the two orders are the same (review 3, L2).
    #[test]
    fn the_kinds_of_the_handlers_are_in_the_order_of_the_registry() {
        let from_handlers: Vec<_> = handlers::build(
            Arc::new(crate::tests::support::Fake::default()),
            Intervals::default(),
        )
        .iter()
        .flat_map(|handler| handler.kinds())
        .map(|description| description.kind)
        .collect();
        let registered: Vec<_> = all().map(|kind| kind.id().to_string()).collect();
        assert_eq!(from_handlers, registered);
    }

    /// Run one action of a kind on a new fake system, with the keys allowed.
    /// Give the result, the calls, and the messages.
    fn run(
        kind: &dyn Kind,
        settings: &Settings,
    ) -> (
        Result<(), String>,
        Vec<crate::tests::support::Call>,
        Vec<Message>,
    ) {
        let system = crate::tests::support::Fake::default();
        let (sender, messages) = std::sync::mpsc::channel();
        let sender = Some(sender);
        let recheck = || true;
        let work = Work::new(&system, &sender).with_keys(true, &recheck);
        let result = kind.invoke(settings, &work);
        (result, system.calls(), messages.try_iter().collect())
    }

    /// The form and the read of each kind use the same limits, choices, and
    /// defaults (review 2, L6). A value that the form offers is never a
    /// setting problem for its own field, and a missing field works like a
    /// field that has its default.
    #[test]
    fn the_form_of_each_kind_offers_only_values_that_the_read_accepts() {
        for entry in REGISTERED {
            let kind = entry.kind;
            let description = kind.description();
            for field in &description.fields {
                let own_problem = Err(setting_problem(kind.id(), &field.key));
                let offered: Vec<Value> = match &field.input {
                    InputType::Integer { min, max } => {
                        vec![Value::Integer(*min), Value::Integer(*max)]
                    }
                    InputType::Choice { options } => options
                        .iter()
                        .map(|option| Value::Text(option.value.clone()))
                        .collect(),
                    _ => Vec::new(),
                };
                for value in offered {
                    let settings = values(&[(field.key.as_str(), value.clone())]);
                    assert_ne!(
                        run(kind, &settings).0,
                        own_problem,
                        "{}: the field {} does not accept {value:?}",
                        kind.id(),
                        field.key
                    );
                }
            }
            let defaults: Settings = description
                .fields
                .iter()
                .filter_map(|field| Some((field.key.clone(), field.default.clone()?)))
                .collect();
            assert_eq!(
                run(kind, &Settings::new()),
                run(kind, &defaults),
                "{}: a missing field is not the same as its default",
                kind.id()
            );
        }
    }
}
