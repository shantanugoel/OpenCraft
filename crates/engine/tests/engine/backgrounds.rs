//! Page background tests: the commands, the image for the device and for
//! the shell, a page change, a new connection, the history, a copy of a
//! page, a restart, and the export and the import (issue #191, decision
//! 0032). The tests use the fake transport, temporary directories, and no
//! thread.

use super::*;
use opencraft_engine::{Background, BackgroundFit, BackgroundState, NewBackground, Page};

/// The background of the N4 Pro: 800 × 480 at 0, 0.
const SIZE: (u32, u32) = (800, 480);
/// Key face 1 is at 6, 8 on the display, in slot 11. Key face 3 is at
/// 346, 8. Bar panel 2 is at 203, 358, in slot 2.
const KEY_1: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 1);
const KEY_1_SLOT: u8 = 11;

fn command(rig: &mut Rig, command: Command) {
    assert!(rig.engine.command(command));
}

/// Write an image file with a red left half and a blue right half, of the
/// size `width` × `height`, and return its path.
fn halves(dir: &Path, width: u32, height: u32) -> PathBuf {
    let path = dir.join(format!("halves-{width}x{height}.png"));
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

fn set_image(rig: &mut Rig, page: usize, path: PathBuf, fit: BackgroundFit) {
    command(
        rig,
        Command::SetBackground {
            page,
            background: Some(NewBackground::Image { path, fit }),
        },
    );
}

/// The background states in `events`, in order.
fn backgrounds(events: &[Event]) -> Vec<&BackgroundState> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Background(state) => Some(state),
            _ => None,
        })
        .collect()
}

/// The background images in `sent`, upright.
fn shown_backgrounds(sent: &[Sent]) -> Vec<RgbImage> {
    sent.iter()
        .filter_map(|s| match s {
            Sent::Background { data } => Some(shown(data)),
            _ => None,
        })
        .collect()
}

#[test]
fn an_image_background_goes_to_the_device_and_to_the_shell_as_one_image() {
    let mut rig = Rig::new();
    let device = rig.connect();
    rig.set_color(BAR_2, BLUE);
    sent(&device);
    rig.take_events();

    let file = halves(rig.dir.path(), 1600, 960);
    set_image(&mut rig, 0, file, BackgroundFit::Fill);

    // The shell gets the background first, then each surface with no fill
    // colour, which now shows the part of the background at its position.
    let events = rig.take_events();
    let states = backgrounds(&events);
    assert_eq!(states.len(), 1);
    assert!(matches!(events[0], Event::Background(_)), "{events:?}");
    let image = states[0].image.clone().expect("an image for the shell");
    assert_eq!(image.dimensions(), SIZE);
    assert_color(&image, 10, 240, RED);
    assert_color(&image, 790, 240, BLUE);
    assert!(matches!(
        &states[0].background,
        Some(Background::Image {
            fit: BackgroundFit::Fill,
            ..
        })
    ));
    assert_eq!(surfaces(&events).len(), 14);
    assert_color(&last_state(&events, KEY_1).image, 2, 2, RED);
    assert_color(&last_state(&events, BAR_2).image, 2, 2, BLUE);

    // The device gets the same background, the changed images (bar panel 2
    // has a fill colour and does not change), and one commit. No start
    // sequence.
    let sent = sent(&device);
    assert!(matches!(sent[0], Sent::Background { .. }), "{sent:?}");
    assert_eq!(pages::image_slots(&sent).len(), 13);
    assert!(!pages::image_slots(&sent).contains(&2));
    assert_eq!(sent.last(), Some(&op("STP")));
    assert!(!sent.contains(&op("DIS")));
    assert_same_picture(&shown_backgrounds(&sent)[0], &image);
    // A key face with no fill shows the background on the device too.
    assert_color(&shown_at(&sent, KEY_1_SLOT), 50, 50, RED);
    assert_color(&shown_at(&sent, KEY_3_SLOT), 100, 50, BLUE);

    // The profile file gets the background after the delay.
    rig.poll_for(SAVE_DELAY * 2);
    let text = rig.profile_text().unwrap();
    assert!(
        text.contains("[pages.background]\nimage = \"images/"),
        "{text}"
    );
}

#[test]
fn a_colour_background_and_a_change_of_fit() {
    let mut rig = Rig::new();
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    command(
        &mut rig,
        Command::SetBackground {
            page: 0,
            background: Some(NewBackground::Color(BLUE)),
        },
    );
    let events = rig.take_events();
    let image = backgrounds(&events)[0].image.clone().unwrap();
    assert!(image.pixels().all(|p| *p == Rgb(BLUE.0)));
    assert_color(&shown_backgrounds(&sent(&device))[0], 400, 240, BLUE);
    // The fit of a colour does not exist: nothing changes.
    command(
        &mut rig,
        Command::SetBackgroundFit {
            page: 0,
            fit: BackgroundFit::Fit,
        },
    );
    assert!(rig.take_events().is_empty());
    assert!(sent(&device).is_empty());

    // A square image: "Fit" puts black at the left and at the right.
    let file = halves(rig.dir.path(), 200, 200);
    set_image(&mut rig, 0, file, BackgroundFit::Fill);
    let fill = backgrounds(&rig.take_events())[0].image.clone().unwrap();
    assert_color(&fill, 2, 240, RED);
    sent(&device);
    command(
        &mut rig,
        Command::SetBackgroundFit {
            page: 0,
            fit: BackgroundFit::Fit,
        },
    );
    let fit = backgrounds(&rig.take_events())[0].image.clone().unwrap();
    assert_color(&fit, 2, 240, Color::BLACK);
    assert_color(&fit, 200, 240, RED);
    assert_color(&shown_backgrounds(&sent(&device))[0], 2, 240, Color::BLACK);
}

#[test]
fn a_page_change_draws_the_background_of_the_new_page_or_removes_it() {
    let mut rig = Rig::new();
    command(&mut rig, Command::AddPage { copy_of: None });
    command(&mut rig, Command::SelectPage { page: 0 });
    let file = halves(rig.dir.path(), 800, 480);
    set_image(&mut rig, 0, file, BackgroundFit::Fill);
    let device = rig.connect();
    // The full draw has the background after the device settings.
    let full = sent(&device);
    assert_full_draw(&full);
    assert!(matches!(full[7], Sent::Background { .. }));
    rig.take_events();

    // Page 2 has no background: the device removes it.
    command(&mut rig, Command::SelectPage { page: 1 });
    let events = rig.take_events();
    assert_eq!(
        backgrounds(&events),
        [&BackgroundState::default()],
        "the shell knows that the page has no background"
    );
    let to_plain = sent(&device);
    assert_eq!(to_plain[0], op("BGCLE"), "{to_plain:?}");
    assert_eq!(pages::image_slots(&to_plain).len(), 14);
    assert_color(&shown_at(&to_plain, KEY_1_SLOT), 50, 50, Color::BLACK);
    assert_eq!(to_plain.last(), Some(&op("STP")));

    // Back to page 1: the background comes back.
    command(&mut rig, Command::SelectPage { page: 0 });
    let back = sent(&device);
    assert_eq!(shown_backgrounds(&back).len(), 1);
    assert_color(&shown_at(&back, KEY_1_SLOT), 50, 50, RED);

    // A page change between two pages with no background writes no
    // background and sends no background event.
    command(&mut rig, Command::RemovePage { page: 0 });
    command(&mut rig, Command::AddPage { copy_of: None });
    rig.take_events();
    sent(&device);
    command(&mut rig, Command::SelectPage { page: 0 });
    assert!(backgrounds(&rig.take_events()).is_empty());
    let plain = sent(&device);
    assert!(!plain.contains(&op("BGCLE")) && shown_backgrounds(&plain).is_empty());
}

#[test]
fn a_new_connection_and_a_restart_show_the_background_again() {
    let mut rig = Rig::new();
    let first = rig.connect();
    let file = halves(rig.dir.path(), 800, 480);
    set_image(&mut rig, 0, file, BackgroundFit::Fill);
    sent(&first);

    first.borrow_mut().disconnect();
    rig.poll_for(Duration::from_secs(1));
    let second = rig.connect();
    let full = sent(&second);
    assert_full_draw(&full);
    let shown = &shown_backgrounds(&full)[0];
    assert_color(shown, 10, 10, RED);
    assert_color(shown, 790, 10, BLUE);

    // After a restart, the saved background shows at the start.
    rig.poll_for(SAVE_DELAY * 2);
    let mut rig = rig.restart();
    let events = rig.take_events();
    let states = backgrounds(&events);
    assert_eq!(states.len(), 1);
    assert_color(states[0].image.as_ref().unwrap(), 10, 10, RED);
    let at = events
        .iter()
        .position(|e| matches!(e, Event::Background(_)))
        .unwrap();
    assert_eq!(at, 2, "after the layout and the kinds, before the surfaces");
    let device = rig.connect();
    assert_eq!(shown_backgrounds(&sent(&device)).len(), 1);
}

#[test]
fn the_history_and_a_copy_of_the_page_cover_the_background() {
    let mut rig = Rig::new();
    let device = rig.connect();
    let file = halves(rig.dir.path(), 800, 480);
    set_image(&mut rig, 0, file, BackgroundFit::Fill);
    let events = rig.take_events();
    assert!(events.contains(&Event::History {
        undo: Some("Set Background".into()),
        redo: None
    }));
    command(
        &mut rig,
        Command::SetBackgroundFit {
            page: 0,
            fit: BackgroundFit::Fit,
        },
    );
    assert!(rig.take_events().contains(&Event::History {
        undo: Some("Set Background Fit".into()),
        redo: None
    }));
    command(
        &mut rig,
        Command::SetBackground {
            page: 0,
            background: None,
        },
    );
    assert!(rig.engine.profile().pages()[0].background.is_none());
    assert!(rig.take_events().contains(&Event::History {
        undo: Some("Remove the Background".into()),
        redo: None
    }));
    sent(&device);

    // Undo puts the background back on the device and in the shell.
    command(&mut rig, Command::Undo);
    let events = rig.take_events();
    assert!(backgrounds(&events)[0].image.is_some());
    assert_eq!(shown_backgrounds(&sent(&device)).len(), 1);
    command(&mut rig, Command::Redo);
    assert_eq!(
        backgrounds(&rig.take_events()),
        [&BackgroundState::default()]
    );
    assert!(sent(&device).contains(&op("BGCLE")));
    command(&mut rig, Command::Undo);
    sent(&device);

    // A copy of the page has the same background.
    command(&mut rig, Command::AddPage { copy_of: Some(0) });
    let pages = rig.engine.profile().pages();
    assert_eq!(pages.len(), 2);
    assert!(pages[1].background.is_some());
    assert_eq!(pages[1].background, pages[0].background);
    // The new page is the active page, and has the same background: the
    // device gets no background write.
    assert!(shown_backgrounds(&sent(&device)).is_empty());
}

#[test]
fn a_background_of_a_page_that_is_not_active_changes_only_the_profile() {
    let mut rig = Rig::new();
    command(&mut rig, Command::AddPage { copy_of: None });
    let device = rig.connect();
    sent(&device);
    rig.take_events();
    command(
        &mut rig,
        Command::SetBackground {
            page: 0,
            background: Some(NewBackground::Color(RED)),
        },
    );
    assert_eq!(
        rig.engine.profile().pages()[0].background,
        Some(Background::Color(RED))
    );
    let events = rig.take_events();
    assert!(backgrounds(&events).is_empty() && surfaces(&events).is_empty());
    assert!(sent(&device).is_empty());
}

#[test]
fn a_file_that_is_not_an_image_and_a_page_that_does_not_exist_give_a_problem() {
    let mut rig = Rig::new();
    rig.take_events();
    let bad = rig.dir.path().join("not-an-image.png");
    fs::write(&bad, b"not an image").unwrap();
    set_image(&mut rig, 0, bad, BackgroundFit::Fill);
    command(
        &mut rig,
        Command::SetBackground {
            page: 5,
            background: Some(NewBackground::Color(RED)),
        },
    );
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 2, "{events:?}");
    assert!(rig.engine.profile().pages()[0].background.is_none());
}

#[test]
fn a_background_image_stays_after_a_load_and_goes_into_an_export() {
    let dir = TempDir::new().unwrap();
    let store = store_in(dir.path());
    let image = store
        .import_background(&halves(dir.path(), 1000, 600), SIZE)
        .unwrap();
    let mut page = Page::new("Main");
    page.background = Some(Background::Image {
        image: image.clone(),
        fit: BackgroundFit::Fit,
    });
    store.save(&Profile::new("Default", vec![page])).unwrap();
    let mut rig = Rig::in_dir(dir);
    // The load removes unused images only: the background stays.
    assert!(store_in(rig.dir.path()).load_image(&image).is_ok());
    rig.take_events();

    let file = rig.dir.path().join("Default.opencraft");
    command(
        &mut rig,
        Command::ExportProfile {
            profile: "default".into(),
            path: file.clone(),
        },
    );
    command(&mut rig, Command::ImportProfile { path: file });
    let imported = rig.engine.profile();
    assert_eq!(rig.engine.active_profile(), "default-2");
    let Some(Background::Image {
        image: copy,
        fit: BackgroundFit::Fit,
    }) = &imported.pages()[0].background
    else {
        panic!("no background: {imported:?}");
    };
    let copy = ProfilesDir::new(rig.dir.path())
        .store("default-2")
        .load_image(copy)
        .unwrap();
    assert_eq!((copy.width(), copy.height()), SIZE, "not a surface copy");
}

#[test]
fn a_model_with_no_background_refuses_a_background_and_sends_no_background_command() {
    let mut rig = Rig::with_model(
        TempDir::new().unwrap(),
        Vec::new(),
        &super::test_model_data::TEST_MODEL,
    );
    rig.take_events();
    command(
        &mut rig,
        Command::SetBackground {
            page: 0,
            background: Some(NewBackground::Color(RED)),
        },
    );
    let events = rig.take_events();
    assert_eq!(problems(&events), ["This device has no background."]);
    assert!(rig.engine.profile().pages()[0].background.is_none());
    let device = rig.plug_model(&super::test_model_data::TEST_MODEL);
    rig.poll_for(Duration::from_secs(2));
    let sent = sent(&device);
    assert!(!sent.contains(&op("BGCLE")));
    assert!(shown_backgrounds(&sent).is_empty());
}
