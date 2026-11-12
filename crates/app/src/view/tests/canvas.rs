//! The device drawing on the canvas, the selection, the page header, and
//! the help line (decision 0029, section 8).

use super::*;
use gpui_kit::{Bounds, Pixels};
use opencraft_engine::{
    ControlLayout, DeviceLayout, DeviceSettings, KnobLightMode, KnobLights, SettingsSupport,
    SurfaceLayout,
};

/// The x of each key face, from left to right, and the y of each row: the
/// measured positions of the N4 Pro (`docs/kb/n4-pro-hardware.md`).
const KEY_X: [u16; 5] = [6, 176, 346, 514, 682];
const KEY_Y: [u16; 2] = [8, 178];
const PANEL_X: [u16; 4] = [0, 203, 406, 609];
const PANEL_Y: u16 = 358;

/// The layout of the N4 Pro as the engine sends it. The shell does not use
/// the device layer (rule 2), thus the test writes it.
fn n4_pro() -> DeviceLayout {
    let mut surfaces = Vec::new();
    let mut controls = Vec::new();
    // Top row: slots 11 to 15. Second row: slots 6 to 10.
    for (row, first) in [(0u8, 11u8), (1, 6)] {
        for column in 0..5u8 {
            let number = first + column;
            let id = SurfaceId::new(SurfaceKind::KeyFace, number);
            surfaces.push(SurfaceLayout {
                id,
                x: KEY_X[usize::from(column)],
                y: KEY_Y[usize::from(row)],
                width: 112,
                height: 112,
            });
            controls.push(ControlLayout {
                id: ControlId::new(ControlKind::Key, number),
                row: row + 1,
                column: column + 1,
                surface: Some(id),
                feedback_surface: Some(id),
            });
        }
    }
    for n in 1..=4u8 {
        let id = SurfaceId::new(SurfaceKind::BarPanel, n);
        surfaces.push(SurfaceLayout {
            id,
            x: PANEL_X[usize::from(n - 1)],
            y: PANEL_Y,
            width: 176,
            height: 112,
        });
        controls.push(ControlLayout {
            id: ControlId::new(ControlKind::BarPanel, n),
            row: 1,
            column: n,
            surface: Some(id),
            feedback_surface: Some(id),
        });
        controls.push(ControlLayout {
            id: ControlId::new(ControlKind::Knob, n),
            row: 1,
            column: n,
            surface: None,
            feedback_surface: Some(id),
        });
    }
    controls.push(ControlLayout {
        id: ControlId::new(ControlKind::TouchBar, 1),
        row: 1,
        column: 1,
        surface: None,
        feedback_surface: None,
    });
    DeviceLayout {
        name: "N4 Pro layout".into(),
        display_width: 800,
        display_height: 480,
        surfaces,
        controls,
        settings: SettingsSupport {
            lowest_brightness: 0,
            knob_lights: 4,
            haptic: true,
        },
    }
}

/// The bounds of the element with the debug selector `id`. The test
/// context takes a static name only, thus a made name stays in memory
/// until the test ends.
fn find(cx: &mut VisualTestContext, id: &str) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(Box::leak(id.to_string().into_boxed_str()))
}

fn bounds(cx: &mut VisualTestContext, id: &str) -> Bounds<Pixels> {
    find(cx, id).unwrap_or_else(|| panic!("No element: {id}"))
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

fn pages(names: &[&str], active: usize) -> Event {
    Event::Pages {
        names: names.iter().map(|n| n.to_string()).collect(),
        active,
    }
}

/// The two values are the same to one device pixel. GPUI rounds the
/// layout to device pixels: the test window has 2 for each point.
#[track_caller]
fn near(actual: Pixels, expected: f32, what: &str) {
    let actual = f32::from(actual);
    assert!(
        (actual - expected).abs() <= 0.5 + 1e-3,
        "{what}: {actual} is not {expected}"
    );
}

/// Check the drawing against the layout, and return the scale.
fn check_drawing(cx: &mut VisualTestContext, layout: &DeviceLayout) -> f32 {
    let display = bounds(cx, "display");
    let scale = f32::from(display.size.width) / f32::from(layout.display_width);
    near(
        display.size.height,
        f32::from(layout.display_height) * scale,
        "display height",
    );
    // Each surface is at its position in the layout, at one scale.
    for surface in &layout.surfaces {
        let id = format!("surface-{}", surface.id);
        let b = bounds(cx, &id);
        near(
            b.left() - display.left(),
            f32::from(surface.x) * scale,
            &format!("{id} x"),
        );
        near(
            b.top() - display.top(),
            f32::from(surface.y) * scale,
            &format!("{id} y"),
        );
        near(
            b.size.width,
            f32::from(surface.width) * scale,
            &format!("{id} width"),
        );
        near(
            b.size.height,
            f32::from(surface.height) * scale,
            &format!("{id} height"),
        );
    }
    // The body: a margin of 28 at the left, the right, and the top.
    let body = bounds(cx, "device-body");
    near(display.left() - body.left(), 28. * scale, "body left");
    near(body.right() - display.right(), 28. * scale, "body right");
    near(display.top() - body.top(), 28. * scale, "body top");
    // Each knob: diameter 90, 22 below the display, and centred below its
    // feedback bar panel.
    for n in 1..=4u8 {
        let knob = bounds(cx, &format!("control-knob {n}"));
        let panel = bounds(cx, &format!("surface-bar panel {n}"));
        near(knob.size.width, 90. * scale, "knob diameter");
        near(knob.top() - display.bottom(), 22. * scale, "knob top");
        near(
            knob.center().x - panel.center().x,
            0.,
            &format!("knob {n} centre"),
        );
        let label = bounds(cx, &format!("label-knob {n}"));
        near(label.center().x - knob.center().x, 0., "knob label centre");
        near(label.top() - knob.bottom(), 8. * scale, "knob label top");
        near(body.bottom() - label.bottom(), 44. * scale, "body bottom");
    }
    // The touch bar control: centred on the display, and in the middle of
    // the space between the key faces and the bar panels.
    let touch = bounds(cx, "control-touch bar 1");
    let key = bounds(cx, "surface-key face 6");
    let panel = bounds(cx, "surface-bar panel 1");
    near(touch.center().x - display.center().x, 0., "touch bar x");
    near(
        touch.center().y,
        f32::from((key.bottom() + panel.top()) / 2.),
        "touch bar y",
    );
    near(touch.size.height, 22., "touch bar height");
    // The help line is 18 below the body, and the drawing has 24 of space
    // in the canvas.
    let help = bounds(cx, "help-line");
    near(help.top() - body.bottom(), 18., "help line");
    let area = bounds(cx, "device-area");
    assert!(body.left() - area.left() >= px(24. - 0.5));
    assert!(area.right() - body.right() >= px(24. - 0.5));
    assert!(body.top() - area.top() >= px(24. - 0.5));
    assert!(area.bottom() - help.bottom() >= px(24. - 0.5));
    scale
}

#[gpui_kit::test]
fn the_canvas_draws_each_surface_at_its_position_at_two_window_sizes(cx: &mut TestAppContext) {
    let (_, cx, tx, _) = window(cx);
    let layout = n4_pro();
    send(cx, &tx, Event::Layout(Arc::new(layout.clone())));
    // The default window: the width of the canvas sets the scale.
    let scale = check_drawing(cx, &layout);
    let canvas = bounds(cx, "canvas");
    assert!((scale - (f32::from(canvas.size.width) - 48.) / 856.).abs() < 1e-3);
    assert!((0.9..=0.95).contains(&scale), "{scale}");
    // The minimum window.
    let (width, height) = theme::WINDOW_MIN;
    cx.simulate_resize(size(px(width), px(height)));
    cx.run_until_parked();
    let small = check_drawing(cx, &layout);
    assert!(small < scale && small >= theme::DEVICE_SCALE_MIN, "{small}");
    assert!(bounds(cx, "device-body").right() <= bounds(cx, "canvas").right());
}

#[gpui_kit::test]
fn a_selected_knob_shows_its_bar_panel_with_a_dashed_outline(cx: &mut TestAppContext) {
    let (view, cx, tx, _) = window(cx);
    send(cx, &tx, Event::Layout(Arc::new(n4_pro())));
    // At the start, the selection is the first key of the top row.
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.selected,
            Some(SurfaceId::new(SurfaceKind::KeyFace, 11))
        )
    });
    let key = bounds(cx, "surface-key face 11");
    let outline = bounds(cx, "selection-key face 11");
    // An outline of 2, 3 outside the control.
    near(key.left() - outline.left(), 5., "outline");
    near(outline.right() - key.right(), 5., "outline");
    assert!(cx.debug_bounds("feedback-bar panel 1").is_none());
    let help = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            let view = view.read(cx);
            view.framed_control()
                .filter(|c| c.kind == ControlKind::Knob)
                .map(control_name)
        })
    };
    assert_eq!(help(cx), None);

    click(cx, "control-knob 2");
    assert!(cx.debug_bounds("selection-key face 11").is_none());
    let knob = bounds(cx, "control-knob 2");
    let outline = bounds(cx, "selection-knob 2");
    near(knob.left() - outline.left(), 6., "knob outline");
    let panel = bounds(cx, "surface-bar panel 2");
    let dashed = bounds(cx, "feedback-bar panel 2");
    near(panel.top() - dashed.top(), 5., "dashed outline");
    for n in [1, 3, 4] {
        assert!(find(cx, &format!("feedback-bar panel {n}")).is_none());
    }
    assert_eq!(help(cx).as_deref(), Some("Knob 2"));

    // The label of a knob also selects it.
    click(cx, "label-knob 3");
    assert!(cx.debug_bounds("feedback-bar panel 3").is_some());
    assert!(cx.debug_bounds("feedback-bar panel 2").is_none());

    // A bar panel: its own outline, and no dashed outline.
    click(cx, "surface-bar panel 3");
    assert!(cx.debug_bounds("selection-bar panel 3").is_some());
    assert!(cx.debug_bounds("feedback-bar panel 3").is_none());

    // The touch bar control selects the page, with the touch bar.
    click(cx, "control-touch bar 1");
    cx.update(|_, cx| {
        let state = &view.read(cx).state;
        assert!(state.page_selected);
        assert_eq!(
            state.selected_control,
            Some(ControlId::new(ControlKind::TouchBar, 1))
        )
    });
}

#[gpui_kit::test]
fn a_click_on_an_empty_part_of_the_canvas_selects_the_page(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, Event::Layout(Arc::new(n4_pro())));
    let page = |cx: &mut VisualTestContext| cx.update(|_, cx| view.read(cx).state.page_selected);
    // The space between two key faces on the display.
    let left = bounds(cx, "surface-key face 11");
    let right = bounds(cx, "surface-key face 12");
    let between = gpui_kit::point((left.right() + right.left()) / 2., left.center().y);
    cx.simulate_click(between, Modifiers::default());
    cx.run_until_parked();
    assert!(page(cx));
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.state.selected, None);
        assert_eq!(
            view.state.selected_control,
            Some(ControlId::new(ControlKind::TouchBar, 1))
        );
    });
    // The inspector shows the page.
    assert!(cx.debug_bounds("page-name").is_some());
    assert!(cx.debug_bounds("action-kind").is_none());

    // A control: the page is not the selection.
    click(cx, "surface-key face 12");
    assert!(!page(cx));
    // The canvas outside the device.
    let area = bounds(cx, "device-area");
    cx.simulate_click(
        gpui_kit::point(area.left() + px(4.), area.top() + px(4.)),
        Modifiers::default(),
    );
    cx.run_until_parked();
    assert!(page(cx));
    assert!(sent.borrow().is_empty(), "A selection sends no command");
}

#[gpui_kit::test]
fn the_page_header_shows_the_page_and_changes_it(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, Event::Layout(Arc::new(n4_pro())));
    send(cx, &tx, pages(&["Main", "Media", ""], 0));
    let header = bounds(cx, "page-header");
    let canvas = bounds(cx, "canvas");
    assert_eq!(header.top(), canvas.top());
    assert_eq!(header.size.height, theme::PAGE_HEADER_HEIGHT);
    for id in [
        "page-header-name",
        "page-header-count",
        "previous-page",
        "next-page",
        "page-settings",
    ] {
        let b = bounds(cx, id);
        assert!(
            b.top() >= header.top() && b.bottom() <= header.bottom(),
            "{id}"
        );
    }
    // The name and the count at the left, the buttons at the right.
    assert!(bounds(cx, "page-header-name").right() < bounds(cx, "page-header-count").left());
    assert!(bounds(cx, "page-header-count").right() < bounds(cx, "previous-page").left());
    assert!(bounds(cx, "previous-page").right() < bounds(cx, "next-page").left());
    assert!(bounds(cx, "next-page").right() < bounds(cx, "page-settings").left());
    near(
        header.right() - bounds(cx, "page-settings").right(),
        20.,
        "padding",
    );
    // The header is above the device.
    assert!(header.bottom() <= bounds(cx, "device-area").top());

    // The first page: "Previous page" does nothing.
    click(cx, "previous-page");
    assert!(sent.borrow().is_empty());
    click(cx, "next-page");
    assert_eq!(sent.borrow().last(), Some(&Command::SelectPage { page: 1 }));
    // The last page: "Next page" does nothing. A page with no name has its
    // number.
    send(cx, &tx, pages(&["Main", "Media", ""], 2));
    sent.borrow_mut().clear();
    click(cx, "next-page");
    assert!(sent.borrow().is_empty());
    click(cx, "previous-page");
    assert_eq!(sent.borrow().last(), Some(&Command::SelectPage { page: 1 }));

    // "Page settings" selects the page and shows the Inspector tab, also
    // when the right panel is hidden.
    click(cx, "tab-library");
    click(cx, "toggle-inspector");
    assert!(cx.debug_bounds("right-panel").is_none());
    click(cx, "page-settings");
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert!(view.state.page_selected);
        assert!(view.panes.inspector_visible);
        assert_eq!(
            view.panes.right_panel_tab,
            crate::settings::RightPanelTab::Inspector
        );
    });
    assert!(cx.debug_bounds("editor").is_some());
}

#[gpui_kit::test]
fn an_empty_key_face_shows_a_plus_and_the_knob_rings_show_the_light_colours(
    cx: &mut TestAppContext,
) {
    let (view, cx, tx, _) = window(cx);
    send(cx, &tx, Event::Layout(Arc::new(n4_pro())));
    let face = |n| SurfaceId::new(SurfaceKind::KeyFace, n);
    let content = |label: Option<&str>| SurfaceContent {
        label: label.map(Into::into),
        ..Default::default()
    };
    for (n, label) in [(11, Some("Docs")), (12, None), (13, None)] {
        send(
            cx,
            &tx,
            Event::Surface(SurfaceState {
                id: face(n),
                content: content(label),
                image: Arc::new(RgbImage::new(112, 112)),
            }),
        );
    }
    // Key 13 has an action, and no label.
    send(
        cx,
        &tx,
        Event::Binding(BindingState {
            binding: BindingId {
                page: 0,
                control: ControlId::new(ControlKind::Key, 13),
                gesture: GestureKind::Press,
            },
            action: Some(Action {
                kind: "test.kind".into(),
                settings: Settings::new(),
            }),
            available: true,
        }),
    );
    assert!(cx.debug_bounds("empty-key face 11").is_none());
    assert!(cx.debug_bounds("empty-key face 12").is_some());
    assert!(cx.debug_bounds("empty-key face 13").is_none());
    // The bar panels have no plus.
    assert!(cx.debug_bounds("empty-bar panel 1").is_none());

    send(
        cx,
        &tx,
        Event::DeviceSettings(DeviceSettings {
            brightness: 80,
            haptic: true,
            knob_lights: KnobLights {
                mode: KnobLightMode::Colors,
                brightness: 100,
                colors: vec![Color([0, 255, 0]); 4],
            },
        }),
    );
    cx.update(|_, cx| {
        let settings = view.read(cx).state.device_settings.clone();
        assert_eq!(
            crate::view::canvas::knob_ring(settings.as_ref(), ControlId::new(ControlKind::Knob, 4)),
            gpui_kit::rgb(0x00ff00)
        );
    });
}
