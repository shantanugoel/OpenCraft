//! The shell of the process: the state with no window, the shell commands,
//! the window, and the Dock icon (decision 0020).
//!
//! One [`Shell`] entity exists for the process. It gets the engine events
//! also when no window is open. A new window starts from its state, and then
//! gets each event from it. The menu bar item shows the same state.
//!
//! The shell also turns the start at login on and off (decision 0021), and
//! opens the licence notices of the bundle (decision 0026).

use crate::{
    menu::MenuState,
    settings::{self, Panes, Settings},
    state::ViewState,
    view::{theme, DeviceView},
};
use gpui_kit::{
    point, prelude::*, px, size, ActivationPolicy, App, Bounds, Context, EventEmitter,
    TitlebarOptions, WindowBounds, WindowOptions,
};
use opencraft_engine::{Command, Event};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
    time::Instant,
};

/// The time from the change to the regular activation policy to the
/// activation of the application. See `docs/kb/gpui.md`, "Windows, Dock icon,
/// and activation".
const ACTIVATE_DELAY: Duration = Duration::from_millis(200);

/// The file of the licence notices in `Contents/Resources` of the bundle.
/// `scripts/release.sh` puts it there (decision 0026).
pub const NOTICES_FILE: &str = "THIRD-PARTY-NOTICES.txt";

/// The licence notices file of the bundle that contains `executable`, if the
/// file exists. `None` for an executable that is not in a bundle, for example
/// after `cargo run`.
pub fn notices_file(executable: &Path) -> Option<PathBuf> {
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    if macos.file_name()? != "MacOS" || contents.file_name()? != "Contents" {
        return None;
    }
    let file = contents.join("Resources").join(NOTICES_FILE);
    file.is_file().then_some(file)
}

/// Opens a file with the default application. False if it cannot.
type OpenFile = fn(&Path) -> bool;

#[cfg(target_os = "macos")]
const OPEN_FILE: OpenFile = crate::menu_bar::open_file;
#[cfg(not(target_os = "macos"))]
const OPEN_FILE: OpenFile = |_| false;

/// A command to the shell, from the menu bar item, from a click on the Dock
/// icon, or from a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand {
    /// Open the window, or bring it to the front.
    Open,
    /// Show the Dock icon if it is hidden, and hide it if it shows.
    ToggleDockIcon,
    /// Turn the start at login on if it is off, and off if it is on.
    ToggleStartAtLogin,
    /// Open the licence notices file of the bundle.
    OpenNotices,
    /// Start the quit path.
    Quit,
}

pub struct Shell {
    /// The latest engine state. The selection fields stay empty here: each
    /// window has its own selection.
    pub state: ViewState,
    settings: Settings,
    /// The directory of `shell.toml`.
    config_dir: PathBuf,
    /// Sends a command to the engine.
    send: Rc<dyn Fn(Command)>,
    /// The licence notices file of the bundle. `None` if the executable is
    /// not in a bundle with the file.
    notices: Option<PathBuf>,
    open_file: OpenFile,
    #[cfg(target_os = "macos")]
    menu_bar: Option<crate::menu_bar::MenuBar>,
    /// The directory of the LaunchAgent file. `None` if the home directory
    /// is not known.
    #[cfg(target_os = "macos")]
    login_dir: Option<PathBuf>,
    /// The executable of this process. The LaunchAgent file names it.
    #[cfg(target_os = "macos")]
    executable: Option<PathBuf>,
    /// The LaunchAgent file starts this executable. The shell reads this at
    /// the start and after each change.
    #[cfg(target_os = "macos")]
    start_at_login: bool,
}

/// The shell sends each engine event to the open window after it applies the
/// event to its own state.
impl EventEmitter<Event> for Shell {}

impl Shell {
    pub fn new(config_dir: PathBuf, settings: Settings, send: Rc<dyn Fn(Command)>) -> Self {
        #[cfg(target_os = "macos")]
        let login_dir = crate::login::agents_dir();
        let current_exe = std::env::current_exe().ok();
        let notices = current_exe.as_deref().and_then(notices_file);
        #[cfg(target_os = "macos")]
        let executable = current_exe;
        Self {
            state: ViewState::default(),
            settings,
            config_dir,
            send,
            notices,
            open_file: OPEN_FILE,
            #[cfg(target_os = "macos")]
            menu_bar: None,
            #[cfg(target_os = "macos")]
            start_at_login: login_dir
                .as_deref()
                .zip(executable.as_deref())
                .is_some_and(|(dir, executable)| crate::login::is_on(dir, executable)),
            #[cfg(target_os = "macos")]
            login_dir,
            #[cfg(target_os = "macos")]
            executable,
        }
    }

    /// Use `dir` in place of `~/Library/LaunchAgents`.
    #[cfg(all(test, target_os = "macos"))]
    pub fn set_login_dir(&mut self, dir: PathBuf) {
        self.login_dir = Some(dir);
        self.start_at_login = self.login_is_on();
    }

    /// Use `executable` in place of the executable of this process.
    #[cfg(all(test, target_os = "macos"))]
    pub fn set_executable(&mut self, executable: PathBuf) {
        self.executable = Some(executable);
        self.start_at_login = self.login_is_on();
    }

    /// Use `notices` as the licence notices file, and `open_file` to open a
    /// file. The tests open no application.
    #[cfg(test)]
    pub fn set_notices(&mut self, notices: Option<PathBuf>, open_file: OpenFile) {
        self.notices = notices;
        self.open_file = open_file;
        self.refresh_menu();
    }

    /// True if the LaunchAgent file starts the executable of this process.
    #[cfg(target_os = "macos")]
    fn login_is_on(&self) -> bool {
        match (&self.login_dir, &self.executable) {
            (Some(dir), Some(executable)) => crate::login::is_on(dir, executable),
            _ => false,
        }
    }

    /// Start the two tasks that receive the engine events and the shell
    /// commands. The tasks stop when the entity goes away.
    pub fn listen(
        &mut self,
        events: async_channel::Receiver<Event>,
        commands: async_channel::Receiver<ShellCommand>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |shell, cx| {
            while let Ok(event) = events.recv().await {
                if shell
                    .update(cx, |shell, cx| shell.receive(event, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        cx.spawn(async move |shell, cx| {
            while let Ok(command) = commands.recv().await {
                if shell
                    .update(cx, |shell, cx| shell.command(command, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Make the menu bar item. Call this on the main thread, in the callback
    /// of `Application::run`.
    #[cfg(target_os = "macos")]
    pub fn add_menu_bar(&mut self, commands: async_channel::Sender<ShellCommand>) {
        self.menu_bar = crate::menu_bar::MenuBar::new(commands);
        if self.menu_bar.is_none() {
            eprintln!("OpenCraft cannot make the menu bar item.");
        }
        self.refresh_menu();
    }

    pub fn receive(&mut self, event: Event, cx: &mut Context<Self>) {
        self.state.apply(event.clone(), Instant::now());
        cx.emit(event);
        self.refresh_menu();
        cx.notify();
    }

    /// Show a problem in the window, also in a window that opens later.
    pub fn problem(&mut self, text: String, cx: &mut Context<Self>) {
        eprintln!("{text}");
        self.receive(Event::Problem(text), cx);
    }

    /// The user closed a problem in the window. A new window does not show it.
    pub fn dismiss_problem(&mut self, text: &str) {
        if let Some(index) = self.state.problems.iter().position(|p| p == text) {
            self.state.problems.remove(index);
        }
    }

    pub fn command(&mut self, command: ShellCommand, cx: &mut Context<Self>) {
        match command {
            ShellCommand::Open => self.open_window(cx),
            ShellCommand::ToggleDockIcon => {
                self.set_show_dock_icon(!self.settings.show_dock_icon, cx)
            }
            ShellCommand::ToggleStartAtLogin => self.toggle_start_at_login(cx),
            ShellCommand::OpenNotices => self.open_notices(cx),
            ShellCommand::Quit => cx.quit(),
        }
    }

    /// Keep the panes of the window in `shell.toml`, and set the titles of
    /// the View menu for them. Nothing changes if the panes are the same.
    pub fn set_panes(&mut self, panes: Panes, cx: &mut Context<Self>) {
        if self.settings.panes == panes {
            return;
        }
        let menus = self.settings.panes.sidebar_visible != panes.sidebar_visible
            || self.settings.panes.inspector_visible != panes.inspector_visible;
        self.settings.panes = panes;
        if menus {
            crate::app_menu::set_menus(cx, &panes);
        }
        if let Err(error) = settings::save(&self.config_dir, &self.settings) {
            self.problem(
                format!(
                    "OpenCraft cannot save the panes of the window in {}: {error}",
                    self.config_dir.join(settings::FILE_NAME).display()
                ),
                cx,
            );
        }
    }

    /// The state for a new window. The short marks of old input do not show.
    pub fn window_state(&self) -> ViewState {
        let mut state = self.state.clone();
        state.highlights.clear();
        state
    }

    // Only the macOS menu bar item and the tests read the menu state.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub fn menu_state(&self) -> MenuState {
        let state = &self.state;
        MenuState {
            connected: state.connected,
            profile: state.profile.as_ref().and_then(|id| {
                state
                    .profiles
                    .iter()
                    .find(|p| &p.id == id)
                    .map(|p| p.name.clone())
            }),
            show_dock_icon: self.settings.show_dock_icon,
            #[cfg(target_os = "macos")]
            start_at_login: self.start_at_login,
            #[cfg(not(target_os = "macos"))]
            start_at_login: false,
            notices: self.notices.is_some(),
        }
    }

    /// Open the licence notices with the default application. The menu shows
    /// the item only if the bundle has the file.
    fn open_notices(&mut self, cx: &mut Context<Self>) {
        let text = match &self.notices {
            Some(file) if (self.open_file)(file) => return,
            Some(file) => format!(
                "OpenCraft cannot open the licence notices in {}.",
                file.display()
            ),
            None => format!(
                "This copy of OpenCraft has no licence notices. The file {NOTICES_FILE} \
                 of the source code has them."
            ),
        };
        self.problem(text, cx);
    }

    fn refresh_menu(&mut self) {
        #[cfg(target_os = "macos")]
        if self.menu_bar.is_some() {
            let state = self.menu_state();
            if let Some(menu_bar) = &mut self.menu_bar {
                menu_bar.show(&state);
            }
        }
    }

    /// Open the window. If a window is open, bring it and the application to
    /// the front.
    pub fn open_window(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = cx.windows().into_iter().next() {
            let _ = window.update(cx, |_, window, _| window.activate_window());
            cx.activate(true);
            return;
        }
        let shell = cx.entity();
        let state = self.window_state();
        let send = self.send.clone();
        let panes = self.settings.panes;
        let options = window_options(cx);
        let opened = cx.open_window(options, move |window, cx| {
            if crate::start_trace() {
                window.on_next_frame(|_, _| crate::trace_now("first frame"));
            }
            let view = cx.new(|cx| DeviceView::new(&shell, state, panes, send, window, cx));
            cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
        });
        if let Err(error) = opened {
            eprintln!("OpenCraft cannot open the window: {error}");
            return;
        }
        cx.activate(true);
    }

    fn set_show_dock_icon(&mut self, show: bool, cx: &mut Context<Self>) {
        self.settings.show_dock_icon = show;
        apply_dock_icon(show, cx);
        if let Err(error) = settings::save(&self.config_dir, &self.settings) {
            self.problem(
                format!(
                    "OpenCraft cannot save the Dock icon setting in {}: {error}",
                    self.config_dir.join(settings::FILE_NAME).display()
                ),
                cx,
            );
        }
        self.refresh_menu();
        cx.notify();
    }

    /// Write or remove the LaunchAgent file. The new state comes from the
    /// file, also after an error.
    ///
    /// If the file starts a different executable, for example a copy of the
    /// application that the user moved, the option is off, and the user can
    /// turn it on for this executable (decision 0025).
    #[cfg(target_os = "macos")]
    fn toggle_start_at_login(&mut self, cx: &mut Context<Self>) {
        use crate::login;
        let (Some(dir), Some(executable)) = (self.login_dir.clone(), self.executable.clone())
        else {
            self.problem(
                "OpenCraft cannot change the start at login: it cannot find the home directory \
                 or its own executable."
                    .into(),
                cx,
            );
            return;
        };
        let on = self.login_is_on();
        if !on && login::on_read_only_volume(&executable) {
            self.problem(
                "To start OpenCraft at login, move it to the Applications folder and start it \
                 from there. It runs now from a read-only location, for example the disk image."
                    .into(),
                cx,
            );
            return;
        }
        let result = if on {
            login::turn_off(&dir)
        } else {
            login::turn_on(&dir, &executable)
        };
        if let Err(error) = result {
            self.problem(
                format!(
                    "OpenCraft cannot change the start at login in {}: {error}",
                    dir.join(login::FILE_NAME).display()
                ),
                cx,
            );
        }
        self.start_at_login = self.login_is_on();
        self.refresh_menu();
        cx.notify();
    }

    /// Only macOS has a start at login (decision 0021).
    #[cfg(not(target_os = "macos"))]
    fn toggle_start_at_login(&mut self, cx: &mut Context<Self>) {
        self.problem(
            "OpenCraft has no start at login on this platform.".into(),
            cx,
        );
    }
}

/// The options of the main window (decision 0029, section 4): 1440 × 900,
/// or the visible area of a smaller screen, and a minimum size so that the
/// panes do not overlap. The title bar is transparent: the sidebar and the
/// toolbar fill its area, and the window moves with a drag on them.
fn window_options(cx: &App) -> WindowOptions {
    let (width, height) = theme::WINDOW_SIZE;
    let mut window_size = size(px(width), px(height));
    let bounds = match cx.primary_display() {
        Some(display) => {
            let area = display.visible_bounds();
            window_size = window_size.min(&area.size);
            Bounds::centered_at(area.center(), window_size)
        }
        None => Bounds::centered(None, window_size, cx),
    };
    let (min_width, min_height) = theme::WINDOW_MIN;
    let (x, y) = theme::WINDOW_BUTTONS;
    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("OpenCraft".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(x, y)),
        }),
        app_owns_titlebar_drag: true,
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(min_width), px(min_height))),
        ..Default::default()
    }
}

/// Show or hide the Dock icon at run time. An open window stays open.
fn apply_dock_icon(show: bool, cx: &mut App) {
    if !show {
        cx.set_activation_policy(ActivationPolicy::Accessory);
        return;
    }
    // If the application is active at the change, the menu bar stays with a
    // different application. The Dock application takes the activation
    // first, and the application takes it back later.
    #[cfg(target_os = "macos")]
    let active = crate::menu_bar::activate_dock_if_active();
    #[cfg(not(target_os = "macos"))]
    let active = false;
    cx.set_activation_policy(ActivationPolicy::Regular);
    if active {
        cx.spawn(async move |cx| {
            cx.background_executor().timer(ACTIVATE_DELAY).await;
            cx.update(|cx| cx.activate(true));
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::tests::{key, layout};
    use gpui_kit::{Entity, TestAppContext};
    use opencraft_engine::{ProfileEntry, RgbImage, SurfaceContent, SurfaceState};
    use std::{cell::RefCell, sync::Arc};

    struct Fixture {
        shell: Entity<Shell>,
        events: async_channel::Sender<Event>,
        commands: async_channel::Sender<ShellCommand>,
        sent: Rc<RefCell<Vec<Command>>>,
        dir: tempfile::TempDir,
    }

    fn shell(cx: &mut TestAppContext) -> Fixture {
        cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);
        });
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().to_path_buf();
        #[cfg(target_os = "macos")]
        let config_for_login = config.clone();
        let (events, event_receiver) = async_channel::unbounded();
        let (commands, command_receiver) = async_channel::unbounded();
        let sent: Rc<RefCell<Vec<Command>>> = Default::default();
        let log = sent.clone();
        let shell = cx.update(|cx| {
            cx.new(|cx| {
                let mut shell = Shell::new(
                    config,
                    Settings::default(),
                    Rc::new(move |c| log.borrow_mut().push(c)),
                );
                // The tests never read or write `~/Library/LaunchAgents`.
                #[cfg(target_os = "macos")]
                shell.set_login_dir(config_for_login.join("LaunchAgents"));
                shell.listen(event_receiver, command_receiver, cx);
                shell
            })
        });
        Fixture {
            shell,
            events,
            commands,
            sent,
            dir,
        }
    }

    fn profiles() -> Event {
        Event::Profiles {
            profiles: vec![ProfileEntry {
                id: "desk".into(),
                name: "Desk".into(),
            }],
            active: "desk".into(),
        }
    }

    fn surface(label: &str) -> Event {
        Event::Surface(SurfaceState {
            id: key(),
            content: SurfaceContent {
                label: Some(label.into()),
                ..Default::default()
            },
            image: Arc::new(RgbImage::new(90, 90)),
        })
    }

    fn view(cx: &mut TestAppContext) -> Option<Entity<DeviceView>> {
        cx.update(|cx| {
            let window = cx.windows().into_iter().next()?;
            window
                .update(cx, |root, _, cx| {
                    root.downcast::<gpui_kit::base::Root>()
                        .ok()
                        .and_then(|root| root.read(cx).view().clone().downcast().ok())
                })
                .ok()
                .flatten()
        })
    }

    #[gpui_kit::test]
    fn the_state_and_the_menu_follow_the_engine_with_no_window(cx: &mut TestAppContext) {
        let f = shell(cx);
        let menu = |cx: &mut TestAppContext| cx.update(|cx| f.shell.read(cx).menu_state());
        assert_eq!(
            menu(cx),
            MenuState {
                connected: false,
                profile: None,
                show_dock_icon: true,
                start_at_login: false,
                notices: false
            }
        );
        f.events.try_send(layout()).unwrap();
        f.events.try_send(profiles()).unwrap();
        f.events.try_send(Event::Connected).unwrap();
        cx.run_until_parked();
        assert!(cx.update(|cx| cx.windows().is_empty()));
        assert_eq!(
            menu(cx),
            MenuState {
                connected: true,
                profile: Some("Desk".into()),
                show_dock_icon: true,
                start_at_login: false,
                notices: false
            }
        );
        f.events.try_send(Event::Disconnected).unwrap();
        cx.run_until_parked();
        assert!(!menu(cx).connected);
    }

    #[gpui_kit::test]
    fn a_new_window_shows_the_current_state_and_gets_later_events(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.events.try_send(layout()).unwrap();
        f.events.try_send(surface("Mail")).unwrap();
        f.events.try_send(Event::Connected).unwrap();
        f.events
            .try_send(Event::Problem("Cannot save.".into()))
            .unwrap();
        cx.run_until_parked();
        f.commands.try_send(ShellCommand::Open).unwrap();
        cx.run_until_parked();
        let view = view(cx).expect("The Open command opens a window.");
        cx.update(|cx| {
            let state = &view.read(cx).state;
            assert!(state.connected);
            assert_eq!(
                state.surfaces[&key()].content.label.as_deref(),
                Some("Mail")
            );
            assert_eq!(state.problems, ["Cannot save."]);
        });
        f.events.try_send(surface("Sound")).unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(
                view.read(cx).state.surfaces[&key()]
                    .content
                    .label
                    .as_deref(),
                Some("Sound")
            )
        });
        // A second Open brings the same window to the front.
        f.commands.try_send(ShellCommand::Open).unwrap();
        cx.run_until_parked();
        assert_eq!(cx.update(|cx| cx.windows().len()), 1);
        assert!(f.sent.borrow().is_empty());
    }

    #[gpui_kit::test]
    fn the_engine_state_continues_after_the_window_closes(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.events.try_send(layout()).unwrap();
        f.commands.try_send(ShellCommand::Open).unwrap();
        cx.run_until_parked();
        let window = cx.update(|cx| cx.windows()[0]);
        cx.update(|cx| {
            window
                .update(cx, |_, window, _| window.remove_window())
                .unwrap()
        });
        cx.run_until_parked();
        assert!(cx.update(|cx| cx.windows().is_empty()));
        f.events.try_send(surface("Later")).unwrap();
        f.events.try_send(Event::Connected).unwrap();
        cx.run_until_parked();
        f.commands.try_send(ShellCommand::Open).unwrap();
        cx.run_until_parked();
        let view = view(cx).unwrap();
        cx.update(|cx| {
            let state = &view.read(cx).state;
            assert!(state.connected);
            assert_eq!(
                state.surfaces[&key()].content.label.as_deref(),
                Some("Later")
            );
        });
    }

    #[gpui_kit::test]
    fn the_dock_icon_option_is_kept_in_shell_toml(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.commands.try_send(ShellCommand::ToggleDockIcon).unwrap();
        cx.run_until_parked();
        assert!(!cx.update(|cx| f.shell.read(cx).menu_state().show_dock_icon));
        assert_eq!(
            settings::load(f.dir.path()),
            (
                Settings {
                    show_dock_icon: false,
                    ..Settings::default()
                },
                None
            )
        );
        f.commands.try_send(ShellCommand::ToggleDockIcon).unwrap();
        cx.run_until_parked();
        assert!(cx.update(|cx| f.shell.read(cx).menu_state().show_dock_icon));
        assert_eq!(settings::load(f.dir.path()), (Settings::default(), None));
    }

    #[cfg(target_os = "macos")]
    #[gpui_kit::test]
    fn the_start_at_login_option_follows_the_launch_agent_file(cx: &mut TestAppContext) {
        use crate::login;
        let f = shell(cx);
        let agents = f.dir.path().join("LaunchAgents");
        let on =
            |cx: &mut TestAppContext| cx.update(|cx| f.shell.read(cx).menu_state().start_at_login);
        assert!(!on(cx));
        f.commands
            .try_send(ShellCommand::ToggleStartAtLogin)
            .unwrap();
        cx.run_until_parked();
        assert!(on(cx));
        let text = std::fs::read_to_string(agents.join(login::FILE_NAME)).unwrap();
        let executable = std::env::current_exe().unwrap();
        assert_eq!(text, login::content(executable.to_str().unwrap()));
        f.commands
            .try_send(ShellCommand::ToggleStartAtLogin)
            .unwrap();
        cx.run_until_parked();
        assert!(!on(cx));
        assert!(!agents.join(login::FILE_NAME).exists());
        assert!(cx.update(|cx| f.shell.read(cx).state.problems.is_empty()));
    }

    #[cfg(target_os = "macos")]
    #[gpui_kit::test]
    fn a_file_for_a_different_executable_shows_off_and_is_replaced(cx: &mut TestAppContext) {
        use crate::login;
        let f = shell(cx);
        let agents = f.dir.path().join("LaunchAgents");
        // The user moved the application after the option was on.
        login::turn_on(
            &agents,
            std::path::Path::new("/old place/OpenCraft.app/opencraft"),
        )
        .unwrap();
        f.shell
            .update(cx, |shell, _| shell.set_login_dir(agents.clone()));
        let on =
            |cx: &mut TestAppContext| cx.update(|cx| f.shell.read(cx).menu_state().start_at_login);
        assert!(!on(cx));
        f.commands
            .try_send(ShellCommand::ToggleStartAtLogin)
            .unwrap();
        cx.run_until_parked();
        assert!(on(cx));
        let executable = std::env::current_exe().unwrap();
        assert_eq!(
            std::fs::read_to_string(agents.join(login::FILE_NAME)).unwrap(),
            login::content(executable.to_str().unwrap())
        );
    }

    #[cfg(target_os = "macos")]
    #[gpui_kit::test]
    fn an_executable_on_a_read_only_volume_does_not_turn_the_option_on(cx: &mut TestAppContext) {
        use crate::login;
        let f = shell(cx);
        // The system volume is read-only, as a disk image is.
        f.shell.update(cx, |shell, _| {
            shell.set_executable(PathBuf::from("/usr/bin/true"))
        });
        f.commands
            .try_send(ShellCommand::ToggleStartAtLogin)
            .unwrap();
        cx.run_until_parked();
        assert!(!cx.update(|cx| f.shell.read(cx).menu_state().start_at_login));
        assert!(!f
            .dir
            .path()
            .join("LaunchAgents")
            .join(login::FILE_NAME)
            .exists());
        let problems = cx.update(|cx| f.shell.read(cx).state.problems.clone());
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("move it to the Applications folder"));
    }

    #[cfg(target_os = "macos")]
    #[gpui_kit::test]
    fn an_error_of_the_start_at_login_shows_as_a_problem(cx: &mut TestAppContext) {
        let f = shell(cx);
        // A file in place of the directory: the directory cannot be made.
        let blocked = f.dir.path().join("file");
        std::fs::write(&blocked, "").unwrap();
        f.shell.update(cx, |shell, _| {
            shell.set_login_dir(blocked.join("LaunchAgents"))
        });
        f.commands
            .try_send(ShellCommand::ToggleStartAtLogin)
            .unwrap();
        cx.run_until_parked();
        assert!(!cx.update(|cx| f.shell.read(cx).menu_state().start_at_login));
        let problems = cx.update(|cx| f.shell.read(cx).state.problems.clone());
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("cannot change the start at login"));
    }

    /// A bundle in `dir` with the executable, and with the notices file if
    /// `with_file`. Gives the executable.
    fn bundle(dir: &Path, with_file: bool) -> PathBuf {
        let contents = dir.join("OpenCraft.app").join("Contents");
        std::fs::create_dir_all(contents.join("MacOS")).unwrap();
        std::fs::create_dir_all(contents.join("Resources")).unwrap();
        if with_file {
            std::fs::write(contents.join("Resources").join(NOTICES_FILE), "Notices").unwrap();
        }
        contents.join("MacOS").join("opencraft")
    }

    #[test]
    fn the_notices_file_is_in_the_resources_of_the_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let executable = bundle(dir.path(), true);
        assert_eq!(
            notices_file(&executable),
            Some(
                dir.path()
                    .join("OpenCraft.app/Contents/Resources")
                    .join(NOTICES_FILE)
            )
        );
        // A bundle with no file.
        let other = tempfile::tempdir().unwrap();
        assert_eq!(notices_file(&bundle(other.path(), false)), None);
        // An executable that is not in a bundle, as after `cargo run`. The
        // file next to it is not used.
        let plain = dir.path().join("target").join("debug");
        std::fs::create_dir_all(plain.join("..").join("Resources")).unwrap();
        std::fs::write(plain.join("..").join("Resources").join(NOTICES_FILE), "").unwrap();
        assert_eq!(notices_file(&plain.join("opencraft")), None);
        assert_eq!(notices_file(Path::new("opencraft")), None);
        // The test executable is not in a bundle.
        assert_eq!(notices_file(&std::env::current_exe().unwrap()), None);
    }

    thread_local! {
        static OPENED: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    }

    fn open_and_record(path: &Path) -> bool {
        OPENED.with(|opened| opened.borrow_mut().push(path.to_path_buf()));
        true
    }

    #[gpui_kit::test]
    fn the_notices_item_opens_the_file_of_the_bundle(cx: &mut TestAppContext) {
        let f = shell(cx);
        // The test executable is not in a bundle: the menu has no item.
        assert!(!cx.update(|cx| f.shell.read(cx).menu_state().notices));
        let file = notices_file(&bundle(f.dir.path(), true)).unwrap();
        f.shell.update(cx, |shell, _| {
            shell.set_notices(Some(file.clone()), open_and_record)
        });
        assert!(cx.update(|cx| f.shell.read(cx).menu_state().notices));
        f.commands.try_send(ShellCommand::OpenNotices).unwrap();
        cx.run_until_parked();
        assert_eq!(OPENED.with(|opened| opened.borrow().clone()), [file]);
        assert!(cx.update(|cx| f.shell.read(cx).state.problems.is_empty()));
        assert!(cx.update(|cx| cx.windows().is_empty()));
    }

    #[gpui_kit::test]
    fn a_notices_file_that_does_not_open_shows_a_problem(cx: &mut TestAppContext) {
        let f = shell(cx);
        let file = notices_file(&bundle(f.dir.path(), true)).unwrap();
        f.shell
            .update(cx, |shell, _| shell.set_notices(Some(file), |_| false));
        f.commands.try_send(ShellCommand::OpenNotices).unwrap();
        cx.run_until_parked();
        // With no file, the command shows where the notices are.
        f.shell
            .update(cx, |shell, _| shell.set_notices(None, open_and_record));
        f.commands.try_send(ShellCommand::OpenNotices).unwrap();
        cx.run_until_parked();
        let problems = cx.update(|cx| f.shell.read(cx).state.problems.clone());
        assert_eq!(problems.len(), 2);
        assert!(problems[0].contains("cannot open the licence notices"));
        assert!(problems[1].contains(NOTICES_FILE));
        assert!(OPENED.with(|opened| opened.borrow().is_empty()));
    }

    #[gpui_kit::test]
    fn a_closed_problem_does_not_show_in_a_new_window(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.events.try_send(Event::Problem("One".into())).unwrap();
        f.events.try_send(Event::Problem("Two".into())).unwrap();
        cx.run_until_parked();
        f.shell.update(cx, |shell, _| shell.dismiss_problem("One"));
        assert_eq!(
            cx.update(|cx| f.shell.read(cx).window_state().problems),
            ["Two"]
        );
    }
}
