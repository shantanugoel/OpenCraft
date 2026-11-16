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
