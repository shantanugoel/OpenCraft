//! Label option tests (issue #189): each option changes the image that the
//! device and the shell get, it stays in the profile file, and a profile of
//! an earlier schema version shows as before.

use super::*;
use opencraft_engine::{Auto, LabelChange, LabelPosition, LabelStyle, Page};

fn style(rig: &mut Rig, change: LabelChange) {
    assert!(rig.engine.command(Command::SetLabelStyle {
        surface: KEY_3,
        change,
    }));
}

/// The first and the last row of `image` with a pixel near white.
#[cfg(target_os = "macos")]
fn white_rows(image: &RgbImage) -> Option<(u32, u32)> {
    let rows: Vec<u32> = image
        .enumerate_pixels()
        .filter(|(_, _, p)| p.0.iter().all(|&v| v > 230))
        .map(|(_, y, _)| y)
        .collect();
    Some((*rows.iter().min()?, *rows.iter().max()?))
}

#[cfg(target_os = "macos")]
#[test]
fn each_position_shows_on_the_device_and_in_the_event_as_the_same_image() {
    let mut rig = Rig::new();
    let device = rig.connect();
    rig.set_color(KEY_3, RED);
    rig.engine.command(Command::SetLabel {
        surface: KEY_3,
        label: Some("Mail".into()),
    });
    sent(&device);
    rig.take_events();

    let mut rows = Vec::new();
    for position in LabelPosition::ALL {
        style(&mut rig, LabelChange::Position(Some(Auto::Value(position))));
        let events = rig.take_events();
        let state = last_state(&events, KEY_3);
        assert_eq!(
            state.content.label_style.position,
            Some(Auto::Value(position))
        );
        let shown = shown_at(&sent(&device), KEY_3_SLOT);
        assert_same_picture(&shown, &state.image);
        rows.push(white_rows(&state.image));
    }
    let [Some((top, _)), Some((middle_top, middle_bottom)), Some((_, bottom)), hidden] = rows[..]
    else {
        panic!("{rows:?}")
    };
    assert!(top < 14, "top {top}");
    assert!(
        middle_top > 37 && middle_bottom < 75,
        "middle {middle_top} {middle_bottom}"
    );
    assert!(bottom > 98, "bottom {bottom}");
    assert_eq!(hidden, None, "the position none shows no label");
    assert_eq!(
        rig.engine.profile().pages()[0]
            .content(KEY_3)
            .label
            .as_deref(),
        Some("Mail"),
        "the text stays"
    );
}

#[test]
fn the_options_stay_in_the_file_and_show_after_a_new_start() {
    let mut rig = Rig::new();
    rig.engine.command(Command::SetLabel {
        surface: KEY_3,
        label: Some("Mail".into()),
    });
    style(
        &mut rig,
        LabelChange::Position(Some(Auto::Value(LabelPosition::Top))),
    );
    style(&mut rig, LabelChange::Size(Some(28)));
    style(&mut rig, LabelChange::Color(Some(Auto::Value(BLUE))));
    style(&mut rig, LabelChange::Bold(Some(false)));
    let image = last_state(&rig.take_events(), KEY_3).image.clone();
    rig.poll_for(Duration::from_secs(1));
    let text = rig.profile_text().unwrap();
    assert!(text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")));
    for line in [
        "label_position = \"top\"",
        "label_size = 28",
        "label_color = \"#1428DC\"",
        "label_bold = false",
    ] {
        assert!(text.contains(line), "{line} in {text}");
    }

    let mut rig = rig.restart();
    let events = rig.take_events();
    let state = last_state(&events, KEY_3);
    assert_eq!(
        state.content.label_style,
        LabelStyle {
            position: Some(Auto::Value(LabelPosition::Top)),
            size: Some(28),
            color: Some(Auto::Value(BLUE)),
            bold: Some(false),
            ..LabelStyle::default()
        }
    );
    assert_eq!(*state.image, *image);
}

#[test]
fn a_size_out_of_the_range_goes_into_the_range() {
    let mut rig = Rig::new();
    style(&mut rig, LabelChange::Size(Some(200)));
    assert_eq!(
        rig.engine.profile().pages()[0]
            .content(KEY_3)
            .label_style
            .size,
        Some(40)
    );
    style(&mut rig, LabelChange::Size(Some(0)));
    assert_eq!(
        rig.engine.profile().pages()[0]
            .content(KEY_3)
            .label_style
            .size,
        Some(10)
    );
}

#[test]
fn a_profile_of_version_three_shows_as_before_and_gets_the_current_version_at_the_next_change() {
    let dir = TempDir::new().unwrap();
    let store = store_in(dir.path());
    fs::create_dir_all(store.dir()).unwrap();
    let old = "schema_version = 3\nname = \"Old\"\nmodel = \"n4-pro\"\n\n\
               [[pages]]\nname = \"Page 1\"\n\n\
               [pages.surfaces.key-face-3]\nlabel = \"Mail\"\ncolor = \"#C81E1E\"\n";
    fs::write(store.profile_path(), old).unwrap();

    let mut rig = Rig::in_dir(dir);
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    let state = last_state(&events, KEY_3);
    assert_eq!(state.content.label_style, LabelStyle::default());
    // The image is the image of the same content in a new profile.
    let mut page = Page::new("Page 1");
    page.update(KEY_3, |c| {
        c.label = Some("Mail".into());
        c.color = Some(RED);
    });
    let mut new = Rig::new();
    new.set_color(KEY_3, RED);
    new.engine.command(Command::SetLabel {
        surface: KEY_3,
        label: Some("Mail".into()),
    });
    assert_eq!(new.engine.profile().pages()[0], page);
    assert_eq!(
        *last_state(&new.take_events(), KEY_3).image,
        *state.image,
        "the same look as a new label with the default options"
    );
    // The load does not write the file.
    rig.poll_for(Duration::from_secs(1));
    assert_eq!(rig.profile_text().as_deref(), Some(old));

    style(&mut rig, LabelChange::Bold(Some(false)));
    rig.poll_for(Duration::from_secs(1));
    let text = rig.profile_text().unwrap();
    assert!(
        text.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
        "{text}"
    );
    assert!(
        text.contains("label = \"Mail\"\nlabel_bold = false\n"),
        "{text}"
    );
}

#[test]
fn the_font_stays_in_the_file_and_a_missing_font_shows_the_system_font() {
    let mut rig = Rig::new();
    rig.engine.command(Command::SetLabel {
        surface: KEY_3,
        label: Some("Mail".into()),
    });
    let system = last_state(&rig.take_events(), KEY_3).image.clone();

    // A font of the user changes the image.
    style(
        &mut rig,
        LabelChange::Font(Some(Auto::Value("Menlo".into()))),
    );
    let events = rig.take_events();
    let menlo = last_state(&events, KEY_3);
    assert_eq!(
        menlo.content.label_style.font,
        Some(Auto::Value("Menlo".into()))
    );
    #[cfg(target_os = "macos")]
    assert_ne!(*menlo.image, *system, "Menlo is not the system font");

    // A font that is not installed keeps its name and shows the system font,
    // with no problem.
    style(
        &mut rig,
        LabelChange::Font(Some(Auto::Value("No Such Font Family 12345".into()))),
    );
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    let missing = last_state(&events, KEY_3);
    assert_eq!(
        missing.content.label_style.font,
        Some(Auto::Value("No Such Font Family 12345".into()))
    );
    #[cfg(target_os = "macos")]
    assert_eq!(
        *missing.image, *system,
        "a font that is not installed shows the system font"
    );

    rig.poll_for(Duration::from_secs(1));
    let text = rig.profile_text().unwrap();
    assert!(
        text.contains("label_font = \"No Such Font Family 12345\""),
        "{text}"
    );

    // The word "system" is the system font, also when the profile default
    // has a font.
    style(&mut rig, LabelChange::Font(Some(Auto::Auto)));
    rig.poll_for(Duration::from_secs(1));
    let text = rig.profile_text().unwrap();
    assert!(text.contains("label_font = \"system\""), "{text}");
    let events = rig.take_events();
    #[cfg(target_os = "macos")]
    assert_eq!(*last_state(&events, KEY_3).image, *system);

    // A new start loads the font, and a font that is missing then shows the
    // system font.
    style(
        &mut rig,
        LabelChange::Font(Some(Auto::Value("No Such Font Family 12345".into()))),
    );
    rig.take_events();
    rig.poll_for(Duration::from_secs(1));
    let mut rig = rig.restart();
    let events = rig.take_events();
    let state = last_state(&events, KEY_3);
    assert!(problems(&events).is_empty(), "{events:?}");
    assert_eq!(
        state.content.label_style.font,
        Some(Auto::Value("No Such Font Family 12345".into()))
    );
    #[cfg(target_os = "macos")]
    assert_eq!(*state.image, *system);
}
