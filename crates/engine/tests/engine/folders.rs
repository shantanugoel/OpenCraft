//! Folder tests: open, back, the page cycle, a profile change, a
//! reconnection, undo, and the migration to schema 5 (decision 0036).
//! The tests use the fake transport and no thread.

use super::pages::image_slots;
use super::*;
use opencraft_engine::{EngineRequest, FolderId, PageTarget};

const KIND: &str = "test.folders";
const A: FolderId = FolderId::from_raw(1).unwrap();
const B: FolderId = FolderId::from_raw(2).unwrap();
const GREEN: Color = Color::rgb(20, 180, 40);
const KEY_1: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 1);

fn command(rig: &mut Rig, command: Command) {
    assert!(rig.engine.command(command));
}

fn add(rig: &mut Rig, id: FolderId, name: Option<&str>) {
    command(
        rig,
        Command::AddFolder {
            id,
            name: name.map(str::to_string),
            copy_of: None,
        },
    );
}

fn open(rig: &mut Rig, id: FolderId) {
    rig.engine
        .handler_message(Message::Engine(EngineRequest::OpenFolder(id)));
}

fn back(rig: &mut Rig) {
    rig.engine
        .handler_message(Message::Engine(EngineRequest::CloseFolder));
}

fn key_binding(folder: Option<FolderId>) -> BindingId {
    BindingId {
        folder,
        ..binding(ControlKind::Key, 3, GestureKind::Press)
    }
}

/// The open folders of the last [`Event::Pages`].
fn open_of(events: &[Event]) -> Vec<FolderId> {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::Pages { open, .. } => Some(open.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no pages event in {events:?}"))
}

fn folder_names(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::Pages { folders, .. } => {
                Some(folders.iter().map(|folder| folder.name.clone()).collect())
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn history(events: &[Event]) -> (Option<String>, Option<String>) {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::History { undo, redo } => Some((undo.clone(), redo.clone())),
            _ => None,
        })
        .unwrap_or((None, None))
}

fn key_requests(receiver: &mpsc::Receiver<Request>) -> Vec<Request> {
    receiver.try_iter().collect()
}

/// A connected engine. Page key 3 is red, and bar panel 2 is blue.
fn painted() -> (Rig, Rc<RefCell<FakeTransport>>, mpsc::Receiver<Request>) {
    let (port, requests) = port(KIND);
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    let device = rig.connect();
    sent(&device);
    rig.take_events();
    key_requests(&requests);
    command(
        &mut rig,
        Command::SetBinding {
            binding: key_binding(None),
            action: Some(action(KIND)),
        },
    );
    let knob = binding(ControlKind::Knob, 1, GestureKind::Push);
    command(
        &mut rig,
        Command::SetBinding {
            binding: knob,
            action: Some(action(KIND)),
        },
    );
    rig.set_color(KEY_3, RED);
    rig.set_color(BAR_2, BLUE);
    sent(&device);
    rig.take_events();
    key_requests(&requests);
    (rig, device, requests)
}

#[test]
fn open_shows_the_folder_keys_and_keeps_the_knobs() {
    let (mut rig, device, requests) = painted();
    add(&mut rig, A, Some("Clips"));
    rig.set_color(KEY_3, GREEN);
    command(
        &mut rig,
        Command::SetBinding {
            binding: key_binding(Some(A)),
            action: Some(action(KIND)),
        },
    );
    back(&mut rig);
    sent(&device);
    rig.take_events();
    key_requests(&requests);

    open(&mut rig, A);
    let sent = sent(&device);
    assert!(
        image_slots(&sent).contains(&KEY_3_SLOT),
        "the key of the folder is written: {sent:?}"
    );
    assert!(
        !image_slots(&sent).contains(&2),
        "the bar panel of the page stays: {sent:?}"
    );
    assert_eq!(rig.engine.open_folders(), &[A]);
    assert_eq!(rig.engine.active_page(), 0);
    let events = rig.take_events();
    assert_eq!(open_of(&events), [A]);
    assert_eq!(folder_names(&events), ["Clips"]);
    assert_color(&last_state(&events, KEY_3).image, 2, 2, GREEN);
    let got = key_requests(&requests);
    assert!(
        got.iter().any(|request| matches!(
            request,
            Request::Disappear { binding } if binding.control.kind == ControlKind::Key
                && binding.folder.is_none()
        )),
        "{got:?}"
    );
    assert!(
        got.iter().all(|request| match request {
            Request::Disappear { binding } | Request::Appear { binding, .. } => {
                binding.control.kind == ControlKind::Key
            }
            _ => true,
        }),
        "a knob gets no Disappear and no Appear: {got:?}"
    );
    assert!(
        got.iter().any(|request| matches!(
            request,
            Request::Appear { binding, .. } if binding.folder == Some(A)
        )),
        "{got:?}"
    );

    // A press of key 3 is the binding of the folder.
    device.borrow_mut().push_input(control_report(0x03, 1));
    rig.engine.poll(POLL_INTERVAL);
    let requests = key_requests(&requests);
    assert!(
        requests.iter().any(|request| matches!(
            request,
            Request::Invoke { binding, .. } if *binding == key_binding(Some(A))
        )),
        "{requests:?}"
    );
}

#[test]
fn back_returns_to_the_keys_of_the_page() {
    let (mut rig, device, requests) = painted();
    add(&mut rig, A, Some("Clips"));
    rig.set_color(KEY_3, GREEN);
    sent(&device);
    rig.take_events();
    key_requests(&requests);

    back(&mut rig);
    let writes = sent(&device);
    assert!(image_slots(&writes).contains(&KEY_3_SLOT), "{writes:?}");
    assert!(!image_slots(&writes).contains(&2), "{writes:?}");
    assert!(rig.engine.open_folders().is_empty());
    let events = rig.take_events();
    assert!(open_of(&events).is_empty());
    assert_color(&last_state(&events, KEY_3).image, 2, 2, RED);
    assert!(
        key_requests(&requests).iter().any(|request| matches!(
            request,
            Request::Appear { binding, .. } if binding.folder.is_none()
                && binding.control.kind == ControlKind::Key
        )),
        "the key of the page appears again"
    );

    back(&mut rig);
    assert!(sent(&device).is_empty());
    assert!(rig.engine.open_folders().is_empty());
}

#[test]
fn a_nested_open_returns_to_the_earlier_folder() {
    let (mut rig, device, _) = painted();
    add(&mut rig, A, Some("Clips"));
    add(&mut rig, B, Some("Mics"));
    assert_eq!(rig.engine.open_folders(), &[A, B]);
    sent(&device);

    open(&mut rig, A);
    assert_eq!(rig.engine.open_folders(), &[A]);
    open(&mut rig, B);
    assert_eq!(rig.engine.open_folders(), &[A, B]);
    open(&mut rig, B);
    assert!(sent(&device).is_empty(), "the open folder does not change");
    open(&mut rig, A);
    assert_eq!(rig.engine.open_folders(), &[A]);
    back(&mut rig);
    assert!(rig.engine.open_folders().is_empty());
}

#[test]
fn a_missing_folder_gives_the_problem_and_changes_nothing() {
    let (mut rig, device, _) = painted();
    open(&mut rig, A);
    assert!(rig.engine.open_folders().is_empty());
    assert!(sent(&device).is_empty());
    assert_eq!(
        problems(&rig.take_events()),
        ["The folder of this key does not exist. Select a folder for the key."]
    );
}

#[test]
fn the_next_page_closes_folders_and_does_not_enter_one() {
    let (mut rig, device, _) = painted();
    command(&mut rig, Command::AddPage { copy_of: None });
    command(&mut rig, Command::SelectPage { page: 1 });
    rig.set_color(KEY_3, BLUE);
    command(&mut rig, Command::SelectPage { page: 0 });
    add(&mut rig, A, Some("Clips"));
    rig.set_color(KEY_3, GREEN);
    sent(&device);
    rig.take_events();

    rig.engine
        .handler_message(Message::Engine(EngineRequest::SelectPage(PageTarget::Next)));
    assert_eq!(rig.engine.active_page(), 1);
    assert!(rig.engine.open_folders().is_empty());
    let events = rig.take_events();
    assert!(open_of(&events).is_empty());
    assert_eq!(folder_names(&events), ["Clips"]);
    assert_color(&last_state(&events, KEY_3).image, 2, 2, BLUE);

    rig.engine
        .handler_message(Message::Engine(EngineRequest::SelectPage(PageTarget::Next)));
    assert_eq!(rig.engine.active_page(), 0);
    assert!(rig.engine.open_folders().is_empty());
    let events = rig.take_events();
    assert_color(&last_state(&events, KEY_3).image, 2, 2, RED);
}

#[test]
fn the_same_page_closes_folders_and_writes_only_the_keys() {
    let (mut rig, device, _) = painted();
    add(&mut rig, A, Some("Clips"));
    rig.set_color(KEY_3, GREEN);
    sent(&device);
    rig.take_events();

    command(&mut rig, Command::SelectPage { page: 0 });
    let writes = sent(&device);
    assert!(image_slots(&writes).contains(&KEY_3_SLOT), "{writes:?}");
    assert!(!image_slots(&writes).contains(&2), "{writes:?}");
    assert!(rig.engine.open_folders().is_empty());
    assert_color(&last_state(&rig.take_events(), KEY_3).image, 2, 2, RED);
}

#[test]
fn a_profile_change_closes_the_folders() {
    let (mut rig, _, _) = painted();
    add(&mut rig, A, Some("Clips"));
    command(
        &mut rig,
        Command::AddProfile {
            name: "Other".into(),
            copy_of: None,
        },
    );
    assert!(rig.engine.open_folders().is_empty());
    assert_ne!(rig.engine.active_profile(), "default");
    assert_eq!(rig.engine.active_page(), 0);

    command(
        &mut rig,
        Command::SelectProfile {
            profile: "default".into(),
        },
    );
    assert!(rig.engine.open_folders().is_empty());
    assert_eq!(rig.engine.active_profile(), "default");
    let events = rig.take_events();
    assert_eq!(folder_names(&events), ["Clips"]);
    assert!(open_of(&events).is_empty());
}

#[test]
fn a_reconnection_shows_the_open_folder_and_the_page_knobs() {
    let (mut rig, first, _) = painted();
    add(&mut rig, A, Some("Clips"));
    rig.set_color(KEY_3, GREEN);
    sent(&first);
    rig.take_events();

    first.borrow_mut().disconnect();
    rig.poll_for(Duration::from_secs(1));
    assert!(rig
        .take_events()
        .iter()
        .any(|event| matches!(event, Event::Disconnected)));

    let second = rig.connect();
    let writes = sent(&second);
    assert_color(&shown_at(&writes, KEY_3_SLOT), 2, 2, GREEN);
    assert_color(&shown_at(&writes, 2), 2, 2, BLUE);
    assert_eq!(rig.engine.open_folders(), &[A]);
}

#[test]
fn undo_of_a_new_folder_removes_it_and_redo_opens_it() {
    let (mut rig, _, _) = painted();
    let key = ControlId::new(ControlKind::Key, 1);
    command(
        &mut rig,
        Command::Group {
            name: "New Folder".into(),
            commands: vec![
                Command::AddFolder {
                    id: A,
                    name: None,
                    copy_of: None,
                },
                Command::SetBinding {
                    binding: BindingId {
                        page: 0,
                        control: key,
                        gesture: GestureKind::Press,
                        folder: Some(A),
                    },
                    action: Some(action(KIND)),
                },
                Command::SetLabel {
                    surface: KEY_1,
                    label: Some("Back".into()),
                },
            ],
        },
    );
    assert_eq!(rig.engine.open_folders(), &[A]);
    assert_eq!(history(&rig.take_events()).0.as_deref(), Some("New Folder"));
    assert_eq!(rig.engine.profile().folders().len(), 1);
    assert_eq!(
        rig.engine.profile().folders()[0]
            .page
            .content(KEY_1)
            .label
            .as_deref(),
        Some("Back")
    );

    command(&mut rig, Command::Undo);
    assert!(rig.engine.profile().folders().is_empty());
    assert!(rig.engine.open_folders().is_empty());
    assert_eq!(history(&rig.take_events()).1.as_deref(), Some("New Folder"));

    command(&mut rig, Command::Redo);
    assert_eq!(rig.engine.open_folders(), &[A]);
    assert_eq!(rig.engine.profile().folders()[0].id, A);
    assert_eq!(
        rig.engine.profile().folders()[0]
            .page
            .content(KEY_1)
            .label
            .as_deref(),
        Some("Back")
    );
}

#[test]
fn undo_of_a_removed_folder_restores_the_same_folder() {
    let (mut rig, _, _) = painted();
    add(&mut rig, A, Some("Clips"));
    rig.set_color(KEY_3, GREEN);
    rig.take_events();
    command(&mut rig, Command::RemoveFolder { folder: A });
    assert!(rig.engine.profile().folders().is_empty());
    assert!(rig.engine.open_folders().is_empty());
    assert_eq!(
        history(&rig.take_events()).0.as_deref(),
        Some("Remove Folder")
    );

    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.profile().folders()[0].id, A);
    assert_eq!(rig.engine.open_folders(), &[A]);
    let events = rig.take_events();
    assert_color(&last_state(&events, KEY_3).image, 2, 2, GREEN);
    open(&mut rig, A);
    assert_eq!(rig.engine.open_folders(), &[A]);
}

#[test]
fn each_folder_command_is_one_step_and_open_is_not() {
    let (mut rig, _, _) = painted();
    add(&mut rig, A, Some("Clips"));
    assert_eq!(history(&rig.take_events()).0.as_deref(), Some("Add Folder"));
    command(
        &mut rig,
        Command::RenameFolder {
            folder: A,
            name: "Mics".into(),
        },
    );
    assert_eq!(
        history(&rig.take_events()).0.as_deref(),
        Some("Rename Folder")
    );
    add(&mut rig, B, None);
    command(&mut rig, Command::MoveFolder { folder: B, to: 0 });
    let events = rig.take_events();
    assert_eq!(history(&events).0.as_deref(), Some("Move Folder"));
    assert_eq!(folder_names(&events), ["Folder 2", "Mics"]);
    command(
        &mut rig,
        Command::AddFolder {
            id: FolderId::from_raw(3).unwrap(),
            name: None,
            copy_of: Some(A),
        },
    );
    assert_eq!(
        history(&rig.take_events()).0.as_deref(),
        Some("Duplicate Folder")
    );

    let before = rig.engine.profile().folders().to_vec();
    open(&mut rig, A);
    back(&mut rig);
    let events = rig.take_events();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::History { .. })),
        "open and back are not steps: {events:?}"
    );
    assert_eq!(rig.engine.profile().folders(), before.as_slice());
}

#[test]
fn copy_to_all_pages_is_not_possible_in_a_folder() {
    let (mut rig, _, _) = painted();
    command(&mut rig, Command::AddPage { copy_of: None });
    add(&mut rig, A, Some("Clips"));
    command(
        &mut rig,
        Command::CopyToAllPages {
            control: ControlId::new(ControlKind::Key, 3),
        },
    );
    assert_eq!(
        problems(&rig.take_events()),
        ["Copy to All Pages is not possible in a folder."]
    );
}

#[test]
fn a_loaded_old_profile_is_written_as_schema_5_with_no_folders() {
    for version in [3, 4] {
        let dir = TempDir::new().unwrap();
        let path = store_in(dir.path()).profile_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            format!(
                "schema_version = {version}\nname = \"Old\"\nmodel = \"n4-pro\"\n\n\
                 [[pages]]\nname = \"Main\"\n\n[pages.bindings.key-3.press]\nkind = \"{KIND}\"\n"
            ),
        )
        .unwrap();
        let mut rig = Rig::in_dir(dir);
        assert_eq!(rig.engine.profile().name, "Old");
        assert_eq!(rig.engine.profile().pages()[0].name, "Main");
        assert!(rig.engine.profile().folders().is_empty());
        command(
            &mut rig,
            Command::RenamePage {
                page: 0,
                name: "Renamed".into(),
            },
        );
        rig.poll_for(SAVE_DELAY + Duration::from_secs(1));
        let text = rig.profile_text().unwrap();
        assert!(
            text.starts_with("schema_version = 5\n"),
            "version {version}: {text}"
        );
        assert!(text.contains("name = \"Renamed\""), "{text}");
        assert!(text.contains("kind = \"test.folders\""), "{text}");
        assert!(!text.contains("[[folders]]"), "{text}");
        assert!(!text.contains("[folders"), "{text}");
    }
}

#[test]
fn an_export_keeps_the_folder_and_its_key() {
    let (mut rig, _, _) = painted();
    add(&mut rig, A, Some("Clips"));
    rig.set_color(KEY_3, GREEN);
    command(
        &mut rig,
        Command::SetBinding {
            binding: key_binding(Some(A)),
            action: Some(action(KIND)),
        },
    );
    let out = TempDir::new().unwrap();
    let file = out.path().join("Default.opencraft");
    command(
        &mut rig,
        Command::ExportProfile {
            profile: "default".into(),
            path: file.clone(),
        },
    );
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains("[[profile.folders]]"), "{text}");
    assert!(text.contains("0000000000000001"), "{text}");
    assert!(text.contains("Clips"), "{text}");

    command(&mut rig, Command::ImportProfile { path: file });
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    assert_eq!(rig.engine.profile().folders().len(), 1);
    assert_eq!(rig.engine.profile().folders()[0].id, A);
    assert_eq!(rig.engine.profile().folders()[0].page.name, "Clips");
    assert_eq!(
        rig.engine.profile().folders()[0].page.content(KEY_3).color,
        Some(GREEN)
    );
    assert!(rig.engine.open_folders().is_empty());
}

/// In one text edit, a title change on the page and a title change of the
/// same key face in a folder are two steps: they change two different
/// surfaces (decision 0030).
#[test]
fn a_title_on_the_page_and_a_title_in_a_folder_are_two_steps() {
    let mut rig = Rig::new();
    rig.take_events();
    add(&mut rig, A, Some("F"));
    command(&mut rig, Command::CloseFolder);
    command(&mut rig, Command::StartTextEdit);
    command(
        &mut rig,
        Command::SetLabel {
            surface: KEY_1,
            label: Some("Page".into()),
        },
    );
    open(&mut rig, A);
    command(
        &mut rig,
        Command::SetLabel {
            surface: KEY_1,
            label: Some("Folder".into()),
        },
    );
    command(&mut rig, Command::EndTextEdit);
    command(&mut rig, Command::Undo);
    let profile = rig.engine.profile();
    assert_eq!(
        profile.pages()[0].content(KEY_1).label.as_deref(),
        Some("Page"),
        "one undo removes only the title in the folder"
    );
    assert_eq!(profile.folder(A).unwrap().page.content(KEY_1).label, None);
    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.profile().pages()[0].content(KEY_1).label, None);
}
