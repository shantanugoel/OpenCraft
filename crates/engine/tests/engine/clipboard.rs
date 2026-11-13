//! Clipboard tests: Copy, Paste, Clear, and Copy to All Pages of a control
//! (decision 0029, section 13), and the undo of each (decision 0030). The
//! tests use the fake transport and no thread.

use super::*;
use opencraft_engine::Page;

const KIND: &str = "test.clipboard";
const KEY_4: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 4);
const BAR_1: SurfaceId = SurfaceId::new(SurfaceKind::BarPanel, 1);

fn control(kind: ControlKind, number: u8) -> ControlId {
    ControlId::new(kind, number)
}

fn on(page: usize, kind: ControlKind, number: u8, gesture: GestureKind) -> BindingId {
    BindingId {
        page,
        ..binding(kind, number, gesture)
    }
}

/// An engine on a saved profile with the pages "A" and "B", and the
/// requests of its one handler. Page A has key 3 with the label "Mail", red,
/// and a binding for the press, and knob 1 with a binding for the push and
/// bar panel 1 with the label "Volume" and a binding for the tap. Page B has
/// key 4 with the label "Old" and a binding for the release. The events and
/// the requests of the start are gone.
fn rig() -> (Rig, mpsc::Receiver<Request>) {
    let dir = TempDir::new().unwrap();
    let mut a = Page::new("A");
    a.update(KEY_3, |c| {
        c.label = Some("Mail".into());
        c.color = Some(RED);
    });
    a.set_binding(
        control(ControlKind::Key, 3),
        GestureKind::Press,
        Some(action(KIND)),
    );
    a.set_binding(
        control(ControlKind::Knob, 1),
        GestureKind::Push,
        Some(action(KIND)),
    );
    a.update(BAR_1, |c| c.label = Some("Volume".into()));
    a.set_binding(
        control(ControlKind::BarPanel, 1),
        GestureKind::Tap,
        Some(action(KIND)),
    );
    let mut b = Page::new("B");
    b.update(KEY_4, |c| c.label = Some("Old".into()));
    b.set_binding(
        control(ControlKind::Key, 4),
        GestureKind::Release,
        Some(action(KIND)),
    );
    store_in(dir.path())
        .save(&Profile::new("Clipboard", vec![a, b]))
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

fn page(rig: &Rig, number: usize) -> &Page {
    &rig.engine.profile().pages()[number]
}

/// The undo name of the last history event since the last call.
fn last_undo(rig: &mut Rig) -> Option<Option<String>> {
    rig.take_events().into_iter().rev().find_map(|e| match e {
        Event::History { undo, .. } => Some(undo),
        _ => None,
    })
}

/// Undo must give `before`, and redo `after`.
fn undo_and_redo(rig: &mut Rig, before: &Profile, after: &Profile) {
    command(rig, Command::Undo);
    assert_eq!(rig.engine.profile(), before, "undo");
    command(rig, Command::Redo);
    assert_eq!(rig.engine.profile(), after, "redo");
}

#[test]
fn a_key_is_copied_and_pasted_on_a_different_key_of_a_different_page() {
    let (mut rig, requests) = rig();
    let key_3 = control(ControlKind::Key, 3);
    let key_4 = control(ControlKind::Key, 4);
    command(&mut rig, Command::CopyControl { control: key_3 });
    let events = rig.take_events();
    assert_eq!(events, [Event::Clipboard(ControlKind::Key)]);
    assert!(rig.engine.profile().pages()[0]
        .content(KEY_3)
        .label
        .is_some());

    command(&mut rig, Command::SelectPage { page: 1 });
    rig.take_events();
    requests.try_iter().count();
    let before = rig.engine.profile().clone();
    command(&mut rig, Command::PasteControl { control: key_4 });

    let pasted = page(&rig, 1);
    assert_eq!(pasted.content(KEY_4), page(&rig, 0).content(KEY_3));
    assert_eq!(
        pasted.binding(key_4, GestureKind::Press),
        Some(&action(KIND))
    );
    assert_eq!(pasted.binding(key_4, GestureKind::Release), None);
    // The handler, the shell, and the device get each change.
    let requests: Vec<_> = requests.try_iter().collect();
    assert!(requests.contains(&Request::Disappear {
        binding: on(1, ControlKind::Key, 4, GestureKind::Release)
    }));
    assert!(requests.contains(&Request::Appear {
        binding: on(1, ControlKind::Key, 4, GestureKind::Press),
        action: action(KIND),
    }));
    let events = rig.take_events();
    assert_eq!(
        last_state(&events, KEY_4).content.label.as_deref(),
        Some("Mail")
    );
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Binding(BindingState { binding, action: Some(_), .. })
            if *binding == on(1, ControlKind::Key, 4, GestureKind::Press)
    )));
    assert_eq!(
        events.iter().rev().find_map(|e| match e {
            Event::History { undo, .. } => Some(undo.clone()),
            _ => None,
        }),
        Some(Some("Paste".to_string())),
        "one step"
    );

    let after = rig.engine.profile().clone();
    undo_and_redo(&mut rig, &before, &after);
}

#[test]
fn a_paste_on_a_control_of_a_different_kind_changes_nothing() {
    let (mut rig, _requests) = rig();
    command(
        &mut rig,
        Command::CopyControl {
            control: control(ControlKind::Key, 3),
        },
    );
    rig.take_events();
    let before = rig.engine.profile().clone();
    command(
        &mut rig,
        Command::PasteControl {
            control: control(ControlKind::Knob, 2),
        },
    );
    assert_eq!(rig.engine.profile(), &before);
    let events = rig.take_events();
    assert_eq!(
        problems(&events),
        ["OpenCraft did not paste: the copy is of a key, and a paste is possible only on a key."]
    );
    assert!(!events.iter().any(|e| matches!(e, Event::History { .. })));
}

#[test]
fn a_paste_with_an_empty_clipboard_changes_nothing() {
    let (mut rig, _requests) = rig();
    let before = rig.engine.profile().clone();
    command(
        &mut rig,
        Command::PasteControl {
            control: control(ControlKind::Key, 3),
        },
    );
    assert_eq!(rig.engine.profile(), &before);
    assert!(rig.take_events().is_empty());
}

#[test]
fn clear_removes_the_appearance_and_the_bindings_as_one_step() {
    let (mut rig, requests) = rig();
    let before = rig.engine.profile().clone();
    command(
        &mut rig,
        Command::ClearControl {
            control: control(ControlKind::Key, 3),
        },
    );
    assert!(page(&rig, 0).content(KEY_3).is_empty());
    assert_eq!(
        page(&rig, 0).binding(control(ControlKind::Key, 3), GestureKind::Press),
        None
    );
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [Request::Disappear {
            binding: on(0, ControlKind::Key, 3, GestureKind::Press)
        }]
    );
    assert_eq!(last_undo(&mut rig), Some(Some("Clear".to_string())));
    let after = rig.engine.profile().clone();
    undo_and_redo(&mut rig, &before, &after);

    // A clear of an empty control is no step.
    command(&mut rig, Command::Undo);
    rig.take_events();
    command(
        &mut rig,
        Command::ClearControl {
            control: control(ControlKind::Key, 5),
        },
    );
    assert_eq!(last_undo(&mut rig), None);
}

#[test]
fn copy_to_all_pages_puts_the_content_on_each_page_as_one_step() {
    let (mut rig, requests) = rig();
    command(&mut rig, Command::AddPage { copy_of: None });
    command(&mut rig, Command::SelectPage { page: 0 });
    rig.take_events();
    requests.try_iter().count();
    let before = rig.engine.profile().clone();
    let key_3 = control(ControlKind::Key, 3);
    command(&mut rig, Command::CopyToAllPages { control: key_3 });

    for number in 1..3 {
        assert_eq!(
            page(&rig, number).content(KEY_3),
            page(&rig, 0).content(KEY_3)
        );
        assert_eq!(
            page(&rig, number).binding(key_3, GestureKind::Press),
            Some(&action(KIND))
        );
    }
    // The other parts of the pages stay.
    assert_eq!(page(&rig, 1).content(KEY_4).label.as_deref(), Some("Old"));
    // The active page does not change: no request to the handler.
    assert_eq!(requests.try_iter().count(), 0);
    assert_eq!(
        last_undo(&mut rig),
        Some(Some("Copy to All Pages".to_string()))
    );
    let after = rig.engine.profile().clone();
    rig.poll_for(SAVE_DELAY + POLL_INTERVAL);
    assert_eq!(&store_in(rig.dir.path()).read().unwrap(), &after);
    undo_and_redo(&mut rig, &before, &after);
}

#[test]
fn a_knob_takes_its_bar_panel_with_it() {
    let (mut rig, _requests) = rig();
    let knob_1 = control(ControlKind::Knob, 1);
    let knob_2 = control(ControlKind::Knob, 2);
    command(&mut rig, Command::CopyControl { control: knob_1 });
    command(&mut rig, Command::PasteControl { control: knob_2 });
    let a = page(&rig, 0);
    assert_eq!(a.binding(knob_2, GestureKind::Push), Some(&action(KIND)));
    assert_eq!(a.content(BAR_2).label.as_deref(), Some("Volume"));
    assert_eq!(
        a.binding(control(ControlKind::BarPanel, 2), GestureKind::Tap),
        Some(&action(KIND))
    );

    command(&mut rig, Command::ClearControl { control: knob_1 });
    let a = page(&rig, 0);
    assert_eq!(a.binding(knob_1, GestureKind::Push), None);
    assert!(a.content(BAR_1).is_empty());
    assert_eq!(
        a.binding(control(ControlKind::BarPanel, 1), GestureKind::Tap),
        None
    );
}

#[test]
fn a_paste_in_a_different_profile_copies_the_image_into_it() {
    let (mut rig, _requests) = rig();
    let file = green_file(rig.dir.path());
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: Some(file.clone()),
        },
    );
    fs::remove_file(&file).unwrap();
    command(
        &mut rig,
        Command::CopyControl {
            control: control(ControlKind::Key, 3),
        },
    );
    command(
        &mut rig,
        Command::AddProfile {
            name: "Second".into(),
            copy_of: None,
        },
    );
    assert_ne!(rig.engine.active_profile(), "default");
    rig.take_events();
    command(
        &mut rig,
        Command::PasteControl {
            control: control(ControlKind::Key, 3),
        },
    );
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    let image = page(&rig, 0).content(KEY_3).image.expect("an image");
    let store = ProfilesDir::new(rig.dir.path()).store(rig.engine.active_profile());
    assert!(store.load_image(&image).is_ok());
}
