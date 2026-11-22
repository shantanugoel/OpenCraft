//! The keyboard focus of the controls (issue 242, `docs/DESIGN.md`,
//! section 5): Tab and Shift-Tab move the focus in the order of the window,
//! Space and Return operate the control with the focus, a click does not
//! give a control the focus, and the key commands of the window keep their
//! work while a control has the focus.

use super::pages::{on, pages, profiles, send};
use super::*;
use crate::{
    settings::RightPanelTab,
    view::{
        controls::tests::{focused, ringed},
        sidebar::ProfileSettingsRow,
    },
};
use gpui_kit::{Bounds, KeyUpEvent, MouseButton, MouseDownEvent, Pixels};
use opencraft_engine::{
    Auto, DeviceLayout, DeviceSettings, KnobLightMode, KnobLights, LabelChange, LabelPosition,
    LabelStyle, SurfaceLayout,
};

/// A window with two profiles and two pages in the active profile.
fn focus_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    send(
        cx,
        &tx,
        profiles(&[("default", "Default"), ("work", "Work")], "default"),
    );
    send(cx, &tx, pages(&["Main", "Meetings"], 0));
    (view, cx, tx, sent)
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

/// Push the key `key`: the key goes down and up. GPUI operates a control
/// with the focus at the key up of Space and Return.
fn push(cx: &mut VisualTestContext, key: &'static str) {
    keys(cx, key);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).unwrap(),
    });
    cx.run_until_parked();
}

/// The IDs of the controls that `count` presses of `key` give the focus to.
/// "-" is a stop that is not a control: a text field.
fn order(cx: &mut VisualTestContext, key: &str, count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            keys(cx, key);
            focused(cx).unwrap_or_else(|| "-".into())
        })
        .collect()
}

/// Press Tab until the control `id` has the focus.
pub(super) fn tab_to(cx: &mut VisualTestContext, id: &str) {
    let mut seen = Vec::new();
    for _ in 0..200 {
        keys(cx, "tab");
        let focused_id = focused(cx);
        if focused_id.as_deref() == Some(id) {
            return;
        }
        seen.push(focused_id);
    }
    panic!("Tab does not get to {id}: {seen:?}");
}

#[gpui_kit::test]
fn knob_functions_are_in_the_keyboard_focus_order(cx: &mut TestAppContext) {
    let (_, cx, _, _) = super::knob::functions_window(cx, &["Volume"]);
    tab_to(cx, "functions.0/function");
    push(cx, "space");
    tab_to(cx, "functions.0/action");
    // A segmented control is one Tab stop: the arrow keys change the
    // direction.
    assert_eq!(order(cx, "tab", 1), ["turn-direction"]);
    tab_to(cx, "functions.0/turn-apart");
    push(cx, "space");
    tab_to(cx, "functions.0/turn-join");
    keys(cx, "tab");
    assert_eq!(focused(cx).as_deref(), Some("functions/add"));
}

fn workspace_has_focus(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| view.read(cx).focus.is_focused(window))
}

/// The start of the order: the sidebar, the toolbar, the page header, and
/// the tabs of the right panel.
const START: [&str; 16] = [
    "hide-sidebar",
    "device-card",
    "profile-add",
    "profile-default",
    "disclosure-default",
    "page-0",
    "page-1",
    "page-add",
    "folder-add",
    "profile-work",
    "sidebar-settings",
    "toolbar-search",
    "toggle-inspector",
    "next-page",
    "page-settings",
    "right-tabs",
];

#[gpui_kit::test]
fn tab_and_shift_tab_move_the_focus_in_the_order_of_the_window(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = focus_window(cx);
    assert_eq!(order(cx, "tab", START.len()), START);
    let mut back: Vec<&str> = START[..START.len() - 1].to_vec();
    back.reverse();
    assert_eq!(order(cx, "shift-tab", back.len()), back);
    // Shift-Tab at the first control goes to the last one, and Tab goes
    // back to the first one.
    let last = order(cx, "shift-tab", 1);
    assert_ne!(last, ["hide-sidebar"]);
    assert_eq!(order(cx, "tab", 1), ["hide-sidebar"]);
}

#[gpui_kit::test]
fn tab_reaches_each_control_of_the_inspector_of_a_key(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = focus_window(cx);
    click(cx, "surface-key face 2");
    let mut inspector = vec![
        "more-button",
        "choose-icon",
        "choose-image",
        "-",
        "label-position",
        "label-size",
        "label-colour",
        "label-bold",
        "label-italic",
        "label-underline",
        "label-font",
        "no-colour",
    ];
    let colours: Vec<String> = (0..COLOURS.len()).map(|n| format!("colour-{n}")).collect();
    inspector.extend(colours.iter().map(String::as_str));
    inspector.extend([
        "other-colour",
        "gesture-Press-kind",
        "gesture-Release-kind",
        "gesture-LongPress-kind",
        "gesture-DoublePress-kind",
    ]);
    let ids = order(cx, "tab", START.len() + inspector.len());
    assert_eq!(ids[..START.len()], START);
    assert_eq!(ids[START.len()..], inspector[..]);
}

#[gpui_kit::test]
fn tab_reaches_the_switch_the_segments_and_the_sliders_of_the_device(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = focus_window(cx);
    send(
        cx,
        &tx,
        Event::DeviceSettings(DeviceSettings {
            brightness: 80,
            screen_off_minutes: 0,
            haptic: true,
            knob_lights: KnobLights {
                mode: KnobLightMode::Effect,
                brightness: 100,
                colors: vec![Color::WHITE; 2],
            },
        }),
    );
    click(cx, "device-card");
    let ids = order(cx, "tab", START.len() + 6);
    assert_eq!(ids[..START.len()], START);
    let device = &ids[START.len()..];
    assert!(device.contains(&"haptic-switch".to_string()), "{device:?}");
    assert!(
        device.iter().any(|id| id.starts_with("slider-keys")),
        "{device:?}"
    );
    // The arrow keys move the slider with the focus.
    tab_to(cx, "haptic-switch");
    push(cx, "space");
    let haptic = |sent: &Commands| {
        sent.borrow().iter().rev().find_map(|c| match c {
            Command::SetDeviceSettings(s) => Some(s.haptic),
            _ => None,
        })
    };
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    assert_eq!(haptic(&sent), Some(false), "Space operates the switch");
    // The arrow keys move the slider with the focus: after the switch, it
    // is the brightness of the knob lights.
    while !focused(cx).is_some_and(|id| id.starts_with("slider-keys")) {
        keys(cx, "tab");
    }
    keys(cx, "left");
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    let brightness = sent.borrow().iter().rev().find_map(|c| match c {
        Command::SetDeviceSettings(s) => Some(s.knob_lights.brightness),
        _ => None,
    });
    assert_eq!(brightness, Some(99));
}

/// A surface with the label position `position`, and an image.
fn positioned(position: LabelPosition) -> Event {
    Event::Surface(SurfaceState {
        id: key(),
        content: SurfaceContent {
            label: Some("Mail".into()),
            label_style: LabelStyle {
                position: Some(Auto::Value(position)),
                ..Default::default()
            },
            ..Default::default()
        },
        image: Arc::new(RgbImage::new(90, 90)),
        image_size: None,
        image_area: None,
        on_image_size: None,
        on_image_area: None,
        states: None,
    })
}

/// A segmented control is one Tab stop, and the arrow keys change the
/// choice, as a radio group does on macOS (`docs/DESIGN.md`, section 5).
#[gpui_kit::test]
fn a_segmented_control_is_one_tab_stop_and_the_arrow_keys_change_the_choice(
    cx: &mut TestAppContext,
) {
    let (_view, cx, tx, sent) = focus_window(cx);
    click(cx, "surface-key face 2");
    tab_to(cx, "label-position");
    let position = |sent: &Commands| {
        sent.borrow()
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::SetLabelStyle {
                    change: LabelChange::Position(Some(Auto::Value(position))),
                    ..
                } => Some(*position),
                _ => None,
            })
    };
    // The surface has an image, thus "Middle" is the selected segment. The
    // right arrow chooses the next segment, and the focus stays on the
    // control.
    keys(cx, "right");
    assert_eq!(position(&sent), Some(LabelPosition::Bottom));
    assert_eq!(focused(cx).as_deref(), Some("label-position"));
    // The engine sends the position, and the left arrow chooses the segment
    // before that one.
    send(cx, &tx, positioned(LabelPosition::Hidden));
    keys(cx, "left");
    assert_eq!(position(&sent), Some(LabelPosition::Bottom));
    // Tab goes to the next control, not to the next segment.
    keys(cx, "tab");
    assert_eq!(focused(cx).as_deref(), Some("label-size"));
}

/// A segmented control shows the focus ring only while the control itself
/// has the keyboard focus (`docs/DESIGN.md`, section 5): the window, the
/// card of a sheet, and another control paint no ring on it (review 3, H1).
/// The layout of the window tests, with a background surface of 400 × 300.
fn layout_with_background() -> Event {
    let Event::Layout(layout) = crate::state::tests::layout() else {
        unreachable!()
    };
    let mut layout = DeviceLayout::clone(&layout);
    layout.surfaces.push(SurfaceLayout {
        id: SurfaceId::new(SurfaceKind::Background, 1),
        x: 0,
        y: 0,
        width: 400,
        height: 300,
    });
    Event::Layout(Arc::new(layout))
}

/// Each row of a section scrolls the panel to its own control, also when
/// two rows of one section are below the visible part: the mode and the
/// samples of the "Background" section (review 3, M1).
#[gpui_kit::test]
fn the_panel_scrolls_to_the_row_of_each_control_of_a_section(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = focus_window(cx);
    cx.update(|window, _| window.activate_window());
    send(cx, &tx, layout_with_background());
    // "Page settings" in the page header selects the page: its inspector
    // has the section "Background".
    click(cx, "page-settings");
    click(cx, "background-colour");
    cx.simulate_resize(size(px(1440.), px(400.)));
    cx.run_until_parked();
    // The end of the section starts below the visible part of the
    // inspector: the row of the mode, and the row of the samples.
    let (_, visible) = inspector_scroll(&view, cx);
    for id in ["background-kind", "colour-0"] {
        assert!(
            bounds(cx, id).bottom() > visible.bottom(),
            "the test needs {id} below the visible part"
        );
    }
    // Tab reaches the mode: the panel shows its row.
    tab_to(cx, "background-kind");
    draw(cx);
    assert!(shows(cx, "background-kind", &view), "the mode shows");
    // Tab reaches the first sample of the body of the section: the panel
    // follows the sample, not the row of the mode.
    tab_to(cx, "colour-0");
    draw(cx);
    assert!(
        shows(cx, "colour-0", &view),
        "the focused sample shows in the visible part"
    );
}

#[gpui_kit::test]
fn the_focus_ring_of_a_segmented_control_shows_only_for_its_own_focus(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = focus_window(cx);
    // A click gives the focus to the window, not to a control. No segmented
    // control shows the ring, also not the selected choice of the tabs.
    click(cx, "surface-key face 2");
    assert!(workspace_has_focus(&view, cx), "the window has the focus");
    assert_eq!(ringed(cx), Vec::<String>::new(), "no ring for a click");

    // Tab reaches the tabs of the right panel: the ring shows on that
    // control, and on no other segmented control.
    tab_to(cx, "right-tabs");
    assert_eq!(focused(cx).as_deref(), Some("right-tabs"));
    assert_eq!(ringed(cx), ["right-tabs"]);

    // Shift-Tab leaves the control: no ring stays.
    keys(cx, "shift-tab");
    assert_eq!(
        ringed(cx),
        Vec::<String>::new(),
        "no ring without the focus"
    );

    // The inspector of the key has a segmented control too.
    tab_to(cx, "label-position");
    assert_eq!(ringed(cx), ["label-position"]);
    // The arrow keys change the choice and the control keeps the ring.
    keys(cx, "right");
    keys(cx, "left");
    assert_eq!(ringed(cx), ["label-position"]);

    // A sheet has the focus, not its segmented control.
    cx.simulate_keystrokes("ctrl-cmd-p");
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("profile-sheet").is_some(),
        "the sheet opens"
    );
    assert_eq!(ringed(cx), Vec::<String>::new(), "no ring for the sheet");
    tab_to(cx, "profile-title-position");
    assert_eq!(ringed(cx), ["profile-title-position"]);
}

#[gpui_kit::test]
fn space_and_return_operate_the_control_with_the_focus(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = focus_window(cx);
    tab_to(cx, "page-1");
    push(cx, "space");
    assert!(sent
        .borrow()
        .iter()
        .any(|c| *c == on("default", Command::SelectPage { page: 1 })));
    // The tabs of the right panel are one control: the arrow keys change the
    // tab, and the control keeps the focus.
    tab_to(cx, "right-tabs");
    keys(cx, "right");
    cx.update(|_, cx| assert_eq!(view.read(cx).panes.right_panel_tab, RightPanelTab::Library));
    assert_eq!(focused(cx).as_deref(), Some("right-tabs"));
    keys(cx, "left");
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).panes.right_panel_tab,
            RightPanelTab::Inspector
        )
    });
    tab_to(cx, "hide-sidebar");
    push(cx, "space");
    cx.update(|_, cx| assert!(!view.read(cx).panes.sidebar_visible));
}

#[gpui_kit::test]
fn a_click_does_not_give_the_focus_to_a_control(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = focus_window(cx);
    click(cx, "tab-library");
    assert_eq!(focused(cx), None);
    assert!(workspace_has_focus(&view, cx));
    click(cx, "tab-inspector");
    // A click on a control takes the focus from a text field, as before.
    click(cx, "surface-key face 2");
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        input.update(cx, |input, cx| input.focus(window, cx));
    });
    click(cx, "label-bold");
    assert!(workspace_has_focus(&view, cx));
}

#[gpui_kit::test]
fn the_key_commands_operate_while_a_control_has_the_focus(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = focus_window(cx);
    click(cx, "surface-key face 2");
    tab_to(cx, "label-bold");
    // Delete clears the selected control, as Edit > Clear does: a button
    // has no use for the key (decision 0029, "Focus rule").
    keys(cx, "backspace");
    assert!(sent
        .borrow()
        .iter()
        .any(|c| matches!(c, Command::ClearControl { .. })));
    assert_eq!(focused(cx).as_deref(), Some("label-bold"));
    keys(cx, "ctrl-cmd-s");
    cx.update(|_, cx| assert!(!view.read(cx).panes.sidebar_visible));
    keys(cx, "cmd-k");
    cx.update(|_, cx| assert!(view.read(cx).palette.open));
}

#[gpui_kit::test]
fn a_menu_opens_with_space_and_its_button_keeps_the_focus(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = focus_window(cx);
    click(cx, "surface-key face 2");
    tab_to(cx, "label-size");
    push(cx, "space");
    assert!(cx.debug_bounds("label-size-menu").is_some());
    // Tab does nothing while the menu is open.
    keys(cx, "tab");
    assert_eq!(focused(cx).as_deref(), Some("label-size"));
    keys(cx, "down");
    push(cx, "space");
    assert!(cx.debug_bounds("label-size-menu").is_none());
    assert!(sent.borrow().iter().any(|c| matches!(
        c,
        Command::SetLabelStyle {
            change: LabelChange::Size(_),
            ..
        }
    )));
    assert_eq!(focused(cx).as_deref(), Some("label-size"));
    // Escape closes the menu that Return opened.
    push(cx, "enter");
    assert!(cx.debug_bounds("label-size-menu").is_some());
    keys(cx, "escape");
    assert!(cx.debug_bounds("label-size-menu").is_none());
    let _ = view;
}

#[gpui_kit::test]
fn the_focus_comes_back_to_the_window_when_its_control_goes_away(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = focus_window(cx);
    send(cx, &tx, Event::Problem("Cannot save the profile.".into()));
    tab_to(cx, "dismiss-0");
    push(cx, "space");
    cx.update(|_, cx| assert!(view.read(cx).state.problems.is_empty()));
    assert!(workspace_has_focus(&view, cx));
    keys(cx, "ctrl-cmd-s");
    cx.update(|_, cx| assert!(!view.read(cx).panes.sidebar_visible));
}

#[gpui_kit::test]
fn tab_stays_in_a_sheet_and_return_does_the_main_action(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = focus_window(cx);
    let position = cx.debug_bounds("profile-default").unwrap().center();
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Right,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.run_until_parked();
    cx.dispatch_action(ProfileSettingsRow);
    cx.run_until_parked();
    let sheet = [
        "-",
        "profile-colour-none",
        "profile-colour-0",
        "profile-colour-1",
        "profile-colour-2",
        "profile-colour-3",
        "profile-colour-4",
        "profile-colour-5",
        "first-page",
        "-",
        "profile-title-size",
        "profile-title-bold",
        "profile-title-italic",
        "profile-title-underline",
        "profile-title-position",
        "profile-title-colour-auto",
        "profile-title-colour-0",
        "profile-title-colour-1",
        "profile-title-colour-2",
        "profile-title-colour-3",
        "profile-title-colour-4",
        "profile-title-colour-5",
        "profile-title-colour-6",
        "profile-title-colour-7",
        "profile-title-colour-8",
        "profile-title-colour-9",
        "profile-title-colour-10",
        "profile-title-colour-11",
        "profile-title-colour-12",
        "profile-title-colour-13",
        "profile-add-application",
        "profile-export",
        "profile-remove",
        "profile-done",
    ];
    assert_eq!(
        order(cx, "tab", sheet.len() + 1),
        [&sheet[..], &sheet[..1]].concat()
    );
    // The focus is on the name field: Shift-Tab goes to "Done" first.
    let mut back = sheet.to_vec();
    back.reverse();
    assert_eq!(order(cx, "shift-tab", sheet.len()), back);
    // Space operates the control with the focus; Return does "Done".
    tab_to(cx, "profile-colour-2");
    push(cx, "space");
    tab_to(cx, "profile-export");
    push(cx, "enter");
    cx.update(|_, cx| assert!(view.read(cx).sheet.profile.is_none()));
    assert!(workspace_has_focus(&view, cx));
}

/// The scroll of the inspector, and its visible part, in the window.
fn inspector_scroll(
    view: &Entity<DeviceView>,
    cx: &mut VisualTestContext,
) -> (Pixels, Bounds<Pixels>) {
    cx.update(|_, cx| {
        let panel = view.read(cx);
        (
            panel.inspector_scroll.offset().y,
            panel.inspector_scroll.bounds(),
        )
    })
}

/// Draw the frame that follows an input.
fn draw(cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(gpui_kit::point(px(4.), px(4.)), None, Modifiers::default());
    cx.run_until_parked();
}

/// The bounds of the element `id` in the window.
fn bounds(cx: &mut VisualTestContext, id: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"))
}

/// True when `id` shows in the visible part of the inspector.
fn shows(cx: &mut VisualTestContext, id: &'static str, view: &Entity<DeviceView>) -> bool {
    let (_, visible) = inspector_scroll(view, cx);
    let bounds = bounds(cx, id);
    bounds.top() >= visible.top() && bounds.bottom() <= visible.bottom()
}

/// A window whose selected key has an open card with many rows, as the
/// key of the audit had (issue 484): at the minimum size the last gesture
/// cards start below the visible part of the inspector.
fn long_inspector(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
) {
    let (view, cx, tx, _sent) = focus_window(cx);
    let kind = super::actions::kind();
    let settings: Settings = kind
        .fields
        .iter()
        .filter_map(|field| Some((field.key.clone(), field.default.clone()?)))
        .collect();
    send(cx, &tx, Event::Kinds(vec![kind.clone()]));
    send(
        cx,
        &tx,
        Event::Binding(BindingState {
            binding: BindingId {
                page: 0,
                control: ControlId::new(ControlKind::Key, 2),
                gesture: GestureKind::Press,
                folder: None,
            },
            action: Some(Action {
                kind: kind.kind.clone(),
                settings,
            }),
            available: true,
            inert_by: None,
        }),
    );
    click(cx, "surface-key face 2");
    // The head of the card opens it: its rows make the inspector long.
    click(cx, "gesture-Press-head");
    click(cx, "surface-key face 2");
    cx.simulate_resize(size(px(theme::WINDOW_MIN.0), px(theme::WINDOW_MIN.1)));
    cx.run_until_parked();
    (view, cx, tx)
}

#[gpui_kit::test]
fn the_inspector_scrolls_to_the_control_that_the_focus_reaches(cx: &mut TestAppContext) {
    let (view, cx, _tx) = long_inspector(cx);
    assert_eq!(
        inspector_scroll(&view, cx).0,
        px(0.),
        "the inspector starts at the top"
    );
    // The last gesture of the key starts below the visible part.
    assert!(
        bounds(cx, "gesture-DoublePress-kind").bottom() > inspector_scroll(&view, cx).1.bottom(),
        "the test needs a control below the visible part"
    );
    tab_to(cx, "gesture-DoublePress-kind");
    assert!(
        inspector_scroll(&view, cx).0 < px(0.),
        "the inspector scrolled down in the frame of the key"
    );
    // GPUI records the bounds of an element in its prepaint, thus a new
    // frame shows the place of the row at the new position.
    draw(cx);
    assert!(
        shows(cx, "gesture-DoublePress-kind", &view),
        "the card shows in the visible part after Tab"
    );
    // Shift-Tab moves the focus up, and the panel follows it: the row of
    // the card above shows too.
    keys(cx, "shift-tab");
    cx.run_until_parked();
    assert!(shows(cx, "gesture-LongPress-kind", &view));
}

#[gpui_kit::test]
fn the_inspector_keeps_the_scroll_while_a_control_keeps_the_focus(cx: &mut TestAppContext) {
    let (view, cx, _tx) = long_inspector(cx);
    tab_to(cx, "gesture-DoublePress-kind");
    cx.run_until_parked();
    let scrolled = inspector_scroll(&view, cx).0;
    assert!(scrolled < px(0.));
    // The panel asks for its new position only when the focus moves to
    // another control, thus a wheel or a scrollbar keeps the position.
    cx.run_until_parked();
    assert_eq!(inspector_scroll(&view, cx).0, scrolled);
}
