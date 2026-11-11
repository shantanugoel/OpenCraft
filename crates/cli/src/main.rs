//! opencraft-probe — hardware bring-up tool.
//!
//! Confirms that we can talk to a StreamDock-class device without the vendor
//! software: enumerate it, draw on every panel, and decode every input.

use opencraft_core::{events, image, list_devices, protocol, Event, Found, Transport};
use std::process::ExitCode;
use std::time::{Duration, Instant};

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
        "clear" => cmd_clear(),
        "raw" => cmd_raw(rest),
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
    clear                Blank every panel
    raw                  Hexdump raw input reports (for unknown hardware)

OPTIONS:
    --seconds <n>        How long to listen (default: 30 for test, 15 for listen)
    --brightness <0-100> Panel brightness for 'test' (default: 80)
    --mode <0-9>         Send a MODE command first (some rebrands need it)
    --keep               Leave the test pattern on screen when exiting
    --verbose            Also print acks and unrecognised packets

NOTE: quit VSD Craft before running — two apps driving one device will fight."
    );
}

fn open_device() -> Result<Transport, Box<dyn std::error::Error>> {
    let found: Vec<Found> = list_devices()?;

    let candidate = found
        .iter()
        .find(|f| f.known.is_some() && f.is_vendor_interface())
        .or_else(|| found.iter().find(|f| f.known.is_some()));

    match candidate {
        Some(f) => {
            println!("  device : {} (0x{:04X}:0x{:04X})", f.label(), f.vid, f.pid);
            if let Some(serial) = &f.serial {
                println!("  serial : {serial}");
            }
            println!(
                "  iface  : interface {} / usage page 0x{:04X}\n",
                f.interface, f.usage_page
            );
            Ok(Transport::open(f)?)
        }
        None => {
            eprintln!("No known StreamDock device found.\n");
            eprintln!("Checklist:");
            eprintln!("  - is the device plugged in?");
            eprintln!("  - is VSD Craft still running and holding the interface?");
            eprintln!("  - run 'opencraft-probe list' to see what is on the bus");
            Err("device not found".into())
        }
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
    let t = open_device()?;
    let d = t.descriptor;
    println!("  panels : {}", d.panels.len());
    println!("  packet : {} bytes", d.packet_size);
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
    for (i, panel) in t.descriptor.panels.iter().enumerate() {
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
        t.descriptor.panels.len()
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
    let mut t = open_device()?;
    println!("Hexdumping raw input reports for {seconds}s.\n");
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut errors = 0u32;
    while Instant::now() < deadline {
        if errors >= 10 {
            return Err("too many consecutive read errors".into());
        }
        if let Some(data) = read_tolerant(&mut t, &mut errors) {
            println!("[{}] {}", data.len(), hex(&data[..data.len().min(32)]));
        }
    }
    Ok(())
}

fn cmd_draw(rest: &[String]) -> AnyResult {
    if rest.len() < 2 {
        return Err("usage: opencraft-probe draw <slot> <rrggbb>".into());
    }
    let slot: u8 = rest[0].parse()?;
    let rgb = parse_hex_color(&rest[1])?;

    let t = open_device()?;
    let panel = t
        .descriptor
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

fn cmd_clear() -> AnyResult {
    let t = open_device()?;
    t.clear_all()?;
    println!("  cleared all panels");
    Ok(())
}

/// Read one report, tolerating the occasional transient IOKit hiccup.
/// Returns None on timeout or a non-fatal error.
fn read_tolerant(t: &mut Transport, consecutive_errors: &mut u32) -> Option<Vec<u8>> {
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
fn pump(t: &mut Transport, duration: Duration, verbose: bool) -> AnyResult {
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
        Event::Touch { x, y } => format!("touch      ({x}, {y})"),
        Event::Ack => "ack".into(),
        Event::Unknown(_) => "unknown".into(),
    }
}

fn flag_u64(args: &[String], name: &str) -> Option<u64> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1)?.parse().ok()
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
