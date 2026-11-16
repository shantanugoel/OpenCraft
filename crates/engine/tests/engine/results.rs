use super::*;

#[test]
fn a_result_follows_the_change_and_its_history_event() {
    let mut rig = Rig::new();
    rig.take_events();
    let (go_on, result) = rig.engine.command_with_result(Command::SetLabel {
        surface: KEY_3,
        label: Some("Ready".into()),
    });
    assert!(go_on);
    assert_eq!(result, Ok(()));
    let events = rig.take_events();
    assert_eq!(surfaces(&events)[0].content.label.as_deref(), Some("Ready"));
    assert!(matches!(
        events.last(),
        Some(Event::History { undo: Some(_), .. })
    ));
}

#[test]
fn a_problem_is_both_the_result_and_the_shell_event() {
    let mut rig = Rig::new();
    rig.take_events();
    let (go_on, result) = rig
        .engine
        .command_with_result(Command::SelectPage { page: usize::MAX });
    assert!(go_on);
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 1);
    assert_eq!(result, Err(problems(&events)[0].to_string()));

    // A problem of an earlier command is not part of this result.
    assert_eq!(
        rig.engine.command_with_result(Command::ReadCounters),
        (true, Ok(()))
    );
}

#[test]
fn a_group_has_one_result_and_keeps_the_first_problem() {
    let mut rig = Rig::new();
    rig.take_events();
    let (go_on, result) = rig.engine.command_with_result(Command::Group {
        name: "Change labels".into(),
        commands: vec![
            Command::SelectPage { page: usize::MAX },
            Command::InProfile {
                profile: "not-active".into(),
                command: Box::new(Command::ReadCounters),
            },
            Command::SetLabel {
                surface: KEY_3,
                label: Some("Still done".into()),
            },
        ],
    });
    assert!(go_on);
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 2);
    assert_eq!(result, Err(problems(&events)[0].to_string()));
    assert_eq!(
        surfaces(&events)[0].content.label.as_deref(),
        Some("Still done")
    );
    assert!(
        matches!(events.last(), Some(Event::History { undo: Some(name), .. }) if name == "Change labels")
    );
}

#[test]
fn a_profile_guard_has_the_result_of_the_whole_group_or_its_refusal() {
    let mut rig = Rig::new();
    rig.take_events();
    let change = || Command::Group {
        name: "Change label".into(),
        commands: vec![Command::SetLabel {
            surface: KEY_3,
            label: Some("Guarded".into()),
        }],
    };
    let (_, result) = rig.engine.command_with_result(Command::InProfile {
        profile: "not-active".into(),
        command: Box::new(change()),
    });
    let events = rig.take_events();
    assert_eq!(result, Err(problems(&events)[0].to_string()));
    assert!(surfaces(&events).is_empty());

    assert_eq!(
        rig.engine.command_with_result(Command::InProfile {
            profile: "default".into(),
            command: Box::new(change()),
        }),
        (true, Ok(()))
    );
    let events = rig.take_events();
    assert_eq!(
        surfaces(&events)[0].content.label.as_deref(),
        Some("Guarded")
    );
    assert_eq!(problems(&events), Vec::<&str>::new());
}

// --- A notice is not a refusal (review 1, M4) ----------------------------------

#[test]
fn a_device_error_after_a_correct_change_is_a_notice_and_the_result_is_a_success() {
    let mut rig = Rig::new();
    let device = rig.connect();
    rig.take_events();
    device.borrow_mut().fail_writes("the write failed");
    let (go_on, result) = rig.engine.command_with_result(Command::SetColor {
        surface: KEY_3,
        color: Some(RED),
    });
    assert!(go_on);
    rig.engine.poll(POLL_INTERVAL);
    let events = rig.take_events();
    // The shell still gets the notice about the device.
    assert_eq!(problems(&events).len(), 1);
    // The change is there, thus a sender must not do it again.
    assert_eq!(result, Ok(()));
    assert_eq!(last_state(&events, KEY_3).content.color, Some(RED));
}

// --- A location that is gone changes nothing (review 1, M5) --------------------

fn label_at(page: usize, label: &str) -> Command {
    Command::InProfile {
        profile: "default".into(),
        command: Box::new(Command::Group {
            name: "Set Label".into(),
            commands: vec![Command::AtLocation {
                location: Location {
                    page,
                    open: Vec::new(),
                },
                command: Box::new(Command::SetLabel {
                    surface: KEY_3,
                    label: Some(label.into()),
                }),
            }],
        }),
    }
}

#[test]
fn a_command_at_a_page_that_is_gone_gives_a_problem_and_changes_nothing() {
    let mut rig = Rig::new();
    rig.engine.command(Command::AddPage { copy_of: None });
    rig.engine.command(Command::SelectPage { page: 1 });
    rig.take_events();
    let (go_on, result) = rig.engine.command_with_result(label_at(5, "Wrong"));
    assert!(go_on);
    let events = rig.take_events();
    assert_eq!(result, Err("The profile has no page 5.".to_string()));
    assert_eq!(problems(&events).len(), 1);
    assert!(surfaces(&events).is_empty());
    assert!(!events.iter().any(|e| matches!(e, Event::History { .. })));
    // The active page did not change: a read of the location follows.
    rig.engine.command(Command::ReadProfile {
        id: 1,
        profile: "default".into(),
    });
    let events = rig.take_events();
    assert!(!events.iter().any(|e| matches!(e, Event::Pages { .. })));
}

#[test]
fn a_command_at_a_folder_that_is_gone_gives_a_problem_and_changes_nothing() {
    let mut rig = Rig::new();
    rig.take_events();
    let folder = opencraft_engine::FolderId::from_raw(7).unwrap();
    let (_, result) = rig.engine.command_with_result(Command::AtLocation {
        location: Location {
            page: 0,
            open: vec![folder],
        },
        command: Box::new(Command::SetLabel {
            surface: KEY_3,
            label: Some("Wrong".into()),
        }),
    });
    let events = rig.take_events();
    assert_eq!(
        result,
        Err("The profile does not have the requested folders.".to_string())
    );
    assert!(surfaces(&events).is_empty());
}

#[test]
fn a_command_at_a_location_selects_it_and_is_one_step_that_undo_removes() {
    let mut rig = Rig::new();
    rig.engine.command(Command::AddPage { copy_of: None });
    rig.engine.command(Command::SelectPage { page: 0 });
    rig.take_events();
    assert_eq!(
        rig.engine.command_with_result(label_at(1, "Second")),
        (true, Ok(()))
    );
    let events = rig.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Pages { location, .. } if location.page == 1)));
    assert_eq!(
        last_state(&events, KEY_3).content.label.as_deref(),
        Some("Second")
    );
    assert!(
        matches!(events.last(), Some(Event::History { undo: Some(name), .. }) if name == "Set Label")
    );
    // The step is one step, even while the window edits a text.
    rig.engine.command(Command::StartTextEdit);
    assert_eq!(
        rig.engine.command_with_result(label_at(1, "Third")),
        (true, Ok(()))
    );
    rig.engine.command(Command::EndTextEdit);
    rig.take_events();
    rig.engine.command(Command::Undo);
    let events = rig.take_events();
    assert_eq!(
        last_state(&events, KEY_3).content.label.as_deref(),
        Some("Second")
    );
}

// --- A profile read (review 1, M5 and the content of another location) ---------

#[test]
fn a_profile_read_gives_the_live_profile_without_a_change_of_the_location() {
    let mut rig = Rig::new();
    rig.engine.command(Command::SetLabel {
        surface: KEY_3,
        label: Some("Live".into()),
    });
    rig.take_events();
    rig.engine.command(Command::ReadProfile {
        id: 5,
        profile: "default".into(),
    });
    rig.engine.command(Command::ReadProfile {
        id: 6,
        profile: "missing".into(),
    });
    let events = rig.take_events();
    let reads: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::ProfileRead { id, profile } => Some((*id, profile)),
            _ => None,
        })
        .collect();
    assert_eq!(reads.len(), 2);
    let live = reads[0].1.as_ref().unwrap();
    assert_eq!(
        Location::default().content(live, KEY_3).label.as_deref(),
        Some("Live")
    );
    assert_eq!(reads[1].0, 6);
    assert!(reads[1].1.is_err());
    assert!(problems(&events).is_empty());
}
