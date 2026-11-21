//! The MCP server, over in-process pipes and against the real control server
//! with a fake engine port. No test opens a device.

use super::{execute, help, tools, Env, INSTRUCTIONS, LEGACY, MODERN, TTL_MS, VERSION_KEY};
use crate::{
    control::{
        protocol::{self, Request},
        Control, Feed, Port,
    },
    ctl::{self, commands::Class, commands::COMMANDS, parse, status},
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
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
            answer_timeout: super::ANSWER_TIMEOUT,
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
fn surface_changes_accept_the_on_argument_in_mcp() {
    for name in [
        "set-label",
        "set-label-style",
        "set-color",
        "set-image",
        "set-icon",
    ] {
        let command = ctl::commands::find(&format!("surface {name}")).unwrap();
        let args = json!({"profile": "p", "page": "0123456789abcdef", "surface": "key_face:1", "on": true});
        let flags = tools::flags(command, args.as_object().unwrap()).unwrap();
        let params = parse::params(command, &flags, None).unwrap();
        assert_eq!(params["on"], true);
        ctl::request(command.method, params).unwrap();
    }
}

#[test]
fn timed_key_gesture_arguments_make_set_and_clear_requests() {
    let command = ctl::commands::find("binding set").unwrap();
    let help = parse::command_help(command);
    for gesture in ["long_press", "double_press"] {
        assert!(help.contains(gesture));
        for action in [
            json!({"kind": "opencraft.hotkey", "settings": {}}),
            Value::Null,
        ] {
            let args = json!({"profile": "p", "page": "0123456789abcdef", "control": "key:1",
                "gesture": gesture, "action": action});
            let flags = tools::flags(command, args.as_object().unwrap()).unwrap();
            let params = parse::params(command, &flags, None).unwrap();
            assert_eq!(params["gesture"], gesture);
            assert_eq!(params["action"], action);
            ctl::request(command.method, params).unwrap();
        }
    }
}

#[test]
fn the_tools_cannot_drift_from_the_commands() {
    let list = tools::list();
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 4, "{list:?}");
    for name in [
        "opencraft_read",
        "opencraft_change",
        "opencraft_remove",
        "opencraft_help",
    ] {
        tool(&tools::list(), name);
    }
    let read = names(&tool(&tools::list(), "opencraft_read"));
    let change = names(&tool(&tools::list(), "opencraft_change"));
    let remove = names(&tool(&tools::list(), "opencraft_remove"));
    // Each command is in exactly one tool, and it is the tool of its class.
    for command in COMMANDS {
        let in_read = read.iter().any(|name| name == command.name);
        let in_change = change.iter().any(|name| name == command.name);
        let in_remove = remove.iter().any(|name| name == command.name);
        let count = [in_read, in_change, in_remove]
            .iter()
            .filter(|found| **found)
            .count();
        assert_eq!(
            count, 1,
            "{} is in {in_read} reads, {in_change} changes, and {in_remove} removes",
            command.name
        );
        assert_eq!(
            tools::tool_of(command.class),
            if in_read {
                "opencraft_read"
            } else if in_change {
                "opencraft_change"
            } else {
                "opencraft_remove"
            },
            "{} has the wrong class",
            command.name
        );
    }
    assert_eq!(read.len() + change.len() + remove.len(), COMMANDS.len());
    // The read tool holds exactly the read requests of the protocol. The
    // example of each command makes a request that the protocol accepts, thus
    // `Method::reads` gives the property of the request without a copy here.
    for command in COMMANDS {
        let request = example_request(command);
        assert_eq!(
            command.class == Class::Read,
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
    // Each description names each command of its tool. The help of a command
    // is in `opencraft_help`, thus the description stays short.
    for (name, group) in [
        ("opencraft_read", &read),
        ("opencraft_change", &change),
        ("opencraft_remove", &remove),
    ] {
        let description = tool(&tools::list(), name)["description"]
            .as_str()
            .unwrap()
            .to_string();
        for command in COMMANDS
            .iter()
            .filter(|c| group.contains(&c.name.to_string()))
        {
            assert!(
                description.contains(&format!("`{}`", command.name)),
                "the description of {name} has no name for {}",
                command.name
            );
        }
    }
    // The hints. `destructiveHint: false` means "only additive updates", thus
    // a tool that removes data or replaces a file has `true`
    // ([kb/mcp.md](../../../docs/kb/mcp.md)).
    let hints = tool(&tools::list(), "opencraft_read")["annotations"].clone();
    assert_eq!(hints["readOnlyHint"], true);
    assert_eq!(hints.get("idempotentHint"), None);
    let hints = tool(&tools::list(), "opencraft_change")["annotations"].clone();
    assert_eq!(hints["readOnlyHint"], false);
    assert_eq!(hints["destructiveHint"], false);
    let hints = tool(&tools::list(), "opencraft_remove")["annotations"].clone();
    assert_eq!(hints["readOnlyHint"], false);
    assert_eq!(hints["destructiveHint"], true);
    let hints = tool(&tools::list(), "opencraft_help")["annotations"].clone();
    assert_eq!(hints["readOnlyHint"], true);
    assert_eq!(hints.get("idempotentHint"), None);
}

#[test]
fn each_command_that_removes_or_writes_a_file_is_in_the_remove_tool() {
    let remove = names(&tool(&tools::list(), "opencraft_remove"));
    // The rule of the class: the name says that the command removes data, or
    // the command replaces a file of the user.
    for command in COMMANDS {
        let removes = command.name.ends_with(" remove")
            || command.name.ends_with(" clear")
            || command.name == "profile export"
            || command.name == "profile unlink-app";
        if !removes {
            continue;
        }
        assert_eq!(command.class, Class::Remove, "{}", command.name);
        assert!(
            remove.iter().any(|name| name == command.name),
            "{} is not in opencraft_remove",
            command.name
        );
    }
    // The commands that the owner named are in the tool.
    for name in [
        "profile remove",
        "page remove",
        "folder remove",
        "control clear",
        "profile export",
    ] {
        assert!(remove.iter().any(|command| command == name), "{name}");
    }
}

#[test]
fn the_instructions_and_the_descriptions_fit_the_limit_of_claude_code() {
    // Claude Code cuts server instructions and the description of a tool that
    // loads at the start after 4,096 characters. The complete skill is a tool
    // result, thus it has no such limit.
    // https://code.claude.com/docs/en/mcp#tool-search-for-mcp-server-authors
    let limit = 4_096;
    assert!(
        INSTRUCTIONS.chars().count() <= limit,
        "the instructions have {} characters",
        INSTRUCTIONS.chars().count()
    );
    for tool in tools::list().as_array().unwrap() {
        let description = tool["description"].as_str().unwrap();
        assert!(
            description.chars().count() <= limit,
            "the description of {} has {} characters",
            tool["name"],
            description.chars().count()
        );
    }
    // The instructions name the way to the complete skill, and the rules that
    // prevent damage.
    for phrase in ["opencraft_help", "profile remove", "page", "Ask the user"] {
        assert!(INSTRUCTIONS.contains(phrase), "{phrase} is not there");
    }
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
    // The instructions are the short text of this server, and not the skill.
    assert_eq!(result["instructions"], INSTRUCTIONS);
    assert_eq!(result.get("resultType"), None);

    // The notification has no answer. The next request proves it.
    session.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    assert_eq!(session.request(2, "ping", json!({}))["result"], json!({}));

    let answer = session.request(3, "tools/list", json!({}));
    let result = &answer["result"];
    assert_eq!(result.get("resultType"), None);
    let list = &result["tools"];
    assert_eq!(list.as_array().unwrap().len(), 4);
    for name in [
        "opencraft_read",
        "opencraft_change",
        "opencraft_remove",
        "opencraft_help",
    ] {
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
    assert_eq!(answer["result"]["tools"].as_array().unwrap().len(), 4);
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
    assert_eq!(result["instructions"], INSTRUCTIONS);
    assert_eq!(
        result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "opencraft"
    );
    assert_eq!(result["ttlMs"], TTL_MS);
    assert_eq!(result["cacheScope"], "public");

    let answer = session.request(2, "tools/list", json!({"_meta": meta()}));
    let result = &answer["result"];
    assert_eq!(result["resultType"], "complete");
    assert_eq!(result["tools"].as_array().unwrap().len(), 4);
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
    assert_eq!(
        answer["error"]["data"]["supported"],
        json!([MODERN, LEGACY])
    );
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
        read_timeout: None,
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
        answer_timeout: super::ANSWER_TIMEOUT,
    };
    let mut session = Session::new(Env {
        config_dir: env.config_dir.clone(),
        running: env.running,
        answer_timeout: env.answer_timeout,
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
    // `arguments` is an object in CallToolRequest, thus null is malformed.
    let answer = session.request(
        15,
        "tools/call",
        json!({"name": "opencraft_read", "arguments": null}),
    );
    assert_eq!(answer["error"]["code"], -32602);
    // A wrong argument of a tool is a tool error, and not a protocol error:
    // the model gets a text that it can correct. These are the three cases of
    // `docs/decisions/0040-control-socket.md`.
    let answer = session.call(10, "opencraft_read", json!({}));
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(
        text(&answer["result"]).contains("has no command"),
        "{answer}"
    );
    let answer = session.call(
        11,
        "opencraft_read",
        json!({"command": "state", "args": "profile=work"}),
    );
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(
        text(&answer["result"]).contains("must be an object"),
        "{answer}"
    );
    let answer = session.call(12, "opencraft_help", json!({"command": 5}));
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(
        text(&answer["result"]).contains("must be a text"),
        "{answer}"
    );
    // An argument that is not in `args` is a wrong argument. The server says
    // where the arguments of the command go, thus a read cannot use the wrong
    // profile.
    let answer = session.call(
        13,
        "opencraft_read",
        json!({"command": "page list", "profile": "work"}),
    );
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(text(&answer["result"]).contains("`profile`"), "{answer}");
    assert!(text(&answer["result"]).contains("in `args`"), "{answer}");
    let answer = session.call(
        14,
        "opencraft_help",
        json!({"command": "state", "extra": 1}),
    );
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(text(&answer["result"]).contains("`extra`"), "{answer}");
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
        answer_timeout: super::ANSWER_TIMEOUT,
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

#[test]
fn a_ping_before_the_handshake_has_an_answer() {
    // 2025-11-25 permits a ping before the answer to `initialize`.
    let rig = Rig::new();
    let mut session = Session::new(rig.env());
    let answer = session.request(1, "ping", json!({}));
    assert_eq!(answer["result"], json!({}));
    // The legacy handshake still works after the ping.
    let answer = session.request(
        2,
        "initialize",
        json!({"protocolVersion": LEGACY, "capabilities": {}}),
    );
    assert_eq!(answer["result"]["protocolVersion"], LEGACY);
    assert_eq!(session.finish(), status::DONE);
}

#[test]
fn a_response_and_a_malformed_message_follow_json_rpc() {
    let env = Env {
        config_dir: None,
        running: |_| false,
        answer_timeout: super::ANSWER_TIMEOUT,
    };
    // A response gets no answer. A message without "jsonrpc": "2.0", an id
    // that is not a whole number, and params that are not an object are
    // invalid. A line that is not UTF-8 is not JSON. A ping before the
    // handshake has an answer.
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"1.0\",\"id\":2,\"method\":\"tools/list\"}\n{\"jsonrpc\":\"2.0\",\"id\":3.5,\"method\":\"tools/list\"}\n{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/list\",\"params\":[]}\n\xff\xfe\n{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"ping\"}\n";
    let mut out = Vec::new();
    let mut err = Vec::new();
    let status = execute(&env, &[], &mut &input[..], &mut out, &mut err);
    assert_eq!(status, status::DONE);
    let lines: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 5, "{lines:?}");
    assert_eq!(lines[0]["id"], 2);
    assert_eq!(lines[0]["error"]["code"], -32600);
    assert_eq!(lines[1]["id"], 3.5);
    assert_eq!(lines[1]["error"]["code"], -32600);
    assert_eq!(lines[2]["id"], 4);
    assert_eq!(lines[2]["error"]["code"], -32600);
    assert_eq!(lines[3]["id"], Value::Null);
    assert_eq!(lines[3]["error"]["code"], -32700);
    assert_eq!(lines[4]["id"], 5);
    assert_eq!(lines[4]["result"], json!({}));
    // The response is for a person on the standard error.
    assert!(
        String::from_utf8(err).unwrap().contains("response"),
        "the server wrote no line about the response"
    );
}

#[test]
fn a_tool_call_that_gets_no_answer_stops_with_a_tool_error() {
    let dir = short_dir();
    let listener = UnixListener::bind(dir.path().join(protocol::FILE_NAME)).unwrap();
    let (done, wait) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        // Keep the connection open with no answer until the test is done.
        let _ = wait.recv();
    });
    let env = Env {
        config_dir: Some(dir.path().to_path_buf()),
        running: |_| true,
        answer_timeout: Duration::from_secs(1),
    };
    let mut session = Session::new(env);
    session.handshake();
    let answer = session.call(2, "opencraft_read", json!({"command": "state"}));
    let result = &answer["result"];
    assert!(is_error(result), "{result}");
    assert!(
        text(result).contains("did not answer within 1 second"),
        "{result}"
    );
    assert!(text(result).contains("read the state"), "{result}");
    assert_eq!(session.finish(), status::DONE);
    let _ = done.send(());
    server.join().unwrap();
}

#[test]
fn a_path_with_a_space_is_one_shell_word() {
    assert_eq!(
        super::shell_word("/opt/homebrew/bin/opencraft"),
        "/opt/homebrew/bin/opencraft"
    );
    assert_eq!(
        super::shell_word("/Users/a/My Apps/opencraft"),
        "'/Users/a/My Apps/opencraft'"
    );
    // A single quote in the path leaves the single quotes of the shell word.
    assert_eq!(
        super::shell_word("/Users/a/it's/opencraft"),
        "'/Users/a/it'\\''s/opencraft'"
    );
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
        answer_timeout: super::ANSWER_TIMEOUT,
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
        answer_timeout: super::ANSWER_TIMEOUT,
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
fn each_method_of_the_protocol_has_one_command_of_one_class() {
    // Each method of the protocol is the method of exactly one command, and
    // `hello` is neither: the client sends it by itself. The class of that
    // command is read or not, and the protocol says which.
    for method in protocol::METHODS
        .iter()
        .filter(|method| **method != "hello")
    {
        let commands: Vec<&ctl::commands::Command> = COMMANDS
            .iter()
            .filter(|command| command.method == *method)
            .collect();
        assert_eq!(commands.len(), 1, "{method}");
        let command = commands[0];
        assert_eq!(
            command.class == Class::Read,
            example_request(command).method.reads(),
            "{method}"
        );
    }
    assert!(!COMMANDS.iter().any(|command| command.method == "hello"));
}

#[test]
fn a_command_in_the_wrong_tool_is_a_tool_error_that_names_the_right_tool() {
    let rig = Rig::new();
    let mut session = Session::new(rig.env());
    session.handshake();
    // A change in the remove tool, a removal in the change tool, and a read
    // in each of the two.
    let answer = session.call(
        1,
        "opencraft_remove",
        json!({"command": "page add", "args": {"profile": "work"}}),
    );
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(
        text(&answer["result"]).contains("is a change. Use opencraft_change for it"),
        "{answer}"
    );
    let answer = session.call(
        2,
        "opencraft_change",
        json!({"command": "page remove", "args": {"profile": "work"}}),
    );
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(
        text(&answer["result"])
            .contains("removes data or writes a file. Use opencraft_remove for it"),
        "{answer}"
    );
    let answer = session.call(3, "opencraft_remove", json!({"command": "state"}));
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(
        text(&answer["result"]).contains("is a read. Use opencraft_read for it"),
        "{answer}"
    );
    let answer = session.call(
        4,
        "opencraft_read",
        json!({"command": "profile remove", "args": {"profile": "work"}}),
    );
    assert!(is_error(&answer["result"]), "{answer}");
    assert!(
        text(&answer["result"]).contains("Use opencraft_remove for it"),
        "{answer}"
    );
    assert_eq!(session.finish(), status::DONE);
}
