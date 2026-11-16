//! The three tools of the MCP server, made from `COMMANDS` (decision 0040,
//! section 6).
//!
//! The client of the command line and the server have one source of truth:
//! the list `COMMANDS` of `ctl/commands.rs`. The enums, the descriptions, and
//! the help texts come from that list when the server starts. A test checks
//! that no command is missing and that the read tool holds exactly the read
//! requests of `control/protocol.rs`.

use crate::ctl::commands::{Arg, Command, Kind, COMMANDS};
use serde_json::{json, Map, Value};

/// The methods of the protocol that only read. `Method::reads` is the check
/// of the protocol; the test `the_tools_cannot_drift_from_the_commands`
/// compares this list with it, with the examples of the commands.
pub(super) const READS: &[&str] = &[
    "state",
    "layout",
    "kinds",
    "icons",
    "choices",
    "profiles",
    "pages",
    "folders",
    "content",
    "device_settings",
    "history",
    "connection",
];

/// The name of the tool that reads.
pub(super) const READ_TOOL: &str = "opencraft_read";
/// The name of the tool that changes.
pub(super) const CHANGE_TOOL: &str = "opencraft_change";
/// The name of the tool that gives the help.
pub(super) const HELP_TOOL: &str = "opencraft_help";

/// True if the command only reads.
pub(super) fn is_read(command: &Command) -> bool {
    READS.contains(&command.method)
}

/// The three tools, as `tools/list` gives them.
pub(super) fn list() -> Value {
    json!([read_tool(), change_tool(), help_tool()])
}

fn names(read: bool) -> Vec<&'static str> {
    COMMANDS
        .iter()
        .filter(|command| is_read(command) == read)
        .map(|command| command.name)
        .collect()
}

/// The names of the commands of the group: the start of the description of
/// the tool. The help of a command is one tool result away, in
/// `opencraft_help`, thus the description stays short (Claude Code cuts a
/// description after 4,096 characters).
fn description(intro: &str, read: bool) -> String {
    let names: Vec<String> = COMMANDS
        .iter()
        .filter(|command| is_read(command) == read)
        .map(|command| format!("`{}`", command.name))
        .collect();
    format!("{intro}\n\nThe commands: {}.", names.join(", "))
}

/// The schema of a tool that runs a command.
fn schema(read: bool) -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": {
                "type": "string",
                "enum": names(read),
                "description": "The command to run. The description of this tool names each command. `opencraft_help` gives the arguments and an example of one command."
            },
            "args": {
                "type": "object",
                "description": "The arguments of the command. A key is the flag of `opencraft ctl <command>` without `--`: {\"profile\": \"work\", \"page\": \"0123456789abcdef\", \"surface\": \"key_face:0\", \"label\": \"Mute\"}. A text is a string, a whole number is a number, and true or false is a boolean. `setting` is an object of the settings of an action, and `knob-colors` is an array of texts. `action` is the action as JSON. A value that you leave out is not in the request: for a value that a command sets, that removes it. `opencraft_help` gives the arguments and an example of each command."
            }
        },
        "required": ["command"],
        "additionalProperties": false
    })
}

fn read_tool() -> Value {
    json!({
        "name": READ_TOOL,
        "title": "Read the state of OpenCraft",
        "description": description(
            "Read the state of a running OpenCraft: the layout, the kinds of action, the icon names, the choices of a field, the profiles, the pages, the folders, the content, the device settings, the history, and the connection. A read changes nothing and does not move the device. Call opencraft_help with a command for its arguments and an example. OpenCraft must run, and the setting \"Allow control by other programs\" must be on.",
            true
        ),
        "inputSchema": schema(true),
        "annotations": {
            "readOnlyHint": true,
            "openWorldHint": false
        }
    })
}

fn change_tool() -> Value {
    json!({
        "name": CHANGE_TOOL,
        "title": "Change OpenCraft",
        "description": description(
            "Change a running OpenCraft: a surface, a binding, a control, a page, a folder, a profile, the device settings, or the history. The user can undo a change of the pages. A change with `page` makes that page the active page, thus the device shows it. Ask the user before `profile remove`: OpenCraft does not delete the profile, it moves it to the folder `removed`, and no undo brings it back. Call opencraft_help with a command for its arguments and an example. OpenCraft must run, and the setting \"Allow control by other programs\" must be on.",
            false
        ),
        "inputSchema": schema(false),
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": false,
            "idempotentHint": false,
            "openWorldHint": false
        }
    })
}

fn help_tool() -> Value {
    json!({
        "name": HELP_TOOL,
        "title": "Read the help of OpenCraft",
        "description": "Give the arguments and an example of one command, as `opencraft ctl <command> --help` does. With no command: the skill for an agent, with the terms, the rules, and the limits of OpenCraft. This tool needs no running OpenCraft.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The name of a command, for example `surface set-label` or `profile list`. Leave it out for the skill text."
                }
            },
            "additionalProperties": false
        },
        "annotations": {
            "readOnlyHint": true,
            "openWorldHint": false
        }
    })
}

/// The flags of `ctl` for the arguments of a tool call. The names are the
/// flags, and the values are JSON. The client of the command line makes the
/// rest: `parse::params` checks the kinds and makes the parameters of the
/// request.
pub(super) fn flags(
    command: &'static Command,
    args: &Map<String, Value>,
) -> Result<Vec<(&'static Arg, String)>, String> {
    let mut flags = Vec::new();
    for (key, value) in args {
        let Some(arg) = command.arguments().find(|arg| arg.flag == key.as_str()) else {
            let known: Vec<&str> = command.arguments().map(|arg| arg.flag).collect();
            return Err(if known.is_empty() {
                format!(
                    "`{}` has no arguments, and the argument `{key}` is not correct.",
                    command.name
                )
            } else {
                format!(
                    "`{}` has no argument `{key}`. The arguments are: {}.",
                    command.name,
                    known.join(", ")
                )
            });
        };
        match arg.kind {
            Kind::Setting => {
                let Value::Object(settings) = value else {
                    return Err(format!(
                        "The argument `{key}` must be an object of settings, as {{\"hotkey\": \"cmd+c\"}}."
                    ));
                };
                for (name, value) in settings {
                    flags.push((arg, format!("{name}={}", json_text(value))));
                }
            }
            _ => flags.push((arg, text(arg, value)?)),
        }
    }
    Ok(flags)
}

fn json_text(value: &Value) -> String {
    serde_json::to_string(value).expect("a value is JSON data")
}

/// The text of one flag, in the form that `ctl` reads.
fn text(arg: &Arg, value: &Value) -> Result<String, String> {
    let flag = arg.flag;
    match arg.kind {
        Kind::Text | Kind::Path => value
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("The argument `{flag}` must be a text.")),
        Kind::Surface | Kind::Control => value.as_str().map(str::to_string).ok_or_else(|| {
            format!(
                "The argument `{flag}` must be a text as `kind:number`, for example `key_face:1`."
            )
        }),
        Kind::Int => value
            .as_i64()
            .map(|number| number.to_string())
            .ok_or_else(|| format!("The argument `{flag}` must be a whole number.")),
        Kind::Bool => value
            .as_bool()
            .map(|value| value.to_string())
            .ok_or_else(|| format!("The argument `{flag}` must be true or false.")),
        Kind::Json => Ok(json_text(value)),
        Kind::List => match value {
            Value::Array(items) => {
                let items: Result<Vec<&str>, String> = items
                    .iter()
                    .map(|item| {
                        item.as_str().ok_or_else(|| {
                            format!("The argument `{flag}` must be an array of texts.")
                        })
                    })
                    .collect();
                Ok(items?.join(","))
            }
            Value::String(text) => Ok(text.clone()),
            _ => Err(format!("The argument `{flag}` must be an array of texts.")),
        },
        Kind::Setting => unreachable!("a setting is read in `flags`"),
    }
}
