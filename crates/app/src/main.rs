//! The OpenCraft shell. Only the engine opens the device and writes the
//! profiles. The shell writes only its own file `shell.toml` (decision 0020).
//!
//! The engine and the shell operate in one process, also when no window is
//! open (decision 0019). The menu bar item opens the window again and quits
//! the application (decision 0020).

mod images;
#[cfg(unix)]
mod lock;
mod menu;
#[cfg(target_os = "macos")]
mod menu_bar;
mod settings;
mod shell;
#[cfg(unix)]
mod signals;
mod state;
mod view;

use gpui_kit::{actions, prelude::*, App, KeyBinding, Menu, MenuItem, QuitMode};
use opencraft_engine::{Command, Config, EngineHandle, Event};
use shell::{Shell, ShellCommand};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

actions!(opencraft, [Quit]);

/// The shell entity lives until the process stops.
struct ShellHandle(#[allow(dead_code)] gpui_kit::Entity<Shell>);
impl gpui_kit::Global for ShellHandle {}

fn profiles_dir() -> PathBuf {
    std::env::var_os("OPENCRAFT_PROFILES_DIR")
        .map(PathBuf::from)
        .or_else(opencraft_engine::default_profiles_dir)
        .expect("OpenCraft cannot find the profiles directory.")
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

/// Stop if a different OpenCraft process runs (decision 0019). The caller
/// keeps the result until the process stops.
#[cfg(unix)]
fn lock_or_stop(config_dir: &Path) -> Option<std::fs::File> {
    match lock::acquire(config_dir) {
        Ok(Some(file)) => Some(file),
        Ok(None) => {
            eprintln!("OpenCraft is already running.");
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

/// The application menu, and Cmd-Q. Both start the quit path (decision 0020).
fn add_application_menu(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    cx.set_menus([Menu::new("OpenCraft").items([MenuItem::action("Quit OpenCraft", Quit)])]);
}

fn main() {
    if start_trace() {
        trace_now("main");
    }
    // All quit requests go to one path: the commands of the shell.
    let (commands, command_receiver) = async_channel::unbounded();
    #[cfg(unix)]
    if let Err(error) = signals::forward(commands.clone()) {
        eprintln!("OpenCraft cannot set the signal handlers: {error}");
    }
    let profiles_dir = profiles_dir();
    let config_dir = settings::config_dir(&profiles_dir);
    #[cfg(unix)]
    let _lock = lock_or_stop(&config_dir);
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
    let mut application = gpui_kit::platform::application().with_quit_mode(QuitMode::Explicit);
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
        add_application_menu(cx);
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
        shell.update(cx, |shell, cx| shell.open_window(cx));
        cx.set_global(ShellHandle(shell));
    });
    if let Some(engine) = fallback.borrow_mut().take() {
        engine.stop();
    };
}
