//! The shell of the process: the state with no window, the shell commands,
//! the window, and the Dock icon (decision 0020).
//!
//! One [`Shell`] entity exists for the process. It gets the engine events
//! also when no window is open. A new window starts from its state, and then
//! gets each event from it. The menu bar item shows the same state.
//!
//! The shell also turns the start at login on and off (decision 0021), opens
//! the licence notices of the bundle (decision 0026), and opens the Settings
//! window (decision 0029, section 17). The window code is in the module
//! `settings_window`.
//!
//! With no profile file at the start, the shell opens the first start window
//! in place of the main window (decision 0029, section 18, module
//! `first_start`), until the user starts with a starter profile.

use crate::{
    app_menu::{EditMenu, OpenAbout, OpenSettings},
    first_start::FirstStartView,
    menu::{Brightness, MenuState},
    settings::{self, Panes, Settings, SettingsPane},
    settings_window::{kind_waiting_for_permission, SettingsView, Snapshot},
    starters::{self, STARTERS},
    state::ViewState,
    view::{theme, DeviceView},
};
use gpui_kit::{
    point, prelude::*, px, size, ActivationPolicy, AnyWindowHandle, App, Bounds, Context,
    EventEmitter, Task, TitlebarOptions, WindowBounds, WindowHandle, WindowOptions,
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

/// The shortest time between two brightness commands while the user moves
/// the slider of the menu. The window uses the same value: see
/// `SETTINGS_INTERVAL` in `view/device.rs` for the reason.
const BRIGHTNESS_INTERVAL: Duration = Duration::from_millis(50);

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

/// The name of the log file (decision 0024).
#[cfg(unix)]
const LOG_FILE_NAME: &str = crate::log_file::FILE_NAME;
#[cfg(not(unix))]
const LOG_FILE_NAME: &str = "opencraft.log";

/// Opens a file with the default application. False if it cannot.
type OpenFile = fn(&Path) -> bool;

#[cfg(target_os = "macos")]
const OPEN_FILE: OpenFile = crate::menu_bar::open_file;
#[cfg(not(target_os = "macos"))]
const OPEN_FILE: OpenFile = |_| false;

/// The address of the pane "Accessibility" of System Settings.
pub const ACCESSIBILITY_PANE: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

/// The functions that show something to the user outside the window. The
/// tests replace them, so that a test opens no application.
#[derive(Clone, Copy)]
struct Desktop {
    /// Show a file or a folder in the Finder.
    show_in_finder: fn(&Path) -> bool,
    /// Open an address, for example a pane of System Settings.
    open_url: fn(&str) -> bool,
}

#[cfg(target_os = "macos")]
const DESKTOP: Desktop = Desktop {
    show_in_finder: crate::menu_bar::show_in_finder,
    open_url: crate::menu_bar::open_url,
};
#[cfg(not(target_os = "macos"))]
const DESKTOP: Desktop = Desktop {
    show_in_finder: |_| false,
    open_url: |_| false,
};

/// A place that the Settings window shows in the Finder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// The folder with the profiles.
    Profiles,
    /// The log file, or its folder if the file does not exist.
    LogFile,
}

/// Fixed help destinations. No address comes from a server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpPage {
    UserGuide,
    Issues,
}

impl HelpPage {
    pub fn url(self) -> &'static str {
        match self {
            Self::UserGuide => {
                "https://github.com/shantanugoel/OpenCraft/blob/main/docs/user/README.md"
            }
            Self::Issues => "https://github.com/shantanugoel/OpenCraft/issues",
        }
    }
}

/// A command to the shell, from the menu bar item, from a click on the Dock
/// icon, or from a signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellCommand {
    /// Open the window, or bring it to the front.
    Open,
    /// A platform power notice; the engine owns the policy.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    PowerNotice(opencraft_engine::PowerNotice),
    /// Wake the controller without opening a window.
    WakeDevice,
    /// Make the profile with this identity the active profile. The window
    /// follows the engine events.
    SelectProfile(String),
    /// Make the page with this number of the active profile the active page.
    SelectPage(usize),
    /// Set the display brightness of the device, in percent. Only the
    /// slider of the menu bar item (macOS) makes this command.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    SetBrightness(u8),
    /// Open the Settings window, or bring it to the front. With a pane, show
    /// that pane; with `None`, show the pane that was visible last.
    OpenSettings(Option<SettingsPane>),
    /// Show the Dock icon if it is hidden, and hide it if it shows.
    ToggleDockIcon,
    /// Turn the start at login on if it is off, and off if it is on.
    ToggleStartAtLogin,
    /// Turn the opening of the window at the start on if it is off, and off
    /// if it is on.
    ToggleShowWindowAtStart,
    /// Open the licence notices file of the bundle.
    OpenNotices,
    /// Copy a local support report, with private log details removed.
    CopyDiagnostics,
    /// Open a fixed help page in the default browser.
    OpenHelp(HelpPage),
    /// Show a folder or a file of OpenCraft in the Finder.
    ShowInFinder(Place),
    /// Open the pane "Accessibility" of System Settings, and ask the engine
    /// to wait for the permission.
    OpenAccessibilitySettings,
    /// Start the quit path.
    Quit,
}

pub struct Shell {
    /// The latest engine state. The selection fields stay empty here: each
    /// window has its own selection.
    pub state: ViewState,
    settings: Settings,
    /// True while a text field of the window that edits the profile has the
    /// focus. Then the Edit menu shows no names of steps.
    typing: bool,
    /// The directory of `shell.toml`.
    config_dir: PathBuf,
    /// Sends a command to the engine.
    send: Rc<dyn Fn(Command)>,
    /// The licence notices file of the bundle. `None` if the executable is
    /// not in a bundle with the file.
    notices: Option<PathBuf>,
    open_file: OpenFile,
    desktop: Desktop,
    /// The profiles directory, for the Settings window.
    profiles_dir: PathBuf,
    /// The window with the editor. `None` before the first window. It can
    /// be a window that the user closed: look in `App::windows`.
    main_window: Option<AnyWindowHandle>,
    /// The Settings window, with the same condition.
    settings_window: Option<WindowHandle<gpui_kit::base::Root>>,
    /// The last problem of a command of the Settings window. The window
    /// shows it below its pane, because the user cannot see the messages of
    /// the main window there.
    settings_notice: Option<String>,
    /// True while the window is the first start window: the profiles
    /// directory had no profile file at the start, and the user did not
    /// start with a starter profile yet (decision 0029, section 18).
    first_start: bool,
    /// The first start window, with the condition of `main_window`.
    first_start_window: Option<WindowHandle<gpui_kit::base::Root>>,
    /// True after the Accessibility request showed in this run (section 18).
    permission_asked: bool,
    /// Holds the interval between two brightness commands.
    brightness_throttle: Option<Task<()>>,
    /// The last brightness of the menu that waits for the end of the interval.
    brightness_pending: Option<u8>,
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
            typing: false,
            profiles_dir: config_dir.join("profiles"),
            config_dir,
            send,
            notices,
            open_file: OPEN_FILE,
            desktop: DESKTOP,
            main_window: None,
            settings_window: None,
            settings_notice: None,
            first_start: false,
            first_start_window: None,
            permission_asked: false,
            brightness_throttle: None,
            brightness_pending: None,
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

    /// Use `show_in_finder` and `open_url` in place of the functions of the
    /// system. The tests open no application.
    #[cfg(test)]
    pub fn set_desktop(&mut self, show_in_finder: fn(&Path) -> bool, open_url: fn(&str) -> bool) {
        self.desktop = Desktop {
            show_in_finder,
            open_url,
        };
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
        // The key command and the menu items of the application menu. They
        // work also when the Settings window or no window has the focus.
        let settings = cx.weak_entity();
        let about = settings.clone();
        let app: &mut App = cx;
        app.on_action(move |_: &OpenSettings, cx| {
            let _ = settings.update(cx, |shell, cx| shell.open_settings(None, cx));
        });
        app.on_action(move |_: &OpenAbout, cx| {
            let _ = about.update(cx, |shell, cx| {
                shell.open_settings(Some(SettingsPane::About), cx)
            });
        });
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
        let history = matches!(event, Event::History { .. });
        self.state.apply(event.clone(), Instant::now());
        if history {
            self.set_menus(cx);
        }
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
            ShellCommand::PowerNotice(notice) => (self.send)(Command::PowerNotice(notice)),
            ShellCommand::WakeDevice => (self.send)(Command::WakeDevice),
            ShellCommand::SelectProfile(profile) => {
                if self.state.profile.as_ref() != Some(&profile) {
                    (self.send)(Command::SelectProfile { profile });
                }
            }
            ShellCommand::SelectPage(page) => {
                if self.state.page != page {
                    (self.send)(Command::SelectPage { page });
                }
            }
            ShellCommand::SetBrightness(value) => self.set_brightness(value, cx),
            ShellCommand::OpenSettings(pane) => self.open_settings(pane, cx),
            ShellCommand::ToggleDockIcon => {
                self.set_show_dock_icon(!self.settings.show_dock_icon, cx)
            }
            ShellCommand::ToggleStartAtLogin => self.toggle_start_at_login(cx),
            ShellCommand::ToggleShowWindowAtStart => {
                self.set_show_window_at_start(!self.settings.show_window_at_start, cx)
            }
            ShellCommand::OpenNotices => self.open_notices(cx),
            ShellCommand::CopyDiagnostics => self.copy_diagnostics(cx),
            ShellCommand::OpenHelp(page) => self.open_help(page, cx),
            ShellCommand::ShowInFinder(place) => self.show_in_finder(place, cx),
            ShellCommand::OpenAccessibilitySettings => self.open_accessibility_settings(cx),
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
            self.set_menus(cx);
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

    /// The icons that the user chose last, the newest first (decision
    /// 0029, section 14).
    pub fn recent_icons(&self) -> &[String] {
        &self.settings.recent_icons
    }

    /// Put the icon `name` at the start of the recent icons, and keep the
    /// list in `shell.toml`.
    pub fn add_recent_icon(&mut self, name: &str, cx: &mut Context<Self>) {
        if !self.settings.add_recent_icon(name) {
            return;
        }
        if let Err(error) = settings::save(&self.config_dir, &self.settings) {
            self.problem(
                format!(
                    "OpenCraft cannot save the recent icons in {}: {error}",
                    self.config_dir.join(settings::FILE_NAME).display()
                ),
                cx,
            );
        }
    }

    /// A text field of the window that edits the profile got the focus
    /// (`true`) or lost it (`false`). The Edit menu then names the steps of
    /// the profile, or no step.
    pub fn set_typing(&mut self, typing: bool, cx: &mut Context<Self>) {
        if self.typing != typing {
            self.typing = typing;
            self.set_menus(cx);
        }
    }

    /// The names of the steps that the Edit menu shows.
    pub fn edit_menu(&self) -> EditMenu {
        if self.typing {
            return EditMenu::default();
        }
        EditMenu {
            undo: self.state.undo.clone(),
            redo: self.state.redo.clone(),
        }
    }

    /// Set the menus for the panes and the steps.
    fn set_menus(&self, cx: &mut Context<Self>) {
        crate::app_menu::set_menus(cx, &self.settings.panes, &self.edit_menu());
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
            device: state.layout.as_ref().map(|l| l.name.clone()),
            profiles: state.profiles.clone(),
            profile: state.profile.clone(),
            pages: state.pages.clone(),
            page: state.page,
            brightness: state.device_settings.as_ref().map(|s| Brightness {
                value: s.brightness,
                lowest: state
                    .layout
                    .as_ref()
                    .map_or(0, |l| l.settings.lowest_brightness),
            }),
        }
    }

    /// Set the display brightness from the menu bar item. While the user
    /// moves the slider, at most one command goes to the engine in each
    /// [`BRIGHTNESS_INTERVAL`], and the last value goes at the end of the
    /// interval (the same rule as the device settings of the window).
    fn set_brightness(&mut self, value: u8, cx: &mut Context<Self>) {
        if self.brightness_throttle.is_some() {
            self.brightness_pending = Some(value);
            return;
        }
        self.send_brightness(value, cx);
    }

    fn send_brightness(&mut self, value: u8, cx: &mut Context<Self>) {
        // The engine puts the value into the range of the model. The menu
        // shows the value of the engine when its reply comes.
        let Some(mut settings) = self.state.device_settings.clone() else {
            return;
        };
        if settings.brightness != value {
            settings.brightness = value;
            self.state.device_settings = Some(settings.clone());
            (self.send)(Command::SetDeviceSettings(settings));
        }
        self.brightness_throttle = Some(cx.spawn(async move |shell, cx| {
            cx.background_executor().timer(BRIGHTNESS_INTERVAL).await;
            let _ = shell.update(cx, |shell, cx| {
                shell.brightness_throttle = None;
                if let Some(next) = shell.brightness_pending.take() {
                    shell.send_brightness(next, cx);
                }
            });
        }));
    }

    /// The settings of the shell.
    #[cfg(test)]
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// True if the LaunchAgent file starts this application. Only macOS has
    /// a start at login (decision 0021).
    pub fn start_at_login(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.start_at_login;
        #[cfg(not(target_os = "macos"))]
        false
    }

    /// True if the bundle has the licence notices file.
    pub fn has_notices(&self) -> bool {
        self.notices.is_some()
    }

    /// What the Settings window shows.
    pub fn settings_snapshot(&self) -> Snapshot {
        Snapshot {
            settings: self.settings.clone(),
            start_at_login: self.start_at_login(),
            has_notices: self.has_notices(),
            profiles_dir: self.profiles_dir.clone(),
            permission: crate::settings_window::permission(&self.state),
            notice: self.settings_notice.clone(),
        }
    }

    /// The last problem of a command of the Settings window, if the user did
    /// not close it.
    #[cfg(test)]
    pub fn settings_notice(&self) -> Option<&str> {
        self.settings_notice.as_deref()
    }

    /// The user closed the message of the Settings window.
    pub fn dismiss_notice(&mut self, cx: &mut Context<Self>) {
        self.settings_notice = None;
        cx.notify();
    }

    /// A problem of a command of the Settings window. The main window and
    /// the log get it as each problem, and the Settings window shows it
    /// below its pane.
    fn settings_problem(&mut self, text: String, cx: &mut Context<Self>) {
        self.settings_notice = Some(text.clone());
        self.problem(text, cx);
    }

    /// The profiles directory.
    #[cfg(test)]
    pub fn profiles_dir(&self) -> &Path {
        &self.profiles_dir
    }

    /// Use `dir` as the profiles directory of the Settings window.
    pub fn set_profiles_dir(&mut self, dir: PathBuf) {
        self.profiles_dir = dir;
    }

    /// The log file of the application, in the directory of the shell. The
    /// file exists only after a start with no terminal (decision 0024).
    pub fn log_file(&self) -> PathBuf {
        self.config_dir.join(LOG_FILE_NAME)
    }

    fn open_help(&mut self, page: HelpPage, cx: &mut Context<Self>) {
        if !(self.desktop.open_url)(page.url()) {
            self.settings_problem(
                "OpenCraft cannot open the help page in your browser.".into(),
                cx,
            );
        }
    }

    /// Read the bounded log tail and system version off the UI thread.
    fn copy_diagnostics(&self, cx: &mut Context<Self>) {
        let path = self.log_file();
        let model = self
            .state
            .layout
            .as_ref()
            .map(|layout| layout.name.clone())
            .unwrap_or_else(|| "Not available".into());
        let firmware = self.state.firmware.clone();
        let work = cx
            .background_executor()
            .spawn(async move { crate::diagnostics::report(&path, &model, firmware.as_deref()) });
        cx.spawn(async move |_shell, cx| {
            let report = work.await;
            cx.update(|cx| cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(report)));
        })
        .detach();
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
        self.settings_problem(text, cx);
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

    /// Show the first start window in place of the main window, until the
    /// user starts with a starter profile. `main` sets this at the start if
    /// the profiles directory has no profile file.
    pub fn set_first_start(&mut self, first_start: bool) {
        self.first_start = first_start;
    }

    /// True while the window is the first start window.
    #[cfg(test)]
    pub fn first_start(&self) -> bool {
        self.first_start
    }

    /// Open the first start window, or bring it to the front.
    fn open_first_start(&mut self, cx: &mut Context<Self>) {
        let open = self
            .first_start_window
            .filter(|w| cx.windows().contains(&AnyWindowHandle::from(*w)));
        if let Some(window) = open {
            let _ = window.update(cx, |_, window, _| window.activate_window());
            cx.activate(true);
            return;
        }
        let shell = cx.entity();
        let snapshot = crate::first_start::Snapshot::of(&self.state);
        let options = crate::first_start::window_options(cx);
        let opened = cx.open_window(options, move |window, cx| {
            let view = cx.new(|cx| FirstStartView::new(&shell, snapshot, window, cx));
            cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
        });
        match opened {
            Ok(window) => self.first_start_window = Some(window),
            Err(error) => {
                eprintln!("OpenCraft cannot open the first start window: {error}");
                return;
            }
        }
        cx.activate(true);
    }

    /// Start with the starter profile `index` of [`STARTERS`]: the engine
    /// adds it and makes it the active profile, and the main window opens.
    /// The first start window closes itself. If the starter profile has an
    /// action that needs the Accessibility permission, and the permission
    /// is not given, the main window shows the request (section 18).
    pub fn start_with(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(starter) = STARTERS.get(index) else {
            return;
        };
        let profile = match starter.profile() {
            Ok(profile) => profile,
            Err(error) => {
                self.problem(
                    format!(
                        "OpenCraft cannot read the starter profile “{}”: {error}.",
                        starter.name
                    ),
                    cx,
                );
                return;
            }
        };
        let actions = starters::actions(&profile);
        if let Some(kind) = kind_waiting_for_permission(&actions, &self.state) {
            self.request_permission(kind);
        }
        (self.send)(Command::AddStarterProfile { profile });
        self.first_start = false;
        self.open_window(cx);
    }

    /// Show the Accessibility request for the kind with the name `kind` in
    /// the main window, if it did not show in this run. True if it shows
    /// now. The caller checked that the permission is not given.
    pub fn request_permission(&mut self, kind: String) -> bool {
        if self.permission_asked {
            return false;
        }
        self.permission_asked = true;
        self.state.permission_request = Some(kind);
        true
    }

    /// The user closed the Accessibility request. A new window does not
    /// show it.
    pub fn close_permission_request(&mut self) {
        self.state.permission_request = None;
    }

    /// Open the window. If the window is open, bring it and the application
    /// to the front. The Settings window is not this window. At the first
    /// start, this is the first start window.
    pub fn open_window(&mut self, cx: &mut Context<Self>) {
        (self.send)(Command::WakeDevice);
        if self.first_start {
            self.open_first_start(cx);
            return;
        }
        if let Some(window) = self.main_window.filter(|w| cx.windows().contains(w)) {
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
        match opened {
            Ok(window) => self.main_window = Some(window.into()),
            Err(error) => {
                eprintln!("OpenCraft cannot open the window: {error}");
                return;
            }
        }
        cx.activate(true);
    }

    /// Open the Settings window. If it is open, bring it and the application
    /// to the front. With a `pane`, the window shows that pane; with `None`,
    /// it shows the pane that was visible last (decision 0029, section 17).
    pub fn open_settings(&mut self, pane: Option<SettingsPane>, cx: &mut Context<Self>) {
        if let Some(pane) = pane {
            self.set_settings_pane(pane, cx);
        }
        let open = self
            .settings_window
            .filter(|w| cx.windows().contains(&AnyWindowHandle::from(*w)));
        if let Some(window) = open {
            let _ = window.update(cx, |_, window, _| window.activate_window());
            cx.activate(true);
            return;
        }
        // A new window does not show the message of an earlier window.
        self.settings_notice = None;
        let shell = cx.entity();
        let snapshot = self.settings_snapshot();
        let options = crate::settings_window::window_options(&shell, &snapshot, cx);
        let opened = cx.open_window(options, move |window, cx| {
            let view = cx.new(|cx| SettingsView::new(&shell, snapshot, window, cx));
            cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
        });
        match opened {
            Ok(window) => self.settings_window = Some(window),
            Err(error) => {
                eprintln!("OpenCraft cannot open the Settings window: {error}");
                return;
            }
        }
        cx.activate(true);
    }

    /// Show `pane` in the Settings window, and keep it in `shell.toml` for
    /// the next time that the window opens.
    pub fn set_settings_pane(&mut self, pane: SettingsPane, cx: &mut Context<Self>) {
        if self.settings.settings_pane == pane {
            return;
        }
        self.settings.settings_pane = pane;
        self.save_settings("the pane of the Settings window", cx);
        cx.notify();
    }

    fn set_show_window_at_start(&mut self, show: bool, cx: &mut Context<Self>) {
        self.settings.show_window_at_start = show;
        self.save_settings("the option to show the window at start", cx);
        cx.notify();
    }

    /// Write `shell.toml`. A failure shows as a problem that names `what`.
    fn save_settings(&mut self, what: &str, cx: &mut Context<Self>) {
        if let Err(error) = settings::save(&self.config_dir, &self.settings) {
            self.settings_problem(
                format!(
                    "OpenCraft cannot save {what} in {}: {error}",
                    self.config_dir.join(settings::FILE_NAME).display()
                ),
                cx,
            );
        }
    }

    /// Show the profiles folder, or the log file, in the Finder. A log file
    /// that does not exist yet gives its folder.
    fn show_in_finder(&mut self, place: Place, cx: &mut Context<Self>) {
        let path = match place {
            Place::Profiles => self.profiles_dir.clone(),
            Place::LogFile => {
                let file = self.log_file();
                if file.exists() {
                    file
                } else {
                    self.config_dir.clone()
                }
            }
        };
        if !(self.desktop.show_in_finder)(&path) {
            self.settings_problem(
                format!("OpenCraft cannot show {} in the Finder.", path.display()),
                cx,
            );
        }
    }

    /// Open the pane "Accessibility" of System Settings. For each kind that
    /// waits for the permission, ask the engine to wait for it: the engine
    /// asks the system, so that OpenCraft is in the list of the pane, and it
    /// tells the window when the user gives the permission.
    fn open_accessibility_settings(&mut self, cx: &mut Context<Self>) {
        if !(self.desktop.open_url)(ACCESSIBILITY_PANE) {
            self.settings_problem(
                "OpenCraft cannot open System Settings. Open it, and go to Privacy & Security > \
                 Accessibility."
                    .into(),
                cx,
            );
        }
        for kind in crate::settings_window::kinds_waiting_for_permission(&self.state) {
            (self.send)(Command::Resolve { kind });
        }
    }

    fn set_show_dock_icon(&mut self, show: bool, cx: &mut Context<Self>) {
        self.settings.show_dock_icon = show;
        apply_dock_icon(show, cx);
        if let Err(error) = settings::save(&self.config_dir, &self.settings) {
            self.settings_problem(
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
            self.settings_problem(
                "OpenCraft cannot change the start at login: it cannot find the home directory \
                 or its own executable."
                    .into(),
                cx,
            );
            return;
        };
        let on = self.login_is_on();
        if !on && login::on_read_only_volume(&executable) {
            self.settings_problem(
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
            self.settings_problem(
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
        self.settings_problem(
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
    use opencraft_engine::{DeviceSettings, ProfileEntry, RgbImage, SurfaceContent, SurfaceState};
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

    fn support() -> opencraft_engine::SettingsSupport {
        opencraft_engine::SettingsSupport {
            lowest_brightness: 15,
            knob_lights: 2,
            haptic: true,
        }
    }

    fn profiles() -> Event {
        Event::Profiles {
            profiles: vec![ProfileEntry {
                id: "desk".into(),
                name: "Desk".into(),
                ..Default::default()
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
            states: None,
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
        assert_eq!(menu(cx), MenuState::default());
        f.events.try_send(layout()).unwrap();
        f.events.try_send(profiles()).unwrap();
        f.events
            .try_send(Event::Pages {
                names: vec!["Main".into(), "Mail".into()],
                active: 1,
                first: 0,
            })
            .unwrap();
        f.events
            .try_send(Event::DeviceSettings(DeviceSettings::defaults(support())))
            .unwrap();
        f.events.try_send(Event::Connected).unwrap();
        cx.run_until_parked();
        assert!(cx.update(|cx| cx.windows().is_empty()));
        assert_eq!(
            menu(cx),
            MenuState {
                connected: true,
                device: Some("Test device".into()),
                profiles: vec![ProfileEntry {
                    id: "desk".into(),
                    name: "Desk".into(),
                    ..Default::default()
                }],
                profile: Some("desk".into()),
                pages: vec!["Main".into(), "Mail".into()],
                page: 1,
                // The lowest value is the one of the layout.
                brightness: Some(Brightness {
                    value: 80,
                    lowest: 15,
                }),
            }
        );
        f.events.try_send(Event::Disconnected).unwrap();
        cx.run_until_parked();
        assert!(!menu(cx).connected);
    }

    #[gpui_kit::test]
    fn a_profile_and_a_page_from_the_menu_go_to_the_engine(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.events.try_send(layout()).unwrap();
        f.events
            .try_send(Event::Profiles {
                profiles: vec![
                    ProfileEntry {
                        id: "desk".into(),
                        name: "Desk".into(),
                        ..Default::default()
                    },
                    ProfileEntry {
                        id: "video".into(),
                        name: "Video".into(),
                        ..Default::default()
                    },
                ],
                active: "desk".into(),
            })
            .unwrap();
        f.events
            .try_send(Event::Pages {
                names: vec!["Main".into(), "Mail".into()],
                active: 0,
                first: 0,
            })
            .unwrap();
        cx.run_until_parked();
        // No window is open: the engine operates with no window.
        f.commands
            .try_send(ShellCommand::SelectProfile("video".into()))
            .unwrap();
        f.commands.try_send(ShellCommand::SelectPage(1)).unwrap();
        // The active profile and the active page: nothing to do.
        f.commands
            .try_send(ShellCommand::SelectProfile("desk".into()))
            .unwrap();
        f.commands.try_send(ShellCommand::SelectPage(0)).unwrap();
        cx.run_until_parked();
        assert!(cx.update(|cx| cx.windows().is_empty()));
        assert_eq!(
            *f.sent.borrow(),
            [
                Command::SelectProfile {
                    profile: "video".into()
                },
                Command::SelectPage { page: 1 },
            ]
        );
    }

    #[gpui_kit::test]
    fn the_brightness_from_the_menu_goes_to_the_engine_with_the_other_settings(
        cx: &mut TestAppContext,
    ) {
        let f = shell(cx);
        let mut settings = DeviceSettings::defaults(support());
        settings.haptic = false;
        f.events.try_send(layout()).unwrap();
        f.events
            .try_send(Event::DeviceSettings(settings.clone()))
            .unwrap();
        cx.run_until_parked();
        f.commands
            .try_send(ShellCommand::SetBrightness(40))
            .unwrap();
        cx.run_until_parked();
        settings.brightness = 40;
        assert_eq!(
            *f.sent.borrow(),
            [Command::SetDeviceSettings(settings.clone())]
        );
        // While the interval runs, only the last value waits.
        f.commands
            .try_send(ShellCommand::SetBrightness(50))
            .unwrap();
        f.commands
            .try_send(ShellCommand::SetBrightness(60))
            .unwrap();
        cx.run_until_parked();
        assert_eq!(f.sent.borrow().len(), 1);
        cx.executor().advance_clock(BRIGHTNESS_INTERVAL);
        cx.run_until_parked();
        settings.brightness = 60;
        assert_eq!(
            *f.sent.borrow(),
            [
                Command::SetDeviceSettings(DeviceSettings {
                    brightness: 40,
                    ..settings.clone()
                }),
                Command::SetDeviceSettings(settings.clone()),
            ]
        );
    }

    #[gpui_kit::test]
    fn a_brightness_before_the_device_settings_sends_nothing(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.commands
            .try_send(ShellCommand::SetBrightness(40))
            .unwrap();
        cx.run_until_parked();
        assert!(f.sent.borrow().is_empty());
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
        assert!(f
            .sent
            .borrow()
            .iter()
            .all(|c| matches!(c, Command::WakeDevice)));
        assert_eq!(f.sent.borrow().len(), 2);
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
        assert!(!cx.update(|cx| f.shell.read(cx).settings().show_dock_icon));
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
        assert!(cx.update(|cx| f.shell.read(cx).settings().show_dock_icon));
        assert_eq!(settings::load(f.dir.path()), (Settings::default(), None));
    }

    #[cfg(target_os = "macos")]
    #[gpui_kit::test]
    fn the_start_at_login_option_follows_the_launch_agent_file(cx: &mut TestAppContext) {
        use crate::login;
        let f = shell(cx);
        let agents = f.dir.path().join("LaunchAgents");
        let on = |cx: &mut TestAppContext| cx.update(|cx| f.shell.read(cx).start_at_login());
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
        let on = |cx: &mut TestAppContext| cx.update(|cx| f.shell.read(cx).start_at_login());
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
        assert!(!cx.update(|cx| f.shell.read(cx).start_at_login()));
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
        assert!(!cx.update(|cx| f.shell.read(cx).start_at_login()));
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
        assert!(!cx.update(|cx| f.shell.read(cx).has_notices()));
        let file = notices_file(&bundle(f.dir.path(), true)).unwrap();
        f.shell.update(cx, |shell, _| {
            shell.set_notices(Some(file.clone()), open_and_record)
        });
        assert!(cx.update(|cx| f.shell.read(cx).has_notices()));
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
