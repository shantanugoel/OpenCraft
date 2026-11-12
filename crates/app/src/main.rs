//! The OpenCraft shell. Only the engine opens the device and writes the
//! profiles. The shell writes only its own files: `shell.toml` (decisions
//! 0020 and 0029), the lock file `opencraft.lock` with the process number
//! (decisions 0019 and 0028), the log file when the process has no terminal
//! (decision 0024), and on macOS the LaunchAgent file of the start at login
//! (decision 0021).
//!
//! The engine and the shell operate in one process, also when no window is
//! open (decision 0019). The menu bar item opens the window again and quits
//! the application (decision 0020).
//!
//! The argument `--login` starts the application with no window, for the
//! start at login (decision 0021).

mod app_menu;
mod images;
#[cfg(unix)]
mod lock;
#[cfg(unix)]
mod log_file;
#[cfg(target_os = "macos")]
mod login;
mod menu;
#[cfg(target_os = "macos")]
mod menu_bar;
mod settings;
mod shell;
#[cfg(unix)]
mod signals;
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

fn profiles_dir() -> PathBuf {
    std::env::var_os("OPENCRAFT_PROFILES_DIR")
        .map(PathBuf::from)
        .or_else(opencraft_engine::default_profiles_dir)
        .expect("OpenCraft cannot find the profiles directory.")
}

/// The argument of a start at login (decision 0021).
const LOGIN_ARGUMENT: &str = "--login";

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

/// Start the device before the GUI toolkit. The sink never waits for the GUI.
fn start_engine(profiles_dir: PathBuf) -> (EngineHandle, async_channel::Receiver<Event>) {
    let (sender, receiver) = async_channel::unbounded();
    let trace = start_trace();
    let engine = opencraft_engine::spawn(
        Config { profiles_dir },
        opencraft_actions::handlers(),
        move |event| {
            if trace && matches!(event, Event::Connected) {
                trace_now("device ready");
            }
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
    // Only macOS has a menu bar item that opens the window later.
    #[cfg(target_os = "macos")]
    let open_window = !login;
    #[cfg(not(target_os = "macos"))]
    let open_window = {
        if login {
            eprintln!("OpenCraft has no menu bar item here, thus it opens the window.");
        }
        true
    };
    if !open_window {
        eprintln!("OpenCraft starts with {LOGIN_ARGUMENT}. It opens no window.");
    }
    let (settings, settings_problem) = settings::load(&config_dir);
    let (engine, events) = start_engine(profiles_dir);
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
    // The window embeds only the icons that it uses (decision 0029, section 3.6).
    let mut application = gpui_kit::platform::application()
        .with_assets(view::theme::WindowIcons)
        .with_quit_mode(QuitMode::Explicit);
    // With the Dock icon hidden, the icon must not show during the start.
    // On Linux, "headless" means no display server, thus macOS only.
    #[cfg(target_os = "macos")]
    if !settings.show_dock_icon {
        application = application.with_windowing(gpui_kit::WindowingRequest::Headless);
    }
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
        cx.on_app_quit(move |_| {
            if let Some(engine) = shutdown.borrow_mut().take() {
                engine.stop();
                eprintln!("OpenCraft stopped the engine, and quits.");
            }
            async {}
        })
        .detach();
        gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);
        app_menu::install(cx, &settings.panes);
        let shell = cx.new(|cx| {
            let mut shell = Shell::new(config_dir, settings, send);
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
