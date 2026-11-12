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
use opencraft_engine::{
    ControlLayout, DeviceLayout, DeviceSettings, KnobLightMode, KnobLights, SettingsSupport,
    SurfaceLayout,
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
        })
    );
    click(cx, "control-knob 1");
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, None));
    click(cx, "gesture-TurnRight-kind");
    click(cx, "kind-0");
    assert_eq!(
        last_binding(&sent),
        Some(BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Knob, 1),
            gesture: GestureKind::TurnRight,
        })
    );
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
            haptic: true,
            knob_lights: KnobLights {
                mode: KnobLightMode::Colors,
                brightness: 100,
                colors: vec![Color::WHITE],
            },
        }),
    );
    click(cx, "open-device-settings");
    assert!(cx.debug_bounds("brightness-slider").is_some());
    assert!(cx.debug_bounds("knob-1-colour-0").is_some());
    for id in ["knob-2-colour-0", "haptic-on", "haptic-off"] {
        assert!(cx.debug_bounds(id).is_none(), "{id} shows");
    }
}
