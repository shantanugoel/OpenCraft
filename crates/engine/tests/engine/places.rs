//! A command at a place (decision 0040): a program names the profile, the
//! page by its identity, and the open folders. The engine checks all of it
//! first. Nothing changes for a place that is not there. The tests use
//! temporary directories, the fake transport, and no thread.

use super::*;
use opencraft_engine::profiles::STATE_FILE;
use opencraft_engine::{FolderId, Page, PageId, PageRef, Place};

const F: FolderId = FolderId::from_raw(0xf).unwrap();

/// A profile with the pages `names`. The pages have identities.
fn profile(name: &str, pages: &[&str]) -> Profile {
    Profile::new(name, pages.iter().map(|n| Page::new(*n)).collect())
}

/// The ids of the pages of `profile`.
fn ids(profile: &Profile) -> Vec<PageId> {
    profile.pages().iter().map(Page::id).collect()
}

/// An engine on a directory with the profiles `default` (pages A and B)
/// and `work` (pages X, Y, and Z). The ids of the pages of `work`, which is
/// not active, come with the engine.
fn rig() -> (Rig, Vec<PageId>) {
    let dir = TempDir::new().unwrap();
    let profiles = ProfilesDir::new(dir.path());
    profiles
        .store("default")
        .save(&profile("Default", &["A", "B"]))
        .unwrap();
    let work = profile("Work", &["X", "Y", "Z"]);
    profiles.store("work").save(&work).unwrap();
    let mut rig = Rig::in_dir(dir);
    assert_eq!(rig.engine.active_profile(), "default");
    rig.take_events();
    (rig, ids(&work))
}

fn page(rig: &Rig, number: usize) -> PageRef {
    PageRef {
        number,
        id: rig.engine.profile().pages()[number].id(),
    }
}

fn place(profile: &str, page: Option<PageRef>, open: Option<Vec<FolderId>>) -> Command {
    label_at(
        Place {
            profile: profile.into(),
            page,
            open,
        },
        "Label",
    )
}

fn label_at(place: Place, label: &str) -> Command {
    Command::AtPlace {
        place,
        command: Box::new(Command::SetLabel {
            surface: KEY_3,
            label: Some(label.into()),
        }),
    }
}

fn pages_events(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, Event::Pages { .. }))
        .count()
}

fn profiles_events(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, Event::Profiles { .. }))
        .count()
}

#[test]
fn the_active_place_changes_no_location() {
    let (mut rig, _) = rig();
    rig.engine.command(Command::AddFolder {
        id: F,
        name: None,
        copy_of: None,
    });
    rig.engine.command(Command::OpenFolder { folder: F });
    rig.take_events();
    let here = place(
        "default",
        Some(page(&rig, 0)),
        Some(vec![F]),
    );
    assert_eq!(rig.engine.command_with_result(here), (true, Ok(())));
    let events = rig.take_events();
    assert_eq!(pages_events(&events), 0, "the folder stays open");
    assert_eq!(
        last_state(&events, KEY_3).content.label.as_deref(),
        Some("Label")
    );
    assert_eq!(rig.engine.open_folders(), [F]);

    // A different page, with a folder open there, is one change.
    rig.engine.command(Command::SelectPage { page: 1 });
    rig.engine.command(Command::AddFolder {
        id: FolderId::from_raw(0xe).unwrap(),
        name: None,
        copy_of: None,
    });
    rig.take_events();
    let there = place("default", Some(page(&rig, 0)), Some(vec![F]));
    assert_eq!(rig.engine.command_with_result(there), (true, Ok(())));
    assert_eq!(pages_events(&rig.take_events()), 1);
    assert_eq!(rig.engine.active_page(), 0);
    assert_eq!(rig.engine.open_folders(), [F]);
}

#[test]
fn a_place_with_no_page_keeps_the_page_and_the_folders() {
    let (mut rig, _) = rig();
    rig.engine.command(Command::SelectPage { page: 1 });
    rig.take_events();
    assert_eq!(
        rig.engine
            .command_with_result(place("default", None, None)),
        (true, Ok(()))
    );
    assert_eq!(pages_events(&rig.take_events()), 0);
    assert_eq!(rig.engine.active_page(), 1);
}

#[test]
fn a_place_that_is_not_there_changes_nothing_also_the_active_profile() {
    let (mut rig, work) = rig();
    let here = page(&rig, 1);
    let wrong_id = PageRef {
        number: 1,
        id: PageId::from_raw(5).unwrap(),
    };
    let wrong_number = PageRef {
        number: 0,
        id: work[1],
    };
    let beyond = PageRef {
        number: 9,
        id: work[1],
    };
    let cases = [
        // A page that is gone, in the active profile and in another profile.
        place("default", Some(wrong_id.clone()), None),
        place("work", Some(wrong_id), None),
        place("work", Some(wrong_number), None),
        place("work", Some(beyond), None),
        // The identity of a page of another profile.
        place("default", Some(PageRef { number: 1, id: work[1] }), None),
        // A folder that is not there.
        place("default", Some(here), Some(vec![F])),
        place(
            "work",
            Some(PageRef {
                number: 1,
                id: work[1],
            }),
            Some(vec![F]),
        ),
        // A profile that is not there.
        place("missing", None, None),
    ];
    for command in cases {
        let (go_on, result) = rig.engine.command_with_result(command.clone());
        assert!(go_on);
        assert!(result.is_err(), "{command:?}");
        assert!(rig.take_events().is_empty(), "{command:?}");
        assert_eq!(rig.engine.active_profile(), "default");
        assert_eq!(rig.engine.active_page(), 0);
    }
}

#[test]
fn a_place_in_another_profile_selects_it_and_makes_the_change_in_one_command() {
    let (mut rig, work) = rig();
    let command = label_at(
        Place {
            profile: "work".into(),
            page: Some(PageRef {
                number: 2,
                id: work[2],
            }),
            open: None,
        },
        "Z label",
    );
    assert_eq!(rig.engine.command_with_result(command), (true, Ok(())));
    assert_eq!(rig.engine.active_profile(), "work");
    assert_eq!(rig.engine.active_page(), 2);
    let events = rig.take_events();
    assert_eq!(profiles_events(&events), 1);
    assert_eq!(
        last_state(&events, KEY_3).content.label.as_deref(),
        Some("Z label")
    );
    // The change is one step of the history of the new profile, with the name
    // of the engine.
    assert!(matches!(
        events.last(),
        Some(Event::History { undo: Some(name), .. }) if name == "Set Title"
    ));
    assert_eq!(rig.engine.profile().pages()[2].content(KEY_3).label.as_deref(), Some("Z label"));
}

#[test]
fn a_refusal_of_the_command_goes_back_to_the_profile_and_the_page() {
    let (mut rig, work) = rig();
    rig.engine.command(Command::SelectPage { page: 1 });
    rig.take_events();
    let command = Command::AtPlace {
        place: Place {
            profile: "work".into(),
            page: Some(PageRef {
                number: 2,
                id: work[2],
            }),
            open: None,
        },
        command: Box::new(Command::SetImage {
            surface: KEY_3,
            path: Some("/this/file/is/not/there.png".into()),
        }),
    };
    let (go_on, result) = rig.engine.command_with_result(command);
    assert!(go_on);
    assert!(result.is_err());
    assert_eq!(rig.engine.active_profile(), "default");
    assert_eq!(rig.engine.active_page(), 1);
    // The work profile has no change, and the history of both has no step.
    assert!(rig.engine.profile().pages()[1].content(KEY_3).image.is_none());
    let events = rig.take_events();
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::History { undo: Some(_), .. })));
    let last_profiles = events.iter().rev().find_map(|e| match e {
        Event::Profiles { active, cause, .. } => Some((active.clone(), cause.clone())),
        _ => None,
    });
    assert_eq!(
        last_profiles,
        Some(("default".to_string(), opencraft_engine::ActiveCause::Selected))
    );
}

#[test]
fn a_page_is_named_by_its_identity_not_by_its_number() {
    let (mut rig, _) = rig();
    let b = page(&rig, 1);
    // The user moves page B before page A.
    rig.engine.command(Command::MovePage { page: 1, to: 0 });
    rig.take_events();
    // A program that read the old order names number 1 and the identity of B.
    let (_, result) = rig.engine.command_with_result(place("default", Some(b.clone()), None));
    assert!(result.is_err(), "{result:?}");
    assert!(rig.take_events().is_empty());
    // With the new number, the change is at page B.
    let now = PageRef { number: 0, id: b.id };
    assert_eq!(
        rig.engine
            .command_with_result(place("default", Some(now), None)),
        (true, Ok(()))
    );
    assert_eq!(rig.engine.profile().pages()[0].name, "B");
    assert_eq!(
        rig.engine.profile().pages()[0].content(KEY_3).label.as_deref(),
        Some("Label")
    );
    assert_eq!(rig.engine.profile().pages()[1].content(KEY_3).label, None);
}

#[test]
fn an_undo_shows_the_page_of_the_change() {
    let (mut rig, _) = rig();
    let b = page(&rig, 1);
    assert_eq!(
        rig.engine
            .command_with_result(place("default", Some(b), None)),
        (true, Ok(()))
    );
    assert_eq!(rig.engine.active_page(), 1);
    // The user goes to page A, and then undoes the step of the program.
    rig.engine.command(Command::SelectPage { page: 0 });
    rig.engine.command(Command::Undo);
    assert_eq!(rig.engine.active_page(), 1);
    assert_eq!(rig.engine.profile().pages()[1].content(KEY_3).label, None);
    rig.engine.command(Command::Redo);
    assert_eq!(rig.engine.active_page(), 1);
    assert_eq!(
        rig.engine.profile().pages()[1].content(KEY_3).label.as_deref(),
        Some("Label")
    );
}

#[test]
fn a_page_keeps_its_identity_for_undo_and_for_a_copy_of_the_profile() {
    let (mut rig, _) = rig();
    let before = ids(rig.engine.profile());
    rig.engine.command(Command::RemovePage { page: 0 });
    rig.engine.command(Command::Undo);
    assert_eq!(ids(rig.engine.profile()), before);
    rig.engine.command(Command::AddPage { copy_of: Some(0) });
    let after = ids(rig.engine.profile());
    assert_eq!(after.len(), 3);
    assert_eq!(after[..2], before);
    assert!(!before.contains(&after[2]));
}

#[test]
fn a_failed_record_of_the_profile_is_a_notice_and_the_result_is_a_success() {
    let (mut rig, _) = rig();
    // The state file cannot be written: a directory has its name.
    fs::create_dir(rig.dir.path().join(STATE_FILE)).unwrap();
    rig.take_events();
    let (go_on, result) = rig.engine.command_with_result(Command::AddProfile {
        name: "New".into(),
        copy_of: None,
    });
    assert!(go_on);
    assert_eq!(result, Ok(()));
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 1, "{:?}", problems(&events));
    assert_eq!(rig.engine.profile().name, "New");
}
