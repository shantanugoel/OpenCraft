//! The schema versions of the profile file (decision 0043): the version 8
//! engine loads a file of each schema version 1 to 7 with no loss, and it
//! writes version 8.

use super::*;
use opencraft_engine::{Auto, LabelPosition, LabelStyle, ResolvedLabelStyle};

/// The text of a profile of schema `version` 1 to 7, with the keys of that
/// version: one page with one key face, its label, and the options that the
/// version has.
fn old_text(version: u32) -> String {
    let model = if version >= 3 {
        "model = \"n4-pro\"\n"
    } else {
        ""
    };
    let id = if version >= 6 {
        "id = \"000000000000002a\"\n"
    } else {
        ""
    };
    let mut text = format!(
        "schema_version = {version}\nname = \"Old\"\n{model}\n[[pages]]\n{id}name = \"Main\"\n"
    );
    // Version 4 adds the label options, the fill colour, and the icon.
    let options = if version >= 4 {
        "label_position = \"top\"\nlabel_size = 24\nlabel_color = \"#FF0000\"\n\
         label_bold = false\ncolor = \"#1E6FD9\"\n"
    } else {
        ""
    };
    text.push_str(&format!(
        "[pages.surfaces.key-face-3]\nlabel = \"Mail\"\n{options}"
    ));
    // Version 7 adds the On face.
    if version >= 7 {
        text.push_str("[pages.surfaces.key-face-3.on]\nlabel = \"Live\"\n");
    }
    // Version 2 adds the bindings.
    if version >= 2 {
        text.push_str("[pages.bindings.key-3.press]\nkind = \"test.schema\"\n");
    }
    // Version 5 adds the folders and the links.
    if version >= 5 {
        text.push_str(
            "[[folders]]\nid = \"0000000000000002\"\nname = \"More\"\n\n\
             [[applications]]\nbundle_id = \"com.apple.Safari\"\nname = \"Safari\"\n",
        );
    }
    text
}

#[test]
fn each_old_schema_version_loads_with_its_pages_surfaces_and_defaults() {
    let control = opencraft_core::ControlId::new(ControlKind::Key, 3);
    for version in 1..SCHEMA_VERSION {
        let dir = TempDir::new().unwrap();
        let store = store_in(dir.path());
        fs::create_dir_all(store.dir()).unwrap();
        let text = old_text(version);
        fs::write(store.profile_path(), &text).unwrap();

        let mut rig = Rig::in_dir(dir);
        let events = rig.take_events();
        assert!(
            problems(&events).is_empty(),
            "version {version}: {events:?}"
        );
        // The load writes nothing.
        rig.poll_for(Duration::from_secs(1));
        assert_eq!(
            rig.profile_text().as_deref(),
            Some(text.as_str()),
            "version {version}"
        );
        let profile = rig.engine.profile().clone();
        assert_eq!(profile.model, "n4-pro", "version {version}");
        assert_eq!(profile.name, "Old");
        assert_eq!(profile.pages().len(), 1);
        let page = &profile.pages()[0];
        assert_eq!(page.name, "Main");
        assert_eq!(
            page.id().to_string(),
            if version >= 6 {
                "000000000000002a"
            } else {
                "0000000000000001"
            },
            "version {version}"
        );
        let face = page.content(KEY_3);
        assert_eq!(face.label.as_deref(), Some("Mail"));
        // An old value stays an own value, and an absent key inherits and
        // gives the engine default of the old look: 20 pixels and bold.
        let want = if version >= 4 {
            LabelStyle {
                position: Some(Auto::Value(LabelPosition::Top)),
                size: Some(24),
                color: Some(Auto::Value(Color::rgb(0xFF, 0, 0))),
                bold: Some(false),
                ..LabelStyle::default()
            }
        } else {
            LabelStyle::default()
        };
        assert_eq!(face.label_style, want, "version {version}");
        let resolved = face.label_style.resolve(&LabelStyle::default());
        // The engine default of the size is 20 pixels.
        assert_eq!(
            resolved.size,
            if version >= 4 { 24 } else { 20 },
            "version {version}"
        );
        assert_eq!(resolved.bold, version < 4, "version {version}");
        if version < 4 {
            assert_eq!(resolved, ResolvedLabelStyle::default(), "version {version}");
        }
        assert_eq!(
            face.color,
            (version >= 4).then_some(Color::rgb(0x1E, 0x6F, 0xD9))
        );
        assert_eq!(
            page.binding(control, GestureKind::Press)
                .map(|action| action.kind.as_str()),
            (version >= 2).then_some("test.schema")
        );
        assert_eq!(profile.folders().len(), usize::from(version >= 5));
        assert_eq!(
            profile.applications.len(),
            usize::from(version >= 5),
            "version {version}"
        );
        // Version 7 has the replacement On face.
        assert_eq!(
            face.on.as_deref().and_then(|face| face.label.as_deref()),
            (version >= 7).then_some("Live")
        );
        // A save writes version 8, and that file loads with the same
        // content.
        store.save(&profile).unwrap();
        let written = rig.profile_text().unwrap();
        assert!(
            written.starts_with(&format!("schema_version = {SCHEMA_VERSION}\n")),
            "version {version}: {written}"
        );
        assert_eq!(store.load().unwrap().profile, profile, "version {version}");
    }
}
