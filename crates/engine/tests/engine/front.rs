//! The front application selects the profile (decision 0035). The tests use
//! the fake transport and injected commands. They do not open a device.

use super::*;
use opencraft_engine::store::SCHEMA_VERSION;
use opencraft_engine::{
    ActiveCause, ApplicationLink, EngineRequest, FrontApplication, Page, ProfileTarget,
};
use std::path::Path;

fn link(bundle_id: &str, name: &str) -> ApplicationLink {
    ApplicationLink {
        bundle_id: bundle_id.into(),
        name: name.into(),
    }
}

fn plain(name: &str) -> Profile {
    Profile::for_model("n4-pro", name, vec![Page::new("Main")])
}

fn zoom_calls() -> Profile {
    let mut profile =
        Profile::for_model("n4-pro", "Calls", vec![Page::new("One"), Page::new("Two")]);
    profile.applications.push(link("us.zoom.xos", "zoom.us"));
    profile
}

fn save_at(dir: &Path, id: &str, profile: &Profile) {
    ProfilesDir::new(dir).store(id).save(profile).unwrap();
}

/// `calls` is before `home` in the order of the identities, so the engine
/// starts on `calls`. `home` has no link, so it is the default profile.
fn calls_and_home() -> Rig {
    let dir = TempDir::new().unwrap();
    save_at(dir.path(), "calls", &zoom_calls());
    save_at(dir.path(), "home", &plain("Home"));
    let mut rig = Rig::in_dir(dir);
    rig.take_events();
    rig
}

fn front(rig: &mut Rig, front: FrontApplication) {
    assert!(rig.engine.command(Command::FrontApplication(front)));
}

fn bundle(bundle_id: &str) -> FrontApplication {
    FrontApplication::Bundle(bundle_id.into())
}

fn select_key(rig: &mut Rig, id: &str) {
    rig.engine
        .handler_message(Message::Engine(EngineRequest::SelectProfile {
            target: ProfileTarget::Profile(id.into()),
            page: None,
        }));
}

fn profiles_of(events: &[Event]) -> Vec<&Event> {
    events
        .iter()
        .filter(|event| matches!(event, Event::Profiles { .. }))
        .collect()
}

fn text_of(rig: &Rig, id: &str) -> String {
    fs::read_to_string(ProfilesDir::new(rig.dir.path()).store(id).profile_path()).unwrap()
}

#[test]
fn an_old_file_is_written_as_version_5_with_the_same_pages() {
    let dir = TempDir::new().unwrap();
    let calls = ProfilesDir::new(dir.path()).store("calls");
    let home = ProfilesDir::new(dir.path()).store("home");
    fs::create_dir_all(calls.dir()).unwrap();
    fs::create_dir_all(home.dir()).unwrap();
    fs::write(
        calls.profile_path(),
        "schema_version = 4\nname = \"Calls\"\nmodel = \"n4-pro\"\n\n[[pages]]\nname = \"Main\"\n",
    )
    .unwrap();
    fs::write(
        home.profile_path(),
        "schema_version = 3\nname = \"Home\"\nmodel = \"n4-pro\"\n\n[[pages]]\nname = \"Desk\"\n",
    )
    .unwrap();
    let mut rig = Rig::in_dir(dir);
    assert!(problems(&rig.take_events()).is_empty());
    assert_eq!(rig.engine.active_profile(), "calls");
    rig.set_color(KEY_3, RED);
    rig.poll_for(Duration::from_secs(1));
    let calls_text = text_of(&rig, "calls");
    assert!(
        calls_text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
        "{calls_text}"
    );
    assert!(calls_text.contains("name = \"Main\""), "{calls_text}");

    assert!(rig.engine.command(Command::SelectProfile {
        profile: "home".into(),
    }));
    rig.set_color(KEY_3, BLUE);
    rig.poll_for(Duration::from_secs(1));
    let home_text = text_of(&rig, "home");
    assert!(
        home_text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
        "{home_text}"
    );
    assert!(home_text.contains("name = \"Desk\""), "{home_text}");
}

#[test]
fn a_link_moves_and_a_duplicate_and_an_import_do_not_take_it() {
    let mut rig = calls_and_home();
    assert!(text_of(&rig, "calls").contains("us.zoom.xos"));

    // The same link again changes nothing.
    assert!(rig.engine.command(Command::LinkApplication {
        profile: "calls".into(),
        link: link("US.ZOOM.XOS", "Zoom"),
    }));
    assert!(rig.take_events().is_empty());

    assert!(rig.engine.command(Command::LinkApplication {
        profile: "home".into(),
        link: link("com.apple.Safari", "Safari"),
    }));
    rig.poll_for(Duration::from_secs(1));
    assert!(text_of(&rig, "home").contains("com.apple.Safari"));
    assert!(text_of(&rig, "calls").contains("us.zoom.xos"));

    // The move writes both profiles.
    assert!(rig.engine.command(Command::LinkApplication {
        profile: "home".into(),
        link: link("us.zoom.xos", "zoom.us"),
    }));
    rig.poll_for(Duration::from_secs(1));
    let calls = text_of(&rig, "calls");
    let home = text_of(&rig, "home");
    assert!(!calls.contains("zoom"), "{calls}");
    assert!(home.contains("us.zoom.xos"), "{home}");

    assert!(rig.engine.command(Command::AddProfile {
        name: "Copy".into(),
        copy_of: Some("home".into()),
    }));
    assert!(rig.engine.profile().applications.is_empty());
    assert!(text_of(&rig, "home").contains("us.zoom.xos"));

    let incoming = {
        let mut profile = plain("Guest");
        profile.applications.push(link("us.zoom.xos", "zoom.us"));
        profile
            .applications
            .push(link("com.apple.calculator", "Calculator"));
        profile
    };
    let export_dir = TempDir::new().unwrap();
    let store = ProfilesDir::new(export_dir.path()).store("guest");
    store.save(&incoming).unwrap();
    let file = export_dir.path().join("guest.opencraft-profile");
    opencraft_engine::share::export(&incoming, &store, &file).unwrap();
    assert!(rig.engine.command(Command::ImportProfile { path: file }));
    let events = rig.take_events();
    assert_eq!(problems(&events), ["“zoom.us” stays linked to “Home”."]);
    assert_eq!(
        rig.engine.profile().applications,
        [link("com.apple.calculator", "Calculator")]
    );

    assert!(rig.engine.command(Command::AddStarterProfile {
        profile: {
            let mut profile = plain("Starter");
            profile
                .applications
                .push(link("com.apple.calculator", "Calculator"));
            profile
        },
    }));
    let events = rig.take_events();
    assert_eq!(problems(&events), ["“Calculator” stays linked to “Guest”."]);
    assert!(rig.engine.profile().applications.is_empty());
}

#[test]
fn a_bad_link_and_a_33rd_link_do_not_change_the_profile() {
    let mut rig = calls_and_home();
    for (bundle_id, name) in [("not valid", "Zoom"), ("com.apple.Safari", "  ")] {
        assert!(rig.engine.command(Command::LinkApplication {
            profile: "calls".into(),
            link: link(bundle_id, name),
        }));
    }
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 2, "{events:?}");
    assert_eq!(rig.engine.profile().applications.len(), 1);

    assert!(rig.engine.command(Command::UnlinkApplication {
        profile: "calls".into(),
        bundle_id: "missing".into(),
    }));
    assert!(rig.take_events().is_empty());
    assert!(rig.engine.command(Command::UnlinkApplication {
        profile: "calls".into(),
        bundle_id: "US.ZOOM.XOS".into(),
    }));
    assert!(rig.engine.profile().applications.is_empty());

    for number in 1..=32 {
        assert!(rig.engine.command(Command::LinkApplication {
            profile: "calls".into(),
            link: link(
                &format!("com.example.app{number}"),
                &format!("App {number}")
            ),
        }));
    }
    assert_eq!(rig.engine.profile().applications.len(), 32);
    assert!(rig.engine.command(Command::LinkApplication {
        profile: "calls".into(),
        link: link("com.example.extra", "Extra"),
    }));
    let events = rig.take_events();
    assert!(
        problems(&events)
            .iter()
            .any(|text| text.contains("32 links")),
        "{events:?}"
    );
    assert_eq!(rig.engine.profile().applications.len(), 32);
}

#[test]
fn a_clash_in_the_files_keeps_the_first_profile() {
    let dir = TempDir::new().unwrap();
    let mut calls = plain("Calls");
    calls.applications.push(link("us.zoom.xos", "zoom.us"));
    let mut home = plain("Home");
    home.applications.push(link("US.ZOOM.XOS", "Zoom"));
    save_at(dir.path(), "calls", &calls);
    save_at(dir.path(), "home", &home);
    let mut rig = Rig::in_dir(dir);
    let events = rig.take_events();
    let Event::Profiles { profiles, .. } = events
        .iter()
        .find(|event| matches!(event, Event::Profiles { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    assert!(profiles
        .iter()
        .find(|entry| entry.id == "calls")
        .unwrap()
        .clashes
        .is_empty());
    assert_eq!(
        profiles
            .iter()
            .find(|entry| entry.id == "home")
            .unwrap()
            .clashes,
        ["US.ZOOM.XOS"]
    );
    // The files stay as they are.
    assert!(text_of(&rig, "home").contains("US.ZOOM.XOS"));
    front(&mut rig, bundle("us.zoom.xos"));
    assert_eq!(rig.engine.active_profile(), "calls");
}

#[test]
fn the_default_profile_follows_the_mark_and_then_the_rule() {
    let mut rig = calls_and_home();
    let events = {
        front(&mut rig, FrontApplication::NoBundle);
        rig.take_events()
    };
    let Event::Profiles {
        default,
        cause,
        active,
        ..
    } = profiles_of(&events).last().unwrap()
    else {
        unreachable!()
    };
    assert_eq!(active, "home");
    assert_eq!(default, "home");
    assert_eq!(cause, &ActiveCause::Default);

    assert!(rig.engine.command(Command::SetDefaultProfile {
        profile: "calls".into(),
    }));
    assert!(rig.engine.command(Command::SetDefaultProfile {
        profile: "missing".into(),
    }));
    let events = rig.take_events();
    assert_eq!(
        problems(&events),
        ["OpenCraft did not change the default profile, because no profile has the identity “missing”."]
    );
    front(&mut rig, FrontApplication::NoBundle);
    assert_eq!(rig.engine.active_profile(), "calls");

    assert!(rig.engine.command(Command::RemoveProfile {
        profile: "calls".into(),
    }));
    let events = rig.take_events();
    let Event::Profiles {
        default, active, ..
    } = profiles_of(&events).last().unwrap()
    else {
        unreachable!()
    };
    assert_eq!(active, "home");
    assert_eq!(default, "home");
}

#[test]
fn the_switch_follows_the_front_application_and_keeps_a_manual_choice() {
    let mut rig = calls_and_home();
    assert!(rig.engine.command(Command::SelectPage { page: 1 }));
    rig.take_events();
    front(&mut rig, bundle("us.zoom.xos"));
    assert_eq!(rig.engine.active_profile(), "calls");
    assert_eq!(
        rig.engine.active_page(),
        1,
        "the same profile is not loaded again"
    );
    let events = rig.take_events();
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Profiles {
            cause: ActiveCause::Linked { bundle_id },
            ..
        } if bundle_id == "us.zoom.xos"
    )));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::Pages { .. })));

    // OpenCraft in front, then back to the same application: no load.
    front(&mut rig, FrontApplication::Shell);
    front(&mut rig, bundle("US.ZOOM.XOS"));
    assert_eq!(rig.engine.active_profile(), "calls");
    assert_eq!(rig.engine.active_page(), 1);
    assert!(rig.take_events().is_empty());

    // A selection in the window comes back to the linked profile.
    front(&mut rig, FrontApplication::Shell);
    assert!(rig.engine.command(Command::SelectProfile {
        profile: "home".into(),
    }));
    front(&mut rig, bundle("us.zoom.xos"));
    assert_eq!(rig.engine.active_profile(), "calls");

    // A key selection between two applications with no link stays.
    front(&mut rig, bundle("com.apple.Safari"));
    assert_eq!(rig.engine.active_profile(), "home");
    select_key(&mut rig, "calls");
    assert_eq!(rig.engine.active_profile(), "calls");
    front(&mut rig, bundle("com.apple.Terminal"));
    assert_eq!(rig.engine.active_profile(), "calls");

    // A key selection while OpenCraft is in front does not stay.
    front(&mut rig, FrontApplication::Shell);
    select_key(&mut rig, "home");
    front(&mut rig, bundle("us.zoom.xos"));
    assert_eq!(rig.engine.active_profile(), "calls");
}

#[test]
fn a_lock_and_a_sleep_do_not_change_the_profile() {
    let mut rig = calls_and_home();
    front(&mut rig, bundle("us.zoom.xos"));
    assert!(rig
        .engine
        .command(Command::PowerNotice(opencraft_engine::PowerNotice::Locked)));
    front(&mut rig, bundle("com.apple.Safari"));
    assert_eq!(rig.engine.active_profile(), "calls");
    assert!(rig.engine.command(Command::PowerNotice(
        opencraft_engine::PowerNotice::Unlocked
    )));
    front(&mut rig, bundle("com.apple.Safari"));
    assert_eq!(rig.engine.active_profile(), "home");

    front(&mut rig, bundle("us.zoom.xos"));
    assert!(rig.engine.command(Command::PowerNotice(
        opencraft_engine::PowerNotice::WillSleep
    )));
    front(&mut rig, bundle("com.apple.Safari"));
    assert_eq!(rig.engine.active_profile(), "calls");
    assert!(rig
        .engine
        .command(Command::PowerNotice(opencraft_engine::PowerNotice::DidWake)));
    front(&mut rig, bundle("com.apple.Safari"));
    assert_eq!(rig.engine.active_profile(), "home");

    // The sleep of the displays is not a lock and not a sleep of the computer.
    front(&mut rig, bundle("us.zoom.xos"));
    assert!(rig.engine.command(Command::PowerNotice(
        opencraft_engine::PowerNotice::DisplaysSleep
    )));
    front(&mut rig, bundle("com.apple.Safari"));
    assert_eq!(rig.engine.active_profile(), "home");
}

#[test]
fn the_setting_turns_the_switch_off_until_the_next_change() {
    let mut rig = calls_and_home();
    let start = Rig::new();
    assert!(start
        .events
        .borrow()
        .iter()
        .any(|event| matches!(event, Event::FollowFrontApplication(true))));
    drop(start);

    assert!(rig
        .engine
        .command(Command::SetFollowFrontApplication(false)));
    let events = rig.take_events();
    assert!(events.contains(&Event::FollowFrontApplication(false)));
    front(&mut rig, bundle("com.apple.Safari"));
    assert_eq!(rig.engine.active_profile(), "calls");

    assert!(rig.engine.command(Command::SetFollowFrontApplication(true)));
    assert!(rig
        .take_events()
        .contains(&Event::FollowFrontApplication(true)));
    assert_eq!(rig.engine.active_profile(), "calls");
    front(&mut rig, bundle("com.apple.Safari"));
    assert_eq!(rig.engine.active_profile(), "home");

    let rig = rig.restart();
    assert!(rig
        .events
        .borrow()
        .iter()
        .any(|event| matches!(event, Event::FollowFrontApplication(true))));
}

/// Two profiles link the same application in their files: "a" has the link
/// that applies, and the link of "b" is in `clashes`.
fn two_with_a_clash() -> Rig {
    let dir = TempDir::new().unwrap();
    let mut a = plain("A");
    a.applications.push(link("com.example.x", "Example"));
    let mut b = plain("B");
    b.applications.push(link("com.example.x", "Example"));
    save_at(dir.path(), "a", &a);
    save_at(dir.path(), "b", &b);
    save_at(dir.path(), "c", &plain("C"));
    let mut rig = Rig::in_dir(dir);
    rig.take_events();
    rig
}

#[test]
fn the_link_command_moves_a_clashing_link_to_the_profile_that_it_names() {
    let mut rig = two_with_a_clash();
    assert!(rig.engine.command(Command::LinkApplication {
        profile: "b".into(),
        link: link("com.example.x", "Example"),
    }));
    // "a" is the active profile: the engine writes it after the delay.
    rig.poll_for(SAVE_DELAY + POLL_INTERVAL);
    assert!(
        !text_of(&rig, "a").contains("com.example.x"),
        "the link moved out of a"
    );
    assert!(text_of(&rig, "b").contains("com.example.x"));
    let events = rig.take_events();
    let Some(Event::Profiles { profiles, .. }) = profiles_of(&events).pop() else {
        panic!("no profiles event in {events:?}");
    };
    let b = profiles.iter().find(|entry| entry.id == "b").unwrap();
    assert_eq!(b.applications.len(), 1, "b keeps one link");
    assert!(b.clashes.is_empty());
    front(&mut rig, bundle("com.example.x"));
    assert_eq!(rig.engine.active_profile(), "b");
    rig.take_events();

    // A link that applies already gives no change.
    assert!(rig.engine.command(Command::LinkApplication {
        profile: "b".into(),
        link: link("com.example.x", "Example"),
    }));
    assert!(profiles_of(&rig.take_events()).is_empty());
}

#[test]
fn the_setting_off_stays_after_a_profile_change_and_a_restart() {
    let mut rig = two_with_a_clash();
    assert!(rig
        .engine
        .command(Command::SetFollowFrontApplication(false)));
    assert!(rig.engine.command(Command::SelectProfile {
        profile: "c".into()
    }));
    assert!(rig.engine.command(Command::SetDefaultProfile {
        profile: "c".into()
    }));
    let rig = rig.restart();
    assert!(rig
        .events
        .borrow()
        .iter()
        .any(|event| matches!(event, Event::FollowFrontApplication(false))));
}

/// The names of the last history event in `events`.
fn last_history(events: &[Event]) -> Option<(Option<String>, Option<String>)> {
    events.iter().rev().find_map(|event| match event {
        Event::History { undo, redo } => Some((undo.clone(), redo.clone())),
        _ => None,
    })
}

/// Go from "calls" to "home" with an automatic switch. "home" has no link,
/// so the front application `NoBundle` gives it.
fn switch_to_home(rig: &mut Rig) {
    front(rig, bundle("us.zoom.xos"));
    front(rig, FrontApplication::NoBundle);
    assert_eq!(rig.engine.active_profile(), "home");
}

/// A trip to an application of a different profile and back keeps the
/// undo history of each profile (decision 0030).
#[test]
fn an_automatic_switch_and_back_keeps_the_undo_of_a_folder_removal() {
    let mut rig = calls_and_home();
    switch_to_home(&mut rig);
    let folder = opencraft_engine::FolderId::from_raw(7).unwrap();
    assert!(rig.engine.command(Command::AddFolder {
        id: folder,
        name: Some("Tools".into()),
        copy_of: None,
    }));
    assert!(rig.engine.command(Command::CloseFolder));
    assert!(rig.engine.command(Command::RemoveFolder { folder }));
    assert!(rig.engine.profile().folders().is_empty());
    rig.take_events();

    front(&mut rig, bundle("us.zoom.xos"));
    assert_eq!(rig.engine.active_profile(), "calls");
    assert_eq!(last_history(&rig.take_events()), Some((None, None)));
    assert!(rig.engine.command(Command::Undo));
    assert!(
        last_history(&rig.take_events()).is_none(),
        "calls has nothing to undo"
    );

    front(&mut rig, FrontApplication::NoBundle);
    assert_eq!(rig.engine.active_profile(), "home");
    assert_eq!(
        last_history(&rig.take_events()),
        Some((Some("Remove Folder".into()), None))
    );
    assert!(rig.engine.command(Command::Undo));
    assert_eq!(rig.engine.profile().folders().len(), 1);
    assert_eq!(rig.engine.profile().folders()[0].page.name, "Tools");
    assert_eq!(
        last_history(&rig.take_events()),
        Some((Some("Add Folder".into()), Some("Remove Folder".into())))
    );
}

#[test]
fn an_automatic_switch_and_back_keeps_the_undo_and_the_redo_of_an_edit() {
    let mut rig = calls_and_home();
    switch_to_home(&mut rig);
    rig.set_color(KEY_3, RED);
    rig.set_color(KEY_3, BLUE);
    assert!(rig.engine.command(Command::Undo));

    front(&mut rig, bundle("us.zoom.xos"));
    rig.set_color(KEY_3, GREEN);
    front(&mut rig, FrontApplication::NoBundle);
    assert_eq!(
        last_history(&rig.take_events()),
        Some((Some("Set Fill".into()), Some("Set Fill".into())))
    );
    assert_eq!(
        rig.engine.profile().pages()[0].content(KEY_3).color,
        Some(RED)
    );
    assert!(rig.engine.command(Command::Redo));
    assert_eq!(
        rig.engine.profile().pages()[0].content(KEY_3).color,
        Some(BLUE)
    );
    assert!(rig.engine.command(Command::Undo));
    assert!(rig.engine.command(Command::Undo));
    assert_eq!(rig.engine.profile().pages()[0].content(KEY_3).color, None);

    // The history of "calls" is its own.
    front(&mut rig, bundle("us.zoom.xos"));
    assert!(rig.engine.command(Command::Undo));
    assert_eq!(rig.engine.profile().pages()[0].content(KEY_3).color, None);
}

/// The load of a profile removes the images that it does not use, but it
/// keeps the images of the steps that the history keeps for it.
#[test]
fn an_undo_after_a_switch_finds_the_image_that_the_user_removed() {
    let mut rig = calls_and_home();
    switch_to_home(&mut rig);
    let file = green_file(rig.dir.path());
    assert!(rig.engine.command(Command::SetImage {
        surface: KEY_3,
        path: Some(file),
    }));
    assert!(rig.engine.command(Command::SetImage {
        surface: KEY_3,
        path: None,
    }));
    front(&mut rig, bundle("us.zoom.xos"));
    front(&mut rig, FrontApplication::NoBundle);
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    assert!(rig.engine.command(Command::Undo));
    let events = rig.take_events();
    let state = last_state(&events, KEY_3);
    let copy = state.content.image.clone().expect("the image is back");
    let home = ProfilesDir::new(rig.dir.path()).store("home");
    assert!(home.dir().join(&copy).exists(), "{copy}");
    assert_color(&state.image, 56, 56, GREEN);
}

#[test]
fn a_removed_profile_loses_its_history() {
    let mut rig = calls_and_home();
    switch_to_home(&mut rig);
    rig.set_color(KEY_3, RED);
    assert!(rig.engine.command(Command::SelectProfile {
        profile: "calls".into()
    }));
    assert!(rig.engine.command(Command::RemoveProfile {
        profile: "home".into()
    }));
    // A new profile with the same name gets the same identity.
    assert!(rig.engine.command(Command::AddProfile {
        name: "Home".into(),
        copy_of: None,
    }));
    assert_eq!(rig.engine.active_profile(), "home");
    assert_eq!(last_history(&rig.take_events()), Some((None, None)));
    assert!(rig.engine.command(Command::Undo));
    assert!(last_history(&rig.take_events()).is_none());
}
