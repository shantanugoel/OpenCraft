//! The Settings window: each switch, the permission state, the tabs, the
//! key commands, and the buttons (decision 0029, section 17). The tests open
//! no application, and read and write no file outside a temporary folder.

use super::{
    about::{no_notices_text, VERSION},
    general::permission_texts,
    shortcuts, *,
};
use crate::{
    app_menu::{self, OpenAbout, OpenSettings},
    settings::{self, Settings},
    shell::{ACCESSIBILITY_PANE, NOTICES_FILE},
    view::theme,
};
use gpui_kit::{
    component::{Theme, ThemeMode},
    Action, Modifiers, OwnedMenuItem, TestAppContext, VisualTestContext,
};
use opencraft_engine::{Command, Event, KindDescription};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

type Sent = Rc<RefCell<Vec<Command>>>;

gpui_kit::actions!(test, [Unbound]);

thread_local! {
    /// What the fake desktop was asked to show: a path, or an address.
    static SHOWN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static OPENED: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
}

fn show_in_finder(path: &Path) -> bool {
    SHOWN.with(|shown| shown.borrow_mut().push(path.display().to_string()));
    true
}

fn open_url(url: &str) -> bool {
    SHOWN.with(|shown| shown.borrow_mut().push(url.to_string()));
    true
}

fn open_file(path: &Path) -> bool {
    OPENED.with(|opened| opened.borrow_mut().push(path.to_path_buf()));
    true
}

fn shown() -> Vec<String> {
    SHOWN.with(|shown| shown.borrow().clone())
}

struct Fixture {
    shell: Entity<Shell>,
    view: Entity<SettingsView>,
    sent: Sent,
    /// The folder of `shell.toml`. It goes away with the fixture.
    dir: tempfile::TempDir,
}

impl Fixture {
    fn settings_on_disk(&self) -> Settings {
        settings::load(self.dir.path()).0
    }
}

/// The Settings window with a shell that has fake desktop functions, and the
/// folder of `shell.toml` in a temporary folder.
fn window(cx: &mut TestAppContext) -> (Fixture, &mut VisualTestContext) {
    SHOWN.with(|shown| shown.borrow_mut().clear());
    OPENED.with(|opened| opened.borrow_mut().clear());
    cx.update(|cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        app_menu::install(cx, &settings::Panes::default());
    });
    let dir = tempfile::tempdir().unwrap();
    let sent: Sent = Default::default();
    let log = sent.clone();
    let config = dir.path().to_path_buf();
    let shell = cx.update(|cx| {
        cx.new(|cx| {
            let mut shell = Shell::new(
                config.clone(),
                Settings::default(),
                Rc::new(move |command| log.borrow_mut().push(command)),
            );
            shell.set_profiles_dir(config.join("profiles"));
            shell.set_desktop(show_in_finder, open_url);
            // The tests never read or write `~/Library/LaunchAgents`.
            #[cfg(target_os = "macos")]
            shell.set_login_dir(config.join("LaunchAgents"));
            shell.listen(
                async_channel::unbounded().1,
                async_channel::unbounded().1,
                cx,
            );
            shell
        })
    });
    let snapshot = cx.update(|cx| shell.read(cx).settings_snapshot());
    let mut view = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let settings_view = cx.new(|cx| SettingsView::new(&shell, snapshot.clone(), window, cx));
        view = Some(settings_view.clone());
        gpui_kit::base::Root::new(settings_view, window, cx)
    });
    // The test text has its own metrics, thus the test window is tall: each
    // element is inside the window. The heights of the real window are
    // checked in the real window.
    cx.simulate_resize(gpui_kit::size(px(theme::SETTINGS_WIDTH), px(800.)));
    cx.run_until_parked();
    (
        Fixture {
            shell,
            view: view.unwrap(),
            sent,
            dir,
        },
        cx,
    )
}

fn bounds(cx: &mut VisualTestContext, id: &str) -> Option<gpui_kit::Bounds<gpui_kit::Pixels>> {
    // The test names live until the end of the test run, and are few.
    cx.debug_bounds(Box::leak(id.to_string().into_boxed_str()))
}

fn click(cx: &mut VisualTestContext, id: &str) {
    let bounds = bounds(cx, id).unwrap_or_else(|| panic!("No element: {id}"));
    cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
    cx.simulate_click(bounds.center(), Modifiers::default());
    cx.run_until_parked();
}

/// True if the switch `id` shows the state "on": its knob is in the end half
/// of the track.
fn is_on(cx: &mut VisualTestContext, id: &str) -> bool {
    let track = bounds(cx, id).unwrap_or_else(|| panic!("No switch: {id}"));
    let knob = bounds(cx, &format!("{id}-knob")).unwrap_or_else(|| panic!("No knob: {id}"));
    knob.center().x > track.center().x
}

fn kind(id: &str, name: &str) -> KindDescription {
    KindDescription {
        kind: id.into(),
        name: name.into(),
        fields: Vec::new(),
    }
}

/// The engine events of the three kinds of the keys: they wait for the
/// permission, or they are ready.
fn key_kinds(ready: bool) -> Vec<Event> {
    let status = |kind: &str| Event::Status {
        kind: kind.into(),
        status: if ready {
            opencraft_engine::Status::Ready
        } else {
            opencraft_engine::Status::NeedsUser {
                text: "Needs the permission.".into(),
                button: "Give the permission".into(),
            }
        },
    };
    vec![
        status("test.hotkey"),
        status("test.text"),
        status("test.media"),
    ]
}

fn send_kinds(f: &Fixture, cx: &mut VisualTestContext, ready: bool) {
    let mut events = vec![Event::Kinds(vec![
        kind("test.hotkey", "Send a hotkey"),
        kind("test.open", "Open an application"),
        kind("test.text", "Type text"),
        kind("test.media", "Media key"),
    ])];
    events.extend(key_kinds(ready));
    for event in events {
        f.shell.update(cx, |shell, cx| shell.receive(event, cx));
    }
    cx.run_until_parked();
}

// MARK: The permission

#[test]
fn the_names_are_joined_with_commas_and_and() {
    let names = |list: &[&str]| list.iter().map(|n| n.to_string()).collect::<Vec<_>>();
    assert_eq!(names_text(&names(&[])), "");
    assert_eq!(names_text(&names(&["Type text"])), "Type text");
    assert_eq!(names_text(&names(&["A", "B"])), "A and B");
    assert_eq!(
        names_text(&names(&["Send a hotkey", "Type text", "Media key"])),
        "Send a hotkey, Type text and Media key"
    );
}

#[test]
fn the_permission_texts_follow_the_statuses_of_the_engine() {
    let mut state = ViewState {
        kinds: vec![
            kind("test.hotkey", "Send a hotkey"),
            kind("test.open", "Open an application"),
            kind("test.text", "Type text"),
            kind("test.media", "Media key"),
        ],
        ..ViewState::default()
    };
    // Before the engine sends a status, no kind waits for the permission.
    assert_eq!(
        permission_texts(&permission(&state)),
        ("given", String::new())
    );
    assert!(kinds_waiting_for_permission(&state).is_empty());

    for event in key_kinds(false) {
        state.apply(event, std::time::Instant::now());
    }
    // The kind with no status does not need the permission.
    assert_eq!(
        permission_texts(&permission(&state)),
        (
            "not given",
            "Necessary for Send a hotkey, Type text and Media key".into()
        )
    );
    assert_eq!(
        kinds_waiting_for_permission(&state),
        ["test.hotkey", "test.media", "test.text"]
    );

    // One kind that waits is enough for "not given".
    for event in key_kinds(true) {
        state.apply(event, std::time::Instant::now());
    }
    state.apply(
        Event::Status {
            kind: "test.text".into(),
            status: opencraft_engine::Status::NeedsUser {
                text: String::new(),
                button: String::new(),
            },
        },
        std::time::Instant::now(),
    );
    assert!(!permission(&state).given);
    assert_eq!(kinds_waiting_for_permission(&state), ["test.text"]);

    // The permission is given: the names stay.
    for event in key_kinds(true) {
        state.apply(event, std::time::Instant::now());
    }
    assert_eq!(
        permission_texts(&permission(&state)),
        (
            "given",
            "Necessary for Send a hotkey, Type text and Media key".into()
        )
    );
}

#[gpui_kit::test]
fn the_window_shows_the_permission_state_and_opens_system_settings(cx: &mut TestAppContext) {
    let (f, cx) = window(cx);
    assert!(bounds(cx, "permission-state").is_some());
    send_kinds(&f, cx, false);
    cx.update(|_, cx| {
        let shell = f.shell.read(cx);
        assert!(!permission(&shell.state).given);
        assert_eq!(
            kinds_waiting_for_permission(&shell.state),
            ["test.hotkey", "test.media", "test.text"]
        );
    });
    // The button opens the pane of System Settings, and asks the engine to
    // wait for the permission, so that the window follows the change.
    click(cx, "open-system-settings");
    assert_eq!(shown(), [ACCESSIBILITY_PANE]);
    let mut resolved: Vec<_> = f
        .sent
        .borrow()
        .iter()
        .map(|command| match command {
            Command::Resolve { kind } => kind.clone(),
            other => panic!("Unexpected command: {other:?}"),
        })
        .collect();
    resolved.sort();
    assert_eq!(resolved, ["test.hotkey", "test.media", "test.text"]);
    // The user gives the permission in System Settings: the engine sends the
    // new statuses. The window shows "given", and the button stays.
    f.sent.borrow_mut().clear();
    send_kinds(&f, cx, true);
    cx.update(|_, cx| assert!(permission(&f.shell.read(cx).state).given));
    click(cx, "open-system-settings");
    assert_eq!(shown().len(), 2);
    assert!(
        f.sent.borrow().is_empty(),
        "Nothing waits for the permission"
    );
}

// MARK: The switches

#[gpui_kit::test]
fn the_dock_icon_switch_follows_and_keeps_the_option(cx: &mut TestAppContext) {
    let (f, cx) = window(cx);
    assert!(is_on(cx, "switch-dock-icon"));
    click(cx, "switch-dock-icon");
    assert!(!is_on(cx, "switch-dock-icon"));
    cx.update(|_, cx| assert!(!f.shell.read(cx).settings().show_dock_icon));
    assert!(!f.settings_on_disk().show_dock_icon);
    click(cx, "switch-dock-icon");
    assert!(is_on(cx, "switch-dock-icon"));
    assert!(f.settings_on_disk().show_dock_icon);
}

#[gpui_kit::test]
fn the_switch_for_the_window_at_start_follows_and_keeps_the_option(cx: &mut TestAppContext) {
    let (f, cx) = window(cx);
    assert!(is_on(cx, "switch-window-at-start"));
    click(cx, "switch-window-at-start");
    assert!(!is_on(cx, "switch-window-at-start"));
    cx.update(|_, cx| assert!(!f.shell.read(cx).settings().show_window_at_start));
    assert!(!f.settings_on_disk().show_window_at_start);
    // The other switches do not change.
    assert!(is_on(cx, "switch-dock-icon"));
    assert!(!is_on(cx, "switch-start-at-login"));
    click(cx, "switch-window-at-start");
    assert!(is_on(cx, "switch-window-at-start"));
    assert!(f.settings_on_disk().show_window_at_start);
}

#[cfg(target_os = "macos")]
#[gpui_kit::test]
fn the_start_at_login_switch_writes_and_removes_the_launch_agent_file(cx: &mut TestAppContext) {
    use crate::login;
    let (f, cx) = window(cx);
    let file = f.dir.path().join("LaunchAgents").join(login::FILE_NAME);
    assert!(!is_on(cx, "switch-start-at-login"));
    assert!(!file.exists());
    click(cx, "switch-start-at-login");
    assert!(is_on(cx, "switch-start-at-login"));
    assert!(file.exists());
    click(cx, "switch-start-at-login");
    assert!(!is_on(cx, "switch-start-at-login"));
    assert!(!file.exists());
    assert!(bounds(cx, "settings-message").is_none());
}

#[cfg(target_os = "macos")]
#[gpui_kit::test]
fn a_start_at_login_that_cannot_change_shows_a_message_in_the_window(cx: &mut TestAppContext) {
    let (f, cx) = window(cx);
    // A file in place of the directory: the directory cannot be made.
    let blocked = f.dir.path().join("file");
    std::fs::write(&blocked, "").unwrap();
    f.shell.update(cx, |shell, _| {
        shell.set_login_dir(blocked.join("LaunchAgents"))
    });
    click(cx, "switch-start-at-login");
    assert!(!is_on(cx, "switch-start-at-login"));
    assert!(bounds(cx, "settings-message").is_some());
    cx.update(|_, cx| {
        let shell = f.shell.read(cx);
        assert!(shell
            .settings_notice()
            .unwrap()
            .contains("cannot change the start at login"));
        // The main window and the log get the same problem.
        assert_eq!(shell.state.problems.len(), 1);
    });
    click(cx, "settings-message-close");
    assert!(bounds(cx, "settings-message").is_none());
    cx.update(|_, cx| assert_eq!(f.shell.read(cx).settings_notice(), None));
}

// MARK: The tabs

#[gpui_kit::test]
fn the_tabs_are_general_shortcuts_and_about_and_keep_the_last_pane(cx: &mut TestAppContext) {
    let (f, cx) = window(cx);
    for tab in ["general", "shortcuts", "about"] {
        assert!(
            bounds(cx, &format!("settings-tab-{tab}")).is_some(),
            "{tab}"
        );
    }
    // "Plug-ins" and "Updates" are places for later: the window has no tab.
    assert!(bounds(cx, "settings-tab-plug-ins").is_none());
    assert!(bounds(cx, "settings-tab-updates").is_none());
    assert!(bounds(cx, "switch-dock-icon").is_some());
    assert!(bounds(cx, "shortcuts").is_none());

    click(cx, "settings-tab-shortcuts");
    assert!(bounds(cx, "shortcuts").is_some());
    assert!(bounds(cx, "switch-dock-icon").is_none());
    assert_eq!(f.settings_on_disk().settings_pane, SettingsPane::Shortcuts);

    click(cx, "settings-tab-about");
    assert!(bounds(cx, "about").is_some());
    assert_eq!(f.settings_on_disk().settings_pane, SettingsPane::About);

    click(cx, "settings-tab-general");
    assert!(bounds(cx, "switch-dock-icon").is_some());
    assert_eq!(f.settings_on_disk().settings_pane, SettingsPane::General);
    cx.update(|_, cx| assert_eq!(f.view.read(cx).pane(), SettingsPane::General));
}

#[test]
fn each_pane_has_a_height_and_the_window_is_wide_enough_for_its_content() {
    for pane in SettingsPane::ALL {
        let height = pane_height(pane, 4, true);
        // The title bar of 34, the tab row of 50 and its padding, and the
        // padding of the content, at the least.
        assert!(height > 34. + 50. + 8. + 36., "{pane:?}");
    }
    assert_eq!(theme::SETTINGS_WIDTH, 640.);
}

// MARK: The shortcuts

#[gpui_kit::test]
fn the_shortcuts_list_the_key_commands_that_have_a_key_binding(cx: &mut TestAppContext) {
    let (_f, cx) = window(cx);
    let rows = cx.update(|_, cx| shortcuts::rows(cx));
    assert_eq!(
        rows,
        [
            ("Undo", "⌘Z".to_string()),
            ("Redo", "⇧⌘Z".to_string()),
            ("Hide or show the sidebar", "⌃⌘S".to_string()),
            ("Hide or show the inspector", "⌥⌘I".to_string()),
            ("Settings", "⌘,".to_string()),
            ("Quit OpenCraft", "⌘Q".to_string()),
        ]
    );
    // A command with no key binding has no row.
    cx.update(|_, cx| {
        assert_eq!(
            shortcuts::keys(&app_menu::CloseWindow, cx),
            Some("⌘W".into())
        );
        assert_eq!(shortcuts::keys(&Unbound, cx), None);
    });
}

#[gpui_kit::test]
fn the_shortcuts_pane_shows_one_row_for_each_command(cx: &mut TestAppContext) {
    let (_f, cx) = window(cx);
    click(cx, "settings-tab-shortcuts");
    assert!(bounds(cx, "shortcuts").is_some());
    let group = bounds(cx, "shortcuts").unwrap();
    // Rows of one line, with their padding and lines: each row is
    // 10 + the line + 10 high.
    let rows = cx.update(|_, cx| shortcuts::rows(cx)).len() as f32;
    let row = f32::from(group.size.height) / rows;
    assert!((34.0..=40.0).contains(&row), "A row is {row} high");
}

// MARK: The buttons

#[gpui_kit::test]
fn show_in_finder_shows_the_profiles_folder_and_the_log_file(cx: &mut TestAppContext) {
    let (f, cx) = window(cx);
    let profiles = f.dir.path().join("profiles");
    click(cx, "show-profiles");
    assert_eq!(shown(), [profiles.display().to_string()]);
    // No log file yet (the application started with a terminal): the
    // button shows the folder.
    click(cx, "show-log-file");
    assert_eq!(shown()[1], f.dir.path().display().to_string());
    // With a log file, the button shows the file.
    let log = f.dir.path().join("opencraft.log");
    std::fs::write(&log, "A line\n").unwrap();
    click(cx, "show-log-file");
    assert_eq!(shown()[2], log.display().to_string());
    cx.update(|_, cx| {
        let shell = f.shell.read(cx);
        assert_eq!(shell.log_file(), log);
        assert_eq!(shell.profiles_dir(), profiles);
        assert!(shell.state.problems.is_empty());
    });
}

#[gpui_kit::test]
fn a_finder_that_does_not_open_shows_a_message(cx: &mut TestAppContext) {
    let (f, cx) = window(cx);
    f.shell
        .update(cx, |shell, _| shell.set_desktop(|_| false, |_| false));
    click(cx, "show-profiles");
    assert!(bounds(cx, "settings-message").is_some());
    cx.update(|_, cx| {
        assert!(f
            .shell
            .read(cx)
            .settings_notice()
            .unwrap()
            .contains("cannot show"));
    });
    click(cx, "open-system-settings");
    cx.update(|_, cx| {
        assert!(f
            .shell
            .read(cx)
            .settings_notice()
            .unwrap()
            .contains("cannot open System Settings"));
    });
}

#[test]
fn a_path_in_the_home_folder_shows_with_a_tilde() {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap();
    assert_eq!(tilde(&home), "~");
    assert_eq!(
        tilde(&home.join("Library/Application Support/OpenCraft/profiles")),
        "~/Library/Application Support/OpenCraft/profiles"
    );
    assert_eq!(tilde(Path::new("/opt/profiles")), "/opt/profiles");
}

// MARK: About

#[gpui_kit::test]
fn the_about_pane_shows_the_version_and_opens_the_licence_notices(cx: &mut TestAppContext) {
    let (f, cx) = window(cx);
    click(cx, "settings-tab-about");
    assert!(bounds(cx, "about-version").is_some());
    assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    // A copy that is not in a bundle has no notices file: the button does
    // nothing, and the pane says where the notices are.
    assert!(bounds(cx, "no-notices").is_some());
    click(cx, "licence-notices");
    assert!(OPENED.with(|opened| opened.borrow().is_empty()));
    assert!(no_notices_text().contains(NOTICES_FILE));

    let file = f.dir.path().join(NOTICES_FILE);
    std::fs::write(&file, "Notices").unwrap();
    f.shell.update(cx, |shell, cx| {
        shell.set_notices(Some(file.clone()), open_file);
        cx.notify();
    });
    cx.run_until_parked();
    assert!(bounds(cx, "no-notices").is_none());
    click(cx, "licence-notices");
    assert_eq!(OPENED.with(|opened| opened.borrow().clone()), [file]);
}

// MARK: The window of the shell

fn windows(cx: &mut TestAppContext) -> usize {
    cx.update(|cx| cx.windows().len())
}

#[gpui_kit::test]
fn the_shell_opens_one_settings_window_and_brings_it_to_the_front(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    cx.update(|cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        app_menu::install(cx, &settings::Panes::default());
    });
    let config = dir.path().to_path_buf();
    let shell = cx.update(|cx| {
        cx.new(|cx| {
            let mut shell = Shell::new(config, Settings::default(), Rc::new(|_| {}));
            shell.listen(
                async_channel::unbounded().1,
                async_channel::unbounded().1,
                cx,
            );
            shell
        })
    });
    assert_eq!(windows(cx), 0);
    shell.update(cx, |shell, cx| shell.open_settings(None, cx));
    cx.run_until_parked();
    assert_eq!(windows(cx), 1);
    // A second "Settings…" brings the same window to the front.
    shell.update(cx, |shell, cx| shell.open_settings(None, cx));
    cx.run_until_parked();
    assert_eq!(windows(cx), 1);
    // The main window is a different window, and "Open OpenCraft" does not
    // bring the Settings window.
    shell.update(cx, |shell, cx| shell.open_window(cx));
    cx.run_until_parked();
    assert_eq!(windows(cx), 2);
    shell.update(cx, |shell, cx| shell.open_window(cx));
    shell.update(cx, |shell, cx| shell.open_settings(None, cx));
    cx.run_until_parked();
    assert_eq!(windows(cx), 2);

    // Close the Settings window: the next command makes a new one.
    let settings = cx.update(|cx| {
        cx.windows()
            .into_iter()
            .find(|w| {
                w.downcast::<gpui_kit::base::Root>().is_some_and(|root| {
                    root.read(cx)
                        .is_ok_and(|root| root.view().clone().downcast::<SettingsView>().is_ok())
                })
            })
            .expect("a Settings window")
    });
    cx.update(|cx| {
        settings
            .update(cx, |_, window, _| window.remove_window())
            .unwrap()
    });
    cx.run_until_parked();
    assert_eq!(windows(cx), 1);
    shell.update(cx, |shell, cx| shell.open_settings(None, cx));
    cx.run_until_parked();
    assert_eq!(windows(cx), 2);
}

#[gpui_kit::test]
fn the_menu_and_the_key_command_open_the_window_at_the_right_pane(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    cx.update(|cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        app_menu::install(cx, &settings::Panes::default());
    });
    let config = dir.path().to_path_buf();
    let shell = cx.update(|cx| {
        cx.new(|cx| {
            let mut shell = Shell::new(config, Settings::default(), Rc::new(|_| {}));
            shell.listen(
                async_channel::unbounded().1,
                async_channel::unbounded().1,
                cx,
            );
            shell
        })
    });
    let pane = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).settings().settings_pane);
    // "Settings…" (⌘,) opens the window at the pane that was visible last.
    cx.update(|cx| cx.dispatch_action(&OpenSettings));
    cx.run_until_parked();
    assert_eq!(windows(cx), 1);
    assert_eq!(pane(cx), SettingsPane::General);
    // "About OpenCraft" shows the pane "About" in the same window.
    cx.update(|cx| cx.dispatch_action(&OpenAbout));
    cx.run_until_parked();
    assert_eq!(windows(cx), 1);
    assert_eq!(pane(cx), SettingsPane::About);
    // A second ⌘, brings the window to the front, and keeps the pane.
    cx.update(|cx| cx.dispatch_action(&OpenSettings));
    cx.run_until_parked();
    assert_eq!(windows(cx), 1);
    assert_eq!(pane(cx), SettingsPane::About);
    // The file keeps the pane for the next start.
    assert_eq!(
        settings::load(dir.path()).0.settings_pane,
        SettingsPane::About
    );

    // ⌘W closes the Settings window and no other window.
    let window = cx.update(|cx| cx.windows()[0]);
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                window.dispatch_action(app_menu::CloseWindow.boxed_clone(), cx)
            })
            .unwrap()
    });
    cx.run_until_parked();
    assert_eq!(windows(cx), 0);
}

#[gpui_kit::test]
fn the_application_menu_has_about_settings_and_quit(cx: &mut TestAppContext) {
    cx.update(|cx| app_menu::install(cx, &settings::Panes::default()));
    let menus = cx.update(|cx| cx.get_menus()).unwrap_or_default();
    let menu = menus
        .into_iter()
        .find(|m| m.name.as_ref() == "OpenCraft")
        .expect("an application menu");
    let items: Vec<String> = menu
        .items
        .iter()
        .map(|item| match item {
            OwnedMenuItem::Action { name, .. } => name.to_string(),
            OwnedMenuItem::Separator => "-".into(),
            _ => "?".into(),
        })
        .collect();
    assert_eq!(
        items,
        ["About OpenCraft", "-", "Settings…", "-", "Quit OpenCraft"]
    );
}
