//! The action "Run a shell command".
//!
//! The command runs on its own thread, so the next request of the handler
//! does not wait (decision 0012). The handler can stop while the command
//! still runs.

use crate::backend::ShellChild;
use crate::descriptions::{field, GROUP_SYSTEM};
use crate::kinds::{nonblank, problem, Kind, Work};
use opencraft_engine::{InputType, KindDescription, Settings};
use std::sync::mpsc;
use std::thread;

/// The kind name.
pub(crate) const ID: &str = "opencraft.shell-command";

const NAME: &str = "Run a shell command";

/// The registry value for this kind.
pub(crate) static KIND: ShellCommand = ShellCommand;

pub(crate) struct ShellCommand;

impl Kind for ShellCommand {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(&read(settings)?, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_SYSTEM.into(),
        icon: Some("terminal".into()),
        fields: vec![field("command", "Command", InputType::LongText)],
        ..KindDescription::default()
    }
}

/// The command line, after a successful read of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Command {
    pub(crate) command: String,
}

/// Read the settings. The error is the text for the user.
/// The command keeps its spaces. Text with only spaces is a problem.
pub(crate) fn read(settings: &Settings) -> Result<Command, String> {
    Ok(Command {
        command: nonblank(ID, settings, "command")?.to_string(),
    })
}

/// Start the command. The call returns when the command has started, not
/// when it ends. A failure to start, or a failure of the waiter thread, is
/// the error. A later exit that is not 0 is a Problem on the sender.
pub(crate) fn perform(command: &Command, work: &Work<'_>) -> Result<(), String> {
    // Make the waiter first. If its thread cannot start, no child exists
    // that the system must remove.
    let (slot, incoming) = mpsc::channel::<Box<dyn ShellChild>>();
    let sender = work.sender.clone();
    thread::Builder::new()
        .name("opencraft-shell-command".into())
        .spawn(move || {
            let Ok(child) = incoming.recv() else {
                return;
            };
            let result = (|| match child
                .wait()
                .map_err(|e| format!("Cannot wait for the shell command: {e}."))?
            {
                Some(0) => Ok(()),
                Some(code) => Err(format!("The shell command ends with code {code}.")),
                None => Err("A signal stops the shell command (no exit code).".into()),
            })();
            problem(&sender, result);
        })
        .map_err(|e| format!("Cannot start the shell command thread: {e}."))?;
    let child = work
        .system
        .start_shell_command(&command.command)
        .map_err(|e| format!("Cannot start the shell command: {e}."))?;
    slot.send(child)
        .map_err(|_| "Cannot wait for the shell command.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptions::setting_problem;
    use crate::kinds::recording::{Call, Fault, ShellEnd, System};
    use crate::kinds::{assert_system_kind, values, Work};
    use opencraft_engine::{Message, MessageSender, Settings, Value};
    use std::sync::mpsc::Receiver;
    use std::time::Duration;

    fn channel() -> (Option<MessageSender>, Receiver<Message>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (Some(tx), rx)
    }

    #[test]
    fn the_description_is_one_command_field() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_system_kind(&description, NAME, "terminal");
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(description.fields.len(), 1);
        assert_eq!(description.fields[0].key, "command");
        assert_eq!(description.fields[0].label, "Command");
        assert_eq!(description.fields[0].input, InputType::LongText);
        assert_eq!(description.fields[0].default, None);
    }

    #[test]
    fn read_keeps_the_command_text_and_rejects_blank_text() {
        assert_eq!(
            read(&values(&[("command", Value::Text(" echo hi ".into()))])).unwrap(),
            Command {
                command: " echo hi ".into(),
            }
        );
        assert_eq!(
            read(&Settings::new()).unwrap_err(),
            setting_problem(ID, "command")
        );
        assert_eq!(
            read(&values(&[("command", Value::Text(" \n ".into()))])).unwrap_err(),
            setting_problem(ID, "command")
        );
        assert_eq!(
            read(&values(&[("command", Value::Integer(1))])).unwrap_err(),
            setting_problem(ID, "command")
        );
    }

    #[test]
    fn a_bad_read_does_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(&Settings::new(), &Work::new(&system, &sender))
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "command"));
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_starts_the_command_and_reports_the_exit_later() {
        let system = System::default();
        let (sender, messages) = channel();
        perform(
            &Command {
                command: "echo".into(),
            },
            &Work::new(&system, &sender),
        )
        .unwrap();
        assert_eq!(system.calls(), [Call::Shell("echo".into())]);
        assert!(system.wait_until_finished());
        assert!(messages.recv_timeout(Duration::from_millis(200)).is_err());

        system.set_end(ShellEnd::Code(7));
        let (sender, messages) = channel();
        perform(
            &Command {
                command: "exit 7".into(),
            },
            &Work::new(&system, &sender),
        )
        .unwrap();
        match messages.recv_timeout(Duration::from_secs(2)).unwrap() {
            Message::Problem(text) => assert!(text.contains("code 7"), "{text}"),
            other => panic!("expected a Problem, got {other:?}"),
        }

        system.set_end(ShellEnd::Signal);
        let (sender, messages) = channel();
        perform(
            &Command {
                command: "signal".into(),
            },
            &Work::new(&system, &sender),
        )
        .unwrap();
        match messages.recv_timeout(Duration::from_secs(2)).unwrap() {
            Message::Problem(text) => assert!(text.contains("signal"), "{text}"),
            other => panic!("expected a Problem, got {other:?}"),
        }

        system.set_end(ShellEnd::WaitFailed);
        let (sender, messages) = channel();
        perform(
            &Command {
                command: "wait".into(),
            },
            &Work::new(&system, &sender),
        )
        .unwrap();
        match messages.recv_timeout(Duration::from_secs(2)).unwrap() {
            Message::Problem(text) => {
                assert!(text.contains("Cannot wait for the shell command"), "{text}");
                assert!(text.contains("42"), "{text}");
            }
            other => panic!("expected a Problem, got {other:?}"),
        }
    }

    #[test]
    fn a_system_error_at_the_start_is_an_error_of_perform() {
        let system = System::default();
        system.fail(Fault::Code("shell"));
        let sender = None;
        let error = perform(
            &Command {
                command: "echo".into(),
            },
            &Work::new(&system, &sender),
        )
        .unwrap_err();
        assert!(error.contains("Cannot start the shell command"), "{error}");
        assert!(error.contains("42"), "{error}");
        assert_eq!(system.calls(), [Call::Shell("echo".into())]);
    }
}
