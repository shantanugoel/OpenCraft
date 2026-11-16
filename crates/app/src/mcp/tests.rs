//! The MCP server, over in-process pipes and against the real control server
//! with a fake engine port. No test opens a device.

use super::{execute, help, tools, Env, LEGACY, MODERN, TTL_MS, VERSION_KEY};
use crate::{
    control::{
        protocol::{self, Request},
        Control, Feed, Port,
    },
    ctl::{self, commands::COMMANDS, parse, status},
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    sync::Arc,
    time::Duration,
};

fn short_dir() -> tempfile::TempDir {
    // Short enough for the macOS limit of 104 bytes for a socket path.
    tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap()
}

/// A session of the server over a Unix socket pair: the test is the client.
struct Session {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
    thread: Option<std::thread::JoinHandle<i32>>,
}

impl Session {
    fn new(env: Env) -> Self {
        let (client, server) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let thread = std::thread::spawn(move || {
            let mut input = BufReader::new(server.try_clone().unwrap());
            let mut out = server;
            let mut err = Vec::new();
            let code = execute(&env, &[], &mut input, &mut out, &mut err);
            assert!(
                err.is_empty(),
                "the server wrote to the standard error: {}",
                String::from_utf8_lossy(&err)
            );
            code
        });
        Self {
            writer: client.try_clone().unwrap(),
            reader: BufReader::new(client),
            thread: Some(thread),
        }
    }

    /// The handshake of 2025-11-25, for a test that calls the tools.
    fn handshake(&mut self) {
        let answer = self.request(
            1,
            "initialize",
            json!({"protocolVersion": LEGACY, "capabilities": {}}),
        );
        assert_eq!(answer["result"]["protocolVersion"], LEGACY);
    }

    fn send(&mut self, message: Value) {
        writeln!(self.writer, "{message}").unwrap();
    }

    fn line(&mut self) -> Value {
        let mut line = String::new();
        assert!(
            self.reader.read_line(&mut line).unwrap() > 0,
            "the server stopped"
        );
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{e}: {line}"))
    }

    /// A request, and its whole answer.
    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        let answer = self.line();
        assert_eq!(answer["jsonrpc"], "2.0");
        assert_eq!(answer["id"], id);
        answer
    }

    fn call(&mut self, id: u64, tool: &str, arguments: Value) -> Value {
        self.request(
            id,
            "tools/call",
            json!({"name": tool, "arguments": arguments}),
        )
    }

    /// Close the input, and get the exit status of the server.
    fn finish(mut self) -> i32 {
        drop(self.writer);
        drop(self.reader);
        self.thread.take().unwrap().join().unwrap()
    }
}

/// The control server with a fake engine port, as the client tests use it.
struct Rig {
    dir: tempfile::TempDir,
    control: Control,
}

impl Rig {
    fn new() -> Self {
        let dir = short_dir();
        let feed = Feed::default();
        // The kinds of this version, as OpenCraft sends them.
        let kinds: Vec<opencraft_engine::KindDescription> = opencraft_actions::handlers()
            .iter()
            .flat_map(|handler| handler.kinds())
            .collect();
        feed.observe(&opencraft_engine::Event::Kinds(kinds));
        // Each change gets its answer at once.
        let answers = feed.clone();
        let port: Port = Arc::new(move |command, reply| {
            if let Some(reply) = reply {
                reply(Ok(()));
            }
            match command {
                opencraft_engine::Command::Choices { id, .. } => {
                    answers.observe(&opencraft_engine::Event::Choices {
                        id,
                        options: vec![opencraft_engine::ChoiceOption {
                            value: "up".into(),
                            label: "Up".into(),
                        }],
                    })
                }
                opencraft_engine::Command::ReadProfile { id, .. } => {
                    answers.observe(&opencraft_engine::Event::ProfileRead {
                        id,
                        profile: Err("The device model has no profile.".into()),
                    })
                }
                _ => {}
            }
        });
        let mut control = Control::with(feed, Some(port));
        control.set_enabled(true, dir.path()).unwrap();
        Self { dir, control }
    }

    fn env(&self) -> Env {
        Env {
            config_dir: Some(self.dir.path().to_path_buf()),
            running: |_| true,
        }
    }
}

/// The text of the only content block of a tool result.
fn text(result: &Value) -> String {
    let content = result["content"].as_array().unwrap_or_else(|| {
        panic!("the result has no content: {result}");
    });
    assert_eq!(content.len(), 1, "{result}");
    content[0]["text"].as_str().unwrap().to_string()
}

fn is_error(result: &Value) -> bool {
    result["isError"].as_bool().unwrap()
}

fn tool(tools: &Value, name: &str) -> Value {
    tools
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == name)
        .unwrap_or_else(|| panic!("there is no tool {name}"))
        .clone()
}

fn names(tool: &Value) -> Vec<String> {
    tool["inputSchema"]["properties"]["command"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap().to_string())
        .collect()
}

// The command-line client and the server have one source of truth.

/// The words of an example line, as a shell gives them.
fn words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in line.chars() {
        match c {
            '\'' => {
                quoted = !quoted;
                any = true;
            }
            ' ' if !quoted => {
                if any || !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                    any = false;
                }
            }
            c => word.push(c),
        }
    }
    if any || !word.is_empty() {
        words.push(word);
    }
    words
}

/// The request of the example of a command, with the checks of the client.
fn example_request(command: &'static ctl::commands::Command) -> Request {
    let device = json!({
        "brightness": 50, "screen_off_minutes": 0, "haptic": true,
        "knob_lights": {"mode": "effect", "brightness": 50, "colors": []}
    });
    let arguments: Vec<String> = words(command.example).into_iter().skip(2).collect();
    let parse::Parsed::Run { flags, .. } =
        parse::parse(&arguments).unwrap_or_else(|e| panic!("{}: {e}", command.example))
    else {
        panic!("{} is help", command.example);
    };
    let base = command.base.map(|_| device.clone());
    let params =
        parse::params(command, &flags, base).unwrap_or_else(|e| panic!("{}: {e}", command.example));
    ctl::request(command.method, params).unwrap_or_else(|e| panic!("{}: {e}", command.example))
}

#[test]
fn the_tools_cannot_drift_from_the_commands() {
    let list = tools::list();
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 3, "{list:?}");
    for name in ["opencraft_read", "opencraft_change", "opencraft_help"] {
        tool(&tools::list(), name);
    }
    let read = names(&tool(&tools::list(), "opencraft_read"));
    let change = names(&tool(&tools::list(), "opencraft_change"));
    // Each command is in exactly one of the two tools.
    for command in COMMANDS {
        let in_read = read.iter().any(|name| name == command.name);
        let in_change = change.iter().any(|name| name == command.name);
        assert!(
            in_read != in_change,
            "{} is in {in_read} reads and {in_change} changes",
            command.name
        );
    }
    assert_eq!(read.len() + change.len(), COMMANDS.len());
    // The read tool holds exactly the read requests of the protocol. The
    // example of each command makes a request that the protocol accepts, thus
    // `Method::reads` gives the property of the request without a copy here.
    for command in COMMANDS {
        let request = example_request(command);
        let in_read = read.iter().any(|name| name == command.name);
        assert_eq!(
            in_read,
            request.method.reads(),
            "{} is a {} in the protocol",
            command.name,
            if request.method.reads() {
                "read"
            } else {
                "change"
            }
        );
    }
    // Each description names each command of its tool with the help text.
    for (name, group) in [("opencraft_read", &read), ("opencraft_change", &change)] {
        let description = tool(&tools::list(), name)["description"]
            .as_str()
            .unwrap()
            .to_string();
        for command in COMMANDS
            .iter()
            .filter(|c| group.contains(&c.name.to_string()))
        {
            assert!(
                description.contains(&format!("`{}`: {}", command.name, command.help)),
                "the description of {name} has no line for {}",
                command.name
            );
        }
    }
    // The hints: a read is read-only. A change is not destructive: the user
    // can undo it, and the skill says to ask before `profile remove`.
    let hints = tool(&tools::list(), "opencraft_read")["annotations"].clone();
    assert_eq!(hints["readOnlyHint"], true);
    assert_eq!(hints["idempotentHint"], true);
    let hints = tool(&tools::list(), "opencraft_change")["annotations"].clone();
    assert_eq!(hints["readOnlyHint"], false);
    assert_eq!(hints["destructiveHint"], false);
    let hints = tool(&tools::list(), "opencraft_help")["annotations"].clone();
    assert_eq!(hints["readOnlyHint"], true);
}

// The revision 2025-11-25.

#[test]
fn a_legacy_session_does_the_handshake_and_lists_and_calls_the_tools() {
    let rig = Rig::new();
    let mut session = Session::new(rig.env());
    let answer = session.request(
        1,
        "initialize",
        json!({
            "protocolVersion": LEGACY,
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1"}
        }),
    );
    let result = &answer["result"];
    assert_eq!(result["protocolVersion"], LEGACY);
    assert_eq!(result["capabilities"]["tools"], json!({}));
    assert_eq!(result["serverInfo"]["name"], "opencraft");
    assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    // The instructions are the skill of this version.
    assert_eq!(result["instructions"], crate::ctl::skill::TEXT);
    assert_eq!(result.get("resultType"), None);

    // The notification has no answer. The next request proves it.
    session.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    assert_eq!(session.request(2, "ping", json!({}))["result"], json!({}));

    let answer = session.request(3, "tools/list", json!({}));
    let result = &answer["result"];
    assert_eq!(result.get("resultType"), None);
    let list = &result["tools"];
    assert_eq!(list.as_array().unwrap().len(), 3);
    for name in ["opencraft_read", "opencraft_change", "opencraft_help"] {
        tool(list, name);
    }
    // The caching hints are of 2026-07-28 and not of this revision.
    assert_eq!(result.get("ttlMs"), None);
    assert_eq!(result.get("cacheScope"), None);

    // A read against the real control server.
    let answer = session.call(4, "opencraft_read", json!({"command": "state"}));
    let result = &answer["result"];
    assert!(!is_error(result), "{result}");
    let state: Value = serde_json::from_str(&text(result)).unwrap();
    assert!(state["profiles"].is_object(), "{state}");
    assert!(!state["kinds"].as_array().unwrap().is_empty(), "{state}");

    // A change against the real control server with the fake engine port.
    let answer = session.call(
        5,
        "opencraft_change",
        json!({"command": "profile follow", "args": {"follow": true}}),
    );
    let result = &answer["result"];
    assert!(!is_error(result), "{result}");
    assert_eq!(
        serde_json::from_str::<Value>(&text(result)).unwrap(),
        json!({"profile": null, "page": null, "folder": null})
    );

    // The help of a command, and the skill with no command.
    let answer = session.call(6, "opencraft_help", json!({"command": "surface set-label"}));
    assert!(
        text(&answer["result"]).starts_with("opencraft ctl surface set-label"),
        "{answer}"
    );
    let answer = session.call(7, "opencraft_help", json!({}));
    assert_eq!(text(&answer["result"]), crate::ctl::skill::TEXT);

    assert_eq!(session.finish(), status::DONE);
}

#[test]
fn a_request_of_the_legacy_revision_before_the_handshake_is_a_protocol_error() {
    let rig = Rig::new();
    let mut session = Session::new(rig.env());
    let answer = session.request(1, "tools/list", json!({}));
    assert_eq!(answer["error"]["code"], -32602);
    assert!(
        answer["error"]["message"]
            .as_str()
            .unwrap()
            .contains("initialize"),
        "{answer}"
    );
    // The server continues after the error: the handshake makes it work.
    let answer = session.request(
        2,
        "initialize",
        json!({"protocolVersion": LEGACY, "capabilities": {}}),
    );
    assert_eq!(answer["result"]["protocolVersion"], LEGACY);
    let answer = session.request(3, "tools/list", json!({}));
    assert_eq!(answer["result"]["tools"].as_array().unwrap().len(), 3);
    assert_eq!(session.finish(), status::DONE);
}

// The revision 2026-07-28.

/// The `_meta` of a request of 2026-07-28.
fn meta() -> Value {
    json!({
        VERSION_KEY: MODERN,
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1"},
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

#[test]
fn a_modern_session_discovers_and_calls_the_tools() {
    let rig = Rig::new();
    let mut session = Session::new(rig.env());
    let answer = session.request(1, "server/discover", json!({"_meta": meta()}));
    let result = &answer["result"];
    assert_eq!(result["resultType"], "complete");
    assert_eq!(result["supportedVersions"], json!([MODERN]));
    assert_eq!(result["capabilities"]["tools"], json!({}));
    assert_eq!(result["instructions"], crate::ctl::skill::TEXT);
    assert_eq!(
        result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "opencraft"
    );
    assert_eq!(result["ttlMs"], TTL_MS);
    assert_eq!(result["cacheScope"], "public");

    let answer = session.request(2, "tools/list", json!({"_meta": meta()}));
    let result = &answer["result"];
    assert_eq!(result["resultType"], "complete");
    assert_eq!(result["tools"].as_array().unwrap().len(), 3);
    assert_eq!(result["ttlMs"], TTL_MS);
    assert_eq!(result["cacheScope"], "public");

    // A read against the real control server.
    let answer = session.request(
        3,
        "tools/call",
        json!({"name": "opencraft_read", "arguments": {"command": "state"}, "_meta": meta()}),
    );
    let result = &answer["result"];
    assert_eq!(result["resultType"], "complete");
    assert!(!is_error(result), "{result}");
    let state: Value = serde_json::from_str(&text(result)).unwrap();
    assert!(state["profiles"].is_object(), "{state}");

    // A change against the real control server.
    let answer = session.request(
        4,
        "tools/call",
        json!({
            "name": "opencraft_change",
            "arguments": {"command": "profile follow", "args": {"follow": false}},
            "_meta": meta()
        }),
    );
    let result = &answer["result"];
    assert_eq!(result["resultType"], "complete");
    assert!(!is_error(result), "{result}");

    // The removed methods and the unknown methods are not there.
    let answer = session.request(5, "ping", json!({"_meta": meta()}));
    assert_eq!(answer["error"]["code"], -32601);
    let answer = session.request(6, "resources/list", json!({"_meta": meta()}));
    assert_eq!(answer["error"]["code"], -32601);

    assert_eq!(session.finish(), status::DONE);
}

#[test]
fn a_modern_request_needs_its_version_and_the_capabilities() {
    let rig = Rig::new();
    let mut session = Session::new(rig.env());
    // No _meta at all, and no handshake.
    let answer = session.request(1, "tools/list", json!({}));
    assert_eq!(answer["error"]["code"], -32602);
    // The version, and no capabilities.
    let answer = session.request(2, "tools/list", json!({"_meta": {VERSION_KEY: MODERN}}));
    assert_eq!(answer["error"]["code"], -32602);
    // Another version.
    let answer = session.request(
        3,
        "tools/list",
        json!({"_meta": {VERSION_KEY: "1900-01-01", "io.modelcontextprotocol/clientCapabilities": {}}}),
    );
    assert_eq!(answer["error"]["code"], -32022);
    assert_eq!(answer["error"]["data"]["supported"], json!([MODERN]));
    assert_eq!(answer["error"]["data"]["requested"], "1900-01-01");
    // The server continues after each error.
    let answer = session.request(4, "tools/list", json!({"_meta": meta()}));
    assert!(answer["error"].is_null(), "{answer}");
    assert_eq!(session.finish(), status::DONE);
}

// The problems of OpenCraft and of the arguments are tool errors.

/// The problem text of the client for the same request.
fn client_problem(env: &Env, command: &'static ctl::commands::Command, args: &Value) -> String {
    let flags = tools::flags(command, args.as_object().unwrap()).unwrap();
    let context = ctl::Context {
        config_dir: env.config_dir.clone().unwrap(),
        running: env.running,
        pretty: false,
    };
    match ctl::perform(&context, command, &flags) {
        ctl::Answer::Result(result) => panic!("the client got a result: {result}"),
        ctl::Answer::Problem { text, .. } => text,
    }
}

#[test]
fn a_problem_of_opencraft_is_a_tool_error_with_the_text_of_the_client() {
    // OpenCraft does not run.
    let dir = short_dir();
    let env = Env {
        config_dir: Some(dir.path().to_path_buf()),
        running: |_| false,
    };
    let mut session = Session::new(Env {
        config_dir: env.config_dir.clone(),
        running: env.running,
    });
    session.handshake();
    let answer = session.call(2, "opencraft_read", json!({"command": "state"}));
    let result = &answer["result"];
    assert!(is_error(result), "{result}");
    let expected = client_problem(&env, ctl::commands::find("state").unwrap(), &json!({}));
    assert_eq!(text(result), expected);
    assert!(text(result).contains("does not run"), "{result}");
    // The server continues: the next call gives the same problem.
    let answer = session.call(3, "opencraft_read", json!({"command": "layout"}));
    assert!(is_error(&answer["result"]), "{answer}");
    assert_eq!(session.finish(), status::DONE);

    // OpenCraft runs, and the setting is off.
    let mut rig = Rig::new();
    rig.control.set_enabled(false, rig.dir.path()).unwrap();
    let mut session = Session::new(rig.env());
    session.handshake();
    let answer = session.call(2, "opencraft_read", json!({"command": "state"}));
    let result = &answer["result"];
    assert!(is_error(result), "{result}");
    assert!(
        text(result).contains("Allow control by other programs"),
        "{result}"
    );
    assert_eq!(session.finish(), status::DONE);
}

#[test]
fn a_refusal_and_a_wrong_argument_are_tool_errors_with_the_text_of_the_client() {
    let rig = Rig::new();
    let mut session = Session::new(rig.env());
    session.handshake();
    // A read of a place that is not there: OpenCraft refuses it.
    let args = json!({"profile": "nothing", "page": "0123456789abcdef"});
    let answer = session.call(
        1,
        "opencraft_read",
        json!({"command": "content", "args": args}),
    );
    let result = &answer["result"];
    assert!(is_error(result), "{result}");
    assert_eq!(
        text(result),
        client_problem(&rig.env(), ctl::commands::find("content").unwrap(), &args)
    );
    // A change with no profile: the argument is needed.
    let answer = session.call(2, "opencraft_change", json!({"command": "profile select"}));
    let result = &answer["result"];
    assert!(is_error(result), "{result}");
    assert_eq!(
        text(result),
        "The argument --profile is needed.\nSee `opencraft ctl profile select --help`."
    );
    // An argument that the command does not have.
    let answer = session.call(
        3,
        "opencraft_change",
        json!({"command": "profile select", "args": {"bogus": "x"}}),
    );
    let result = &answer["result"];
    assert!(is_error(result), "{result}");
    assert!(text(result).contains("has no argument `bogus`"), "{result}");
    // A value of the wrong kind.
    let answer = session.call(
        4,
        "opencraft_change",
        json!({"command": "profile follow", "args": {"follow": "yes"}}),
    );
    let result = &answer["result"];
    assert!(is_error(result), "{result}");
    assert!(text(result).contains("must be true or false"), "{result}");
    // A command of the other tool, and a command that does not exist.
    let answer = session.call(5, "opencraft_read", json!({"command": "profile select"}));
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(
        text(&answer["result"]).contains("is a change. Use opencraft_change"),
        "{answer}"
    );
    let answer = session.call(6, "opencraft_read", json!({"command": "nonsense"}));
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(
        text(&answer["result"]).contains("is not a command"),
        "{answer}"
    );
    // A tool that does not exist, and a call with no tool, are protocol
    // errors: the model cannot correct them.
    let answer = session.request(7, "tools/call", json!({"name": "nope", "arguments": {}}));
    assert_eq!(answer["error"]["code"], -32602);
    let answer = session.request(8, "tools/call", json!({"arguments": {}}));
    assert_eq!(answer["error"]["code"], -32602);
    // A change that does the work: `device set-settings` reads first, and the
    // cache of the fake engine has no device settings.
    let answer = session.call(
        9,
        "opencraft_change",
        json!({"command": "device set-settings", "args": {"brightness": 60}}),
    );
    assert!(is_error(&answer["result"]), "{answer}");
    assert_eq!(
        text(&answer["result"]),
        client_problem(
            &rig.env(),
            ctl::commands::find("device set-settings").unwrap(),
            &json!({"brightness": 60})
        )
    );
    assert_eq!(session.finish(), status::DONE);
}

#[test]
fn a_message_that_is_not_json_is_a_protocol_error_and_the_server_continues() {
    let rig = Rig::new();
    let mut session = Session::new(rig.env());
    session.writer.write_all(b"not JSON\n").unwrap();
    let answer = session.line();
    assert_eq!(answer["error"]["code"], -32700);
    assert_eq!(answer["id"], Value::Null);
    // An array is not a message.
    session.send(json!([1, 2]));
    assert_eq!(session.line()["error"]["code"], -32600);
    // An id that is null is not a request of MCP.
    session.send(json!({"jsonrpc": "2.0", "id": null, "method": "initialize"}));
    assert_eq!(session.line()["error"]["code"], -32600);
    let answer = session.request(
        1,
        "initialize",
        json!({"protocolVersion": LEGACY, "capabilities": {}}),
    );
    assert_eq!(answer["result"]["protocolVersion"], LEGACY);
    assert_eq!(session.finish(), status::DONE);
}

#[test]
fn the_server_starts_with_no_opencraft_and_takes_no_file() {
    let dir = short_dir();
    let env = Env {
        config_dir: None,
        running: |_| false,
    };
    let mut session = Session::new(env);
    session.handshake();
    let answer = session.call(2, "opencraft_read", json!({"command": "state"}));
    let result = &answer["result"];
    assert!(is_error(result), "{result}");
    assert_eq!(
        text(result),
        "OpenCraft cannot find the profiles directory."
    );
    assert_eq!(session.finish(), status::DONE);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

// The texts that need no server.

#[test]
fn the_help_names_the_tools_and_the_config_agents() {
    let text = help();
    assert!(text.contains("opencraft_read"), "{text}");
    assert!(text.contains("--config"), "{text}");
    assert!(text.contains("claude"), "{text}");
    assert!(text.contains("codex"), "{text}");
}

#[test]
fn the_config_text_names_this_executable_and_changes_no_file() {
    let dir = short_dir();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let env = Env {
        config_dir: None,
        running: |_| false,
    };
    for arguments in [
        vec!["--config", "claude"],
        vec!["--config", "codex"],
        vec!["--config"],
    ] {
        out.clear();
        err.clear();
        let arguments: Vec<String> = arguments.iter().map(|a| a.to_string()).collect();
        let status = execute(&env, &arguments, &mut std::io::empty(), &mut out, &mut err);
        assert_eq!(status, super::status::DONE, "{err:?}");
        let text = String::from_utf8(out.clone()).unwrap();
        let path = std::env::current_exe().unwrap().display().to_string();
        assert!(text.contains(&path), "{arguments:?}: {text}");
    }
    // The general form is a JSON block with the two programs of the client.
    out.clear();
    execute(
        &env,
        &[String::from("--config")],
        &mut std::io::empty(),
        &mut out,
        &mut err,
    );
    let block: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(block["mcpServers"]["opencraft"]["args"], json!(["mcp"]));
    // An agent that OpenCraft does not know.
    out.clear();
    err.clear();
    let status = execute(
        &env,
        &[String::from("--config"), String::from("nobody")],
        &mut std::io::empty(),
        &mut out,
        &mut err,
    );
    assert_eq!(status, status::ARGUMENTS);
    assert!(String::from_utf8(err).unwrap().contains("nobody"));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn the_wrong_arguments_stop_with_status_2() {
    let env = Env {
        config_dir: None,
        running: |_| false,
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let status = execute(
        &env,
        &[String::from("--install")],
        &mut std::io::empty(),
        &mut out,
        &mut err,
    );
    assert_eq!(status, super::status::ARGUMENTS);
    assert!(out.is_empty());
    assert!(String::from_utf8(err)
        .unwrap()
        .contains("opencraft mcp --help"));
}

#[test]
fn the_read_methods_of_the_protocol_are_the_reads_of_the_client() {
    // Each method of the protocol is a read or a change, and `hello` is
    // neither: the client sends it by itself.
    for method in protocol::METHODS
        .iter()
        .filter(|method| **method != "hello")
    {
        let reads = tools::READS.contains(method);
        let changes = COMMANDS
            .iter()
            .any(|command| command.method == *method && !tools::is_read(command));
        assert!(reads || changes, "{method} is neither");
        assert!(!(reads && changes), "{method} is both");
        assert_eq!(
            COMMANDS
                .iter()
                .filter(|command| command.method == *method)
                .count(),
            1,
            "{method}"
        );
    }
    for method in tools::READS {
        assert!(protocol::METHODS.contains(method), "{method}");
    }
    assert!(!tools::READS.contains(&"hello"));
    let mut sorted = tools::READS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), tools::READS.len());
}
