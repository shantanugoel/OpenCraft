//! opencraft-probe — hardware bring-up tool.
//!
//! Confirms that we can talk to a StreamDock-class device without the vendor
//! software: enumerate it, draw on every panel, and decode every input.

use opencraft_core::{
    events, image, list_devices, protocol, recording, select, Device, Error, Event, Found,
    HidTransport,
};
use std::io::Write;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod record;

type AnyResult = Result<(), Box<dyn std::error::Error>>;

fn main() -> ExitCode {
    // Behave like a normal Unix tool when stdout is piped into `head`:
    // die quietly on SIGPIPE instead of panicking with exit code 101.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let rest = &args[1..];

    let result = match cmd {
        "list" | "ls" => cmd_list(),
        "identify" | "info" => cmd_identify(),
        "test" => cmd_test(rest),
        "listen" => cmd_listen(rest),
        "draw" => cmd_draw(rest),
        "clear" => cmd_clear(rest),
        "background" | "bg" => cmd_background(rest),
        "raw" => cmd_raw(rest),
        "record" => record::cmd_record(rest),
        "help" | "--help" | "-h" => {
            usage();
            Ok(())
        }
        other => {
            eprintln!("unknown command: {other}\n");
            usage();
            return ExitCode::FAILURE;
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\n  error: {e}\n");
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    println!(
        "opencraft-probe — StreamDock hardware bring-up

USAGE:
    opencraft-probe <command> [options]

COMMANDS:
    list                 Enumerate HID devices and flag known controllers
    identify             Open the device and dump its identity + firmware
    test                 Draw a numbered panel on every slot, then decode input
    listen               Decode input events only
    draw <slot> <rrggbb> Paint one slot a solid colour
    clear [slot]         Send the clear command for one slot, or for all slots
    background           Draw a test pattern on the background (800x480)
    raw                  Hexdump raw input reports; can save them to a file
    record               Record each gesture to a file, one prompt at a time
                         (--out <dir>, --from <step>)

OPTIONS:
    --seconds <n>        How long to listen (default: 30 for test, 15 for listen)
    --brightness <0-100> Panel brightness for 'test' (default: 80)
    --mode <0-9>         Send a MODE command first (some rebrands need it)
    --keep               Leave the test pattern on screen when exiting
    --verbose            Also print acks and unrecognised packets

OPTIONS FOR 'raw':
    --save <file>        Write each full input report to <file> (text format)
    --note <text>        Say in the file which gesture it holds
    --until-enter        Stop when you press Enter ('--seconds' is the time
                         limit; default: 10)
    --draw               Draw the slot numbers first, as 'test' does
    --keep-alive <n>     Send the keep-alive command each <n> seconds

OPTIONS FOR 'background':
    --file <path>        Draw this PNG or JPEG file, not the test pattern
    --quality <1-100>    JPEG quality (default: 60, the value of the vendor SDK)
    --with-panels        Then draw the numbered key faces and bar panels on top
    --clear              Remove the background image. Draw nothing.

NOTE: quit VSD Craft before running — two apps driving one device will fight."
    );
}

fn open_device() -> Result<Device<HidTransport>, Box<dyn std::error::Error>> {
    let found: Vec<Found> = list_devices()?;

    // The device layer chooses the interface. It chooses only a vendor
    // interface of a known device.
    match select(&found) {
        Ok(f) => {
            println!("  device : {} (0x{:04X}:0x{:04X})", f.label(), f.vid, f.pid);
            if let Some(serial) = &f.serial {
                println!("  serial : {serial}");
            }
            println!(
                "  iface  : interface {} / usage page 0x{:04X}\n",
                f.interface, f.usage_page
            );
            Ok(Device::open(f)?)
        }
        Err(Error::NoVendorInterface) => {
            eprintln!("A known StreamDock device is connected, but it has no vendor interface.\n");
            eprintln!("Interfaces of the known device:");
            for f in found.iter().filter(|f| f.known.is_some()) {
                eprintln!(
                    "  - interface {} / usage page 0x{:04X}",
                    f.interface, f.usage_page
                );
            }
            eprintln!("\nOpenCraft opens only the vendor interface. It does not open the");
            eprintln!("keyboard interface, and it does not need Input Monitoring.");
            eprintln!("Connect the device again. If the error stays, run");
            eprintln!("'opencraft-probe list' and look at the interfaces.");
            Err(Error::NoVendorInterface.into())
        }
        Err(Error::DeviceNotFound) => {
            eprintln!("No known StreamDock device found.\n");
            eprintln!("Checklist:");
            eprintln!("  - is the device plugged in?");
            eprintln!("  - is VSD Craft still running and holding the interface?");
            eprintln!("  - run 'opencraft-probe list' to see what is on the bus");
            Err("device not found".into())
        }
        Err(e) => Err(e.into()),
    }
}

fn cmd_list() -> AnyResult {
    let found = list_devices()?;

    println!(
        "{:<22} {:<10} {:<6} {:<7} STATE",
        "PRODUCT", "VID:PID", "IFACE", "USAGE"
    );
    println!("{}", "-".repeat(78));
    for f in &found {
        let state = match f.known {
            Some(_) if f.is_vendor_interface() => "SUPPORTED",
            Some(_) => "supported (other iface)",
            None if opencraft_core::device::KNOWN_VIDS.contains(&f.vid) => "known vendor",
            None => "",
        };
        println!(
            "{:<22} {:04X}:{:<5} {:<6} 0x{:<5} {}",
            truncate(&f.product, 22),
            f.vid,
            format!("{:04X}", f.pid),
            f.interface,
            format!("{:04X}", f.usage_page),
            state
        );
    }
    println!("\n{} HID device(s) on the bus.", found.len());
    Ok(())
}

fn cmd_identify() -> AnyResult {
    let mut t = open_device()?;
    let d = t.descriptor();
    println!("  panels : {}", d.panels.len());
    println!("  packet : {} bytes", d.packet_size);
    if let Some(bg) = d.background {
        println!("  background: {}x{}", bg.width, bg.height);
    }
    match t.firmware_version() {
        Ok(v) if !v.is_empty() => println!("  firmware: {v}"),
        _ => println!("  firmware: (unavailable)"),
    }
    println!();
    for p in d.panels {
        println!(
            "    slot {:>2}  {:>3}x{:<3}  {:?}",
            p.slot, p.width, p.height, p.kind
        );
    }
    Ok(())
}

fn cmd_test(rest: &[String]) -> AnyResult {
    let seconds = flag_u64(rest, "--seconds").unwrap_or(30);
    let brightness = flag_u64(rest, "--brightness").unwrap_or(80) as u8;
    let keep = rest.iter().any(|a| a == "--keep");
    let verbose = rest.iter().any(|a| a == "--verbose");

    let mut t = open_device()?;

    if let Some(mode) = flag_u64(rest, "--mode") {
        println!("  mode   : sending MODE {mode}");
        t.send(protocol::set_mode(mode as u8))?;
        std::thread::sleep(Duration::from_millis(300));
    }

    println!("  init   : clearing framebuffer");
    t.init()?;
    std::thread::sleep(Duration::from_millis(150));

    println!("  light  : brightness {brightness}%");
    t.set_brightness(brightness)?;

    let mut led_colors = Vec::new();
    for (i, panel) in t.descriptor().panels.iter().enumerate() {
        let png =
            image::render_test_panel(panel.width, panel.height, panel.slot, image::test_color(i))?;
        t.send_image(panel.slot, &png)?;
        if panel.kind == opencraft_core::PanelKind::Knob {
            led_colors.push(image::test_color(i));
        }
    }

    if !led_colors.is_empty() {
        t.send(protocol::set_led_colors(&led_colors))?;
    }

    t.commit()?;
    println!(
        "  drawn  : {} panels committed\n",
        t.descriptor().panels.len()
    );
    println!("Each display now shows its slot number.");
    println!("Press keys / turn knobs / swipe the touch bar for {seconds}s.\n");

    pump(&mut t, Duration::from_secs(seconds), verbose)?;

    if keep {
        println!("\nLeaving the test pattern on screen.");
    } else {
        println!("\nRestoring: blanking panels.");
        t.set_brightness(60)?;
        t.clear_all()?;
    }
    Ok(())
}

fn cmd_listen(rest: &[String]) -> AnyResult {
    let seconds = flag_u64(rest, "--seconds").unwrap_or(15);
    let verbose = rest.iter().any(|a| a == "--verbose");
    let mut t = open_device()?;
    println!("Listening for {seconds}s. Ctrl-C to stop.\n");
    pump(&mut t, Duration::from_secs(seconds), verbose)
}

fn cmd_raw(rest: &[String]) -> AnyResult {
    let seconds = flag_u64(rest, "--seconds").unwrap_or(10);
    let save = flag_str(rest, "--save");
    let note = flag_str(rest, "--note").unwrap_or("");
    let until_enter = rest.iter().any(|a| a == "--until-enter");
    let draw = rest.iter().any(|a| a == "--draw");
    let keep_alive = flag_u64(rest, "--keep-alive").map(Duration::from_secs);
    let mut t = open_device()?;

    // The file never gets the serial number: the header has the model name
    // and the firmware version only.
    let mut file = match save {
        Some(path) => {
            let firmware = t.firmware_version().unwrap_or_default();
            let firmware = if firmware.is_empty() {
                "(unavailable)".to_string()
            } else {
                firmware
            };
            let mut f =
                std::fs::File::create(path).map_err(|e| format!("cannot write {path}: {e}"))?;
            f.write_all(recording::header(note, t.descriptor().name, &firmware).as_bytes())?;
            Some(f)
        }
        None => None,
    };

    if draw {
        draw_test_pattern(&mut t, None, 80)?;
    }

    let stop = Arc::new(AtomicBool::new(false));
    if until_enter {
        watch_enter(Arc::clone(&stop));
        println!("Hexdumping raw input reports. Press Enter to stop ({seconds}s at most).\n");
    } else {
        println!("Hexdumping raw input reports for {seconds}s.\n");
    }

    let start = Instant::now();
    let deadline = start + Duration::from_secs(seconds);
    let mut errors = 0u32;
    let mut saved = 0u64;
    let mut last_keep_alive = Instant::now();
    while Instant::now() < deadline && !stop.load(Ordering::Relaxed) {
        if errors >= 10 {
            return Err("too many consecutive read errors".into());
        }
        if let Some(interval) = keep_alive {
            if last_keep_alive.elapsed() >= interval {
                t.keep_alive()?;
                last_keep_alive = Instant::now();
            }
        }
        if let Some(data) = read_tolerant(&mut t, &mut errors) {
            let ms = start.elapsed().as_millis() as u64;
            println!("[{}] {}", data.len(), hex(&data[..data.len().min(32)]));
            if let Some(f) = file.as_mut() {
                let line = recording::format_report_line(ms, &data) + "\n";
                f.write_all(line.as_bytes())?;
                saved += 1;
            }
        }
    }
    if let Some(path) = save {
        println!("\nSaved {saved} report(s) to {path}");
    }
    Ok(())
}

/// Draw a numbered panel on every slot.
fn draw_test_pattern(
    t: &mut Device<HidTransport>,
    mode: Option<u8>,
    brightness: u8,
) -> opencraft_core::Result<usize> {
    if let Some(mode) = mode {
        t.send(protocol::set_mode(mode))?;
        std::thread::sleep(Duration::from_millis(300));
    }

    t.init()?;
    std::thread::sleep(Duration::from_millis(150));
    t.set_brightness(brightness)?;

    let mut led_colors = Vec::new();
    for (i, panel) in t.descriptor().panels.iter().enumerate() {
        let png =
            image::render_test_panel(panel.width, panel.height, panel.slot, image::test_color(i))?;
        t.send_image(panel.slot, &png)?;
        if panel.kind == opencraft_core::PanelKind::Knob {
            led_colors.push(image::test_color(i));
        }
    }

    if !led_colors.is_empty() {
        t.send(protocol::set_led_colors(&led_colors))?;
    }

    t.commit()?;
    Ok(t.descriptor().panels.len())
}

/// Set `stop` when the user presses Enter. If standard input is closed, do
/// nothing: the time limit still applies.
fn watch_enter(stop: Arc<AtomicBool>) {
    // Drop an Enter key that the user typed before the recording started.
    #[cfg(unix)]
    unsafe {
        libc::tcflush(0, libc::TCIFLUSH);
    }
    std::thread::spawn(move || {
        let mut line = String::new();
        if matches!(std::io::stdin().read_line(&mut line), Ok(n) if n > 0) {
            stop.store(true, Ordering::Relaxed);
        }
    });
}

fn cmd_draw(rest: &[String]) -> AnyResult {
    if rest.len() < 2 {
        return Err("usage: opencraft-probe draw <slot> <rrggbb>".into());
    }
    let slot: u8 = rest[0].parse()?;
    let rgb = parse_hex_color(&rest[1])?;

    let mut t = open_device()?;
    let panel = t
        .descriptor()
        .panel(slot)
        .ok_or_else(|| format!("slot {slot} does not exist on this device"))?;

    t.init()?;
    let png = image::render_solid(panel.width, panel.height, rgb)?;
    t.send_image(slot, &png)?;
    t.commit()?;
    println!(
        "  painted slot {slot} with #{:02X}{:02X}{:02X}",
        rgb[0], rgb[1], rgb[2]
    );
    Ok(())
}

fn cmd_clear(rest: &[String]) -> AnyResult {
    let slot: Option<u8> = match rest.first() {
        Some(text) => Some(
            text.parse()
                .map_err(|_| "usage: opencraft-probe clear [slot]")?,
        ),
        None => None,
    };
    let mut t = open_device()?;
    match slot {
        Some(slot) => {
            t.send(protocol::clear_slot(slot))?;
            t.commit()?;
            println!("  cleared slot {slot}");
        }
        None => {
            t.clear_all()?;
            println!("  cleared all panels");
        }
    }
    Ok(())
}

/// Draw the background: the test pattern or an image file. This sends the
/// temporary background command ("BGPIC"), never the boot logo command.
fn cmd_background(rest: &[String]) -> AnyResult {
    let quality = flag_u64(rest, "--quality")
        .unwrap_or(image::BACKGROUND_JPEG_QUALITY as u64)
        .clamp(1, 100) as u8;
    let with_panels = rest.iter().any(|a| a == "--with-panels");
    let clear = rest.iter().any(|a| a == "--clear");

    // Read the file before the device opens, so that a bad path stops early.
    let file = match flag_str(rest, "--file") {
        Some(path) => Some(std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?),
        None if rest.iter().any(|a| a == "--file") => {
            return Err("usage: opencraft-probe background --file <path>".into())
        }
        None => None,
    };

    let mut t = open_device()?;
    let bg = t
        .descriptor()
        .background
        .ok_or("this device has no background in its descriptor")?;

    if clear {
        t.clear_background()?;
        t.commit()?;
        println!("  background: cleared (BGCLE 0x03, then STP)");
        return Ok(());
    }

    let jpeg = match &file {
        Some(data) => image::render_background_file(data, bg.width, bg.height, quality)?,
        None => image::render_background_test(bg.width, bg.height, quality)?,
    };
    println!(
        "  background: {}x{} JPEG, quality {quality}, {} bytes ({} the 2-byte length limit of 65535)",
        bg.width,
        bg.height,
        jpeg.len(),
        if jpeg.len() > 0xFFFF { "above" } else { "below" }
    );

    t.send_background(&jpeg)?;
    t.commit()?;
    println!("  drawn  : background committed");

    if with_panels {
        // A short pause makes the two steps visible one after the other.
        std::thread::sleep(Duration::from_secs(1));
        for (i, panel) in t.descriptor().panels.iter().enumerate() {
            let png = image::render_test_panel(
                panel.width,
                panel.height,
                panel.slot,
                image::test_color(i),
            )?;
            t.send_image(panel.slot, &png)?;
        }
        t.commit()?;
        println!(
            "  drawn  : {} panels committed on top",
            t.descriptor().panels.len()
        );
    }

    if file.is_none() {
        println!(
            "\nThe test pattern, upright: red square at the top left, green at the top right,"
        );
        println!("blue at the bottom left, yellow at the bottom right. A white 1-pixel line at the edge.");
        println!("A grey line each 40 pixels. The text \"x,y\" gives the coordinates of a bright crossing.");
    }
    Ok(())
}

/// Read one report, tolerating the occasional transient IOKit hiccup.
/// Returns None on timeout or a non-fatal error.
fn read_tolerant(t: &mut Device<HidTransport>, consecutive_errors: &mut u32) -> Option<Vec<u8>> {
    match t.read(Duration::from_millis(100)) {
        Ok(report) => {
            *consecutive_errors = 0;
            report
        }
        Err(e) => {
            *consecutive_errors += 1;
            if *consecutive_errors == 1 {
                eprintln!("  (transient read error, continuing: {e})");
            }
            None
        }
    }
}

/// Read and decode events until the deadline.
fn pump(t: &mut Device<HidTransport>, duration: Duration, verbose: bool) -> AnyResult {
    let deadline = Instant::now() + duration;
    let mut errors = 0u32;
    while Instant::now() < deadline {
        if errors >= 10 {
            return Err("too many consecutive read errors".into());
        }
        let Some(data) = read_tolerant(t, &mut errors) else {
            continue;
        };
        match events::decode(&data) {
            Event::Ack => {
                if verbose {
                    println!("  ack");
                }
            }
            Event::Unknown(raw) => {
                if verbose {
                    println!("  ? {}", hex(&raw[..raw.len().min(24)]));
                }
            }
            ev => println!("  {}", describe(&ev)),
        }
    }
    Ok(())
}

fn describe(ev: &Event) -> String {
    match ev {
        Event::Key { code, pressed } => format!(
            "{:<10} {:<5} (code 0x{code:02X})",
            events::key_label(*code),
            if *pressed { "DOWN" } else { "up" }
        ),
        Event::KnobRotate { knob, dir } => {
            format!(
                "knob {knob}     {}",
                if *dir == events::Direction::Left {
                    "<<<"
                } else {
                    ">>>"
                }
            )
        }
        Event::KnobPress { knob, pressed } => {
            format!(
                "knob {knob}     {}",
                if *pressed { "pressed" } else { "released" }
            )
        }
        Event::Swipe { dir } => {
            format!(
                "swipe      {}",
                if *dir == events::Direction::Left {
                    "<<<"
                } else {
                    ">>>"
                }
            )
        }
        Event::TouchStart { x } => format!("touch down x={x}"),
        Event::Tap { panel, x } => format!("tap        bar panel {panel} (x={x})"),
        Event::TouchEnd => "touch up   (gesture not identified)".into(),
        Event::Touch { x, y } => format!("touch      ({x}, {y})"),
        Event::Ack => "ack".into(),
        Event::Unknown(_) => "unknown".into(),
    }
}

fn flag_u64(args: &[String], name: &str) -> Option<u64> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1)?.parse().ok()
}

fn flag_str<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).map(String::as_str)
}

fn parse_hex_color(s: &str) -> Result<[u8; 3], Box<dyn std::error::Error>> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return Err("colour must be 6 hex digits, e.g. ff8800".into());
    }
    Ok([
        u8::from_str_radix(&s[0..2], 16)?,
        u8::from_str_radix(&s[2..4], 16)?,
        u8::from_str_radix(&s[4..6], 16)?,
    ])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X} ")).collect()
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>() + "~"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn flag_str_reads_the_value_after_the_flag() {
        let a = args(&["--save", "a.txt", "--note", "key 6 press", "--until-enter"]);
        assert_eq!(flag_str(&a, "--save"), Some("a.txt"));
        assert_eq!(flag_str(&a, "--note"), Some("key 6 press"));
        assert_eq!(flag_str(&a, "--seconds"), None);
    }

    #[test]
    fn flag_str_has_no_value_at_the_end() {
        assert_eq!(flag_str(&args(&["--save"]), "--save"), None);
    }

    #[test]
    fn saved_line_has_all_bytes() {
        let data = vec![0x41u8; 100];
        let line = recording::format_report_line(5, &data);
        assert_eq!(line.len(), 2 + 200);
        assert!(line.starts_with("5 4141"));
    }

    #[test]
    fn saved_header_has_no_serial_field() {
        let h = recording::header("a gesture", "VSD StreamDock N4 Pro", "V4.VSD N4 Pro.02.010");
        assert!(!h.to_lowercase().contains("serial"));
        assert_eq!(h.lines().count(), 5);
    }
}
