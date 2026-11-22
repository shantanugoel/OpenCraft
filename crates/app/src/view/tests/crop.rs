//! The crop of an image in the "Appearance" section (decision 0043, KEY-02):
//! "Zoom", "Offset", "Reset", and the warning that the image is too small.

use super::*;
use opencraft_engine::{ImageCrop, SurfaceIcon};

/// The crop commands in `sent`.
fn crops(sent: &Commands) -> Vec<(SurfaceId, Option<ImageCrop>)> {
    sent.borrow()
        .iter()
        .filter_map(|c| match c {
            Command::SetImageCrop { surface, crop } => Some((*surface, *crop)),
            _ => None,
        })
        .collect()
}

/// A surface event with an image, the crop `crop`, and the sizes that the
/// engine sends for the image and for the area of the picture.
pub(super) fn image_surface(
    id: SurfaceId,
    crop: Option<ImageCrop>,
    image_size: (u32, u32),
    image_area: (u32, u32),
) -> Event {
    Event::Surface(SurfaceState {
        id,
        content: SurfaceContent {
            label: Some("Mail".into()),
            image: Some("images/0123456789abcdef.png".into()),
            image_crop: crop,
            ..Default::default()
        },
        image: Arc::new(RgbImage::new(image_area.0, image_area.1)),
        image_size: Some(image_size),
        image_area: Some(image_area),
        states: None,
    })
}

/// The warning of the small image of the inspector, if it shows.
pub(super) fn warning_shows(cx: &mut VisualTestContext) -> bool {
    cx.debug_bounds("image-small-warning").is_some()
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
    // A new frame, so that the elements of the new state are in the tree.
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

/// The key face 2 selected, with an image of `size` and an area.
fn window_with_image(
    cx: &mut TestAppContext,
    crop: Option<ImageCrop>,
    size: (u32, u32),
    area: (u32, u32),
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    send(cx, &tx, image_surface(key(), crop, size, area));
    (view, cx, tx, sent)
}

fn zoom(value: u16, x: i16, y: i16) -> Option<ImageCrop> {
    Some(ImageCrop { zoom: value, x, y })
}

#[gpui_kit::test]
fn the_crop_rows_show_only_when_the_face_has_an_image(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    let ids = [
        "image-zoom",
        "image-offset-left",
        "image-offset-right",
        "image-offset-up",
        "image-offset-down",
        "image-reset",
    ];
    for id in ids {
        assert!(cx.debug_bounds(id).is_none(), "{id} with no image");
    }
    send(cx, &tx, image_surface(key(), None, (64, 64), (112, 112)));
    for id in ids {
        assert!(cx.debug_bounds(id).is_some(), "no {id} with an image");
    }
    // A bar panel has them too.
    click(cx, "surface-bar panel 3");
    send(cx, &tx, image_surface(bar(), None, (64, 64), (176, 112)));
    assert!(cx.debug_bounds("image-zoom").is_some());
    // A face with an icon has no crop.
    send(
        cx,
        &tx,
        Event::Surface(SurfaceState {
            id: bar(),
            content: SurfaceContent {
                icon: Some(SurfaceIcon::new("star")),
                ..Default::default()
            },
            image: Arc::new(RgbImage::new(176, 112)),
            image_size: None,
            image_area: None,
            states: None,
        }),
    );
    assert!(cx.debug_bounds("image-zoom").is_none());
    assert!(sent.borrow().is_empty(), "the rows send nothing");
}

#[gpui_kit::test]
fn the_zoom_menu_sets_each_zoom_of_the_image(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = window_with_image(cx, None, (300, 200), (112, 112));
    assert!(cx.debug_bounds("image-zoom-menu").is_none());
    for value in opencraft_engine::IMAGE_ZOOMS {
        click(cx, "image-zoom");
        assert!(cx.debug_bounds("image-zoom-menu").is_some(), "{value}");
        // A test ID must be static.
        let id: &'static str = format!("image-zoom-{value}").leak();
        click(cx, id);
        assert!(cx.debug_bounds("image-zoom-menu").is_none(), "{value}");
    }
    // The fit is no crop.
    let want: Vec<_> = opencraft_engine::IMAGE_ZOOMS
        .into_iter()
        .map(|value| (key(), (value > 100).then(|| zoom(value, 0, 0)).flatten()))
        .collect();
    assert_eq!(crops(&sent), want);
    // Escape closes the menu with no change.
    sent.borrow_mut().clear();
    click(cx, "image-zoom");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("image-zoom-menu").is_none());
    assert!(crops(&sent).is_empty());
}

#[gpui_kit::test]
fn each_arrow_moves_the_image_by_a_step_of_the_overflow(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window_with_image(cx, zoom(200, 0, 0), (100, 100), (112, 112));
    for (id, want) in [
        ("image-offset-left", zoom(200, -100, 0)),
        ("image-offset-right", zoom(200, 100, 0)),
        ("image-offset-up", zoom(200, 0, -100)),
        ("image-offset-down", zoom(200, 0, 100)),
    ] {
        click(cx, id);
        assert_eq!(crops(&sent).last(), Some(&(key(), want)), "{id}");
        sent.borrow_mut().clear();
    }
    // The pan stays in its range: a value at the edge does not go further.
    send(
        cx,
        &tx,
        image_surface(key(), zoom(200, 1000, 1000), (100, 100), (112, 112)),
    );
    click(cx, "image-offset-right");
    assert_eq!(
        crops(&sent).last(),
        Some(&(key(), zoom(200, 1000, 1000))),
        "the right edge"
    );
    sent.borrow_mut().clear();
    click(cx, "image-offset-down");
    assert_eq!(
        crops(&sent).last(),
        Some(&(key(), zoom(200, 1000, 1000))),
        "the bottom edge"
    );
}

#[gpui_kit::test]
fn an_arrow_of_an_axis_with_no_overflow_is_disabled(cx: &mut TestAppContext) {
    // A small image at the fit has no overflow on either axis: an arrow
    // changes no pixel, thus it does nothing.
    let (_view, cx, tx, sent) = window_with_image(cx, None, (10, 10), (112, 112));
    for id in [
        "image-offset-left",
        "image-offset-right",
        "image-offset-up",
        "image-offset-down",
    ] {
        click(cx, id);
        assert!(crops(&sent).is_empty(), "{id}");
    }
    // At zoom 200 the image is 224 × 224: both axes move.
    send(
        cx,
        &tx,
        image_surface(key(), zoom(200, 0, 0), (10, 10), (112, 112)),
    );
    for id in ["image-offset-left", "image-offset-up"] {
        click(cx, id);
        assert_eq!(crops(&sent).len(), 1, "{id}");
        sent.borrow_mut().clear();
    }
    // A wide image at zoom 200 fills the height of a key face: only the x
    // axis moves.
    send(
        cx,
        &tx,
        image_surface(key(), zoom(200, 0, 0), (200, 50), (112, 112)),
    );
    sent.borrow_mut().clear();
    click(cx, "image-offset-down");
    assert!(crops(&sent).is_empty(), "the y axis has no overflow");
    click(cx, "image-offset-right");
    assert_eq!(crops(&sent).len(), 1, "the x axis has the overflow");
}

#[gpui_kit::test]
fn reset_removes_the_crop_and_is_disabled_at_the_fit(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window_with_image(cx, zoom(300, 500, -500), (300, 200), (112, 112));
    click(cx, "image-reset");
    assert_eq!(crops(&sent), [(key(), None)]);
    // The fit has no crop to reset.
    sent.borrow_mut().clear();
    send(cx, &tx, image_surface(key(), None, (300, 200), (112, 112)));
    click(cx, "image-reset");
    assert!(crops(&sent).is_empty());
}

#[gpui_kit::test]
fn the_warning_shows_when_the_image_is_smaller_than_the_area(cx: &mut TestAppContext) {
    // 64 × 64 pixels on a key face of 112 × 112.
    let (_view, cx, tx, _sent) = window_with_image(cx, None, (64, 64), (112, 112));
    assert!(warning_shows(cx));
    // 512 × 512 pixels is enough for the fit.
    send(cx, &tx, image_surface(key(), None, (512, 512), (112, 112)));
    assert!(!warning_shows(cx));
    // At 400 % a copy of 512 pixels gives 128: enough for a key face, and not
    // enough for a bar panel of 176.
    send(
        cx,
        &tx,
        image_surface(key(), zoom(400, 0, 0), (512, 512), (112, 112)),
    );
    assert!(!warning_shows(cx));
    send(
        cx,
        &tx,
        image_surface(bar(), zoom(400, 0, 0), (512, 512), (176, 112)),
    );
    click(cx, "surface-bar panel 3");
    assert!(warning_shows(cx));
    // A key face of 112 with an image of 256 at 400 %: 64 pixels are too few.
    click(cx, "surface-key face 2");
    send(
        cx,
        &tx,
        image_surface(key(), zoom(400, 0, 0), (256, 256), (112, 112)),
    );
    assert!(warning_shows(cx));
    // No size from the engine: no warning.
    let mut event = image_surface(key(), zoom(400, 0, 0), (256, 256), (112, 112));
    let Event::Surface(state) = &mut event else {
        panic!()
    };
    state.image_size = None;
    state.image_area = None;
    send(cx, &tx, event);
    assert!(!warning_shows(cx));
}

#[gpui_kit::test]
fn the_crop_goes_to_the_selected_surface(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window_with_image(cx, None, (300, 200), (112, 112));
    send(cx, &tx, image_surface(bar(), None, (300, 200), (176, 112)));
    click(cx, "surface-bar panel 3");
    click(cx, "image-zoom");
    click(cx, "image-zoom-300");
    assert_eq!(crops(&sent), [(bar(), zoom(300, 0, 0))]);
    sent.borrow_mut().clear();
    click(cx, "surface-key face 2");
    click(cx, "image-zoom");
    click(cx, "image-zoom-200");
    assert_eq!(crops(&sent), [(key(), zoom(200, 0, 0))]);
}
