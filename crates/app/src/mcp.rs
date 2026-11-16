//! The MCP server `opencraft mcp` (decision 0040, section 6).
//!
//! The server is a second client of the control socket, as
//! `opencraft ctl` is ([`crate::ctl`]). It uses the commands, the
//! arguments, and the connection of the client. In this mode the process
//! takes no lock, starts no engine, opens no device, and starts no GUI
//! toolkit. `main` calls [`run`] before any other work. The standard output
//! carries the protocol only; the messages for a person go to the standard
//! error.
//!
//! The protocol is the Model Context Protocol over stdio: one JSON-RPC
//! message on one line, on the standard input and the standard output. The
//! server supports two revisions of the specification:
//!
//! - `2026-07-28`: every request carries its version in `_meta`, the result
//!   has a `resultType`, and `server/discover` names the versions.
//! - `2025-11-25`: the `initialize` handshake.
//!
//! A session is the format of the requests that the client sends. A request
//! with `_meta.io.modelcontextprotocol/protocolVersion` is of the first
//! kind; an `initialize` request, and the requests after it, are of the
//! second kind. There is no other session state.
//!
//! The server has three tools, and [`tools`] makes them from `COMMANDS`.
//! Each tool call opens one connection to the control socket, as the client
//! does: a new start of OpenCraft needs no new start of the server. When
//! OpenCraft does not run, or the setting "Allow control by other programs"
//! is off, the tool call is a tool error with the text of the client, and
//! the server continues.

#[cfg(test)]
mod tests;
mod tools;

use crate::ctl::{commands, parse, perform, skill, status, Answer, Context};
use serde_json::{json, Map, Value};
use std::{
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    time::Duration,
};

/// The revision of the specification with a version on each request.
const MODERN: &str = "2026-07-28";
/// The revision of the specification with the `initialize` handshake.
const LEGACY: &str = "2025-11-25";

/// The name of the server, and its version, as the initialize result and
/// `server/discover` give them.
const SERVER_NAME: &str = "opencraft";

/// The key of the protocol version in `_meta` of a request.
const VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";
/// The key of the capabilities of the client in `_meta` of a request.
const CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
/// The key of the name and the version of the server in `_meta` of a result.
const SERVER_KEY: &str = "io.modelcontextprotocol/serverInfo";

/// The tool list and the `server/discover` result do not change while the
/// process runs. A client can cache them for an hour.
const TTL_MS: u64 = 3_600_000;

/// The longest time that the server waits for an answer of OpenCraft. A client
/// of MCP cannot stop one request of a tool, and OpenCraft can hang.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(60);

/// The text of `instructions` of `initialize` and of `server/discover`. It is
/// short on purpose: Claude Code cuts a server instruction after 4,096
/// characters ([kb/mcp.md](../../../docs/kb/mcp.md)), and it must not
/// repeat the tool descriptions. The complete skill is the result of
/// `opencraft_help` with no command.
const INSTRUCTIONS: &str = concat!(
    "OpenCraft operates a StreamDock controller. `opencraft_read` and ",
    "`opencraft_change` read and change the OpenCraft that runs now: its ",
    "profiles, pages, folders, surfaces, bindings, actions, device settings, ",
    "and history. The user sees each change at once on the device and in the ",
    "window.\n",
    "\n",
    "Rules that prevent damage:\n",
    "- A change with `page` moves the device: it makes the profile active and ",
    "shows the page. When you are done, go home: `profile select`, ",
    "`page select`, and `folder open` for each open folder of your home.\n",
    "- Ask the user before `profile remove`. OpenCraft does not delete the ",
    "profile: it moves it to the folder `removed`, and no undo brings it back.\n",
    "- Read the identities from `profile list`, `page list`, and ",
    "`folder list`. Never guess them.\n",
    "- You cannot run an action. A person must push the key.\n",
    "- OpenCraft must run, and the setting \"Allow control by other programs\" ",
    "must be on.\n",
    "\n",
    "Call `opencraft_help` with no `command` first: it gives the complete ",
    "skill, with the rules, the terms, an example, and the commands. Call it ",
    "with a `command` for the arguments and an example of that command.",
);

// The error codes of JSON-RPC 2.0 and of the specification of 2026-07-28.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const UNSUPPORTED_VERSION: i64 = -32022;

/// What the server needs from its surroundings. A test gives its own.
pub struct Env {
    /// The directory of `control.sock` and `opencraft.lock`. None: OpenCraft
    /// cannot find the profiles directory.
    pub config_dir: Option<PathBuf>,
    /// True if an OpenCraft process holds the lock in `config_dir`.
    pub running: fn(&Path) -> bool,
    /// The longest time to wait for an answer of OpenCraft.
    pub answer_timeout: Duration,
}

/// The arguments of `opencraft mcp` that ask for a text, and not for the
/// server.
enum Given {
    Serve,
    Help,
    Config(String),
}

/// Run `opencraft mcp`. The result is the exit status.
pub fn run(arguments: Vec<std::ffi::OsString>, profiles_dir: Option<PathBuf>) -> i32 {
    let arguments: Result<Vec<String>, _> =
        arguments.into_iter().map(|a| a.into_string()).collect();
    let Ok(arguments) = arguments else {
        eprintln!("An argument is not text.");
        return status::ARGUMENTS;
    };
    let env = Env {
        config_dir: profiles_dir.as_deref().map(crate::settings::config_dir),
        running: |dir| crate::lock::holder(dir).is_some(),
        answer_timeout: ANSWER_TIMEOUT,
    };
    execute(
        &env,
        &arguments,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    )
}

/// Do the work of [`run`], with its own streams.
pub fn execute(
    env: &Env,
    arguments: &[String],
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match given(arguments) {
        Ok(Given::Serve) => serve(env, input, out, err),
        Ok(Given::Help) => {
            let _ = out.write_all(help().as_bytes());
            status::DONE
        }
        Ok(Given::Config(agent)) => match config(&agent) {
            Ok(text) => {
                let _ = out.write_all(text.as_bytes());
                status::DONE
            }
            Err(problem) => {
                let _ = writeln!(err, "{problem}\nSee `opencraft mcp --help`.");
                status::ARGUMENTS
            }
        },
        Err(problem) => {
            let _ = writeln!(err, "{problem}\nSee `opencraft mcp --help`.");
            status::ARGUMENTS
        }
    }
}

/// Read the arguments. Each flag takes `--flag VALUE` and `--flag=VALUE`.
fn given(arguments: &[String]) -> Result<Given, String> {
    match arguments {
        [] => Ok(Given::Serve),
        [flag] if flag == "--help" || flag == "-h" => Ok(Given::Help),
        [flag] if flag == "--config" => Ok(Given::Config(String::new())),
        [flag, agent] if flag == "--config" => Ok(Given::Config(agent.clone())),
        [flag] if flag.starts_with("--config=") => {
            Ok(Given::Config(flag["--config=".len()..].to_string()))
        }
        [flag, ..] => Err(format!(
            "The argument {flag} is not correct. `opencraft mcp` takes no argument, or --config, or --help."
        )),
    }
}

/// The text of `opencraft mcp --help`.
pub fn help() -> String {
    String::from(
        "opencraft mcp\n\n\
         Serve the tools of the Model Context Protocol on the standard input and the\n\
         standard output, for an agent. The server is a client of the control socket:\n\
         OpenCraft must run, and the setting \"Allow control by other programs\" must be\n\
         on. It takes no lock, starts no engine, and opens no device.\n\n\
         The server has the tools opencraft_read, opencraft_change, and opencraft_help.\n\
         The instructions of the server give the first rules; opencraft_help with no\n\
         command gives the complete skill for an agent.\n\n\
         Arguments:\n  \
         --config [AGENT]  Write the line that installs the server in an agent:\n  \
         \x20                 `claude` for Claude Code, `codex` for the Codex CLI, or no\n  \
         \x20                 agent for a JSON block that other clients use. It changes\n  \
         \x20                 no file.\n  \
         --help            Write this text.\n\n\
         Example:\n  opencraft mcp --config claude\n",
    )
}

/// The installation text for an agent, with the path of this executable.
fn config(agent: &str) -> Result<String, String> {
    let path = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| String::from("opencraft"));
    match agent {
        "claude" => Ok(format!(
            "claude mcp add --scope user opencraft -- {} mcp\n",
            shell_word(&path)
        )),
        "codex" => Ok(format!(
            "codex mcp add opencraft -- {} mcp\n",
            shell_word(&path)
        )),
        "" => Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "mcpServers": {"opencraft": {"command": path, "args": ["mcp"]}}
            }))
            .expect("a configuration is JSON data")
        )),
        other => Err(format!(
            "OpenCraft does not know the agent {other:?}. Give `claude`, `codex`, or no agent."
        )),
    }
}

/// A path as one word of a shell. A word with no character that the shell
/// reads stays as it is; every other word goes in single quotes. Thus a path
/// with a space, or with a `$`, is one word in the line of the agent.
fn shell_word(path: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "/._-+:,=@%".contains(c);
    if !path.is_empty() && path.chars().all(plain) {
        path.to_string()
    } else {
        format!("'{}'", path.replace('\'', "'\\''"))
    }
}

/// Serve the protocol until the standard input closes.
fn serve(env: &Env, input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut session = Session::default();
    loop {
        let mut buffer = Vec::new();
        match input.read_until(b'\n', &mut buffer) {
            Ok(0) => return status::DONE,
            Ok(_) => {}
            Err(error) => {
                let _ = writeln!(err, "OpenCraft cannot read the standard input: {error}.");
                return status::REFUSED;
            }
        }
        // A line that is not UTF-8 text is not JSON: it gets an error, and
        // the server continues.
        let Ok(line) = std::str::from_utf8(&buffer) else {
            let answer = error(
                None,
                PARSE_ERROR,
                "The message is not UTF-8 text, and a message of MCP is JSON text.",
            );
            if write_answer(out, &answer).is_err() {
                return status::REFUSED;
            }
            continue;
        };
        let line = line.trim_end_matches(['\n', '\r']);
        // No message is empty. A blank line is not a message, and it gets no
        // answer.
        if line.trim().is_empty() {
            continue;
        }
        let Some(answer) = session.message(line, env, err) else {
            continue;
        };
        if write_answer(out, &answer).is_err() {
            return status::REFUSED;
        }
    }
}

/// Write one answer on the standard output, and send it.
fn write_answer(out: &mut dyn Write, answer: &Value) -> io::Result<()> {
    let text = serde_json::to_string(answer).expect("an answer is JSON data");
    writeln!(out, "{text}")?;
    out.flush()
}

/// The format of the requests of one client.
#[derive(Default)]
struct Session {
    /// The client sent `initialize`: it uses the revision `2025-11-25`.
    legacy: bool,
}

impl Session {
    /// The answer to one line, or None for a notification or a response.
    fn message(&mut self, line: &str, env: &Env, err: &mut dyn Write) -> Option<Value> {
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(problem) => {
                return Some(error(
                    None,
                    PARSE_ERROR,
                    &format!("The message is not JSON: {problem}."),
                ))
            }
        };
        let Value::Object(message) = message else {
            return Some(error(
                None,
                INVALID_REQUEST,
                "A message must be a JSON object.",
            ));
        };
        let Some(method) = message.get("method") else {
            // A response. JSON-RPC has no answer to a response, and this
            // server sends no request. Thus it writes a line for a person and
            // no message for the client.
            let _ = writeln!(
                err,
                "OpenCraft received a response, but OpenCraft sent no request. OpenCraft \
                 sends no answer to a response."
            );
            return None;
        };
        if message.get("jsonrpc") != Some(&Value::String(String::from("2.0"))) {
            return Some(error(
                message.get("id").cloned(),
                INVALID_REQUEST,
                "A message of JSON-RPC 2.0 has \"jsonrpc\": \"2.0\".",
            ));
        }
        let Value::String(method) = method else {
            return Some(error(
                message.get("id").cloned(),
                INVALID_REQUEST,
                "The method of a request must be a text.",
            ));
        };
        let params = match message.get("params") {
            None => Value::Object(Map::new()),
            Some(params @ Value::Object(_)) => params.clone(),
            Some(_) => {
                return Some(error(
                    message.get("id").cloned(),
                    INVALID_REQUEST,
                    "The params of a request must be an object.",
                ))
            }
        };
        let Some(id) = message.get("id") else {
            // A notification. A receiver must not answer it.
            return None;
        };
        if !(id.is_string() || id.as_i64().is_some() || id.as_u64().is_some()) {
            return Some(error(
                Some(id.clone()),
                INVALID_REQUEST,
                "The id of a request must be a text or a whole number, not null and not a \
                 fraction.",
            ));
        }
        Some(self.request(method, &params, id.clone(), env))
    }

    fn request(&mut self, method: &str, params: &Value, id: Value, env: &Env) -> Value {
        // The handshake of 2025-11-25. A dual-era server selects the legacy
        // format from how the client opens.
        if method == "initialize" {
            self.legacy = true;
            return legacy_result(id, initialize(params));
        }
        let meta = params.get("_meta");
        let version = meta
            .and_then(|meta| meta.get(VERSION_KEY))
            .and_then(Value::as_str);
        if let Some(version) = version {
            if version != MODERN {
                return unsupported_version(id, version);
            }
            let capable = meta
                .and_then(|meta| meta.get(CAPABILITIES_KEY))
                .is_some_and(Value::is_object);
            if !capable {
                return error(
                    Some(id),
                    INVALID_PARAMS,
                    &format!(
                        "The request has no _meta.{CAPABILITIES_KEY} object. A request of \
                         {MODERN} must have _meta.{VERSION_KEY} and \
                         _meta.{CAPABILITIES_KEY}."
                    ),
                );
            }
            return match method {
                "server/discover" => modern_result(id, discover()),
                "tools/list" => modern_result(id, tool_list()),
                "tools/call" => match call(params, env) {
                    Ok(result) => modern_result(id, result),
                    Err((code, text)) => error(Some(id), code, &text),
                },
                other => error(
                    Some(id),
                    METHOD_NOT_FOUND,
                    &format!("`{other}` is not a method of MCP {MODERN} that OpenCraft has."),
                ),
            };
        }
        if self.legacy {
            return match method {
                "ping" => legacy_result(id, json!({})),
                // The result of 2025-11-25 has no `ttlMs` and no `cacheScope`.
                "tools/list" => legacy_result(id, json!({"tools": tools::list()})),
                "tools/call" => match call(params, env) {
                    Ok(result) => legacy_result(id, result),
                    Err((code, text)) => error(Some(id), code, &text),
                },
                other => error(
                    Some(id),
                    METHOD_NOT_FOUND,
                    &format!("`{other}` is not a method of MCP {LEGACY} that OpenCraft has."),
                ),
            };
        }
        if method == "ping" {
            // 2025-11-25 permits a ping before the answer to `initialize`.
            // This server supports that revision, thus it answers the ping.
            // A modern `ping` has a `_meta` version, and it is an error
            // above.
            return legacy_result(id, json!({}));
        }
        error(
            Some(id),
            INVALID_PARAMS,
            &format!(
                "The request has no _meta.{VERSION_KEY}, and the client did not send \
                 initialize. A client of {MODERN} gives _meta.{VERSION_KEY} and \
                 _meta.{CAPABILITIES_KEY} in each request. A client of {LEGACY} sends \
                 initialize first."
            ),
        )
    }
}

/// The result of `initialize`: the versions and the tools of this server, and
/// the first rules in `instructions`. The complete skill is the result of
/// `opencraft_help`.
fn initialize(_params: &Value) -> Value {
    // This server has one version of this kind. It names that version also
    // when the client names a different one: the client then stops, as the
    // lifecycle of 2025-11-25 says.
    json!({
        "protocolVersion": LEGACY,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION")},
        "instructions": INSTRUCTIONS
    })
}

/// The result of `server/discover`.
fn discover() -> Value {
    json!({
        "supportedVersions": [MODERN],
        "capabilities": {"tools": {}},
        "instructions": INSTRUCTIONS,
        "ttlMs": TTL_MS,
        "cacheScope": "public"
    })
}

/// The result of `tools/list`.
fn tool_list() -> Value {
    json!({
        "tools": tools::list(),
        "ttlMs": TTL_MS,
        "cacheScope": "public"
    })
}

/// Run one tool. The error is a protocol error for a malformed request or an
/// unknown tool; a wrong argument, and a problem of OpenCraft, is a tool
/// result with `isError`.
fn call(params: &Value, env: &Env) -> Result<Value, (i64, String)> {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Err((
            INVALID_PARAMS,
            String::from("The request of tools/call has no name of a tool."),
        ));
    };
    // `arguments` is not an object: the request does not agree with
    // CallToolRequest, thus it is a protocol error.
    let arguments = match params.get("arguments") {
        None => Map::new(),
        Some(Value::Object(arguments)) => arguments.clone(),
        Some(_) => {
            return Err((
                INVALID_PARAMS,
                String::from("The arguments of a tool must be an object."),
            ))
        }
    };
    match name {
        tools::HELP_TOOL => {
            // An argument that a tool does not have is a wrong argument of
            // the tool, thus a tool error that the model can correct.
            if let Some(key) = unknown_key(arguments.keys(), &["command"]) {
                return Ok(failed(&format!(
                    "The argument `{key}` of {} is not correct. {} has only \
                     the argument `command`.",
                    tools::HELP_TOOL,
                    tools::HELP_TOOL
                )));
            }
            let command = match arguments.get("command") {
                None => None,
                Some(Value::String(command)) => Some(command.as_str()),
                Some(_) => {
                    return Ok(failed(&format!(
                        "The argument `command` of {} must be a text.",
                        tools::HELP_TOOL
                    )))
                }
            };
            Ok(match command {
                None => done(skill::TEXT),
                Some(name) => match commands::find(name) {
                    Some(command) => done(&parse::command_help(command)),
                    None => failed(&format!(
                        "`{name}` is not a command. See `opencraft ctl --help`."
                    )),
                },
            })
        }
        tools::READ_TOOL | tools::CHANGE_TOOL => {
            if let Some(key) = unknown_key(arguments.keys(), &["command", "args"]) {
                return Ok(failed(&format!(
                    "The argument `{key}` of {name} is not correct. Put the arguments of the \
                     command in `args`."
                )));
            }
            let Some(command) = arguments.get("command") else {
                return Ok(failed(&format!("The request of {name} has no command.")));
            };
            let Value::String(command) = command else {
                return Ok(failed(&format!(
                    "The argument `command` of {name} must be a text."
                )));
            };
            let args = match arguments.get("args") {
                None => Map::new(),
                Some(Value::Object(args)) => args.clone(),
                Some(_) => {
                    return Ok(failed(&format!(
                        "The argument `args` of {name} must be an object."
                    )))
                }
            };
            let Some(command) = commands::find(command) else {
                return Ok(failed(&format!(
                    "`{command}` is not a command. See `opencraft ctl --help`."
                )));
            };
            let read = tools::is_read(command);
            if (name == tools::READ_TOOL) != read {
                let text = if read {
                    format!(
                        "`{}` is a read. Use {} for it.",
                        command.name,
                        tools::READ_TOOL
                    )
                } else {
                    format!(
                        "`{}` is a change. Use {} for it.",
                        command.name,
                        tools::CHANGE_TOOL
                    )
                };
                return Ok(failed(&text));
            }
            Ok(execute_command(env, command, &args))
        }
        other => Err((INVALID_PARAMS, format!("OpenCraft has no tool `{other}`."))),
    }
}

/// The first key of a tool call that the tool does not have, in alphabetical
/// order.
fn unknown_key<'a>(keys: impl Iterator<Item = &'a String>, allowed: &[&str]) -> Option<&'a str> {
    keys.map(String::as_str)
        .filter(|key| !allowed.contains(key))
        .min()
}

/// Run one command of `COMMANDS` against the control socket.
fn execute_command(
    env: &Env,
    command: &'static commands::Command,
    args: &Map<String, Value>,
) -> Value {
    let flags = match tools::flags(command, args) {
        Ok(flags) => flags,
        Err(problem) => {
            return failed(&format!(
                "{problem}\nSee `opencraft ctl {} --help`.",
                command.name
            ))
        }
    };
    let Some(config_dir) = env.config_dir.clone() else {
        return failed("OpenCraft cannot find the profiles directory.");
    };
    let context = Context {
        config_dir,
        running: env.running,
        pretty: false,
        read_timeout: Some(env.answer_timeout),
    };
    match perform(&context, command, &flags) {
        Answer::Result(result) => {
            done(&serde_json::to_string_pretty(&result).expect("JSON data has a text"))
        }
        Answer::Problem { text, .. } => failed(&text),
    }
}

/// A tool result with the text of a success.
fn done(text: &str) -> Value {
    result(text, false)
}

/// A tool result with the text of a problem.
fn failed(text: &str) -> Value {
    result(text, true)
}

fn result(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{"type": "text", "text": text}],
        "isError": is_error
    })
}

/// A result of a request of 2026-07-28: the type of the result and the name
/// of the server.
fn modern_result(id: Value, result: Value) -> Value {
    let mut result = match result {
        Value::Object(result) => result,
        other => {
            let mut map = Map::new();
            map.insert(String::from("value"), other);
            map
        }
    };
    result.insert(String::from("resultType"), json!("complete"));
    result.insert(
        String::from("_meta"),
        json!({SERVER_KEY: {"name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION")}}),
    );
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// A result of a request of 2025-11-25.
fn legacy_result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message}
    })
}

/// The error of a request with a version that this server does not have. The
/// list names each version that the server serves, as the example of the
/// specification does: the format of a request differs, but this server does
/// answer both.
fn unsupported_version(id: Value, requested: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": UNSUPPORTED_VERSION,
            "message": "Unsupported protocol version",
            "data": {"supported": [MODERN, LEGACY], "requested": requested}
        }
    })
}
