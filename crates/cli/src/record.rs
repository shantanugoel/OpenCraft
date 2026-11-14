//! `opencraft-probe record`: record one file for each gesture.
//!
//! The steps come from the descriptor of the connected device. The user does
//! the gesture, and the command goes to the next line by itself. Each gesture
//! goes to one file in the format of `opencraft_core::recording`.
//!
//! The names and the lines for the N4 Pro are the same as the fixtures in
//! `crates/core/tests/fixtures/input/`.

use crate::{draw_test_pattern, flag_str, flag_u64, AnyResult};
use opencraft_core::link::KEEP_ALIVE_INTERVAL;
use opencraft_core::{
    events, list_devices, recording, select, Control, ControlKind, Decoded, Descriptor, Device,
    Gesture, HidTransport, InputEvent,
};
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// The time to wait for the first report of a step that ends on a quiet gap.
const FIRST_REPORT_LIMIT: Duration = Duration::from_secs(20);

/// The longest time for one step.
const STEP_LIMIT: Duration = Duration::from_secs(120);

/// How a step ends after its first report, or before any report for a step
/// that waits for a knob push or for Enter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
    /// No report for this number of milliseconds.
    Quiet(u64),
    /// The user pushes a knob. The report of the push is not in the file.
    /// The descriptor has a knob, and each knob has a push code.
    KnobPush,
    /// The user pushes Enter. The descriptor has no knob. The Enter key is
    /// not in the file. The step also ends at [`STEP_LIMIT`].
    Enter,
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

/// The lines that the command shows before the first step.
///
/// A step that ends on Enter is the only step that uses the keyboard.
fn introduction(steps: &[Step]) -> &'static str {
    if steps.iter().any(|step| step.end == End::Enter) {
        "Use the keyboard only when a line says to push Enter. Each dot is one report from the device."
    } else {
        "Do not use the keyboard. Each dot is one report from the device."
    }
}

/// The steps for `descriptor`, in the order of the recording.
///
/// Keys come first, by row and then by column. Then each knob, then each bar
/// panel, then the touch-bar steps when the model has a touch bar. A step
/// with 20 swipes ends on a knob push when the model has a knob. With no
/// knob, that step ends when the user pushes Enter.
fn steps(descriptor: &Descriptor) -> Vec<Step> {
    let mut out = Vec::new();
    let keys = controls_sorted(descriptor, ControlKind::Key);
    add_key_steps(&mut out, &keys);
    let knobs = controls_sorted(descriptor, ControlKind::Knob);
    add_knob_steps(&mut out, &knobs);
    let panels = controls_sorted(descriptor, ControlKind::BarPanel);
    add_bar_steps(&mut out, &panels);
    if descriptor
        .controls_of(ControlKind::TouchBar)
        .next()
        .is_some()
    {
        // Each knob stores a push code, so any knob can end the long swipes.
        add_touch_steps(&mut out, !knobs.is_empty());
    }
    out
}

fn controls_sorted(descriptor: &Descriptor, kind: ControlKind) -> Vec<&Control> {
    let mut controls: Vec<&Control> = descriptor.controls_of(kind).collect();
    controls.sort_by_key(|control| {
        (
            control.position.row,
            control.position.column,
            control.number,
        )
    });
    controls
}

fn add_key_steps(out: &mut Vec<Step>, keys: &[&Control]) {
    for key in keys {
        out.push(step(
            format!("key-row{}-col{}", key.position.row, key.position.column),
            format!(
                "Key, {}, {} from the left: press it 1 time",
                row_name(key.position.row),
                ordinal(place_in_row(keys, key)),
            ),
            End::Quiet(1000),
        ));
    }
    if let Some((left, right)) = two_keys(keys) {
        out.push(step(
            "keys-two",
            format!(
                "Keys, {}, {} and {} from the left: press them together 1 time",
                row_name(left.position.row),
                ordinal(place_in_row(keys, left)),
                ordinal(place_in_row(keys, right)),
            ),
            End::Quiet(1500),
        ));
    }
    if let Some(key) = hold_key(keys) {
        out.push(step(
            "key-hold",
            format!(
                "Key, {}, {} from the left: hold it for 3 seconds",
                row_name(key.position.row),
                ordinal(place_in_row(keys, key)),
            ),
            End::Quiet(4500),
        ));
    }
}

/// The two keys for the step "press them together".
///
/// The N4 Pro uses the first two keys of the second row. Use that row when
/// it has two keys. Otherwise use the first row that has two keys.
fn two_keys<'a>(keys: &[&'a Control]) -> Option<(&'a Control, &'a Control)> {
    let row = if keys.iter().filter(|key| key.position.row == 2).count() >= 2 {
        2
    } else {
        let mut rows: Vec<u8> = keys.iter().map(|key| key.position.row).collect();
        rows.sort_unstable();
        rows.dedup();
        rows.into_iter()
            .find(|row| keys.iter().filter(|key| key.position.row == *row).count() >= 2)?
    };
    let mut in_row = keys.iter().copied().filter(|key| key.position.row == row);
    Some((in_row.next()?, in_row.next()?))
}

/// The key for the held-key step.
///
/// The N4 Pro holds the first key of the second row. Use that key when the
/// second row has a key. Otherwise use the first key.
fn hold_key<'a>(keys: &[&'a Control]) -> Option<&'a Control> {
    keys.iter()
        .copied()
        .find(|key| key.position.row == 2)
        .or_else(|| keys.first().copied())
}

fn add_knob_steps(out: &mut Vec<Step>, knobs: &[&Control]) {
    if knobs.is_empty() {
        return;
    }
    for knob in knobs {
        let place = knob_place(knobs, knob);
        let n = knob.number;
        out.push(step(
            format!("knob-{n}-left"),
            format!("Knob, {place}: turn it LEFT 5 clicks"),
            End::Quiet(2000),
        ));
        out.push(step(
            format!("knob-{n}-right"),
            format!("Knob, {place}: turn it RIGHT 5 clicks"),
            End::Quiet(2000),
        ));
        out.push(step(
            format!("knob-{n}-push"),
            format!("Knob, {place}: push it 1 time"),
            End::Quiet(1500),
        ));
    }
    let first = knobs[0];
    let place = knob_place(knobs, first);
    out.push(step(
        format!("knob-{}-fast", first.number),
        format!("Knob, {place}: turn it as fast as you can for 5 seconds"),
        End::Quiet(1500),
    ));
}

fn add_bar_steps(out: &mut Vec<Step>, panels: &[&Control]) {
    if panels.is_empty() {
        return;
    }
    // The N4 Pro has four panels in one row, and its lines say "quarter".
    let word = if panels.len() == 4 && single_row(panels) {
        "quarter"
    } else {
        "part"
    };
    for panel in panels {
        let ord = ordinal(place_in_row(panels, panel));
        let place = if single_row(panels) {
            format!("{ord} {word} from the left")
        } else {
            format!(
                "{}, {ord} {word} from the left",
                row_name(panel.position.row)
            )
        };
        out.push(step(
            format!("bar-tap-{}", panel.number),
            format!("Touch bar, {place}: tap it 1 time"),
            End::Quiet(1500),
        ));
    }
}

fn add_touch_steps(out: &mut Vec<Step>, knob_push: bool) {
    out.push(step(
        "bar-slide-left-to-right",
        "Touch bar: move 1 finger slowly from the left end to the right end",
        End::Quiet(2000),
    ));
    out.push(step(
        "bar-slide-top-to-bottom",
        "Touch bar: move 1 finger slowly from the top edge to the bottom edge",
        End::Quiet(2000),
    ));
    out.push(step(
        "bar-hold",
        "Touch bar, left end: hold 1 finger on it for 2 seconds",
        End::Quiet(3500),
    ));
    out.push(step(
        "swipe-left-1",
        "Touch bar: swipe LEFT 1 time",
        End::Quiet(2000),
    ));
    out.push(step(
        "swipe-right-1",
        "Touch bar: swipe RIGHT 1 time",
        End::Quiet(2000),
    ));
    let (ending, end) = if knob_push {
        ("Then push a knob.", End::KnobPush)
    } else {
        ("Then push Enter.", End::Enter)
    };
    out.push(step(
        "swipe-left-20",
        format!("Touch bar: swipe LEFT 20 times. {ending}"),
        end,
    ));
    out.push(step(
        "swipe-right-20",
        format!("Touch bar: swipe RIGHT 20 times. {ending}"),
        end,
    ));
}

fn knob_place(knobs: &[&Control], knob: &Control) -> String {
    let ord = ordinal(place_in_row(knobs, knob));
    if single_row(knobs) {
        format!("{ord} from the left")
    } else {
        format!("{}, {ord} from the left", row_name(knob.position.row))
    }
}

fn single_row(controls: &[&Control]) -> bool {
    controls
        .iter()
        .all(|control| control.position.row == controls[0].position.row)
}

/// The place of `control` in its row. 1 is the left control of that row.
fn place_in_row(controls: &[&Control], control: &Control) -> usize {
    controls
        .iter()
        .filter(|other| {
            other.position.row == control.position.row
                && other.position.column <= control.position.column
        })
        .count()
}

fn ordinal(place: usize) -> String {
    let suffix = match place % 100 {
        11..=13 => "th",
        _ => match place % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{place}{suffix}")
}

fn row_name(row: u8) -> String {
    match row {
        1 => "top row".to_string(),
        2 => "second row".to_string(),
        3 => "third row".to_string(),
        4 => "fourth row".to_string(),
        5 => "fifth row".to_string(),
        other => format!("row {other}"),
    }
}

/// Lines from standard input. One thread reads for the whole command, so a
/// step that ends on Enter can see the key without a second reader.
struct EnterLines {
    rx: Receiver<()>,
}

impl EnterLines {
    fn start() -> Self {
        // Drop an Enter key that the user typed before the first such step.
        #[cfg(unix)]
        unsafe {
            libc::tcflush(0, libc::TCIFLUSH);
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut lines = stdin.lock();
            let mut line = String::new();
            loop {
                line.clear();
                match lines.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if tx.send(()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Self { rx }
    }

    /// Drop Enter keys that arrived before this step.
    fn discard_pending(&self) {
        while self.take_press() {
            // An Enter from an earlier step must not end this step.
        }
    }

    fn take_press(&self) -> bool {
        self.rx.try_recv().is_ok()
    }
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

    // The step list needs the descriptor, so open the device before the
    // check of `--from`. Do not draw until the check passes.
    let found = list_devices()?;
    let mut device = Device::open(select(&found)?)?;
    let firmware = device.firmware_version().unwrap_or_default();
    let model = device.descriptor().name;
    let steps = steps(device.descriptor());
    if steps.is_empty() {
        return Err("This device has no control to record.".into());
    }
    if from > steps.len() {
        return Err(format!("--from must be from 1 to {}", steps.len()).into());
    }
    std::fs::create_dir_all(out)?;

    // Open the device with no text on the screen: the serial number must not
    // show, because the user can copy the screen into an issue.
    draw_test_pattern(&mut device, None, 80)?;
    let mut session = Session {
        device,
        last_command: Instant::now(),
    };
    let enter = steps
        .iter()
        .any(|step| step.end == End::Enter)
        .then(EnterLines::start);

    println!("Do what each line says. Then wait for the next line.");
    println!("{}\n", introduction(&steps));

    let mut missing = Vec::new();
    for (i, step) in steps.iter().enumerate().skip(from - 1) {
        let n = i + 1;
        if step.end == End::Enter {
            if let Some(lines) = enter.as_ref() {
                lines.discard_pending();
            }
        }
        print!("[{n:>2}/{}] {}  ", steps.len(), step.prompt);
        std::io::stdout().flush()?;

        let reports = run_step(&mut session, step.end, enter.as_ref())
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
fn run_step(
    session: &mut Session,
    end: End,
    enter: Option<&EnterLines>,
) -> opencraft_core::Result<Vec<(u64, Vec<u8>)>> {
    let mut reports: Vec<(u64, Vec<u8>)> = Vec::new();
    let started = Instant::now();
    let mut first: Option<Instant> = None;
    let mut last = Instant::now();

    loop {
        if end == End::Enter && enter.is_some_and(EnterLines::take_press) {
            return Ok(reports);
        }
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

#[cfg(test)]
#[path = "../../core/tests/support/test_model.rs"]
mod test_model;

#[cfg(test)]
mod tests {
    use super::*;
    use opencraft_core::{InputCodes, Position, SettingsSupport};
    use std::path::PathBuf;

    fn n4_pro() -> &'static Descriptor {
        opencraft_core::device::DESCRIPTORS
            .iter()
            .find(|descriptor| descriptor.model == "n4-pro" && descriptor.pid == 0x1023)
            .expect("N4 Pro descriptor")
    }

    fn key(number: u8, row: u8, column: u8) -> Control {
        Control {
            number,
            codes: InputCodes::Key { code: number },
            position: Position { row, column },
            surface: None,
            feedback_surface: None,
        }
    }

    fn knob(number: u8, column: u8) -> Control {
        Control {
            number,
            codes: InputCodes::Knob {
                turn_left: 0x10 + number,
                turn_right: 0x20 + number,
                push: 0x30 + number,
            },
            position: Position { row: 1, column },
            surface: None,
            feedback_surface: None,
        }
    }

    fn panel(number: u8, column: u8) -> Control {
        Control {
            number,
            codes: InputCodes::BarPanel { tap: 0x40 + number },
            position: Position { row: 1, column },
            surface: None,
            feedback_surface: None,
        }
    }

    fn touch_bar() -> Control {
        Control {
            number: 1,
            codes: InputCodes::TouchBar {
                touch_start: 0x01,
                swipe_right: 0x02,
                swipe_left: 0x03,
                touch_end: 0x04,
            },
            position: Position { row: 1, column: 1 },
            surface: None,
            feedback_surface: None,
        }
    }

    fn model(controls: Vec<Control>) -> Descriptor {
        let controls: &'static [Control] = Box::leak(controls.into_boxed_slice());
        Descriptor {
            name: "record test",
            model: "record-test",
            vid: 0,
            pid: 0,
            packet_size: opencraft_core::protocol::PACKET_SIZE,
            surfaces: &[],
            controls,
            settings: SettingsSupport {
                lowest_brightness: 0,
                knob_lights: 0,
                haptic: false,
            },
        }
    }

    fn names(steps: &[Step]) -> Vec<&str> {
        steps.iter().map(|step| step.name.as_str()).collect()
    }

    /// The quiet time, or the other end, of one N4 Pro step. The name is the
    /// file name with no step number.
    fn n4_end(name: &str) -> End {
        match name {
            "keys-two" => End::Quiet(1500),
            "key-hold" => End::Quiet(4500),
            "knob-1-fast" => End::Quiet(1500),
            "bar-hold" => End::Quiet(3500),
            "swipe-left-20" | "swipe-right-20" => End::KnobPush,
            name if name.starts_with("key-row") => End::Quiet(1000),
            name if name.ends_with("-push") => End::Quiet(1500),
            name if name.starts_with("knob-") => End::Quiet(2000),
            name if name.starts_with("bar-tap-") => End::Quiet(1500),
            "bar-slide-left-to-right" | "bar-slide-top-to-bottom" => End::Quiet(2000),
            "swipe-left-1" | "swipe-right-1" => End::Quiet(2000),
            other => panic!("unexpected N4 Pro step {other}"),
        }
    }

    fn fixture_recordings() -> Vec<PathBuf> {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../core/tests/fixtures/input");
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
            .map(|entry| entry.expect("fixture entry").path())
            .filter(|path| {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                let bytes = name.as_bytes();
                bytes.len() >= 3
                    && bytes[0].is_ascii_digit()
                    && bytes[1].is_ascii_digit()
                    && name.ends_with(".txt")
            })
            .collect();
        files.sort();
        files
    }

    #[test]
    fn the_n4_pro_steps_match_the_input_fixtures() {
        let steps = steps(n4_pro());
        let files = fixture_recordings();
        assert_eq!(
            files.len(),
            steps.len(),
            "fixture files and steps must be the same list"
        );
        for (index, (path, step)) in files.iter().zip(&steps).enumerate() {
            let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap();
            assert_eq!(stem, format!("{:02}-{}", index + 1, step.name));
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            let note = text
                .lines()
                .find_map(|line| line.strip_prefix("# note: "))
                .unwrap_or_else(|| panic!("no note in {}", path.display()));
            assert_eq!(note, step.prompt);
            assert_eq!(step.end, n4_end(&step.name));
        }
        assert_eq!(
            introduction(&steps),
            "Do not use the keyboard. Each dot is one report from the device."
        );
    }

    #[test]
    fn each_n4_pro_identity_has_the_same_steps() {
        let primary = steps(n4_pro());
        for descriptor in opencraft_core::device::DESCRIPTORS
            .iter()
            .filter(|descriptor| descriptor.model == "n4-pro")
        {
            let other = steps(descriptor);
            assert_eq!(names(&other), names(&primary));
            assert_eq!(
                other
                    .iter()
                    .map(|step| step.prompt.as_str())
                    .collect::<Vec<_>>(),
                primary
                    .iter()
                    .map(|step| step.prompt.as_str())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn the_test_model_has_its_own_key_and_knob_steps() {
        let steps = steps(&super::test_model::TEST_MODEL);
        assert_eq!(
            names(&steps),
            [
                "key-row1-col1",
                "key-row1-col2",
                "key-row1-col3",
                "key-row2-col1",
                "key-row2-col2",
                "key-row2-col3",
                "keys-two",
                "key-hold",
                "knob-1-left",
                "knob-1-right",
                "knob-1-push",
                "knob-1-fast",
            ]
        );
        let prompts: Vec<&str> = steps.iter().map(|step| step.prompt.as_str()).collect();
        assert_eq!(
            prompts,
            [
                "Key, top row, 1st from the left: press it 1 time",
                "Key, top row, 2nd from the left: press it 1 time",
                "Key, top row, 3rd from the left: press it 1 time",
                "Key, second row, 1st from the left: press it 1 time",
                "Key, second row, 2nd from the left: press it 1 time",
                "Key, second row, 3rd from the left: press it 1 time",
                "Keys, second row, 1st and 2nd from the left: press them together 1 time",
                "Key, second row, 1st from the left: hold it for 3 seconds",
                "Knob, 1st from the left: turn it LEFT 5 clicks",
                "Knob, 1st from the left: turn it RIGHT 5 clicks",
                "Knob, 1st from the left: push it 1 time",
                "Knob, 1st from the left: turn it as fast as you can for 5 seconds",
            ]
        );
        assert!(steps.iter().all(|step| step.end != End::Enter));
        assert!(steps.iter().all(|step| step.end != End::KnobPush));
    }

    #[test]
    fn one_row_of_keys_uses_the_top_row_for_the_pair_and_the_hold() {
        let steps = steps(&model(vec![key(3, 1, 3), key(1, 1, 1), key(2, 1, 2)]));
        assert_eq!(
            names(&steps),
            [
                "key-row1-col1",
                "key-row1-col2",
                "key-row1-col3",
                "keys-two",
                "key-hold",
            ]
        );
        assert_eq!(
            steps[3].prompt,
            "Keys, top row, 1st and 2nd from the left: press them together 1 time"
        );
        assert_eq!(
            steps[4].prompt,
            "Key, top row, 1st from the left: hold it for 3 seconds"
        );
    }

    #[test]
    fn a_second_row_with_one_key_holds_that_key_and_pairs_the_top_row() {
        let steps = steps(&model(vec![key(1, 1, 1), key(2, 1, 2), key(3, 2, 1)]));
        assert_eq!(
            steps[3].prompt,
            "Keys, top row, 1st and 2nd from the left: press them together 1 time"
        );
        assert_eq!(
            steps[4].prompt,
            "Key, second row, 1st from the left: hold it for 3 seconds"
        );
    }

    #[test]
    fn one_key_has_a_hold_and_no_pair() {
        let steps = steps(&model(vec![key(1, 3, 1)]));
        assert_eq!(names(&steps), ["key-row3-col1", "key-hold"]);
        assert_eq!(
            steps[0].prompt,
            "Key, third row, 1st from the left: press it 1 time"
        );
        assert_eq!(
            steps[1].prompt,
            "Key, third row, 1st from the left: hold it for 3 seconds"
        );
    }

    #[test]
    fn knobs_and_bar_panels_follow_the_column_not_the_table_order() {
        let steps = steps(&model(vec![
            knob(2, 2),
            knob(1, 1),
            panel(2, 2),
            panel(1, 1),
        ]));
        assert_eq!(
            names(&steps),
            [
                "knob-1-left",
                "knob-1-right",
                "knob-1-push",
                "knob-2-left",
                "knob-2-right",
                "knob-2-push",
                "knob-1-fast",
                "bar-tap-1",
                "bar-tap-2",
            ]
        );
        assert_eq!(
            steps[7].prompt,
            "Touch bar, 1st part from the left: tap it 1 time"
        );
        assert_eq!(
            steps[8].prompt,
            "Touch bar, 2nd part from the left: tap it 1 time"
        );
        assert!(steps.iter().all(|step| !step.name.starts_with("swipe-")));
    }

    #[test]
    fn a_touch_bar_with_no_knob_ends_the_long_swipes_on_enter() {
        let steps = steps(&model(vec![touch_bar()]));
        assert_eq!(
            names(&steps),
            [
                "bar-slide-left-to-right",
                "bar-slide-top-to-bottom",
                "bar-hold",
                "swipe-left-1",
                "swipe-right-1",
                "swipe-left-20",
                "swipe-right-20",
            ]
        );
        assert_eq!(steps[5].end, End::Enter);
        assert_eq!(steps[6].end, End::Enter);
        assert_eq!(
            steps[5].prompt,
            "Touch bar: swipe LEFT 20 times. Then push Enter."
        );
        assert_eq!(
            steps[6].prompt,
            "Touch bar: swipe RIGHT 20 times. Then push Enter."
        );
        assert_eq!(
            introduction(&steps),
            "Use the keyboard only when a line says to push Enter. Each dot is one report from the device."
        );
    }

    #[test]
    fn a_model_with_no_control_has_no_step() {
        assert!(steps(&model(vec![])).is_empty());
    }

    #[test]
    fn ordinals_use_the_english_suffix() {
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(4), "4th");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(12), "12th");
        assert_eq!(ordinal(13), "13th");
        assert_eq!(ordinal(21), "21st");
        assert_eq!(ordinal(22), "22nd");
        assert_eq!(ordinal(23), "23rd");
    }
}
