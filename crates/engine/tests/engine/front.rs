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
        ["OpenCraft did not change the default profile, because no profile has the identity 'missing'."]
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
