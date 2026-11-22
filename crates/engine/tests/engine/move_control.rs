//! Move and swap of two controls on the active page or folder (KEY-14):
//! the appearance, the bindings, the On face, a folder, the gesture timers,
//! and the one step of the history (decision 0030). The tests use the fake
//! transport and no thread.

use super::*;
use opencraft_engine::{EngineRequest, FolderId, Page};

const KIND: &str = "test.move";
const KEY_3: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 3);
const KEY_4: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 4);
const KEY_5: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 5);
const BAR_1: SurfaceId = SurfaceId::new(SurfaceKind::BarPanel, 1);

fn control(kind: ControlKind, number: u8) -> ControlId {
    ControlId::new(kind, number)
}

fn key(number: u8) -> ControlId {
    control(ControlKind::Key, number)
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

fn move_control(rig: &mut Rig, from: ControlId, to: ControlId) {
    command(rig, Command::MoveControl { from, to });
}

/// An engine on a saved profile with one page. Key 3 has the label "Mail",
/// red, the On face "Working", and the Press binding. Key 4 has the label
/// "Old" and the Release binding. Key 5 is empty. Bar panel 1 has the label
/// "Volume" and the Tap binding, and knob 1 has the Push binding. The events
/// of the start are gone.
fn rig() -> Rig {
    let dir = TempDir::new().unwrap();
    let mut a = Page::new("A");
    a.update(KEY_3, |c| {
        c.label = Some("Mail".into());
        c.color = Some(RED);
        c.face_mut(true).label = Some("Working".into());
    });
    a.set_binding(key(3), GestureKind::Press, Some(action(KIND)));
    a.update(KEY_4, |c| c.label = Some("Old".into()));
    a.set_binding(key(4), GestureKind::Release, Some(action(KIND)));
    a.update(BAR_1, |c| c.label = Some("Volume".into()));
    a.set_binding(
        control(ControlKind::BarPanel, 1),
        GestureKind::Tap,
        Some(action(KIND)),
    );
    a.set_binding(
        control(ControlKind::Knob, 1),
        GestureKind::Push,
        Some(action(KIND)),
    );
    store_in(dir.path())
        .save(&Profile::new("Move", vec![a]))
        .unwrap();
    let (port, _requests) = port(KIND);
    let mut rig = Rig::with_ports(dir, vec![port]);
    rig.take_events();
    rig
}

/// The profile after the change, for the undo and the redo.
fn undo_and_redo(rig: &mut Rig, before: &Profile, after: &Profile) {
    command(rig, Command::Undo);
    assert_eq!(rig.engine.profile(), before, "undo");
    command(rig, Command::Redo);
    assert_eq!(rig.engine.profile(), after, "redo");
}

#[test]
fn a_key_moves_to_an_empty_key_in_one_step() {
    let mut rig = rig();
    let mail = page(&rig, 0).content(KEY_3).clone();
    let before = rig.engine.profile().clone();
    move_control(&mut rig, key(3), key(5));
    let events = rig.take_events();
    // The destination has the whole content of the source.
    assert_eq!(page(&rig, 0).content(KEY_5), mail);
    assert_eq!(
        page(&rig, 0).binding(key(5), GestureKind::Press),
        Some(&action(KIND))
    );
    // The source is empty.
    assert!(page(&rig, 0).content(KEY_3).is_empty());
    assert_eq!(page(&rig, 0).binding(key(3), GestureKind::Press), None);
    // The shell gets both surfaces, and one step "Move Key". It has no
    // second step for the second surface.
    let surfaces: Vec<SurfaceId> = surfaces(&events).iter().map(|state| state.id).collect();
    assert_eq!(surfaces, [KEY_5, KEY_3]);
    let names: Vec<Option<String>> = events
        .iter()
        .filter_map(|event| match event {
            Event::History { undo, .. } => Some(undo.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(names, [Some("Move Key".to_string())]);
    let after = rig.engine.profile().clone();
    undo_and_redo(&mut rig, &before, &after);
}

#[test]
fn a_key_on_an_occupied_key_swaps_the_two_keys() {
    let mut rig = rig();
    let mail = page(&rig, 0).content(KEY_3).clone();
    let old = page(&rig, 0).content(KEY_4).clone();
    let before = rig.engine.profile().clone();
    move_control(&mut rig, key(3), key(4));
    assert_eq!(page(&rig, 0).content(KEY_4), mail);
    assert_eq!(page(&rig, 0).content(KEY_3), old);
    assert_eq!(
        page(&rig, 0).binding(key(4), GestureKind::Press),
        Some(&action(KIND))
    );
    assert_eq!(
        page(&rig, 0).binding(key(3), GestureKind::Release),
        Some(&action(KIND))
    );
    assert_eq!(page(&rig, 0).binding(key(4), GestureKind::Release), None);
    assert_eq!(page(&rig, 0).binding(key(3), GestureKind::Press), None);
    assert_eq!(last_undo(&mut rig), Some(Some("Move Key".into())));
    let after = rig.engine.profile().clone();
    undo_and_redo(&mut rig, &before, &after);
}

#[test]
fn the_on_face_and_the_images_move_with_the_key() {
    let mut rig = rig();
    command(
        &mut rig,
        Command::OnFace(Box::new(Command::SetLabel {
            surface: KEY_3,
            label: Some("On".into()),
        })),
    );
    command(
        &mut rig,
        Command::OnFace(Box::new(Command::SetColor {
            surface: KEY_3,
            color: Some(BLUE),
        })),
    );
    rig.take_events();
    let face = |rig: &Rig, surface: SurfaceId| {
        rig.engine.profile().pages()[0]
            .content(surface)
            .face(true)
            .label
            .clone()
    };
    assert_eq!(face(&rig, KEY_3).as_deref(), Some("On"));
    // An image goes with the appearance of the key.
    let directory = TempDir::new().unwrap();
    let file = directory.path().join("icon.png");
    RgbImage::from_pixel(4, 4, Rgb([200, 30, 30]))
        .save(&file)
        .unwrap();
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: Some(file),
        },
    );
    let image = page(&rig, 0).content(KEY_3).image.clone();
    assert!(image.is_some(), "the key has an image");
    move_control(&mut rig, key(3), key(5));
    assert_eq!(face(&rig, KEY_5).as_deref(), Some("On"));
    assert_eq!(face(&rig, KEY_3), None);
    assert_eq!(page(&rig, 0).content(KEY_5).image, image);
    assert_eq!(page(&rig, 0).content(KEY_3).image, None);
    // The base face of the source is empty, and the destination keeps the
    // base face of the source.
    assert_eq!(page(&rig, 0).content(KEY_5).label.as_deref(), Some("Mail"));
}

#[test]
fn a_key_of_a_folder_moves_inside_the_folder_and_the_page_keeps_its_keys() {
    let mut rig = rig();
    let folder = FolderId::from_raw(7).unwrap();
    command(
        &mut rig,
        Command::AddFolder {
            id: folder,
            name: Some("Tools".into()),
            copy_of: None,
        },
    );
    rig.engine
        .handler_message(Message::Engine(EngineRequest::OpenFolder(folder)));
    // The folder has key 3 and key 5 now. The page under it has key 3
    // "Mail".
    let folder_page = |rig: &Rig| {
        rig.engine
            .profile()
            .folder(folder)
            .expect("the folder")
            .page
            .clone()
    };
    command(
        &mut rig,
        Command::SetLabel {
            surface: KEY_3,
            label: Some("In folder".into()),
        },
    );
    command(
        &mut rig,
        Command::SetBinding {
            binding: BindingId {
                folder: Some(folder),
                ..binding(ControlKind::Key, 3, GestureKind::Press)
            },
            action: Some(action(KIND)),
        },
    );
    assert_eq!(
        page(&rig, 0).content(KEY_3).label.as_deref(),
        Some("Mail"),
        "the page under the folder does not change"
    );
    move_control(&mut rig, key(3), key(5));
    let folder_page = folder_page(&rig);
    assert_eq!(
        folder_page.content(KEY_5).label.as_deref(),
        Some("In folder")
    );
    assert!(folder_page.content(KEY_3).is_empty());
    assert_eq!(
        folder_page.binding(key(5), GestureKind::Press),
        Some(&action(KIND))
    );
    assert_eq!(folder_page.binding(key(3), GestureKind::Press), None);
    assert_eq!(page(&rig, 0).content(KEY_3).label.as_deref(), Some("Mail"));
    assert_eq!(last_undo(&mut rig), Some(Some("Move Key".into())));
}

#[test]
fn a_move_of_a_bar_panel_or_a_knob_swaps_that_control() {
    let mut rig = rig();
    let bar = control(ControlKind::BarPanel, 1);
    let bar_2 = control(ControlKind::BarPanel, 2);
    move_control(&mut rig, bar, bar_2);
    assert_eq!(
        page(&rig, 0).content(BAR_1).label,
        None,
        "the content follows the control"
    );
    assert_eq!(
        page(&rig, 0).binding(bar_2, GestureKind::Tap),
        Some(&action(KIND))
    );
    assert_eq!(page(&rig, 0).binding(bar, GestureKind::Tap), None);
    assert_eq!(last_undo(&mut rig), Some(Some("Move Control".into())));
    // A knob has its feedback bar panel: a move takes both.
    let knob = control(ControlKind::Knob, 1);
    let knob_2 = control(ControlKind::Knob, 2);
    move_control(&mut rig, knob, knob_2);
    assert_eq!(
        page(&rig, 0).binding(knob_2, GestureKind::Push),
        Some(&action(KIND))
    );
    assert_eq!(page(&rig, 0).binding(knob, GestureKind::Push), None);
}

#[test]
fn a_move_onto_itself_or_onto_another_kind_changes_nothing() {
    let mut rig = rig();
    let before = rig.engine.profile().clone();
    move_control(&mut rig, key(3), key(3));
    assert_eq!(rig.take_events(), []);
    assert_eq!(rig.engine.profile(), &before);
    // A key does not go onto a bar panel.
    move_control(&mut rig, key(3), control(ControlKind::BarPanel, 1));
    let events = rig.take_events();
    assert_eq!(rig.engine.profile(), &before);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Problem(_))),
        "{events:?}"
    );
}

#[test]
fn a_move_cancels_the_gesture_timers_of_both_keys() {
    let (mut port, requests) = port(KIND);
    port.kinds = vec![description(KIND).with_handles_long_press()];
    let mut rig = Rig::with_ports(TempDir::new().unwrap(), vec![port]);
    for number in [3, 4] {
        set_binding(
            &mut rig,
            binding(ControlKind::Key, number, GestureKind::Press),
            Some(action(KIND)),
        );
        set_binding(
            &mut rig,
            binding(ControlKind::Key, number, GestureKind::DoublePress),
            Some(action(KIND)),
        );
    }
    let device = rig.connect();
    rig.take_events();
    requests.try_iter().count();
    // A press of each key starts the window of the double press.
    for code in [3, 4] {
        device.borrow_mut().push_input(control_report(code, 1));
        device.borrow_mut().push_input(control_report(code, 0));
        rig.engine.poll(Duration::ZERO);
    }
    move_control(&mut rig, key(3), key(4));
    rig.world
        .now
        .set(rig.world.now.get() + Duration::from_millis(500));
    rig.engine.poll(Duration::ZERO);
    let invokes: Vec<GestureKind> = requests
        .try_iter()
        .filter_map(|request| match request {
            Request::Invoke { binding, .. } => Some(binding.gesture),
            _ => None,
        })
        .collect();
    assert!(invokes.is_empty(), "{invokes:?}");
}

#[test]
fn a_move_of_an_empty_key_changes_nothing() {
    let mut rig = rig();
    let before = rig.engine.profile().clone();
    move_control(&mut rig, key(5), key(3));
    assert_eq!(rig.take_events(), []);
    assert_eq!(rig.engine.profile(), &before);
}
