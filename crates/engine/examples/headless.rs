//! Run the engine with no window (architecture rule 3).
//!
//! This program is for a test of the engine on a real device. It opens the
//! device: do not run it while a different program operates the device.
//!
//! ```sh
//! cargo run -p opencraft-engine --example headless -- <profile directory>
//! ```
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
//! quit
//! ```

use opencraft_engine::profile::parse_surface_key;
use opencraft_engine::{spawn, Command, Config, Event};
use std::io::BufRead;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let Some(profile_dir) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: headless <profile directory>");
        std::process::exit(2);
    };
    let start = Instant::now();
    let sink = move |event: Event| {
        let at = start.elapsed().as_secs_f64();
        match event {
            Event::Layout(layout) => println!("[{at:8.3}] layout: {}", layout.name),
            Event::Surface(state) => {
                println!("[{at:8.3}] surface: {} {:?}", state.id, state.content)
            }
            Event::Connected => println!("[{at:8.3}] connected"),
            Event::Disconnected => println!("[{at:8.3}] disconnected"),
            Event::Input(input) => {
                println!("[{at:8.3}] input: {} {:?}", input.control, input.gesture)
            }
            Event::Problem(text) => println!("[{at:8.3}] problem: {text}"),
        }
    };
    let engine = spawn(Config { profile_dir }, sink).expect("cannot start the engine thread");

    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let mut parts = line.trim().splitn(3, ' ');
        let (name, surface, value) = (parts.next().unwrap_or(""), parts.next(), parts.next());
        if name == "quit" {
            break;
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
    engine.stop();
}
