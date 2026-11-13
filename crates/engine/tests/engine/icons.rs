//! Icon tests (issue #190, decision 0033): the engine draws an icon of the
//! built-in set on a key face and on a bar panel, the device and the shell
//! get the same image, and the icon stays in the profile file, in the
//! history, in the clipboard, and in a profile export.

use super::*;
use opencraft_engine::{share, Page, SurfaceIcon};

const BAR_2_SLOT: u8 = 2;
const AMBER: Color = Color::rgb(245, 158, 11);

fn command(rig: &mut Rig, command: Command) {
    assert!(rig.engine.command(command));
}

fn icon(name: &str, color: Option<Color>) -> Option<SurfaceIcon> {
    Some(SurfaceIcon {
        name: name.into(),
        color,
    })
}

fn set_icon(rig: &mut Rig, surface: SurfaceId, value: Option<SurfaceIcon>) {
    command(
        rig,
        Command::SetIcon {
            surface,
            icon: value,
        },
    );
}

/// The number of pixels of `image` near `color`.
fn count_of(image: &RgbImage, color: Color) -> usize {
    image
        .pixels()
        .filter(|p| (0..3).all(|i| p[i].abs_diff(color.0[i]) <= 12))
        .count()
}

/// The undo name of the last history event since the last call.
fn last_undo(rig: &mut Rig) -> Option<Option<String>> {
    rig.take_events().into_iter().rev().find_map(|e| match e {
        Event::History { undo, .. } => Some(undo),
        _ => None,
    })
}

#[test]
fn an_icon_shows_on_a_key_face_and_a_bar_panel_as_the_same_image_on_the_device_and_in_the_event() {
    let mut rig = Rig::new();
    let device = rig.connect();
    rig.set_color(KEY_3, RED);
    rig.set_color(BAR_2, BLUE);
    sent(&device);
    rig.take_events();

    for (surface, slot) in [(KEY_3, KEY_3_SLOT), (BAR_2, BAR_2_SLOT)] {
        set_icon(&mut rig, surface, icon("square", Some(AMBER)));
        let events = rig.take_events();
        let state = last_state(&events, surface);
        assert_eq!(state.content.icon, icon("square", Some(AMBER)));
        let shown = shown_at(&sent(&device), slot);
        assert_same_picture(&shown, &state.image);
        // The icon "square" of 72 at the centre: its top edge is 6 below the
        // top of the square, and 6 wide.
        let (w, h) = state.image.dimensions();
        let top = (h - 72) / 2;
        assert_color(&state.image, w / 2, top + 8, AMBER);
        assert_color(&shown, w / 2, top + 8, AMBER);
        assert!(count_of(&state.image, AMBER) > 600, "{surface}");
        // The fill shows inside the icon.
        assert_ne!(state.image.get_pixel(w / 2, h / 2).0, AMBER.0);
    }
}

#[test]
fn an_icon_replaces_the_image_and_an_image_replaces_the_icon() {
    let mut rig = Rig::new();
    let file = green_file(rig.dir.path());
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: Some(file.clone()),
        },
    );
    set_icon(&mut rig, KEY_3, icon("globe", None));
    let content = rig.engine.profile().pages()[0].content(KEY_3);
    assert_eq!(content.image, None);
    assert_eq!(content.icon, icon("globe", None));

    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: Some(file),
        },
    );
    let content = rig.engine.profile().pages()[0].content(KEY_3);
    assert!(content.image.is_some());
    assert_eq!(content.icon, None);

    // "No icon" removes the image also.
    set_icon(&mut rig, KEY_3, icon("globe", None));
    let file = green_file(rig.dir.path());
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: Some(file),
        },
    );
    set_icon(&mut rig, KEY_3, None);
    let content = rig.engine.profile().pages()[0].content(KEY_3);
    assert_eq!((content.image, content.icon), (None, None));
}

#[test]
fn an_icon_that_the_set_does_not_have_gives_a_problem_and_no_change() {
    let mut rig = Rig::new();
    rig.take_events();
    set_icon(&mut rig, KEY_3, icon("no-such-icon", None));
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 1, "{events:?}");
    assert!(surfaces(&events).is_empty());
    assert!(rig.engine.profile().pages()[0].content(KEY_3).is_empty());
}

#[test]
fn an_icon_and_its_colour_stay_in_the_profile_file() {
    let mut rig = Rig::new();
    set_icon(&mut rig, KEY_3, icon("mail", Some(AMBER)));
    set_icon(&mut rig, BAR_2, icon("volume-2", None));
    rig.poll_for(SAVE_DELAY + POLL_INTERVAL);
    let text = rig.profile_text().unwrap();
    assert!(
        text.contains("icon = \"mail\"\nicon_color = \"#F59E0B\""),
        "{text}"
    );
    assert!(text.contains("icon = \"volume-2\""), "{text}");
    let rig = rig.restart();
    let page = &rig.engine.profile().pages()[0];
    assert_eq!(page.content(KEY_3).icon, icon("mail", Some(AMBER)));
    assert_eq!(page.content(BAR_2).icon, icon("volume-2", None));
}

#[test]
fn a_change_of_the_icon_is_a_step_of_the_history() {
    let mut rig = Rig::new();
    rig.take_events();
    let empty = rig.engine.profile().clone();
    set_icon(&mut rig, KEY_3, icon("globe", None));
    assert_eq!(last_undo(&mut rig), Some(Some("Set Icon".into())));
    let globe = rig.engine.profile().clone();
    set_icon(&mut rig, KEY_3, icon("globe", Some(AMBER)));
    assert_eq!(last_undo(&mut rig), Some(Some("Set Icon".into())));
    let amber = rig.engine.profile().clone();
    set_icon(&mut rig, KEY_3, None);
    assert_eq!(last_undo(&mut rig), Some(Some("Remove the Icon".into())));

    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.profile(), &amber);
    let events = rig.take_events();
    assert_eq!(
        last_state(&events, KEY_3).content.icon,
        icon("globe", Some(AMBER))
    );
    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.profile(), &globe);
    command(&mut rig, Command::Undo);
    assert_eq!(rig.engine.profile(), &empty);
    command(&mut rig, Command::Redo);
    assert_eq!(rig.engine.profile(), &globe);
}

#[test]
fn copy_paste_clear_and_copy_to_all_pages_take_the_icon() {
    let dir = TempDir::new().unwrap();
    let mut a = Page::new("A");
    a.update(KEY_3, |c| {
        c.label = Some("Web".into());
        c.icon = icon("globe", Some(AMBER));
    });
    store_in(dir.path())
        .save(&Profile::new(
            "Icons",
            vec![a, Page::new("B"), Page::new("C")],
        ))
        .unwrap();
    let mut rig = Rig::in_dir(dir);
    let key_3 = ControlId::new(ControlKind::Key, 3);
    let key_4 = ControlId::new(ControlKind::Key, 4);
    let key_4_face = SurfaceId::new(SurfaceKind::KeyFace, 4);
    let content = rig.engine.profile().pages()[0].content(KEY_3);

    command(&mut rig, Command::CopyControl { control: key_3 });
    command(&mut rig, Command::SelectPage { page: 1 });
    command(&mut rig, Command::PasteControl { control: key_4 });
    assert_eq!(rig.engine.profile().pages()[1].content(key_4_face), content);

    command(&mut rig, Command::ClearControl { control: key_4 });
    assert!(rig.engine.profile().pages()[1]
        .content(key_4_face)
        .is_empty());

    command(&mut rig, Command::SelectPage { page: 0 });
    command(&mut rig, Command::CopyToAllPages { control: key_3 });
    for page in rig.engine.profile().pages() {
        assert_eq!(page.content(KEY_3), content, "{}", page.name);
    }
}

#[test]
fn an_export_and_an_import_keep_the_icon() {
    let mut rig = Rig::new();
    set_icon(&mut rig, KEY_3, icon("music", Some(AMBER)));
    let out = TempDir::new().unwrap();
    let file = out.path().join("Icons.opencraft");
    command(
        &mut rig,
        Command::ExportProfile {
            profile: "default".into(),
            path: file.clone(),
        },
    );
    let package = share::import(&file).unwrap();
    assert_eq!(
        package.profile.pages()[0].content(KEY_3).icon,
        icon("music", Some(AMBER))
    );
    command(&mut rig, Command::ImportProfile { path: file });
    assert_ne!(rig.engine.active_profile(), "default");
    assert_eq!(
        rig.engine.profile().pages()[0].content(KEY_3).icon,
        icon("music", Some(AMBER))
    );
}
