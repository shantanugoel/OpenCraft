//! One module for each built-in action kind that has moved, and the registry
//! of those modules.
//!
//! A kind module holds the kind name, the description (decision 0014), a
//! typed read of the settings, and the work. The work uses the system port
//! ([`crate::backend::System`]). The handler does not read the settings.
//!
//! [`REGISTERED`] lists the modules. The order is the order of the kinds in
//! the library (decision 0029, section 11). Kinds with the same [`Group`]
//! share one handler, one thread, and one queue (decision 0012).
//!
//! To add a kind, copy [`open_application`]: `ID`, `description`, `read`,
//! `perform`, and a value that implements [`Kind`]. Then add one line to
//! [`REGISTERED`]. Do not add the kind to the tables in
//! [`crate::descriptions`]. Those tables serve only a kind that is not in
//! the registry yet. A new [`Group`] also needs a handler that calls
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

mod hotkey;
mod mac_function;
mod media_key;
mod mouse;
mod open_application;
mod open_url;
mod quit_application;
mod shell_command;
mod type_text;
mod volume;

#[cfg(test)]
mod recording;

use crate::backend::System;
use crate::descriptions::setting_problem;
use opencraft_engine::{KindDescription, Message, MessageSender, Settings, Value};

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
}

impl<'a> Work<'a> {
    pub(crate) fn new(system: &'a dyn System, sender: &'a Option<MessageSender>) -> Self {
        Self {
            system,
            sender,
            keys_allowed: true,
            recheck_keys: None,
        }
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
}

struct Registered {
    group: Group,
    kind: &'static dyn Kind,
}

/// The built-in kinds that have a module, in library order.
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
];

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
        if let Some(sender) = sender {
            let _ = sender.send(Message::Problem(text));
        }
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
        let open = handlers::build(Arc::new(recording::System::default()), Intervals::default())
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
    fn the_key_and_volume_groups_list_their_kinds_in_library_order() {
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
        let volume_ids: Vec<_> = kinds_of(Group::Volume).map(|kind| kind.id()).collect();
        assert_eq!(volume_ids, [volume::ID]);
        let handlers =
            handlers::build(Arc::new(recording::System::default()), Intervals::default());
        for (id, group) in [(hotkey::ID, Group::Key), (volume::ID, Group::Volume)] {
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
}
