//! opencraft-probe — hardware bring-up tool.
//!
//! Confirms that we can talk to a StreamDock-class device without the vendor
//! software: enumerate it, draw on each surface, and decode each input event.
//! The surfaces and the controls come from the descriptor.

use opencraft_core::link::KEEP_ALIVE_INTERVAL;
use opencraft_core::{
    events, image, list_devices, protocol, recording, select, ControlKind, Decoded, Descriptor,
    Device, Error, Found, Gesture, HidConnector, HidTransport, InputCodes, InputEvent, Link,
    LinkEvent, Transport,
};
use std::io::Write;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod measure;
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
        "send" => measure::cmd_send(rest),
        "bars" => measure::cmd_bars(rest),
        "wide" => measure::cmd_wide(rest),
        "repaint" => measure::cmd_repaint(rest),
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
    identify             Open the device, show its identity, firmware, surfaces
                         and controls
    test                 Draw its slot number on each surface, then show each
                         input event (control and gesture)
    listen               Show input events only
    draw <slot> <rrggbb> Paint the surface of one slot a solid colour
    clear [slot]         Send the clear command for one slot, or for all slots
    background           Draw a test pattern on the background
    raw                  Hexdump raw input reports; can save them to a file
    record               Record each gesture to a file, one prompt at a time
                         (--out <dir>, --from <step>)
    send <name> [args]   Send one protocol command and nothing else. The names
                         are: brightness <0-100>..., knob-colors <rrggbb>...,
                         knob-brightness <0-100>..., sleep, keep-alive, start,
                         commit, haptic on|off. Prints the first 16 bytes.
    bars                 Draw the 4 bar panels with a pattern that shows
                         their edges
    wide <slot>          Send an image that is wider than the surface
                         (--size <w>x<h> is necessary)
    repaint              Draw all surfaces many times and print the time of
                         each repaint (for the repaint time and the image
                         format)

OPTIONS:
    --seconds <n>        How long to listen (default: 30 for test, 15 for listen)
    --brightness <0-100> Display brightness for 'test' (default: 80)
    --mode <0-9>         Send a MODE command first (some rebrands need it)
    --keep               Leave the test pattern on screen when exiting
    --verbose            Also print the acknowledgements
    --keep-alive <n>     For 'test', 'listen' and 'raw'. Not necessary: the
                         device layer sends the keep-alive each 10 seconds.
                         For a measurement, <n> changes the interval in
                         seconds, and 0 stops the keep-alive.

OPTIONS FOR 'raw':
    --save <file>        Write each full input report to <file> (text format)
    --note <text>        Say in the file which gesture it holds
    --until-enter        Stop when you press Enter ('--seconds' is the time
                         limit; default: 10)
    --draw               Draw the slot numbers after each connection, as
                         'test' does

OPTIONS FOR 'background':
    --file <path>        Draw this PNG or JPEG file, not the test pattern
    --quality <1-100>    JPEG quality (default: 60, the value of the vendor SDK)
    --with-surfaces      Then draw the numbered key faces and bar panels on top
    --clear              Remove the background image. Draw nothing.
    --init               Send the start commands before the background
    --hold <n>           Keep the connection for <n> seconds after the draw

OPTIONS FOR 'send', 'bars', 'wide' AND 'repaint':
    --hold <n>           Keep the connection for <n> seconds after the command,
                         with the keep-alive. Without it, the device stops the
                         connection approximately 35 seconds after a draw, and
                         its displays go blank.
    --keep-alive <n>     The keep-alive interval in seconds for --hold
                         (default: 10). 0 sends no keep-alive.
    --no-init            'bars', 'wide' and 'repaint' send the start commands
                         (DIS, then the brightness command) before they draw.
                         This option skips them.
    --brightness <0-100> The brightness of the start commands (default: 80)

OPTIONS FOR 'send':
    --each <n>           Seconds between two values of 'brightness' or
                         'knob-brightness' (default: 5)
    --wait-key           Wait for a key press on the device before each value
    --draw               First send the start commands and draw the numbered
                         key faces and bar panels, so that you can see the
                         effect of the command

OPTIONS FOR 'wide':
    --size <w>x<h>       The size of the image, for example 352x112 or 800x112.
                         It does not come from the descriptor.
    --format png|jpeg    The image format (default: the format of the surface)
    --quality <1-100>    JPEG quality (default: 60)
    --with-bars          Draw the edge pattern on the other bar panels also

OPTIONS FOR 'repaint':
    --count <n>          The number of repaints (default: 20)
    --format png|jpeg    The image format (default: the format of each surface
                         in the descriptor)
    --quality <1-100>    JPEG quality (default: 60)
    --one <slot>         Draw the surface of this slot only
    --pause <ms>         Wait between two repaints (default: 0)
    --wait-key           Wait for a key press on the device before the first
                         repaint

'test', 'listen' and 'raw' send the keep-alive, so the device stays
connected. They continue after a disconnection and wait for the device to
connect again. 'test' and 'raw --draw' draw the surfaces again. '--seconds' is
the total time.

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
    println!("  packet : {} bytes", d.packet_size);
    match t.firmware_version() {
        Ok(v) if !v.is_empty() => println!("  firmware: {v}"),
        _ => println!("  firmware: (unavailable)"),
    }
    print!("{}", describe_descriptor(d));
    Ok(())
}

/// The surfaces and the controls of the descriptor, as text.
fn describe_descriptor(d: &Descriptor) -> String {
    let mut out = format!("\n  surfaces: {}\n", d.surfaces.len());
    for s in d.surfaces {
        let slot = match s.slot {
            Some(slot) => format!("slot {slot:>2}"),
            None => "no slot".to_string(),
        };
        out += &format!(
            "    {:<14} {slot}  {:>3} x {:<3}  {:<4}  turned {}°\n",
            s.id.to_string(),
            s.width,
            s.height,
            s.format.name(),
            s.rotation.degrees()
        );
    }
    out += &format!("\n  controls: {}\n", d.controls.len());
    for c in d.controls {
        let codes: Vec<String> = gesture_codes(&c.codes)
            .iter()
            .map(|(gesture, code)| format!("{gesture} 0x{code:02X}"))
            .collect();
        let surface = match c.surface {
            Some(id) => format!("  surface: {id}"),
            None => String::new(),
        };
        out += &format!(
            "    {:<12} row {} column {}  {}{surface}\n",
            c.id().to_string(),
            c.position.row,
            c.position.column,
            codes.join(", ")
        );
    }
    out
}

/// The input codes of a control, each with the name of its gesture.
fn gesture_codes(codes: &InputCodes) -> Vec<(&'static str, u8)> {
    match *codes {
        InputCodes::Key { code } => vec![("press/release", code)],
        InputCodes::Knob {
            turn_left,
            turn_right,
            push,
        } => vec![
            ("turn left", turn_left),
            ("turn right", turn_right),
            ("push", push),
        ],
        InputCodes::BarPanel { tap } => vec![("tap", tap)],
        InputCodes::TouchBar {
            touch_start,
            swipe_right,
            swipe_left,
            touch_end,
        } => vec![
            ("touch start", touch_start),
            ("swipe right", swipe_right),
            ("swipe left", swipe_left),
            ("touch end", touch_end),
        ],
    }
}

fn cmd_test(rest: &[String]) -> AnyResult {
    let seconds = flag_u64(rest, "--seconds").unwrap_or(30);
    let brightness = flag_u64(rest, "--brightness").unwrap_or(80) as u8;
    let mode = flag_u64(rest, "--mode").map(|m| m as u8);
    let keep = rest.iter().any(|a| a == "--keep");
    let verbose = rest.iter().any(|a| a == "--verbose");
    let keep_alive = measure::keep_alive_flag(rest)?;

    let device = open_device()?;
    let d = device.descriptor();
    let mut link = Link::from_device(HidConnector, device);
    if let Some(interval) = keep_alive {
        link.set_keep_alive_interval(Some(interval));
    }
    if let Some(mode) = mode {
        println!("  mode   : MODE {mode} before each draw");
    }
    println!("  alive  : {}", measure::describe_keep_alive(keep_alive));
    println!("  light  : brightness {brightness}%\n");
    println!("Each key face and each bar panel shows its slot number after the draw.");
    println!("Press keys / turn knobs / swipe the touch bar for {seconds}s.");
    println!("You can disconnect and connect the cable. The probe draws again.\n");

    run_link(
        &mut link,
        Duration::from_secs(seconds),
        None,
        |d| draw_test_pattern(d, mode, brightness),
        |data| {
            print_event(d, data, verbose);
            Ok(())
        },
    )?;

    if keep {
        println!("\nLeaving the test pattern on screen.");
    } else if link.is_connected() {
        println!("\nRestoring: clearing all slots.");
        link.with_device(|d| {
            d.set_brightness(60)?;
            d.clear_all()
        })?;
    } else {
        println!("\nThe device is not connected. Nothing to restore.");
    }
    Ok(())
}

fn cmd_listen(rest: &[String]) -> AnyResult {
    let seconds = flag_u64(rest, "--seconds").unwrap_or(15);
    let verbose = rest.iter().any(|a| a == "--verbose");
    let keep_alive = measure::keep_alive_flag(rest)?;
    let device = open_device()?;
    let d = device.descriptor();
    let mut link = Link::from_device(HidConnector, device);
    if let Some(interval) = keep_alive {
        link.set_keep_alive_interval(Some(interval));
    }
    println!("  alive  : {}", measure::describe_keep_alive(keep_alive));
    println!("Listening for {seconds}s. Ctrl-C to stop.\n");
    run_link(
        &mut link,
        Duration::from_secs(seconds),
        None,
        |_| Ok(0),
        |data| {
            print_event(d, data, verbose);
            Ok(())
        },
    )
}

fn cmd_raw(rest: &[String]) -> AnyResult {
    let seconds = flag_u64(rest, "--seconds").unwrap_or(10);
    let save = flag_str(rest, "--save");
    let note = flag_str(rest, "--note").unwrap_or("");
    let until_enter = rest.iter().any(|a| a == "--until-enter");
    let draw = rest.iter().any(|a| a == "--draw");
    let keep_alive = measure::keep_alive_flag(rest)?;
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

    // With '--draw', the link draws the slot numbers after each connection.
    // The link sends the keep-alive. '--keep-alive' changes its interval for
    // a measurement, and 0 stops it.
    let mut link = Link::from_device(HidConnector, t);
    if let Some(interval) = keep_alive {
        link.set_keep_alive_interval(Some(interval));
    }
    println!("  alive  : {}", measure::describe_keep_alive(keep_alive));
    let stop = Arc::new(AtomicBool::new(false));
    if until_enter {
        watch_enter(Arc::clone(&stop));
        println!("Hexdumping raw input reports. Press Enter to stop ({seconds}s at most).\n");
    } else {
        println!("Hexdumping raw input reports for {seconds}s.\n");
    }

    let start = Instant::now();
    let mut saved = 0u64;
    run_link(
        &mut link,
        Duration::from_secs(seconds),
        Some(&stop),
        |d| {
            if draw {
                draw_test_pattern(d, None, 80)
            } else {
                Ok(0)
            }
        },
        |data| {
            let ms = start.elapsed().as_millis() as u64;
            println!("[{}] {}", data.len(), hex(&data[..data.len().min(32)]));
            if let Some(f) = file.as_mut() {
                let line = recording::format_report_line(ms, data) + "\n";
                f.write_all(line.as_bytes())?;
                saved += 1;
            }
            Ok(())
        },
    )?;
    if let Some(path) = save {
        println!("\nSaved {saved} report(s) to {path}");
    }
    Ok(())
}

/// Send the start commands, then draw its slot number on each surface that has
/// a slot. The probe does this after each connection, as the engine must
/// (architecture rule 10).
fn draw_test_pattern(
    t: &mut Device<HidTransport>,
    mode: Option<u8>,
    brightness: u8,
) -> opencraft_core::Result<usize> {
    if let Some(mode) = mode {
        t.send(protocol::set_mode(mode))?;
        std::thread::sleep(Duration::from_millis(300));
    }

    start_display(t, brightness)?;
    draw_slot_surfaces(t)
}

/// Send the start commands: `DIS`, then the brightness command. After a new
/// connection, the displays show nothing until these commands go out.
fn start_display<T: Transport>(t: &mut Device<T>, brightness: u8) -> opencraft_core::Result<()> {
    t.init()?;
    std::thread::sleep(Duration::from_millis(150));
    t.set_brightness(brightness)
}

/// Draw its slot number on each surface that has a slot, give each knob light
/// a test colour, and commit. Return the number of surfaces.
///
/// Surface `i` of the table and knob `i` get the test colour `i`.
fn draw_slot_surfaces(t: &mut Device<HidTransport>) -> opencraft_core::Result<usize> {
    let d = t.descriptor();
    let mut drawn = 0;
    for (i, surface) in d.surfaces.iter().enumerate() {
        let Some(slot) = surface.slot else { continue };
        let data = image::render_test_surface(surface, slot, image::test_color(i))?;
        t.send_image(slot, &data)?;
        drawn += 1;
    }

    let led_colors: Vec<[u8; 3]> = (0..d.controls_of(ControlKind::Knob).count())
        .map(image::test_color)
        .collect();
    if !led_colors.is_empty() {
        t.send(protocol::set_led_colors(&led_colors))?;
    }

    t.commit()?;
    Ok(drawn)
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
    let surface = t
        .descriptor()
        .surface_at_slot(slot)
        .ok_or_else(|| format!("slot {slot} does not exist on this device"))?;

    t.init()?;
    let data = image::render_solid(surface, rgb)?;
    t.send_image(slot, &data)?;
    t.commit()?;
    println!(
        "  painted {} (slot {slot}) with #{:02X}{:02X}{:02X}",
        surface.id, rgb[0], rgb[1], rgb[2]
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
            println!("  cleared all slots");
        }
    }
    Ok(())
}

/// Draw the background: the test pattern or an image file. This sends the
/// temporary background command ("BGPIC"), never the boot logo command.
fn cmd_background(rest: &[String]) -> AnyResult {
    let quality = flag_u64(rest, "--quality")
        .unwrap_or(image::DEFAULT_JPEG_QUALITY as u64)
        .clamp(1, 100) as u8;
    let with_surfaces = rest.iter().any(|a| a == "--with-surfaces");
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
        .background()
        .ok_or("this device has no background in its descriptor")?;

    if clear {
        t.clear_background()?;
        t.commit()?;
        println!("  background: cleared (BGCLE 0x03, then STP)");
        return Ok(());
    }

    let data = match &file {
        Some(file) => image::render_background_file(file, bg, quality)?,
        None => image::render_background_test(bg, quality)?,
    };
    println!(
        "  background: {}x{} {}, quality {quality}, {} bytes ({} the 2-byte length limit of 65535)",
        bg.width,
        bg.height,
        bg.format.name(),
        data.len(),
        if data.len() > 0xFFFF {
            "above"
        } else {
            "below"
        }
    );

    if rest.iter().any(|a| a == "--init") {
        // The same start as the test command.
        start_display(&mut t, 80)?;
        println!("  init   : DIS, then brightness 80%");
    }

    t.send_background(&data)?;
    t.commit()?;
    println!("  drawn  : background committed");

    if with_surfaces {
        // A short pause makes the two steps visible one after the other.
        std::thread::sleep(Duration::from_secs(1));
        let drawn = draw_slot_surfaces(&mut t)?;
        println!("  drawn  : {drawn} surfaces committed on top");
    }

    if file.is_none() {
        println!(
            "\nThe test pattern, upright: red square at the top left, green at the top right,"
        );
        println!("blue at the bottom left, yellow at the bottom right. A white 1-pixel line at the edge.");
        println!("A grey line each 40 pixels. The text \"x,y\" gives the coordinates of a bright crossing.");
    }

    // The device stops the connection 35 seconds after the last command, and
    // its displays go blank. Keep the connection, so that a person has the
    // time to look at the result.
    if let Some(seconds) = flag_u64(rest, "--hold") {
        let mut keeper = measure::Keeper::new(Some(KEEP_ALIVE_INTERVAL));
        measure::hold(&mut t, seconds, &mut keeper)?;
    }
    Ok(())
}

/// Operate the link until `duration` expires or `stop` is set.
///
/// After each connection, `draw` draws all surfaces; it returns the number of
/// surfaces that it drew. For each input report, `on_input` runs. The link sends
/// the keep-alive. The function prints a line when the device disconnects and
/// when it connects again. An error that does not stop the link is printed one
/// time, until it changes.
fn run_link(
    link: &mut Link<HidConnector>,
    duration: Duration,
    stop: Option<&AtomicBool>,
    mut draw: impl FnMut(&mut Device<HidTransport>) -> opencraft_core::Result<usize>,
    mut on_input: impl FnMut(&[u8]) -> AnyResult,
) -> AnyResult {
    let start = Instant::now();
    let deadline = start + duration;
    let mut first = true;
    let mut lost_at: Option<Instant> = None;
    let mut last_error: Option<String> = None;
    let stamp = || format!("[{:6.1}s]", start.elapsed().as_secs_f64());

    while !stop.is_some_and(|s| s.load(Ordering::Relaxed)) {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let wait = (deadline - now).min(Duration::from_millis(100));
        match link.next_event(wait) {
            Ok(LinkEvent::Connected) => {
                last_error = None;
                if let Some(lost) = lost_at.take() {
                    println!(
                        "  {} connected again, {:.1}s after the disconnection",
                        stamp(),
                        lost.elapsed().as_secs_f64()
                    );
                }
                let drawn_at = Instant::now();
                match link.with_device(|d| draw(d)) {
                    Ok(0) => {}
                    Ok(n) => println!(
                        "  {} drawn  : {n} surfaces committed in {} ms",
                        stamp(),
                        drawn_at.elapsed().as_millis()
                    ),
                    Err(e) => eprintln!("  {} (draw failed: {e})", stamp()),
                }
                if first {
                    println!();
                }
                first = false;
            }
            Ok(LinkEvent::Disconnected) => {
                lost_at = Some(Instant::now());
                last_error = None;
                println!(
                    "  {} disconnected. Waiting for the device to connect again.",
                    stamp()
                );
            }
            Ok(LinkEvent::Input(data)) => on_input(&data)?,
            Ok(LinkEvent::Timeout) => {}
            Err(e) => {
                let message = e.to_string();
                if last_error.as_deref() != Some(message.as_str()) {
                    eprintln!("  {} (error, continuing: {message})", stamp());
                    last_error = Some(message);
                }
            }
        }
    }
    Ok(())
}

/// Print one decoded input report.
fn print_event(d: &Descriptor, data: &[u8], verbose: bool) {
    match events::decode(d, data) {
        Decoded::Ack => {
            if verbose {
                println!("  ack");
            }
        }
        // A report that the decoder does not know. Show it always: it can be
        // a code that nobody measured yet.
        Decoded::Unknown(raw) => println!("  ? {}", hex(&raw[..raw.len().min(24)])),
        Decoded::Input(ev) => println!("  {}", describe(d, &ev)),
    }
}

/// One input event as text: the control, the gesture, and the coordinates.
/// For a control with a surface, the text also gives the slot, because the
/// `test` command draws the slot number on each surface.
fn describe(d: &Descriptor, ev: &InputEvent) -> String {
    let detail = match ev.gesture {
        Gesture::Tap { x } | Gesture::TouchStart { x } => format!("x={x}"),
        Gesture::TouchPoint { x, y } => format!("({x}, {y})"),
        _ => String::new(),
    };
    let slot = d
        .control(ev.control)
        .and_then(|c| c.surface)
        .and_then(|id| d.surface(id))
        .and_then(|s| s.slot)
        .map(|slot| format!("slot {slot}"))
        .unwrap_or_default();
    format!(
        "{:<12} {:<12} {:<11} {slot}",
        ev.control.to_string(),
        ev.gesture.name(),
        detail
    )
    .trim_end()
    .to_string()
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
    // A character that is not ASCII is not a hex digit, and it can make the
    // byte slices below end inside a character.
    if s.len() != 6 || !s.is_ascii() {
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
        assert_eq!(flag_str(&args(&["--save"]), "--save"), None);
    }
}
