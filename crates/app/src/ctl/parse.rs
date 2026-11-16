//! The arguments of `opencraft ctl`: find the command, read the flags, and
//! make the `params` of the request. The text of the help is here too.

use super::commands::{find, Arg, Command, Kind, COMMANDS};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

/// What the arguments ask for.
pub enum Parsed {
    /// The help of the client (`None`) or of one command.
    Help(Option<&'static Command>),
    Run {
        command: &'static Command,
        flags: Vec<(&'static Arg, String)>,
    },
}

/// Find the command and read the flags. An error is a text for the person.
pub fn parse(arguments: &[String]) -> Result<Parsed, String> {
    let words: Vec<&str> = arguments
        .iter()
        .take_while(|argument| !argument.starts_with('-'))
        .map(String::as_str)
        .collect();
    let rest = &arguments[words.len()..];
    let wants_help = rest.iter().any(|a| a == "--help" || a == "-h");
    if words.is_empty() {
        return if rest.is_empty() {
            Err("No command.".into())
        } else if wants_help {
            Ok(Parsed::Help(None))
        } else {
            Err(format!("The argument {} must follow a command.", rest[0]))
        };
    }
    if words[0] == "help" && words.len() == 1 {
        return Ok(Parsed::Help(None));
    }
    let name = words.join(" ");
    let Some(command) = find(&name) else {
        return Err(unknown_command(&words));
    };
    if wants_help {
        return Ok(Parsed::Help(Some(command)));
    }
    let mut flags: Vec<(&'static Arg, String)> = Vec::new();
    let mut rest = rest.iter();
    while let Some(token) = rest.next() {
        let Some(token) = token.strip_prefix("--") else {
            return Err(format!(
                "The argument {token} is not correct for `{name}`. Write a flag as --name VALUE."
            ));
        };
        let (flag, inline) = match token.split_once('=') {
            Some((flag, value)) => (flag, Some(value.to_string())),
            None => (token, None),
        };
        let Some(arg) = command.arguments().find(|arg| arg.flag == flag) else {
            return Err(format!(
                "`{name}` has no argument --{flag}. See `opencraft ctl {name} --help`."
            ));
        };
        let value = match inline {
            Some(value) => value,
            None => rest
                .next()
                .cloned()
                .ok_or_else(|| format!("The argument --{flag} needs a value."))?,
        };
        if arg.kind != Kind::Setting && flags.iter().any(|(given, _)| given.flag == flag) {
            return Err(format!("The argument --{flag} is given twice."));
        }
        flags.push((arg, value));
    }
    Ok(Parsed::Run { command, flags })
}

fn unknown_command(words: &[&str]) -> String {
    let group = words[0];
    let names: Vec<&str> = COMMANDS
        .iter()
        .filter(|c| c.name.split(' ').next() == Some(group) && c.name.contains(' '))
        .map(|c| c.name.split(' ').nth(1).unwrap_or_default())
        .collect();
    if names.is_empty() || words.len() > 1 {
        format!(
            "`{}` is not a command. See `opencraft ctl --help`.",
            words.join(" ")
        )
    } else {
        format!("`{group}` needs one of: {}.", names.join(", "))
    }
}

/// The `params` of the request. `base` is the result of the read request of
/// the command, for a command that has one.
pub fn params(
    command: &Command,
    flags: &[(&'static Arg, String)],
    base: Option<Value>,
) -> Result<Value, String> {
    for arg in command.arguments().filter(|arg| arg.required) {
        if !flags.iter().any(|(given, _)| given.flag == arg.flag) {
            return Err(format!("The argument --{} is needed.", arg.flag));
        }
    }
    // `--action` and `--action-kind` both fill the same object.
    for (first, _) in flags {
        for (second, _) in flags {
            if second.path.len() > first.path.len()
                && second.path.starts_with(first.path)
                && second.path[first.path.len()..].starts_with('.')
            {
                return Err(format!(
                    "Use --{} or --{}, not both.",
                    first.flag, second.flag
                ));
            }
        }
    }
    let mut map = match base {
        Some(Value::Object(map)) => map,
        Some(_) => return Err("OpenCraft gave no value to start from.".into()),
        None => Map::new(),
    };
    for (arg, text) in flags {
        if arg.kind == Kind::Setting {
            let (key, value) = text.split_once('=').ok_or_else(|| {
                format!(
                    "The value of --{} must be KEY=VALUE, not {text:?}.",
                    arg.flag
                )
            })?;
            let value = serde_json::from_str(value).unwrap_or_else(|_| Value::from(value));
            set(&mut map, &format!("{}.{key}", arg.path), value)?;
        } else {
            set(&mut map, arg.path, convert(arg, text)?)?;
        }
    }
    for arg in command.arguments() {
        let Some(default) = arg.fill else { continue };
        let Some((parent, key)) = arg.path.rsplit_once('.') else {
            continue;
        };
        if flags.iter().any(|(given, _)| given.flag == arg.flag) {
            continue;
        }
        if let Some(parent) = object_at(&mut map, parent) {
            parent
                .entry(key)
                .or_insert(convert(arg, default).expect("a default is correct"));
        }
    }
    if let Some(finish) = command.finish {
        finish(&mut map)?;
    }
    Ok(Value::Object(map))
}

fn object_at<'a>(
    map: &'a mut Map<String, Value>,
    path: &str,
) -> Option<&'a mut Map<String, Value>> {
    let mut here = map;
    for name in path.split('.') {
        here = here.get_mut(name)?.as_object_mut()?;
    }
    Some(here)
}

fn set(map: &mut Map<String, Value>, path: &str, value: Value) -> Result<(), String> {
    let mut names: Vec<&str> = path.split('.').collect();
    let last = names.pop().expect("a path has a name");
    let mut here = map;
    for name in names {
        here = here
            .entry(name)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| format!("The arguments for {path} do not agree."))?;
    }
    here.insert(last.into(), value);
    Ok(())
}

fn convert(arg: &Arg, text: &str) -> Result<Value, String> {
    let flag = arg.flag;
    match arg.kind {
        Kind::Text => Ok(Value::from(text)),
        Kind::Int => text
            .parse::<i64>()
            .map(Value::from)
            .map_err(|_| format!("The value of --{flag} must be a whole number, not {text:?}.")),
        Kind::Bool => match text {
            "true" | "on" | "yes" => Ok(Value::Bool(true)),
            "false" | "off" | "no" => Ok(Value::Bool(false)),
            _ => Err(format!(
                "The value of --{flag} must be true or false, not {text:?}."
            )),
        },
        Kind::Json => serde_json::from_str(text)
            .map_err(|error| format!("The value of --{flag} is not JSON: {error}.")),
        Kind::Path => Ok(Value::from(absolute(text)?)),
        Kind::Surface | Kind::Control => {
            let (kind, number) = text.split_once(':').ok_or_else(|| {
                format!("The value of --{flag} must be kind:number, for example key_face:3.")
            })?;
            let number: u8 = number
                .parse()
                .map_err(|_| format!("The number in --{flag} must be from 0 to 255."))?;
            Ok(json!({"kind": kind, "number": number}))
        }
        Kind::List => Ok(Value::Array(
            text.split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(Value::from)
                .collect(),
        )),
        Kind::Setting => unreachable!("a setting is read in `params`"),
    }
}

/// A file input of a request is an absolute path. `~/` is the home
/// directory, and another relative path is relative to the current directory.
fn absolute(text: &str) -> Result<String, String> {
    let path = match text.strip_prefix("~/") {
        Some(rest) => std::env::home_dir()
            .ok_or("OpenCraft cannot find the home directory.")?
            .join(rest),
        None => PathBuf::from(text),
    };
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|error| format!("OpenCraft cannot read the current directory: {error}."))?
            .join(path)
    };
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| "The path is not text.".into())
}

/// The text of `opencraft ctl --help`.
pub fn help() -> String {
    let mut text = String::from(
        "opencraft ctl: control a running OpenCraft from the command line.\n\n\
         Usage: opencraft ctl <command> [--flag value]...\n\
         \x20      opencraft ctl <command> --help\n\n\
         OpenCraft must run, and the setting \"Allow control by other programs\" must be\n\
         on. The result goes to the standard output as JSON. A problem goes to the\n\
         standard error as text. A profile, a page, and a folder have an identity: read\n\
         it with `profile list`, `page list`, and `folder list`. A change gives the\n\
         identities of the things that it made. The commands and the output can change\n\
         in each release before version 1.0.\n\n\
         Commands:\n",
    );
    let width = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(0);
    for command in COMMANDS {
        let first = command.help.split(". ").next().unwrap_or(command.help);
        text.push_str(&format!(
            "  {:width$}  {}\n",
            command.name,
            first.trim_end_matches('.')
        ));
    }
    text.push_str(
        "\nExit status:\n\
         \x20 0  The command was done.\n\
         \x20 1  OpenCraft refused the command. The standard error has the problem.\n\
         \x20 2  The arguments are not correct.\n\
         \x20 3  OpenCraft does not run.\n\
         \x20 4  OpenCraft runs, and the setting \"Allow control by other programs\" is off.\n\
         \x20 5  The protocol version of this client is not the version of OpenCraft.\n",
    );
    text
}

/// The text of `opencraft ctl <command> --help`.
pub fn command_help(command: &Command) -> String {
    let mut text = format!("opencraft ctl {}\n\n{}\n\n", command.name, command.help);
    let flags: Vec<&Arg> = command.arguments().collect();
    if flags.is_empty() {
        text.push_str("This command has no arguments.\n");
    } else {
        text.push_str("Arguments:\n");
        for arg in flags {
            text.push_str(&format!(
                "  --{}  {}{}\n",
                arg.flag,
                if arg.required { "(needed) " } else { "" },
                arg.help
            ));
        }
    }
    text.push_str(&format!("\nExample:\n  {}\n", command.example));
    text
}
