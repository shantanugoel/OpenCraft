//! History tests: the undo and the redo of each change to the pages, in
//! memory and in the profile file (decision 0030). The tests use the fake
//! transport and no thread.

use super::*;
use opencraft_engine::Page;

const KIND: &str = "test.history";

/// The binding of the press of key 3 on `page`.
fn key_3(page: usize) -> BindingId {
    BindingId {
        page,
        ..binding(ControlKind::Key, 3, GestureKind::Press)
    }
}

fn other_action() -> Action {
    Action {
        kind: KIND.into(),
        settings: Settings::from([("text".into(), Value::Text("other".into()))]),
    }
}

/// An engine on a saved profile with the pages "A" and "B", and the
/// requests of its one handler. Page A has red on key face 3 and a binding
/// for the press of key 3. Page B has blue on key face 3. The events and the
/// requests of the start are gone.
fn rig() -> (Rig, mpsc::Receiver<Request>) {
    let dir = TempDir::new().unwrap();
    let mut a = Page::new("A");
    a.update(KEY_3, |c| c.color = Some(RED));
    let id = key_3(0);
    a.set_binding(id.control, id.gesture, Some(action(KIND)));
    let mut b = Page::new("B");
    b.update(KEY_3, |c| c.color = Some(BLUE));
    store_in(dir.path())
        .save(&Profile::new("History", vec![a, b]))
        .unwrap();
    let (port, requests) = port(KIND);
    let mut rig = Rig::with_ports(dir, vec![port]);
    rig.take_events();
    requests.try_iter().count();
    (rig, requests)
}

fn command(rig: &mut Rig, command: Command) {
    assert!(rig.engine.command(command));
}

type Names = (Option<String>, Option<String>);

fn names(undo: Option<&str>, redo: Option<&str>) -> Names {
    (undo.map(str::to_string), redo.map(str::to_string))
}

/// The names of each history event in `events`, in order.
fn history_events(events: &[Event]) -> Vec<Names> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::History { undo, redo } => Some((undo.clone(), redo.clone())),
            _ => None,
        })
        .collect()
}

/// The names of the last history event since the last call.
fn last_names(rig: &mut Rig) -> Option<Names> {
    history_events(&rig.take_events()).pop()
}

/// The profile in the file, after the delay of the write.
fn saved(rig: &mut Rig) -> Profile {
    rig.poll_for(SAVE_DELAY + POLL_INTERVAL);
    store_in(rig.dir.path()).read().unwrap()
}

fn label(text: &str) -> Command {
    Command::SetLabel {
        surface: KEY_3,
        label: Some(text.into()),
    }
}

/// Do `change` on the page `page`, then check the profile in memory and in
/// the file after the change, after the undo, and after the redo. Check the
/// active page and the names of the steps also.
fn check(page: usize, change: Command, name: &str) {
    let (mut rig, _requests) = rig();
    command(&mut rig, Command::SelectPage { page });
    rig.take_events();
    let before = rig.engine.profile().clone();
    command(&mut rig, change);
    let after = rig.engine.profile().clone();
    let page_after = rig.engine.active_page();
    assert_ne!(before, after, "{name} changes the profile");
    assert_eq!(last_names(&mut rig), Some(names(Some(name), None)));
    assert_eq!(saved(&mut rig), after, "{name}: the file after the change");

    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.profile(), &before, "{name}: undo");
    assert_eq!(rig.engine.active_page(), page, "{name}: the page of undo");
    assert_eq!(last_names(&mut rig), Some(names(None, Some(name))));
    assert_eq!(saved(&mut rig), before, "{name}: the file after the undo");

    command(&mut rig, Command::Redo);
    assert_eq!(rig.engine.profile(), &after, "{name}: redo");
    assert_eq!(
        rig.engine.active_page(),
        page_after,
        "{name}: the page of redo"
    );
    assert_eq!(last_names(&mut rig), Some(names(Some(name), None)));
    assert_eq!(saved(&mut rig), after, "{name}: the file after the redo");
}

#[test]
fn each_change_of_the_pages_is_undone_and_done_again_in_memory_and_in_the_file() {
    let image = TempDir::new().unwrap();
    let file = green_file(image.path());
    let cases = [
        (1, label("Mail"), "Set Title"),
        (
            0,
            Command::SetColor {
                surface: KEY_3,
                color: Some(BLUE),
            },
            "Set Fill",
        ),
        (
            0,
            Command::SetColor {
                surface: KEY_3,
                color: None,
            },
            "Set Fill",
        ),
        (
            1,
            Command::SetImage {
                surface: KEY_3,
                path: Some(file),
            },
            "Set Image",
        ),
        (
            0,
            Command::SetBinding {
                binding: key_3(0),
                action: Some(other_action()),
            },
            "Set Action",
        ),
        (
            0,
            Command::SetBinding {
                binding: key_3(0),
                action: None,
            },
            "Remove the Action",
        ),
        (0, Command::AddPage { copy_of: None }, "Add Page"),
        (1, Command::AddPage { copy_of: Some(0) }, "Duplicate Page"),
        (
            0,
            Command::RenamePage {
                page: 1,
                name: "Media".into(),
            },
            "Rename Page",
        ),
        (1, Command::MovePage { page: 1, to: 0 }, "Move Page"),
        (0, Command::MovePage { page: 1, to: 0 }, "Move Page"),
        (0, Command::RemovePage { page: 0 }, "Remove Page"),
        (0, Command::RemovePage { page: 1 }, "Remove Page"),
        (1, Command::RemovePage { page: 1 }, "Remove Page"),
        (
            0,
            Command::Group {
                name: "Drop Action".into(),
                commands: vec![
                    Command::SetBinding {
                        binding: key_3(0),
                        action: Some(other_action()),
                    },
                    Command::SetColor {
                        surface: KEY_3,
                        color: None,
                    },
                ],
            },
            "Drop Action",
        ),
    ];
    for (page, change, name) in cases {
        check(page, change, name);
    }
}

#[test]
fn the_undo_of_an_image_shows_the_old_image_and_the_redo_the_new_image() {
    let (mut rig, _requests) = rig();
    let device = rig.connect();
    sent(&device);
    rig.take_events();
    let file = green_file(rig.dir.path());
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: Some(file.clone()),
        },
    );
    fs::remove_file(&file).unwrap();
    assert_color(&shown_at(&sent(&device), KEY_3_SLOT), 56, 56, GREEN);

    command(&mut rig, Command::Undo);
    let events = rig.take_events();
    assert_eq!(last_state(&events, KEY_3).content.image, None);
    assert_color(&shown_at(&sent(&device), KEY_3_SLOT), 56, 56, RED);

    // The copy of the image stays in the profile until the next load, so
    // the redo shows it with no original file.
    command(&mut rig, Command::Redo);
    let events = rig.take_events();
    assert!(last_state(&events, KEY_3).content.image.is_some());
    assert!(problems(&events).is_empty(), "{events:?}");
    assert_color(&shown_at(&sent(&device), KEY_3_SLOT), 56, 56, GREEN);
}

#[test]
fn undo_shows_the_page_of_the_change_and_redo_the_page_after_it() {
    let (mut rig, requests) = rig();
    command(&mut rig, Command::SelectPage { page: 1 });
    command(&mut rig, label("Mail"));
    command(&mut rig, Command::SelectPage { page: 0 });
    rig.take_events();
    requests.try_iter().count();

    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.active_page(), 1);
    assert_eq!(rig.engine.profile().pages()[1].content(KEY_3).label, None);
    let events = rig.take_events();
    assert!(events.contains(&Event::Pages {
        names: vec!["A".into(), "B".into()],
        active: 1,
        first: 0,
    }));
    // The steps of a page change: the handler of page A gets Disappear.
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [Request::Disappear { binding: key_3(0) }]
    );

    command(&mut rig, Command::SelectPage { page: 0 });
    command(&mut rig, Command::Redo);
    assert_eq!(rig.engine.active_page(), 1);
    assert_eq!(
        rig.engine.profile().pages()[1]
            .content(KEY_3)
            .label
            .as_deref(),
        Some("Mail")
    );
}

#[test]
fn undo_of_a_binding_gives_the_old_action_to_the_handler_and_the_shell() {
    let (mut rig, requests) = rig();
    command(
        &mut rig,
        Command::SetBinding {
            binding: key_3(0),
            action: Some(other_action()),
        },
    );
    rig.take_events();
    requests.try_iter().count();

    command(&mut rig, Command::Undo);
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [
            Request::Disappear { binding: key_3(0) },
            Request::Appear {
                binding: key_3(0),
                action: action(KIND)
            }
        ]
    );
    assert!(rig.take_events().contains(&Event::Binding(BindingState {
        binding: key_3(0),
        action: Some(action(KIND)),
        available: true,
    })));
}

#[test]
fn the_changes_of_one_target_in_one_text_edit_are_one_step() {
    let (mut rig, _requests) = rig();
    command(&mut rig, Command::StartTextEdit);
    for text in ["M", "Ma", "Mai", "Mail"] {
        command(&mut rig, label(text));
    }
    command(&mut rig, Command::EndTextEdit);
    // One event for the new step, and none for the changes that joined it.
    assert_eq!(
        history_events(&rig.take_events()),
        [names(Some("Set Title"), None)]
    );
    assert_eq!(
        saved(&mut rig).pages()[0].content(KEY_3).label.as_deref(),
        Some("Mail")
    );

    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.profile().pages()[0].content(KEY_3).label, None);
    assert_eq!(last_names(&mut rig), Some(names(None, Some("Set Title"))));
    assert_eq!(saved(&mut rig).pages()[0].content(KEY_3).label, None);

    // The text of a binding setting is one step also.
    let typed = |text: &str| {
        let mut action = action(KIND);
        action
            .settings
            .insert("text".into(), Value::Text(text.into()));
        Command::SetBinding {
            binding: key_3(0),
            action: Some(action),
        }
    };
    command(&mut rig, Command::StartTextEdit);
    command(&mut rig, typed("u"));
    command(&mut rig, typed("ur"));
    command(&mut rig, typed("url"));
    command(&mut rig, Command::EndTextEdit);
    command(&mut rig, Command::Undo);
    assert_eq!(
        rig.engine.profile().pages()[0].binding(key_3(0).control, key_3(0).gesture),
        Some(&action(KIND))
    );
    assert_eq!(last_names(&mut rig), Some(names(None, Some("Set Action"))));
}

#[test]
fn the_groups_of_two_bindings_in_one_text_edit_are_one_step() {
    // The "Turn" card of a knob writes two bindings for each change.
    let (mut rig, _requests) = rig();
    let left = binding(ControlKind::Knob, 1, GestureKind::TurnLeft);
    let right = binding(ControlKind::Knob, 1, GestureKind::TurnRight);
    let typed = |text: &str| {
        let mut action = action(KIND);
        action
            .settings
            .insert("text".into(), Value::Text(text.into()));
        Command::Group {
            name: "Set Action".into(),
            commands: vec![
                Command::SetBinding {
                    binding: right,
                    action: Some(action.clone()),
                },
                Command::SetBinding {
                    binding: left,
                    action: Some(action),
                },
            ],
        }
    };
    command(&mut rig, typed("a"));
    command(&mut rig, Command::StartTextEdit);
    for text in ["a1", "a12", "a123"] {
        command(&mut rig, typed(text));
    }
    command(&mut rig, Command::EndTextEdit);
    let text = |rig: &Rig, id: BindingId| {
        rig.engine.profile().pages()[0]
            .binding(id.control, id.gesture)
            .and_then(|a| a.settings.get("text").cloned())
    };
    assert_eq!(text(&rig, left), Some(Value::Text("a123".into())));
    // One undo removes the typed text from the two bindings.
    command(&mut rig, Command::Undo);
    assert_eq!(text(&rig, left), Some(Value::Text("a".into())));
    assert_eq!(text(&rig, right), Some(Value::Text("a".into())));
    // The first group is a step of its own: no text edit was open.
    command(&mut rig, Command::Undo);
    assert_eq!(text(&rig, left), None);
    assert_eq!(text(&rig, right), None);
    assert_eq!(last_names(&mut rig), Some(names(None, Some("Set Action"))));
}

#[test]
fn a_different_target_a_new_text_edit_or_no_text_edit_starts_a_new_step() {
    let (mut rig, _requests) = rig();
    // With no text edit, each change is one step.
    command(&mut rig, label("a"));
    command(&mut rig, label("ab"));
    // In one text edit, a different surface is a different step.
    command(&mut rig, Command::StartTextEdit);
    command(&mut rig, label("abc"));
    command(
        &mut rig,
        Command::SetLabel {
            surface: BAR_2,
            label: Some("x".into()),
        },
    );
    command(&mut rig, label("abcd"));
    // Return ends the text edit, and the shell starts the next one.
    command(&mut rig, Command::EndTextEdit);
    command(&mut rig, Command::StartTextEdit);
    command(&mut rig, label("abcde"));
    command(&mut rig, Command::EndTextEdit);

    // Six steps. Each undo shows the labels of key face 3 and bar panel 2
    // before one step.
    let expected = [
        (Some("abcd"), Some("x")),
        (Some("abc"), Some("x")),
        (Some("abc"), None),
        (Some("ab"), None),
        (Some("a"), None),
        (None, None),
    ];
    for (key, bar) in expected {
        command(&mut rig, Command::Undo);
        let page = &rig.engine.profile().pages()[0];
        assert_eq!(page.content(KEY_3).label.as_deref(), key);
        assert_eq!(page.content(BAR_2).label.as_deref(), bar);
    }
    assert_eq!(last_names(&mut rig), Some(names(None, Some("Set Title"))));
}

#[test]
fn a_text_that_the_user_types_and_removes_in_one_text_edit_leaves_no_step() {
    let (mut rig, _requests) = rig();
    command(&mut rig, Command::StartTextEdit);
    command(&mut rig, label("x"));
    command(
        &mut rig,
        Command::SetLabel {
            surface: KEY_3,
            label: None,
        },
    );
    command(&mut rig, Command::EndTextEdit);
    assert_eq!(
        history_events(&rig.take_events()),
        [names(Some("Set Title"), None), names(None, None)]
    );
}

#[test]
fn a_new_step_empties_the_redo_list() {
    let (mut rig, _requests) = rig();
    command(&mut rig, label("a"));
    command(&mut rig, Command::Undo);
    assert_eq!(last_names(&mut rig), Some(names(None, Some("Set Title"))));
    command(&mut rig, Command::AddPage { copy_of: None });
    assert_eq!(last_names(&mut rig), Some(names(Some("Add Page"), None)));
    command(&mut rig, Command::Redo);
    assert!(rig.take_events().is_empty(), "the redo list is empty");
}

#[test]
fn a_command_that_changes_no_page_makes_no_step() {
    let (mut rig, _requests) = rig();
    // The same colour, a page change, a binding of a page that the device
    // does not show, a page name with only spaces, and the device settings.
    rig.set_color(KEY_3, RED);
    command(&mut rig, Command::SelectPage { page: 1 });
    command(
        &mut rig,
        Command::SetBinding {
            binding: key_3(0),
            action: None,
        },
    );
    command(
        &mut rig,
        Command::RenamePage {
            page: 0,
            name: " ".into(),
        },
    );
    let mut settings = rig.engine.device_settings().clone();
    settings.brightness = 10;
    command(&mut rig, Command::SetDeviceSettings(settings));
    command(
        &mut rig,
        Command::Group {
            name: "Nothing".into(),
            commands: vec![Command::Undo, Command::ReadCounters],
        },
    );
    // A test of an action runs a binding and changes no page.
    command(&mut rig, Command::TestAction { binding: key_3(1) });
    command(&mut rig, Command::SelectPage { page: 0 });
    command(&mut rig, Command::TestAction { binding: key_3(0) });
    assert!(history_events(&rig.take_events()).is_empty());
}

#[test]
fn undo_and_redo_with_an_empty_list_do_nothing() {
    let (mut rig, requests) = rig();
    command(&mut rig, Command::Undo);
    command(&mut rig, Command::Redo);
    assert!(rig.take_events().is_empty());
    assert!(requests.try_recv().is_err());
    rig.poll_for(SAVE_DELAY + POLL_INTERVAL);
    assert_eq!(store_in(rig.dir.path()).read().unwrap().pages().len(), 2);
}

#[test]
fn a_change_of_the_active_profile_empties_the_history() {
    let (mut rig, _requests) = rig();
    command(&mut rig, label("a"));
    command(
        &mut rig,
        Command::AddProfile {
            name: "Work".into(),
            copy_of: None,
        },
    );
    assert_eq!(last_names(&mut rig), Some(names(None, None)));
    command(
        &mut rig,
        Command::SelectProfile {
            profile: "default".into(),
        },
    );
    assert_eq!(last_names(&mut rig), Some(names(None, None)));
    command(&mut rig, Command::Undo);
    assert!(rig.take_events().is_empty());
    assert_eq!(
        rig.engine.profile().pages()[0]
            .content(KEY_3)
            .label
            .as_deref(),
        Some("a")
    );
}

#[test]
fn a_rename_of_the_active_profile_is_not_in_the_history_and_stays_after_an_undo() {
    let (mut rig, _requests) = rig();
    command(&mut rig, label("a"));
    command(
        &mut rig,
        Command::RenameProfile {
            profile: "default".into(),
            name: "Renamed".into(),
        },
    );
    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.profile().name, "Renamed");
    assert_eq!(saved(&mut rig).name, "Renamed");
}

/// The label options are a new part of the content of a surface. The
/// history covers them with no special code (decision 0030): each change is
/// one step, in memory and in the file.
#[test]
fn each_label_option_is_undone_and_done_again_in_memory_and_in_the_file() {
    use opencraft_engine::{LabelChange, LabelPosition};
    let style = |change| Command::SetLabelStyle {
        surface: KEY_3,
        change,
    };
    let cases = [
        (
            0,
            style(LabelChange::Position(Some(LabelPosition::Top))),
            "Set Title Position",
        ),
        (
            1,
            style(LabelChange::Position(Some(LabelPosition::Hidden))),
            "Set Title Position",
        ),
        (0, style(LabelChange::Size(14)), "Set Title Size"),
        (0, style(LabelChange::Color(Some(BLUE))), "Set Title Colour"),
        (1, style(LabelChange::Bold(false)), "Set Title Weight"),
    ];
    for (page, change, name) in cases {
        check(page, change, name);
    }
}
