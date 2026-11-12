//! Count taps and swipes through the engine, for the question "do tap and
//! swipe operate reliably?".
//!
//! This program opens the device: do not run it while a different program
//! operates the device.
//!
//! ```sh
//! cargo run --release -p opencraft-engine --example touch_count
//! ```
//!
//! The program binds the tap of each bar panel and the two swipes of the
//! touch bar to an action kind of its own, in a profile in a temporary
//! directory. It shows three steps, one after the other. A step ends 4
//! seconds after its last touch. For each step the program counts:
//!
//! - the touches: each start of a touch that the shell got;
//! - the gesture that the device gave at the end of each touch;
//! - the Invoke requests that the handler got.
//!
//! The result is in `docs/kb/n4-pro-hardware.md`.

use opencraft_engine::{
    spawn, Action, BindingId, Command, Config, ControlId, ControlKind, Event, Gesture, GestureKind,
    Handler, KindDescription, MessageSender, Request, Settings, SurfaceId, SurfaceKind,
};
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

const KIND: &str = "example.count";
const QUIET: Duration = Duration::from_secs(4);
/// The program stops if no touch comes in this time.
const NO_INPUT: Duration = Duration::from_secs(120);

const STEPS: [&str; 3] = [
    "Tap each bar panel 5 times, from left to right (20 taps).",
    "Swipe to the left 10 times, at different places.",
    "Swipe to the right 10 times, at different places.",
];

enum Note {
    Connected,
    Input(ControlId, Gesture),
    Invoke(ControlId, GestureKind),
    Problem(String),
}

struct CountHandler {
    notes: Sender<Note>,
}

impl Handler for CountHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![KindDescription {
            kind: KIND.into(),
            name: "Count".into(),
            fields: Vec::new(),
        }]
    }

    fn start(&mut self, _: MessageSender) {}

    fn handle(&mut self, request: Request) {
        if let Request::Invoke { binding, .. } = request {
            let _ = self
                .notes
                .send(Note::Invoke(binding.control, binding.gesture));
        }
    }
}

/// The counts of one step.
#[derive(Default)]
struct Counts {
    touches: usize,
    /// The gestures that the device gave, by their name and control.
    gestures: BTreeMap<String, usize>,
    /// The ends of a touch with no identified gesture.
    not_identified: usize,
    invokes: usize,
}

impl Counts {
    fn identified(&self) -> usize {
        self.gestures.values().sum()
    }
}

/// Count one step. Return `None` if no touch came.
fn count_step(receiver: &Receiver<Note>) -> Option<Counts> {
    let mut counts = Counts::default();
    let started = Instant::now();
    let mut last_touch: Option<Instant> = None;
    loop {
        match receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(Note::Input(control, gesture)) => {
                last_touch = Some(Instant::now());
                match gesture {
                    Gesture::TouchStart { .. } => counts.touches += 1,
                    Gesture::TouchEnd => counts.not_identified += 1,
                    Gesture::Tap { .. } | Gesture::SwipeLeft | Gesture::SwipeRight => {
                        let name = format!("{} on {control}", gesture.name());
                        *counts.gestures.entry(name).or_default() += 1;
                    }
                    _ => {}
                }
            }
            Ok(Note::Invoke(..)) => counts.invokes += 1,
            Ok(Note::Problem(text)) => println!("problem: {text}"),
            Ok(Note::Connected) | Err(_) => {}
        }
        if last_touch.is_some_and(|at| at.elapsed() >= QUIET) {
            return Some(counts);
        }
        if last_touch.is_none() && started.elapsed() >= NO_INPUT {
            return None;
        }
    }
}

fn main() {
    let profiles_dir =
        std::env::temp_dir().join(format!("opencraft-touch-count-{}", std::process::id()));
    let (notes, receiver) = mpsc::channel();
    let handler = CountHandler {
        notes: notes.clone(),
    };
    let sink = move |event: Event| {
        let note = match event {
            Event::Connected => Note::Connected,
            Event::Input(input)
                if matches!(
                    input.control.kind,
                    ControlKind::BarPanel | ControlKind::TouchBar
                ) =>
            {
                Note::Input(input.control, input.gesture)
            }
            Event::Problem(text) => Note::Problem(text),
            _ => return,
        };
        let _ = notes.send(note);
    };
    let engine = spawn(
        Config {
            profiles_dir: profiles_dir.clone(),
        },
        vec![Box::new(handler)],
        sink,
    )
    .expect("cannot start the engine thread");
    let action = || {
        Some(Action {
            kind: KIND.into(),
            settings: Settings::new(),
        })
    };
    for number in 1..=4 {
        engine.send(Command::SetLabel {
            surface: SurfaceId::new(SurfaceKind::BarPanel, number),
            label: Some(number.to_string()),
        });
        engine.send(Command::SetBinding {
            binding: BindingId {
                page: 0,
                control: ControlId::new(ControlKind::BarPanel, number),
                gesture: GestureKind::Tap,
            },
            action: action(),
        });
    }
    for gesture in [GestureKind::SwipeLeft, GestureKind::SwipeRight] {
        engine.send(Command::SetBinding {
            binding: BindingId {
                page: 0,
                control: ControlId::new(ControlKind::TouchBar, 1),
                gesture,
            },
            action: action(),
        });
    }

    println!("Waiting for the device...");
    while !matches!(receiver.recv(), Ok(Note::Connected)) {}

    let mut fault = false;
    let mut results = Vec::new();
    for (number, text) in STEPS.iter().enumerate() {
        println!();
        println!("Step {} of {}: {text}", number + 1, STEPS.len());
        println!("The step ends 4 seconds after your last touch.");
        let Some(counts) = count_step(&receiver) else {
            println!("No touch came in {} seconds.", NO_INPUT.as_secs());
            fault = true;
            break;
        };
        println!("  Got {} touches.", counts.touches);
        results.push((text, counts));
    }
    engine.stop();
    let _ = std::fs::remove_dir_all(&profiles_dir);

    println!();
    for (text, counts) in &results {
        println!("{text}");
        println!("  Touches:                          {}", counts.touches);
        for (name, count) in &counts.gestures {
            println!("  {name:<33} {count}");
        }
        println!(
            "  Touches with no gesture:          {}",
            counts.not_identified
        );
        println!("  Invoke requests at the handler:   {}", counts.invokes);
        // Each gesture that the device identified must give one Invoke.
        if counts.invokes != counts.identified() {
            fault = true;
        }
    }
    if fault {
        println!("RESULT: FAULT. The engine did not send one Invoke for each gesture.");
        std::process::exit(1);
    }
    println!("RESULT: the engine sent one Invoke for each gesture that the device identified.");
}
