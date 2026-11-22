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
        read_timeout: None,
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
fn each_surface_change_accepts_the_on_flag() {
    let mut count = 0;
    for command in COMMANDS
        .iter()
        .filter(|c| c.name.starts_with("surface set-"))
    {
        let line = format!("{} --on true", command.example);
        let parse::Parsed::Run { command, flags } = parse::parse(&arguments(&line)).unwrap() else {
            panic!("not a command")
        };
        let params = parse::params(command, &flags, None).unwrap();
        assert_eq!(params["on"], true);
        super::request(command.method, params).unwrap();
        count += 1;
    }
    assert_eq!(count, 6);
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
    // The font of a surface is a family, or the word "system".
    assert_eq!(
        params(
            "opencraft ctl surface set-label-style --profile p --page g --surface key_face:2 \
             --font Menlo",
            None
        )
        .unwrap()["style"]["font"],
        "Menlo"
    );
    assert_eq!(
        params(
            "opencraft ctl surface set-label-style --profile p --page g --surface key_face:2",
            None
        )
        .unwrap()["style"]
            .get("font"),
        None
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
    for gesture in ["long_press", "double_press"] {
        for action in ["--action-kind hotkey", "--action null"] {
            let request = params(&format!("opencraft ctl binding set --profile p --page g --control key:0 --gesture {gesture} {action}"), None).unwrap();
            assert_eq!(request["gesture"], gesture);
            assert_eq!(request["action"].is_null(), action == "--action null");
        }
    }
    // A text that is a number needs the form of a JSON string. The skill and
    // the help of --setting give this form (review 3, L1).
    let text = |line: &str| params(line, None).unwrap()["action"]["settings"]["text"].clone();
    assert_eq!(
        text(
            "opencraft ctl binding set --profile p --page g --control key:1 --gesture press \
             --action-kind opencraft.type-text --setting 'text=\"2026\"'"
        ),
        json!("2026")
    );
    // Without the quotation marks, the value is a number, and the kind
    // refuses it.
    assert_eq!(
        text(
            "opencraft ctl binding set --profile p --page g --control key:1 --gesture press \
             --action-kind opencraft.type-text --setting text=2026"
        ),
        json!(2026)
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
        // A command with a read first: the value is not a number.
        "opencraft ctl device set-settings --brightness x",
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
    // `help <command>` gives the help of the command, also with two words.
    for line in [
        "opencraft ctl help state",
        "opencraft ctl help profile add",
        "opencraft ctl help device set-settings",
    ] {
        let (status, out, err) = ctl(&context, line);
        assert_eq!(status, status::DONE, "{line}: {err}");
        assert!(out.starts_with("opencraft ctl "), "{out}");
        assert!(out.contains("Example:"), "{out}");
        assert!(err.is_empty());
    }
    let (status, out, err) = ctl(&context, "opencraft ctl help nothing");
    assert_eq!(status, status::ARGUMENTS);
    assert!(out.is_empty());
    assert!(err.contains("is not a command"), "{err}");
    // The version of the client needs no OpenCraft.
    for line in ["opencraft ctl --version", "opencraft ctl state --version"] {
        let (status, out, err) = ctl(&context, line);
        assert_eq!(status, status::DONE, "{line}");
        assert!(out.starts_with("opencraft "), "{out}");
        assert!(out.contains(env!("CARGO_PKG_VERSION")), "{out}");
        assert!(err.is_empty());
    }
    assert!(!dir.path().join(protocol::FILE_NAME).exists());
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
    // The socket file is there and an OpenCraft process holds the lock: the
    // worker ended, or the start did not remove the file. OpenCraft runs,
    // thus a start would only open its window.
    let (status1, out, err) = ctl(&context(dir.path(), true), "opencraft ctl state");
    assert_eq!(status1, status::REFUSED);
    assert!(out.is_empty());
    assert!(err.contains("does not answer"), "{err}");
    assert!(!err.contains("does not run"), "{err}");
    std::fs::remove_file(dir.path().join(protocol::FILE_NAME)).unwrap();
    // OpenCraft runs and has no socket file.
    let (status4, out, err) = ctl(&context(dir.path(), true), "opencraft ctl state");
    assert_eq!(status4, status::CONTROL_OFF);
    assert!(out.is_empty());
    assert!(err.contains("Allow control by other programs"), "{err}");
    // OpenCraft can be starting: the text says so.
    assert!(err.contains("starting"), "{err}");
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
    assert_eq!(seen[1]["params"], json!({"profile": null}));
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
    let feed = Feed::default();
    // The kinds of this version, as OpenCraft sends them: an example of a
    // read must work with them (the example of `choices` names a field that
    // has choices).
    let kinds: Vec<opencraft_engine::KindDescription> = opencraft_actions::handlers()
        .iter()
        .flat_map(|handler| handler.kinds())
        .collect();
    feed.observe(&opencraft_engine::Event::Kinds(kinds));
    // Each change gets its answer at once. A choice that depends on the
    // computer gets an answer as the engine gives it.
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
            // The read of a profile that is not active. The test has no such
            // profile.
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
    let context = context(dir.path(), true);

    let (status0, out, err) = ctl(&context, "opencraft ctl state");
    assert_eq!(status0, status::DONE, "{err}");
    let state: Value = serde_json::from_str(&out).unwrap();
    assert!(state.get("profiles").is_some());

    // Each read command gets an answer or a problem of OpenCraft, not a
    // problem of the client. The example of `choices` gets its choices.
    let reads = [
        "state",
        "layout",
        "kinds",
        "icons",
        "choices",
        "content",
        "device_settings",
        "history",
        "connection",
        "profiles",
        "pages",
        "folders",
    ];
    let mut seen = 0;
    for command in COMMANDS.iter().filter(|c| reads.contains(&c.method)) {
        seen += 1;
        let (status, out, err) = ctl(&context, command.example);
        assert!(
            status == status::DONE || status == status::REFUSED,
            "{}: {status} {err}",
            command.example
        );
        if command.method == "choices" {
            assert_eq!(status, status::DONE, "{}: {err}", command.example);
            let list: Value = serde_json::from_str(&out).unwrap();
            assert_eq!(list[0]["value"], "up", "{}", command.example);
        }
    }
    assert_eq!(seen, reads.len());

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
fn each_example_names_a_surface_and_a_control_of_the_real_layout() {
    // The numbers of the examples are those of a layout: a surface or a
    // control that the layout does not have makes an example that fails.
    let layout: protocol::Layout = (&opencraft_engine::DeviceLayout::of_default_model()).into();
    let mut surfaces = 0;
    let mut controls = 0;
    for command in COMMANDS {
        let parse::Parsed::Run { flags, .. } =
            parse::parse(&arguments(command.example)).unwrap_or_else(|e| panic!("{e}"))
        else {
            panic!("{} is help", command.example);
        };
        let params = parse::params(command, &flags, command.base.map(|_| json!({})))
            .unwrap_or_else(|e| panic!("{}: {e}", command.example));
        if let Some(id) = params.get("surface") {
            let id: protocol::SurfaceId =
                serde_json::from_value(id.clone()).expect("a surface is an id");
            assert!(
                layout.surfaces.iter().any(|s| s.id == id),
                "{}: the layout has no surface {id:?}",
                command.example
            );
            surfaces += 1;
        }
        if let Some(id) = params.get("control") {
            let id: protocol::ControlId =
                serde_json::from_value(id.clone()).expect("a control is an id");
            assert!(
                layout.controls.iter().any(|c| c.id == id),
                "{}: the layout has no control {id:?}",
                command.example
            );
            controls += 1;
        }
    }
    assert!(surfaces > 0 && controls > 0, "{surfaces} {controls}");
}

#[test]
fn the_choices_example_reads_a_field_that_has_choices() {
    // The examples of the help must succeed. A field of the type path has no
    // choices, and a request for them is a refusal.
    let choices = COMMANDS
        .iter()
        .find(|command| command.method == "choices")
        .unwrap();
    let parse::Parsed::Run { flags, .. } = parse::parse(&arguments(choices.example)).unwrap()
    else {
        panic!()
    };
    let params = parse::params(choices, &flags, None).unwrap();
    let kind = params["action"]["kind"].as_str().unwrap();
    let field = params["field"].as_str().unwrap();
    let kinds: Vec<protocol::Kind> = opencraft_actions::handlers()
        .iter()
        .flat_map(|handler| handler.kinds())
        .map(|kind| (&kind).into())
        .collect();
    let kind = kinds
        .iter()
        .find(|k| k.kind == kind)
        .unwrap_or_else(|| panic!("no kind {kind}"));
    let field = kind
        .fields
        .iter()
        .find(|f| f.key == field)
        .unwrap_or_else(|| panic!("no field {field}"));
    assert!(
        matches!(
            field.input,
            protocol::Input::Choice { .. }
                | protocol::Input::RuntimeChoice
                | protocol::Input::RuntimeChoiceList
        ),
        "{} has no choices",
        field.key
    );
}

#[test]
fn the_skill_answers_the_kinds_of_the_code_and_its_own_words() {
    // The words of the skill about choices agree with the code: only a field
    // with choices is a read of choices, and an application is a path.
    let text = skill::TEXT;
    assert!(!text.contains("applications that run now"), "{text}");
    assert!(text.contains("the sound devices"));
    assert!(text.contains("--setting application=/Applications/Safari.app"));
    // A kind that an accepted binding has is available: `available` is not
    // the test of a fault at run time. The status of the kind is (review 3,
    // M2).
    assert!(!text.contains("check that the binding has"), "{text}");
    assert!(text.contains("needs_user"));
    // A change of a profile is not a step of the history (review 3, M1).
    assert!(text.contains("No `profile` request has an undo"), "{text}");
    assert!(text.contains("moves it to the folder `removed`"), "{text}");
    // A text that is JSON needs the form of a JSON string (review 3, L1).
    assert!(text.contains("--setting 'text=\"2026\"'"), "{text}");
    // The names of the icons are in the build: the agent reads them (test 3,
    // finding 1).
    assert!(text.contains("opencraft ctl icons --search mic"), "{text}");
    // The places of the pages count from 1, as the page number of the action
    // (test 3, finding 2).
    assert!(text.contains("page 1"), "{text}");
    assert!(!text.contains("from 0)"), "{text}");
    // A new profile has one page (test 3, finding 3).
    assert!(text.contains("A new profile has one page"), "{text}");
    // The name of a step of the history is the name of the window (test 3,
    // finding 4).
    assert!(text.contains("the name that the window shows"), "{text}");
    // A field with no default has no value (test 3, finding 5).
    assert!(text.contains("`default: null`"), "{text}");
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

// The skill (decision 0040, section 5).

use super::skill;

/// The flags in a text, as `--name`.
fn flags_in(text: &str) -> Vec<String> {
    text.split("--")
        .skip(1)
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_ascii_lowercase() || *c == '-')
                .collect::<String>()
        })
        .filter(|flag| !flag.is_empty())
        .collect()
}

/// The rows of the table of the commands: the name, and the text of the flags.
fn skill_rows() -> Vec<(String, String)> {
    skill::TEXT
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            (cells[1].trim_matches('`').to_string(), cells[2].to_string())
        })
        .collect()
}

#[test]
fn the_skill_has_its_name_and_a_description() {
    let mut lines = skill::TEXT.lines();
    assert_eq!(lines.next(), Some("---"));
    assert_eq!(lines.next(), Some("name: opencraft"));
    let description = lines.next().unwrap();
    assert!(description.starts_with("description: "), "{description}");
    assert_eq!(lines.next(), Some("---"));
}

#[test]
fn the_table_of_the_skill_has_each_command_and_no_other() {
    let rows = skill_rows();
    for command in COMMANDS {
        let count = rows.iter().filter(|(name, _)| name == command.name).count();
        assert_eq!(count, 1, "the skill has {count} rows for {}", command.name);
    }
    for (name, flags) in &rows {
        let Some(command) = COMMANDS.iter().find(|c| c.name == name) else {
            panic!("the skill names the command `{name}`, which does not exist");
        };
        let named = flags_in(flags);
        for flag in &named {
            assert!(
                command.arguments().any(|a| a.flag == flag),
                "the skill gives `{name}` the flag --{flag}, which does not exist"
            );
        }
        // Each flag that the command needs is in the row. `target` means the
        // flags of a target.
        for arg in command.arguments().filter(|a| a.required) {
            let in_target =
                flags.contains("target") && ["profile", "page", "folder"].contains(&arg.flag);
            assert!(
                named.iter().any(|n| n == arg.flag) || in_target,
                "the skill row of `{name}` has no --{}",
                arg.flag
            );
        }
    }
}

#[test]
fn each_command_line_of_the_skill_makes_a_request_that_the_protocol_accepts() {
    let device = json!({
        "brightness": 50, "screen_off_minutes": 0, "haptic": true,
        "knob_lights": {"mode": "effect", "brightness": 50, "colors": []}
    });
    let mut count = 0;
    let mut fenced = false;
    for line in skill::TEXT.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if !fenced || !line.starts_with("opencraft ctl ") {
            continue;
        }
        count += 1;
        let parse::Parsed::Run { command, flags } =
            parse::parse(&arguments(line)).unwrap_or_else(|e| panic!("{line}: {e}"))
        else {
            panic!("{line} is help");
        };
        let base = command.base.map(|_| device.clone());
        let params = parse::params(command, &flags, base).unwrap_or_else(|e| panic!("{line}: {e}"));
        super::request(command.method, params).unwrap_or_else(|e| panic!("{line}: {e}"));
    }
    assert!(count >= 8, "the skill has {count} example lines");
}

#[test]
fn the_skill_sets_the_same_functions_on_all_three_knob_gestures() {
    let commands: Vec<_> = skill::TEXT
        .lines()
        .filter(|line| {
            line.starts_with("opencraft ctl binding set ")
                && line.contains("opencraft.knob-functions")
        })
        .map(|line| {
            let parse::Parsed::Run { command, flags } = parse::parse(&arguments(line)).unwrap()
            else {
                panic!()
            };
            parse::params(command, &flags, None).unwrap()
        })
        .collect();
    let gestures: Vec<_> = commands
        .iter()
        .map(|params| params["gesture"].as_str().unwrap())
        .collect();
    assert_eq!(gestures, ["turn_left", "turn_right", "push"]);
    for params in &commands {
        assert_eq!(params["control"], json!({"kind":"knob","number":1}));
        assert_eq!(params["action"], commands[0]["action"]);
        let functions = &params["action"]["settings"]["functions"];
        assert_eq!(functions[0]["left"]["settings"]["hotkey"], "cmd+c");
        assert_eq!(functions[0]["right"]["settings"]["hotkey"], "cmd+v");
    }
}

#[test]
fn the_skill_has_no_personal_data() {
    for word in ["/Users/", "/home/", "@", "shantanu", "goel", "http://"] {
        assert!(
            !skill::TEXT.to_lowercase().contains(&word.to_lowercase()),
            "the skill has {word}"
        );
    }
}

#[test]
fn skill_writes_the_text_and_needs_no_opencraft() {
    let dir = short_dir();
    // No socket, no lock: the client does not look for OpenCraft.
    let context = context(dir.path(), false);
    let (status, out, err) = ctl(&context, "opencraft ctl skill");
    assert_eq!((status, err.as_str()), (status::DONE, ""));
    assert_eq!(out, skill::TEXT);
    let (status, out, _) = ctl(&context, "opencraft ctl skill --help");
    assert_eq!(status, status::DONE);
    assert!(out.contains("--install"));
    let (status, _, err) = ctl(&context, "opencraft ctl skill --unknown");
    assert_eq!(status, status::ARGUMENTS);
    assert!(err.contains("skill --help"), "{err}");
}

#[test]
fn skill_install_writes_the_skill_directory() {
    let dir = short_dir();
    let target = dir.path().join("skills");
    let context = context(dir.path(), false);
    let line = format!("opencraft ctl skill --install {}", target.display());
    let (status, out, err) = ctl(&context, &line);
    assert_eq!((status, err.as_str()), (status::DONE, ""));
    let file = target.join("opencraft").join("SKILL.md");
    assert_eq!(out.trim_end(), file.display().to_string());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), skill::TEXT);
    // A second install replaces the file.
    std::fs::write(&file, "old").unwrap();
    let (status, _, _) = ctl(&context, &line);
    assert_eq!(status, status::DONE);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), skill::TEXT);
    // `--install=DIR` is correct, as each other flag of the client.
    let other = dir.path().join("other");
    let line = format!("opencraft ctl skill --install={}", other.display());
    let (status, out, err) = ctl(&context, &line);
    assert_eq!((status, err.as_str()), (status::DONE, ""));
    assert_eq!(
        std::fs::read_to_string(other.join("opencraft").join("SKILL.md")).unwrap(),
        skill::TEXT
    );
    assert_eq!(
        out.trim_end(),
        other
            .join("opencraft")
            .join("SKILL.md")
            .display()
            .to_string()
    );
    // A directory that cannot be made is a refusal.
    let blocked = format!("opencraft ctl skill --install {}", file.display());
    let (status, _, err) = ctl(&context, &blocked);
    assert_eq!(status, status::REFUSED);
    assert!(err.contains("cannot make"), "{err}");
}
