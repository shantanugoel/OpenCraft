//! A drag of a key face onto another key face (KEY-14): the command, the
//! selection, the drop target, and the keys that send nothing. The engine
//! does the move, the swap, and the undo; these tests check what the window
//! sends.

use super::*;
use opencraft_engine::{ControlLayout, SurfaceLayout};

const KEY_2: ControlId = ControlId::new(ControlKind::Key, 2);
const KEY_4: ControlId = ControlId::new(ControlKind::Key, 4);
const KEY_8: ControlId = ControlId::new(ControlKind::Key, 8);
const KEY_FACE_4: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 4);
const KEY_FACE_8: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 8);

fn key_face_4() -> SurfaceId {
    KEY_FACE_4
}

/// The layout of [`super::layout`] with key faces 4 and 8.
fn three_keys() -> Event {
    let Event::Layout(layout) = layout() else {
        unreachable!()
    };
    let mut layout = (*layout).clone();
    for (id, x) in [(KEY_FACE_4, 120), (KEY_FACE_8, 220)] {
        layout.surfaces.push(SurfaceLayout {
            id,
            x,
            y: 10,
            width: 90,
            height: 90,
        });
    }
    for (id, column) in [(KEY_4, 2), (KEY_8, 3)] {
        let surface = SurfaceId::new(SurfaceKind::KeyFace, id.number);
        layout.controls.push(ControlLayout {
            id,
            row: 1,
            column,
            surface: Some(surface),
            feedback_surface: Some(surface),
        });
    }
    Event::Layout(Arc::new(layout))
}

/// A window with three key faces: key 2 "Mail", and the empty keys 4 and 8.
fn three_key_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    tx.try_send(three_keys()).unwrap();
    cx.run_until_parked();
    (view, cx, tx, sent)
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

/// The commands of the canvas keys in `sent`: all others are not of this
/// test.
fn moves(sent: &Commands) -> Vec<Command> {
    sent.borrow()
        .iter()
        .filter(|command| matches!(command, Command::MoveControl { .. }))
        .cloned()
        .collect()
}

/// Press on the element `from` and move the pointer to the centre of `to`
/// in eight steps, with the left button down.
fn drag_over(cx: &mut VisualTestContext, from: &'static str, to: &'static str) {
    let start = cx
        .debug_bounds(from)
        .unwrap_or_else(|| panic!("No element: {from}"))
        .center();
    let end = cx
        .debug_bounds(to)
        .unwrap_or_else(|| panic!("No element: {to}"))
        .center();
    cx.simulate_mouse_move(start, None, Modifiers::default());
    cx.simulate_mouse_down(start, gpui_kit::MouseButton::Left, Modifiers::default());
    for step in 1..=8 {
        let f = step as f32 / 8.;
        cx.simulate_mouse_move(
            gpui_kit::point(
                start.x + (end.x - start.x) * f,
                start.y + (end.y - start.y) * f,
            ),
            Some(gpui_kit::MouseButton::Left),
            Modifiers::default(),
        );
    }
    cx.run_until_parked();
}

/// Release the left button at the centre of `to`: the drop.
fn release(cx: &mut VisualTestContext, to: &'static str) {
    let end = cx
        .debug_bounds(to)
        .unwrap_or_else(|| panic!("No element: {to}"))
        .center();
    cx.simulate_mouse_up(end, gpui_kit::MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
}

fn drag_to(cx: &mut VisualTestContext, from: &'static str, to: &'static str) {
    drag_over(cx, from, to);
    release(cx, to);
}

fn message(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| view.read(cx).status_message.as_ref().map(|m| m.to_string()))
}

#[gpui_kit::test]
fn a_key_dropped_on_an_empty_key_moves_and_the_destination_is_selected(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = three_key_window(cx);
    drag_to(cx, "surface-key face 2", "surface-key face 4");
    assert_eq!(
        moves(&sent),
        [Command::MoveControl {
            from: KEY_2,
            to: KEY_4,
        }]
    );
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, Some(key_face_4())));
    assert_eq!(message(&view, cx).as_deref(), Some("Key 2 moves to Key 4"));
}

#[gpui_kit::test]
fn a_key_dropped_on_a_key_with_content_swaps_them(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, surface(key_face_4(), "Other"));
    tx.try_send(three_keys()).unwrap();
    cx.run_until_parked();
    drag_to(cx, "surface-key face 2", "surface-key face 4");
    assert_eq!(
        moves(&sent),
        [Command::MoveControl {
            from: KEY_2,
            to: KEY_4,
        }]
    );
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, Some(key_face_4())));
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("Key 2 and Key 4 are swapped")
    );
}

#[gpui_kit::test]
fn a_key_dropped_on_itself_sends_nothing(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = three_key_window(cx);
    drag_to(cx, "surface-key face 2", "surface-key face 2");
    assert!(moves(&sent).is_empty(), "{:?}", sent.borrow());
    // The key stays the selection.
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, Some(key())));
}

#[gpui_kit::test]
fn a_click_on_a_key_selects_it_and_sends_no_move(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = three_key_window(cx);
    click(cx, "surface-key face 4");
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, Some(key_face_4())));
    assert!(moves(&sent).is_empty(), "{:?}", sent.borrow());
}

#[gpui_kit::test]
fn an_empty_key_is_not_a_source(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = three_key_window(cx);
    drag_to(cx, "surface-key face 4", "surface-key face 2");
    assert!(moves(&sent).is_empty(), "{:?}", sent.borrow());
}

#[gpui_kit::test]
fn a_key_drag_shows_the_drop_target_on_key_faces_only(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = three_key_window(cx);
    drag_over(cx, "surface-key face 2", "surface-key face 4");
    // The drag is active: each key face has its target, and the bar panel
    // has none, because a key goes to a key face only.
    assert!(cx.debug_bounds("drop-key face 4").is_some());
    assert!(cx.debug_bounds("drop-key face 8").is_some());
    assert!(cx.debug_bounds("drop-bar panel 3").is_none());
    // A key does not go to itself: the key in the drag shows no target (L1
    // of review 2).
    assert!(
        cx.debug_bounds("drop-key face 2").is_none(),
        "the source key shows the target"
    );
}

#[gpui_kit::test]
fn a_key_back_on_itself_shows_no_target_and_sends_nothing(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = three_key_window(cx);
    drag_over(cx, "surface-key face 2", "surface-key face 2");
    assert!(
        cx.debug_bounds("drop-key face 2").is_none(),
        "the source key shows the target"
    );
    release(cx, "surface-key face 2");
    assert!(moves(&sent).is_empty(), "{:?}", sent.borrow());
}

#[gpui_kit::test]
fn a_drag_keeps_the_plus_icon_of_an_empty_key_that_is_not_the_target(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = three_key_window(cx);
    // An unrelated drag hides no plus icon on the other key faces (review 1,
    // M5).
    drag_over(cx, "surface-key face 2", "surface-key face 4");
    assert!(cx.debug_bounds("empty-key face 8").is_some());
}

#[gpui_kit::test]
fn the_drop_is_one_step_and_undo_sends_the_command(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = three_key_window(cx);
    drag_to(cx, "surface-key face 2", "surface-key face 4");
    send(
        cx,
        &tx,
        Event::History {
            undo: Some("Move Key".into()),
            redo: None,
        },
    );
    assert!(cx.update(|window, cx| window.is_action_available(&crate::app_menu::Undo, cx)));
    cx.simulate_keystrokes("cmd-z");
    cx.run_until_parked();
    assert_eq!(sent.borrow().last(), Some(&Command::Undo));
}
