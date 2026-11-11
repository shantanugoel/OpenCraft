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
