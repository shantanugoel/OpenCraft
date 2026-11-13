//! Profile settings tests: the colour and the first page of a profile, and
//! the export and the import of a profile file with its images (issue
//! #195). The tests use temporary directories, the fake transport, and no
//! thread.

use super::*;
use opencraft_engine::share;
use opencraft_engine::Page;

const MODEL: &str = "n4-pro";
const TEAL: Color = Color::rgb(0x14, 0xB8, 0xA6);

fn command(rig: &mut Rig, command: Command) {
    assert!(rig.engine.command(command));
}

/// A profile with three pages, an image on key face 3 of the first page, a
/// red fill on the second page, and a binding.
fn profile_with_image(dir: &Path, id: &str, name: &str) -> Profile {
    let store = ProfilesDir::new(dir).store(id);
    let source = dir.join(format!("{id}-icon.png"));
    image::RgbaImage::from_fn(40, 30, |x, y| {
        image::Rgba([x as u8 * 6, y as u8 * 8, 90, 255])
    })
    .save(&source)
    .unwrap();
    let image = store.import_image(&source).unwrap();
    fs::remove_file(&source).unwrap();
    let mut first = Page::new("Main");
    first.update(KEY_3, |c| {
        c.image = Some(image.clone());
        c.label = Some("Mail".into());
    });
    let press = binding(ControlKind::Key, 3, GestureKind::Press);
    first.set_binding(press.control, press.gesture, Some(action("test.share")));
    let mut second = Page::new("Media");
    second.update(KEY_3, |c| c.color = Some(RED));
    let mut profile = Profile::for_model(MODEL, name, vec![first, second, Page::new("Calls")]);
    profile.color = Some(TEAL);
    profile.set_first_page(1);
    store.save(&profile).unwrap();
    profile
}

/// An engine with the profile `default` (empty) active, and the profile
/// `work` of [`profile_with_image`].
fn rig() -> (Rig, Profile) {
    let dir = TempDir::new().unwrap();
    ProfilesDir::new(dir.path())
        .store("default")
        .save(&Profile::for_model(MODEL, "Default", Vec::new()))
        .unwrap();
    let work = profile_with_image(dir.path(), "work", "Work");
    fs::write(
        dir.path().join("state.toml"),
        "[active]\nn4-pro = \"default\"\n",
    )
    .unwrap();
    let mut rig = Rig::in_dir(dir);
    rig.take_events();
    (rig, work)
}

fn entry<'a>(events: &'a [Event], id: &str) -> Option<&'a ProfileEntry> {
    events.iter().rev().find_map(|e| match e {
        Event::Profiles { profiles, .. } => profiles.iter().find(|p| p.id == id),
        _ => None,
    })
}

#[test]
fn the_list_has_the_colour_the_pages_and_the_first_page_of_each_profile() {
    let dir = TempDir::new().unwrap();
    profile_with_image(dir.path(), "work", "Work");
    let mut rig = Rig::in_dir(dir);
    let events = rig.take_events();
    let work = entry(&events, "work").unwrap();
    assert_eq!(work.color, Some(TEAL));
    assert_eq!(work.pages, ["Main", "Media", "Calls"]);
    assert_eq!(work.first_page, 1);
}

#[test]
fn a_profile_starts_on_its_first_page() {
    let (mut rig, _) = rig();
    command(
        &mut rig,
        Command::SelectProfile {
            profile: "work".into(),
        },
    );
    assert_eq!(rig.engine.active_page(), 1);
    let events = rig.take_events();
    assert!(events.contains(&Event::Pages {
        names: vec!["Main".into(), "Media".into(), "Calls".into()],
        active: 1,
        first: 1,
        folders: Vec::new(),
        open: Vec::new(),
    }));
    assert_color(&last_state(&events, KEY_3).image, 2, 2, RED);

    // At a new start too.
    let mut rig = rig.restart();
    assert_eq!(rig.engine.active_profile(), "work");
    assert_eq!(rig.engine.active_page(), 1);
    rig.take_events();
}

#[test]
fn the_colour_of_a_profile_changes_in_memory_and_in_the_file_with_no_step() {
    let (mut rig, _) = rig();
    // The active profile: the file follows after the delay.
    command(
        &mut rig,
        Command::SetProfileColor {
            profile: "default".into(),
            color: Some(BLUE),
        },
    );
    let events = rig.take_events();
    assert_eq!(entry(&events, "default").unwrap().color, Some(BLUE));
    assert!(
        !events.iter().any(|e| matches!(e, Event::History { .. })),
        "{events:?}"
    );
    rig.poll_for(SAVE_DELAY * 2);
    assert!(rig.profile_text().unwrap().contains("color = \"#1428DC\""));

    // A profile that is not active: the engine writes its file at once.
    command(
        &mut rig,
        Command::SetProfileColor {
            profile: "work".into(),
            color: None,
        },
    );
    let events = rig.take_events();
    assert_eq!(entry(&events, "work").unwrap().color, None);
    let store = ProfilesDir::new(rig.dir.path()).store("work");
    assert_eq!(store.read().unwrap().color, None);

    // The same colour sends nothing.
    command(
        &mut rig,
        Command::SetProfileColor {
            profile: "work".into(),
            color: None,
        },
    );
    assert!(rig.take_events().is_empty());
}

#[test]
fn the_first_page_of_the_active_profile_is_a_step_of_the_history() {
    let (mut rig, _) = rig();
    command(
        &mut rig,
        Command::SelectProfile {
            profile: "work".into(),
        },
    );
    rig.take_events();
    command(
        &mut rig,
        Command::SetFirstPage {
            profile: "work".into(),
            page: 2,
        },
    );
    let events = rig.take_events();
    assert!(events.contains(&Event::Pages {
        names: vec!["Main".into(), "Media".into(), "Calls".into()],
        active: 1,
        first: 2,
        folders: Vec::new(),
        open: Vec::new(),
    }));
    assert!(events.contains(&Event::History {
        undo: Some("Set First Page".into()),
        redo: None,
    }));
    assert_eq!(
        rig.engine.active_page(),
        1,
        "the device shows the same page"
    );

    // A move of the page keeps the mark on the page.
    command(&mut rig, Command::MovePage { page: 2, to: 0 });
    assert_eq!(rig.engine.profile().first_page(), 0);
    command(&mut rig, Command::Undo);
    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.profile().first_page(), 1);

    // A page that does not exist is a problem.
    rig.take_events();
    command(
        &mut rig,
        Command::SetFirstPage {
            profile: "work".into(),
            page: 7,
        },
    );
    assert_eq!(problems(&rig.take_events()).len(), 1);
}

#[test]
fn the_first_page_of_a_profile_that_is_not_active_goes_to_its_file() {
    let (mut rig, _) = rig();
    command(
        &mut rig,
        Command::SetFirstPage {
            profile: "work".into(),
            page: 0,
        },
    );
    let events = rig.take_events();
    assert_eq!(entry(&events, "work").unwrap().first_page, 0);
    assert!(!events.iter().any(|e| matches!(e, Event::History { .. })));
    let store = ProfilesDir::new(rig.dir.path()).store("work");
    assert_eq!(store.read().unwrap().first_page(), 0);
}

#[test]
fn an_export_and_an_import_give_the_same_profile_with_the_images_as_a_new_profile() {
    let (mut rig, work) = rig();
    let out = TempDir::new().unwrap();
    let file = out.path().join("Work.opencraft");
    command(
        &mut rig,
        Command::ExportProfile {
            profile: "work".into(),
            path: file.clone(),
        },
    );
    assert_eq!(
        rig.take_events(),
        [Event::ProfileExported {
            profile: "work".into(),
            path: file.clone(),
        }]
    );

    // The name exists, thus the new profile gets a number after it.
    command(&mut rig, Command::ImportProfile { path: file.clone() });
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    assert_eq!(
        events.last(),
        Some(&Event::ProfileImported {
            profile: "work-2".into()
        })
    );
    assert_eq!(rig.engine.active_profile(), "work-2");
    assert_eq!(rig.engine.active_page(), 1, "the first page of the file");
    let mut want = work.clone();
    want.name = "Work 2".into();
    assert_eq!(rig.engine.profile(), &want);
    let imported = ProfilesDir::new(rig.dir.path()).store("work-2");
    assert_eq!(imported.read().unwrap(), want);
    let source = ProfilesDir::new(rig.dir.path()).store("work");
    for path in work.image_paths() {
        assert_eq!(
            fs::read(imported.dir().join(path)).unwrap(),
            fs::read(source.dir().join(path)).unwrap()
        );
    }
    // The device shows the image of the imported profile.
    command(&mut rig, Command::SelectPage { page: 0 });
    let events = rig.take_events();
    assert_eq!(
        last_state(&events, KEY_3).content.label.as_deref(),
        Some("Mail")
    );
    assert!(last_state(&events, KEY_3).content.image.is_some());

    // A third import gets the next number.
    command(&mut rig, Command::ImportProfile { path: file });
    assert_eq!(rig.engine.profile().name, "Work 3");
}

#[test]
fn the_export_of_the_active_profile_has_the_changes_that_are_not_in_its_file() {
    let (mut rig, _) = rig();
    rig.set_color(KEY_3, BLUE);
    let out = TempDir::new().unwrap();
    let file = out.path().join("Default.opencraft");
    command(
        &mut rig,
        Command::ExportProfile {
            profile: "default".into(),
            path: file.clone(),
        },
    );
    let package = share::import(&file).unwrap();
    assert_eq!(package.profile.pages()[0].content(KEY_3).color, Some(BLUE));
}

#[test]
fn a_wrong_file_gives_a_problem_and_changes_nothing() {
    let (mut rig, _) = rig();
    let out = TempDir::new().unwrap();
    let wrong = out.path().join("notes.opencraft");
    fs::write(&wrong, "Hello").unwrap();
    let other = out.path().join("other.opencraft");
    let mut profile = Profile::for_model("other-model", "Other", Vec::new());
    profile.color = None;
    share::export(&profile, &ProfilesDir::new(out.path()).store("x"), &other).unwrap();
    let missing = out.path().join("missing.opencraft");

    let before: Vec<String> = fs::read_dir(rig.dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    for (path, want) in [
        (&wrong, "not a profile file of OpenCraft"),
        (&other, "different device model (other-model)"),
        (&missing, "cannot read"),
    ] {
        command(&mut rig, Command::ImportProfile { path: path.clone() });
        let events = rig.take_events();
        let problems = problems(&events);
        assert_eq!(problems.len(), 1, "{events:?}");
        assert!(problems[0].contains(want), "{}", problems[0]);
        assert!(problems[0].starts_with("OpenCraft could not import the profile, because "));
    }
    assert_eq!(rig.engine.active_profile(), "default");
    let after: Vec<String> = fs::read_dir(rig.dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(before, after);

    // An export to a folder that does not exist is a problem.
    command(
        &mut rig,
        Command::ExportProfile {
            profile: "work".into(),
            path: out.path().join("no-folder/Work.opencraft"),
        },
    );
    let events = rig.take_events();
    assert!(problems(&events)[0].starts_with("OpenCraft could not export the profile"));
}
