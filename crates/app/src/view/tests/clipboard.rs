//! Copy, Paste, Clear, and Copy to All Pages of a control: the key
//! commands, the Edit menu, the "⋯" menu, and the right-click menu
//! (decision 0029, sections 8.3, 10.2, and 13). The engine does the work:
//! these tests check the commands that the window sends.

use super::*;
use crate::app_menu::{Clear, Copy, CopyToAllPages, Paste};
use crate::pasteboard::{ClipboardImage, SystemClipboard};
use gpui_kit::{ExternalPaths, FileDropEvent, MouseButton, MouseDownEvent, PlatformInput};
use opencraft_engine::Counters;

const KEY: ControlId = ControlId::new(ControlKind::Key, 2);
const KNOB: ControlId = ControlId::new(ControlKind::Knob, 2);

/// The clipboard commands in `sent`: all others are not of this test.
fn clipboard_commands(sent: &Commands) -> Vec<Command> {
    sent.borrow()
        .iter()
        .filter(|c| {
            matches!(
                c,
                Command::CopyControl { .. }
                    | Command::PasteControl { .. }
                    | Command::ClearControl { .. }
                    | Command::CopyToAllPages { .. }
            )
        })
        .cloned()
        .collect()
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

fn available(cx: &mut VisualTestContext, action: &dyn gpui_kit::Action) -> bool {
    cx.update(|window, cx| window.is_action_available(action, cx))
}

fn message(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| view.read(cx).status_message.as_ref().map(|m| m.to_string()))
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

fn right_click(cx: &mut VisualTestContext, id: &'static str) {
    let position = cx.debug_bounds(id).expect(id).center();
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Right,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
fn the_edit_menu_has_copy_paste_and_clear_after_the_history(cx: &mut TestAppContext) {
    let (_, cx, _, _) = window(cx);
    assert_eq!(
        super::history::edit_menu_items(cx),
        ["Undo", "Redo", "Copy", "Paste", "Clear", "Search…"]
    );
}

#[gpui_kit::test]
fn a_key_is_copied_and_pasted_with_the_keys(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    assert!(available(cx, &Copy));
    assert!(!available(cx, &Paste), "the clipboard is empty");
    keys(cx, "cmd-c");
    assert_eq!(
        clipboard_commands(&sent),
        [Command::CopyControl { control: KEY }]
    );
    assert_eq!(message(&view, cx).as_deref(), Some("Key 2 is copied"));

    // The engine tells the kind of the copy. Then Paste is possible.
    send(cx, &tx, Event::Clipboard(ControlKind::Key));
    assert!(available(cx, &Paste));
    keys(cx, "cmd-v");
    assert_eq!(
        clipboard_commands(&sent).last(),
        Some(&Command::PasteControl { control: KEY })
    );
}

#[gpui_kit::test]
fn a_paste_on_a_control_of_a_different_kind_is_not_possible(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, Event::Clipboard(ControlKind::Key));
    click(cx, "control-knob 2");
    assert!(!available(cx, &Paste), "the Edit menu item is disabled");
    keys(cx, "cmd-v");
    assert!(clipboard_commands(&sent).is_empty());
    assert_eq!(
        message(&view, cx).as_deref(),
        Some("You can paste a key only on a key")
    );
    // A knob copies to a knob.
    keys(cx, "cmd-c");
    assert_eq!(
        clipboard_commands(&sent),
        [Command::CopyControl { control: KNOB }]
    );
}

#[gpui_kit::test]
fn delete_clears_a_control_with_content(cx: &mut TestAppContext) {
    let (_, cx, _, sent) = window(cx);
    click(cx, "surface-key face 2");
    assert!(available(cx, &Clear), "the key has the label Mail");
    keys(cx, "backspace");
    assert_eq!(
        clipboard_commands(&sent),
        [Command::ClearControl { control: KEY }]
    );
    // The page has no clipboard commands.
    click(cx, "help-line");
    assert!(!available(cx, &Clear));
    assert!(!available(cx, &Copy));
}

/// Choose `item` in the open menu. The native menu of macOS sends its action
/// to the root element of the window. A different platform can show its
/// own menu with the focus in it: the test gives the focus back first.
fn choose(view: &Entity<DeviceView>, cx: &mut VisualTestContext, item: impl gpui_kit::Action) {
    cx.update(|window, cx| {
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx)
    });
    cx.run_until_parked();
    cx.dispatch_action(item);
    cx.run_until_parked();
}

#[gpui_kit::test]
fn copy_to_all_pages_and_the_menus_send_the_command_for_the_selection(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = window(cx);
    click(cx, "surface-key face 2");
    // The "⋯" button is enabled for a control. The native menu shows
    // nothing on the test platform: the test sends the action of the item.
    click(cx, "more-button");
    choose(&view, cx, CopyToAllPages);
    cx.run_until_parked();
    assert_eq!(
        clipboard_commands(&sent),
        [Command::CopyToAllPages { control: KEY }]
    );

    // A right click selects the control, and its menu is for it.
    right_click(cx, "control-knob 2");
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected_control, Some(KNOB)));
    choose(&view, cx, Copy);
    assert_eq!(
        clipboard_commands(&sent).last(),
        Some(&Command::CopyControl { control: KNOB })
    );
}

#[gpui_kit::test]
fn a_focused_text_field_keeps_its_copy_paste_and_delete(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = window(cx);
    cx.update(|window, _| window.activate_window());
    click(cx, "surface-key face 2");
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        window.focus(&input.focus_handle(cx), cx);
    });
    cx.run_until_parked();
    let value = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| view.read(cx).input.read(cx).value().to_string())
    };
    keys(cx, "backspace");
    assert_eq!(value(cx), "Mai");
    // The keys of the text field are those of macOS: ⌘A, ⌘C, and ⌘V.
    if cfg!(target_os = "macos") {
        keys(cx, "cmd-a cmd-c backspace");
        assert_eq!(value(cx), "");
        keys(cx, "cmd-v");
        assert_eq!(value(cx), "Mai");
    }
    // The Edit menu item goes to the field too.
    cx.update(|window, cx| window.dispatch_action(Box::new(Clear), cx));
    cx.run_until_parked();
    assert!(clipboard_commands(&sent).is_empty(), "{:?}", sent.borrow());
}

// MARK: An image from a file drop or a paste (decision 0043, section 5)

/// The bytes of an image of the format, for a pasteboard or a file.
fn image_bytes(format: image::ImageFormat) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(3, 2, image::Rgba([10, 20, 30, 255]));
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, format)
        .unwrap();
    cursor.into_inner()
}

/// The system pasteboard of the window, as the window reads it in the
/// application. A test has no pasteboard, so it sets the value.
fn set_system_clipboard(
    view: &Entity<DeviceView>,
    cx: &mut VisualTestContext,
    change_count: i64,
    image: Option<ClipboardImage>,
) {
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.state.system_clipboard = SystemClipboard {
                change_count,
                item: None,
                image,
            };
            cx.notify();
        });
    });
    cx.run_until_parked();
}

/// Drop the file `path` on the element `id`, as Finder does.
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
    // The control under the pointer shows the drop target.
    let target = Box::leak(format!("drop-{}", &id["surface-".len()..]).into_boxed_str());
    assert!(cx.debug_bounds(target).is_some(), "{target} does not show");
    cx.update(|window, cx| {
        window.dispatch_event(
            PlatformInput::FileDrop(FileDropEvent::Submit { position }),
            cx,
        );
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds(target).is_none(), "{target} stays");
}

/// The path of the last `SetImage` of the commands, and its surface.
fn last_image(sent: &Commands) -> Option<(SurfaceId, std::path::PathBuf)> {
    sent.borrow()
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::SetImage {
                surface,
                path: Some(path),
            } => Some((*surface, path.clone())),
            _ => None,
        })
}

#[gpui_kit::test]
fn a_pasted_image_goes_to_a_temporary_file_and_sets_the_image(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    let bytes = image_bytes(image::ImageFormat::Png);
    set_system_clipboard(
        &view,
        cx,
        4,
        Some(ClipboardImage::Bytes {
            bytes: bytes.clone(),
            extension: "png",
        }),
    );
    // No control is copied: the image of the system pasteboard is the
    // only paste.
    assert!(available(cx, &Paste));
    keys(cx, "cmd-v");
    let (target, path) = last_image(&sent).expect("the paste sets the image");
    assert_eq!(target, key());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(
        path.starts_with(std::env::temp_dir()),
        "{} is not in the temporary directory",
        path.display()
    );
    // A surface of an earlier change does not delete the file: the engine
    // reads it on its thread. The reply to the ReadCounters after the image
    // command proves the read (review 1, H3).
    send(cx, &tx, surface(key(), "Picture"));
    assert!(
        path.exists(),
        "{} went before the engine read it",
        path.display()
    );
    send(cx, &tx, Event::Counters(Counters::default()));
    assert!(!path.exists(), "{} stays", path.display());
}

#[gpui_kit::test]
fn a_paste_reads_the_pasteboard_even_when_the_last_frame_saw_no_image(cx: &mut TestAppContext) {
    let (view, cx, _, sent) = window(cx);
    click(cx, "surface-key face 2");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("picture.png");
    std::fs::write(&path, image_bytes(image::ImageFormat::Png)).unwrap();
    // A copy in another application does not draw a frame: the window has
    // the handlers of the last frame, where the pasteboard had no image.
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.state.system_clipboard = SystemClipboard {
                change_count: 7,
                item: None,
                image: Some(ClipboardImage::File(path.clone())),
            };
        });
    });
    keys(cx, "cmd-v");
    assert_eq!(last_image(&sent), Some((key(), path)));
}

#[gpui_kit::test]
fn a_pasted_image_a_webp_file_and_a_jpeg_file_set_the_image_of_a_bar_panel(
    cx: &mut TestAppContext,
) {
    let (view, cx, _, sent) = window(cx);
    click(cx, "surface-bar panel 3");
    let directory = tempfile::tempdir().unwrap();
    for (name, format) in [
        ("picture.webp", image::ImageFormat::WebP),
        ("picture.jpg", image::ImageFormat::Jpeg),
    ] {
        let path = directory.path().join(name);
        std::fs::write(&path, image_bytes(format)).unwrap();
        set_system_clipboard(&view, cx, 10, Some(ClipboardImage::File(path.clone())));
        assert!(available(cx, &Paste), "{name}");
        keys(cx, "cmd-v");
        assert_eq!(last_image(&sent), Some((bar(), path)), "{name}");
    }
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, Some(bar())));
}

#[gpui_kit::test]
fn the_newer_of_the_control_and_the_system_pasteboard_item_wins(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    // The copy of the control at change count 1.
    set_system_clipboard(&view, cx, 1, None);
    keys(cx, "cmd-c");
    send(cx, &tx, Event::Clipboard(ControlKind::Key));
    cx.update(|_, cx| assert_eq!(view.read(cx).state.control_copy_change_count, Some(1)));
    // The same count: the copy of the control is the newer item.
    keys(cx, "cmd-v");
    assert_eq!(
        clipboard_commands(&sent).last(),
        Some(&Command::PasteControl { control: KEY })
    );
    // An image copied after the control: the count is different.
    set_system_clipboard(
        &view,
        cx,
        2,
        Some(ClipboardImage::Bytes {
            bytes: image_bytes(image::ImageFormat::Bmp),
            extension: "bmp",
        }),
    );
    keys(cx, "cmd-v");
    let (target, path) = last_image(&sent).expect("the paste sets the image");
    assert_eq!(target, key());
    assert_eq!(&std::fs::read(&path).unwrap()[..2], b"BM");
    // The engine reads the file and sends the surface: the file goes away.
    send(cx, &tx, Event::Counters(Counters::default()));
    assert!(!path.exists(), "{} stays", path.display());
}

#[gpui_kit::test]
fn a_repeated_paste_of_one_image_removes_each_temporary_file(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    let bytes = image_bytes(image::ImageFormat::Png);
    let mut paths = Vec::new();
    for count in [4, 5] {
        set_system_clipboard(
            &view,
            cx,
            count,
            Some(ClipboardImage::Bytes {
                bytes: bytes.clone(),
                extension: "png",
            }),
        );
        keys(cx, "cmd-v");
        paths.push(last_image(&sent).expect("the paste sets the image").1);
    }
    assert_ne!(paths[0], paths[1]);
    assert!(paths.iter().all(|path| path.exists()));
    // One Counters reply for each ReadCounters of the shell: the files go
    // in the order of the commands, also when the engine sends no surface
    // because the image did not change (review 1, H3).
    send(cx, &tx, Event::Counters(Counters::default()));
    assert!(!paths[0].exists(), "{} stays", paths[0].display());
    assert!(paths[1].exists(), "{} went too early", paths[1].display());
    send(cx, &tx, Event::Counters(Counters::default()));
    assert!(!paths[1].exists(), "{} stays", paths[1].display());
}

#[gpui_kit::test]
fn a_closed_window_does_not_delete_a_temporary_image_before_the_engine_reads_it(
    cx: &mut TestAppContext,
) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    set_system_clipboard(
        &view,
        cx,
        4,
        Some(ClipboardImage::Bytes {
            bytes: image_bytes(image::ImageFormat::Png),
            extension: "png",
        }),
    );
    keys(cx, "cmd-v");
    let path = last_image(&sent).expect("the paste sets the image").1;
    cx.update(|_, cx| {
        for window in cx.windows() {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
    });
    cx.run_until_parked();
    // The shell owns the file, not the window (review 1, H3).
    assert!(path.exists(), "the window deleted {}", path.display());
    send(cx, &tx, Event::Counters(Counters::default()));
    assert!(!path.exists(), "{} stays", path.display());
}

#[gpui_kit::test]
fn a_file_that_is_not_an_image_does_not_hide_a_copied_control(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    // The copy of the control at change count 1.
    set_system_clipboard(&view, cx, 1, None);
    keys(cx, "cmd-c");
    send(cx, &tx, Event::Clipboard(ControlKind::Key));
    // Finder copies a text file. The system pasteboard has a newer item,
    // but it is not an image or an application bundle: it must not hide the
    // copy of the control (review 1, H2).
    set_system_clipboard(&view, cx, 7, None);
    keys(cx, "cmd-v");
    assert_eq!(
        clipboard_commands(&sent).last(),
        Some(&Command::PasteControl { control: KEY })
    );
}

#[gpui_kit::test]
fn a_focused_text_field_keeps_its_paste_when_the_system_pasteboard_has_an_image(
    cx: &mut TestAppContext,
) {
    let (view, cx, _, sent) = window(cx);
    cx.update(|window, _| window.activate_window());
    click(cx, "surface-key face 2");
    cx.update(|window, cx| {
        let input = view.read(cx).input.clone();
        window.focus(&input.focus_handle(cx), cx);
    });
    cx.run_until_parked();
    set_system_clipboard(
        &view,
        cx,
        6,
        Some(ClipboardImage::Bytes {
            bytes: image_bytes(image::ImageFormat::Png),
            extension: "png",
        }),
    );
    keys(cx, "cmd-v");
    assert!(last_image(&sent).is_none(), "the field keeps the paste");
    assert!(clipboard_commands(&sent).is_empty());
}

#[gpui_kit::test]
fn a_file_dropped_on_a_key_face_or_a_bar_panel_sets_the_image_as_one_step(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    let directory = tempfile::tempdir().unwrap();
    let key_file = directory.path().join("key.png");
    std::fs::write(&key_file, image_bytes(image::ImageFormat::Png)).unwrap();
    let panel_file = directory.path().join("panel.webp");
    std::fs::write(&panel_file, image_bytes(image::ImageFormat::WebP)).unwrap();

    drop_file(cx, "surface-key face 2", key_file.clone());
    assert_eq!(last_image(&sent), Some((key(), key_file.clone())));
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, Some(key())));
    send(cx, &tx, surface(key(), "Picture"));

    // The drop of a bar panel selects it, and one drop is one command.
    let before = sent.borrow().len();
    drop_file(cx, "surface-bar panel 3", panel_file.clone());
    assert_eq!(sent.borrow().len(), before + 1, "{:?}", sent.borrow());
    assert_eq!(last_image(&sent), Some((bar(), panel_file.clone())));
    cx.update(|_, cx| assert_eq!(view.read(cx).state.selected, Some(bar())));

    // One step of the history: the undo command is there after the event
    // of the engine, and ⌘Z sends it.
    send(
        cx,
        &tx,
        Event::History {
            undo: Some("Set Image".into()),
            redo: None,
        },
    );
    assert!(available(cx, &crate::app_menu::Undo));
    keys(cx, "cmd-z");
    assert_eq!(sent.borrow().last(), Some(&Command::Undo));
}

#[gpui_kit::test]
fn a_file_dropped_on_a_key_face_that_is_not_an_image_goes_to_the_engine(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("notes.txt");
    std::fs::write(&file, "this is not an image").unwrap();
    drop_file(cx, "surface-key face 2", file.clone());
    // The engine checks the file (rule 9). It refuses it with the usual
    // problem that names the file.
    assert_eq!(last_image(&sent), Some((key(), file.clone())));
    let text = format!(
        "OpenCraft could not use the image: cannot use the image {}: the image format is not supported",
        file.display()
    );
    send(cx, &tx, Event::Problem(text.clone()));
    cx.update(|_, cx| assert_eq!(view.read(cx).state.problems, [text]));
}
