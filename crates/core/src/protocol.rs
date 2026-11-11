//! StreamDock wire protocol (protocol v3, 1024-byte output reports).
//!
//! Every command is `[0x00] + "CRT" + [0x00, 0x00] + <opcode> + <args>`, padded
//! to 1025 bytes (1 report id + 1024 payload). Opcodes are ASCII and were taken
//! from Mirabox's MIT-licensed SDK and the mirajazz crate.

pub const REPORT_ID: u8 = 0x00;
pub const PACKET_SIZE: usize = 1024;

/// `[0x00] "CRT" 0x00 0x00` — the 6-byte prefix on every command.
const PREFIX: [u8; 6] = [0x00, b'C', b'R', b'T', 0x00, 0x00];

fn cmd(op: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(&PREFIX);
    v.extend_from_slice(op);
    v
}

/// Start command. The probe sends it one time before the first image.
/// It does not remove the images that the device shows (measured).
pub fn init() -> Vec<u8> {
    cmd(b"DIS")
}

/// Commit pending image writes ("stop"/flush).
pub fn commit() -> Vec<u8> {
    cmd(b"STP")
}

/// Screen brightness, 0-100.
pub fn set_brightness(percent: u8) -> Vec<u8> {
    let mut v = cmd(b"LIG");
    v.extend_from_slice(&[0x00, 0x00, percent.min(100)]);
    v
}

/// Knob LED ring brightness, 0-100.
pub fn set_led_brightness(percent: u8) -> Vec<u8> {
    let mut v = cmd(b"LBLIG");
    v.push(percent.min(100));
    v
}

/// Per-knob LED colours, one `[r, g, b]` per knob.
pub fn set_led_colors(colors: &[[u8; 3]]) -> Vec<u8> {
    let mut v = cmd(b"SETLB");
    for [r, g, b] in colors {
        v.extend_from_slice(&[*r, *g, *b]);
    }
    v
}

/// Clear one slot, or every slot when `slot == 0xFF`.
///
/// The slot comes after 3 zero bytes. The Mirabox SDK transport
/// (`Transport::clearKey`, `Transport::clearAllKeys`) writes it at that
/// position. With 2 zero bytes, the device does nothing.
pub fn clear_slot(slot: u8) -> Vec<u8> {
    let mut v = cmd(b"CLE");
    v.extend_from_slice(&[0x00, 0x00, 0x00, slot]);
    v
}

/// Put the device to sleep.
pub fn sleep() -> Vec<u8> {
    cmd(b"HAN")
}

/// Heartbeat; devices drop the link without it.
pub fn keep_alive() -> Vec<u8> {
    cmd(b"CONNECT")
}

/// Force a firmware mode. Some rebrands need this before accepting commands.
pub fn set_mode(mode: u8) -> Vec<u8> {
    let mut v = cmd(b"MOD");
    v.extend_from_slice(&[0x00, 0x00, b'0' + mode]);
    v
}

/// Haptic feedback. The SDK has this command for the N4 Pro. The descriptor
/// does not say which models have haptic feedback.
pub fn set_vibration(enabled: bool) -> Vec<u8> {
    let vib = if enabled { 0x11 } else { 0xFF };
    let mut v = cmd(b"QUCMD");
    v.extend_from_slice(&[0x1f, 0x11, 0x00, vib, 0x00, 0x11]);
    v
}

/// Header of an image transfer to `slot`. The image data follows in parts of
/// 1024 bytes.
///
/// The length is 4 bytes, big-endian. The Mirabox SDK transport
/// (`Transport::setKeyBitmap`) writes the 4 bytes in this order. For an image
/// smaller than 65 536 bytes, the first 2 bytes are zero.
pub fn image_header(payload_len: u32, slot: u8) -> Vec<u8> {
    let mut v = cmd(b"BAT");
    v.extend_from_slice(&payload_len.to_be_bytes());
    v.push(slot);
    v
}

/// Header of a background image transfer. The JPEG data follows in parts of
/// 1024 bytes, as for [`image_header`].
///
/// The device puts the image at `x`, `y` with the size `width` × `height`.
/// The SDK calls this a "frame background"
/// (`Transport::setBackgroundFrameStream`). It is the temporary background,
/// not the boot logo. All numbers are big-endian. The layout is:
/// length (4 bytes), x (2), y (2), width (2), height (2), `0x00`, layer.
///
/// `x` and `y` are in the turned display, as the image data is. An image that
/// is smaller than the display changes only its rectangle. The value of
/// `layer` made no difference on our device (measured). See
/// `docs/kb/streamdock-protocol.md`.
///
/// The boot logo has a different command. OpenCraft does not make that
/// command, because the device writes the boot logo to permanent memory. See
/// `docs/kb/streamdock-protocol.md`.
pub fn background_header(
    payload_len: u32,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    layer: u8,
) -> Vec<u8> {
    let mut v = cmd(b"BGPIC");
    v.extend_from_slice(&payload_len.to_be_bytes());
    v.extend_from_slice(&x.to_be_bytes());
    v.extend_from_slice(&y.to_be_bytes());
    v.extend_from_slice(&width.to_be_bytes());
    v.extend_from_slice(&height.to_be_bytes());
    v.extend_from_slice(&[0x00, layer]);
    v
}

/// Remove the background image from an area of the display. `area` is `0x01`
/// for the area behind the keys, `0x02` for the area behind the touch bar, and
/// `0x03` for the two areas (measured). The SDK calls the argument a layer, and
/// uses `0x03` as the default (`Transport::clearBackgroundFrameStream`).
pub fn clear_background(area: u8) -> Vec<u8> {
    let mut v = cmd(b"BGCLE");
    v.push(area);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    // The expected bytes come from the command table in
    // `docs/kb/streamdock-protocol.md`, not from the builders. A command is
    // `0x00`, "CRT", `0x00 0x00`, the ASCII operation code, then the arguments.
    // The builder does not add the zero bytes at the end. `Device::send` adds
    // them.

    /// `0x00 "CRT" 0x00 0x00` followed by `rest`.
    fn command(rest: &[u8]) -> Vec<u8> {
        let mut v = vec![0x00, b'C', b'R', b'T', 0x00, 0x00];
        v.extend_from_slice(rest);
        v
    }

    #[test]
    fn constants_match_the_report_format() {
        // An output report is 1 report ID byte (0x00) and 1024 payload bytes.
        assert_eq!(REPORT_ID, 0x00);
        assert_eq!(PACKET_SIZE, 1024);
    }

    #[test]
    fn every_command_starts_with_the_prefix() {
        let all = [
            init(),
            commit(),
            set_brightness(50),
            set_led_brightness(50),
            set_led_colors(&[[1, 2, 3]]),
            clear_slot(1),
            sleep(),
            keep_alive(),
            set_mode(0),
            set_vibration(true),
            image_header(1, 6),
        ];
        for cmd in all {
            assert_eq!(&cmd[..6], &[0x00, b'C', b'R', b'T', 0x00, 0x00]);
        }
    }

    #[test]
    fn image_header_bytes() {
        // `BAT`, `0x00 0x00`, length (2 bytes, big-endian), slot.
        // The length is small, so that this test stays correct when the length
        // field gets more bytes.
        assert_eq!(image_header(0x0123, 6), command(b"BAT\x00\x00\x01\x23\x06"));
        assert_eq!(image_header(1, 15), command(b"BAT\x00\x00\x00\x01\x0F"));
        assert_eq!(image_header(2500, 7), command(b"BAT\x00\x00\x09\xC4\x07"));
    }

    #[test]
    fn commit_bytes() {
        assert_eq!(commit(), command(b"STP"));
    }

    #[test]
    fn init_bytes() {
        // The table calls this "Start".
        assert_eq!(init(), command(b"DIS"));
    }

    #[test]
    fn brightness_bytes() {
        // `LIG`, `0x00 0x00`, percent 0 to 100.
        assert_eq!(set_brightness(0), command(b"LIG\x00\x00\x00"));
        assert_eq!(set_brightness(50), command(b"LIG\x00\x00\x32"));
        assert_eq!(set_brightness(100), command(b"LIG\x00\x00\x64"));
    }

    #[test]
    fn brightness_is_limited_to_100() {
        assert_eq!(set_brightness(101), set_brightness(100));
        assert_eq!(set_brightness(255), set_brightness(100));
    }

    #[test]
    fn clear_slot_bytes() {
        // `CLE`, `0x00 0x00 0x00`, slot. The slot `0xFF` clears all slots.
        assert_eq!(clear_slot(6), command(b"CLE\x00\x00\x00\x06"));
        assert_eq!(clear_slot(15), command(b"CLE\x00\x00\x00\x0F"));
        assert_eq!(clear_slot(1), command(b"CLE\x00\x00\x00\x01"));
        assert_eq!(clear_slot(0xFF), command(b"CLE\x00\x00\x00\xFF"));
    }

    #[test]
    fn knob_colour_bytes() {
        // `SETLB`, then red, green, blue for each knob.
        let colors = [[1, 2, 3], [4, 5, 6], [7, 8, 9], [0xFF, 0x80, 0x00]];
        assert_eq!(
            set_led_colors(&colors),
            command(b"SETLB\x01\x02\x03\x04\x05\x06\x07\x08\x09\xFF\x80\x00")
        );
        assert_eq!(
            set_led_colors(&[[0xAA, 0xBB, 0xCC]]),
            command(b"SETLB\xAA\xBB\xCC")
        );
        assert_eq!(set_led_colors(&[]), command(b"SETLB"));
    }

    #[test]
    fn knob_light_brightness_bytes() {
        // `LBLIG`, percent 0 to 100.
        assert_eq!(set_led_brightness(0), command(b"LBLIG\x00"));
        assert_eq!(set_led_brightness(75), command(b"LBLIG\x4B"));
        assert_eq!(set_led_brightness(100), command(b"LBLIG\x64"));
        assert_eq!(set_led_brightness(200), set_led_brightness(100));
    }

    #[test]
    fn sleep_bytes() {
        assert_eq!(sleep(), command(b"HAN"));
    }

    #[test]
    fn keep_alive_bytes() {
        assert_eq!(keep_alive(), command(b"CONNECT"));
    }

    #[test]
    fn mode_bytes() {
        // `MOD`, `0x00 0x00`, an ASCII digit.
        assert_eq!(set_mode(0), command(b"MOD\x00\x00\x30"));
        assert_eq!(set_mode(2), command(b"MOD\x00\x00\x32"));
        assert_eq!(set_mode(3), command(b"MOD\x00\x00\x33"));
    }

    #[test]
    fn haptic_feedback_bytes() {
        // `QUCMD`, `0x1F 0x11 0x00`, `0x11` on or `0xFF` off, `0x00 0x11`.
        assert_eq!(
            set_vibration(true),
            command(b"QUCMD\x1F\x11\x00\x11\x00\x11")
        );
        assert_eq!(
            set_vibration(false),
            command(b"QUCMD\x1F\x11\x00\xFF\x00\x11")
        );
    }
    #[test]
    fn image_header_has_a_4_byte_big_endian_length() {
        // 2500 = 0x09C4. The bytes are the same as in the 2-byte form.
        assert_eq!(
            image_header(2500, 7),
            b"\x00CRT\x00\x00BAT\x00\x00\x09\xC4\x07".to_vec()
        );
        // 70 000 = 0x0001_1170 is larger than 65 535.
        assert_eq!(
            image_header(70_000, 6),
            b"\x00CRT\x00\x00BAT\x00\x01\x11\x70\x06".to_vec()
        );
    }

    #[test]
    fn background_header_layout() {
        // 70 000 = 0x0001_1170, 800 = 0x0320, 480 = 0x01E0.
        assert_eq!(
            background_header(70_000, 0, 0, 800, 480, 0),
            b"\x00CRT\x00\x00BGPIC\x00\x01\x11\x70\x00\x00\x00\x00\x03\x20\x01\xE0\x00\x00"
                .to_vec()
        );
        let v = background_header(1, 0x0102, 0x0304, 0x0506, 0x0708, 0x09);
        assert_eq!(&v[15..], &[1, 2, 3, 4, 5, 6, 7, 8, 0, 9]);
    }

    #[test]
    fn clear_background_has_the_layer_after_the_operation_code() {
        assert_eq!(clear_background(3), b"\x00CRT\x00\x00BGCLE\x03".to_vec());
    }

    /// The boot logo is possibly in permanent memory (issue #15). No builder
    /// in the device layer may make its operation code.
    #[test]
    fn no_code_makes_the_boot_logo_command() {
        let logo = concat!("b\"", "LO", "G\"");
        for (name, source) in [
            ("protocol.rs", include_str!("protocol.rs")),
            ("connection.rs", include_str!("connection.rs")),
        ] {
            assert!(!source.contains(logo), "{name} contains the boot logo code");
        }
    }
}
