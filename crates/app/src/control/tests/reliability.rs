//! Review 1 of the control socket: half close, a worker that ends, time
//! limits, the redaction, the wire forms, and the order of a list.

use super::*;
use crate::control::server::{Limits, Message, Server};

fn short_dir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap()
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(start.elapsed() < Duration::from_secs(5), "{what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_client_that_closes_its_write_side_still_gets_each_response() {
    let rig = Rig::new();
    rig.enable(true);
    let mut client = rig.client();
    // A long request line, more than one read, and the close before the
    // worker reads. On macOS poll gives POLLHUP for this socket.
    let long = "x".repeat(20_000);
    let lines = format!(
        "{}\n{}\n{}\n",
        json!({"id": 1, "method": "hello", "params": {"version": VERSION}}),
        json!({"id": 2, "method": "state", "params": {}}),
        json!({"id": 3, "method": "nothing", "params": {"long": long}}),
    );
    client.0.get_mut().write_all(lines.as_bytes()).unwrap();
    client
        .0
        .get_mut()
        .shutdown(std::net::Shutdown::Write)
        .unwrap();
    assert_eq!(client.read()["id"], 1);
    let state = client.read();
    assert_eq!(state["id"], 2);
    assert!(state["result"]["layout"].is_object());
    assert_eq!(client.read()["error"]["code"], "unknown_method");
    // After the last response the worker closes the connection.
    assert_eq!(client.0.read(&mut [0]).unwrap(), 0);
}

#[test]
fn a_worker_that_ends_removes_the_socket_and_the_next_enable_starts_a_new_one() {
    let rig = Rig::new();
    rig.enable(true);
    {
        let control = rig.control.borrow();
        let server = control.server.as_ref().unwrap();
        assert!(server.is_running());
        // The same end as an error of poll or a panic: the worker returns.
        server.post(Message::Stop);
        wait_for("the worker ends", || !server.is_running());
    }
    assert!(!rig.socket().exists());
    rig.enable(true);
    assert!(rig.socket().exists());
    let mut client = rig.client();
    client.hello();
    rig.enable(false);
    assert!(!rig.socket().exists());
}

#[test]
fn a_connection_with_no_hello_closes_and_the_17th_connection_gets_busy() {
    let dir = short_dir();
    let limits = Limits {
        hello: Duration::from_millis(300),
        ..Limits::default()
    };
    let server = Server::start_with(dir.path(), Feed::default(), None, limits).unwrap();
    let path = dir.path().join(FILE_NAME);
    let mut idle = Client::new(UnixStream::connect(&path).unwrap());
    // The idle connection is closed after the time limit.
    assert_eq!(idle.0.read(&mut [0]).unwrap(), 0);
    // 16 connections fill the places. The next gets the error `busy`.
    let mut clients: Vec<_> = (0..16)
        .map(|_| {
            let mut client = Client::new(UnixStream::connect(&path).unwrap());
            client.hello();
            client
        })
        .collect();
    let mut extra = Client::new(UnixStream::connect(&path).unwrap());
    assert_eq!(extra.read()["error"]["code"], "busy");
    assert_eq!(extra.0.read(&mut [0]).unwrap(), 0);
    // The other clients still work, and a place is free after a close.
    clients[0].request("state");
    clients.pop();
    wait_for("a free place", || {
        // A connection that gets `busy` is closed: the write can fail.
        let mut client = Client::new(UnixStream::connect(&path).unwrap());
        let hello = json!({"id": 1, "method": "hello", "params": {"version": VERSION}});
        let _ = writeln!(client.0.get_mut(), "{hello}");
        let mut line = String::new();
        let _ = client.0.read_line(&mut line);
        line.contains("\"result\"")
    });
    drop(server);
}

#[test]
fn a_client_that_does_not_read_its_response_is_closed() {
    let dir = short_dir();
    let limits = Limits {
        write: Duration::from_millis(300),
        ..Limits::default()
    };
    // A response of about 2 MB is more than the buffers of the socket hold.
    let feed = Feed::default();
    feed.observe(&e::Event::Kinds(
        (0..100)
            .map(|i| e::KindDescription {
                kind: format!("kind-{i}"),
                help: Some("long text ".repeat(2_000)),
                ..Default::default()
            })
            .collect(),
    ));
    let server = Server::start_with(dir.path(), feed, None, limits).unwrap();
    let path = dir.path().join(FILE_NAME);
    let mut client = Client::new(UnixStream::connect(&path).unwrap());
    client.hello();
    client.send(json!({"id": 2, "method": "kinds", "params": {}}));
    // Nobody reads. The worker closes the connection after the time limit;
    // the client then reads the data that the socket held, and the end.
    let mut sink = Vec::new();
    std::thread::sleep(Duration::from_millis(800));
    let result = client.0.read_to_end(&mut sink);
    if let Err(error) = result {
        // A reset is also a close. A time-out of the test is not.
        assert!(!matches!(
            error.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ));
    }
    assert!(sink.len() < 2_000_000, "the response was not cut");
    drop(server);
}

#[test]
fn hello_ignores_a_field_that_it_does_not_know_and_the_error_names_the_versions() {
    let rig = Rig::new();
    rig.enable(true);
    let mut client = rig.client();
    client.send(json!({"id": 1, "method": "hello",
        "params": {"version": VERSION + 1, "later": [1, 2]}}));
    let error = client.read()["error"].clone();
    assert_eq!(error["code"], "version_mismatch");
    let text = error["text"].as_str().unwrap();
    assert!(text.contains(&(VERSION + 1).to_string()) && text.contains(&VERSION.to_string()));
    let mut client = rig.client();
    client.send(json!({"id": 1, "method": "hello",
        "params": {"version": VERSION, "later": true}}));
    assert_eq!(client.read()["result"]["version"], VERSION);
}

#[test]
fn the_home_directory_is_hidden_at_a_path_boundary_with_any_case() {
    let rig = Rig::new();
    let home = std::env::var("HOME").unwrap();
    let setting = |text: String| {
        rig.control
            .borrow()
            .observe(&e::Event::Binding(e::BindingState {
                binding: e::BindingId {
                    page: 0,
                    folder: None,
                    control: e::ControlId::new(e::ControlKind::Key, 1),
                    gesture: e::GestureKind::Press,
                },
                action: Some(e::Action {
                    kind: "unknown".into(),
                    settings: e::Settings::from([("path".into(), e::Value::Text(text))]),
                }),
                available: false,
            }));
    };
    rig.enable(true);
    let mut client = rig.client();
    client.hello();
    let read = |client: &mut Client| {
        client.request("content")["bindings"][0]["action"]["settings"]["path"]
            .as_str()
            .unwrap()
            .to_string()
    };
    setting(format!("{}/x.sh", home.to_uppercase()));
    assert_eq!(read(&mut client), "~/x.sh");
    // A different directory that starts with the same text keeps its text.
    setting(format!("{home}2/a"));
    assert_eq!(read(&mut client), format!("{home}2/a"));
    // Text that is not a path stays as it is: a client can write it back.
    let script = format!("cat \"{home}/a\"");
    setting(script.clone());
    assert_eq!(read(&mut client), script);
}

#[test]
fn a_change_of_a_surface_keeps_its_place_in_the_list() {
    let mut rig = Rig::new();
    rig.enable(true);
    let mut client = rig.client();
    client.hello();
    // The order of the kinds is the order of the enum: key faces, bar panels,
    // and the background. The numbers go up in each kind.
    let order = |client: &mut Client| -> Vec<(u64, u64)> {
        client.request("content")["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                let kind = match s["id"]["kind"].as_str().unwrap() {
                    "key_face" => 0,
                    "bar_panel" => 1,
                    "background" => 2,
                    other => panic!("{other}"),
                };
                (kind, s["id"]["number"].as_u64().unwrap())
            })
            .collect()
    };
    let before = order(&mut client);
    assert!(before.len() > 2);
    rig.engine.command(e::Command::SetLabel {
        surface: e::SurfaceId::new(e::SurfaceKind::KeyFace, 1),
        label: Some("Changed".into()),
    });
    assert_eq!(order(&mut client), before);
    assert!(before.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn each_wire_enum_has_one_text_and_goes_to_the_engine_value_and_back() {
    /// The text on the wire, and the value after the way to the engine and
    /// back.
    fn check<W, E>(all: &[(E, &str)])
    where
        W: serde::Serialize
            + serde::de::DeserializeOwned
            + From<E>
            + Copy
            + PartialEq
            + std::fmt::Debug,
        E: From<W> + Copy + PartialEq + std::fmt::Debug,
    {
        for (engine, text) in all {
            let wire = W::from(*engine);
            assert_eq!(serde_json::to_value(wire).unwrap(), json!(text));
            assert_eq!(serde_json::from_value::<W>(json!(text)).unwrap(), wire);
            assert_eq!(E::from(wire), *engine);
        }
    }
    check::<Gesture, e::GestureKind>(&[
        (e::GestureKind::Press, "press"),
        (e::GestureKind::Release, "release"),
        (e::GestureKind::TurnLeft, "turn_left"),
        (e::GestureKind::TurnRight, "turn_right"),
        (e::GestureKind::Push, "push"),
        (e::GestureKind::Tap, "tap"),
        (e::GestureKind::SwipeLeft, "swipe_left"),
        (e::GestureKind::SwipeRight, "swipe_right"),
    ]);
    check::<SurfaceKind, e::SurfaceKind>(&[
        (e::SurfaceKind::KeyFace, "key_face"),
        (e::SurfaceKind::BarPanel, "bar_panel"),
        (e::SurfaceKind::Background, "background"),
    ]);
    check::<ControlKind, e::ControlKind>(&[
        (e::ControlKind::Key, "key"),
        (e::ControlKind::Knob, "knob"),
        (e::ControlKind::BarPanel, "bar_panel"),
        (e::ControlKind::TouchBar, "touch_bar"),
    ]);
    check::<LabelPosition, e::LabelPosition>(&[
        (e::LabelPosition::Top, "top"),
        (e::LabelPosition::Middle, "middle"),
        (e::LabelPosition::Bottom, "bottom"),
        (e::LabelPosition::Hidden, "hidden"),
    ]);
    check::<BackgroundFit, e::BackgroundFit>(&[
        (e::BackgroundFit::Fill, "fill"),
        (e::BackgroundFit::Fit, "fit"),
    ]);
    check::<KnobLightMode, e::KnobLightMode>(&[
        (e::KnobLightMode::Effect, "effect"),
        (e::KnobLightMode::Colors, "colors"),
        (e::KnobLightMode::Off, "off"),
    ]);
    // A text that is not a value gives an error.
    assert!(serde_json::from_value::<Gesture>(json!("wave")).is_err());
    assert!(serde_json::from_value::<Gesture>(json!("turn-left")).is_err());
    assert!(serde_json::from_value::<SurfaceKind>(json!("key face")).is_err());
}

#[test]
fn an_action_setting_has_the_wire_forms_text_number_flag_list_and_table() {
    let action = Action {
        kind: "k".into(),
        settings: [
            ("a".to_string(), json!("text")),
            ("b".to_string(), json!(-3)),
            ("c".to_string(), json!(true)),
            ("d".to_string(), json!(["x", 1])),
            ("e".to_string(), json!({"f": "g"})),
        ]
        .into(),
    };
    let engine: e::Action = action.clone().try_into().unwrap();
    assert_eq!(engine.settings["b"], e::Value::Integer(-3));
    assert_eq!(Action::from(&engine).settings, action.settings);
    for bad in [json!(null), json!(1.5), json!([null])] {
        let action = Action {
            kind: "k".into(),
            settings: [("a".to_string(), bad)].into(),
        };
        assert!(e::Action::try_from(action).is_err());
    }
}

// --- A change has no time limit, and a dropped reply is an error (review 2) ----

/// A server with the pending limit `pending` on the fake port of the rig.
fn serve_with_pending(rig: &Rig, pending: Duration) {
    let mut control = rig.control.borrow_mut();
    let limits = Limits {
        pending,
        ..Limits::default()
    };
    control.server = Some(
        Server::start_with(
            rig.dir.path(),
            control.feed.clone(),
            control.port.clone(),
            limits,
        )
        .unwrap(),
    );
}

/// A request for the label of key 1 on the active page.
fn label_request(client: &mut Client, id: u64) {
    let state = client.request("state");
    let profile = state["profiles"]["active"].clone();
    let page = state["pages"]["location"]["page"].clone();
    client.send(json!({
        "id": id, "method": "set_label",
        "params": {"target": {"profile": profile, "page": page, "folder": null},
                   "surface": {"kind": "key_face", "number": 1}, "label": "Late"}
    }));
}

#[test]
fn a_change_that_the_engine_obeys_late_has_its_response_and_no_timeout() {
    let rig = Rig::new();
    serve_with_pending(&rig, Duration::from_millis(150));
    let mut client = rig.client();
    client.hello();
    label_request(&mut client, 5);
    let (command, reply) = rig.commands.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(matches!(command, e::Command::AtPlace { .. }));
    // The engine is busy for more than the limit of a read.
    client
        .0
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(600)))
        .unwrap();
    let mut line = String::new();
    let error = client.0.read_line(&mut line).unwrap_err();
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
    // Then it obeys the command. The client gets the success, not an error
    // for a change that is done.
    client
        .0
        .get_ref()
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    reply.unwrap()(Ok(()));
    let response = client.read();
    assert_eq!(response["id"], 5);
    assert_eq!(
        response["result"],
        json!({"profile": null, "page": null, "folder": null})
    );
    // The next request works on the same connection.
    assert!(client.request("history").is_object());
}

#[test]
fn a_read_that_waits_for_a_handler_has_the_time_limit() {
    let rig = Rig::new();
    serve_with_pending(&rig, Duration::from_millis(150));
    rig.control
        .borrow()
        .observe(&e::Event::Kinds(vec![e::KindDescription {
            kind: "test".into(),
            fields: vec![e::Field {
                key: "choice".into(),
                label: "Choice".into(),
                input: e::InputType::RuntimeChoice,
                default: None,
            }],
            ..Default::default()
        }]));
    let mut client = rig.client();
    client.hello();
    client.send(json!({
        "id": 3, "method": "choices",
        "params": {"action": {"kind": "test", "settings": {}}, "field": "choice"}
    }));
    // Nobody answers.
    let response = client.read();
    assert_eq!(response["id"], 3);
    assert_eq!(response["error"]["code"], "timeout");
}

#[test]
fn an_engine_that_drops_the_reply_gives_the_error_and_not_a_wait() {
    let rig = Rig::new();
    rig.enable(true);
    let mut client = rig.client();
    client.hello();
    label_request(&mut client, 6);
    // The engine stops before it obeys the command: it drops the reply.
    let (_, reply) = rig.commands.recv_timeout(Duration::from_secs(3)).unwrap();
    drop(reply);
    let response = client.read();
    assert_eq!(response["id"], 6);
    assert_eq!(response["error"]["code"], "engine_problem");
    assert_eq!(response["error"]["text"], "The engine is not available.");
}

#[test]
fn the_cache_ignores_the_events_that_the_feed_skips() {
    let mut rig = Rig::new();
    rig.engine.poll(Duration::ZERO);
    let before = rig
        .control
        .borrow()
        .feed
        .read(|state| serde_json::to_value(state).unwrap());
    let ignored = [
        e::Event::Input(e::InputEvent {
            control: e::ControlId::new(e::ControlKind::Key, 1),
            gesture: e::Gesture::Press,
        }),
        e::Event::Counters(Default::default()),
        e::Event::Problem("A problem".into()),
        e::Event::ProfileImported {
            profile: "other".into(),
        },
    ];
    for event in &ignored {
        assert!(crate::control::state::ignored(event), "{event:?}");
        let mut state = rig.control.borrow().feed.read(|state| state.clone());
        state.apply(event);
        assert_eq!(serde_json::to_value(&state).unwrap(), before);
    }
    // An event that changes the cache is not ignored.
    assert!(!crate::control::state::ignored(&e::Event::Connected));
    assert!(!crate::control::state::ignored(&e::Event::Layout(
        std::sync::Arc::new(e::DeviceLayout::of_default_model())
    )));
}

#[test]
fn an_error_line_is_a_failure_for_each_result_type_and_null_is_a_success() {
    let error = r#"{"id":7,"error":{"code":"unavailable","text":"No layout."}}"#;
    let null = r#"{"id":7,"result":null}"#;
    // A result type that is an option read an error line as a success with no
    // value (review 2, M7).
    let response: Response<Option<Layout>> = serde_json::from_str(error).unwrap();
    assert_eq!(
        response.into_result().unwrap_err().code,
        ErrorCode::Unavailable
    );
    let response: Response<Option<Layout>> = serde_json::from_str(null).unwrap();
    assert!(response.into_result().unwrap().is_none());
    let response: Response<Value> = serde_json::from_str(error).unwrap();
    assert!(response.into_result().is_err());
    let response: Response<Changed> = serde_json::from_str(error).unwrap();
    assert!(response.into_result().is_err());
    let response: Response<Changed> =
        serde_json::from_str(r#"{"id":1,"result":{"profile":"p","page":null,"folder":null}}"#)
            .unwrap();
    assert_eq!(
        response.into_result().unwrap(),
        Changed {
            profile: Some("p".into()),
            ..Changed::default()
        }
    );
}
