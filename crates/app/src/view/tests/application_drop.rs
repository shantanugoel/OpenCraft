//! A drop of an application bundle from Finder onto a key face (KEY-13),
//! and the one step of the history that it makes (decision 0030).
//!
//! The window sends the commands to the engine. Thus these tests check the
//! commands that the window sends; the engine checks the image file itself.

use super::*;
use crate::pasteboard::{ClipboardImage, SystemClipboard};
use crate::view::application_drop::DROP_APPLICATION_STEP;
use gpui_kit::{ExternalPaths, FileDropEvent, PlatformInput};
use opencraft_engine::InputType;

/// The kind that the engine describes for an application, as the real
/// description of "Open an application" does: a path field of kind
/// Application, with the setting of the bundle identifier.
fn application_kind() -> KindDescription {
    KindDescription {
        kind: "test.open".into(),
        name: "Open an application".into(),
        fields: vec![
            Field {
                key: "application".into(),
                label: "Application, file or folder".into(),
                input: InputType::Path {
                    kind: PathKind::Application {
                        bundle_id: "bundle_id".into(),
                    },
                },
                default: None,
            },
            Field {
                key: "arguments".into(),
                label: "Arguments".into(),
                input: InputType::Text,
                default: None,
            },
        ],
        ..Default::default()
    }
}

/// An application bundle in `root`, with the name `Demo` and the bundle
/// identifier `com.example.demo`.
fn write_bundle(root: &std::path::Path) -> std::path::PathBuf {
    let bundle = root.join("Demo.app");
    let contents = bundle.join("Contents");
    std::fs::create_dir_all(&contents).unwrap();
    std::fs::write(
        contents.join("Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleDisplayName</key><string>Demo</string>
<key>CFBundleIdentifier</key><string>com.example.demo</string>
</dict></plist>"#,
    )
    .unwrap();
    bundle
}

/// Drop `path` on the control with the element `id`.
fn drop_file(cx: &mut VisualTestContext, id: &'static str, path: std::path::PathBuf) {
    let position = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"))
        .center();
    let paths = ExternalPaths(vec![path].into_iter().collect());
    cx.update(|window, cx| {
        window.dispatch_event(
            PlatformInput::FileDrop(FileDropEvent::Entered {
                position,
                paths: paths.clone(),
            }),
            cx,
        );
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.dispatch_event(
            PlatformInput::FileDrop(FileDropEvent::Submit { position }),
            cx,
        );
    });
    cx.run_until_parked();
}

/// A window with the kind that opens an application.
fn window_with_kind(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    tx.try_send(Event::Kinds(vec![application_kind()])).unwrap();
    cx.run_until_parked();
    (view, cx, tx, sent)
}

#[gpui_kit::test]
fn an_application_dropped_on_a_key_face_sets_the_action_the_image_and_the_title(
    cx: &mut TestAppContext,
) {
    let (view, cx, tx, sent) = window_with_kind(cx);
    let directory = tempfile::tempdir().unwrap();
    let bundle = write_bundle(directory.path());
    drop_file(cx, "surface-key face 2", bundle.clone());

    let group = sent
        .borrow()
        .iter()
        .find(|command| matches!(command, Command::Group { .. }))
        .cloned()
        .expect("the drop is one step");
    let Command::Group { name, commands } = group else {
        unreachable!()
    };
    assert_eq!(name, DROP_APPLICATION_STEP);
    let binding = BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Key, 2),
        gesture: GestureKind::Press,
        folder: None,
    };
    let mut settings = Settings::new();
    settings.insert(
        "application".into(),
        Value::Text(bundle.to_string_lossy().into_owned()),
    );
    settings.insert("bundle_id".into(), Value::Text("com.example.demo".into()));
    assert_eq!(
        commands[0],
        Command::SetBinding {
            binding,
            action: Some(Action {
                kind: "test.open".into(),
                settings,
            }),
        }
    );
    assert_eq!(
        commands[1],
        Command::SetLabel {
            surface: key(),
            label: Some("Demo".into()),
        }
    );
    #[cfg(target_os = "macos")]
    {
        // The icon is a 256-pixel PNG in a temporary file that the engine
        // reads, as a paste sets an image.
        let Command::SetImage {
            surface: face,
            path: Some(path),
        } = &commands[2]
        else {
            panic!("the drop sets the image: {commands:?}");
        };
        assert_eq!(*face, key());
        assert!(path.starts_with(std::env::temp_dir()));
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let icon = image::load_from_memory(&bytes).unwrap();
        assert_eq!((icon.width(), icon.height()), (256, 256));
        // The engine read the file: the window removes it.
        tx.try_send(surface(key(), "Demo")).unwrap();
        cx.run_until_parked();
        assert!(!path.exists(), "{} stays", path.display());
    }
    #[cfg(not(target_os = "macos"))]
    assert_eq!(commands.len(), 2, "{commands:?}");

    cx.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.state.selected, Some(key()));
        assert_eq!(
            view.status_message.as_deref(),
            Some("Key 2 gets “Open an application” for Press")
        );
    });
    // One undo step: the engine sends the name, and ⌘Z sends Undo.
    tx.try_send(Event::History {
        undo: Some(DROP_APPLICATION_STEP.into()),
        redo: None,
    })
    .unwrap();
    cx.run_until_parked();
    cx.simulate_keystrokes("cmd-z");
    cx.run_until_parked();
    assert_eq!(sent.borrow().last(), Some(&Command::Undo));
}

#[gpui_kit::test]
fn an_application_dropped_on_a_bar_panel_sets_the_action_and_the_title(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = window_with_kind(cx);
    let directory = tempfile::tempdir().unwrap();
    let bundle = write_bundle(directory.path());
    drop_file(cx, "surface-bar panel 3", bundle);
    let group = sent
        .borrow()
        .iter()
        .find(|command| matches!(command, Command::Group { .. }))
        .cloned()
        .expect("the drop is one step");
    let Command::Group { name, commands } = group else {
        unreachable!()
    };
    assert_eq!(name, DROP_APPLICATION_STEP);
    assert!(
        matches!(
            &commands[0],
            Command::SetBinding { binding, action: Some(action) }
                if binding.control == ControlId::new(ControlKind::BarPanel, 2)
                    && binding.gesture == GestureKind::Tap
                    && action.kind == "test.open"
        ),
        "{commands:#?}"
    );
    assert_eq!(
        commands[1],
        Command::SetLabel {
            surface: bar(),
            label: Some("Demo".into()),
        }
    );
}

#[gpui_kit::test]
fn a_pasted_application_bundle_sets_the_action_and_the_title(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = window_with_kind(cx);
    click(cx, "surface-key face 2");
    let directory = tempfile::tempdir().unwrap();
    let bundle = write_bundle(directory.path());
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.state.system_clipboard = SystemClipboard {
                change_count: 9,
                image: Some(ClipboardImage::File(bundle.clone())),
            };
        });
    });
    cx.simulate_keystrokes("cmd-v");
    cx.run_until_parked();
    let group = sent
        .borrow()
        .iter()
        .find(|command| matches!(command, Command::Group { .. }))
        .cloned()
        .expect("the paste is one step");
    let Command::Group { name, commands } = group else {
        unreachable!()
    };
    assert_eq!(name, DROP_APPLICATION_STEP);
    assert!(
        matches!(
            &commands[0],
            Command::SetBinding { action: Some(action), .. }
                if action.settings.get("application")
                    == Some(&Value::Text(bundle.to_string_lossy().into_owned()))
                    && action.settings.get("bundle_id")
                        == Some(&Value::Text("com.example.demo".into()))
        ),
        "{commands:#?}"
    );
    assert_eq!(
        commands[1],
        Command::SetLabel {
            surface: key(),
            label: Some("Demo".into()),
        }
    );
}

#[gpui_kit::test]
fn a_drop_of_a_folder_or_a_file_is_still_an_image(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = window_with_kind(cx);
    let directory = tempfile::tempdir().unwrap();
    // A folder with a name that does not end with `.app`, and a file with
    // that name (not a bundle): both go to the image command.
    let folder = directory.path().join("Pictures");
    std::fs::create_dir_all(&folder).unwrap();
    let file = directory.path().join("Notes.app");
    std::fs::write(&file, "not a bundle").unwrap();
    for path in [folder, file] {
        sent.borrow_mut().clear();
        drop_file(cx, "surface-key face 2", path.clone());
        assert_eq!(
            sent.borrow().last(),
            Some(&Command::SetImage {
                surface: key(),
                path: Some(path),
            })
        );
    }
}
