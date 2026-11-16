#![cfg(unix)]

use super::{protocol::*, Control, Feed, Port, Reply};
use opencraft_core::{device::DESCRIPTORS, Connector, Device, FakeTransport, Link};
use opencraft_engine as e;
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    io::{BufRead, BufReader, Read, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    rc::Rc,
    time::{Duration, Instant},
};

struct FakeConnector(
    Option<FakeTransport>,
    &'static opencraft_core::device::Descriptor,
);
impl Connector for FakeConnector {
    type Transport = FakeTransport;
    fn connect(&mut self) -> opencraft_core::Result<Option<Device<FakeTransport>>> {
        Ok(self.0.take().map(|t| Device::new(t, self.1)))
    }
}

type Sink = Box<dyn FnMut(e::Event)>;
type Sent = std::sync::mpsc::Receiver<(e::Command, Option<Reply>)>;
struct Rig {
    control: Rc<RefCell<Control>>,
    commands: Sent,
    engine: e::Engine<FakeConnector, Sink>,
    dir: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        Self::with_device(&DESCRIPTORS[0])
    }

    /// The engine starts with the first descriptor. The device that connects
    /// has the descriptor `device`.
    fn with_device(device: &'static opencraft_core::device::Descriptor) -> Self {
        // Short enough for the macOS sockaddr_un limit of 104 bytes.
        let dir = tempfile::Builder::new()
            .prefix("oc")
            .tempdir_in("/tmp")
            .unwrap();
        // The commands of the control module go to the test, which answers
        // them as the engine would.
        let (sent, commands) = std::sync::mpsc::channel();
        let sent = std::sync::Mutex::new(sent);
        let port: Port = std::sync::Arc::new(move |command, reply| {
            let _ = sent.lock().unwrap().send((command, reply));
        });
        let control = Rc::new(RefCell::new(Control::with(Feed::default(), Some(port))));
        let sink = control.clone();
        let sink: Sink = Box::new(move |event| sink.borrow().observe(&event));
        let mut engine = e::Engine::new(
            Link::new(FakeConnector(Some(FakeTransport::new()), device)),
            e::ProfilesDir::new(dir.path().join("profiles")),
            &DESCRIPTORS[0],
            sink,
        );
        engine.poll(Duration::ZERO);
        Self {
            control,
            commands,
            engine,
            dir,
        }
    }

    fn enable(&self, enabled: bool) {
        self.control
            .borrow_mut()
            .set_enabled(enabled, self.dir.path())
            .unwrap();
    }

    fn socket(&self) -> std::path::PathBuf {
        self.dir.path().join(FILE_NAME)
    }
    fn client(&self) -> Client {
        Client::new(UnixStream::connect(self.socket()).unwrap())
    }
}

struct Client(BufReader<UnixStream>);
impl Client {
    fn new(socket: UnixStream) -> Self {
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        Self(BufReader::new(socket))
    }
    fn send(&mut self, value: Value) {
        writeln!(self.0.get_mut(), "{value}").unwrap();
    }
    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.0.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
    fn request(&mut self, method: &str) -> Value {
        self.send(json!({"id": 7, "method": method, "params": {}}));
        let reply = self.read();
        assert_eq!(reply["id"], 7);
        assert!(reply.get("error").is_none(), "{reply}");
        reply["result"].clone()
    }
    fn hello(&mut self) {
        self.send(json!({"id": 1, "method": "hello", "params": {"version": VERSION}}));
        let reply = self.read();
        assert_eq!(reply["result"]["version"], VERSION);
        assert_eq!(reply["result"]["app_version"], env!("CARGO_PKG_VERSION"));
    }
}

#[test]
fn off_has_no_worker_on_serves_fake_engine_state_and_stop_removes_the_socket() {
    let mut rig = Rig::new();
    assert!(rig.control.borrow().server.is_none());
    assert!(!rig.socket().exists());
    rig.enable(true);
    assert_eq!(
        std::fs::metadata(rig.socket())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let mut client = rig.client();
    client.hello();
    let state = client.request("state");
    assert_eq!(state["layout"]["name"], DESCRIPTORS[0].name);
    assert_eq!(
        state["layout"]["controls"].as_array().unwrap().len(),
        DESCRIPTORS[0].controls.len()
    );
    assert!(state["connection"]["connected"].as_bool().unwrap());
    assert!(!state["content"]["surfaces"].as_array().unwrap().is_empty());
    for method in [
        "layout",
        "kinds",
        "profiles",
        "pages",
        "folders",
        "content",
        "device_settings",
        "history",
        "connection",
    ] {
        client.request(method);
    }
    rig.engine.command(e::Command::SetLabel {
        surface: e::SurfaceId::new(e::SurfaceKind::KeyFace, 1),
        label: Some("Changed".into()),
    });
    let content = client.request("content");
    assert!(content["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["label"] == "Changed"));
    assert!(client.request("history")["undo"].is_string());
    rig.enable(false);
    assert!(!rig.socket().exists());
    assert!(rig.control.borrow().server.is_none());
    let mut byte = [0];
    assert_eq!(client.0.read(&mut byte).unwrap(), 0);
    rig.enable(true);
    let mut next = rig.client();
    next.hello();
    assert_eq!(next.request("history")["undo"], "Set Title");
    rig.enable(false);
}

#[test]
fn versions_invalid_lines_unknown_methods_and_limits_do_not_stop_the_server() {
    let rig = Rig::new();
    rig.enable(true);
    let mut client = rig.client();
    client.send(json!({"id": 3, "method": "hello", "params": {"version": VERSION + 1}}));
    assert_eq!(client.read()["error"]["code"], "version_mismatch");
    assert_eq!(client.0.read(&mut [0]).unwrap(), 0);
    let mut client = rig.client();
    client.send(json!({"id": 2, "method": "state", "params": {}}));
    assert_eq!(client.read()["error"]["code"], "hello_required");
    client.hello();
    client.0.get_mut().write_all(b"not JSON\n").unwrap();
    assert_eq!(client.read()["error"]["code"], "invalid_request");
    client.send(json!({"id": 4, "method": "stop", "params": {}}));
    let reply = client.read();
    assert_eq!(reply["id"], 4);
    assert_eq!(reply["error"]["code"], "unknown_method");
    for bad in [
        json!({"id": 1, "method": "state", "params": {}, "extra": true}),
        json!({"id": 1, "method": "state", "params": {"extra": true}}),
        json!({"id": 1, "method": "hello", "params": {"version": "one"}}),
        json!([]),
    ] {
        client.send(bad);
        assert!(client.read().get("error").is_some());
    }
    client
        .0
        .get_mut()
        .write_all(&vec![b'x'; MAX_LINE + 1])
        .unwrap();
    assert_eq!(client.read()["error"]["code"], "line_too_long");
    assert_eq!(client.0.read(&mut [0]).unwrap(), 0);
    let mut good = rig.client();
    good.hello();
    good.request("state");
}

#[test]
fn partial_and_pipelined_lines_are_framed_correctly() {
    let rig = Rig::new();
    rig.enable(true);
    let mut client = rig.client();
    client
        .0
        .get_mut()
        .write_all(b"{\"id\":1,\"method\":\"hel")
        .unwrap();
    client
        .0
        .get_mut()
        .write_all(
            b"lo\",\"params\":{\"version\":1}}\n{\"id\":2,\"method\":\"history\",\"params\":{}}\n",
        )
        .unwrap();
    assert_eq!(client.read()["id"], 1);
    assert_eq!(client.read()["id"], 2);
}

#[test]
fn slow_clients_do_not_block_events_other_clients_or_disable() {
    let mut rig = Rig::new();
    rig.enable(true);
    // Each response exceeds the socket buffer; this peer never reads it.
    rig.engine.command(e::Command::SetLabel {
        surface: e::SurfaceId::new(e::SurfaceKind::KeyFace, 1),
        label: Some("X".repeat(512 * 1024)),
    });
    let mut slow = rig.client();
    slow.send(json!({"id": 1, "method": "hello", "params": {"version": VERSION}}));
    for id in 2..8 {
        slow.send(json!({"id": id, "method": "state", "params": {}}));
    }
    let start = Instant::now();
    for _ in 0..600 {
        rig.control.borrow().observe(&e::Event::History {
            undo: Some("Latest".into()),
            redo: None,
        });
    }
    let mut good = rig.client();
    good.hello();
    assert_eq!(good.request("history")["undo"], "Latest");
    rig.enable(false);
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(!rig.socket().exists());
}

#[test]
fn stale_cleanup_is_explicit_and_never_removes_other_files() {
    let rig = Rig::new();
    let listener = UnixListener::bind(rig.socket()).unwrap();
    drop(listener);
    super::remove_stale(rig.dir.path()).unwrap();
    assert!(!rig.socket().exists());
    super::remove_stale(rig.dir.path()).unwrap();
    std::fs::write(rig.socket(), "keep").unwrap();
    assert!(super::remove_stale(rig.dir.path()).is_err());
    assert_eq!(std::fs::read_to_string(rig.socket()).unwrap(), "keep");
}

#[test]
fn results_hide_home_paths_in_nested_actions_and_never_include_engine_logs() {
    let rig = Rig::new();
    let home = std::env::var("HOME").unwrap();
    rig.control
        .borrow()
        .observe(&e::Event::Problem("device serial: PRIVATE-SERIAL".into()));
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
                settings: e::Settings::from([(
                    "nested".into(),
                    e::Value::Table(e::Settings::from([(
                        "path".into(),
                        e::Value::Text(format!("{home}/Documents/test")),
                    )])),
                )]),
            }),
            available: false,
        }));
    rig.enable(true);
    let mut client = rig.client();
    client.hello();
    let state = client.request("state").to_string();
    assert!(!state.contains(&home));
    assert!(!state.contains("PRIVATE-SERIAL"));
    assert!(state.contains("~/Documents/test"));
}

#[test]
fn choices_are_correlated_per_client_and_do_not_hold_up_reads() {
    let rig = Rig::new();
    rig.control
        .borrow()
        .observe(&e::Event::Kinds(vec![e::KindDescription {
            kind: "test".into(),
            fields: vec![e::Field {
                key: "device".into(),
                label: "Device".into(),
                input: e::InputType::RuntimeChoice,
                default: None,
            }],
            ..Default::default()
        }]));

    rig.enable(true);
    let mut first = rig.client();
    let mut second = rig.client();
    first.hello();
    second.hello();
    first.send(json!({"id": 10, "method": "choices", "params": {"action": {"kind": "test", "settings": {}}, "field": "device"}}));
    let command = rig.commands.recv_timeout(Duration::from_secs(3)).unwrap().0;
    let e::Command::Choices { id: first_id, .. } = command else {
        panic!("not choices")
    };
    second.request("history");
    second.send(json!({"id": 11, "method": "choices", "params": {"action": {"kind": "test"}, "field": "device"}}));
    let e::Command::Choices { id: second_id, .. } =
        rig.commands.recv_timeout(Duration::from_secs(3)).unwrap().0
    else {
        panic!("not choices")
    };
    assert_ne!(first_id, second_id);
    for (id, value) in [(second_id, "second"), (first_id, "first")] {
        rig.control.borrow().observe(&e::Event::Choices {
            id,
            options: vec![e::ChoiceOption {
                value: value.into(),
                label: value.into(),
            }],
        });
    }
    assert_eq!(first.read()["result"][0]["value"], "first");
    assert_eq!(second.read()["result"][0]["value"], "second");
    rig.enable(false);
    rig.enable(true);
    let mut restarted = rig.client();
    restarted.hello();
    restarted.send(json!({"id": 12, "method": "choices", "params": {"action": {"kind": "test"}, "field": "device"}}));
    let e::Command::Choices { id: next_id, .. } =
        rig.commands.recv_timeout(Duration::from_secs(3)).unwrap().0
    else {
        panic!("not choices")
    };
    assert_ne!(first_id, next_id);
    for (id, value) in [(first_id, "late"), (next_id, "new")] {
        rig.control.borrow().observe(&e::Event::Choices {
            id,
            options: vec![e::ChoiceOption {
                value: value.into(),
                label: value.into(),
            }],
        });
    }
    assert_eq!(restarted.read()["result"][0]["value"], "new");
}

#[test]
fn pages_clear_bindings_and_model_changes_clear_model_state() {
    let rig = Rig::new();
    let control = rig.control.borrow();
    control.observe(&e::Event::Binding(e::BindingState {
        binding: e::BindingId {
            page: 0,
            folder: None,
            control: e::ControlId::new(e::ControlKind::Key, 1),
            gesture: e::GestureKind::Press,
        },
        action: Some(e::Action {
            kind: "test".into(),
            settings: Default::default(),
        }),
        available: false,
    }));
    assert_eq!(control.feed.read(|s| s.content.bindings.len()), 1);
    control.observe(&e::Event::Pages {
        names: vec!["Other".into()],
        location: e::Location::default(),
        first: 0,
        folders: vec![],
    });
    assert!(control.feed.read(|s| s.content.bindings.is_empty()));
}

#[test]
fn a_new_layout_keeps_the_state_that_the_engine_does_not_send_again() {
    // The second descriptor has the model of the first one. The engine sends
    // the layout, the kinds and the surfaces again, and no other state.
    assert_eq!(DESCRIPTORS[0].model, DESCRIPTORS[1].model);
    let mut rig = Rig::with_device(&DESCRIPTORS[1]);
    rig.enable(true);
    let mut client = rig.client();
    client.hello();
    let state = client.request("state");
    assert!(state["profiles"]["active"].is_string());
    assert!(!state["pages"]["names"].as_array().unwrap().is_empty());
    assert!(state["device_settings"].is_object());
    assert_eq!(state["layout"]["name"], DESCRIPTORS[1].name);
    // A status and the choice to follow the front application stay when the
    // layout and the kinds come again.
    let kinds = || {
        e::Event::Kinds(vec![e::KindDescription {
            kind: "test".into(),
            ..Default::default()
        }])
    };
    let control = rig.control.borrow();
    control.observe(&kinds());
    control.observe(&e::Event::Status {
        kind: "test".into(),
        status: e::Status::Ready,
    });
    control.observe(&e::Event::FollowFrontApplication(true));
    control.observe(&e::Event::Layout(std::sync::Arc::new(
        e::DeviceLayout::of_default_model(),
    )));
    control.observe(&kinds());
    drop(control);
    let state = client.request("state");
    assert!(state["profiles"]["follow_front_application"]
        .as_bool()
        .unwrap());
    assert!(!state["kinds"][0]["status"].is_null());
    assert!(state["profiles"]["active"].is_string());
    rig.engine.poll(Duration::ZERO);
}

#[test]
fn protocol_types_round_trip_and_the_command_audit_rejects_unsafe_commands() {
    let request = Request {
        id: 5,
        method: Method::Hello(HelloParams { version: VERSION }),
    };
    let encoded = serde_json::to_value(request).unwrap();
    assert_eq!(
        encoded,
        json!({"id": 5, "method": "hello", "params": {"version": VERSION}})
    );
    assert!(serde_json::from_value::<Request>(encoded).is_ok());
    let response = Response::success(
        5,
        HelloResult {
            version: VERSION,
            app_version: "test".into(),
        },
    );
    assert!(matches!(
        serde_json::from_value::<Response<HelloResult>>(serde_json::to_value(response).unwrap())
            .unwrap()
            .body,
        ResponseBody::Success { .. }
    ));
    assert!(!super::state::permitted(&e::Command::Stop));
    assert!(!super::state::permitted(&e::Command::AtLocation {
        location: e::Location::default(),
        command: Box::new(e::Command::Stop),
    }));
    assert!(!super::state::permitted(&e::Command::InProfile {
        profile: "p".into(),
        command: Box::new(e::Command::TestAction {
            binding: e::BindingId {
                page: 0,
                folder: None,
                control: e::ControlId::new(e::ControlKind::Key, 1),
                gesture: e::GestureKind::Press,
            },
        }),
    }));
    assert!(super::state::permitted(&e::Command::ReadProfile {
        id: 1,
        profile: "p".into()
    }));
    assert!(!super::state::permitted(&e::Command::Group {
        name: "test".into(),
        commands: vec![e::Command::Stop]
    }));
}

mod changes;
mod reliability;
