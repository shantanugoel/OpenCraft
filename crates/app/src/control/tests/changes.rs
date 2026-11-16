//! The change requests, against a real engine thread with no device.

use super::*;

const KEY_1: &str = r#"{"kind":"key face","number":1}"#;

struct Live {
    control: Control,
    engine: Option<e::EngineHandle>,
    dir: tempfile::TempDir,
    client: Client,
    active: String,
}

impl Live {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("oc")
            .tempdir_in("/tmp")
            .unwrap();
        let feed = Feed::default();
        let sink = feed.clone();
        let engine = e::spawn(
            e::Config {
                profiles_dir: dir.path().join("profiles"),
                no_device: true,
            },
            Vec::new(),
            move |event| sink.observe(&event),
        )
        .unwrap();
        let mut control = Control::with(feed, Some(super::super::port(engine.sender())));
        control.set_enabled(true, dir.path()).unwrap();
        let mut client = Client::new(UnixStream::connect(dir.path().join(FILE_NAME)).unwrap());
        client.hello();
        let mut live = Self {
            control,
            engine: Some(engine),
            dir,
            client,
            active: String::new(),
        };
        // The engine thread starts at its own time.
        let start = Instant::now();
        loop {
            let state = live.read("state");
            if let Some(active) = state["profiles"]["active"].as_str() {
                if !state["pages"]["names"].as_array().unwrap().is_empty() {
                    live.active = active.into();
                    break;
                }
            }
            assert!(start.elapsed() < Duration::from_secs(10), "no start");
            std::thread::sleep(Duration::from_millis(20));
        }
        live
    }

    fn read(&mut self, method: &str) -> Value {
        self.client.request(method)
    }

    /// The whole reply of one request.
    fn call(&mut self, method: &str, params: Value) -> Value {
        self.client
            .send(json!({"id": 9, "method": method, "params": params}));
        let reply = self.client.read();
        assert_eq!(reply["id"], 9);
        reply
    }

    fn ok(&mut self, method: &str, params: Value) {
        let reply = self.call(method, params);
        assert_eq!(reply["result"], json!({}), "{method}: {reply}");
    }

    /// The code and the text of the error of one request.
    fn fails(&mut self, method: &str, params: Value) -> (String, String) {
        let reply = self.call(method, params);
        let error = &reply["error"];
        assert!(error.is_object(), "{method} did not fail: {reply}");
        (
            error["code"].as_str().unwrap().into(),
            error["text"].as_str().unwrap().into(),
        )
    }

    fn at(&self, page: usize) -> Value {
        json!({"profile": self.active, "page": page, "folder": null})
    }

    fn at_in(&self, page: usize, folder: &str) -> Value {
        json!({"profile": self.active, "page": page, "folder": folder})
    }

    /// The content of a location, with no selection of the location.
    fn content(&mut self, target: Value) -> Value {
        let reply = self.call("content", json!({ "target": target }));
        assert!(reply.get("error").is_none(), "{reply}");
        reply["result"].clone()
    }

    fn label(&mut self, page: usize) -> Value {
        let content = self.content(self.at(page));
        content["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == serde_json::from_str::<Value>(KEY_1).unwrap())
            .map(|s| s["label"].clone())
            .unwrap_or(Value::Null)
    }

    fn page_count(&mut self) -> usize {
        self.read("pages")["names"].as_array().unwrap().len()
    }

    fn file(&self, name: &str) -> String {
        self.dir.path().join(name).to_str().unwrap().into()
    }

    fn image(&self, name: &str) -> String {
        let path = self.file(name);
        e::RgbImage::from_pixel(8, 8, image::Rgb([200, 30, 30]))
            .save(&path)
            .unwrap();
        path
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        self.control.set_enabled(false, self.dir.path()).unwrap();
        if let Some(engine) = self.engine.take() {
            engine.stop();
        }
    }
}

#[test]
fn a_surface_request_changes_the_named_page_and_makes_it_the_active_page() {
    let mut live = Live::new();
    live.ok("add_page", json!({"target": live.at(0), "copy_of": null}));
    assert_eq!(live.page_count(), 2);
    live.ok(
        "set_label",
        json!({"target": live.at(0), "surface": serde_json::from_str::<Value>(KEY_1).unwrap(), "label": "Zero"}),
    );
    // The request moved the device to page 0, as the decision says.
    assert_eq!(live.read("pages")["location"]["page"], 0);
    live.ok(
        "set_label",
        json!({"target": live.at(1), "surface": serde_json::from_str::<Value>(KEY_1).unwrap(), "label": "One"}),
    );
    assert_eq!(live.read("pages")["location"]["page"], 1);
    assert_eq!(live.label(0), "Zero");
    assert_eq!(live.label(1), "One");
    // A read of the content of a location that is not active does not move.
    assert_eq!(live.read("pages")["location"]["page"], 1);
    assert_eq!(live.read("content")["location"]["page"], 1);
}

#[test]
fn each_surface_request_sets_and_removes_its_part() {
    let mut live = Live::new();
    let surface = serde_json::from_str::<Value>(KEY_1).unwrap();
    let target = live.at(0);
    let find = |content: &Value| {
        content["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == serde_json::from_str::<Value>(KEY_1).unwrap())
            .unwrap()
            .clone()
    };
    live.ok(
        "set_label",
        json!({"target": target, "surface": surface, "label": "Hi"}),
    );
    live.ok(
        "set_label_style",
        json!({"target": target, "surface": surface,
               "style": {"position": "top", "size": 20, "color": "#ff0000", "bold": false}}),
    );
    live.ok(
        "set_color",
        json!({"target": target, "surface": surface, "color": "#00ff00"}),
    );
    live.ok(
        "set_icon",
        json!({"target": target, "surface": surface, "icon": {"name": "star", "color": null}}),
    );
    // An icon and an image replace each other, thus read the icon first.
    let key = find(&live.content(live.at(0)));
    assert!(!key["icon"].is_null());
    let picture = live.image("a.png");
    live.ok(
        "set_image",
        json!({"target": target, "surface": surface, "path": picture}),
    );
    let key = find(&live.content(live.at(0)));
    assert_eq!(key["label"], "Hi");
    assert_eq!(key["label_style"]["position"], "top");
    assert_eq!(key["label_style"]["size"], 20);
    assert_eq!(key["label_style"]["bold"], false);
    assert_eq!(key["color"], "#00FF00");
    assert!(key["image"].is_string());
    // null removes each part. No style is the default style.
    for (method, name) in [
        ("set_label", "label"),
        ("set_color", "color"),
        ("set_icon", "icon"),
        ("set_image", "path"),
    ] {
        live.ok(
            method,
            json!({"target": live.at(0), "surface": surface, name: null}),
        );
    }
    live.ok(
        "set_label_style",
        json!({"target": live.at(0), "surface": surface, "style": null}),
    );
    let key = find(&live.content(live.at(0)));
    assert!(key["label"].is_null());
    assert!(key["color"].is_null());
    assert!(key["image"].is_null());
    assert!(key["icon"].is_null());
    assert!(key["label_style"]["position"].is_null());
    assert_eq!(key["label_style"]["bold"], true);
}

#[test]
fn binding_and_control_requests_set_remove_copy_paste_clear_and_copy_to_all_pages() {
    let mut live = Live::new();
    live.ok("add_page", json!({"target": live.at(0), "copy_of": null}));
    let key1 = json!({"kind": "key", "number": 1});
    let key2 = json!({"kind": "key", "number": 2});
    let bindings = |content: &Value| content["bindings"].as_array().unwrap().len();
    live.ok(
        "set_binding",
        json!({"target": live.at(0), "control": key1, "gesture": "press",
               "action": {"kind": "test", "settings": {"a": "b"}}}),
    );
    let content = live.content(live.at(0));
    assert_eq!(bindings(&content), 1);
    assert_eq!(content["bindings"][0]["action"]["kind"], "test");
    // Copy, paste on a different key, and clear.
    live.ok(
        "copy_control",
        json!({"target": live.at(0), "control": key1}),
    );
    live.ok(
        "paste_control",
        json!({"target": live.at(0), "control": key2}),
    );
    assert_eq!(bindings(&live.content(live.at(0))), 2);
    live.ok(
        "clear_control",
        json!({"target": live.at(0), "control": key2}),
    );
    assert_eq!(bindings(&live.content(live.at(0))), 1);
    live.ok(
        "copy_to_all_pages",
        json!({"target": live.at(0), "control": key1}),
    );
    assert_eq!(bindings(&live.content(live.at(1))), 1);
    // null removes the action.
    live.ok(
        "set_binding",
        json!({"target": live.at(1), "control": key1, "gesture": "press", "action": null}),
    );
    assert_eq!(bindings(&live.content(live.at(1))), 0);
    assert_eq!(bindings(&live.content(live.at(0))), 1);
}

#[test]
fn page_requests_add_rename_move_select_remove_and_set_the_background() {
    let mut live = Live::new();
    live.ok("add_page", json!({"target": live.at(0), "copy_of": null}));
    live.ok("add_page", json!({"target": live.at(0), "copy_of": 0}));
    assert_eq!(live.page_count(), 3);
    live.ok(
        "rename_page",
        json!({"target": live.at(2), "name": "Third"}),
    );
    assert_eq!(live.read("pages")["names"][2], "Third");
    live.ok("move_page", json!({"target": live.at(2), "to": 0}));
    assert_eq!(live.read("pages")["names"][0], "Third");
    live.ok("select_page", json!({"target": live.at(1)}));
    assert_eq!(live.read("pages")["location"]["page"], 1);
    live.ok(
        "set_background",
        json!({"target": live.at(1), "background": {"type": "color", "color": "#102030"}}),
    );
    assert_eq!(live.content(live.at(1))["background"]["type"], "color");
    let picture = live.image("b.png");
    live.ok(
        "set_background",
        json!({"target": live.at(1), "background": {"type": "image", "path": picture, "fit": "fill"}}),
    );
    live.ok(
        "set_background_fit",
        json!({"target": live.at(1), "fit": "fit"}),
    );
    assert_eq!(live.content(live.at(1))["background"]["fit"], "fit");
    live.ok(
        "set_background",
        json!({"target": live.at(1), "background": null}),
    );
    assert!(live.content(live.at(1))["background"].is_null());
    live.ok("remove_page", json!({"target": live.at(2)}));
    assert_eq!(live.page_count(), 2);
}

#[test]
fn folder_requests_add_rename_move_open_close_and_remove() {
    let mut live = Live::new();
    live.ok(
        "add_folder",
        json!({"target": live.at(0), "id": "00000000000000a1", "name": "First", "copy_of": null}),
    );
    // Without an identity, OpenCraft makes one.
    live.ok("add_folder", json!({"target": live.at(0), "name": "Other"}));
    let folders = live.read("folders");
    assert_eq!(folders.as_array().unwrap().len(), 2);
    live.ok(
        "rename_folder",
        json!({"target": live.at(0), "folder": "00000000000000a1", "name": "Renamed"}),
    );
    live.ok(
        "move_folder",
        json!({"target": live.at(0), "folder": "00000000000000a1", "to": 1}),
    );
    let folders = live.read("folders");
    assert_eq!(folders[1]["id"], "00000000000000a1");
    assert_eq!(folders[1]["name"], "Renamed");
    live.ok(
        "open_folder",
        json!({"target": live.at(0), "folder": "00000000000000a1"}),
    );
    assert_eq!(
        live.read("pages")["location"]["open"],
        json!(["00000000000000a1"])
    );
    // A request for a location in a folder works in that folder.
    live.ok(
        "set_label",
        json!({"target": live.at_in(0, "00000000000000a1"),
               "surface": serde_json::from_str::<Value>(KEY_1).unwrap(), "label": "In"}),
    );
    let inside = live.content(live.at_in(0, "00000000000000a1"));
    assert!(inside["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["label"] == "In"));
    assert_eq!(live.label(0), Value::Null);
    live.ok("close_folder", json!({"target": live.at(0)}));
    assert_eq!(live.read("pages")["location"]["open"], json!([]));
    live.ok(
        "remove_folder",
        json!({"target": live.at(0), "folder": "00000000000000a1"}),
    );
    assert_eq!(live.read("folders").as_array().unwrap().len(), 1);
}

#[test]
fn profile_requests_add_rename_colour_link_export_import_select_and_remove() {
    let mut live = Live::new();
    let first = live.active.clone();
    live.ok("add_profile", json!({"name": "Second", "copy_of": null}));
    let profiles = live.read("profiles");
    let second = profiles["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Second")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(profiles["active"], second);
    live.ok(
        "rename_profile",
        json!({"profile": second, "name": "Renamed"}),
    );
    live.ok(
        "set_profile_color",
        json!({"profile": second, "color": "#336699"}),
    );
    live.ok("set_default_profile", json!({"profile": second}));
    live.ok(
        "link_application",
        json!({"profile": second, "link": {"bundle_id": "com.example.app", "name": "Example"}}),
    );
    live.ok("set_follow_front_application", json!({"follow": true}));
    live.ok("set_first_page", json!({"profile": second, "page": 0}));
    let profiles = live.read("profiles");
    let entry = profiles["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == second)
        .unwrap();
    assert_eq!(entry["name"], "Renamed");
    assert_eq!(entry["color"], "#336699");
    assert_eq!(entry["applications"][0]["bundle_id"], "com.example.app");
    assert_eq!(profiles["default"], second);
    assert_eq!(profiles["follow_front_application"], true);
    live.ok(
        "unlink_application",
        json!({"profile": second, "bundle_id": "com.example.app"}),
    );
    live.ok("set_follow_front_application", json!({"follow": false}));
    let export = live.file("exported.opencraft");
    live.ok("export_profile", json!({"profile": second, "path": export}));
    assert!(std::path::Path::new(&export).exists());
    live.ok("import_profile", json!({"path": export}));
    assert_eq!(
        live.read("profiles")["profiles"].as_array().unwrap().len(),
        3
    );
    live.ok("select_profile", json!({"profile": first}));
    assert_eq!(live.read("profiles")["active"], first);
    live.ok("remove_profile", json!({"profile": second}));
    assert_eq!(
        live.read("profiles")["profiles"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn a_device_settings_request_sets_the_settings() {
    let mut live = Live::new();
    let mut settings = live.read("device_settings");
    settings["brightness"] = json!(37);
    settings["haptic"] = json!(false);
    live.ok("set_device_settings", settings);
    let settings = live.read("device_settings");
    assert_eq!(settings["brightness"], 37);
    assert_eq!(settings["haptic"], false);
}

#[test]
fn one_request_is_one_step_of_the_history_and_the_window_can_undo_it() {
    let mut live = Live::new();
    let surface = serde_json::from_str::<Value>(KEY_1).unwrap();
    live.ok(
        "set_label_style",
        json!({"target": live.at(0), "surface": surface,
               "style": {"position": "top", "size": 20, "color": "#ff0000", "bold": false}}),
    );
    // The four changes of the style are one step.
    assert_eq!(live.read("history")["undo"], "Set Label Style");
    live.ok(
        "set_label",
        json!({"target": live.at(0), "surface": surface, "label": "Window"}),
    );
    assert_eq!(live.read("history")["undo"], "Set Label");
    // The window sends Undo with no reply.
    live.engine.as_ref().unwrap().send(e::Command::Undo);
    let start = Instant::now();
    while live.read("history")["undo"] != "Set Label Style" {
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(live.label(0).is_null());
    let key = &live.content(live.at(0))["surfaces"];
    assert!(key
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["label_style"]["size"] == 20));
    // The client can undo and redo, too.
    live.ok("undo", json!({"profile": live.active}));
    assert_eq!(live.read("history")["undo"], Value::Null);
    live.ok("redo", json!({"profile": live.active}));
    assert_eq!(live.read("history")["undo"], "Set Label Style");
}

#[test]
fn a_response_comes_after_the_events_of_its_change() {
    let mut live = Live::new();
    let surface = serde_json::from_str::<Value>(KEY_1).unwrap();
    for number in 0..40 {
        let label = format!("Label {number}");
        live.ok(
            "set_label",
            json!({"target": live.at(0), "surface": surface, "label": label}),
        );
        // The read after the response never shows an older state.
        let content = live.read("content");
        let shown = content["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == surface)
            .unwrap()["label"]
            .clone();
        assert_eq!(shown, label);
        assert_eq!(live.read("history")["undo"], "Set Label");
    }
}

#[test]
fn a_request_for_a_page_or_a_folder_that_is_gone_or_a_profile_that_is_gone_changes_nothing() {
    let mut live = Live::new();
    live.ok("add_page", json!({"target": live.at(0), "copy_of": null}));
    live.ok("select_page", json!({"target": live.at(1)}));
    let surface = serde_json::from_str::<Value>(KEY_1).unwrap();
    let history = live.read("history");
    for target in [
        live.at(9),
        live.at_in(0, "00000000000000ee"),
        json!({"profile": "not-a-profile", "page": 0, "folder": null}),
    ] {
        let (code, text) = live.fails(
            "set_label",
            json!({"target": target, "surface": surface, "label": "No"}),
        );
        assert_eq!(code, "engine_problem");
        assert!(!text.is_empty());
        // The page that was active stays active, and nothing has the label.
        assert_eq!(live.read("pages")["location"]["page"], 1);
        assert_eq!(live.read("history"), history);
        assert!(live.label(0).is_null());
        assert!(live.label(1).is_null());
    }
    // A request that changes the structure also changes nothing.
    let (_, text) = live.fails("rename_page", json!({"target": live.at(7), "name": "No"}));
    assert!(text.contains("no page"), "{text}");
    assert_eq!(live.page_count(), 2);
}

#[test]
fn a_problem_of_the_engine_is_the_error_of_the_request_with_its_text() {
    let mut live = Live::new();
    let (code, text) = live.fails("rename_page", json!({"target": live.at(0), "name": ""}));
    assert_eq!(code, "engine_problem");
    assert_eq!(text, "A page must have a name.");
    let (_, text) = live.fails("remove_page", json!({"target": live.at(0)}));
    assert_eq!(text, "A profile keeps one page at the minimum.");
    let (_, text) = live.fails("add_profile", json!({"name": "  ", "copy_of": null}));
    assert!(text.contains("name"), "{text}");
    // A problem of a request is not left for the next request.
    live.ok("add_page", json!({"target": live.at(0), "copy_of": null}));
}

#[test]
fn a_relative_file_path_gives_an_error_and_changes_nothing() {
    let mut live = Live::new();
    let surface = serde_json::from_str::<Value>(KEY_1).unwrap();
    for (method, params) in [
        (
            "set_image",
            json!({"target": live.at(0), "surface": surface, "path": "picture.png"}),
        ),
        (
            "set_background",
            json!({"target": live.at(0),
                   "background": {"type": "image", "path": "picture.png", "fit": "fill"}}),
        ),
        (
            "export_profile",
            json!({"profile": live.active, "path": "out.opencraft"}),
        ),
        ("import_profile", json!({"path": "in.opencraft"})),
    ] {
        let (code, text) = live.fails(method, params);
        assert_eq!(code, "invalid_params", "{method}");
        assert_eq!(text, "A file path must be absolute.");
    }
    assert!(live.read("history")["undo"].is_null());
}

#[test]
fn no_request_can_make_the_commands_that_a_client_must_not_send() {
    let mut live = Live::new();
    for method in [
        "test_action",
        "stop",
        "power_notice",
        "front_application",
        "start_text_edit",
        "end_text_edit",
        "wake_device",
        "read_counters",
        "add_starter_profile",
        "resolve",
        "press",
        "gesture",
    ] {
        assert!(!METHODS.contains(&method), "{method}");
        let (code, _) = live.fails(method, json!({}));
        assert_eq!(code, "unknown_method", "{method}");
    }
    // Each name in the table is a method that exists, and a request with bad
    // parameters is not an unknown method.
    for method in METHODS.iter().filter(|m| **m != "hello") {
        let reply = live.call(method, json!({"bad": 1}));
        assert_ne!(reply["error"]["code"], "unknown_method", "{method}");
    }
}

#[test]
fn a_content_request_reads_an_inactive_profile_and_a_page_that_is_not_active() {
    let mut live = Live::new();
    let first = live.active.clone();
    let surface = serde_json::from_str::<Value>(KEY_1).unwrap();
    live.ok(
        "set_label",
        json!({"target": live.at(0), "surface": surface, "label": "First profile"}),
    );
    live.ok(
        "set_binding",
        json!({"target": live.at(0), "control": {"kind": "key", "number": 1},
               "gesture": "press", "action": {"kind": "test", "settings": {}}}),
    );
    live.ok("add_profile", json!({"name": "Second", "copy_of": null}));
    assert_ne!(live.read("profiles")["active"], first);
    let content = live.content(json!({"profile": first, "page": 0, "folder": null}));
    assert_eq!(content["profile"], first);
    assert!(content["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["label"] == "First profile"));
    assert_eq!(content["bindings"].as_array().unwrap().len(), 1);
    // The active profile did not change, and the read made no step.
    assert_ne!(live.read("profiles")["active"], first);
    let reply = live.call(
        "content",
        json!({"target": {"profile": first, "page": 5, "folder": null}}),
    );
    assert_eq!(reply["error"]["code"], "engine_problem");
    let reply = live.call(
        "content",
        json!({"target": {"profile": "not-a-profile", "page": 0, "folder": null}}),
    );
    assert_eq!(reply["error"]["code"], "engine_problem");
}

#[test]
fn a_request_with_a_wrong_surface_or_value_is_invalid_and_sends_no_command() {
    let mut live = Live::new();
    for (method, params) in [
        (
            "set_label",
            json!({"target": live.at(0), "surface": {"kind": "key face", "number": 99}, "label": "x"}),
        ),
        (
            "set_color",
            json!({"target": live.at(0), "surface": {"kind": "key face", "number": 1}, "color": "red"}),
        ),
        (
            "set_binding",
            json!({"target": live.at(0), "control": {"kind": "key", "number": 1},
                   "gesture": "wave", "action": null}),
        ),
        (
            "set_background_fit",
            json!({"target": live.at(0), "fit": "sideways"}),
        ),
        (
            "open_folder",
            json!({"target": live.at(0), "folder": "not hex"}),
        ),
    ] {
        let (code, _) = live.fails(method, params);
        assert_eq!(code, "invalid_params", "{method}");
    }
    assert!(live.read("history")["undo"].is_null());
}
