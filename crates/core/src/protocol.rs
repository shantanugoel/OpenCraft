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

/// Clear the framebuffer. Sent once at startup.
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
pub fn clear_slot(slot: u8) -> Vec<u8> {
    let mut v = cmd(b"CLE");
    v.extend_from_slice(&[0x00, 0x00, if slot == 0xFF { 0xFF } else { slot }]);
    v
}

/// Put the panel to sleep.
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

/// Haptic feedback (N4 Pro family only).
pub fn set_vibration(enabled: bool) -> Vec<u8> {
    let vib = if enabled { 0x11 } else { 0xFF };
    let mut v = cmd(b"QUCMD");
    v.extend_from_slice(&[0x1f, 0x11, 0x00, vib, 0x00, 0x11]);
    v
}

/// Header announcing an image transfer to `slot`, followed by the payload in
/// 1024-byte chunks.
pub fn image_header(payload_len: usize, slot: u8) -> Vec<u8> {
    let mut v = cmd(b"BAT");
    v.extend_from_slice(&[
        0x00,
        0x00,
        (payload_len >> 8) as u8,
        payload_len as u8,
        slot,
    ]);
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
        // The table calls this "Start, clear the frame buffer".
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
        // `CLE`, `0x00 0x00`, slot. The slot `0xFF` clears all slots.
        assert_eq!(clear_slot(6), command(b"CLE\x00\x00\x06"));
        assert_eq!(clear_slot(15), command(b"CLE\x00\x00\x0F"));
        assert_eq!(clear_slot(1), command(b"CLE\x00\x00\x01"));
        assert_eq!(clear_slot(0xFF), command(b"CLE\x00\x00\xFF"));
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
}
