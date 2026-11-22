//! The crop of a surface image (decision 0043, KEY-02): the zoom and the pan,
//! the size of the image and of its area in the state, and the crop in the
//! profile file, the history, and the clipboard.

use super::*;
use opencraft_engine::{ImageCrop, SurfaceIcon};

const BAR_2_SLOT: u8 = 2;

fn command(rig: &mut Rig, command: Command) {
    assert!(rig.engine.command(command));
}

/// The last state of `surface` in the events since the last call.
fn last(rig: &mut Rig, surface: SurfaceId) -> SurfaceState {
    last_state(&rig.take_events(), surface).clone()
}

/// A file with an image of two colours side by side: the left half in `RED`
/// and the right half in `BLUE`.
fn halves(dir: &Path, width: u32, height: u32) -> PathBuf {
    let path = dir.join(format!("crop-{width}x{height}.png"));
    RgbImage::from_fn(width, height, |x, _| {
        if x < width / 2 {
            Rgb(RED.0)
        } else {
            Rgb(BLUE.0)
        }
    })
    .save(&path)
    .unwrap();
    path
}

fn set_image(rig: &mut Rig, surface: SurfaceId, path: PathBuf) {
    command(
        rig,
        Command::SetImage {
            surface,
            path: Some(path),
        },
    );
}

fn set_crop(rig: &mut Rig, surface: SurfaceId, crop: Option<ImageCrop>) {
    command(rig, Command::SetImageCrop { surface, crop });
}

/// The crop for `zoom` percent, moved to the left edge and to the top.
fn at_left_top(zoom: u16) -> Option<ImageCrop> {
    Some(ImageCrop {
        zoom,
        x: 1000,
        y: 1000,
    })
}

/// The name of the undo step of the last change in `events`.
fn undo_name(events: &[Event]) -> Option<Option<String>> {
    events.iter().rev().find_map(|e| match e {
        Event::History { undo, .. } => Some(undo.clone()),
        _ => None,
    })
}

/// The profile of the active profile directory, after the write.
fn saved(rig: &mut Rig) -> opencraft_engine::Profile {
    rig.poll_for(SAVE_DELAY + POLL_INTERVAL);
    store_in(rig.dir.path()).read().unwrap()
}

#[test]
fn the_state_gives_the_size_of_the_image_and_of_the_area_of_the_picture() {
    let mut rig = Rig::new();
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    let events = rig.take_events();
    let state = last_state(&events, KEY_3);
    assert_eq!(state.image_size, Some((120, 60)));
    assert_eq!(state.image_area, Some((112, 112)));
    // The On face shows the image of the base face, and its area is the same.
    assert_eq!(state.on_image_size, Some((120, 60)));
    assert_eq!(state.on_image_area, Some((112, 112)));

    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, BAR_2, file);
    let events = rig.take_events();
    let state = last_state(&events, BAR_2);
    assert_eq!(state.image_size, Some((120, 60)));
    assert_eq!(state.image_area, Some((176, 112)));

    // A face with no image has no size and no area.
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: None,
        },
    );
    let state = last(&mut rig, KEY_3);
    assert_eq!(state.image_size, None);
    assert_eq!(state.image_area, None);
    assert_eq!(state.on_image_size, None);
    assert_eq!(state.on_image_area, None);
    let _ = state;
}

/// The On face has the size of its own image, and the area of its own band of
/// the label (M3 of review 2): the inspector reads them for the warning that
/// the image is too small.
#[test]
fn the_state_gives_the_size_of_the_image_of_the_on_face() {
    let mut rig = Rig::new();
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    // A small image on the On face only.
    let small = halves(rig.dir.path(), 40, 40);
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: Some(small),
        }
        .for_face(true),
    );
    let state = last(&mut rig, KEY_3);
    assert_eq!(state.image_size, Some((120, 60)));
    assert_eq!(state.image_area, Some((112, 112)));
    assert_eq!(state.on_image_size, Some((40, 40)));
    assert_eq!(state.on_image_area, Some((112, 112)));

    // A label at the bottom of the On face shrinks its area, and not the area
    // of the base face. The height of the band of the label comes from the
    // font of the platform, thus the test reads the width and a smaller area.
    command(
        &mut rig,
        Command::SetLabel {
            surface: KEY_3,
            label: Some("Mail".into()),
        }
        .for_face(true),
    );
    let state = last(&mut rig, KEY_3);
    assert_eq!(state.image_area, Some((112, 112)));
    let (width, height) = state.on_image_area.expect("the area of the On face");
    assert_eq!(width, 112);
    assert!(height < 112, "the label takes no row of the On face");

    // An On face with no image shows no picture, and the base face keeps
    // its own.
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: None,
        }
        .for_face(true),
    );
    let state = last(&mut rig, KEY_3);
    assert_eq!(state.image_size, Some((120, 60)));
    assert_eq!(state.on_image_size, None);
    assert_eq!(state.on_image_area, None);
}

#[test]
fn the_zoom_and_the_pan_show_on_the_device_and_in_the_state() {
    let mut rig = Rig::new();
    let device = rig.connect();
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    rig.take_events();
    sent(&device);

    // 120 × 60 fits a key face at 112 × 56, at the centre. At zoom 200 it is
    // 224 × 112, and the pan to the left edge shows the left half of the
    // image on all of the face.
    set_crop(&mut rig, KEY_3, at_left_top(200));
    let events = rig.take_events();
    assert_eq!(undo_name(&events), Some(Some("Set Image Crop".into())));
    let state = last_state(&events, KEY_3);
    assert_eq!(state.content.image_crop, at_left_top(200));
    assert_color(&state.image, 10, 10, RED);
    assert_color(&state.image, 100, 100, RED);
    let shown = shown_at(&sent(&device), KEY_3_SLOT);
    assert_same_picture(&shown, &state.image);
    assert_color(&shown, 10, 10, RED);

    // The fit shows the two halves again.
    set_crop(&mut rig, KEY_3, None);
    let events = rig.take_events();
    assert_eq!(
        undo_name(&events),
        Some(Some("Reset the Image Crop".into()))
    );
    let state = last_state(&events, KEY_3);
    assert_eq!(state.content.image_crop, None);
    assert_color(&state.image, 10, 56, RED);
    assert_color(&state.image, 100, 56, BLUE);
    // The black band above the fitted image.
    assert_eq!(state.image.get_pixel(56, 10), &Rgb([0, 0, 0]));

    // A bar panel takes the same crop, and the device shows the same picture.
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, BAR_2, file);
    set_crop(&mut rig, BAR_2, at_left_top(400));
    let events = rig.take_events();
    let state = last_state(&events, BAR_2);
    assert_eq!(state.content.image_crop, at_left_top(400));
    let shown = shown_at(&sent(&device), BAR_2_SLOT);
    assert_same_picture(&shown, &state.image);
    assert_color(&shown, 10, 10, RED);
}

#[test]
fn the_engine_puts_the_zoom_and_the_pan_into_their_range() {
    let mut rig = Rig::new();
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    for (crop, want) in [
        (
            ImageCrop {
                zoom: 900,
                x: 4000,
                y: -4000,
            },
            ImageCrop {
                zoom: 400,
                x: 1000,
                y: -1000,
            },
        ),
        (
            ImageCrop {
                zoom: 1,
                x: 0,
                y: 0,
            },
            ImageCrop {
                zoom: 100,
                x: 0,
                y: 0,
            },
        ),
    ] {
        set_crop(&mut rig, KEY_3, Some(crop));
        let state = last(&mut rig, KEY_3);
        // The fit is no crop.
        let want = (!want.is_fit()).then_some(want);
        assert_eq!(state.content.image_crop, want, "{crop:?}");
    }
}

#[test]
fn a_face_with_no_image_takes_no_crop() {
    let mut rig = Rig::new();
    rig.set_color(KEY_3, RED);
    rig.take_events();
    set_crop(&mut rig, KEY_3, at_left_top(200));
    let events = rig.take_events();
    assert_eq!(undo_name(&events), None);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::Surface(state) if state.id == KEY_3)),
        "the face with no image changed"
    );
}

#[test]
fn a_new_image_or_an_icon_removes_the_crop() {
    let mut rig = Rig::new();
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    set_crop(&mut rig, KEY_3, at_left_top(200));
    assert!(last(&mut rig, KEY_3).content.image_crop.is_some());

    // Another image.
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    assert_eq!(last(&mut rig, KEY_3).content.image_crop, None);

    // An icon.
    set_crop(&mut rig, KEY_3, at_left_top(300));
    command(
        &mut rig,
        Command::SetIcon {
            surface: KEY_3,
            icon: Some(SurfaceIcon::new("star")),
        },
    );
    let state = last(&mut rig, KEY_3);
    assert_eq!(state.content.image_crop, None);
    assert!(state.content.image.is_none());

    // The image that replaces the icon has no crop of the icon.
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    assert_eq!(last(&mut rig, KEY_3).content.image_crop, None);
}

#[test]
fn the_on_face_has_its_own_crop() {
    let mut rig = Rig::new();
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    set_crop(&mut rig, KEY_3, at_left_top(200));
    rig.take_events();
    command(
        &mut rig,
        Command::SetImageCrop {
            surface: KEY_3,
            crop: at_left_top(400),
        }
        .for_face(true),
    );
    let events = rig.take_events();
    assert_eq!(undo_name(&events), Some(Some("Set Image Crop".into())));
    let state = last_state(&events, KEY_3);
    assert_eq!(state.content.image_crop, at_left_top(200));
    assert_eq!(
        state.content.on.as_deref().and_then(|on| on.image_crop),
        at_left_top(400)
    );

    // The file keeps the two crops apart.
    let profile = saved(&mut rig);
    let content = profile.pages()[0].content(KEY_3);
    assert_eq!(content.image_crop, at_left_top(200));
    assert_eq!(
        content.on.as_deref().and_then(|on| on.image_crop),
        at_left_top(400)
    );
}

#[test]
fn undo_and_redo_cover_the_crop() {
    let mut rig = Rig::new();
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    set_crop(&mut rig, KEY_3, at_left_top(200));
    rig.take_events();
    command(&mut rig, Command::Undo);
    assert_eq!(last(&mut rig, KEY_3).content.image_crop, None);
    command(&mut rig, Command::Redo);
    assert_eq!(last(&mut rig, KEY_3).content.image_crop, at_left_top(200));
}

#[test]
fn a_copy_and_a_paste_cover_the_crop() {
    let mut rig = Rig::new();
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    set_crop(&mut rig, KEY_3, at_left_top(200));
    let source = SurfaceId::new(SurfaceKind::KeyFace, 4);
    command(
        &mut rig,
        Command::CopyControl {
            control: ControlId::new(ControlKind::Key, 3),
        },
    );
    command(
        &mut rig,
        Command::PasteControl {
            control: ControlId::new(ControlKind::Key, 4),
        },
    );
    let state = last(&mut rig, source);
    assert_eq!(state.content.image_crop, at_left_top(200));
}

#[test]
fn crop_and_zoom_stay_after_a_restart() {
    let mut rig = Rig::new();
    let file = halves(rig.dir.path(), 120, 60);
    set_image(&mut rig, KEY_3, file);
    set_crop(&mut rig, KEY_3, at_left_top(200));
    let image = last(&mut rig, KEY_3).image.clone();
    let text = {
        rig.poll_for(SAVE_DELAY + POLL_INTERVAL);
        rig.profile_text().unwrap()
    };
    for line in [
        "image_zoom = 200",
        "image_offset_x = 1000",
        "image_offset_y = 1000",
    ] {
        assert!(text.contains(line), "{line} in {text}");
    }

    // The new engine reads the same image, with the same crop.
    let mut rig = rig.restart();
    let events = rig.take_events();
    let state = last_state(&events, KEY_3);
    assert_eq!(state.content.image_crop, at_left_top(200));
    assert_eq!(*state.image, *image);
}
