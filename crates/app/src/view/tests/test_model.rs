//! The window shows a second model from the layout message only.
//!
//! The layout has the shape of the test model of the device layer tests
//! (`crates/core/tests/support/test_model.rs`): 6 key faces of 96 × 96 in 2
//! rows of 3, 1 knob, 1 knob light, no haptic feedback, no bar panel, and no
//! touch bar. The shell does not use the device layer (architecture rule 2),
//! so the test writes the layout as the engine sends it. The engine test
//! `test_model::the_engine_shows_the_layout_and_the_surfaces_of_the_test_model`
//! checks that the engine makes this layout from the descriptor. See
//! `docs/ADDING-A-DEVICE-MODEL.md`.

use super::*;
use crate::view::theme;
use gpui_kit::{BorderStyle, Hsla};
use opencraft_engine::{
    ControlLayout, DeviceLayout, DeviceSettings, InputEvent, KnobLightMode, KnobLights,
    SettingsSupport, SurfaceLayout,
};

fn face(number: u8) -> SurfaceId {
    SurfaceId::new(SurfaceKind::KeyFace, number)
}

/// The layout of the test model, as the engine makes it.
fn test_model_layout() -> Event {
    let index = |number: u8| u16::from(number - 1);
    let surfaces = (1..=6)
        .map(|n| SurfaceLayout {
            id: face(n),
            x: 4 + (index(n) % 3) * 100,
            y: 4 + (index(n) / 3) * 100,
            width: 96,
            height: 96,
        })
        .collect();
    let mut controls: Vec<ControlLayout> = (1..=6)
        .map(|n| ControlLayout {
            id: ControlId::new(ControlKind::Key, n),
            row: (n - 1) / 3 + 1,
            column: (n - 1) % 3 + 1,
            surface: Some(face(n)),
            feedback_surface: Some(face(n)),
        })
        .collect();
    controls.push(ControlLayout {
        id: ControlId::new(ControlKind::Knob, 1),
        row: 1,
        column: 1,
        surface: None,
        feedback_surface: None,
    });
    Event::Layout(Arc::new(DeviceLayout {
        name: "OpenCraft test model (not a real device)".into(),
        display_width: 300,
        display_height: 200,
        surfaces,
        controls,
        settings: SettingsSupport {
            lowest_brightness: 0,
            knob_lights: 1,
            haptic: false,
        },
    }))
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

fn kind() -> KindDescription {
    KindDescription {
        kind: "test.test-model".into(),
        name: "Test action".into(),
        fields: Vec::new(),
        ..Default::default()
    }
}

/// The binding of the last binding command.
fn last_binding(sent: &Commands) -> Option<BindingId> {
    sent.borrow().iter().rev().find_map(|c| match c {
        Command::SetBinding { binding, .. } => Some(*binding),
        _ => None,
    })
}

#[gpui_kit::test]
fn the_window_shows_each_control_of_the_test_model_and_no_other(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, test_model_layout());
    for n in 1..=6 {
        send(
            cx,
            &tx,
            Event::Surface(SurfaceState {
                id: face(n),
                content: SurfaceContent::default(),
                image: Arc::new(RgbImage::new(96, 96)),
                states: None,
            }),
        );
    }
    send(cx, &tx, Event::Kinds(vec![kind()]));

    // The surfaces of the layout before are gone: bar panel 3 and its touch
    // bar are not in this layout.
    cx.update(|_, cx| assert_eq!(view.read(cx).images.len(), 6));
    for id in [
        "surface-key face 1",
        "surface-key face 2",
        "surface-key face 3",
        "surface-key face 4",
        "surface-key face 5",
        "surface-key face 6",
    ] {
        assert!(cx.debug_bounds(id).is_some(), "No element: {id}");
    }
    assert!(cx.debug_bounds("control-knob 1").is_some());
    for id in [
        "surface-key face 7",
        "surface-bar panel 3",
        "control-touch bar 2",
        "control-knob 2",
    ] {
        assert!(cx.debug_bounds(id).is_none(), "{id} shows");
    }

    // The key faces have the size of the layout, at one scale.
    let first = cx.debug_bounds("surface-key face 1").unwrap();
    let sixth = cx.debug_bounds("surface-key face 6").unwrap();
    assert_eq!(first.size, sixth.size);
    assert!(sixth.origin.x > first.origin.x && sixth.origin.y > first.origin.y);

    // A binding for key 6 and for knob 1 of the test model.
    click(cx, "surface-key face 6");
    click(cx, "gesture-Press-kind");
    click(cx, "kind-0");
    assert_eq!(
        last_binding(&sent),
        Some(BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Key, 6),
            gesture: GestureKind::Press,
            folder: None,
        })
    );
    click(cx, "control-knob 1");
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, None));
    // The "Turn" card writes the two turn bindings in one group.
    click(cx, "gesture-Turn-kind");
    click(cx, "kind-0");
    let Some(Command::Group { commands, .. }) = sent.borrow().last().cloned() else {
        panic!("expected a group")
    };
    let gestures: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            Command::SetBinding { binding, .. } => {
                assert_eq!(binding.control, ControlId::new(ControlKind::Knob, 1));
                Some(binding.gesture)
            }
            _ => None,
        })
        .collect();
    assert_eq!(gestures, [GestureKind::TurnRight, GestureKind::TurnLeft]);
}

#[gpui_kit::test]
fn the_device_settings_of_the_test_model_have_one_knob_light_and_no_haptic_feedback(
    cx: &mut TestAppContext,
) {
    let (_view, cx, tx, _sent) = window(cx);
    send(cx, &tx, test_model_layout());
    send(
        cx,
        &tx,
        Event::DeviceSettings(DeviceSettings {
            brightness: 80,
            screen_off_minutes: 0,
            haptic: true,
            knob_lights: KnobLights {
                mode: KnobLightMode::Colors,
                brightness: 100,
                colors: vec![Color::WHITE],
            },
        }),
    );
    click(cx, "device-card");
    assert!(cx.debug_bounds("brightness-slider").is_some());
    assert!(cx.debug_bounds("knob-1-colour").is_some());
    for id in ["knob-2-colour", "haptic-switch", "feedback"] {
        assert!(cx.debug_bounds(id).is_none(), "{id} shows");
    }
}

/// A layout of the test model, plus controls that have no surface: one bar
/// panel and three keys. The shell does not use the device layer (rule 2).
fn layout_with_controls_that_have_no_surface() -> Event {
    let Event::Layout(layout) = test_model_layout() else {
        panic!("layout")
    };
    let mut layout = (*layout).clone();
    let bare = |kind, number, row, column| ControlLayout {
        id: ControlId::new(kind, number),
        row,
        column,
        surface: None,
        feedback_surface: None,
    };
    layout.controls.extend([
        bare(ControlKind::BarPanel, 1, 1, 1),
        bare(ControlKind::Key, 8, 3, 2),
        bare(ControlKind::Key, 7, 3, 1),
        bare(ControlKind::Key, 9, 4, 1),
    ]);
    Event::Layout(Arc::new(layout))
}

#[gpui_kit::test]
fn the_user_can_select_a_key_that_has_no_surface_and_give_it_a_binding(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, layout_with_controls_that_have_no_surface());
    send(cx, &tx, Event::Kinds(vec![kind()]));

    let bounds = |cx: &mut VisualTestContext, id: &str| {
        cx.debug_bounds(Box::leak(id.to_string().into_boxed_str()))
            .unwrap_or_else(|| panic!("No element: {id}"))
    };
    let display = bounds(cx, "display");
    let panel = bounds(cx, "control-bar panel 1");
    let key7 = bounds(cx, "control-key 7");
    let key8 = bounds(cx, "control-key 8");
    let key9 = bounds(cx, "control-key 9");
    let knob = bounds(cx, "label-knob 1");
    // Below the display and below the knob. Row order, then column order.
    assert!(panel.top() > display.bottom());
    assert!(panel.top() > knob.bottom());
    assert!(key7.top() > panel.bottom());
    assert!(key9.top() > key7.bottom());
    assert!((f32::from(key7.top()) - f32::from(key8.top())).abs() <= 0.5);
    assert!(key7.center().x < display.center().x);
    assert!(key8.center().x > display.center().x);
    assert!((f32::from(key9.center().x) - f32::from(display.center().x)).abs() <= 0.5);
    assert!((f32::from(panel.center().x) - f32::from(display.center().x)).abs() <= 0.5);
    // A key that has a surface stays on the display. The knob stays a knob.
    assert!(bounds(cx, "surface-key face 1").bottom() <= display.bottom());
    assert!(cx.debug_bounds("control-key 1").is_none());

    click(cx, "control-key 7");
    cx.update(|_, cx| {
        let state = &view.read(cx).state;
        assert_eq!(
            state.selected_control,
            Some(ControlId::new(ControlKind::Key, 7))
        );
        assert_eq!(state.selected, None);
    });
    assert!(cx.debug_bounds("selection-key 7").is_some());
    assert!(cx.debug_bounds("gesture-Press-kind").is_some());
    assert!(cx.debug_bounds("appearance").is_none());
    click(cx, "gesture-Press-kind");
    click(cx, "kind-0");
    assert_eq!(
        last_binding(&sent),
        Some(BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Key, 7),
            gesture: GestureKind::Press,
            folder: None,
        })
    );
}

#[derive(Debug)]
struct PlainBorder {
    width: f32,
    colour: Hsla,
    style: BorderStyle,
}

fn same_colour(left: Hsla, right: Hsla) -> bool {
    (left.h - right.h).abs() < 1e-3
        && (left.s - right.s).abs() < 1e-3
        && (left.l - right.l).abs() < 1e-3
        && (left.a - right.a).abs() < 1e-3
}

/// The borders painted on the element `id`. GPUI can paint one border as
/// more than one quad. Each quad has the size of the element.
fn plain_borders(cx: &mut VisualTestContext, id: &'static str) -> Vec<PlainBorder> {
    let bounds = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"));
    cx.update(|window, app| {
        window.draw(app).clear(app);
        let scale = window.scale_factor();
        let left = f32::from(bounds.origin.x);
        let top = f32::from(bounds.origin.y);
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        let mut found = Vec::new();
        for quad in window.painted_quads() {
            let quad_left = quad.bounds.origin.x.0 / scale;
            let quad_top = quad.bounds.origin.y.0 / scale;
            let quad_width = quad.bounds.size.width.0 / scale;
            let quad_height = quad.bounds.size.height.0 / scale;
            let border = quad.border_widths.top.0 / scale;
            let same = (quad_left - left).abs() < 1.5
                && (quad_top - top).abs() < 1.5
                && (quad_width - width).abs() < 1.5
                && (quad_height - height).abs() < 1.5;
            if same && border > 0.4 {
                found.push((border, quad.border_color, quad.border_style));
            }
        }
        assert!(!found.is_empty(), "no border quad for {id}");
        found
            .into_iter()
            .map(|(width, colour, style)| PlainBorder {
                width,
                colour,
                style,
            })
            .collect()
    })
}

#[gpui_kit::test]
fn a_press_of_a_key_with_no_surface_glows_until_the_release(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = window(cx);
    send(cx, &tx, layout_with_controls_that_have_no_surface());
    let id = ControlId::new(ControlKind::Key, 7);

    expect_border(
        &plain_borders(cx, "control-key 7"),
        1.,
        BorderStyle::Dashed,
        theme::DEVICE_KEY_EMPTY_BORDER.into(),
    );

    send(
        cx,
        &tx,
        Event::Input(InputEvent {
            control: id,
            gesture: Gesture::Press,
        }),
    );
    cx.update(|_, cx| assert!(view.read(cx).state.pressed.contains(&id)));
    expect_border(
        &plain_borders(cx, "control-key 7"),
        2.,
        BorderStyle::Solid,
        theme::INPUT_GLOW.into(),
    );

    send(
        cx,
        &tx,
        Event::Input(InputEvent {
            control: id,
            gesture: Gesture::Release,
        }),
    );
    cx.update(|_, cx| assert!(!view.read(cx).state.pressed.contains(&id)));
    expect_border(
        &plain_borders(cx, "control-key 7"),
        1.,
        BorderStyle::Dashed,
        theme::DEVICE_KEY_EMPTY_BORDER.into(),
    );
}

fn expect_border(borders: &[PlainBorder], width: f32, style: BorderStyle, colour: Hsla) {
    assert!(
        borders.iter().all(|border| {
            (border.width - width).abs() < 0.6
                && border.style == style
                && same_colour(border.colour, colour)
        }),
        "{borders:?}"
    );
}
