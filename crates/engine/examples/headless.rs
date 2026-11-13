//! Run the engine with no window (architecture rule 3).
//!
//! This program is for a test of the engine on a real device. It opens the
//! device: do not run it while a different program operates the device.
//!
//! ```sh
//! cargo run -p opencraft-engine --example headless -- <profiles directory> [--counters]
//! ```
//!
//! With `--counters`, the program prints the input counters each second.
//!
//! The program prints each event. It reads one command from each line of
//! standard input:
//!
//! ```text
//! label key-face-3 Mail
//! label key-face-3
//! color bar-panel-1 #1E6FD9
//! color bar-panel-1
//! image key-face-1 /path/to/file.png
//! image key-face-1
//! undo
//! redo
//! brightness 40
//! haptic off
//! knobs colors
//! knob-brightness 50
//! knob-colors #FF0000 #00FF00 #0000FF #FFFFFF
//! quit
//! ```
//!
//! The device setting commands change one setting, and send all settings to
//! the engine. `knobs` takes `effect`, `colors`, or `off`.

use opencraft_engine::profile::parse_surface_key;
use opencraft_engine::{spawn, Color, Command, Config, DeviceSettings, Event, KnobLightMode};
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Change one device setting in `settings`, from the words `name` and
/// `value`. `None` if the line is not a setting command.
fn change_setting(settings: &mut DeviceSettings, name: &str, value: &str) -> Option<()> {
    let number = || value.parse::<u8>().ok();
    match name {
        "brightness" => settings.brightness = number()?,
        "haptic" => {
            settings.haptic = match value {
                "on" => true,
                "off" => false,
                _ => return None,
            }
        }
        "knobs" => {
            settings.knob_lights.mode = match value {
                "effect" => KnobLightMode::Effect,
                "colors" => KnobLightMode::Colors,
                "off" => KnobLightMode::Off,
                _ => return None,
            }
        }
        "knob-brightness" => settings.knob_lights.brightness = number()?,
        "knob-colors" => {
            settings.knob_lights.colors = value
                .split_whitespace()
                .map(|c| c.parse::<Color>().ok())
                .collect::<Option<Vec<_>>>()?
        }
        _ => return None,
    }
    Some(())
}

fn main() {
    let Some(profiles_dir) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: headless <profiles directory> [--counters]");
        std::process::exit(2);
    };
    let start = Instant::now();
    // The last device settings of the engine. A setting command changes them.
    let settings: Arc<Mutex<Option<DeviceSettings>>> = Arc::default();
    let latest = Arc::clone(&settings);
    let sink = move |event: Event| {
        let at = start.elapsed().as_secs_f64();
        match event {
            Event::Layout(layout) => println!("[{at:8.3}] layout: {}", layout.name),
            Event::Surface(state) => {
                println!("[{at:8.3}] surface: {} {:?}", state.id, state.content)
            }
            Event::Background(state) => {
                println!("[{at:8.3}] background: {:?}", state.background)
            }
            Event::Connected => println!("[{at:8.3}] connected"),
            Event::Disconnected => println!("[{at:8.3}] disconnected"),
            Event::Input(input) => {
                println!("[{at:8.3}] input: {} {:?}", input.control, input.gesture)
            }
            Event::Kinds(kinds) => println!("[{at:8.3}] kinds: {kinds:?}"),
            Event::Profiles { profiles, active } => {
                println!("[{at:8.3}] profiles: {profiles:?}, active {active}")
            }
            Event::Pages {
                names,
                active,
                first,
            } => {
                println!("[{at:8.3}] pages: {names:?}, active {active}, first {first}")
            }
            Event::ProfileExported { profile, path } => {
                println!("[{at:8.3}] exported: {profile} to {}", path.display())
            }
            Event::ProfileImported { profile } => println!("[{at:8.3}] imported: {profile}"),
            Event::Binding(binding) => println!("[{at:8.3}] binding: {binding:?}"),
            Event::History { undo, redo } => {
                println!("[{at:8.3}] history: undo {undo:?}, redo {redo:?}")
            }
            Event::Clipboard(kind) => println!("[{at:8.3}] clipboard: {kind:?}"),
            Event::Status { kind, status } => println!("[{at:8.3}] status: {kind}: {status:?}"),
            Event::Counters(counters) => println!("[{at:8.3}] counters: {counters:?}"),
            Event::Problem(text) => println!("[{at:8.3}] problem: {text}"),
            Event::DeviceSettings(settings) => {
                println!("[{at:8.3}] device settings: {settings:?}");
                *latest.lock().unwrap() = Some(settings);
            }
        }
    };
    let engine =
        spawn(Config { profiles_dir }, Vec::new(), sink).expect("cannot start the engine thread");

    std::thread::scope(|scope| {
        let (stop, receiver) = std::sync::mpsc::channel::<()>();
        if std::env::args().any(|arg| arg == "--counters") {
            let engine = &engine;
            scope.spawn(move || {
                while matches!(
                    receiver.recv_timeout(std::time::Duration::from_secs(1)),
                    Err(RecvTimeoutError::Timeout)
                ) {
                    engine.send(Command::ReadCounters);
                }
            });
        }
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            let mut parts = line.trim().splitn(3, ' ');
            let (name, surface, value) = (parts.next().unwrap_or(""), parts.next(), parts.next());
            if name == "quit" {
                break;
            }
            if let Some(command) = match name {
                "undo" => Some(Command::Undo),
                "redo" => Some(Command::Redo),
                _ => None,
            } {
                engine.send(command);
                continue;
            }
            let rest = line.trim().split_once(' ').map_or("", |(_, rest)| rest);
            let current = settings.lock().unwrap().clone();
            if let Some(mut current) = current {
                if change_setting(&mut current, name, rest).is_some() {
                    // The next line changes these settings, also before
                    // the event of the engine comes.
                    *settings.lock().unwrap() = Some(current.clone());
                    engine.send(Command::SetDeviceSettings(current));
                    continue;
                }
            }
            let Some(surface) = surface.and_then(parse_surface_key) else {
                eprintln!("not a command: {line}");
                continue;
            };
            let command = match name {
                "label" => Command::SetLabel {
                    surface,
                    label: value.map(str::to_string),
                },
                "color" => match value.map(str::parse) {
                    Some(Err(e)) => {
                        eprintln!("{e}");
                        continue;
                    }
                    Some(Ok(color)) => Command::SetColor {
                        surface,
                        color: Some(color),
                    },
                    None => Command::SetColor {
                        surface,
                        color: None,
                    },
                },
                "image" => Command::SetImage {
                    surface,
                    path: value.map(PathBuf::from),
                },
                _ => {
                    eprintln!("not a command: {line}");
                    continue;
                }
            };
            engine.send(command);
        }
        let _ = stop.send(());
    });
    engine.stop();
}
