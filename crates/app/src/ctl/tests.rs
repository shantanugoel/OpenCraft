use super::{
    commands::{Kind, COMMANDS},
    execute, parse, status, Context,
};
use crate::control::{
    protocol::{self, METHODS},
    Control, Feed, Port,
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::Path,
    sync::Arc,
};

/// Split an example like a shell: spaces, and single quotes.
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

fn short_dir() -> tempfile::TempDir {
    // Short enough for the macOS limit of 104 bytes for a socket path.
    tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap()
}

fn context(dir: &Path, running: bool) -> Context {
    Context {
        config_dir: dir.to_path_buf(),
        running: if running { |_| true } else { |_| false },
        pretty: false,
    }
}

fn arguments(line: &str) -> Vec<String> {
    words(line).into_iter().skip(2).collect()
}

fn ctl(context: &Context, line: &str) -> (i32, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = execute(context, &arguments(line), &mut out, &mut err);
    (
        status,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn each_request_of_the_protocol_has_one_command() {
    for method in METHODS.iter().filter(|m| **m != "hello") {
        let count = COMMANDS.iter().filter(|c| c.method == *method).count();
        assert_eq!(count, 1, "{method} has {count} commands");
    }
    for command in COMMANDS {
        assert!(
            METHODS.contains(&command.method),
            "{} names the method {}, which does not exist",
            command.name,
            command.method
        );
        if let Some(base) = command.base {
            assert!(METHODS.contains(&base), "{base}");
        }
    }
}

#[test]
fn the_names_and_the_flags_are_unique_and_the_help_and_the_example_are_there() {
    let mut names = std::collections::BTreeSet::new();
    for command in COMMANDS {
        assert!(names.insert(command.name), "{} twice", command.name);
        assert!(command.name.split(' ').count() <= 2, "{}", command.name);
        assert!(
            command.help.ends_with('.') && command.help.len() > 10,
            "{}",
            command.name
        );
        assert!(
            command
                .example
                .starts_with(&format!("opencraft ctl {}", command.name)),
            "{}",
            command.example
        );
        let mut flags = std::collections::BTreeSet::new();
        for arg in command.arguments() {
            assert!(
                flags.insert(arg.flag),
                "{}: --{} twice",
                command.name,
                arg.flag
            );
            assert!(!arg.help.is_empty());
        }
    }
    // The first word of a command with two words is not a command.
    for command in COMMANDS.iter().filter(|c| c.name.contains(' ')) {
        assert!(!names.contains(command.name.split(' ').next().unwrap()));
    }
}

#[test]
fn each_example_makes_a_request_that_the_protocol_accepts() {
    // The start of a command that changes only some values.
    let device = json!({
        "brightness": 50, "screen_off_minutes": 0, "haptic": true,
        "knob_lights": {"mode": "effect", "brightness": 50, "colors": []}
    });
    for command in COMMANDS {
        let arguments = arguments(command.example);
        let parse::Parsed::Run {
            command: found,
            flags,
        } = parse::parse(&arguments).unwrap_or_else(|e| panic!("{}: {e}", command.example))
        else {
            panic!("{} is help", command.example);
        };
        assert_eq!(found.name, command.name);
        let base = command.base.map(|_| device.clone());
        let params = parse::params(command, &flags, base)
            .unwrap_or_else(|e| panic!("{}: {e}", command.example));
        super::request(command.method, params)
            .unwrap_or_else(|e| panic!("{}: {e}", command.example));
        // The help text names the example.
        assert!(parse::command_help(command).contains(command.example));
    }
    assert!(parse::help().lines().count() > COMMANDS.len());
}

#[test]
fn the_flags_make_the_params_of_the_protocol() {
    let params = |line: &str, base: Option<Value>| {
        let parse::Parsed::Run { command, flags } = parse::parse(&arguments(line)).unwrap() else {
            panic!()
        };
        parse::params(command, &flags, base)
    };
    // Defaults fill the style, and no value removes a label.
    assert_eq!(
        params(
            "opencraft ctl surface set-label-style --profile p --page g --surface key_face:2 --position top",
            None
        )
        .unwrap(),
        json!({
            "target": {"profile": "p", "page": "g"},
            "surface": {"kind": "key_face", "number": 2},
            "style": {"position": "top", "size": 20, "bold": true}
        })
    );
    assert_eq!(
        params(
            "opencraft ctl surface set-label --profile p --page g --surface key_face:2",
            None
        )
        .unwrap()
        .get("label"),
        None
    );
    // A setting is JSON when it can be, and text otherwise.
    let binding = params(
        "opencraft ctl binding set --profile p --page g --control key:0 --gesture press \
         --action-kind hotkey --setting key=cmd+c --setting repeat=3",
        None,
    )
    .unwrap();
    assert_eq!(
        binding["action"],
        json!({"kind": "hotkey", "settings": {"key": "cmd+c", "repeat": 3}})
    );
    // A device value that is not given stays.
    let device = json!({
        "brightness": 50, "screen_off_minutes": 3, "haptic": true,
        "knob_lights": {"mode": "effect", "brightness": 50, "colors": ["#fff"]}
    });
    let changed = params(
        "opencraft ctl device set-settings --brightness 10 --knob-colors #000,#111",
        Some(device),
    )
    .unwrap();
    assert_eq!(changed["brightness"], 10);
    assert_eq!(changed["screen_off_minutes"], 3);
    assert_eq!(changed["knob_lights"]["colors"], json!(["#000", "#111"]));
    // A background has its type.
    let image = params(
        "opencraft ctl page set-background --profile p --page g --image /tmp/a.png",
        None,
    )
    .unwrap();
    assert_eq!(
        image["background"],
        json!({"type": "image", "path": "/tmp/a.png", "fit": "fill"})
    );
    // A relative path becomes absolute.
    let relative = params(
        "opencraft ctl surface set-image --profile p --page g --surface key_face:0 --path a.png",
        None,
    )
    .unwrap();
    assert!(Path::new(relative["path"].as_str().unwrap()).is_absolute());
}

#[test]
fn arguments_that_are_not_correct_are_found_before_a_connection() {
    let dir = short_dir();
    let context = context(dir.path(), false);
    let bad = [
        "opencraft ctl",
        "opencraft ctl nothing",
        "opencraft ctl profile",
        "opencraft ctl profile nothing",
        "opencraft ctl profile add",
        "opencraft ctl profile add --name",
        "opencraft ctl profile add --name a --name b",
        "opencraft ctl profile add --name a --colour red",
        "opencraft ctl profile add --name a extra",
        "opencraft ctl page move --profile p --page g --to x",
        "opencraft ctl page set-background --profile p --page g --color red --image /a",
        "opencraft ctl binding set --profile p --page g --control key:0 --gesture press --action {} --action-kind x",
        // The protocol types find these.
        "opencraft ctl binding set --profile p --page g --control key:0 --gesture bad --action-kind x",
        "opencraft ctl surface set-icon --profile p --page g --surface wall:1 --icon-name x",
    ];
    for line in bad {
        let (status, out, err) = ctl(&context, line);
        assert_eq!(status, status::ARGUMENTS, "{line}: {err}");
        assert!(out.is_empty(), "{line}");
        assert!(
            err.contains("--help") || err.contains("not correct"),
            "{line}: {err}"
        );
    }
    // Nothing connected: no socket file was made.
    assert!(!dir.path().join(protocol::FILE_NAME).exists());
}

#[test]
fn help_goes_to_the_standard_output_with_status_0() {
    let dir = short_dir();
    let context = context(dir.path(), false);
    for line in [
        "opencraft ctl --help",
        "opencraft ctl help",
        "opencraft ctl profile add --help",
        "opencraft ctl state -h",
    ] {
        let (status, out, err) = ctl(&context, line);
        assert_eq!(status, status::DONE, "{line}");
        assert!(out.starts_with("opencraft ctl"), "{out}");
        assert!(err.is_empty());
    }
}

#[test]
fn status_3_when_nothing_runs_and_status_4_when_control_is_off() {
    let dir = short_dir();
    let (status3, out, err) = ctl(&context(dir.path(), false), "opencraft ctl state");
    assert_eq!(status3, status::NOT_RUNNING);
    assert!(out.is_empty());
    assert!(err.contains("does not run"), "{err}");
    // A socket file that nobody serves is not a running OpenCraft.
    drop(UnixListener::bind(dir.path().join(protocol::FILE_NAME)).unwrap());
    let (status3, _, _) = ctl(&context(dir.path(), false), "opencraft ctl state");
    assert_eq!(status3, status::NOT_RUNNING);
    std::fs::remove_file(dir.path().join(protocol::FILE_NAME)).unwrap();
    // OpenCraft runs and has no socket file.
    let (status4, out, err) = ctl(&context(dir.path(), true), "opencraft ctl state");
    assert_eq!(status4, status::CONTROL_OFF);
    assert!(out.is_empty());
    assert!(err.contains("Allow control by other programs"), "{err}");
    // A command with a read first also gets the status.
    let (status4, _, _) = ctl(
        &context(dir.path(), true),
        "opencraft ctl device set-settings --brightness 5",
    );
    assert_eq!(status4, status::CONTROL_OFF);
}

/// A server that answers `hello` with `version`, and each other request with
/// `answer`.
fn fake_server(dir: &Path, version: u32, answer: Value) -> std::thread::JoinHandle<Vec<Value>> {
    let listener = UnixListener::bind(dir.join(protocol::FILE_NAME)).unwrap();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut seen = Vec::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                return seen;
            }
            let request: Value = serde_json::from_str(&line).unwrap();
            let id = request["id"].clone();
            let hello = request["method"] == "hello";
            seen.push(request);
            let response = if hello {
                if version != protocol::VERSION {
                    json!({"id": id, "error": {"code": "version_mismatch", "text": "The versions are not the same."}})
                } else {
                    json!({"id": id, "result": {"version": version, "app_version": "9.9.9"}})
                }
            } else {
                let mut response = answer.clone();
                response["id"] = id;
                response
            };
            writeln!(reader.get_mut(), "{response}").unwrap();
            if hello && version != protocol::VERSION {
                return seen;
            }
        }
    })
}

#[test]
fn status_5_when_the_protocol_versions_differ() {
    let dir = short_dir();
    let server = fake_server(dir.path(), 999, json!({}));
    let (status5, out, err) = ctl(&context(dir.path(), true), "opencraft ctl state");
    assert_eq!(status5, status::VERSION);
    assert!(out.is_empty());
    assert!(err.contains("versions"), "{err}");
    // The client stopped after `hello`.
    let seen = server.join().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0]["params"]["version"], protocol::VERSION);
}

#[test]
fn a_result_goes_to_the_output_and_an_error_to_the_standard_error() {
    let dir = short_dir();
    let server = fake_server(
        dir.path(),
        protocol::VERSION,
        json!({"result": {"a": [1, 2]}}),
    );
    let (status0, out, err) = ctl(&context(dir.path(), true), "opencraft ctl history");
    assert_eq!(status0, status::DONE);
    assert_eq!(
        serde_json::from_str::<Value>(&out).unwrap(),
        json!({"a": [1, 2]})
    );
    assert!(err.is_empty());
    let seen = server.join().unwrap();
    assert_eq!(seen[1]["method"], "history");
    assert_eq!(seen[1]["params"], json!({}));
}

#[test]
fn a_refusal_is_status_1_with_the_problem_text() {
    let dir = short_dir();
    let server = fake_server(
        dir.path(),
        protocol::VERSION,
        json!({"error": {"code": "engine_problem", "text": "The page is not there."}}),
    );
    let (status1, out, err) = ctl(
        &context(dir.path(), true),
        "opencraft ctl page remove --profile p --page 0123456789abcdef",
    );
    assert_eq!(status1, status::REFUSED);
    assert!(out.is_empty());
    assert_eq!(err.trim(), "The page is not there.");
    let seen = server.join().unwrap();
    assert_eq!(seen[1]["method"], "remove_page");
    assert_eq!(
        seen[1]["params"]["target"],
        json!({"profile": "p", "page": "0123456789abcdef"})
    );
}

#[test]
fn a_closed_connection_is_status_1() {
    let dir = short_dir();
    let listener = UnixListener::bind(dir.path().join(protocol::FILE_NAME)).unwrap();
    let thread = std::thread::spawn(move || drop(listener.accept().unwrap()));
    let (status1, _, err) = ctl(&context(dir.path(), true), "opencraft ctl state");
    thread.join().unwrap();
    assert_eq!(status1, status::REFUSED);
    assert!(err.contains("closed the connection"), "{err}");
}

#[test]
fn the_client_works_with_the_real_server() {
    let dir = short_dir();
    // Each change gets its answer at once.
    let port: Port = Arc::new(|_, reply| {
        if let Some(reply) = reply {
            reply(Ok(()));
        }
    });
    let mut control = Control::with(Feed::default(), Some(port));
    control.set_enabled(true, dir.path()).unwrap();
    let context = context(dir.path(), true);

    let (status0, out, err) = ctl(&context, "opencraft ctl state");
    assert_eq!(status0, status::DONE, "{err}");
    let state: Value = serde_json::from_str(&out).unwrap();
    assert!(state.get("profiles").is_some());

    // Each read command gets an answer or a problem of OpenCraft, not a
    // problem of the client.
    for command in COMMANDS.iter().filter(|c| c.args.is_empty()) {
        let (status, _, err) = ctl(&context, command.example);
        assert!(
            status == status::DONE || status == status::REFUSED,
            "{}: {status} {err}",
            command.example
        );
    }

    // A change for a place that is not there is refused.
    let (status1, out, err) = ctl(
        &context,
        "opencraft ctl page rename --profile nothing --page 0123456789abcdef --name x",
    );
    assert_eq!(status1, status::REFUSED, "{out} {err}");
    assert!(!err.is_empty());

    // The two kinds that have a value of JSON get the same result as the
    // protocol gives.
    let (status0, out, _) = ctl(&context, "opencraft ctl profile list");
    assert_eq!(status0, status::DONE);
    assert!(serde_json::from_str::<Value>(&out)
        .unwrap()
        .get("profiles")
        .is_some());

    // Control off: the socket goes, and the client sees status 4.
    control.set_enabled(false, dir.path()).unwrap();
    let (status4, _, _) = ctl(&context, "opencraft ctl state");
    assert_eq!(status4, status::CONTROL_OFF);
}

#[test]
fn the_kinds_of_value_have_a_flag_that_uses_them() {
    // A kind that no command uses is dead code.
    for kind in [
        Kind::Text,
        Kind::Int,
        Kind::Bool,
        Kind::Json,
        Kind::Path,
        Kind::Surface,
        Kind::Control,
        Kind::List,
        Kind::Setting,
    ] {
        assert!(
            COMMANDS
                .iter()
                .flat_map(|c| c.arguments())
                .any(|a| a.kind == kind),
            "{kind:?}"
        );
    }
}
