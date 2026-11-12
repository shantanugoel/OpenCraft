//! `opencraft-probe record`: record the input of each control of the N4 Pro.
//!
//! The command shows one short line for each gesture. The user does the
//! gesture on the device, and the command goes to the next line by itself.
//! The user does not use the keyboard. Each gesture goes to one file in the
//! format of `opencraft_core::recording`.
//!
//! The steps are for the N4 Pro only (issue #157). For a different model,
//! use `opencraft-probe raw --save`: see `docs/ADDING-A-DEVICE-MODEL.md`.

use crate::{draw_test_pattern, flag_str, flag_u64, AnyResult};
use opencraft_core::link::KEEP_ALIVE_INTERVAL;
use opencraft_core::{
    events, list_devices, recording, select, Decoded, Device, Gesture, HidTransport, InputEvent,
};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

/// The time to wait for the first report of a step.
const FIRST_REPORT_LIMIT: Duration = Duration::from_secs(20);

/// The longest time for one step.
const STEP_LIMIT: Duration = Duration::from_secs(120);

/// How a step ends after its first report.
#[derive(Clone, Copy)]
enum End {
    /// No report for this number of milliseconds.
    Quiet(u64),
    /// The user pushes a knob. The report of the push is not in the file.
    KnobPush,
}

struct Step {
    name: String,
    prompt: String,
    end: End,
}

fn step(name: impl Into<String>, prompt: impl Into<String>, end: End) -> Step {
    Step {
        name: name.into(),
        prompt: prompt.into(),
        end,
    }
}

fn steps() -> Vec<Step> {
    const ORD: [&str; 5] = ["1st", "2nd", "3rd", "4th", "5th"];
    let mut s = Vec::new();

    for (row, row_name) in ["top row", "second row"].iter().enumerate() {
        for (col, ord) in ORD.iter().enumerate() {
            s.push(step(
                format!("key-row{}-col{}", row + 1, col + 1),
                format!("Key, {row_name}, {ord} from the left: press it 1 time"),
                End::Quiet(1000),
            ));
        }
    }
    s.push(step(
        "keys-two",
        "Keys, second row, 1st and 2nd from the left: press them together 1 time",
        End::Quiet(1500),
    ));
    s.push(step(
        "key-hold",
        "Key, second row, 1st from the left: hold it for 3 seconds",
        End::Quiet(4500),
    ));

    for (k, ord) in ORD.iter().take(4).enumerate() {
        let n = k + 1;
        s.push(step(
            format!("knob-{n}-left"),
            format!("Knob, {ord} from the left: turn it LEFT 5 clicks"),
            End::Quiet(2000),
        ));
        s.push(step(
            format!("knob-{n}-right"),
            format!("Knob, {ord} from the left: turn it RIGHT 5 clicks"),
            End::Quiet(2000),
        ));
        s.push(step(
            format!("knob-{n}-push"),
            format!("Knob, {ord} from the left: push it 1 time"),
            End::Quiet(1500),
        ));
    }
    s.push(step(
        "knob-1-fast",
        "Knob, 1st from the left: turn it as fast as you can for 5 seconds",
        End::Quiet(1500),
    ));

    for (k, ord) in ORD.iter().take(4).enumerate() {
        s.push(step(
            format!("bar-tap-{}", k + 1),
            format!("Touch bar, {ord} quarter from the left: tap it 1 time"),
            End::Quiet(1500),
        ));
    }
    s.push(step(
        "bar-slide-left-to-right",
        "Touch bar: move 1 finger slowly from the left end to the right end",
        End::Quiet(2000),
    ));
    s.push(step(
        "bar-slide-top-to-bottom",
        "Touch bar: move 1 finger slowly from the top edge to the bottom edge",
        End::Quiet(2000),
    ));
    s.push(step(
        "bar-hold",
        "Touch bar, left end: hold 1 finger on it for 2 seconds",
        End::Quiet(3500),
    ));
    s.push(step(
        "swipe-left-1",
        "Touch bar: swipe LEFT 1 time",
        End::Quiet(2000),
    ));
    s.push(step(
        "swipe-right-1",
        "Touch bar: swipe RIGHT 1 time",
        End::Quiet(2000),
    ));
    s.push(step(
        "swipe-left-20",
        "Touch bar: swipe LEFT 20 times. Then push a knob.",
        End::KnobPush,
    ));
    s.push(step(
        "swipe-right-20",
        "Touch bar: swipe RIGHT 20 times. Then push a knob.",
        End::KnobPush,
    ));
    s
}

/// The device, with the time of the last command, for the keep-alive.
struct Session {
    device: Device<HidTransport>,
    last_command: Instant,
}

impl Session {
    /// Wait up to 50 ms for one report. Send the keep-alive when it is time.
    fn poll(&mut self) -> opencraft_core::Result<Option<Vec<u8>>> {
        if self.last_command.elapsed() >= KEEP_ALIVE_INTERVAL {
            self.device.keep_alive()?;
            self.last_command = Instant::now();
        }
        self.device.read(Duration::from_millis(50))
    }
}

pub fn cmd_record(rest: &[String]) -> AnyResult {
    let out = flag_str(rest, "--out").unwrap_or("crates/core/tests/fixtures/input");
    let from = flag_u64(rest, "--from").unwrap_or(1).max(1) as usize;
    let steps = steps();
    if from > steps.len() {
        return Err(format!("--from must be from 1 to {}", steps.len()).into());
    }
    std::fs::create_dir_all(out)?;

    // Open the device with no text on the screen: the serial number must not
    // show, because the user can copy the screen into an issue.
    let found = list_devices()?;
    let mut device = Device::open(select(&found)?)?;
    let firmware = device.firmware_version().unwrap_or_default();
    let model = device.descriptor().name;
    draw_test_pattern(&mut device, None, 80)?;
    let mut session = Session {
        device,
        last_command: Instant::now(),
    };

    println!("Do what each line says. Then wait for the next line.");
    println!("Do not use the keyboard. Each dot is one report from the device.\n");

    let mut missing = Vec::new();
    for (i, step) in steps.iter().enumerate().skip(from - 1) {
        let n = i + 1;
        print!("[{n:>2}/{}] {}  ", steps.len(), step.prompt);
        std::io::stdout().flush()?;

        let reports = run_step(&mut session, step.end)
            .map_err(|e| format!("{e}\n\n  To continue: opencraft-probe record --from {n}"))?;

        let path = Path::new(out).join(format!("{n:02}-{}.txt", step.name));
        if reports.is_empty() {
            println!(" NO REPORT");
            missing.push(n);
            let _ = std::fs::remove_file(&path);
            continue;
        }
        let mut text = recording::header(&step.prompt, model, &firmware);
        for (ms, bytes) in &reports {
            text.push_str(&recording::format_report_line(*ms, bytes));
            text.push('\n');
        }
        std::fs::write(&path, text)?;
        println!(" ok ({})", reports.len());
    }

    session.device.clear_all()?;
    println!("\nDone. The files are in {out}.");
    if !missing.is_empty() {
        println!("Steps with no report: {missing:?}. That is also a result.");
    }
    Ok(())
}

/// Record the reports of one step. The time of each report is in
/// milliseconds from the first report of the step.
fn run_step(session: &mut Session, end: End) -> opencraft_core::Result<Vec<(u64, Vec<u8>)>> {
    let mut reports: Vec<(u64, Vec<u8>)> = Vec::new();
    let started = Instant::now();
    let mut first: Option<Instant> = None;
    let mut last = Instant::now();

    loop {
        let report = session.poll()?;
        let now = Instant::now();
        if let Some(bytes) = report {
            let decoded = events::decode(session.device.descriptor(), &bytes);
            // An acknowledgement is not a gesture.
            if decoded == Decoded::Ack {
                continue;
            }
            if matches!(end, End::KnobPush)
                && matches!(
                    decoded,
                    Decoded::Input(InputEvent {
                        gesture: Gesture::Push,
                        ..
                    })
                )
            {
                // The push ends the step, also when no swipe gave a report.
                return Ok(reports);
            }
            let t0 = *first.get_or_insert(now);
            last = now;
            reports.push((now.duration_since(t0).as_millis() as u64, bytes));
            print!(".");
            let _ = std::io::stdout().flush();
            continue;
        }
        if started.elapsed() >= STEP_LIMIT {
            return Ok(reports);
        }
        if let End::Quiet(ms) = end {
            let done = match first {
                None => started.elapsed() >= FIRST_REPORT_LIMIT,
                Some(_) => now.duration_since(last) >= Duration::from_millis(ms),
            };
            if done {
                return Ok(reports);
            }
        }
    }
}
