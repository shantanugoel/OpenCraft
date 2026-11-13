//! The "Background" section of the page inspector, and the background on
//! the canvas and in the page picture (decision 0029, sections 8.2 and
//! 10.6; decision 0032).

use super::pages::{pages, profiles, send};
use super::*;
use opencraft_engine::{
    Background, BackgroundFit, BackgroundState, DeviceLayout, NewBackground, SurfaceLayout,
};

/// The layout of the window tests with a background of 400 × 300.
fn layout_with_background() -> Event {
    let Event::Layout(layout) = layout() else {
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

/// The window with a model that has a background, two pages, the second
/// page active, and the page as the selection.
fn background_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    cx.update(|window, _| window.activate_window());
    send(cx, &tx, layout_with_background());
    send(cx, &tx, profiles(&[("work", "Work")], "work"));
    send(cx, &tx, pages(&["Main", "Media"], 1));
    click(cx, "page-settings");
    assert!(cx.update(|_, cx| view.read(cx).state.page_selected));
    sent.borrow_mut().clear();
    (view, cx, tx, sent)
}

fn image_state(fit: BackgroundFit) -> Event {
    Event::Background(BackgroundState {
        background: Some(Background::Image {
            image: "images/0123456789abcdef.png".into(),
            fit,
        }),
        image: Some(Arc::new(RgbImage::from_pixel(
            400,
            300,
            image::Rgb([200, 30, 30]),
        ))),
    })
}

fn colour_state(color: Color) -> Event {
    Event::Background(BackgroundState {
        background: Some(Background::Color(color)),
        image: Some(Arc::new(RgbImage::from_pixel(
            400,
            300,
            image::Rgb(color.0),
        ))),
    })
}

fn has(cx: &mut VisualTestContext, id: &'static str) -> bool {
    cx.debug_bounds(id).is_some()
}

#[gpui_kit::test]
fn a_model_with_no_background_has_no_background_section(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = window(cx);
    send(cx, &tx, profiles(&[("work", "Work")], "work"));
    send(cx, &tx, pages(&["Main"], 0));
    click(cx, "page-settings");
    assert!(has(cx, "page-name"));
    assert!(!has(cx, "background-kind"));
}

#[gpui_kit::test]
fn the_colour_segment_sends_a_colour_for_the_active_page(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = background_window(cx);
    // With no background, "None" is selected and no other control shows.
    assert!(has(cx, "background-kind"));
    assert!(!has(cx, "colour-0") && !has(cx, "background-preview"));

    // "Colour" shows the samples, with no "No fill", and sends nothing.
    click(cx, "background-colour");
    assert!(sent.borrow().is_empty());
    assert!(has(cx, "colour-0") && !has(cx, "no-colour"));
    click(cx, "colour-3");
    assert_eq!(
        sent.borrow().as_slice(),
        [Command::SetBackground {
            page: 1,
            background: Some(NewBackground::Color(colour(COLOURS[3]))),
        }]
    );
    send(cx, &tx, colour_state(colour(COLOURS[3])));
    assert!(has(cx, "colour-0"));

    // "Other colour" takes a value in hex.
    click(cx, "other-colour");
    cx.simulate_input("#123456");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBackground {
            page: 1,
            background: Some(NewBackground::Color(colour(0x123456))),
        })
    );

    // "None" removes the background.
    click(cx, "background-none");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBackground {
            page: 1,
            background: None,
        })
    );
    send(cx, &tx, Event::Background(BackgroundState::default()));
    assert!(!has(cx, "colour-0"));
    cx.update(|_, cx| assert!(view.read(cx).background_image.is_none()));
}

#[gpui_kit::test]
fn the_image_segment_chooses_a_file_and_sets_the_fit(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = background_window(cx);
    click(cx, "background-image");
    assert!(has(cx, "background-preview"));
    assert!(sent.borrow().is_empty());
    // With no image yet, the fit is disabled.
    click(cx, "background-fit");
    assert!(!has(cx, "background-fit-menu"));
    click(cx, "background-choose");
    assert!(cx.did_prompt_for_paths());

    // The engine sends the image: the preview, the canvas, and the page
    // picture show it.
    send(cx, &tx, image_state(BackgroundFit::Fill));
    assert!(has(cx, "display-background"));
    assert!(has(cx, "page-picture-background"));
    click(cx, "background-fit");
    assert!(has(cx, "background-fit-menu"));
    // The selected fit does not send a command.
    click(cx, "background-fit-fill");
    assert!(sent.borrow().is_empty());
    click(cx, "background-fit");
    click(cx, "background-fit-fit");
    assert_eq!(
        sent.borrow().as_slice(),
        [Command::SetBackgroundFit {
            page: 1,
            fit: BackgroundFit::Fit,
        }]
    );
    assert!(!has(cx, "background-fit-menu"));
    // Escape closes the menu.
    click(cx, "background-fit");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!has(cx, "background-fit-menu"));
}

#[gpui_kit::test]
fn video_is_disabled_and_a_page_change_forgets_the_selected_segment(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = background_window(cx);
    click(cx, "background-video");
    assert!(sent.borrow().is_empty());
    let mode = |cx: &mut VisualTestContext| cx.update(|_, cx| view.read(cx).background_mode());
    assert_eq!(mode(cx), super::super::page_inspector::BackgroundMode::None);

    click(cx, "background-image");
    assert_eq!(
        mode(cx),
        super::super::page_inspector::BackgroundMode::Image
    );
    // A new page: the segment shows the background of that page.
    send(cx, &tx, pages(&["Main", "Media"], 0));
    assert_eq!(mode(cx), super::super::page_inspector::BackgroundMode::None);
    // A page with a colour shows "Colour".
    send(cx, &tx, colour_state(Color::rgb(1, 2, 3)));
    assert_eq!(
        mode(cx),
        super::super::page_inspector::BackgroundMode::Colour
    );
    assert!(has(cx, "display-background"));
}
