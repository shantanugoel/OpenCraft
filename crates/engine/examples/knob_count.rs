//! Count the input during a fast knob turn, for the question "does the
//! engine lose an input event?".
//!
//! This program opens the device: do not run it while a different program
//! operates the device.
//!
//! ```sh
//! cargo run --release -p opencraft-engine --example knob_count
//! ```
//!
//! The program binds each gesture of each knob to an action kind of its own,
//! in a profile in a temporary directory. Turn the knobs fast, in the two
//! directions. The program stops 3 seconds after the last input. Then it
//! compares three records:
//!
//! - the counters of the engine: reports, input events, and Invoke requests;
//! - the input events of the knobs that the shell got, in order;
//! - the Invoke requests that the handler got, in order.
//!
//! The result is in `docs/kb/n4-pro-hardware.md`.

use opencraft_engine::{
    spawn, Action, BindingId, Command, Config, ControlId, ControlKind, Counters, Event,
    GestureKind, Handler, KindDescription, MessageSender, Request, Settings,
};
use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant};

const KIND: &str = "example.count";
const QUIET: Duration = Duration::from_secs(3);
/// The program stops if no input comes in this time.
const NO_INPUT: Duration = Duration::from_secs(120);

/// One knob gesture, with the time at which a thread got it.
type Record = (ControlId, GestureKind, Instant);

enum Note {
    Connected,
    Input(Record),
    Invoke(Record),
    Counters(Counters),
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
            ..Default::default()
        }]
    }

    fn start(&mut self, _: MessageSender) {}

    fn handle(&mut self, request: Request) {
        if let Request::Invoke { binding, .. } = request {
            let record = (binding.control, binding.gesture, Instant::now());
            let _ = self.notes.send(Note::Invoke(record));
        }
    }
}

fn main() {
    let profiles_dir =
        std::env::temp_dir().join(format!("opencraft-knob-count-{}", std::process::id()));
    let (notes, receiver) = mpsc::channel();
    let handler = CountHandler {
        notes: notes.clone(),
    };
    let sink = move |event: Event| {
        let note = match event {
            Event::Connected => Note::Connected,
            Event::Input(input) if input.control.kind == ControlKind::Knob => {
                match GestureKind::from_gesture(input.gesture) {
                    Some(gesture) => Note::Input((input.control, gesture, Instant::now())),
                    None => return,
                }
            }
            Event::Counters(counters) => Note::Counters(counters),
            Event::Problem(text) => Note::Problem(text),
            _ => return,
        };
        let _ = notes.send(note);
    };
    let engine = spawn(
        Config {
            profiles_dir: profiles_dir.clone(),
            no_device: false,
        },
        vec![Box::new(handler)],
        sink,
    )
    .expect("cannot start the engine thread");
    for number in 1..=4 {
        for gesture in [
            GestureKind::TurnLeft,
            GestureKind::TurnRight,
            GestureKind::Push,
        ] {
            engine.send(Command::SetBinding {
                binding: BindingId {
                    page: 0,
                    control: ControlId::new(ControlKind::Knob, number),
                    gesture,
                },
                action: Some(Action {
                    kind: KIND.into(),
                    settings: Settings::new(),
                }),
            });
        }
    }

    println!("Waiting for the device...");
    let mut inputs: Vec<Record> = Vec::new();
    let mut invokes: Vec<Record> = Vec::new();
    let mut last_input: Option<Instant> = None;
    let mut printed = 0;
    let started = Instant::now();
    loop {
        match receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(Note::Connected) => {
                println!("Turn the knobs fast, in the two directions, for 10 seconds or more.");
                println!("The program stops 3 seconds after the last turn.");
            }
            Ok(Note::Input(record)) => {
                last_input = Some(record.2);
                inputs.push(record);
            }
            Ok(Note::Invoke(record)) => invokes.push(record),
            Ok(Note::Problem(text)) => println!("problem: {text}"),
            Ok(Note::Counters(_)) | Err(_) => {}
        }
        if inputs.len() >= printed + 50 {
            printed = inputs.len() / 50 * 50;
            println!("  {printed} input events...");
        }
        if last_input.is_some_and(|at| at.elapsed() >= QUIET) {
            break;
        }
        if last_input.is_none() && started.elapsed() >= NO_INPUT {
            println!("No knob input came in {} seconds.", NO_INPUT.as_secs());
            break;
        }
    }

    // The handler has its own thread. Give it time to get the last request.
    engine.send(Command::ReadCounters);
    let mut counters = None;
    let end = Instant::now() + Duration::from_secs(1);
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        match receiver.recv_timeout(left) {
            Ok(Note::Counters(c)) => counters = Some(c),
            Ok(Note::Invoke(record)) => invokes.push(record),
            Ok(Note::Input(record)) => inputs.push(record),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    engine.stop();
    let _ = std::fs::remove_dir_all(&profiles_dir);

    let seconds = match (inputs.first(), inputs.last()) {
        (Some(first), Some(last)) => last.2.duration_since(first.2).as_secs_f64(),
        _ => 0.0,
    };
    let mut most_in_one_second = 0;
    let mut start = 0;
    for (end, record) in inputs.iter().enumerate() {
        while record.2.duration_since(inputs[start].2) > Duration::from_secs(1) {
            start += 1;
        }
        most_in_one_second = most_in_one_second.max(end - start + 1);
    }
    let longest_wait = inputs
        .iter()
        .zip(&invokes)
        .map(|(input, invoke)| invoke.2.saturating_duration_since(input.2))
        .max()
        .unwrap_or_default();
    let same_order = inputs.len() == invokes.len()
        && inputs
            .iter()
            .zip(&invokes)
            .all(|(a, b)| (a.0, a.1) == (b.0, b.1));

    println!();
    println!("Knob input events that the shell got:   {}", inputs.len());
    println!("Invoke requests that the handler got:   {}", invokes.len());
    if let Some(c) = counters {
        println!(
            "Engine counter, reports received:       {}",
            c.reports_received
        );
        println!("Engine counter, input events:           {}", c.input_events);
        println!("Engine counter, Invoke requests sent:   {}", c.invokes_sent);
    }
    println!("Time from the first to the last event:  {seconds:.1} s");
    println!("Most events in 1 second:                {most_in_one_second}");
    println!(
        "Longest time from the shell event to the handler: {:.1} ms",
        longest_wait.as_secs_f64() * 1000.0
    );
    let counters_agree = counters.is_some_and(|c| c.invokes_sent as usize == invokes.len());
    if same_order && counters_agree {
        println!("RESULT: no input event is lost, and the order is the same.");
    } else {
        println!("RESULT: FAULT. The records do not agree.");
        std::process::exit(1);
    }
}
