# StreamDock USB protocol, version 3

Last update: 2026-11-11

This entry gives the protocol that the N4 Pro family uses. The device facts are
in [n4-pro-hardware.md](n4-pro-hardware.md). The code is in `crates/core/src/`.
`protocol.rs` makes the commands. `connection.rs` sends the commands and the
images. `events.rs` decodes the input reports. `transport/hid.rs` contains the
`hidapi` I/O.

## Reports

| Fact | Confidence | Source |
|---|---|---|
| An output report is 1 report ID byte (`0x00`) and 1024 payload bytes | Measured | Probe |
| An input report is 512 bytes | Measured | `hidutil` |
| A report size of 512 in and 1024 out identifies protocol version 3 | Source | `mirajazz` |
| A command is `0x00`, `"CRT"`, `0x00 0x00`, an ASCII operation code, then the arguments. Zero bytes fill the remaining space. | Measured | Probe |
| Feature report `0x01` gives the firmware version as text | Measured | Probe |

## Commands

"Measured" means that the probe sent the command to our device and the record
of the bring-up confirms the result. "Sent" means that the probe sends the
command in its test, but the record does not confirm the result. "Not sent"
means that the command is in the code but is not tested.

| Operation | Code | Arguments | Confidence |
|---|---|---|---|
| Image header | `BAT` | `0x00 0x00`, length (2 bytes, big-endian), slot | Measured |
| Commit the images | `STP` | None | Measured |
| Start, clear the frame buffer | `DIS` | None | Sent |
| Brightness | `LIG` | `0x00 0x00`, percent 0 to 100 | Sent |
| Clear one slot, or all slots with `0xFF` | `CLE` | `0x00 0x00`, slot | Sent |
| Knob light colours | `SETLB` | Red, green, blue for each knob | Sent |
| Knob light brightness | `LBLIG` | Percent 0 to 100 | Source: not sent |
| Sleep | `HAN` | None | Source: not sent |
| Keep-alive | `CONNECT` | None | Source: not sent |
| Set the firmware mode | `MOD` | `0x00 0x00`, ASCII digit | Source: not sent |
| Haptic feedback | `QUCMD` | `0x1F 0x11 0x00`, `0x11` on or `0xFF` off, `0x00 0x11` | Source: not sent |

## Image transfer

1. Send the `BAT` header with the length of the image data and the slot.
2. Send the image data in parts of 1024 bytes. Each part has the report ID byte
   in front.
3. Send `STP` after all images. The device then shows them.

| Fact | Confidence | Source |
|---|---|---|
| The device accepts PNG for key faces and bar panels | Measured | Probe |
| The image must be turned 180° before it is sent | Measured | Probe |
| `mirajazz` and `opendeck-akp05` send JPEG to the same slots | Source | Their source code |
| The background image goes through a different command, as a JPEG | Source | SDK |

**Caution.** The code sends only the low 16 bits of the image length. An image
larger than 65 535 bytes will have an incorrect header. The two zero bytes
before the length are possibly the high bytes of a 4-byte length. (Inference)

## Input reports

All rows are measured on 2026-11-11, from 999 recorded reports. The input
codes of the N4 Pro are in
[n4-pro-hardware.md](n4-pro-hardware.md#controls-and-input-codes).

| Fact | Confidence | Source |
|---|---|---|
| Each input report is 512 bytes. All bytes after byte 13 were zero. | Measured | Recordings |
| A control report has `"ACK"` at bytes 0 to 2 and `"OK"` at bytes 5 to 6 | Measured | Recordings |
| Byte 9 is the input code. Byte 10 is the state: `0x01` or `0x00`. | Measured | Recordings |
| For a touch bar code, bytes 11 and 12 are the x coordinate of the start of the touch, big-endian. For a swipe code they are zero. | Measured | Recordings |
| A touch report has `"ACK"` at bytes 0 to 2, `"ARX"` at bytes 4 to 6, x at bytes 10 and 11, and y at bytes 12 and 13, big-endian | Measured | Recordings |
| The device sends a touch report each 35 ms while a finger is on the touch bar | Measured | Recordings |
| Input code `0xFF` is an acknowledgement, not user input | Source | SDK |

## Keep-alive and stop

| Fact | Confidence | Source |
|---|---|---|
| The SDK and `opendeck-akp05` send `CONNECT` each 10 seconds | Source | Their source code |
| The keep-alive is necessary. If the host sends no command, the device stops the USB connection approximately 35 seconds after the last command. Input from the user does not change this time. | Measured | Probe, 2026-11-11: 35.1 s and 35.4 s after the draw |
| With `CONNECT` each 10 seconds, the connection stays. The test was 100 seconds. | Measured | Probe, 2026-11-11 |
| After the device stops the connection, it is on the USB bus again in less than 2 seconds, and its panels are blank | Measured | Probe and owner, 2026-11-11 |
| The bring-up in Phase 0 passed with no `CONNECT`, because its test was 30 seconds | Inference | The rows above |
| `opendeck-akp05` sends a clear command and then `HAN` when it stops | Source | Its source code |
| After the probe draws the panels and stops with no clear command, the panels go blank after a short time. The cause is possibly the same 35 seconds. | Measured (the blank panels), Inference (the cause) | Owner, 2026-11-11 |
| The device sent no input report in answer to the draw commands or to `CONNECT` during 100 seconds | Measured | Probe, 2026-11-11 |
| The SDK README refers to `ACK` and `OK` in input reports for newer protocol versions | Source | SDK README |

## Not known

Measure these in Phase 1.

- The longest interval between two keep-alive commands that keeps the
  connection. Only 10 seconds is tested.
- Whether a command that is not `CONNECT` also starts the 35 seconds again.
- What the device shows after the host process is killed.
- Whether each command with the label "Sent" gives the correct result.
- The time to paint all 14 panels.
- Whether JPEG is quicker than PNG for a full repaint.
- The correct width of the length field in the `BAT` header.
- The command and the limits for the 800 × 480 background image.

## Sources

- Mirabox StreamDock Device SDK (MIT): <https://github.com/MiraboxSpace/StreamDock-Device-SDK>
- `mirajazz` (MPL-2.0): <https://github.com/4ndv/mirajazz>
- `opendeck-akp05`: <https://github.com/ambiso/opendeck-akp05>
