//! Dispatch real window clicks and keys with GPUI's no-display test platform.

use super::*;
use crate::state::tests::{bar, key, layout};
use gpui_kit::{
    component::{Theme, ThemeMode},
    size, Focusable, Keystroke, Modifiers, TestAppContext, VisualTestContext,
};
use opencraft_engine::{
    Action, BindingId, BindingState, ChoiceOption, Field, GestureKind, InputType, KindDescription,
    PathKind, RgbImage, Settings, Status, SurfaceContent, SurfaceState, Value,
};
use std::cell::RefCell;

type Commands = Rc<RefCell<Vec<Command>>>;

/// Open a window on `state`. No engine event follows. A test that wants the
/// usual first events uses [`window`].
fn open_window(
    cx: &mut TestAppContext,
    state: ViewState,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        crate::app_menu::install(cx, &Panes::default());
    });
    let (tx, rx) = async_channel::unbounded();
    let sent: Commands = Default::default();
    let commands = sent.clone();
    // The shell writes `shell.toml` here. The command function keeps the
    // directory until the window and the shell go away.
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().to_path_buf();
    let send: Rc<dyn Fn(Command)> = Rc::new(move |c| {
        let _ = &dir;
        commands.borrow_mut().push(c)
    });
    let shell_send = send.clone();
    // The events go through the shell entity, as in the application.
    let shell = cx.update(|cx| {
        cx.new(|cx| {
            let mut shell = Shell::new(config, crate::settings::Settings::default(), shell_send);
            shell.listen(rx, async_channel::unbounded().1, cx);
            shell
        })
    });
    let mut child = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| DeviceView::new(&shell, state, Panes::default(), send, window, cx));
        child = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    let (width, height) = theme::WINDOW_SIZE;
    cx.simulate_resize(size(px(width), px(height)));
    (child.unwrap(), cx, tx, sent)
}

fn window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = open_window(cx, ViewState::default());
    tx.try_send(layout()).unwrap();
    tx.try_send(surface(key(), "Mail")).unwrap();
    tx.try_send(surface(bar(), "Sound")).unwrap();
    cx.run_until_parked();
    (view, cx, tx, sent)
}

fn surface(id: SurfaceId, label: &str) -> Event {
    Event::Surface(SurfaceState {
        id,
        content: SurfaceContent {
            label: Some(label.into()),
            ..Default::default()
        },
        image: Arc::new(RgbImage::new(90, 90)),
        states: None,
    })
}

fn click(cx: &mut VisualTestContext, id: &'static str) {
    let bounds = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"));
    cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
    cx.simulate_click(bounds.center(), Modifiers::default());
    cx.run_until_parked();
}

#[gpui_kit::test]
fn events_update_the_view_and_keep_early_problems(cx: &mut TestAppContext) {
    let (view, cx, tx, _) = window(cx);
    tx.try_send(Event::Problem("Cannot save the profile.".into()))
        .unwrap();
    tx.try_send(Event::Connected).unwrap();
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert!(view.read(cx).state.connected);
        assert_eq!(
            view.read(cx).state.surfaces[&key()]
                .content
                .label
                .as_deref(),
            Some("Mail")
        );
        assert_eq!(view.read(cx).state.problems, ["Cannot save the profile."]);
        assert_eq!(view.read(cx).images.len(), 2);
    });
    tx.try_send(surface(key(), "New")).unwrap();
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.surfaces[&key()]
                .content
                .label
                .as_deref(),
            Some("New")
        )
    });
}

#[gpui_kit::test]
fn a_problem_closes_by_mouse_and_stays_closed_in_the_shell(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    tx.try_send(Event::Problem("The profile has no page 1.".into()))
        .unwrap();
    cx.run_until_parked();
    click(cx, "dismiss-0");
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert!(view.state.problems.is_empty());
        assert!(view.shell.read(cx).state.problems.is_empty());
    });
    assert!(cx.debug_bounds("problems").is_none());
    assert!(sent.borrow().is_empty());
}

#[gpui_kit::test]
fn clicks_select_surfaces_and_send_colour_commands(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, Some(key())));
    assert!(
        sent.borrow().is_empty(),
        "Selection must send no label command"
    );
    click(cx, "colour-8");
    assert_eq!(
        sent.borrow().as_slice(),
        &[Command::SetColor {
            surface: key(),
            color: Some(colour(COLOURS[8]))
        }]
    );
    click(cx, "surface-bar panel 3");
    click(cx, "no-colour");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetColor {
            surface: bar(),
            color: None
        })
    );
    cx.update(|_, cx| assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "Sound"));
}

#[gpui_kit::test]
fn other_colour_takes_a_value_in_hex(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    click(cx, "other-colour");
    assert!(cx.debug_bounds("other-colour-popover").is_some());
    // The field has the focus. An incomplete value does nothing.
    cx.simulate_input("#12345");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(sent.borrow().is_empty());
    assert!(cx.debug_bounds("other-colour-popover").is_some());
    cx.simulate_input("6");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        sent.borrow().as_slice(),
        &[Command::SetColor {
            surface: key(),
            color: Some(colour(0x123456))
        }]
    );
    assert!(cx.debug_bounds("other-colour-popover").is_none());
    // Escape closes the popover with no change.
    click(cx, "other-colour");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    click(cx, "no-colour");
    assert!(cx.debug_bounds("other-colour-popover").is_none());
    assert_eq!(sent.borrow().len(), 2);
}

#[gpui_kit::test]
fn label_replies_do_not_overwrite_typing_and_empty_text_removes_label(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        window.focus(&input.focus_handle(cx), cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.dispatch_keystroke(Keystroke::parse("a").unwrap(), cx);
    });
    cx.run_until_parked();
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetLabel {
            surface: key(),
            label: Some("a".into())
        })
    );
    tx.try_send(surface(key(), "Old reply")).unwrap();
    cx.run_until_parked();
    cx.update(|_, cx| assert_eq!(view.read(cx).input.read(cx).value().as_ref(), "a"));
    cx.update(|window, cx| {
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.dispatch_action(Box::new(gpui_kit::component::input::Backspace), cx);
    });
    cx.run_until_parked();
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetLabel {
            surface: key(),
            label: None
        })
    );
}

mod accessibility;
mod actions;
mod backgrounds;
mod canvas;
mod clipboard;
mod device;
mod focus;
mod focus_return;
mod history;
mod hotkey;
mod icons;
mod knob;
mod label_options;
mod library;
mod mac_function;
mod menu;
mod mouse;
mod nested;
mod page_inspector;
mod pages;
mod palette;
mod panes;
mod permission_sheet;
mod profile_sheet;
mod sidebar;
mod test_model;
