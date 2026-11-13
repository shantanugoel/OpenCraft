//! Profile tests: the profiles directory, the active profile, the commands
//! for profiles, and the request of a handler to select a profile. The tests
//! use temporary directories, the fake transport, and no thread.

use super::pages::{binding_states, image_slots, Step};
use super::*;
use opencraft_engine::profiles::{REMOVED_DIR, STATE_FILE};
use opencraft_engine::{EngineRequest, Page};

const KIND: &str = "test.profiles";
const MODEL: &str = "n4-pro";

/// A device model with a different model identifier and the same layout.
static OTHER_MODEL: opencraft_core::Descriptor = opencraft_core::Descriptor {
    name: "Other test model",
    model: "test-other",
    ..DESCRIPTORS[0]
};

/// The binding of the press of key 3 on the first page.
fn key_3() -> BindingId {
    binding(ControlKind::Key, 3, GestureKind::Press)
}

/// A profile with one page that has `color` on key face 3 and a binding for
/// the press of key 3.
fn profile(model: &str, name: &str, color: Color) -> Profile {
    let mut page = Page::new("Page 1");
    page.update(KEY_3, |c| c.color = Some(color));
    page.set_binding(key_3().control, key_3().gesture, Some(action(KIND)));
    Profile::for_model(model, name, vec![page])
}

fn save(dir: &Path, id: &str, profile: &Profile) {
    ProfilesDir::new(dir).store(id).save(profile).unwrap();
}

/// An engine on a profiles directory with the profiles `profiles`, and the
/// requests of its one handler. The events and the requests of the start are
/// gone.
fn rig_with(profiles: &[(&str, Profile)]) -> (Rig, mpsc::Receiver<Request>) {
    let dir = TempDir::new().unwrap();
    for (id, profile) in profiles {
        save(dir.path(), id, profile);
    }
    let (port, requests) = port(KIND);
    let mut rig = Rig::with_ports(dir, vec![port]);
    rig.take_events();
    requests.try_iter().count();
    (rig, requests)
}

/// The usual profiles: `default` with red on key face 3, and `work` with
/// blue.
fn two_profiles() -> (Rig, mpsc::Receiver<Request>) {
    rig_with(&[
        ("default", profile(MODEL, "Default", RED)),
        ("work", profile(MODEL, "Work", BLUE)),
    ])
}

fn entries(list: &[(&str, &str)]) -> Vec<ProfileEntry> {
    list.iter()
        .map(|(id, name)| ProfileEntry {
            id: id.to_string(),
            name: name.to_string(),
            pages: vec!["Page 1".into()],
            ..Default::default()
        })
        .collect()
}

fn profiles_event(list: &[(&str, &str)], active: &str) -> Event {
    Event::Profiles {
        profiles: entries(list),
        active: active.into(),
    }
}

fn command(rig: &mut Rig, command: Command) {
    assert!(rig.engine.command(command));
}

fn select(rig: &mut Rig, id: &str) {
    command(rig, Command::SelectProfile { profile: id.into() });
}

fn appear() -> Request {
    Request::Appear {
        binding: key_3(),
        action: action(KIND),
    }
}

fn disappear() -> Request {
    Request::Disappear { binding: key_3() }
}

fn state_text(rig: &Rig) -> Option<String> {
    fs::read_to_string(rig.dir.path().join(STATE_FILE)).ok()
}

fn file_of(rig: &Rig, id: &str) -> Option<String> {
    fs::read_to_string(ProfilesDir::new(rig.dir.path()).store(id).profile_path()).ok()
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

// --- The start ------------------------------------------------------------------

#[test]
fn an_old_default_directory_with_a_version_two_file_is_the_active_profile_with_no_loss() {
    let dir = TempDir::new().unwrap();
    let store = store_in(dir.path());
    fs::create_dir_all(store.dir()).unwrap();
    let old = "schema_version = 2\nname = \"Mine\"\n\n[[pages]]\nname = \"Main\"\n\n\
               [pages.surfaces.key-face-3]\ncolor = \"#C81E1E\"\n\n\
               [pages.bindings.key-3.press]\nkind = \"test.profiles\"\n\n\
               [pages.bindings.bar-panel-2.tap]\nkind = \"test.profiles\"\n\n\
               [pages.bindings.touch-bar-1.swipe-left]\nkind = \"test.profiles\"\n\n\
               [[pages]]\nname = \"Media\"\n";
    fs::write(store.profile_path(), old).unwrap();
    let (port, requests) = port(KIND);
    let mut rig = Rig::with_ports(dir, vec![port]);

    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    let mut entry = entries(&[("default", "Mine")]);
    entry[0].pages = vec!["Main".into(), "Media".into()];
    assert!(events.contains(&Event::Profiles {
        profiles: entry,
        active: "default".into(),
    }));
    assert_eq!(rig.engine.active_profile(), "default");
    assert_eq!(rig.engine.profile().model, MODEL);
    assert_eq!(rig.engine.profile().pages().len(), 2);
    assert_eq!(rig.engine.profile().pages()[0].bindings().count(), 3);
    assert_color(&last_state(&events, KEY_3).image, 2, 2, RED);
    assert_eq!(
        requests.try_iter().count(),
        3,
        "one Appear for each binding"
    );

    // The start writes nothing: not the profile, and not the state file.
    rig.poll_for(Duration::from_secs(2));
    assert_eq!(rig.profile_text().as_deref(), Some(old));
    assert_eq!(state_text(&rig), None);

    // The next change writes the current version with the model.
    rig.set_color(BAR_2, BLUE);
    rig.engine.finish();
    let text = rig.profile_text().unwrap();
    assert!(text.starts_with(&format!(
        "schema_version = {SCHEMA_VERSION}\nname = \"Mine\"\nmodel = \"n4-pro\"\n"
    )));
    let mut rig = rig.restart();
    assert!(problems(&rig.take_events()).is_empty());
    assert_eq!(rig.engine.profile().pages()[0].bindings().count(), 3);
}

#[test]
fn with_no_profile_the_engine_makes_one_and_writes_it_at_the_first_change() {
    let mut rig = Rig::new();
    let events = rig.take_events();
    assert!(events.contains(&profiles_event(&[("default", "Default")], "default")));
    rig.poll_for(Duration::from_secs(2));
    assert!(names_in(rig.dir.path()).is_empty(), "no write at the start");

    rig.set_color(KEY_3, RED);
    rig.poll_for(Duration::from_secs(1));
    assert!(file_of(&rig, "default")
        .unwrap()
        .contains("model = \"n4-pro\""));
}

#[test]
fn the_state_file_selects_the_active_profile_at_the_start() {
    let (mut rig, _requests) = two_profiles();
    select(&mut rig, "work");
    let state = state_text(&rig).expect("the change writes the state file");
    assert!(state.contains("n4-pro = \"work\""), "{state}");

    let mut rig = rig.restart();
    assert_eq!(rig.engine.active_profile(), "work");
    let events = rig.take_events();
    assert!(events.contains(&profiles_event(
        &[("default", "Default"), ("work", "Work")],
        "work"
    )));
    assert_color(&last_state(&events, KEY_3).image, 2, 2, BLUE);
}

#[test]
fn a_state_file_that_names_no_profile_of_the_model_gives_the_first_profile() {
    let other = profile("test-other", "Other", GREEN);
    for state in [
        "[active]\nn4-pro = \"missing\"\n",
        "[active]\nn4-pro = \"aaa-other\"\n",
        "[active]\ntest-other = \"work\"\n",
        "this is not TOML [",
        "active = \"work\"\n",
    ] {
        let dir = TempDir::new().unwrap();
        save(dir.path(), "aaa-other", &other);
        save(dir.path(), "work", &profile(MODEL, "Work", BLUE));
        save(dir.path(), "zed", &profile(MODEL, "Zed", RED));
        fs::write(dir.path().join(STATE_FILE), state).unwrap();
        let mut rig = Rig::in_dir(dir);
        assert_eq!(rig.engine.active_profile(), "work", "{state}");
        let events = rig.take_events();
        assert!(problems(&events).is_empty(), "{state}: {events:?}");
        assert!(
            events.contains(&profiles_event(&[("work", "Work"), ("zed", "Zed")], "work")),
            "{state}"
        );
        // The start does not change the state file.
        assert_eq!(state_text(&rig).as_deref(), Some(state));
    }
}

#[test]
fn a_profile_of_a_different_model_is_not_offered_and_does_not_change() {
    let (mut rig, requests) = rig_with(&[("default", profile("test-other", "Other", GREEN))]);
    let before = file_of(&rig, "default").unwrap();
    // The model has no profile, so the engine makes one with a free name.
    assert_eq!(rig.engine.active_profile(), "default-2");
    assert_eq!(rig.engine.profile().model, MODEL);

    select(&mut rig, "default");
    rig.engine
        .handler_message(Message::Engine(EngineRequest::SelectProfile {
            name: "Other".into(),
        }));
    command(
        &mut rig,
        Command::RenameProfile {
            profile: "default".into(),
            name: "New".into(),
        },
    );
    command(
        &mut rig,
        Command::RemoveProfile {
            profile: "default".into(),
        },
    );
    command(
        &mut rig,
        Command::AddProfile {
            name: "Copy".into(),
            copy_of: Some("default".into()),
        },
    );
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 5, "{events:?}");
    assert!(requests.try_recv().is_err());
    assert_eq!(rig.engine.active_profile(), "default-2");
    rig.engine.finish();
    assert_eq!(file_of(&rig, "default").unwrap(), before);
}

#[test]
fn a_bad_profile_that_is_not_active_does_not_stop_the_start_and_does_not_change() {
    let dir = TempDir::new().unwrap();
    save(dir.path(), "default", &profile(MODEL, "Default", RED));
    let broken = ProfilesDir::new(dir.path()).store("broken");
    fs::create_dir_all(broken.dir()).unwrap();
    let text = "schema_version = 2\nname = \"Broken\"\nunknown = [\n";
    fs::write(broken.profile_path(), text).unwrap();
    let newer = ProfilesDir::new(dir.path()).store("newer");
    fs::create_dir_all(newer.dir()).unwrap();
    let newer_text = "schema_version = 99\nname = \"Newer\"\nmodel = \"n4-pro\"\n";
    fs::write(newer.profile_path(), newer_text).unwrap();
    fs::write(
        dir.path().join(STATE_FILE),
        "[active]\nn4-pro = \"default\"\n",
    )
    .unwrap();

    let mut rig = Rig::in_dir(dir);
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    assert_eq!(rig.engine.active_profile(), "default");
    // The profile with no readable name has its identity as its name.
    // A file with no readable pages lists no page.
    let mut list = entries(&[
        ("broken", "broken"),
        ("default", "Default"),
        ("newer", "Newer"),
    ]);
    list[0].pages.clear();
    list[2].pages.clear();
    assert!(events.contains(&Event::Profiles {
        profiles: list,
        active: "default".into(),
    }));
    rig.poll_for(Duration::from_secs(2));
    assert_eq!(names_in(broken.dir()), [PROFILE_FILE]);
    assert_eq!(fs::read_to_string(broken.profile_path()).unwrap(), text);

    // A rename and a copy of it change nothing.
    command(
        &mut rig,
        Command::RenameProfile {
            profile: "newer".into(),
            name: "Renamed".into(),
        },
    );
    command(
        &mut rig,
        Command::AddProfile {
            name: "Copy".into(),
            copy_of: Some("broken".into()),
        },
    );
    assert_eq!(problems(&rig.take_events()).len(), 2);
    assert_eq!(
        fs::read_to_string(newer.profile_path()).unwrap(),
        newer_text
    );
    assert_eq!(rig.engine.active_profile(), "default");

    // When the user selects it, decision 0009 applies: the engine keeps the
    // file as a backup and starts with an empty profile.
    select(&mut rig, "newer");
    let events = rig.take_events();
    let problems = problems(&events);
    assert_eq!(problems.len(), 1, "{events:?}");
    assert!(problems[0].contains("99"), "{}", problems[0]);
    assert_eq!(rig.engine.active_profile(), "newer");
    assert_eq!(rig.engine.profile().model, MODEL);
    assert!(last_state(&events, KEY_3).content.is_empty());
    assert_eq!(
        fs::read_to_string(newer.dir().join("profile.backup-1.toml")).unwrap(),
        newer_text
    );
}

// --- A change of the active profile -------------------------------------------------

#[test]
fn a_profile_change_sends_disappear_then_writes_only_changed_images_then_sends_appear() {
    let (mut rig, requests) = two_profiles();
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    let requests = Rc::new(requests);
    let steps = Rc::new(RefCell::new(Vec::new()));
    let (queue, list) = (Rc::clone(&requests), Rc::clone(&steps));
    *rig.world.before_write.borrow_mut() = Some(Box::new(move || {
        let mut list = list.borrow_mut();
        list.extend(queue.try_iter().map(Step::Request));
        list.push(Step::Write);
    }));
    select(&mut rig, "work");
    *rig.world.before_write.borrow_mut() = None;
    let mut steps = steps.take();
    steps.extend(requests.try_iter().map(Step::Request));
    steps.dedup();
    assert_eq!(
        steps,
        [
            Step::Request(disappear()),
            Step::Write,
            Step::Request(appear())
        ]
    );
    assert_eq!(rig.engine.active_profile(), "work");

    // Only key face 3 is different. No start sequence.
    let sent = sent(&device);
    assert_eq!(image_slots(&sent), [KEY_3_SLOT]);
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert_color(&shown_at(&sent, KEY_3_SLOT), 2, 2, BLUE);

    // Each surface, then the profiles, then the pages, then the bindings.
    let events = rig.take_events();
    assert_eq!(surfaces(&events).len(), 14);
    assert_eq!(
        events[14],
        profiles_event(&[("default", "Default"), ("work", "Work")], "work")
    );
    assert_eq!(
        events[15],
        Event::Pages {
            names: vec!["Page 1".into()],
            active: 0,
            first: 0,
        }
    );
    assert_eq!(binding_states(&events[16..]), [key_3()]);
    // The history of the old profile is gone.
    assert_eq!(
        events[17],
        Event::History {
            undo: None,
            redo: None
        }
    );
    assert_eq!(events.len(), 18, "{events:?}");
}

#[test]
fn a_profile_change_starts_on_the_first_page_and_removes_the_feedback() {
    let mut second_page = profile(MODEL, "Default", RED);
    second_page.add_page(Page::new("Page 2"));
    let (mut rig, requests) = rig_with(&[
        ("default", second_page),
        ("work", profile(MODEL, "Work", BLUE)),
    ]);
    command(&mut rig, Command::SelectPage { page: 1 });
    rig.engine.handler_message(Message::Feedback {
        binding: key_3(),
        feedback: Some(Feedback {
            label: Some("x".into()),
            level: None,
            state: None,
        }),
    });
    select(&mut rig, "work");
    select(&mut rig, "default");
    assert_eq!(rig.engine.active_page(), 0);
    requests.try_iter().count();
    let events = rig.take_events();
    assert!(events.contains(&Event::Pages {
        names: vec!["Page 1".into(), "Page 2".into()],
        active: 0,
        first: 0,
    }));
}

#[test]
fn unsaved_changes_are_written_before_a_change_of_profile() {
    let (mut rig, _requests) = two_profiles();
    rig.set_color(BAR_2, GREEN);
    select(&mut rig, "work");
    assert!(file_of(&rig, "default").unwrap().contains("#00C800"));

    // The delayed save of the old profile does not come later.
    let text = file_of(&rig, "default");
    rig.poll_for(Duration::from_secs(1));
    assert_eq!(file_of(&rig, "default"), text);
    assert!(!file_of(&rig, "work").unwrap().contains("#00C800"));
}

#[test]
fn a_failed_write_of_the_old_profile_keeps_it_active() {
    let (mut rig, requests) = two_profiles();
    rig.set_color(BAR_2, GREEN);
    // A directory in the place of the temporary file: the write fails.
    let store = store_in(rig.dir.path());
    fs::create_dir_all(store.dir().join("profile.toml.tmp")).unwrap();
    rig.take_events();

    select(&mut rig, "work");
    assert_eq!(problems(&rig.take_events()).len(), 1);
    assert_eq!(rig.engine.active_profile(), "default");
    assert!(requests.try_recv().is_err());
}

#[test]
fn a_select_of_the_active_profile_does_nothing_and_a_missing_profile_is_a_problem() {
    let (mut rig, requests) = two_profiles();
    select(&mut rig, "default");
    assert!(rig.take_events().is_empty());
    select(&mut rig, "missing");
    assert_eq!(problems(&rig.take_events()).len(), 1);
    assert!(requests.try_recv().is_err());
    assert_eq!(state_text(&rig), None);
}

// --- Add, rename, and remove ------------------------------------------------------

#[test]
fn an_added_profile_is_empty_and_becomes_the_active_profile() {
    let (mut rig, requests) = two_profiles();
    command(
        &mut rig,
        Command::AddProfile {
            name: "  Video Calls ".into(),
            copy_of: None,
        },
    );
    assert_eq!(rig.engine.active_profile(), "video-calls");
    assert_eq!(rig.engine.profile().name, "Video Calls");
    assert_eq!(requests.try_iter().collect::<Vec<_>>(), [disappear()]);
    let events = rig.take_events();
    assert!(events.contains(&profiles_event(
        &[
            ("default", "Default"),
            ("video-calls", "Video Calls"),
            ("work", "Work")
        ],
        "video-calls"
    )));
    assert!(binding_states(&events).is_empty());
    assert!(last_state(&events, KEY_3).content.is_empty());
    assert!(file_of(&rig, "video-calls")
        .unwrap()
        .contains("name = \"Video Calls\""));
    assert!(state_text(&rig).unwrap().contains("\"video-calls\""));

    // The same name gives a new identity. A name of spaces is a problem.
    command(
        &mut rig,
        Command::AddProfile {
            name: "Video calls".into(),
            copy_of: None,
        },
    );
    assert_eq!(rig.engine.active_profile(), "video-calls-2");
    rig.take_events();
    for (name, copy_of) in [(" ", None), ("Copy", Some("missing"))] {
        command(
            &mut rig,
            Command::AddProfile {
                name: name.into(),
                copy_of: copy_of.map(str::to_string),
            },
        );
    }
    assert_eq!(problems(&rig.take_events()).len(), 2);
    assert_eq!(rig.engine.active_profile(), "video-calls-2");
}

#[test]
fn a_copy_of_a_profile_has_its_pages_and_its_image_files() {
    let (mut rig, _requests) = two_profiles();
    let file = green_file(rig.dir.path());
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: Some(file),
        },
    );
    command(&mut rig, Command::AddPage { copy_of: Some(0) });
    let original = rig.engine.profile().clone();

    // A copy of the active profile, with its changes that are not saved.
    command(
        &mut rig,
        Command::AddProfile {
            name: "Copy".into(),
            copy_of: Some("default".into()),
        },
    );
    assert_eq!(rig.engine.active_profile(), "copy");
    let copy = rig.engine.profile().clone();
    assert_eq!(copy.name, "Copy");
    assert_eq!(copy.pages(), original.pages());
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    assert_color(&last_state(&events, KEY_3).image, 56, 56, GREEN);
    let root = rig.dir.path().to_path_buf();
    let images = |id: &str| names_in(&root.join(id).join("images"));
    assert_eq!(images("copy"), images("default"));
    assert_eq!(images("copy").len(), 1);
    // The old profile is in its file.
    assert_eq!(
        ProfilesDir::new(rig.dir.path())
            .store("default")
            .read()
            .unwrap(),
        original
    );

    // A copy of a profile that is not active reads its file.
    command(
        &mut rig,
        Command::AddProfile {
            name: "Copy".into(),
            copy_of: Some("default".into()),
        },
    );
    assert_eq!(rig.engine.active_profile(), "copy-2");
    assert_eq!(rig.engine.profile().pages(), original.pages());
    assert_eq!(images("copy-2").len(), 1);

    // The copy is complete without the original.
    command(
        &mut rig,
        Command::RemoveProfile {
            profile: "default".into(),
        },
    );
    select(&mut rig, "copy");
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    assert_color(&last_state(&events, KEY_3).image, 56, 56, GREEN);
}

#[test]
fn a_rename_changes_the_name_and_not_the_identity() {
    let (mut rig, requests) = two_profiles();
    let device = rig.connect();
    sent(&device);
    rig.take_events();
    let rename = |profile: &str, name: &str| Command::RenameProfile {
        profile: profile.into(),
        name: name.into(),
    };

    // The active profile: the delayed save writes the name.
    command(&mut rig, rename("default", "Home"));
    assert_eq!(
        rig.take_events(),
        [profiles_event(
            &[("default", "Home"), ("work", "Work")],
            "default"
        )]
    );
    rig.poll_for(Duration::from_secs(1));
    assert!(file_of(&rig, "default")
        .unwrap()
        .contains("name = \"Home\""));

    // A profile that is not active: the engine writes its file now.
    command(&mut rig, rename("work", " Office "));
    assert_eq!(
        rig.take_events(),
        [profiles_event(
            &[("default", "Home"), ("work", "Office")],
            "default"
        )]
    );
    assert!(file_of(&rig, "work").unwrap().contains("name = \"Office\""));
    assert!(requests.try_recv().is_err());
    assert!(sent(&device).is_empty());
    assert_eq!(names_in(rig.dir.path()), ["default", "work"]);

    // The same name changes nothing. A name of spaces and a missing profile
    // are problems.
    command(&mut rig, rename("work", "Office"));
    assert!(rig.take_events().is_empty());
    command(&mut rig, rename("work", "  "));
    command(&mut rig, rename("missing", "X"));
    assert_eq!(problems(&rig.take_events()).len(), 2);
}

#[test]
fn a_removal_moves_the_directory_into_removed_and_one_profile_stays() {
    let (mut rig, requests) = rig_with(&[
        ("default", profile(MODEL, "Default", RED)),
        ("media", profile(MODEL, "Media", GREEN)),
        ("work", profile(MODEL, "Work", BLUE)),
    ]);
    let remove = |profile: &str| Command::RemoveProfile {
        profile: profile.into(),
    };

    // A profile that is not active: only the list changes.
    command(&mut rig, remove("work"));
    assert_eq!(
        rig.take_events(),
        [profiles_event(
            &[("default", "Default"), ("media", "Media")],
            "default"
        )]
    );
    assert!(requests.try_recv().is_err());
    let removed = rig.dir.path().join(REMOVED_DIR);
    assert_eq!(names_in(&removed), ["work"]);
    assert!(ProfileStore::new(removed.join("work")).read().is_ok());

    // The active profile: the profile after it becomes active. Its changes
    // go with it into `removed`.
    rig.set_color(BAR_2, BLUE);
    command(&mut rig, remove("default"));
    assert_eq!(rig.engine.active_profile(), "media");
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(), appear()]
    );
    let events = rig.take_events();
    assert!(events.contains(&profiles_event(&[("media", "Media")], "media")));
    assert_color(&last_state(&events, KEY_3).image, 2, 2, GREEN);
    let kept = ProfileStore::new(removed.join("default")).read().unwrap();
    assert_eq!(kept.pages()[0].content(BAR_2).color, Some(BLUE));
    assert!(state_text(&rig).unwrap().contains("\"media\""));
    // The delayed save does not make the removed directory again.
    rig.poll_for(Duration::from_secs(1));
    assert_eq!(names_in(rig.dir.path()), ["media", REMOVED_DIR, STATE_FILE]);

    // One profile stays. A missing profile is a problem.
    command(&mut rig, remove("media"));
    command(&mut rig, remove("missing"));
    assert_eq!(problems(&rig.take_events()).len(), 2);
    assert_eq!(rig.engine.active_profile(), "media");

    // A new profile with a removed name gets a new directory, and its later
    // removal keeps the first one.
    command(
        &mut rig,
        Command::AddProfile {
            name: "Work".into(),
            copy_of: None,
        },
    );
    assert_eq!(rig.engine.active_profile(), "work");
    command(&mut rig, remove("work"));
    assert_eq!(
        rig.engine.active_profile(),
        "media",
        "the profile before it"
    );
    assert_eq!(names_in(&removed), ["default", "work", "work-2"]);
}

// --- The request of a handler ------------------------------------------------------

fn select_by_name(rig: &mut Rig, name: &str) {
    rig.engine
        .handler_message(Message::Engine(EngineRequest::SelectProfile {
            name: name.into(),
        }));
}

#[test]
fn a_handler_selects_a_profile_by_its_name_for_the_user() {
    let (mut rig, requests) = rig_with(&[
        ("default", profile(MODEL, "Default", RED)),
        ("work", profile(MODEL, "Work", BLUE)),
        ("work-2", profile(MODEL, "Work", GREEN)),
    ]);
    select_by_name(&mut rig, "Work");
    assert_eq!(rig.engine.active_profile(), "work", "the first identity");
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(), appear()]
    );

    // The active profile changes nothing. A missing name is a problem.
    rig.take_events();
    select_by_name(&mut rig, "Work");
    assert!(rig.take_events().is_empty());
    select_by_name(&mut rig, "work");
    assert_eq!(problems(&rig.take_events()).len(), 1);
    assert_eq!(rig.engine.active_profile(), "work");
    assert!(requests.try_recv().is_err());
}

// --- A device of a different model ----------------------------------------------

#[test]
fn a_device_of_a_different_model_selects_the_profiles_of_that_model() {
    let (mut rig, requests) = rig_with(&[
        ("default", profile(MODEL, "Default", RED)),
        ("other", profile("test-other", "Other", GREEN)),
    ]);
    rig.set_color(BAR_2, BLUE);
    rig.take_events();

    let device = rig.plug_model(&OTHER_MODEL);
    rig.poll_for(Duration::from_secs(2));
    assert!(rig.engine.link().is_connected());
    assert_eq!(rig.engine.active_profile(), "other");
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(), appear()]
    );
    // The change of the old profile is in its file.
    assert!(file_of(&rig, "default").unwrap().contains("#1428DC"));

    let events = rig.take_events();
    assert!(matches!(&events[0], Event::Layout(l) if l.name == OTHER_MODEL.name));
    assert!(matches!(events[1], Event::Kinds(_)));
    assert_eq!(surfaces(&events[2..16]).len(), 14);
    assert_eq!(events[16], profiles_event(&[("other", "Other")], "other"));
    assert!(matches!(events[17], Event::Pages { active: 0, .. }));
    assert_eq!(binding_states(&events[18..19]), [key_3()]);
    assert!(matches!(events[19], Event::History { undo: None, .. }));
    assert!(matches!(events[20], Event::DeviceSettings(_)));
    assert_eq!(events[21], Event::Connected);
    assert_color(&shown_at(&sent(&device), KEY_3_SLOT), 2, 2, GREEN);
}

#[test]
fn a_device_of_a_model_with_no_profile_gets_a_new_profile() {
    let (mut rig, _requests) = rig_with(&[("default", profile(MODEL, "Default", RED))]);
    let device = rig.plug_model(&OTHER_MODEL);
    rig.poll_for(Duration::from_secs(2));
    assert_eq!(rig.engine.active_profile(), "default-2");
    assert_eq!(rig.engine.profile().model, "test-other");
    let events = rig.take_events();
    assert!(events.contains(&profiles_event(&[("default-2", "Default")], "default-2")));
    assert!(last_state(&events, KEY_3).content.is_empty());

    // A device of the first model gets its profiles again.
    device.borrow_mut().disconnect();
    rig.poll_for(Duration::from_secs(1));
    rig.connect();
    assert_eq!(rig.engine.active_profile(), "default");
    let events = rig.take_events();
    assert!(events.contains(&profiles_event(&[("default", "Default")], "default")));
    assert_color(&last_state(&events, KEY_3).image, 2, 2, RED);
    // The new profile of the other model was written when the engine left it.
    assert!(file_of(&rig, "default-2")
        .unwrap()
        .contains("model = \"test-other\""));
}
