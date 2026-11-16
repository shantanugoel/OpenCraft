//! Review 1 of the control socket: half close, a worker that ends, time
//! limits, the redaction, the wire forms, and the order of a list.

use super::*;
use crate::control::server::{Limits, Message, Server};
use crate::control::state::{gesture_from_name, gesture_name};

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
        write: Duration::from_secs(10),
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
        let mut client = Client::new(UnixStream::connect(&path).unwrap());
        client.send(json!({"id": 1, "method": "hello", "params": {"version": VERSION}}));
        client.read().get("result").is_some()
    });
    drop(server);
}

#[test]
fn a_client_that_does_not_read_its_response_is_closed() {
    let dir = short_dir();
    let limits = Limits {
        hello: Duration::from_secs(10),
        write: Duration::from_millis(300),
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
    let order = |client: &mut Client| -> Vec<(String, u64)> {
        client.request("content")["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                (
                    s["id"]["kind"].as_str().unwrap().to_string(),
                    s["id"]["number"].as_u64().unwrap(),
                )
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
fn each_gesture_has_one_wire_text_and_the_texts_match_the_protocol_record() {
    let all = [
        (e::GestureKind::Press, "press"),
        (e::GestureKind::Release, "release"),
        (e::GestureKind::TurnLeft, "turn-left"),
        (e::GestureKind::TurnRight, "turn-right"),
        (e::GestureKind::Push, "push"),
        (e::GestureKind::Tap, "tap"),
        (e::GestureKind::SwipeLeft, "swipe-left"),
        (e::GestureKind::SwipeRight, "swipe-right"),
    ];
    for (gesture, name) in all {
        assert_eq!(gesture_name(gesture), name);
        assert_eq!(gesture_from_name(name), Some(gesture));
    }
    assert_eq!(gesture_from_name("wave"), None);
    assert_eq!(gesture_from_name("Press"), None);
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
