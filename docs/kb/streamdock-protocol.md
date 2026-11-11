# StreamDock USB protocol, version 3

Last update: 2026-11-11

This entry gives the protocol that the N4 Pro family uses. The device facts are
in [n4-pro-hardware.md](n4-pro-hardware.md). The code is in `crates/core/src/`.
`protocol.rs` makes the commands. `connection.rs` sends the commands and the
images. `events.rs` decodes the input reports. It gets the input codes of the
model from the descriptor in `device.rs`. `transport/hid.rs` contains the
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

"Measured" means that a program sent the command to our device, and a person
or a camera saw the result. "Not sent" means that no program sent the command
to our device. How the camera measurements were made is in
[n4-pro-hardware.md](n4-pro-hardware.md#how-the-camera-measurements-were-made).

| Operation | Code | Arguments | Confidence |
|---|---|---|---|
| Image header | `BAT` | Length (4 bytes, big-endian), slot | Measured for an image smaller than 65 536 bytes. The 4-byte length is Source for `BAT`, and Measured for `BGPIC`. |
| Commit the images | `STP` | None | Measured |
| Background image header | `BGPIC` | Length (4 bytes), x (2 bytes), y (2 bytes), width (2 bytes), height (2 bytes), `0x00`, layer. All big-endian. | Measured: owner and camera, 2026-11-11. See "Background image". |
| Remove the background image | `BGCLE` | Area: `0x01` the key area, `0x02` the touch bar area, `0x03` the two areas | Measured: camera, 2026-11-11 |
| Boot logo header. **Do not send.** | `LOG` | Length (4 bytes, big-endian), `0x01` | Source: not sent |
| Start | `DIS` | None | Measured: it does not remove the images. See "Background image". |
| Brightness | `LIG` | `0x00 0x00`, percent 0 to 100 | Measured: camera, 2026-11-11. See "Brightness". |
| Clear one slot, or all slots with `0xFF` | `CLE` | `0x00 0x00 0x00`, slot | Measured: owner, 2026-11-11 |
| Knob light colours | `SETLB` | Red, green, blue for each knob | Measured: camera, 2026-11-11. See [n4-pro-hardware.md](n4-pro-hardware.md#knob-lights). |
| Knob light brightness | `LBLIG` | Percent 0 to 100 | Measured: camera, 2026-11-11 |
| Reset the knob lights | `DELED` | None | Source: SDK binary, `Transport::resetLedColor`. Sent 1 time: the camera saw no change. OpenCraft has no builder for it. |
| Sleep | `HAN` | None | Source: not sent |
| Keep-alive | `CONNECT` | None | Measured. See "Keep-alive and stop". |
| Set the firmware mode | `MOD` | `0x00 0x00`, ASCII digit | Source: not sent |
| Haptic feedback | `QUCMD` | `0x1F 0x11 0x00`, `0x11` on or `0xFF` off, `0x00 0x11` | Source: not sent |

The clear command needs 3 zero bytes before the slot. With 2 zero bytes, the
device does nothing. The code had 2 zero bytes until 2026-11-11. The SDK
transport library (`Transport::clearKey`, `Transport::clearAllKeys`) shows the
correct position. (Measured, and Source)

The SDK transport library has one more command that starts with `CLE`:
`CLE`, `0x00 0x00`, `"DC"`. The SDK function is `Transport::disconnected`.
OpenCraft does not send it. (Source)

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
| The length in the `BAT` header is 4 bytes, big-endian. Byte 8 of the command buffer is bits 31 to 24, and byte 11 is bits 7 to 0. The slot follows. | Source | SDK binary, `Transport::setKeyBitmap` |
| `mirajazz` writes `0x00 0x00` and then a 2-byte length. For an image smaller than 65 536 bytes, the bytes are the same as the 4-byte form. | Source | `mirajazz`, `src/device.rs`, `send_image` |
| The SDK puts the image data in parts of `chunkSize() - 1` bytes, with no header in a part | Source | SDK binary, `Transport::setKeyBitmap` |
| The SDK does not send `STP` in an image transfer. A separate call (`refresh`) sends it. | Source | SDK binary, `Transport::refresh`; SDK examples |

The OpenCraft code writes the 4-byte length. A test with the fake transport
examines the header of an image of 70 000 bytes.

"SDK binary" is the transport library of the SDK, `libtransport.so`. Its source
code is not public. We read the operation codes in its constant data, and the
order of the bytes in its machine code. The function names are in its symbol
table. The command buffer in the SDK has no report ID: byte 0 is `C` of
`CRT`. In OpenCraft, the report ID is in front, so each byte number is 1 more.

## Background image

The SDK has two different commands for a full-screen image. Only `BGPIC` is
safe to send.

| Fact | Confidence | Source |
|---|---|---|
| `set_frame_background` sets the background. It sends `BGPIC` through `transport_set_background_frame_stream`. | Source | SDK, `StreamDock.py`, `_set_frame_background`; SDK binary |
| The SDK calls this image a "temporary background image". The SDK sends one `BGPIC` image for each frame of a GIF or MP4 background. | Source | SDK, `LibUSBHIDAPI.py`, `setBackgroundImgFrame`; `GifController.py` |
| `set_touchscreen_image` sets the **boot logo**. It sends `LOG` through `transport_set_background_image_stream`. | Source | SDK, `StreamDockN4Pro.py`, `StreamDock.py`, `_set_boot_logo`; SDK binary |
| The device writes the boot logo to ROM. Incorrect data can make the device unusable. The SDK tells users to set the boot logo only with the vendor software. | Source | SDK, `transport_c.h`, `TransportCWrapper.h`, `Python-SDK/README.md` |
| `BGPIC` header: `"CRT" 0x00 0x00 "BGPIC"`, length (4 bytes), x (2), y (2), width (2), height (2), `0x00`, layer. All numbers are big-endian. | Source | SDK binary, `Transport::setBackgroundFrameStream` |
| The SDK draws a full background at x 0, y 0, with the size 800 × 480, on layer `0x00` | Source | SDK, `LibUSBHIDAPI.py`, `set_background_frame_stream`; `StreamDock.py` |
| The `BGPIC` image data goes in parts, as for `BAT`. The function sends no `STP`. | Source | SDK binary, `Transport::setBackgroundFrameStream` |
| The SDK animation loop sends `STP` after each group of frames | Source | SDK, `GifController.py`, `_gif_work_loop` |
| The background is a JPEG of 800 × 480, turned 180°. The SDK encodes it with quality 60. | Source | SDK, `StreamDockN4Pro.py`, `touchscreen_image_format`; `PILHelper.py`, `encode_background_frame`; `streamdock.cpp`, `setFrameBackgroundStream` |
| A `BGPIC` image must be inside the 800 × 480 area: x + width ≤ 800, and y + height ≤ 480 | Source | SDK, `GifController.py`, `_background_frame_fits` |
| `BGCLE` removes a background image. The SDK calls the argument a layer, and its default is `0x03`. The measured meaning of the argument is in the next table. | Source | SDK binary, `Transport::clearBackgroundFrameStream`; `transport_c.h` |
| `LOG` header: `"CRT" 0x00 0x00 "LOG"`, length (4 bytes, big-endian), `0x01`. The SDK refuses a boot logo larger than 522 240 bytes. | Source | SDK binary, `Transport::setBackgroundImgStream`; `PILHelper.py`, `encode_boot_logo` |
| `mirajazz` and `opendeck-akp05` have no command for the background or the boot logo | Source | Their source code |

The owner looked at the device on 2026-11-11, and the probe sent the commands.
These facts are measured:

| Fact | Confidence |
|---|---|
| `BGPIC` and then `STP` show the background on our device. The image is upright after the 180° turn. | Measured |
| A background JPEG of 110 233 bytes shows correctly. Thus the 4-byte length is correct for `BGPIC`. | Measured |
| After the device stops the connection and connects again, `BGPIC` alone shows nothing. After `DIS` and the brightness command, `BGPIC` shows the image. Which of the two commands is necessary is not measured. | Measured |
| If the display is in operation, `BGPIC` shows the image with no `DIS` before it | Measured |
| The key faces and the bar panels show on top of the background, and the background stays visible around them | Measured |
| `BGCLE 0x03` and then `STP` remove the background. The key faces and the bar panels stay. | Measured |
| `DIS` does not remove the background, the key faces, or the bar panels | Measured |
| After the cable is disconnected and connected again, the background is gone, and the boot logo and the boot animation are the same as before | Measured |

A camera recorded these facts on 2026-11-11:

| Fact | Confidence |
|---|---|
| `BGPIC` shows the image with no `STP` after it | Measured: 1 time |
| The clear command for all slots (`CLE 0xFF`) does not remove the background | Measured |
| The argument of `BGCLE` is an area, not a layer. `0x01` removes the background behind the keys. `0x02` removes it behind the touch bar. `0x03` removes all of it. `0x00` removes nothing. | Measured: 2 times |
| The layer argument of `BGPIC` made no difference that the camera saw. A full image with layer 0, 1, 2, or 3 replaced the image before it. After `BGCLE`, no image from a different layer showed. | Measured |
| `BGPIC` can draw a part of the background. A JPEG of 300 × 100 with x 100, y 50, and a JPEG of 800 × 122 with y 0 and with y 358, changed only that rectangle. The other parts of the background stayed. | Measured |
| x and y of `BGPIC` are in the turned display, as the image data is. For an upright rectangle with the top left corner at (ux, uy), send x = 800 − ux − width, and y = 480 − uy − height. | Measured |
| Thus one image of 800 × 122 with x 0, y 0 draws the touch bar area only (upright y from 358 to 479) | Measured |
| The device showed background JPEG files of 286 904, 460 967, 704 568, and 1 191 260 bytes. The limit is not found. The write time of the largest file was 527 ms. | Measured |

OpenCraft has no builder for `LOG`. A test in `protocol.rs` makes sure that
`protocol.rs` and `connection.rs` do not contain it. The probe command
`background` sends `BGPIC`, then `STP`. With `--clear`, it sends
`BGCLE 0x03`, then `STP`.

## Brightness

A camera recorded these facts on 2026-11-11. The camera adjusts its exposure,
so the values are a comparison, not a measurement of light.

| Fact | Confidence |
|---|---|
| `LIG` with 50 gives a darker display than with 80 or 100 | Measured |
| `LIG` with 0 does not switch off the display. The camera saw no difference between 0 and 10. | Measured |
| `LIG` does not change the knob lights. `LBLIG` does. | Measured |

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

## Repaint time

The probe command `repaint` measured these times on 2026-11-11, with 20
repaints for each row. A repaint is all 14 key faces and bar panels, then one
commit. The time is the time of the writes on the host. It is not the time
until the device shows the images. No person looked at the device.

| Images | Format | Bytes written for one repaint | Time: minimum, median, maximum | Confidence |
|---|---|---|---|---|
| 14 | PNG | Approximately 61 000 | 22.3 ms, 26.2 ms, 28.6 ms | Measured |
| 14 | JPEG, quality 60 | Approximately 66 000 | 28.5 ms, 32.3 ms, 42.7 ms | Measured |
| 14 | JPEG, quality 90 | Approximately 108 000 | 39.8 ms, 42.2 ms, 50.0 ms | Measured |
| 1 key face | PNG | 5 125 | 1.9 ms, 2.3 ms, 11.1 ms | Measured |
| 1 key face | JPEG, quality 60 | 5 125 to 6 150 | 1.8 ms, 2.2 ms, 3.8 ms | Measured |

- The time increases with the number of bytes: approximately 0.4 ms for each
  report of 1025 bytes. (Measured)
- The device sent no acknowledgement and no other input report during the
  repaints. (Measured)
- The probe commands `test` and `raw --draw` print a longer time for a draw
  (approximately 175 ms). That time includes a pause of 150 ms that the probe
  makes after the start command. (Measured)
- These times do not decide the image format. Issue #19 decides it after a
  person looks at the device.

## Keep-alive and stop

| Fact | Confidence | Source |
|---|---|---|
| The SDK and `opendeck-akp05` send `CONNECT` each 10 seconds | Source | Their source code |
| The keep-alive is necessary. After the start commands and a draw, with no keep-alive, the device stops the USB connection approximately 35 seconds later. Input from the user does not change this time. | Measured | Probe, 2026-11-11: 35.1 s, 35.4 s, and 35.0 s after the draw |
| With `CONNECT` each 10 seconds, the connection stays. The longest test was 125 seconds with `opencraft-probe test`. | Measured | Probe, 2026-11-11 |
| With one `CONNECT` 30 seconds after a draw, the device stopped the connection 34.9 seconds after the draw. With one `CONNECT` 33 seconds after a draw, the result was the same. Thus the rule of the device is not "35 seconds after the last `CONNECT`". | Measured: 1 time for each interval | Probe, 2026-11-11: `bars --hold 120 --keep-alive 30`, and the same with 33 |
| A draw does not start the 35 seconds again. In one test, the device stopped the connection 28.3 seconds after a full draw. The last `CONNECT` was some seconds before that draw. | Measured: 1 time | Probe, 2026-11-11 |
| The clear command and the commit, sent to a device that has no timer in operation, do not start the timer. The device stayed on the bus for 75 seconds. | Measured: 1 time | Probe, 2026-11-11 |
| After the device stops the connection, its displays are blank. The device layer opens it again 7 to 8 seconds after it sees the disconnection, and it then draws the key faces and the bar panels again. | Measured: 2 times | Probe and owner, 2026-11-11 |
| The bring-up in Phase 0 passed with no `CONNECT`, because its test was 30 seconds | Inference | The rows above |
| `opendeck-akp05` sends a clear command and then `HAN` when it stops | Source | Its source code |
| After the probe draws the key faces and the bar panels and stops with no clear command, they go blank after a short time. The cause is possibly the same 35 seconds. | Measured (the blank surfaces), Inference (the cause) | Owner, 2026-11-11 |
| The device sent no input report in answer to the draw commands or to `CONNECT` during 100 seconds | Measured | Probe, 2026-11-11 |
| The SDK README refers to `ACK` and `OK` in input reports for newer protocol versions | Source | SDK README |

The device layer sends `CONNECT` each 10 seconds while a device is connected:
`Link::next_event` in `crates/core/src/link.rs` sends it, and the constant
`KEEP_ALIVE_INTERVAL` holds the interval. It sends the keep-alive also when
other commands went to the device, because a draw does not start the 35
seconds again.

## Not known

Measure these in Phase 1.

- The longest interval between two keep-alive commands that keeps the
  connection. 10 seconds keeps it. 30 seconds does not. No value between
  them is tested.
- The rule that the device uses for the keep-alive. One `CONNECT` at 30
  seconds did not prevent the stop at 35 seconds.
- Which commands start the timer of 35 seconds. A draw with the start commands
  starts it. The clear command does not.
- What the device shows after the host process is killed.
- The time until the device shows a full draw. Only the time to send it is
  measured. See "Repaint time".
- Whether the device shows a JPEG key face correctly, and whether small text is
  clear in a JPEG. The device accepted JPEG data with no error, but no person
  looked at the device.
- Which format is smaller for real icons. The test icon is a smooth gradient,
  and PNG is smaller for it.
- Whether the device accepts a `BAT` image larger than 65 535 bytes. The SDK
  writes a 4-byte length (Source).
- Which command starts the display after a new connection: `DIS`, the
  brightness command, or the two together.

## Sources

- Mirabox StreamDock Device SDK (MIT): <https://github.com/MiraboxSpace/StreamDock-Device-SDK>.
  We read commit `fc124cb` on 2026-11-11. The files are:
  - `Python-SDK/src/StreamDock/Devices/StreamDockN4Pro.py` and `StreamDock.py`
  - `Python-SDK/src/StreamDock/Devices/GifController.py`
  - `Python-SDK/src/StreamDock/ImageHelpers/PILHelper.py`
  - `Python-SDK/src/StreamDock/Transport/LibUSBHIDAPI.py`
  - `CPP-SDK/src/Transport/TransportDLL/transport_c.h`
  - `CPP-SDK/src/Transport/TransportCWrapper.h`
  - `CPP-SDK/src/DeviceInfo/streamdock.cpp`
  - The SDK binary: `CPP-SDK/src/Transport/TransportDLL/libtransport.so`
    (x86-64). We read it with `objdump`.
- `mirajazz` (MPL-2.0): <https://github.com/4ndv/mirajazz>
- `opendeck-akp05`: <https://github.com/ambiso/opendeck-akp05>
