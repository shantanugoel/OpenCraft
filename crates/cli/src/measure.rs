//! Probe commands for the measurements of issues #17, #18, and #19.
//!
//! - `send`: send one protocol command and nothing else.
//! - `bars` and `wide`: draw the bar panels with a pattern that shows their
//!   edges, and send an image that is wider than the surface.
//! - `repaint`: draw all surfaces many times, and print the time of each
//!   repaint.
//!
//! Each command that a person must look at has `--hold <seconds>`, which keeps
//! the connection with the keep-alive. Without the keep-alive the device stops
//! the connection approximately 35 seconds after a draw, and its displays go
//! blank. `--keep-alive <n>` changes the interval, and `0` stops the
//! keep-alive.
//!
//! A command that draws sends the start commands first (`DIS`, then the
//! brightness command), because the displays show nothing after a new
//! connection without them. `--no-init` skips them.
//!
//! The code that writes to the device works with any transport, so the tests
//! run it on a fake transport.

use crate::{hex, open_device, parse_hex_color, start_display, AnyResult};
use opencraft_core::image::{self, DEFAULT_JPEG_QUALITY};
use opencraft_core::link::KEEP_ALIVE_INTERVAL;
use opencraft_core::{
    events, protocol, ControlKind, Decoded, Descriptor, Device, Gesture, ImageFormat, InputEvent,
    Surface, SurfaceKind, Transport,
};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// The default brightness of the start commands, as in `test`.
const DEFAULT_BRIGHTNESS: u64 = 80;

/// The longest time that `--wait-key` waits for a key press.
const WAIT_KEY_LIMIT: Duration = Duration::from_secs(600);

/// How long one read waits while the probe holds the connection.
const POLL: Duration = Duration::from_millis(200);

/// The time between two progress lines of a long hold.
const PROGRESS: Duration = Duration::from_secs(60);

/// The longest value for `--hold`: one day.
const MAX_HOLD: u64 = 86_400;

// --- Arguments ---------------------------------------------------------------

/// The arguments of one command, checked. A flag that the command does not
/// know, or a flag without a value, is an error. The main parser of the probe
/// ignores both, and a measurement must not run with a wrong setting.
#[derive(Debug, Default)]
pub struct Args {
    positional: Vec<String>,
    values: HashMap<String, String>,
    flags: HashSet<String>,
}

impl Args {
    /// Split `args`. `values` are the options that have a value. `flags` are
    /// the options that have none.
    pub fn parse(args: &[String], values: &[&str], flags: &[&str]) -> Result<Self, String> {
        let mut out = Args::default();
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if !arg.starts_with("--") {
                out.positional.push(arg.clone());
            } else if flags.contains(&arg.as_str()) {
                out.flags.insert(arg.clone());
            } else if values.contains(&arg.as_str()) {
                match iter.next() {
                    Some(v) if !v.starts_with("--") => {
                        out.values.insert(arg.clone(), v.clone());
                    }
                    _ => return Err(format!("{arg} needs a value")),
                }
            } else {
                return Err(format!("unknown option: {arg}"));
            }
        }
        Ok(out)
    }

    fn has(&self, name: &str) -> bool {
        self.flags.contains(name)
    }

    fn text(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    /// The value of `name` as a whole number from `min` to `max`.
    fn number(&self, name: &str, min: u64, max: u64) -> Result<Option<u64>, String> {
        let Some(text) = self.text(name) else {
            return Ok(None);
        };
        match text.parse::<u64>() {
            Ok(n) if (min..=max).contains(&n) => Ok(Some(n)),
            _ => Err(format!(
                "{name} needs a whole number from {min} to {max}, not \"{text}\""
            )),
        }
    }

    /// `--hold <seconds>`.
    fn hold(&self) -> Result<Option<u64>, String> {
        self.number("--hold", 0, MAX_HOLD)
    }

    /// `--keep-alive <n>`: the interval for the keep-alive. The default is
    /// the interval of the device layer. `0` gives `None`: no keep-alive.
    fn keep_alive(&self) -> Result<Option<Duration>, String> {
        match self.number("--keep-alive", 0, MAX_HOLD)? {
            None => Ok(Some(KEEP_ALIVE_INTERVAL)),
            Some(0) => Ok(None),
            Some(n) => Ok(Some(Duration::from_secs(n))),
        }
    }

    /// `--brightness <0-100>` for the start commands.
    fn brightness(&self) -> Result<u8, String> {
        Ok(self
            .number("--brightness", 0, 100)?
            .unwrap_or(DEFAULT_BRIGHTNESS) as u8)
    }
}

/// The options that `hold` and the keep-alive need, for the command `own`.
fn value_options<'a>(own: &[&'a str]) -> Vec<&'a str> {
    let mut v = vec!["--hold", "--keep-alive"];
    v.extend_from_slice(own);
    v
}

/// The `--keep-alive <n>` option of `test`, `listen`, and `raw`. `None` if the
/// option is absent. A value of zero stops the keep-alive. A value that is not
/// a whole number is an error: the probe must not run a measurement with the
/// default interval when the person asked for a different one.
pub fn keep_alive_flag(args: &[String]) -> Result<Option<Duration>, String> {
    let Some(i) = args.iter().position(|a| a == "--keep-alive") else {
        return Ok(None);
    };
    match args.get(i + 1).map(|v| v.parse::<u64>()) {
        Some(Ok(n)) => Ok(Some(Duration::from_secs(n))),
        _ => Err("--keep-alive needs a whole number of seconds (0 stops it)".to_string()),
    }
}

/// The keep-alive setting as text, for the first lines of a command.
pub fn describe_keep_alive(setting: Option<Duration>) -> String {
    match setting {
        None => format!("keep-alive each {} s", KEEP_ALIVE_INTERVAL.as_secs()),
        Some(d) if d.is_zero() => "no keep-alive".to_string(),
        Some(d) => format!("keep-alive each {} s", d.as_secs()),
    }
}

/// `352x112` as a width and a height. Both are 1 to 65535.
fn parse_size(text: &str) -> Result<(u16, u16), String> {
    let wrong = || format!("a size looks like 352x112, not \"{text}\"");
    let (w, h) = text.split_once(['x', 'X']).ok_or_else(wrong)?;
    match (w.parse::<u16>(), h.parse::<u16>()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => Ok((w, h)),
        _ => Err(wrong()),
    }
}

fn parse_format(text: &str) -> Result<ImageFormat, String> {
    match text.to_ascii_lowercase().as_str() {
        "png" => Ok(ImageFormat::Png),
        "jpeg" | "jpg" => Ok(ImageFormat::Jpeg),
        _ => Err(format!("--format is png or jpeg, not \"{text}\"")),
    }
}

fn parse_percent(text: &str) -> Result<u8, String> {
    match text.parse::<u8>() {
        Ok(p) if p <= 100 => Ok(p),
        _ => Err(format!(
            "a percent is a whole number from 0 to 100, not \"{text}\""
        )),
    }
}

// --- Keep-alive, hold, and wait ------------------------------------------------

/// The keep-alive of a command that writes to the device directly. The
/// device layer's `Link` does the same for `test`, `listen`, and `raw`.
#[derive(Debug)]
pub struct Keeper {
    interval: Option<Duration>,
    last: Instant,
}

impl Keeper {
    /// `None` means "send no keep-alive". The interval starts now.
    pub fn new(interval: Option<Duration>) -> Self {
        Self {
            interval,
            last: Instant::now(),
        }
    }

    /// True if the keep-alive must go out at `now`.
    pub fn is_due(&self, now: Instant) -> bool {
        self.interval
            .is_some_and(|i| now.saturating_duration_since(self.last) >= i)
    }

    /// The time from `now` until the next keep-alive. `None` if there is none.
    pub fn next_in(&self, now: Instant) -> Option<Duration> {
        self.interval
            .map(|i| i.saturating_sub(now.saturating_duration_since(self.last)))
    }

    /// Send the keep-alive if it is due at `now`. Return true if it went out.
    pub fn tick<T: Transport>(&mut self, device: &mut Device<T>, now: Instant) -> Res<bool> {
        if !self.is_due(now) {
            return Ok(false);
        }
        device.keep_alive()?;
        self.last = now;
        Ok(true)
    }

    fn describe(&self) -> String {
        match self.interval {
            Some(i) => format!("keep-alive each {} s", i.as_secs()),
            None => "no keep-alive".to_string(),
        }
    }
}

/// Wait `duration`. Send the keep-alive when it is due. Read the input
/// reports, and return how many arrived.
fn idle<T: Transport>(device: &mut Device<T>, keeper: &mut Keeper, duration: Duration) -> Res<u64> {
    let deadline = Instant::now() + duration;
    let mut reports = 0;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Ok(reports);
        }
        keeper.tick(device, now)?;
        let mut wait = (deadline - now).min(POLL);
        if let Some(next) = keeper.next_in(now) {
            wait = wait.min(next.max(Duration::from_millis(1)));
        }
        if device.read(wait)?.is_some() {
            reports += 1;
        }
    }
}

/// Keep the connection for `seconds`, so that a person has the time to look.
/// This is the one place where the probe holds a connection after a draw.
///
/// The function prints a line each 60 seconds. If the connection fails, it
/// says how many seconds the hold lasted. That time is a measurement: the
/// device stops the connection when it does not get the keep-alive.
pub fn hold<T: Transport>(device: &mut Device<T>, seconds: u64, keeper: &mut Keeper) -> AnyResult {
    if seconds == 0 {
        return Ok(());
    }
    println!(
        "\nKeeping the connection for {seconds}s ({}).",
        keeper.describe()
    );
    let start = Instant::now();
    let total = Duration::from_secs(seconds);
    let mut reports = 0;
    while start.elapsed() < total {
        let part = (total - start.elapsed()).min(PROGRESS);
        match idle(device, keeper, part) {
            Ok(n) => reports += n,
            Err(e) => {
                println!(
                    "  the connection failed {:.1}s after the hold started: {e}",
                    start.elapsed().as_secs_f64()
                );
                return Err(e);
            }
        }
        if start.elapsed() < total {
            println!(
                "  [{:5.0}s] connection is up, {reports} input report(s) so far",
                start.elapsed().as_secs_f64()
            );
        }
    }
    println!("  held for {seconds}s, {reports} input report(s) arrived.");
    Ok(())
}

/// True if `report` is the press of a key. A person starts a timed sequence
/// with a key press on the device, so that the person does not need the
/// keyboard.
pub fn is_key_press(d: &Descriptor, report: &[u8]) -> bool {
    matches!(
        events::decode(d, report),
        Decoded::Input(InputEvent {
            gesture: Gesture::Press,
            ..
        })
    )
}

/// Wait for a key press on the device, and keep the connection.
fn wait_for_key<T: Transport>(
    device: &mut Device<T>,
    keeper: &mut Keeper,
    limit: Duration,
) -> Res<()> {
    let d = device.descriptor();
    let deadline = Instant::now() + limit;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(format!("no key press in {} s", limit.as_secs()).into());
        }
        keeper.tick(device, now)?;
        let mut wait = (deadline - now).min(POLL);
        if let Some(next) = keeper.next_in(now) {
            wait = wait.min(next.max(Duration::from_millis(1)));
        }
        if let Some(report) = device.read(wait)? {
            if is_key_press(d, &report) {
                return Ok(());
            }
        }
    }
}

/// Count the input reports that wait in the buffer: acknowledgements, and
/// everything else.
fn drain_input<T: Transport>(device: &mut Device<T>) -> Res<(u64, u64)> {
    let d = device.descriptor();
    let (mut acks, mut other) = (0, 0);
    // The limit stops the loop if a finger stays on the touch bar.
    for _ in 0..10_000 {
        match device.read(Duration::from_millis(1))? {
            Some(report) if matches!(events::decode(d, &report), Decoded::Ack) => acks += 1,
            Some(_) => other += 1,
            None => break,
        }
    }
    Ok((acks, other))
}

/// Send the start commands, and say so. With `--no-init`, send nothing.
fn begin<T: Transport>(device: &mut Device<T>, args: &Args) -> Res<()> {
    if args.has("--no-init") {
        println!("  start  : none (--no-init)");
    } else {
        let brightness = args.brightness()?;
        start_display(device, brightness)?;
        println!("  start  : DIS, then brightness {brightness}%");
    }
    Ok(())
}

/// Send each image, then the commit.
fn draw_images<T: Transport>(device: &mut Device<T>, images: &[(u8, Vec<u8>)]) -> Res<()> {
    for (slot, data) in images {
        device.send_image(*slot, data)?;
    }
    device.commit()?;
    Ok(())
}

// --- send --------------------------------------------------------------------

/// The commands of `send`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SendKind {
    Brightness,
    KnobColors,
    KnobBrightness,
    Sleep,
    KeepAlive,
    Start,
    Commit,
    Haptic,
}

impl SendKind {
    /// The command with the name `name`. The operation code of the protocol
    /// is a name too.
    fn parse(name: &str) -> Result<Self, String> {
        Ok(match name.to_ascii_lowercase().as_str() {
            "brightness" | "lig" => SendKind::Brightness,
            "knob-colors" | "knob-colours" | "setlb" => SendKind::KnobColors,
            "knob-brightness" | "lblig" => SendKind::KnobBrightness,
            "sleep" | "han" => SendKind::Sleep,
            "keep-alive" | "connect" => SendKind::KeepAlive,
            "start" | "dis" => SendKind::Start,
            "commit" | "stp" => SendKind::Commit,
            "haptic" | "qucmd" => SendKind::Haptic,
            other => {
                return Err(format!(
                    "unknown command \"{other}\". The commands are: brightness, knob-colors, \
                     knob-brightness, sleep, keep-alive, start, commit, haptic"
                ))
            }
        })
    }
}

/// One command that `send` writes.
#[derive(Debug, PartialEq, Eq)]
struct SendStep {
    /// What the step sends, as text.
    text: String,
    /// The command as the protocol builder makes it.
    command: Vec<u8>,
}

/// The steps for the command `kind` with the arguments `values`. `knobs` is
/// the number of knobs of the device. `None` means that the device is not open
/// yet: the function checks all arguments except the number of colours.
/// `brightness` and `knob-brightness` make one step for each value.
fn send_steps(
    kind: SendKind,
    values: &[String],
    knobs: Option<usize>,
) -> Result<Vec<SendStep>, String> {
    let none = |name: &str| {
        if values.is_empty() {
            Ok(())
        } else {
            Err(format!("{name} has no argument"))
        }
    };
    let step = |text: &str, command: Vec<u8>| {
        Ok(vec![SendStep {
            text: text.to_string(),
            command,
        }])
    };
    match kind {
        SendKind::Brightness | SendKind::KnobBrightness => {
            if values.is_empty() {
                return Err("give a percent from 0 to 100 (more than one is possible)".into());
            }
            values
                .iter()
                .map(|v| {
                    let p = parse_percent(v)?;
                    Ok(if kind == SendKind::Brightness {
                        SendStep {
                            text: format!("LIG {p}%"),
                            command: protocol::set_brightness(p),
                        }
                    } else {
                        SendStep {
                            text: format!("LBLIG {p}%"),
                            command: protocol::set_led_brightness(p),
                        }
                    })
                })
                .collect()
        }
        SendKind::KnobColors => {
            if let Some(knobs) = knobs {
                if knobs == 0 {
                    return Err("this device has no knob".into());
                }
                if values.len() != knobs {
                    return Err(format!(
                        "this device has {knobs} knobs: give {knobs} colours, one rrggbb for each knob"
                    ));
                }
            }
            let colors = values
                .iter()
                .map(|v| parse_hex_color(v).map_err(|e| format!("colour \"{v}\": {e}")))
                .collect::<Result<Vec<_>, _>>()?;
            step(
                &format!("SETLB {}", values.join(" ")),
                protocol::set_led_colors(&colors),
            )
        }
        SendKind::Sleep => {
            none("sleep")?;
            step("HAN (sleep)", protocol::sleep())
        }
        SendKind::KeepAlive => {
            none("keep-alive")?;
            step("CONNECT (keep-alive)", protocol::keep_alive())
        }
        SendKind::Start => {
            none("start")?;
            step("DIS (start)", protocol::init())
        }
        SendKind::Commit => {
            none("commit")?;
            step("STP (commit)", protocol::commit())
        }
        SendKind::Haptic => match values {
            [v] if v.eq_ignore_ascii_case("on") => {
                step("QUCMD haptic on", protocol::set_vibration(true))
            }
            [v] if v.eq_ignore_ascii_case("off") => {
                step("QUCMD haptic off", protocol::set_vibration(false))
            }
            _ => Err("haptic needs \"on\" or \"off\"".into()),
        },
    }
}

/// The first 16 bytes that go out for `command`, as text. The device layer
/// pads a command with zero bytes to the report size, so the text shows the
/// padding too.
fn shown_bytes(command: &[u8]) -> String {
    let mut bytes = command.to_vec();
    bytes.resize(16, 0);
    hex(&bytes[..16]).trim_end().to_string()
}

/// How `run_send` waits between the steps.
struct SendTiming {
    /// Wait for a key press before each step, not a time.
    wait_key: bool,
    /// The time between two steps.
    each: Duration,
}

/// Write the steps, one after the other, and print what goes out.
fn run_send<T: Transport>(
    device: &mut Device<T>,
    steps: &[SendStep],
    timing: &SendTiming,
    keeper: &mut Keeper,
) -> Res<()> {
    for (i, step) in steps.iter().enumerate() {
        if timing.wait_key {
            println!("  Press a key on the device to send {}.", step.text);
            wait_for_key(device, keeper, WAIT_KEY_LIMIT)?;
        } else if i > 0 {
            idle(device, keeper, timing.each)?;
        }
        device.send(step.command.clone())?;
        println!("  sent {}: {}", step.text, shown_bytes(&step.command));
    }
    Ok(())
}

/// `send <name> [args]`: send one protocol command and nothing else.
pub fn cmd_send(rest: &[String]) -> AnyResult {
    let usage = "usage: opencraft-probe send <name> [args] [--hold <s>] [--keep-alive <n>]\n\
                 names: brightness <0-100>..., knob-colors <rrggbb>..., knob-brightness <0-100>..., \
                 sleep, keep-alive, start, commit, haptic on|off";
    let args = Args::parse(rest, &value_options(&["--each"]), &["--draw", "--wait-key"])?;
    let Some((name, values)) = args.positional.split_first() else {
        return Err(usage.into());
    };
    let kind = SendKind::parse(name)?;
    let hold_seconds = args.hold()?;
    let keep_alive = args.keep_alive()?;
    args.brightness()?;
    let timing = SendTiming {
        wait_key: args.has("--wait-key"),
        each: Duration::from_secs(args.number("--each", 0, MAX_HOLD)?.unwrap_or(5)),
    };

    // Check the arguments before the device opens.
    send_steps(kind, values, None)?;
    let mut device = open_device()?;
    let knobs = device.descriptor().controls_of(ControlKind::Knob).count();
    let steps = send_steps(kind, values, Some(knobs))?;

    let mut keeper = Keeper::new(keep_alive);
    println!("  {}", keeper.describe());
    if args.has("--draw") {
        // The start commands, then the numbered surfaces, so that a person
        // can see the effect of the command.
        let drawn = crate::draw_test_pattern(&mut device, None, args.brightness()?)?;
        println!("  draw   : start commands, then {drawn} numbered surfaces");
    }
    run_send(&mut device, &steps, &timing, &mut keeper)?;
    hold(&mut device, hold_seconds.unwrap_or(0), &mut keeper)
}

// --- bars and wide -----------------------------------------------------------

/// The bar panels of the descriptor, in table order.
fn bar_panels(d: &Descriptor) -> Vec<&'static Surface> {
    d.slot_surfaces()
        .filter(|s| s.kind() == SurfaceKind::BarPanel)
        .collect()
}

/// The images that show the edges of each bar panel: the slot and the data.
fn bar_edge_images(panels: &[&Surface]) -> Res<Vec<(u8, Vec<u8>)>> {
    panels
        .iter()
        .map(|s| {
            let slot = s.slot.ok_or("a bar panel has no slot")?;
            let img = image::bar_edge_pattern(s.width, s.height, slot);
            Ok((slot, image::encode_for(s, &img, DEFAULT_JPEG_QUALITY)?))
        })
        .collect()
}

/// `bars`: draw each bar panel with a pattern that shows its edges.
pub fn cmd_bars(rest: &[String]) -> AnyResult {
    let args = Args::parse(rest, &value_options(&["--brightness"]), &["--no-init"])?;
    let hold_seconds = args.hold()?;
    args.brightness()?;
    let keep_alive = args.keep_alive()?;

    let mut device = open_device()?;
    let mut keeper = Keeper::new(keep_alive);
    let panels = bar_panels(device.descriptor());
    if panels.is_empty() {
        return Err("this device has no bar panel in its descriptor".into());
    }
    let images = bar_edge_images(&panels)?;

    println!("  {}", keeper.describe());
    begin(&mut device, &args)?;
    draw_images(&mut device, &images)?;
    println!(
        "  drawn  : {} bar panels ({}), committed",
        images.len(),
        panels
            .iter()
            .map(|s| format!("{}x{}", s.width, s.height))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "
Each bar panel has a white border of 1 pixel, its own fill colour, its slot
number, and its size as text. The marks in the corners, as the device must show
them (upright): red at the top left, green at the top right, blue at the
bottom left, yellow at the bottom right.
Look at: a space between the panels, if the border shows on all 4 sides of
each panel, and the corner marks."
    );
    hold(&mut device, hold_seconds.unwrap_or(0), &mut keeper)
}

/// `wide <slot> --size <w>x<h>`: send an image that is wider than the surface.
pub fn cmd_wide(rest: &[String]) -> AnyResult {
    let usage = "usage: opencraft-probe wide <slot> --size <w>x<h> [--format png|jpeg] \
                 [--quality <1-100>] [--with-bars] [--hold <s>]";
    let args = Args::parse(
        rest,
        &value_options(&["--size", "--format", "--quality", "--brightness"]),
        &["--with-bars", "--no-init"],
    )?;
    let [slot] = args.positional.as_slice() else {
        return Err(usage.into());
    };
    let slot: u8 = slot.parse().map_err(|_| usage)?;
    // The size is a measurement of what the device accepts, so it never
    // comes from the descriptor.
    let (width, height) = parse_size(args.text("--size").ok_or(format!(
        "--size is necessary: the size is not from the descriptor.\n{usage}"
    ))?)?;
    let format = args.text("--format").map(parse_format).transpose()?;
    let quality = args
        .number("--quality", 1, 100)?
        .unwrap_or(DEFAULT_JPEG_QUALITY as u64) as u8;
    let hold_seconds = args.hold()?;
    args.brightness()?;
    let keep_alive = args.keep_alive()?;

    let mut device = open_device()?;
    let mut keeper = Keeper::new(keep_alive);
    let d = device.descriptor();
    let surface = d
        .surface_at_slot(slot)
        .ok_or_else(|| format!("slot {slot} does not exist on this device"))?;
    let format = format.unwrap_or(surface.format);

    let img = image::wide_pattern(width, height, surface.width);
    let data = image::encode_as(surface, &img, format, quality)?;
    println!(
        "  image  : {width}x{height} {} for {} (slot {slot}, {}x{} in the descriptor), {} bytes ({} the 2-byte length limit of 65535)",
        format.name(),
        surface.id,
        surface.width,
        surface.height,
        data.len(),
        if data.len() > 0xFFFF { "above" } else { "below" }
    );

    let mut images = Vec::new();
    if args.has("--with-bars") {
        let others: Vec<_> = bar_panels(d)
            .into_iter()
            .filter(|s| s.slot != Some(slot))
            .collect();
        images = bar_edge_images(&others)?;
    }
    images.push((slot, data));

    println!("  {}", keeper.describe());
    begin(&mut device, &args)?;
    draw_images(&mut device, &images)?;
    println!("  drawn  : {} image(s), committed", images.len());
    println!(
        "
The image has a thick white line at each multiple of {w} pixels (the width of
the surface in the descriptor), a thin line at each multiple of {q} pixels, and
the x coordinate as text at each multiple of {h} pixels. Look at: how much of
the image shows, where it is cut, if it goes over the next panel, and if the
device shows anything wrong.",
        w = surface.width,
        q = surface.width / 4,
        h = surface.width / 2
    );
    hold(&mut device, hold_seconds.unwrap_or(0), &mut keeper)
}

// --- repaint -----------------------------------------------------------------

/// The words on the key faces. They are different for each surface.
const KEY_WORDS: [&str; 10] = [
    "PLAY", "MUTE", "MIC", "CAM", "NEXT", "BACK", "SCENE", "LIGHT", "VOLUME", "MORE",
];

/// The words on the bar panels.
const BAR_WORDS: [&str; 4] = ["VOLUME", "BRIGHT", "TEMP", "MIXER"];

/// A short word for a surface. `variant` shifts the word, so that the two
/// sets of a repaint are different.
fn word_for(surface: &Surface, variant: u8) -> &'static str {
    let words: &[&str] = match surface.kind() {
        SurfaceKind::KeyFace => &KEY_WORDS,
        SurfaceKind::BarPanel => &BAR_WORDS,
        SurfaceKind::Background => &["ICON"],
    };
    words[(surface.id.number as usize - 1 + variant as usize) % words.len()]
}

/// One full set of images: the slot and the data of each surface.
type ImageSet = Vec<(u8, Vec<u8>)>;

/// Make the two sets of icons for a repaint. All rendering and encoding is
/// here, so that it is outside the measured time. `only` limits the sets to
/// one slot. `format` replaces the format of the descriptor.
fn build_sets(
    d: &Descriptor,
    only: Option<u8>,
    format: Option<ImageFormat>,
    quality: u8,
) -> Res<[ImageSet; 2]> {
    let surfaces: Vec<&Surface> = match only {
        Some(slot) => vec![d.surface_at_slot(slot).ok_or_else(|| {
            let slots: Vec<String> = d
                .slot_surfaces()
                .filter_map(|s| s.slot.map(|n| n.to_string()))
                .collect();
            format!(
                "slot {slot} does not exist on this device (slots: {})",
                slots.join(", ")
            )
        })?],
        None => d.slot_surfaces().collect(),
    };
    let mut sets = [Vec::new(), Vec::new()];
    for (variant, set) in sets.iter_mut().enumerate() {
        for s in &surfaces {
            let slot = s.slot.ok_or("a surface in the repaint has no slot")?;
            let img = image::icon_image(
                s.width,
                s.height,
                slot,
                word_for(s, variant as u8),
                variant as u8,
            );
            let data = image::encode_as(s, &img, format.unwrap_or(s.format), quality)?;
            set.push((slot, data));
        }
    }
    Ok(sets)
}

/// The number of output reports for one repaint of images with the data
/// lengths `lens`: for each image the header and the parts, and one commit.
fn reports_per_repaint(lens: &[usize], packet_size: usize) -> usize {
    lens.iter()
        .map(|&n| 1 + n.div_ceil(packet_size))
        .sum::<usize>()
        + 1
}

/// The minimum, the median, and the maximum of some times.
#[derive(Debug, PartialEq, Eq)]
struct Stats {
    min: Duration,
    median: Duration,
    max: Duration,
}

fn summarize(times: &[Duration]) -> Option<Stats> {
    let mut sorted = times.to_vec();
    sorted.sort();
    let (&min, &max) = (sorted.first()?, sorted.last()?);
    let mid = sorted.len() / 2;
    let median = if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2
    };
    Some(Stats { min, median, max })
}

fn ms(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1000.0)
}

/// The result of the repaints.
struct RepaintResult {
    times: Vec<Duration>,
    acks: u64,
    other: u64,
}

/// Draw `count` times. Repaint `i` draws set `i % 2` and commits once. The
/// time of a repaint is from before the first write to after the commit. The
/// writes block, so the time is the time to give the reports to the device.
/// Everything else is outside the time: the pause, the keep-alive, and the
/// count of the input reports.
fn run_repaints<T: Transport>(
    device: &mut Device<T>,
    sets: &[ImageSet; 2],
    count: u64,
    pause: Duration,
    keeper: &mut Keeper,
) -> Res<RepaintResult> {
    let mut result = RepaintResult {
        times: Vec::new(),
        acks: 0,
        other: 0,
    };
    for i in 0..count {
        let which = (i % 2) as usize;
        let start = Instant::now();
        for (slot, data) in &sets[which] {
            device.send_image(*slot, data)?;
        }
        device.commit()?;
        let elapsed = start.elapsed();

        result.times.push(elapsed);
        println!(
            "  repaint {:>3}  set {}  {}",
            i + 1,
            ["A", "B"][which],
            ms(elapsed)
        );
        let (acks, other) = drain_input(device)?;
        result.acks += acks;
        result.other += other;
        keeper.tick(device, Instant::now())?;
        if !pause.is_zero() {
            std::thread::sleep(pause);
        }
    }
    Ok(result)
}

/// `repaint`: draw all surfaces many times, and print the time of each.
pub fn cmd_repaint(rest: &[String]) -> AnyResult {
    let args = Args::parse(
        rest,
        &value_options(&[
            "--count",
            "--format",
            "--quality",
            "--one",
            "--pause",
            "--brightness",
        ]),
        &["--wait-key", "--no-init"],
    )?;
    let count = args.number("--count", 1, 100_000)?.unwrap_or(20);
    let format = args.text("--format").map(parse_format).transpose()?;
    let quality = args
        .number("--quality", 1, 100)?
        .unwrap_or(DEFAULT_JPEG_QUALITY as u64) as u8;
    let only = args.number("--one", 0, 255)?.map(|s| s as u8);
    let pause = Duration::from_millis(args.number("--pause", 0, 60_000)?.unwrap_or(0));
    let hold_seconds = args.hold()?;
    let keep_alive = args.keep_alive()?;
    args.brightness()?;

    let mut device = open_device()?;
    let d = device.descriptor();

    // Render and encode first: this time is not in the measurement.
    let sets = build_sets(d, only, format, quality)?;

    let jpeg_used = format == Some(ImageFormat::Jpeg)
        || (format.is_none() && d.slot_surfaces().any(|s| s.format == ImageFormat::Jpeg));
    println!(
        "  format : {}{}",
        match format {
            Some(f) => format!("{} (set with --format)", f.name()),
            None => "from the descriptor".to_string(),
        },
        if jpeg_used {
            format!(", JPEG quality {quality}")
        } else {
            String::new()
        }
    );
    println!("  images : two sets, A and B. A repaint draws one set, then the other.");
    for (name, set) in ["A", "B"].iter().zip(&sets) {
        let lens: Vec<usize> = set.iter().map(|(_, data)| data.len()).collect();
        let reports = reports_per_repaint(&lens, d.packet_size);
        println!(
            "  set {name}  : {} image(s), {} bytes of image data, {reports} reports of {} bytes ({} bytes written)",
            lens.len(),
            lens.iter().sum::<usize>(),
            1 + d.packet_size,
            reports * (1 + d.packet_size)
        );
    }
    for ((slot, data), s) in sets[0].iter().zip(set_surfaces(d, only)) {
        println!(
            "           slot {slot:>2}  {:<12} {:>6} bytes",
            s.id.to_string(),
            data.len()
        );
    }

    let mut keeper = Keeper::new(keep_alive);
    println!("  {}", keeper.describe());
    begin(&mut device, &args)?;
    if args.has("--wait-key") {
        println!("\nPress a key on the device to start the repaints.");
        wait_for_key(&mut device, &mut keeper, WAIT_KEY_LIMIT)?;
    }
    println!("\nRepainting {count} times. The time is the time of the writes.\n");
    let result = run_repaints(&mut device, &sets, count, pause, &mut keeper)?;

    if let Some(stats) = summarize(&result.times) {
        println!(
            "\n  repaints : {}\n  minimum  : {}\n  median   : {}\n  maximum  : {}",
            result.times.len(),
            ms(stats.min),
            ms(stats.median),
            ms(stats.max)
        );
    }
    println!(
        "  input reports during the repaints: {} acknowledgement(s), {} other",
        result.acks, result.other
    );
    println!(
        "  The last repaint drew set {}.",
        ["A", "B"][((count - 1) % 2) as usize]
    );
    hold(&mut device, hold_seconds.unwrap_or(0), &mut keeper)
}

/// The surfaces of a repaint, in the same order as the images.
fn set_surfaces(d: &Descriptor, only: Option<u8>) -> Vec<&'static Surface> {
    match only {
        Some(slot) => d.surface_at_slot(slot).into_iter().collect(),
        None => d.slot_surfaces().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencraft_core::FakeTransport;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn n4pro() -> &'static Descriptor {
        opencraft_core::device::find(0x5548, 0x1023).unwrap()
    }

    fn fake() -> Device<FakeTransport> {
        Device::new(FakeTransport::new(), n4pro())
    }

    /// An input report for the control code `code` and the state `state`.
    fn report(code: u8, state: u8) -> Vec<u8> {
        let mut r = vec![0u8; 512];
        r[..3].copy_from_slice(b"ACK");
        r[5..7].copy_from_slice(b"OK");
        r[9] = code;
        r[10] = state;
        r
    }

    /// The operation code of an output report, if it has the prefix.
    fn op(report: &[u8]) -> &[u8] {
        assert_eq!(&report[..6], &[0x00, b'C', b'R', b'T', 0x00, 0x00]);
        let end = report[6..].iter().position(|&b| b == 0).unwrap_or(0);
        &report[6..6 + end]
    }

    // --- Args ---

    #[test]
    fn args_split_positional_values_and_flags() {
        let a = Args::parse(
            &args(&["brightness", "0", "50", "--each", "3", "--draw"]),
            &["--each"],
            &["--draw"],
        )
        .unwrap();
        assert_eq!(a.positional, ["brightness", "0", "50"]);
        assert_eq!(a.text("--each"), Some("3"));
        assert!(a.has("--draw"));
        assert!(!a.has("--wait-key"));
    }

    #[test]
    fn args_refuse_an_unknown_option_and_a_missing_value() {
        let e = Args::parse(&args(&["--holld", "5"]), &["--hold"], &[]).unwrap_err();
        assert!(e.contains("--holld"), "{e}");
        let e = Args::parse(&args(&["--hold"]), &["--hold"], &[]).unwrap_err();
        assert!(e.contains("needs a value"), "{e}");
        // The next word is an option, not the value.
        let e = Args::parse(&args(&["--hold", "--draw"]), &["--hold"], &["--draw"]).unwrap_err();
        assert!(e.contains("needs a value"), "{e}");
    }

    #[test]
    fn args_numbers_have_limits() {
        let a = Args::parse(&args(&["--count", "0"]), &["--count"], &[]).unwrap();
        assert!(a.number("--count", 1, 100).is_err());
        let a = Args::parse(&args(&["--count", "101"]), &["--count"], &[]).unwrap();
        assert!(a.number("--count", 1, 100).is_err());
        let a = Args::parse(&args(&["--count", "x"]), &["--count"], &[]).unwrap();
        assert!(a.number("--count", 1, 100).is_err());
        let a = Args::parse(&args(&["--count", "20"]), &["--count"], &[]).unwrap();
        assert_eq!(a.number("--count", 1, 100), Ok(Some(20)));
        assert_eq!(a.number("--other", 1, 100), Ok(None));
    }

    #[test]
    fn keep_alive_option_default_zero_and_value() {
        let parse = |list: &[&str]| {
            Args::parse(&args(list), &["--keep-alive"], &[])
                .unwrap()
                .keep_alive()
        };
        assert_eq!(parse(&[]), Ok(Some(KEEP_ALIVE_INTERVAL)));
        assert_eq!(parse(&["--keep-alive", "0"]), Ok(None));
        assert_eq!(
            parse(&["--keep-alive", "25"]),
            Ok(Some(Duration::from_secs(25)))
        );
        assert!(parse(&["--keep-alive", "ten"]).is_err());
    }

    #[test]
    fn keep_alive_flag_for_test_listen_and_raw() {
        assert_eq!(keep_alive_flag(&args(&["--seconds", "5"])), Ok(None));
        assert_eq!(
            keep_alive_flag(&args(&["--keep-alive", "0"])),
            Ok(Some(Duration::ZERO))
        );
        assert_eq!(
            keep_alive_flag(&args(&["--seconds", "5", "--keep-alive", "30"])),
            Ok(Some(Duration::from_secs(30)))
        );
        // A wrong value is an error, not the default interval.
        assert!(keep_alive_flag(&args(&["--keep-alive", "abc"])).is_err());
        assert!(keep_alive_flag(&args(&["--keep-alive"])).is_err());
    }

    #[test]
    fn size_format_and_percent_parsing() {
        assert_eq!(parse_size("352x112"), Ok((352, 112)));
        assert_eq!(parse_size("800X112"), Ok((800, 112)));
        for wrong in [
            "352", "x112", "352x", "0x112", "352x0", "70000x1", "axb", "1x2x3",
        ] {
            assert!(parse_size(wrong).is_err(), "{wrong}");
        }
        assert_eq!(parse_format("PNG"), Ok(ImageFormat::Png));
        assert_eq!(parse_format("jpeg"), Ok(ImageFormat::Jpeg));
        assert_eq!(parse_format("jpg"), Ok(ImageFormat::Jpeg));
        assert!(parse_format("gif").is_err());
        assert_eq!(parse_percent("0"), Ok(0));
        assert_eq!(parse_percent("100"), Ok(100));
        assert!(parse_percent("101").is_err());
        assert!(parse_percent("-1").is_err());
        assert!(parse_percent("").is_err());
    }

    // --- send ---

    #[test]
    fn send_kinds_have_names_and_protocol_codes() {
        for (name, kind) in [
            ("brightness", SendKind::Brightness),
            ("LIG", SendKind::Brightness),
            ("knob-colors", SendKind::KnobColors),
            ("setlb", SendKind::KnobColors),
            ("knob-brightness", SendKind::KnobBrightness),
            ("lblig", SendKind::KnobBrightness),
            ("sleep", SendKind::Sleep),
            ("han", SendKind::Sleep),
            ("keep-alive", SendKind::KeepAlive),
            ("connect", SendKind::KeepAlive),
            ("start", SendKind::Start),
            ("dis", SendKind::Start),
            ("commit", SendKind::Commit),
            ("stp", SendKind::Commit),
            ("haptic", SendKind::Haptic),
            ("qucmd", SendKind::Haptic),
        ] {
            assert_eq!(SendKind::parse(name), Ok(kind), "{name}");
        }
        assert!(SendKind::parse("logo").is_err());
        // The boot logo command does not exist in the probe.
        assert!(SendKind::parse("log").is_err());
    }

    #[test]
    fn send_brightness_makes_one_step_for_each_value() {
        let steps = send_steps(SendKind::Brightness, &args(&["0", "50", "100"]), Some(4)).unwrap();
        assert_eq!(steps.len(), 3);
        for (step, percent) in steps.iter().zip([0, 50, 100]) {
            assert_eq!(step.command, protocol::set_brightness(percent));
        }

        let steps = send_steps(SendKind::KnobBrightness, &args(&["30"]), Some(4)).unwrap();
        assert_eq!(steps[0].command, protocol::set_led_brightness(30));

        assert!(send_steps(SendKind::Brightness, &[], Some(4)).is_err());
        assert!(send_steps(SendKind::Brightness, &args(&["101"]), Some(4)).is_err());
        assert!(send_steps(SendKind::KnobBrightness, &args(&["50", "x"]), Some(4)).is_err());
    }

    #[test]
    fn send_knob_colors_needs_one_colour_for_each_knob() {
        let colors = args(&["ff0000", "00ff00", "#0000ff", "ffffff"]);
        let steps = send_steps(SendKind::KnobColors, &colors, Some(4)).unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(
            steps[0].command,
            protocol::set_led_colors(&[[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]])
        );
        let e = send_steps(SendKind::KnobColors, &colors[..3], Some(4)).unwrap_err();
        assert!(e.contains("4 knobs"), "{e}");
        for invalid in ["ff00", "gg0000"] {
            assert!(send_steps(SendKind::KnobColors, &args(&[invalid]), Some(1)).is_err());
        }
        assert!(send_steps(SendKind::KnobColors, &colors, Some(0)).is_err());
        // Before the device opens, the number of colours is not checked. A
        // colour that is not correct is an error at once.
        assert!(send_steps(SendKind::KnobColors, &colors[..3], None).is_ok());
        assert!(send_steps(SendKind::KnobColors, &args(&["red"]), None).is_err());
    }

    #[test]
    fn send_knob_colors_refuses_non_ascii_without_panicking() {
        // Six bytes, but the first two-byte slice ends inside a character.
        assert!(send_steps(SendKind::KnobColors, &args(&["a€bc"]), Some(1)).is_err());
    }

    #[test]
    fn send_commands_without_arguments_and_haptic() {
        for (kind, want) in [
            (SendKind::Sleep, protocol::sleep()),
            (SendKind::KeepAlive, protocol::keep_alive()),
            (SendKind::Start, protocol::init()),
            (SendKind::Commit, protocol::commit()),
        ] {
            let steps = send_steps(kind, &[], Some(4)).unwrap();
            assert_eq!(steps[0].command, want);
            assert!(send_steps(kind, &args(&["1"]), Some(4)).is_err());
        }
        let on = send_steps(SendKind::Haptic, &args(&["ON"]), Some(4)).unwrap();
        assert_eq!(on[0].command, protocol::set_vibration(true));
        let off = send_steps(SendKind::Haptic, &args(&["off"]), Some(4)).unwrap();
        assert_eq!(off[0].command, protocol::set_vibration(false));
        assert!(send_steps(SendKind::Haptic, &[], Some(4)).is_err());
        assert!(send_steps(SendKind::Haptic, &args(&["maybe"]), Some(4)).is_err());
    }

    #[test]
    fn run_send_writes_one_report_for_each_step_and_nothing_else() {
        let mut device = fake();
        let steps = send_steps(SendKind::Brightness, &args(&["0", "50", "100"]), Some(4)).unwrap();
        let timing = SendTiming {
            wait_key: false,
            each: Duration::ZERO,
        };
        run_send(&mut device, &steps, &timing, &mut Keeper::new(None)).unwrap();
        let writes = device.transport().writes();
        assert_eq!(writes.len(), 3);
        for (w, p) in writes.iter().zip([0u8, 50, 100]) {
            assert_eq!(w.len(), 1025);
            assert_eq!(op(w), b"LIG");
            assert_eq!(w[11], p);
        }
    }

    #[test]
    fn run_send_with_wait_key_sends_after_the_key_press() {
        let mut device = fake();
        // No release, knob, tap, acknowledgement or unknown report starts
        // the send. The key press is the last report before the marker.
        for r in [
            report(0x01, 0x00),
            report(0xA0, 0x00),
            report(0x35, 0x01),
            report(0x40, 0x00),
            report(0xFF, 0x00),
            b"not a report".to_vec(),
            report(0x01, 0x01),
            vec![9],
        ] {
            device.transport_mut().push_input(r);
        }
        let steps = send_steps(SendKind::Sleep, &[], Some(4)).unwrap();
        let timing = SendTiming {
            wait_key: true,
            each: Duration::ZERO,
        };
        run_send(&mut device, &steps, &timing, &mut Keeper::new(None)).unwrap();
        let writes = device.transport().writes();
        assert_eq!(writes.len(), 1);
        assert_eq!(op(&writes[0]), b"HAN");
        assert_eq!(device.read(Duration::ZERO).unwrap(), Some(vec![9]));
    }

    // --- keep-alive, hold, and wait ---

    #[test]
    fn keeper_tick_sends_connect_and_starts_the_interval_again() {
        let mut device = fake();
        let mut keeper = Keeper::new(Some(Duration::from_secs(10)));
        let t0 = keeper.last;
        assert_eq!(
            keeper.next_in(t0 + Duration::from_secs(4)),
            Some(Duration::from_secs(6))
        );
        assert_eq!(
            keeper.next_in(t0 + Duration::from_secs(40)),
            Some(Duration::ZERO)
        );
        assert!(!keeper
            .tick(&mut device, t0 + Duration::from_secs(9))
            .unwrap());
        assert!(device.transport().writes().is_empty());

        let t1 = t0 + Duration::from_secs(10);
        assert!(keeper.tick(&mut device, t1).unwrap());
        assert_eq!(device.transport().writes().len(), 1);
        assert_eq!(op(&device.transport().writes()[0]), b"CONNECT");
        // The interval starts again from the keep-alive.
        assert!(!keeper
            .tick(&mut device, t1 + Duration::from_secs(9))
            .unwrap());
        assert!(keeper
            .tick(&mut device, t1 + Duration::from_secs(10))
            .unwrap());
        assert_eq!(device.transport().writes().len(), 2);

        let mut never = Keeper::new(None);
        assert_eq!(never.next_in(t0), None);
        assert!(!never
            .tick(&mut device, t0 + Duration::from_secs(10_000))
            .unwrap());
        assert_eq!(device.transport().writes().len(), 2);
    }

    #[test]
    fn idle_sends_keep_alives_and_counts_input() {
        let mut device = fake();
        device.transport_mut().push_input(report(0x01, 0x01));
        device.transport_mut().push_input(report(0xA0, 0x00));
        let mut keeper = Keeper::new(Some(Duration::from_millis(5)));
        let n = idle(&mut device, &mut keeper, Duration::from_millis(60)).unwrap();
        assert_eq!(n, 2);
        let writes = device.transport().writes();
        assert!(!writes.is_empty(), "no keep-alive");
        assert!(writes.iter().all(|w| op(w) == b"CONNECT"));
    }

    #[test]
    fn idle_without_keep_alive_writes_nothing() {
        let mut device = fake();
        let mut keeper = Keeper::new(None);
        idle(&mut device, &mut keeper, Duration::from_millis(20)).unwrap();
        assert!(device.transport().writes().is_empty());
    }

    #[test]
    fn hold_for_zero_seconds_does_nothing() {
        let mut device = fake();
        hold(&mut device, 0, &mut Keeper::new(Some(Duration::ZERO))).unwrap();
        assert!(device.transport().writes().is_empty());
    }

    #[test]
    fn hold_reports_a_lost_connection() {
        let mut device = fake();
        device.transport_mut().disconnect();
        assert!(hold(&mut device, 1, &mut Keeper::new(None)).is_err());
    }

    #[test]
    fn wait_for_key_gives_an_error_at_the_limit() {
        let mut device = fake();
        device.transport_mut().push_input(report(0x35, 0x01)); // a knob push
        let e = wait_for_key(
            &mut device,
            &mut Keeper::new(None),
            Duration::from_millis(30),
        );
        assert!(e.unwrap_err().to_string().contains("no key press"));
    }

    // --- bars and wide ---

    #[test]
    fn bar_edge_images_have_one_image_for_each_bar_panel_of_the_descriptor() {
        let panels = bar_panels(n4pro());
        assert_eq!(panels.len(), 4);
        let images = bar_edge_images(&panels).unwrap();
        let slots: Vec<u8> = images.iter().map(|(s, _)| *s).collect();
        assert_eq!(slots, [1, 2, 3, 4]);
        for (_, data) in &images {
            assert_eq!(&data[..4], b"\x89PNG");
        }
        assert_ne!(images[0].1, images[1].1);
    }

    #[test]
    fn draw_images_sends_each_image_and_then_the_commit() {
        let mut device = fake();
        let images = vec![(1u8, vec![0xAA; 10]), (2, vec![0xBB; 2000])];
        draw_images(&mut device, &images).unwrap();
        let ops: Vec<Vec<u8>> = device
            .transport()
            .writes()
            .iter()
            .map(|w| {
                // A part of image data has no prefix.
                if w.len() > 6 && &w[1..4] == b"CRT" {
                    op(w).to_vec()
                } else {
                    b"data".to_vec()
                }
            })
            .collect();
        let ops: Vec<&[u8]> = ops.iter().map(Vec::as_slice).collect();
        let (bat, data, stp): (&[u8], &[u8], &[u8]) = (b"BAT", b"data", b"STP");
        assert_eq!(ops, [bat, data, bat, data, data, stp]);
    }

    // --- repaint ---

    #[test]
    fn build_sets_makes_two_different_sets_for_all_surfaces_with_a_slot() {
        let d = n4pro();
        let [a, b] = build_sets(d, None, None, 60).unwrap();
        assert_eq!(a.len(), 14);
        assert_eq!(b.len(), 14);
        let slots: Vec<u8> = a.iter().map(|(s, _)| *s).collect();
        let want: Vec<u8> = d.slot_surfaces().filter_map(|s| s.slot).collect();
        assert_eq!(slots, want);
        assert_ne!(a, b);
        // The images of the surfaces are different from each other.
        assert_ne!(a[0].1, a[1].1);
        assert_ne!(a[4].1, a[5].1);
    }

    #[test]
    fn build_sets_uses_the_format_of_the_descriptor_or_the_override() {
        let d = n4pro();
        let [png, _] = build_sets(d, None, None, 60).unwrap();
        assert!(png.iter().all(|(_, data)| data.starts_with(b"\x89PNG")));

        let [jpeg, _] = build_sets(d, None, Some(ImageFormat::Jpeg), 60).unwrap();
        assert!(jpeg
            .iter()
            .all(|(_, data)| data.starts_with(&[0xFF, 0xD8, 0xFF])));

        let [back, _] = build_sets(d, None, Some(ImageFormat::Png), 60).unwrap();
        assert_eq!(back, png);
    }

    #[test]
    fn build_sets_jpeg_quality_changes_the_size() {
        let d = n4pro();
        let size = |q| {
            let [a, _] = build_sets(d, None, Some(ImageFormat::Jpeg), q).unwrap();
            a.iter().map(|(_, data)| data.len()).sum::<usize>()
        };
        assert!(size(95) > size(30));
    }

    #[test]
    fn build_sets_with_one_slot_has_one_image_and_refuses_an_unknown_slot() {
        let d = n4pro();
        let [a, b] = build_sets(d, Some(6), None, 60).unwrap();
        assert_eq!((a.len(), b.len()), (1, 1));
        assert_eq!(a[0].0, 6);
        assert_eq!(set_surfaces(d, Some(6)).len(), 1);

        let e = build_sets(d, Some(5), None, 60).unwrap_err().to_string();
        assert!(e.contains("slot 5") && e.contains("slots:"), "{e}");
    }

    #[test]
    fn report_count_of_a_repaint() {
        // An image of 1 byte: a header and 1 part. 1024 bytes: a header and
        // 1 part. 1025 bytes: a header and 2 parts. Then the commit.
        assert_eq!(reports_per_repaint(&[1, 1024, 1025], 1024), 2 + 2 + 3 + 1);
        assert_eq!(reports_per_repaint(&[], 1024), 1);
    }

    #[test]
    fn statistics_of_the_times() {
        let t = |ms| Duration::from_millis(ms);
        assert_eq!(summarize(&[]), None);
        assert_eq!(
            summarize(&[t(7)]),
            Some(Stats {
                min: t(7),
                median: t(7),
                max: t(7)
            })
        );
        // Odd count: the middle value.
        assert_eq!(
            summarize(&[t(30), t(10), t(20)]),
            Some(Stats {
                min: t(10),
                median: t(20),
                max: t(30)
            })
        );
        // Even count: the mean of the two middle values.
        assert_eq!(
            summarize(&[t(40), t(10), t(30), t(20)]),
            Some(Stats {
                min: t(10),
                median: t(25),
                max: t(40)
            })
        );
    }

    #[test]
    fn run_repaints_draws_the_two_sets_in_turn_and_commits_each_time() {
        let mut device = fake();
        let sets = [
            vec![(6u8, vec![0xAA; 100]), (7, vec![0xAA; 1500])],
            vec![(6u8, vec![0xBB; 100]), (7, vec![0xBB; 1500])],
        ];
        let mut keeper = Keeper::new(None);
        let result = run_repaints(&mut device, &sets, 3, Duration::ZERO, &mut keeper).unwrap();
        assert_eq!(result.times.len(), 3);

        let writes = device.transport().writes();
        let per = reports_per_repaint(&[100, 1500], 1024);
        assert_eq!(writes.len(), 3 * per);
        // Each repaint ends with the commit, and only there.
        for r in 0..3 {
            assert_eq!(op(&writes[(r + 1) * per - 1]), b"STP");
        }
        assert_eq!(writes.iter().filter(|w| op_is(w, b"STP")).count(), 3);
        // Repaint 1 and 3 draw set A, and repaint 2 draws set B.
        let first_data = |r: usize| writes[r * per + 1][1];
        assert_eq!(first_data(0), 0xAA);
        assert_eq!(first_data(1), 0xBB);
        assert_eq!(first_data(2), 0xAA);
    }

    fn op_is(report: &[u8], code: &[u8]) -> bool {
        report.len() > 6 + code.len()
            && report[..6] == [0x00, b'C', b'R', b'T', 0x00, 0x00]
            && &report[6..6 + code.len()] == code
    }

    #[test]
    fn run_repaints_counts_the_input_reports_and_sends_the_keep_alive() {
        let mut device = fake();
        device.transport_mut().push_input(report(0xFF, 0x00));
        device.transport_mut().push_input(report(0xFF, 0x00));
        device.transport_mut().push_input(report(0x01, 0x01));
        device.transport_mut().push_input(b"unknown".to_vec());
        let sets = [vec![(6u8, vec![1; 10])], vec![(6u8, vec![2; 10])]];
        // The interval is zero in effect: every tick sends the keep-alive.
        let mut keeper = Keeper::new(Some(Duration::from_nanos(1)));
        let result = run_repaints(&mut device, &sets, 2, Duration::ZERO, &mut keeper).unwrap();
        assert_eq!((result.acks, result.other), (2, 2));
        let writes = device.transport().writes();
        assert_eq!(writes.iter().filter(|w| op_is(w, b"CONNECT")).count(), 2);
    }

    #[test]
    fn run_repaints_stops_with_an_error_when_a_write_fails() {
        let mut device = fake();
        device.transport_mut().disconnect();
        let sets = [vec![(6u8, vec![1; 10])], vec![(6u8, vec![2; 10])]];
        let r = run_repaints(
            &mut device,
            &sets,
            3,
            Duration::ZERO,
            &mut Keeper::new(None),
        );
        assert!(r.is_err());
    }
}
