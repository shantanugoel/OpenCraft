//! The change requests, against a real engine thread with no device.

use super::*;

const KEY_1: &str = r#"{"kind":"key_face","number":1}"#;

fn surface_1() -> Value {
    serde_json::from_str(KEY_1).unwrap()
}

/// What a change that makes nothing gives: the place that the request names.
/// A part that the request has not is null. `undo` and `redo` give the place
/// of the step that they reversed, thus the test does not use this function
/// for them.
fn named(method: &str, params: &Value) -> Value {
    let target = &params["target"];
    let (profile, page, folder) = if target.is_object() {
        (&target["profile"], &target["page"], &target["folder"])
    } else {
        (&params["profile"], &Value::Null, &Value::Null)
    };
    let folder = match method {
        "open_folder" | "rename_folder" | "move_folder" | "remove_folder" => &params["folder"],
        _ => folder,
    };
    json!({"profile": profile, "page": page, "folder": folder})
}

/// A handler that has two kinds and does no work: the kind `test` with a
/// text, and the kind `test.keys` with a hotkey, a number, a choice and a list.
struct Kinds;

impl e::Handler for Kinds {
    fn kinds(&self) -> Vec<e::KindDescription> {
        let field = |key: &str, input| e::Field {
            key: key.into(),
            label: key.into(),
            input,
            default: None,
        };
        vec![
            e::KindDescription {
                kind: "test".into(),
                name: "Test".into(),
                fields: vec![
                    field("a", e::InputType::Text),
                    field("profile", e::InputType::Profile),
                    field("folder", e::InputType::Folder),
                    field("child", e::InputType::Action),
                    field("actions", e::InputType::ActionList),
                ],
                ..Default::default()
            },
            e::KindDescription {
                kind: "test.knob".into(),
                fields: vec![field("functions", e::InputType::TurnList)],
                ..Default::default()
            }
            .with_knob_group(),
            // A field that another field shows (decision 0038).
            e::KindDescription {
                kind: "test.shown".into(),
                name: "Shown".into(),
                fields: vec![
                    e::Field {
                        key: "operation".into(),
                        label: "Operation".into(),
                        input: e::InputType::Choice {
                            options: vec![
                                e::ChoiceOption {
                                    value: "up".into(),
                                    label: "Up".into(),
                                },
                                e::ChoiceOption {
                                    value: "down".into(),
                                    label: "Down".into(),
                                },
                            ],
                        },
                        default: Some(e::Value::Text("up".into())),
                    },
                    field("step", e::InputType::Integer { min: 0, max: 10 }),
                ],
                shown_when: vec![e::ShownWhen::new("step", "operation", &["up"])],
                ..Default::default()
            },
            e::KindDescription {
                kind: "test.keys".into(),
                name: "Keys".into(),
                fields: vec![
                    field("hotkey", e::InputType::Hotkey),
                    field("step", e::InputType::Integer { min: 1, max: 9 }),
                    field(
                        "mode",
                        e::InputType::Choice {
                            options: vec![e::ChoiceOption {
                                value: "up".into(),
                                label: "Up".into(),
                            }],
                        },
                    ),
                    field("steps", e::InputType::TimedActionList),
                ],
                ..Default::default()
            },
        ]
    }

    fn start(&mut self, _sender: e::MessageSender) {}

    fn handle(&mut self, _request: e::Request) {}
}

#[test]
fn knob_functions_are_knob_only_and_validate_both_nested_actions() {
    let mut live = Live::new();
    let kinds = live.read("kinds");
    let kind = kinds
        .as_array()
        .unwrap()
        .iter()
        .find(|kind| kind["kind"] == "test.knob")
        .unwrap();
    assert_eq!(kind["knob_group"], true);
    assert_eq!(kind["fields"][0]["input"]["type"], "turn_list");
    let target = live.at(0);
    let set = |control, gesture, action| {
        json!({"target": target,
        "control": {"kind": control, "number": 1}, "gesture": gesture, "action": action})
    };
    let pair = |left, right| json!({"kind":"test.knob","settings":{"functions":[{"label":"Copy","left":left,"right":right}]}});
    let leaf = json!({"kind":"test", "settings":{"a":"Good"}});
    let action = pair(leaf.clone(), leaf.clone());
    live.ok("set_binding", set("knob", "turn_left", action.clone()));
    assert_eq!(
        live.content(target.clone())["bindings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    for gesture in ["turn_right", "push"] {
        live.ok("set_binding", set("knob", gesture, action.clone()));
    }
    for (control, gesture) in [
        ("key", "press"),
        ("bar_panel", "tap"),
        ("touch_bar", "swipe_left"),
    ] {
        assert!(live
            .fails("set_binding", set(control, gesture, action.clone()))
            .1
            .contains("for a knob only"));
    }
    let bad_leaf = json!({"kind":"test", "settings":{"a":42}});
    for (action, error) in [
        (pair(bad_leaf.clone(), leaf.clone()), "must be text"),
        (pair(leaf.clone(), bad_leaf), "must be text"),
        (pair(action.clone(), leaf.clone()), "for a knob only"),
        (
            json!({"kind":"test.knob","settings":{"functions":[{"left":leaf}]}}),
            "must be a list of functions",
        ),
        (
            json!({"kind":"test","settings":{"child":action.clone()}}),
            "for a knob only",
        ),
        (
            json!({"kind":"test","settings":{"actions":[action.clone()]}}),
            "for a knob only",
        ),
        (
            json!({"kind":"test.keys","settings":{"steps":[action]}}),
            "for a knob only",
        ),
    ] {
        assert!(live
            .fails("set_binding", set("knob", "push", action))
            .1
            .contains(error));
    }
}

struct Live {
    control: Control,
    engine: Option<e::EngineHandle>,
    dir: tempfile::TempDir,
    client: Client,
    /// A second connection that reads the pages for `at`.
    peek: std::cell::RefCell<Client>,
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
            vec![Box::new(Kinds)],
            move |event| sink.observe(&event),
        )
        .unwrap();
        let mut control = Control::with(feed, Some(super::super::port(engine.sender())));
        control.set_enabled(true, dir.path()).unwrap();
        let socket = dir.path().join(FILE_NAME);
        let mut client = Client::new(UnixStream::connect(&socket).unwrap());
        client.hello();
        let mut peek = Client::new(UnixStream::connect(&socket).unwrap());
        peek.hello();
        let mut live = Self {
            control,
            engine: Some(engine),
            dir,
            client,
            peek: std::cell::RefCell::new(peek),
            active: String::new(),
        };
        // The engine thread starts at its own time.
        let start = Instant::now();
        loop {
            let state = live.read("state");
            if let Some(active) = state["profiles"]["active"].as_str() {
                if !state["pages"]["pages"].as_array().unwrap().is_empty() {
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

    /// A change that makes nothing: the result names the place of the request.
    fn ok(&mut self, method: &str, params: Value) {
        let expected = named(method, &params);
        assert_eq!(self.made(method, params), expected, "{method}");
    }

    /// A change; the result is what it made.
    fn made(&mut self, method: &str, params: Value) -> Value {
        let reply = self.call(method, params);
        assert!(reply.get("error").is_none(), "{method}: {reply}");
        reply["result"].clone()
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

    /// The identity of the page with this number, as the pages read now gives.
    fn id(&self, page: usize) -> String {
        let pages = self.peek.borrow_mut().request("pages");
        pages["pages"][page]["id"].as_str().unwrap().to_string()
    }

    fn page(&self, page: usize) -> Value {
        json!({"profile": self.active, "page": self.id(page)})
    }

    fn at(&self, page: usize) -> Value {
        json!({"profile": self.active, "page": self.id(page), "folder": null})
    }

    fn at_in(&self, page: usize, folder: &str) -> Value {
        json!({"profile": self.active, "page": self.id(page), "folder": folder})
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
            .find(|s| s["id"] == surface_1())
            .map(|s| s["label"].clone())
            .unwrap_or(Value::Null)
    }

    fn page_count(&mut self) -> usize {
        self.read("pages")["pages"].as_array().unwrap().len()
    }

    /// The identity of the page that is active now.
    fn active_page(&mut self) -> String {
        self.read("pages")["location"]["page"]
            .as_str()
            .unwrap()
            .to_string()
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
    let profile = live.active.clone();
    live.made("add_page", json!({"profile": profile}));
    assert_eq!(live.page_count(), 2);
    live.ok(
        "set_label",
        json!({"target": live.at(0), "surface": surface_1(), "label": "Zero"}),
    );
    // The request moved the device to page 0, as the decision says.
    let (zero, one) = (live.id(0), live.id(1));
    assert_eq!(live.active_page(), zero);
    live.ok(
        "set_label",
        json!({"target": live.at(1), "surface": surface_1(), "label": "One"}),
    );
    assert_eq!(live.active_page(), one);
    assert_eq!(live.label(0), "Zero");
    assert_eq!(live.label(1), "One");
    // A read of the content of a location that is not active does not move.
    assert_eq!(live.active_page(), one);
    assert_eq!(live.read("content")["location"]["page"], one);
}

#[test]
fn each_surface_request_sets_and_removes_its_part() {
    let mut live = Live::new();
    let surface = surface_1();
    let target = live.at(0);
    let find = |content: &Value| {
        content["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == surface_1())
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
fn the_surface_requests_can_write_and_read_the_on_face() {
    let mut live = Live::new();
    let surface = surface_1();
    let target = live.at(0);
    let find = |live: &mut Live| {
        live.content(target.clone())["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == surface)
            .unwrap()
            .clone()
    };
    live.ok(
        "set_label",
        json!({"target": target, "surface": surface, "label": "Base"}),
    );
    assert!(find(&mut live).get("on").is_none());
    for (method, key, value) in [
        ("set_label", "label", json!("Live")),
        (
            "set_label_style",
            "style",
            json!({"position": "top", "size": 24, "color": "#ff0000", "bold": false}),
        ),
        ("set_color", "color", json!("#00ff00")),
        ("set_icon", "icon", json!({"name": "star", "color": null})),
    ] {
        live.ok(
            method,
            json!({"target": target, "surface": surface, "on": true, key: value}),
        );
    }
    let key = find(&mut live);
    assert_eq!(key["on"]["icon"]["name"], "star");
    let image = live.image("on.png");
    live.ok(
        "set_image",
        json!({"target": target, "surface": surface, "on": true, "path": image}),
    );
    let key = find(&mut live);
    assert_eq!(key["label"], "Base");
    assert!(key["color"].is_null() && key["image"].is_null() && key["icon"].is_null());
    assert_eq!(key["on"]["label"], "Live");
    assert_eq!(
        key["on"]["label_style"],
        json!({"position": "top", "size": 24, "color": "#FF0000", "bold": false})
    );
    assert_eq!(key["on"]["color"], "#00FF00");
    assert!(key["on"]["image"].as_str().unwrap().starts_with("images/"));
    assert!(key["on"]["icon"].is_null());
}

#[test]
fn binding_and_control_requests_set_remove_copy_paste_clear_and_copy_to_all_pages() {
    let mut live = Live::new();
    let profile = live.active.clone();
    live.made("add_page", json!({"profile": profile}));
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
    assert_eq!(content["bindings"][0]["page"], live.id(0));
    assert_eq!(content["bindings"][0]["gesture"], "press");
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
fn a_binding_needs_a_gesture_that_the_control_can_make_and_the_layout_lists_them() {
    let mut live = Live::new();
    let layout = live.read("layout");
    let gestures_of = |kind: &str| -> Vec<String> {
        layout["controls"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"]["kind"] == kind)
            .unwrap()["gestures"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g.as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(
        gestures_of("key"),
        ["press", "release", "long_press", "double_press"]
    );
    assert_eq!(gestures_of("knob"), ["turn_left", "turn_right", "push"]);
    assert_eq!(gestures_of("bar_panel"), ["tap"]);
    assert_eq!(gestures_of("touch_bar"), ["swipe_left", "swipe_right"]);
    // A gesture that the control cannot make is not a binding.
    let (code, text) = live.fails(
        "set_binding",
        json!({"target": live.at(0), "control": {"kind": "key", "number": 1},
               "gesture": "push", "action": {"kind": "test", "settings": {}}}),
    );
    assert_eq!(code, "invalid_params");
    assert!(text.contains("gesture"), "{text}");
    assert!(live.read("history")["undo"].is_null());
    assert!(live.content(live.at(0))["bindings"]
        .as_array()
        .unwrap()
        .is_empty());
    for gesture in ["long_press", "double_press"] {
        let action = json!({"kind": "test", "settings": {}});
        live.ok(
            "set_binding",
            json!({"target": live.at(0), "control": {"kind": "key", "number": 1}, "gesture": gesture, "action": action}),
        );
        assert!(live.content(live.at(0))["bindings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|binding| binding["gesture"] == gesture));
        let (code, _) = live.fails(
            "set_binding",
            json!({"target": live.at(0), "control": {"kind": "knob", "number": 1}, "gesture": gesture, "action": action}),
        );
        assert_eq!(code, "invalid_params");
    }
    // A knob has the push.
    live.ok(
        "set_binding",
        json!({"target": live.at(0), "control": {"kind": "knob", "number": 1},
               "gesture": "push", "action": {"kind": "test", "settings": {}}}),
    );
}

#[test]
fn page_requests_add_rename_move_select_remove_and_set_the_background() {
    let mut live = Live::new();
    let profile = live.active.clone();
    let second = live.made("add_page", json!({"profile": profile}));
    assert_eq!(second["page"], live.id(1));
    // A copy has its own identity, and it is the new active page.
    let third = live.made("duplicate_page", json!({"target": live.page(0)}));
    assert_eq!(third["page"], live.id(2));
    assert_eq!(live.active_page(), live.id(2));
    assert_eq!(live.page_count(), 3);
    let ids: Vec<_> = (0..3).map(|n| live.id(n)).collect();
    assert_eq!(
        ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
        3
    );
    live.ok(
        "rename_page",
        json!({"target": live.page(2), "name": "Third"}),
    );
    assert_eq!(live.read("pages")["pages"][2]["name"], "Third");
    live.ok("move_page", json!({"target": live.page(2), "to": 1}));
    assert_eq!(live.read("pages")["pages"][0]["name"], "Third");
    // The identity moved with the page.
    assert_eq!(live.id(0), ids[2]);
    // The places count from 1, as the window numbers the pages: 0 is not a
    // place, and the list has no place 4. A request that would move nothing
    // is a refusal, not a success.
    for to in [0, 4] {
        let (code, text) = live.fails("move_page", json!({"target": live.page(0), "to": to}));
        assert_eq!(code, "invalid_params", "{to}: {text}");
        assert!(text.contains("The number of the place"), "{text}");
    }
    assert_eq!(live.id(0), ids[2]);
    live.ok("select_page", json!({"target": live.page(1)}));
    assert_eq!(live.active_page(), live.id(1));
    live.ok(
        "set_background",
        json!({"target": live.page(1), "background": {"type": "color", "color": "#102030"}}),
    );
    assert_eq!(live.content(live.at(1))["background"]["type"], "color");
    let picture = live.image("b.png");
    live.ok(
        "set_background",
        json!({"target": live.page(1), "background": {"type": "image", "path": picture, "fit": "fill"}}),
    );
    live.ok(
        "set_background_fit",
        json!({"target": live.page(1), "fit": "fit"}),
    );
    assert_eq!(live.content(live.at(1))["background"]["fit"], "fit");
    live.ok(
        "set_background",
        json!({"target": live.page(1), "background": null}),
    );
    assert!(live.content(live.at(1))["background"].is_null());
    live.ok("remove_page", json!({"target": live.page(2)}));
    assert_eq!(live.page_count(), 2);
    // The first page is a page of the pages read, with its identity.
    live.ok("set_first_page", json!({"target": live.page(1)}));
    let profiles = live.read("profiles");
    assert_eq!(profiles["profiles"][0]["first_page"], live.id(1));
    assert_eq!(live.read("pages")["first"], live.id(1));
}

#[test]
fn folder_requests_need_no_page_and_add_rename_move_open_close_and_remove() {
    let mut live = Live::new();
    let profile = live.active.clone();
    let made = live.made(
        "add_folder",
        json!({"profile": profile, "id": "00000000000000a1", "name": "First", "copy_of": null}),
    );
    assert_eq!(
        made,
        json!({"profile": profile, "page": null, "folder": "00000000000000a1"})
    );
    // Without an identity, OpenCraft makes one, and the result has it.
    let made = live.made("add_folder", json!({"profile": profile, "name": "Other"}));
    let other = made["folder"].as_str().unwrap().to_string();
    let folders = live.read("folders");
    assert_eq!(folders.as_array().unwrap().len(), 2);
    assert_eq!(folders[1]["id"], other);
    // The page that the device shows stays: a folder is a part of the profile.
    let page = live.active_page();
    live.ok(
        "rename_folder",
        json!({"profile": profile, "folder": "00000000000000a1", "name": "Renamed"}),
    );
    live.ok(
        "move_folder",
        json!({"profile": profile, "folder": "00000000000000a1", "to": 2}),
    );
    // The places count from 1, as the places of the pages.
    let (code, text) = live.fails(
        "move_folder",
        json!({"profile": profile, "folder": "00000000000000a1", "to": 0}),
    );
    assert_eq!(code, "invalid_params");
    assert!(text.contains("The number of the place"), "{text}");
    assert_eq!(live.active_page(), page);
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
               "surface": surface_1(), "label": "In"}),
    );
    let inside = live.content(live.at_in(0, "00000000000000a1"));
    assert!(inside["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["label"] == "In"));
    assert_eq!(live.label(0), Value::Null);
    live.ok(
        "close_folder",
        json!({"target": live.at_in(0, "00000000000000a1")}),
    );
    assert_eq!(live.read("pages")["location"]["open"], json!([]));
    // A folder that is not open cannot close: the request would look like a
    // success and change nothing.
    let (code, text) = live.fails(
        "close_folder",
        json!({"target": live.at_in(0, "00000000000000a1")}),
    );
    assert_eq!(code, "invalid_params");
    assert!(text.contains("not open"), "{text}");
    live.ok(
        "remove_folder",
        json!({"profile": profile, "folder": "00000000000000a1"}),
    );
    assert_eq!(live.read("folders").as_array().unwrap().len(), 1);
}

#[test]
fn profile_requests_add_rename_colour_link_export_import_select_and_remove() {
    let mut live = Live::new();
    let first = live.active.clone();
    // The result of the request has the identity of the new profile.
    let made = live.made("add_profile", json!({"name": "Second", "copy_of": null}));
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
    // The result gives the identity of the new profile and of its first page.
    assert_eq!(made["profile"], json!(second));
    assert_eq!(made["page"], live.read("pages")["first"]);
    assert_eq!(made["folder"], Value::Null);
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
    let first_page = live.read("profiles")["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == second)
        .unwrap()["pages"][0]["id"]
        .clone();
    live.ok(
        "set_first_page",
        json!({"target": {"profile": second, "page": first_page}}),
    );
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
    // An import makes a profile, and the result has its identity.
    let made = live.made("import_profile", json!({"path": export}));
    let profiles = live.read("profiles");
    assert_eq!(profiles["profiles"].as_array().unwrap().len(), 3);
    let imported = made["profile"].as_str().unwrap();
    assert_eq!(profiles["active"], imported);
    assert!(imported != second && imported != first);
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
    assert!(settings["knob_lights"]["mode"].is_string());
}

#[test]
fn one_request_is_one_step_of_the_history_with_the_name_of_the_window_and_the_window_can_undo_it() {
    let mut live = Live::new();
    let surface = surface_1();
    live.ok(
        "set_label_style",
        json!({"target": live.at(0), "surface": surface,
               "style": {"position": "top", "size": 20, "color": "#ff0000", "bold": false}}),
    );
    // The four changes of the style are one step.
    assert_eq!(live.read("history")["undo"], "Set Title Style");
    live.ok(
        "set_label",
        json!({"target": live.at(0), "surface": surface, "label": "Window"}),
    );
    // The name is the name that the window uses for the same change.
    assert_eq!(live.read("history")["undo"], "Set Title");
    live.ok(
        "set_color",
        json!({"target": live.at(0), "surface": surface, "color": "#336699"}),
    );
    assert_eq!(live.read("history")["undo"], "Set Fill");
    live.ok(
        "copy_to_all_pages",
        json!({"target": live.at(0), "control": {"kind": "key", "number": 1}}),
    );
    // The window sends Undo with no reply.
    live.engine.as_ref().unwrap().send(e::Command::Undo);
    let start = Instant::now();
    while live.read("history")["undo"] == "Set Fill" {
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(10));
    }
    live.engine.as_ref().unwrap().send(e::Command::Undo);
    let start = Instant::now();
    while live.read("history")["undo"] != "Set Title Style" {
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
    // The client can undo and redo, too. The result gives the place of the
    // step that it reversed, not only the profile.
    let made = live.made("undo", json!({"profile": live.active}));
    assert_eq!(made["profile"], json!(live.active));
    assert_eq!(made["page"], live.active_page());
    assert_eq!(live.read("history")["undo"], Value::Null);
    let made = live.made("redo", json!({"profile": live.active}));
    assert_eq!(made["page"], live.active_page());
    assert_eq!(live.read("history")["undo"], "Set Title Style");
}

#[test]
fn a_response_comes_after_the_events_of_its_change() {
    let mut live = Live::new();
    let surface = surface_1();
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
        assert_eq!(live.read("history")["undo"], "Set Title");
    }
}

#[test]
fn a_request_for_a_place_that_is_gone_changes_nothing() {
    let mut live = Live::new();
    let profile = live.active.clone();
    live.made("add_page", json!({"profile": profile}));
    live.ok("select_page", json!({"target": live.page(1)}));
    let surface = surface_1();
    let history = live.read("history");
    let at_gone =
        |live: &Live| json!({"profile": live.active, "page": "00000000000000ee", "folder": null});
    // The cache has no such page, or no such profile: the request is invalid.
    for target in [
        at_gone(&live),
        json!({"profile": "not-a-profile", "page": live.id(0), "folder": null}),
    ] {
        let (code, text) = live.fails(
            "set_label",
            json!({"target": target, "surface": surface, "label": "No"}),
        );
        assert_eq!(code, "invalid_params");
        assert!(text.contains("does not exist"), "{text}");
    }
    // The engine checks the folder.
    let (code, text) = live.fails(
        "set_label",
        json!({"target": live.at_in(0, "00000000000000ee"), "surface": surface, "label": "No"}),
    );
    assert_eq!(code, "engine_problem");
    assert!(!text.is_empty());
    // The page that was active stays active, and nothing has the label.
    assert_eq!(live.active_page(), live.id(1));
    assert_eq!(live.read("history"), history);
    assert!(live.label(0).is_null());
    assert!(live.label(1).is_null());
    // A request that changes the structure also changes nothing.
    let (code, _) = live.fails(
        "rename_page",
        json!({"target": {"profile": live.active, "page": "00000000000000ee"}, "name": "No"}),
    );
    assert_eq!(code, "invalid_params");
    assert_eq!(live.page_count(), 2);
}

#[test]
fn a_failed_request_for_a_profile_that_is_not_active_changes_nothing_also_the_active_profile() {
    let mut live = Live::new();
    let first = live.active.clone();
    live.made("add_profile", json!({"name": "Second", "copy_of": null}));
    let profiles = live.read("profiles");
    let second = profiles["active"].as_str().unwrap().to_string();
    let second_page = profiles["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == second)
        .unwrap()["pages"][0]["id"]
        .clone();
    live.ok("select_profile", json!({"profile": first}));
    let history = live.read("history");
    let pages = live.read("pages");
    // A folder that the other profile does not have: the engine refuses it
    // before it makes the profile active.
    let (code, _) = live.fails(
        "set_label",
        json!({"target": {"profile": second, "page": second_page, "folder": "00000000000000ee"},
               "surface": surface_1(), "label": "No"}),
    );
    assert_eq!(code, "engine_problem");
    assert_eq!(live.read("profiles")["active"], first);
    assert_eq!(live.read("pages"), pages);
    assert_eq!(live.read("history"), history);
    // A change that the engine refuses after the check goes back, too.
    let (code, _) = live.fails(
        "set_image",
        json!({"target": {"profile": second, "page": second_page, "folder": null},
               "surface": surface_1(), "path": "/this/file/is/not/there.png"}),
    );
    assert_eq!(code, "engine_problem");
    assert_eq!(live.read("profiles")["active"], first);
    assert_eq!(live.read("pages")["location"], pages["location"]);
    assert_eq!(live.read("history"), history);
    // A good request for the other profile is one request.
    live.ok(
        "set_label",
        json!({"target": {"profile": second, "page": second_page, "folder": null},
               "surface": surface_1(), "label": "Other"}),
    );
    assert_eq!(live.read("profiles")["active"], second);
    assert_eq!(live.read("history")["undo"], "Set Title");
}

#[test]
fn a_problem_of_the_engine_is_the_error_of_the_request_with_its_text() {
    let mut live = Live::new();
    let profile = live.active.clone();
    let (code, text) = live.fails("rename_page", json!({"target": live.page(0), "name": ""}));
    assert_eq!(code, "engine_problem");
    assert_eq!(text, "A page must have a name.");
    let (_, text) = live.fails("remove_page", json!({"target": live.page(0)}));
    assert_eq!(text, "A profile keeps one page at the minimum.");
    let (_, text) = live.fails("add_profile", json!({"name": "  ", "copy_of": null}));
    assert!(text.contains("name"), "{text}");
    // A problem of a request is not left for the next request.
    live.made("add_page", json!({"profile": profile}));
}

#[test]
fn a_relative_file_path_gives_an_error_and_changes_nothing() {
    let mut live = Live::new();
    let surface = surface_1();
    for (method, params) in [
        (
            "set_image",
            json!({"target": live.at(0), "surface": surface, "path": "picture.png"}),
        ),
        (
            "set_background",
            json!({"target": live.page(0),
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
    let first_page = live.id(0);
    let surface = surface_1();
    live.ok(
        "set_label",
        json!({"target": live.at(0), "surface": surface, "label": "First profile"}),
    );
    live.ok(
        "set_binding",
        json!({"target": live.at(0), "control": {"kind": "key", "number": 1},
               "gesture": "press", "action": {"kind": "test", "settings": {}}}),
    );
    live.made("add_profile", json!({"name": "Second", "copy_of": null}));
    assert_ne!(live.read("profiles")["active"], first);
    let content = live.content(json!({"profile": first, "page": first_page, "folder": null}));
    assert_eq!(content["profile"], first);
    assert_eq!(content["location"]["page"], first_page);
    assert!(content["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["label"] == "First profile"));
    assert_eq!(content["bindings"].as_array().unwrap().len(), 1);
    assert_eq!(content["bindings"][0]["page"], first_page);
    // The active profile did not change, and the read made no step.
    assert_ne!(live.read("profiles")["active"], first);
    let reply = live.call(
        "content",
        json!({"target": {"profile": first, "page": "00000000000000ee", "folder": null}}),
    );
    assert_eq!(reply["error"]["code"], "engine_problem");
    let reply = live.call(
        "content",
        json!({"target": {"profile": "not-a-profile", "page": first_page, "folder": null}}),
    );
    assert_eq!(reply["error"]["code"], "engine_problem");
}

#[test]
fn pages_and_folders_read_a_profile_that_is_not_active_and_select_nothing() {
    let mut live = Live::new();
    let first = live.active.clone();
    live.made("add_page", json!({"profile": first}));
    live.made(
        "add_folder",
        json!({"profile": first, "id": "00000000000000a1", "name": "Tools", "copy_of": null}),
    );
    let first_pages = live.read("pages");
    live.made("add_profile", json!({"name": "Second", "copy_of": null}));
    let second = live.read("profiles")["active"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(second, first);

    // No profile and the active profile give the same result, with a location.
    assert_eq!(
        live.call("pages", json!({"profile": second}))["result"],
        live.read("pages")
    );
    assert_eq!(live.read("pages")["profile"], second);
    assert!(live.read("pages")["location"].is_object());

    // A profile that is not active gives its pages and its folders, and no
    // location. The active profile does not change.
    let reply = live.call("pages", json!({"profile": first}));
    let pages = &reply["result"];
    assert_eq!(pages["profile"], first);
    assert_eq!(pages["pages"], first_pages["pages"]);
    assert_eq!(pages["first"], first_pages["first"]);
    assert!(pages["location"].is_null());
    assert_eq!(pages["folders"][0]["id"], "00000000000000a1");
    let reply = live.call("folders", json!({"profile": first}));
    assert_eq!(reply["result"][0]["name"], "Tools");
    assert_eq!(live.read("profiles")["active"], second);
    assert_eq!(live.read("folders").as_array().unwrap().len(), 0);

    let (code, _) = live.fails("pages", json!({"profile": "not-a-profile"}));
    assert_eq!(code, "invalid_params");
    let (code, _) = live.fails("folders", json!({"profile": "not-a-profile"}));
    assert_eq!(code, "invalid_params");
}

#[test]
fn a_request_with_a_wrong_surface_or_value_is_invalid_and_sends_no_command() {
    let mut live = Live::new();
    for (method, params, code) in [
        (
            "set_label",
            json!({"target": live.at(0), "surface": {"kind": "key_face", "number": 99}, "label": "x"}),
            "invalid_params",
        ),
        (
            "set_color",
            json!({"target": live.at(0), "surface": {"kind": "key_face", "number": 1}, "color": "red"}),
            "invalid_params",
        ),
        // A text that is not one of the values of an enum.
        (
            "set_binding",
            json!({"target": live.at(0), "control": {"kind": "key", "number": 1},
                   "gesture": "wave", "action": null}),
            "invalid_request",
        ),
        (
            "set_label",
            json!({"target": live.at(0), "surface": {"kind": "key face", "number": 1}, "label": "x"}),
            "invalid_request",
        ),
        (
            "set_background_fit",
            json!({"target": live.page(0), "fit": "sideways"}),
            "invalid_request",
        ),
        (
            "open_folder",
            json!({"target": live.at(0), "folder": "not hex"}),
            "invalid_params",
        ),
    ] {
        let (found, _) = live.fails(method, params);
        assert_eq!(found, code, "{method}");
    }
    assert!(live.read("history")["undo"].is_null());
}

#[test]
fn a_history_read_names_the_profile_and_an_empty_history_has_its_own_code() {
    let mut live = Live::new();
    let profile = live.active.clone();
    let history = |live: &mut Live, profile: &str| {
        let reply = live.call("history", json!({"profile": profile}));
        assert!(reply.get("error").is_none(), "{reply}");
        reply["result"].clone()
    };
    // A new profile has no step.
    let read = history(&mut live, &profile);
    assert_eq!(read["profile"], json!(profile));
    assert!(read["undo"].is_null());
    assert!(read["redo"].is_null());
    // A correct request with nothing to reverse has its own code: a client
    // can tell it from a request that is not correct.
    let (code, text) = live.fails("undo", json!({"profile": profile}));
    assert_eq!(code, "nothing_to_undo");
    assert_eq!(text, "There is no step to undo.");
    let (code, text) = live.fails("redo", json!({"profile": profile}));
    assert_eq!(code, "nothing_to_redo");
    assert_eq!(text, "There is no step to do again.");

    live.ok(
        "set_label",
        json!({"target": live.at(0), "surface": surface_1(), "label": "One"}),
    );
    assert_eq!(history(&mut live, &profile)["undo"], "Set Title");
    // A profile that is not active keeps the history that OpenCraft kept for
    // it: it cannot change while the profile is not active.
    let made = live.made("add_profile", json!({"name": "Second", "copy_of": null}));
    let second = made["profile"].as_str().unwrap().to_string();
    let read = history(&mut live, &second);
    assert_eq!(read["profile"], json!(second));
    assert!(read["undo"].is_null());
    let read = history(&mut live, &profile);
    assert_eq!(read["profile"], json!(profile));
    assert_eq!(read["undo"], "Set Title");
    // A read with no profile is the active profile.
    let reply = live.call("history", json!({}));
    assert_eq!(reply["result"]["profile"], json!(second));
    // An identity that does not exist is not a read with no steps.
    let (code, _) = live.fails("history", json!({"profile": "nothing"}));
    assert_eq!(code, "invalid_params");
}

#[test]
fn an_undo_for_a_profile_that_is_not_active_is_refused_and_changes_nothing() {
    let mut live = Live::new();
    let first = live.active.clone();
    live.ok(
        "set_label",
        json!({"target": live.at(0), "surface": surface_1(), "label": "Mine"}),
    );
    let made = live.made("add_profile", json!({"name": "Second", "copy_of": null}));
    let second = made["profile"].as_str().unwrap().to_string();
    live.ok("select_profile", json!({"profile": first}));
    // The history belongs to the profile that the user sees.
    let (code, text) = live.fails("undo", json!({"profile": second}));
    assert_eq!(code, "engine_problem");
    assert!(text.contains(&format!("“{first}”")), "{text}");
    assert_eq!(live.read("profiles")["active"], first);
    assert_eq!(live.read("history")["undo"], "Set Title");
    assert_eq!(live.label(0), "Mine");
}

#[test]
fn a_binding_needs_a_known_kind_known_fields_and_values_that_the_kind_accepts() {
    let mut live = Live::new();
    let key1 = json!({"kind": "key", "number": 1});
    let at = live.at(0);
    let set = |action: Value| json!({"target": at, "control": key1, "gesture": "press", "action": action});
    let refused = [
        // An unknown kind: the short name of a built-in kind.
        (json!({"kind": "hotkey", "settings": {}}), "hotkey"),
        // An unknown field.
        (json!({"kind": "test", "settings": {"b": "x"}}), "\"b\""),
        // A hotkey that the kind refuses.
        (
            json!({"kind": "test.keys", "settings": {"hotkey": "cmd+bogus"}}),
            "hotkey",
        ),
        (
            json!({"kind": "test.keys", "settings": {"step": 10}}),
            "step",
        ),
        (
            json!({"kind": "test.keys", "settings": {"mode": "down"}}),
            "mode",
        ),
        (json!({"kind": "test", "settings": {"a": 1}}), "a"),
        // A step of a list: an unknown kind, and a delay out of range.
        (
            json!({"kind": "test.keys", "settings": {"steps": [{"kind": "nope"}]}}),
            "nope",
        ),
        (
            json!({"kind": "test.keys", "settings": {"steps": [
                {"kind": "test", "settings": {"delay_before_ms": -1}}]}}),
            "delay_before_ms",
        ),
        // The limit of the delay is the limit of the engine, not a copy.
        (
            json!({"kind": "test.keys", "settings": {"steps": [
                {"kind": "test", "settings": {"delay_before_ms": 10001}}]}}),
            "delay_before_ms",
        ),
        // A field that the form hides for this value of the choice.
        (
            json!({"kind": "test.shown", "settings": {"operation": "down", "step": 1}}),
            "step",
        ),
        // A value that is not a setting: the text names the setting.
        (
            json!({"kind": "test", "settings": {"a": null}}),
            "The setting \"a\" is not correct",
        ),
        (
            json!({"kind": "test.keys", "settings": {"step": 1.5}}),
            "The setting \"step\" is not correct",
        ),
    ];
    for (action, word) in refused {
        let (code, text) = live.fails("set_binding", set(action.clone()));
        assert_eq!(code, "invalid_params", "{action}");
        assert!(text.contains(word), "{action}: {text}");
    }
    // Nothing changed.
    assert!(live.read("history")["undo"].is_null());
    assert!(live.content(live.at(0))["bindings"]
        .as_array()
        .unwrap()
        .is_empty());
    // A correct binding still works, with the largest delay of the engine.
    live.ok(
        "set_binding",
        set(json!({"kind": "test.keys", "settings": {
            "hotkey": "shift+cmd+4", "step": 5, "mode": "up",
            "steps": [{"kind": "test", "settings": {"a": "b", "delay_before_ms": 10000}}]}})),
    );
    assert_eq!(
        live.content(live.at(0))["bindings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // A field that the form shows for the default of the choice is accepted.
    live.ok(
        "set_binding",
        set(json!({"kind": "test.shown", "settings": {"step": 5}})),
    );
    live.ok(
        "set_binding",
        set(json!({"kind": "test.shown", "settings": {"operation": "up", "step": 5}})),
    );
}

#[test]
fn a_profile_setting_and_a_folder_setting_name_a_thing_that_exists() {
    let mut live = Live::new();
    let profile = live.active.clone();
    let key1 = json!({"kind": "key", "number": 1});
    let at = live.at(0);
    let set = |action: Value| json!({"target": at, "control": key1, "gesture": "press", "action": action});
    let folder = "00000000000000a1";
    live.made(
        "add_folder",
        json!({"profile": profile, "id": folder, "name": "Tools", "copy_of": null}),
    );
    // The window shows only the profiles and the folders that exist: a name
    // gives a fault only when a person pushes the key.
    for (action, word) in [
        // The name of the profile, not its identity.
        (
            json!({"kind": "test", "settings": {"profile": "Default"}}),
            "profile",
        ),
        // The name of the folder, not its identity.
        (
            json!({"kind": "test", "settings": {"folder": "Tools"}}),
            "folder",
        ),
        // The form of an identity, and no such folder.
        (
            json!({"kind": "test", "settings": {"folder": "00000000000000ff"}}),
            "folder",
        ),
    ] {
        let (code, text) = live.fails("set_binding", set(action.clone()));
        assert_eq!(code, "invalid_params", "{action}");
        assert!(text.contains(word), "{action}: {text}");
    }
    // The identities of the profile and of the folder are correct.
    live.ok(
        "set_binding",
        set(json!({"kind": "test", "settings": {"profile": profile, "folder": folder}})),
    );
    // A folder of a profile that is not active is not in the cache: the
    // engine checks it at the place.
    live.made("add_profile", json!({"name": "Second", "copy_of": null}));
    let second = live.active.clone();
    live.ok("select_profile", json!({"profile": profile}));
    live.ok(
        "set_binding",
        json!({"target": json!({"profile": second, "page": live.id(0)}),
               "control": key1, "gesture": "press",
               "action": {"kind": "test", "settings": {"folder": "00000000000000a1"}}}),
    );
}
