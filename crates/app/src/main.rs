//! The OpenCraft shell. Only the engine opens the device and writes the
//! profiles. The shell writes only its own files: `shell.toml` (decisions
//! 0020 and 0029), the lock file `opencraft.lock` with the process number
//! (decisions 0019 and 0028), the log file when the process has no terminal
//! (decision 0024), the socket of the control by other programs while the
//! setting is on (decision 0040), and on macOS the LaunchAgent file of the
//! start at login (decision 0021).
//!
//! The client `opencraft ctl` writes one other file: the skill, into the
//! directory that `ctl skill --install` names (decision 0040, section 5).
//!
//! The server `opencraft mcp` serves the tools of the Model Context Protocol
//! for an agent. It is a second client of the same socket (decision 0040,
//! section 6). It writes no file, and it takes no lock.
//!
//! The engine and the shell operate in one process, also when no window is
//! open (decision 0019). The menu bar item opens the window again and quits
//! the application (decision 0020).
//!
//! The argument `--login` starts the application with no window, for the
//! start at login (decision 0021).

mod app_menu;
mod appearance;
mod applications;
mod control;
#[cfg(unix)]
mod ctl;
mod diagnostics;
mod first_start;
#[cfg(target_os = "macos")]
mod front_app;
mod images;
#[cfg(unix)]
mod lock;
#[cfg(unix)]
mod log_file;
#[cfg(target_os = "macos")]
mod login;
#[cfg(unix)]
mod mcp;
mod menu;
#[cfg(target_os = "macos")]
mod menu_bar;
mod pasteboard;
#[cfg(target_os = "macos")]
mod power;
mod ready;
mod settings;
mod settings_window;
mod shell;
#[cfg(unix)]
mod signals;
mod starters;
mod state;
mod view;

use gpui_kit::{prelude::*, QuitMode};
use opencraft_engine::{Command, Config, EngineHandle, Event};
use shell::{Shell, ShellCommand};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

/// The shell entity lives until the process stops.
struct ShellHandle(#[allow(dead_code)] gpui_kit::Entity<Shell>);
impl gpui_kit::Global for ShellHandle {}

/// The profiles directory of the environment variable, or the default.
fn profiles_dir_if_known() -> Option<PathBuf> {
    std::env::var_os("OPENCRAFT_PROFILES_DIR")
        .map(PathBuf::from)
        .or_else(opencraft_engine::default_profiles_dir)
}

fn profiles_dir() -> PathBuf {
    profiles_dir_if_known().expect("OpenCraft cannot find the profiles directory.")
}

/// The argument that makes the executable a command-line client
/// (decision 0040, section 4). It must be the first argument.
const CTL_ARGUMENT: &str = "ctl";

/// The argument that makes the executable an MCP server for an agent
/// (decision 0040, section 6). It must be the first argument.
const MCP_ARGUMENT: &str = "mcp";

/// The argument of a start at login (decision 0021).
const LOGIN_ARGUMENT: &str = "--login";

/// What the first argument asks for, before the application starts.
enum Start {
    /// Start the application.
    Application,
    /// Write the short help, with the version.
    Help,
    /// Write the version.
    Version,
    /// The first word is not a command: write the problem.
    Unknown(std::ffi::OsString),
}

/// Read the first argument. `opencraft` starts the application, and
/// `opencraft ctl <command>` is the client and `opencraft mcp` is the server
/// for an agent; `main` starts both before this function. `--help`, `-h`,
/// `help`, and `--version` must not start the application: an agent or a
/// person that tries the new command in `PATH` must get the text, not a
/// started OpenCraft with the device and the lock. An argument that the
/// Finder gives (`-psn_…`) starts the application, as before.
fn start_of(arguments: &[std::ffi::OsString]) -> Start {
    let Some(first) = arguments.first() else {
        return Start::Application;
    };
    match first.to_str() {
        Some(LOGIN_ARGUMENT) | Some(CTL_ARGUMENT) | Some(MCP_ARGUMENT) => Start::Application,
        Some("--help" | "-h" | "help") => Start::Help,
        Some("--version") => Start::Version,
        Some(word) if word.starts_with('-') => Start::Application,
        // The first word of the client is `ctl`: a different first word is a
        // mistake, and it must not start the application.
        Some(_) => Start::Unknown(first.clone()),
        None => Start::Application,
    }
}

/// The short help of `opencraft`. It names the application, the client, and
/// the server for an agent, because the same executable is all three
/// (decision 0040, sections 4 and 6).
fn help_text() -> String {
    format!(
        "OpenCraft {version}\n\n\
         `opencraft` starts the OpenCraft application for the device.\n\
         `opencraft ctl <command>` changes a running OpenCraft: `opencraft ctl --help`.\n\
         `opencraft ctl skill` writes the skill for an agent.\n\
         `opencraft mcp` serves the tools of the Model Context Protocol for an agent.\n",
        version = env!("CARGO_PKG_VERSION"),
    )
}

/// True if the arguments, with no program name, contain `--login`. The
/// function prints each other argument and ignores it: an unknown argument
/// does not stop the start.
fn login_start(arguments: impl IntoIterator<Item = std::ffi::OsString>) -> bool {
    let mut login = false;
    for argument in arguments {
        if argument == LOGIN_ARGUMENT {
            login = true;
        } else {
            eprintln!("OpenCraft ignores the argument {argument:?}.");
        }
    }
    login
}

/// True if the start opens the window. A start at login opens no window
/// (decision 0021). The user can turn off the window for the other starts
/// (decision 0029, section 17).
#[cfg(target_os = "macos")]
fn opens_window(login: bool, settings: &settings::Settings) -> bool {
    !login && settings.show_window_at_start
}

/// True if the environment variable `OPENCRAFT_NO_DEVICE` is set. Then the
/// engine does not look for a device, and the window shows "not connected",
/// for a check of the window with no device while a device is connected.
fn no_device() -> bool {
    std::env::var_os("OPENCRAFT_NO_DEVICE").is_some()
}

/// Start the device before the GUI toolkit. The sink never waits for the GUI.
fn start_engine(
    profiles_dir: PathBuf,
    feed: control::Feed,
) -> (EngineHandle, async_channel::Receiver<Event>) {
    let (sender, receiver) = async_channel::unbounded();
    let trace = start_trace();
    let engine = opencraft_engine::spawn(
        Config {
            profiles_dir,
            no_device: no_device(),
        },
        opencraft_actions::handlers(),
        move |event| {
            if trace && matches!(event, Event::Connected) {
                trace_now("device ready");
            }
            // The cache of the control socket follows the engine, in the order
            // of the events and of the results of control requests.
            feed.observe(&event);
            let _ = sender.try_send(event);
        },
    )
    .expect("OpenCraft cannot start the engine.");
    (engine, receiver)
}

/// Stop if a different OpenCraft process runs (decisions 0019 and 0028).
/// Before the stop, ask that process to open its window, except for a start
/// at login. The caller keeps the result until the process stops.
#[cfg(unix)]
fn lock_or_stop(config_dir: &Path, login: bool) -> Option<std::fs::File> {
    match lock::acquire(config_dir) {
        Ok(Some(file)) => Some(file),
        Ok(None) => {
            let holder = lock::holder(config_dir).filter(|_| !login);
            // SAFETY: `kill` only sends a signal. The process is OpenCraft,
            // which has a handler for this signal.
            let asked =
                holder.is_some_and(|pid| unsafe { libc::kill(pid, signals::OPEN_SIGNAL) } == 0);
            if asked {
                eprintln!("OpenCraft is already running. It shows its window now.");
            } else {
                eprintln!(
                    "OpenCraft is already running. To open its window, click the OpenCraft \
                     item in the menu bar, and then Open OpenCraft."
                );
            }
            std::process::exit(0);
        }
        Err(error) => {
            // The device layer opens the device for exclusive use, thus a
            // second process cannot take the device from this one.
            eprintln!(
                "OpenCraft cannot lock {}: {error}. It continues with no lock.",
                config_dir.join(lock::FILE_NAME).display()
            );
            None
        }
    }
}

/// True if the environment variable `OPENCRAFT_START_TRACE` is set. Then the
/// application prints the time of each step of its start, for a measurement
/// of the cold start. See `docs/kb/footprint.md`.
fn start_trace() -> bool {
    std::env::var_os("OPENCRAFT_START_TRACE").is_some()
}

/// Print `step` with the time of the system clock, in milliseconds.
fn trace_now(step: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    eprintln!("start-trace {step} {:.3}", now.as_secs_f64() * 1000.0);
}

fn main() {
    // The arguments that ask for a text must not start the application. Do
    // this first, with no lock, no engine, and no device (decision 0040).
    let given: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    match start_of(&given) {
        Start::Application => {}
        Start::Help => {
            println!("{}", help_text());
            std::process::exit(0);
        }
        Start::Version => {
            println!("opencraft {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        Start::Unknown(word) => {
            eprintln!(
                "OpenCraft does not know the command {word:?}. `opencraft` starts the \
                 application; `opencraft ctl --help` shows the commands of the client."
            );
            std::process::exit(2);
        }
    }
    // Before everything else: the client and the server for an agent take no
    // lock, start no engine, and write their messages to the terminal, not to
    // the log file.
    let mut arguments = std::env::args_os().skip(1);
    match arguments.next().as_deref() {
        Some(argument) if argument == CTL_ARGUMENT => {
            #[cfg(unix)]
            std::process::exit(ctl::run(arguments.collect(), profiles_dir_if_known()));
            #[cfg(not(unix))]
            {
                eprintln!("`opencraft ctl` needs a Unix socket. This system has none.");
                std::process::exit(3);
            }
        }
        Some(argument) if argument == MCP_ARGUMENT => {
            #[cfg(unix)]
            std::process::exit(mcp::run(arguments.collect(), profiles_dir_if_known()));
            #[cfg(not(unix))]
            {
                eprintln!("`opencraft mcp` needs a Unix socket. This system has none.");
                std::process::exit(3);
            }
        }
        _ => {}
    }
    drop(arguments);
    if start_trace() {
        trace_now("main");
    }
    let profiles_dir = profiles_dir();
    let config_dir = settings::config_dir(&profiles_dir);
    let login = login_start(std::env::args_os().skip(1));
    // All quit requests go to one path: the commands of the shell. The
    // signal handlers come before the lock: the lock file then gets the
    // number of this process, and a second start can send the signal that
    // opens the window at once.
    let (commands, command_receiver) = async_channel::unbounded();
    #[cfg(unix)]
    if let Err(error) = signals::forward(commands.clone()) {
        eprintln!("OpenCraft cannot set the signal handlers: {error}");
    }
    #[cfg(unix)]
    let _lock = lock_or_stop(&config_dir, login);
    #[cfg(unix)]
    if _lock.is_some() {
        if let Err(error) = control::remove_stale(&config_dir) {
            eprintln!("OpenCraft cannot remove the old control socket: {error}");
        }
    }
    // After the lock, so that only one process writes and renames the log
    // file. A start through launchd or the Finder has no terminal, and the
    // messages then go to the log file.
    #[cfg(unix)]
    {
        if let Err(error) = log_file::use_if_no_output(&config_dir) {
            eprintln!("OpenCraft cannot open its log file: {error}");
        }
        log_file::write_start_line();
    }
    if login {
        eprintln!(
            "OpenCraft {LOGIN_ARGUMENT}: Accessibility permission: {}",
            if opencraft_actions::system::can_send_keys() {
                "given"
            } else {
                "not given"
            }
        );
    }
    let (settings, settings_problem) = settings::load(&config_dir);
    // Only macOS has a menu bar item that opens the window later. A start at
    // login opens no window (decision 0021). The user can also turn off the
    // window for the other starts (decision 0029, section 17).
    #[cfg(target_os = "macos")]
    let open_window = opens_window(login, &settings);
    #[cfg(not(target_os = "macos"))]
    let open_window = {
        if login {
            eprintln!("OpenCraft has no menu bar item here, thus it opens the window.");
        }
        true
    };
    if !open_window {
        if login {
            eprintln!("OpenCraft starts with {LOGIN_ARGUMENT}. It opens no window.");
        } else {
            eprintln!("OpenCraft opens no window: the option to show the window at start is off.");
        }
    }
    // Before the engine starts: with no profile file, the window is the
    // first start window (decision 0029, section 18).
    let first_start = !opencraft_engine::ProfilesDir::new(&profiles_dir).has_profiles();
    let control_feed = control::Feed::default();
    let (engine, events) = start_engine(profiles_dir.clone(), control_feed.clone());
    let control_port = control::port(engine.sender());
    // Keep ownership outside the views. The quit hook stops the engine; code
    // after the run loop is a fallback on platforms where it returns.
    let engine = Rc::new(RefCell::new(Some(engine)));
    let shutdown = engine.clone();
    let fallback = engine.clone();
    let send: Rc<dyn Fn(Command)> = Rc::new(move |command| {
        if let Some(engine) = engine.borrow().as_ref() {
            engine.send(command);
        }
    });

    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    // The window embeds only the icons that it uses (decision 0029, section
    // 3.6), and it shows the icons of the engine (decision 0033).
    let mut application = gpui_kit::platform::application()
        .with_assets(view::theme::WindowAssets)
        .with_quit_mode(QuitMode::Explicit);
    // With the Dock icon hidden, the icon must not show during the start.
    // On Linux, "headless" means no display server, thus macOS only.
    #[cfg(target_os = "macos")]
    if !settings.show_dock_icon {
        application = application.with_windowing(gpui_kit::WindowingRequest::Headless);
    }
    #[cfg(target_os = "macos")]
    let _power_notices = power::Notices::new(commands.clone());
    // After the engine starts, so the first report has an engine to reach.
    #[cfg(target_os = "macos")]
    let _front_application = front_app::Notices::new(commands.clone());
    let reopen = commands.clone();
    // A click on the Dock icon when no window is open.
    application.on_reopen(move |_| {
        let _ = reopen.try_send(ShellCommand::Open);
    });
    application.run(move |cx| {
        gpui_kit::init(cx);
        // Each quit request calls `App::quit`, and a terminate from macOS
        // also comes here. Stop the engine in the closure itself: GPUI
        // waits for the future for at most 200 ms, and the stop must
        // complete. The main thread waits for the engine.
        cx.on_app_quit(move |cx| {
            if let Some(shell) = cx.try_global::<ShellHandle>().map(|h| h.0.clone()) {
                shell.update(cx, |shell, _| shell.stop_control());
            }
            if let Some(engine) = shutdown.borrow_mut().take() {
                engine.stop();
                eprintln!("OpenCraft stopped the engine, and quits.");
            }
            async {}
        })
        .detach();
        // The appearance of Settings > General, and the theme of the
        // controls of the kit, before a window shows (decision 0029,
        // section 21).
        appearance::apply(settings.appearance, cx);
        app_menu::install(cx, &settings.panes);
        let shell = cx.new(|cx| {
            let mut shell = Shell::new(config_dir, settings, send);
            shell.set_profiles_dir(profiles_dir);
            shell.set_control(control_feed, control_port);
            shell.set_first_start(first_start);
            shell.listen(events, command_receiver, cx);
            #[cfg(target_os = "macos")]
            shell.add_menu_bar(commands);
            #[cfg(not(target_os = "macos"))]
            drop(commands);
            if let Some(problem) = settings_problem {
                shell.problem(problem, cx);
            }
            shell
        });
        if open_window {
            shell.update(cx, |shell, cx| shell.open_window(cx));
        }
        cx.set_global(ShellHandle(shell));
    });
    if let Some(engine) = fallback.borrow_mut().take() {
        engine.stop();
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn arguments(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_window_opens_at_a_start_unless_the_option_or_the_login_stops_it() {
        let mut settings = settings::Settings::default();
        assert!(opens_window(false, &settings));
        assert!(!opens_window(true, &settings));
        settings.show_window_at_start = false;
        assert!(!opens_window(false, &settings));
        assert!(!opens_window(true, &settings));
    }

    #[test]
    fn the_arguments_that_ask_for_a_text_do_not_start_the_application() {
        let start = |list: &[&str]| match start_of(&arguments(list)) {
            Start::Application => "start",
            Start::Help => "help",
            Start::Version => "version",
            Start::Unknown(_) => "unknown",
        };
        assert_eq!(start(&[]), "start");
        assert_eq!(start(&["--login"]), "start");
        assert_eq!(start(&["ctl", "state"]), "start");
        assert_eq!(start(&["mcp"]), "start");
        assert_eq!(start(&["mcp", "--config", "claude"]), "start");
        // The Finder gives an argument of this form.
        assert_eq!(start(&["-psn_0_12345"]), "start");
        assert_eq!(start(&["--help"]), "help");
        assert_eq!(start(&["-h"]), "help");
        assert_eq!(start(&["help"]), "help");
        assert_eq!(start(&["--version"]), "version");
        assert_eq!(start(&["clt", "state"]), "unknown");
        assert_eq!(start(&["state"]), "unknown");
    }

    #[test]
    fn the_login_argument_is_found_and_other_arguments_are_ignored() {
        assert!(!login_start(arguments(&[])));
        assert!(login_start(arguments(&["--login"])));
        assert!(login_start(arguments(&["-psn_0_12345", "--login", "x"])));
        assert!(!login_start(arguments(&[
            "--unknown",
            "login",
            "--login=1"
        ])));
        #[cfg(target_os = "macos")]
        assert_eq!(LOGIN_ARGUMENT, login::ARGUMENT);
    }
}
