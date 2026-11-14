# StreamDock USB protocol, version 3

Last update: 2026-11-14

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
| The firmware text starts at the first byte that `hidapi` returns for the feature report. No report ID byte is in front of it. | Measured | The header of each recording |

## Commands

"Measured" means that a program sent the command to our device, and a person
or a camera saw the result. "Not sent" means that no program sent the command
to our device. How the camera measurements were made is in
[n4-pro-hardware.md](n4-pro-hardware.md#how-the-camera-measurements-were-made).

| Operation | Code | Arguments | Confidence |
|---|---|---|---|
| Image header | `BAT` | Length (4 bytes, big-endian), slot | Measured for an image smaller than 65 536 bytes. The 4-byte length is Source for `BAT`, and Measured for `BGPIC`. |
| Commit | `STP` | None | Measured. See "Start sequence". |
| Background image header | `BGPIC` | Length (4 bytes), x (2 bytes), y (2 bytes), width (2 bytes), height (2 bytes), `0x00`, layer. All big-endian. | Measured: owner and camera, 2026-11-11. See "Background image". |
| Remove the background image | `BGCLE` | Area: `0x01` the key area, `0x02` the touch bar area, `0x03` the two areas | Measured: camera, 2026-11-11 |
| Boot logo header. **Do not send.** | `LOG` | Length (4 bytes, big-endian), `0x01` | Source: not sent |
| Start | `DIS` | None | Measured. See "Start sequence". |
| Brightness | `LIG` | `0x00 0x00`, percent 0 to 100 | Measured: camera, 2026-11-11 and 2026-11-12. See "Brightness". |
| Clear one slot, or all slots with `0xFF` | `CLE` | `0x00 0x00 0x00`, slot | Measured: owner, 2026-11-11 |
| Knob light colours | `SETLB` | Red, green, blue for each knob | Measured: camera, 2026-11-11 and 2026-11-12. See [n4-pro-hardware.md](n4-pro-hardware.md#knob-lights). |
| Knob light brightness | `LBLIG` | Percent 0 to 100 | Measured: camera, 2026-11-11 and 2026-11-12 |
| Reset the knob lights | `DELED` | None | Source: SDK binary, `Transport::resetLedColor`. Sent 1 time: the camera saw no change. OpenCraft has no builder for it. |
| Sleep | `HAN` | None | Measured: camera, 2026-11-11 and 2026-11-12. See "Sleep". |
| Disconnected | `CLE` | `0x00 0x00`, `"DC"` | Measured: camera, 2026-11-11 and 2026-11-12. See "Keep-alive and stop". |
| Keep-alive | `CONNECT` | None | Measured. See "Keep-alive and stop". |
| Set the mode | `MOD` | `0x00 0x00`, ASCII digit | Source: `mirajazz` `set_mode`. Not sent to our device. See "Start sequence". |
| Device configuration, which includes the haptic feedback | `QUCMD` | 6 bytes, one for each setting | Measured: sent, 2026-11-12. See "Device configuration". |

The clear command needs 3 zero bytes before the slot. With 2 zero bytes, the
device does nothing. The code had 2 zero bytes until 2026-11-11. The SDK
transport library (`Transport::clearKey`, `Transport::clearAllKeys`) shows the
correct position. (Measured, and Source)

The SDK transport library has one more command that starts with `CLE`:
`CLE`, `0x00 0x00`, `"DC"`. The SDK function is `Transport::disconnected`, and
the SDK sends it when it closes a device. (Source) This document calls it
`DC`. Its effect is in "Keep-alive and stop". (Measured) The device layer
builder is `protocol::disconnected`.

## Image transfer

1. Send the `BAT` header with the length of the image data and the slot.
2. Send the image data in parts of 1024 bytes. Each part has the report ID byte
   in front.
3. Send `STP` after all images. In one test, the images showed with no
   `STP`. The clear command needs `STP`. See "Start sequence".

| Fact | Confidence | Source |
|---|---|---|
| The device shows PNG and JPEG on key faces and on bar panels, upright and with the correct colours | Measured | Probe and camera, 2026-11-11: `repaint` with PNG, with JPEG quality 90, and with JPEG quality 60 |
| The owner saw no difference between PNG and JPEG with quality 90, for key faces and bar panels with text of 9 to 13 pixels. The surfaces changed between the two formats each 3 seconds, and the owner did not see the change. | Measured | Owner, 2026-11-11 |
| The device shows no slot image that is too large, and it gives no error. A JPEG of 58 187 bytes showed. A JPEG of 60 969 bytes, and each larger one to 133 313 bytes, did not show. The next image showed correctly. | Measured | Camera, 2026-11-11: 12 sizes from 39 485 to 133 313 bytes |
| Thus the test with a `BAT` length above 65 535 has no result: the device refuses such an image for its size | Inference | The row above |
| A bar panel image of noise has 59 316 bytes as PNG and 37 460 bytes as JPEG with quality 90. For a key face, it has 37 812 and 24 096 bytes. | Measured | The encoder of the device layer |
| The largest JPEG that we made is noise of saturated colours. With quality 90, it has 45 832 bytes for a bar panel and 29 400 bytes for a key face. With quality 95, it has 57 755 bytes for a bar panel. | Measured | The encoder of the device layer |
| For 95 application icons of macOS at 112 × 112, the median size is 8 924 bytes as PNG and 4 776 bytes as JPEG with quality 90. For 313 icons of the vendor plug-ins, it is 5 587 and 2 937 bytes. | Measured | The encoder of the device layer |
| For 10 key faces with small text on a flat colour, the mean size is 4 929 bytes as PNG and 4 346 bytes as JPEG with quality 90 | Measured | The encoder of the device layer |
| The encoder of the device layer needs approximately 0.05 ms for a PNG key face and 0.12 ms for a JPEG key face. It writes JPEG with no chroma subsampling (4:4:4). | Measured | The encoder, release build, the computer of the owner |
| The SDK sends a PNG only for a key image that has transparency, with a limit of 29 000 bytes. It sends a JPEG for each other key image. It starts with quality 100 and decreases the quality until the image is 9 000 bytes or smaller. | Source | SDK, `PILHelper.py`, `encode_key_image` |
| The image must be turned 180° before it is sent | Measured | Probe |
| `mirajazz` and `opendeck-akp05` send JPEG to the same slots | Source | Their source code |
| The length in the `BAT` header is 4 bytes, big-endian. Byte 8 of the command buffer is bits 31 to 24, and byte 11 is bits 7 to 0. The slot follows. | Source | SDK binary, `Transport::setKeyBitmap` |
| `mirajazz` writes `0x00 0x00` and then a 2-byte length. For an image smaller than 65 536 bytes, the bytes are the same as the 4-byte form. | Source | `mirajazz`, `src/device.rs`, `send_image` |
| The SDK puts the image data in parts of `chunkSize() - 1` bytes, with no header in a part | Source | SDK binary, `Transport::setKeyBitmap` |
| The SDK does not send `STP` in an image transfer. A separate call (`refresh`) sends it. | Source | SDK binary, `Transport::refresh`; SDK examples |

The OpenCraft code writes the 4-byte length. A test with the fake transport
examines the header of an image of 70 000 bytes.

The device layer sends JPEG with quality 90 to key faces and bar panels. See
[decision 0007](../decisions/0007-jpeg-for-key-faces-and-bar-panels.md).

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
| After the device stops the connection and connects again, `BGPIC` alone shows nothing. After `DIS` and the brightness command, `BGPIC` shows the image. | Measured |
| If the display is in operation, `BGPIC` shows the image with no `DIS` before it | Measured |
| The key faces and the bar panels show on top of the background, and the background stays visible around them | Measured |
| `BGCLE 0x03` and then `STP` remove the background. The key faces and the bar panels stay. | Measured |
| `DIS` does not remove the background, the key faces, or the bar panels, if it is not the first `DIS` of the connection | Measured |
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

A camera recorded these facts on 2026-11-13, with the engine of OpenCraft
and the page background of
[decision 0032](../decisions/0032-page-background.md):

| Fact | Confidence |
|---|---|
| `BGPIC`, then slot images, then one `STP` in one transfer show the background and the slot images. This occurred in the full draw after `DIS` and at a page change. | Measured: 6 times |
| `BGCLE 0x03`, then slot images, then one `STP` remove the background at a page change | Measured: 1 time |
| The device showed a background, OpenCraft quit, and OpenCraft started again on a page with no background. The full draw sent `BGCLE 0x03` after the device settings, and the device showed no background. It is not known if the device keeps a background across a quit and a start with no `BGCLE`. | Measured: 1 time |
| A key face image that is the part of the background at the position of the key face (from [n4-pro-hardware.md](n4-pro-hardware.md#positions-on-the-display)) continues the lines of the background around it. The camera saw no offset. | Measured |

OpenCraft has no builder for `LOG`. A test in `protocol.rs` makes sure that
`protocol.rs` and `connection.rs` do not contain it. The probe command
`background` sends `BGPIC`, then `STP`. With `--clear`, it sends
`BGCLE 0x03`, then `STP`.

## Brightness

A camera recorded these facts on 2026-11-11 and 2026-11-12. The camera adjusts
its exposure, so the values are a comparison, not a measurement of light. On
2026-11-12, the test program drew green on all 14 slots and measured the mean
green in the area of the key faces and in the area of the touch bar.

| Fact | Confidence |
|---|---|
| `LIG` with 50 gives a darker display than with 80 or 100 | Measured |
| `LIG` with 0 to 15 gives the same brightness. The values 0, 1, 2, 3, 5, 10, 12, 14, and 15 gave the same picture. `LIG` with 0 does not switch off the display. | Measured: 2 runs, 2026-11-12 |
| From 16 to 100, each higher value that was tested gave a brighter display: 16, 18, 20, 22, 25, 30, 40, 50, 60, 80, 100 | Measured: 2 runs, 2026-11-12 |
| Thus the values that change the brightness are 15 to 100 | Inference |
| The key faces and the bar panels change together. One `LIG` command sets the brightness of the full display. | Measured, 2026-11-12 |
| `LIG` does not change the knob lights. `LBLIG` does. | Measured |
| After `DC` and a new `DIS` with no `LIG`, the brightness is the last value of `LIG`. This was the result after `LIG` 100, 50, 10, and 0. | Measured, 2026-11-12 |
| After the device stops the USB connection and starts again, the brightness is not kept. After `LIG` 10 and a stop, `DIS` with no `LIG` gave a display approximately as bright as with `LIG` 100. | Measured: 2 times, 2026-11-12 |
| The brightness command wakes the displays after `HAN` | Measured. See "Sleep". |

The engine sent these commands on 2026-11-12, with white on the 10 key faces.
The camera measured the mean light in the area of the key faces, minus the
mean light of the dark device.

| Fact | Confidence |
|---|---|
| `LIG` 15 gave 87 to 98 in 4 measurements. `LIG` 50 gave 138. `LIG` 100 gave 153 to 154 in 3 measurements. Thus the lowest value of the setting (15) is clearly darker than the highest. | Measured: camera, 2026-11-12 |
| 86 brightness commands in 0.86 seconds, one each 10 ms from 15 to 100, gave no write error and no disconnection. 1.5 seconds later, the display showed the brightness 100. | Measured: camera, 1 time, 2026-11-12 |

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
- These times do not decide the image format. See
  [decision 0007](../decisions/0007-jpeg-for-key-faces-and-bar-panels.md).

A camera of 30 pictures each second measured the time until the device shows
a repaint, on 2026-11-11. A test program sent 14 images and the commit, 16
times for each format. The time is from the end of the commit write to the
first picture that shows the new image. It includes the delay of the camera,
which is not measured.

| Format | First image of the repaint, median | Last image of the repaint, median | Confidence |
|---|---|---|---|
| PNG | 71 ms and 73 ms | 118 ms and 116 ms | Measured: 2 runs |
| JPEG, quality 90 | 72 ms and 83 ms | 117 ms and 120 ms | Measured: 2 runs |
| JPEG, quality 60 | 76 ms | 122 ms | Measured: 1 run |

- The device shows PNG and JPEG after the same time. (Measured)
- The device shows the images one after the other, in the order of the
  transfer. An image can show before the commit. (Measured)
- A full repaint shows less than 0.2 seconds after its first write: the
  writes are less than 50 ms, and the last image shows less than 130 ms after
  the commit, with the delay of the camera. (Measured)
- After `BGPIC` and `STP` for a full background, the camera showed the new
  background 110 to 180 ms after the commit. (Measured: 6 times)

## Start sequence

A camera recorded these facts on 2026-11-11. A "new connection" is the state
after the cable is connected, or after the device stops the USB connection
and connects again.

| Fact | Confidence |
|---|---|
| After a new connection, the device shows its logo on an animated background. See [n4-pro-hardware.md](n4-pro-hardware.md#the-device-with-no-host). | Measured |
| The first `DIS` of a connection removes the logo, the animation, and each slot image. The displays are black. The knob lights come on with the default effect. The timer of the keep-alive starts. | Measured |
| The SDK name for `DIS` is "wake up the screen" (`Transport::wakeupScreen`) | Source: SDK binary |
| A second `DIS` does not remove the images and does not start the timer again | Measured |
| After `DIS`, slot images show with no brightness command | Measured: 1 time |
| A slot image that goes out before `DIS` shows on top of the logo animation. `DIS` then removes it. | Measured |
| The brightness command before `DIS` changes nothing that the camera saw | Measured: 1 time |
| `BGPIC` before `DIS` shows nothing | Measured: owner, 1 time |
| **If `STP` goes out before the first `DIS` of a connection, slot images that go out after `DIS` do not show.** This occurred with `STP` alone, with `CLE 0xFF` and `STP`, and with images and `STP`, before `DIS`. | Measured: 5 times |
| In that state, `CLE 0xFF` after `DIS` corrects the fault: slot images show again. A second `STP`, a second `DIS`, and a time of 3 seconds do not correct it. | Measured |
| `CLE 0xFF` with no `STP` before `DIS`, and images with no `STP` before `DIS`, do not cause the fault | Measured: 1 time each |
| A slot image shows with no `STP` after it | Measured: 1 time |
| `CLE` has an effect only after `STP` | Measured: 1 time |

Thus the safe sequence after each connection is: `DIS`, the brightness
command, `CLE 0xFF`, `STP`, the images, `STP`. Do not send `STP` before
`DIS`. The device layer function `Device::start` in
`crates/core/src/connection.rs` sends `DIS`, the brightness command,
`CLE 0xFF`, and `STP`. The probe commands `test`, `raw --draw`, `bars`,
`wide`, `repaint`, and `send --draw` use it. The command `background` uses it
only with `--init`. The command `draw` sends only `DIS`. The probe command
`clear` sends `CLE` and `STP` with no `DIS`. On a new connection, that causes
the fault for a program that does not send `CLE 0xFF` after `DIS`.

No pause is necessary in the sequence. Until 2026-11-11, the probe made a
pause of 150 ms after `DIS`. A test program then sent the sequence and 14
images with no pause, on 5 new connections. The writes took 30 to 49 ms, and
a camera saw all 14 images each time. (Measured: camera, 2026-11-11)

`MOD` is not part of this sequence. We read the sources again on 2026-11-14.

| Fact | Confidence | Source |
|---|---|---|
| The vendor SDK names `transport_change_mode` an N1 mode switch. The classes of the N4 and the N4 Pro do not call it. | Source | SDK commit `fc124cb`: `transport_c.h`, `streamdockN4.cpp`, `streamdockN4Pro.cpp` |
| The Python class `StreamDockN1` calls `switchMode(2)` at the end of `open`. It makes no pause after that call. | Source | SDK, `StreamDockN1.py`, `open` |
| `mirajazz` 0.16.3 has `set_mode` for a device that has more than one mode. The N1 example sends the mode, waits 50 ms, then sends the brightness and the clear. The example does not send `DIS`. | Source | `src/device.rs`, `examples/n1.rs`, CHANGELOG 0.7.0 |
| These sources do not give a position for `MOD` in the start sequence, and they do not give a pause for that sequence. | Inference | The three rows above |
| The probe option `--mode` of `test` sends `MOD` before `Device::start`, then waits 300 ms. `Device::start` does not send `MOD`. | Measured | `crates/cli/src/main.rs`, `crates/core/src/connection.rs` |

## Keep-alive and stop

All rows are measured on 2026-11-11, with no person at the device. A test
program sent the commands and recorded the time at which the device stopped
the USB connection. The times are in seconds after `DIS`.

| `CONNECT` sent at | The device stopped the connection at | First period with no `CONNECT`, after the period 0 to 15 | Stop that the rule gives |
|---|---|---|---|
| Not sent | 35.0, 35.1, 35.4 | 15 to 30 | 35 |
| 10.0 | 34.9 | 15 to 30 | 35 |
| 14.0 | 34.9 | 15 to 30 | 35 |
| 16.0 | 49.9 | 30 to 45 | 50 |
| 20.0 | 49.9 | 30 to 45 | 50 |
| 29.0 | 49.9 | 30 to 45 | 50 |
| 30, and in a second test 33 | 34.9 | 15 to 30 | 35 |
| 16.0 and 44.1 | 64.9 | 45 to 60 | 65 |
| Each 20 seconds: 20.0, 40.1, 60.1 | 64.9 | 45 to 60 | 65 |
| Each 16 seconds, 15 times: 16.0, 32.1, and so on, the last 2 at 224.4 and 240.5 | 244.6 | 225 to 240 | 245 |
| Each 14 seconds | Not stopped in 300 seconds | None | No stop |
| Each 10 seconds | Not stopped in 47 minutes | None | No stop |

One rule agrees with each row:

- `DIS` starts a timer. The timer has periods of 15 seconds: 0 to 15, 15 to
  30, 30 to 45, and so on.
- The device must get one `CONNECT` or more in each period, but not in the
  first period (0 to 15 seconds). A `CONNECT` in the first period does not
  count for a later period. A `DIS` that is not the first `DIS` of the timer
  counts as a `CONNECT`. (Measured on 2026-11-12. See the next table.)
- If a period ends with no `CONNECT`, the device stops the USB connection
  5 seconds after the end of that period. A `CONNECT` in these 5 seconds does
  not prevent the stop.

The rule is an inference from the rows. It gave the correct time for the test
with 16 seconds before that test ran: the period from 225 to 240 seconds had
no `CONNECT`.

| Fact | Confidence | Source |
|---|---|---|
| The keep-alive is necessary | Measured | The table above |
| An interval that is less than 15 seconds puts a `CONNECT` in each period. An interval of exactly 15 seconds is not tested. | Inference | The rule |
| Only `DIS` starts the timer. On a connection with no timer in operation, the device stayed connected for 60 seconds after each of these: no command, the brightness command, an image and `STP`, `CONNECT`, `STP`. After `DIS` alone, it stopped the connection at 35.1 seconds. | Measured | Test program |
| A second `DIS`, a draw, and input from the user do not start the timer again and do not move its periods. A test that started 300 seconds after the first `DIS`, on the same connection, kept the periods of the first `DIS`. | Measured | Test program, probe |
| A `DIS` that is not the first `DIS` of the timer counts as a `CONNECT` in its period. `DIS` at 8 seconds and no `CONNECT`: the stop came at 35.1 seconds. `CONNECT` at 10 seconds, `DIS` at 22 seconds, and nothing after: the stop came at 50.1 seconds. `DIS` at 22 seconds and `CONNECT` at 32, 42, and 52 seconds: no stop in 62 seconds. | Measured: 2026-11-12. The last case 3 times, on a new connection and after `DC`. | Test program |
| `DC` stops the timer. With no `CONNECT` after `DC`, the device stayed connected for 120 seconds. | Measured: 2026-11-12, 3 times | Test program |
| The first `DIS` after `DC` starts a new timer, as the first `DIS` of a connection does. With no `CONNECT` after it, the device stopped the connection 35.1 seconds after that `DIS`. | Measured: 2026-11-12, 1 time | Test program |
| A process that is killed 12 seconds after `DIS`, and a new process that sends the start sequence at 22 seconds and `CONNECT` each 10 seconds: the device stayed connected. When the new process sent only `CONNECT`, the device stopped the connection at 35 seconds. The rows above give these results. | Measured: 2026-11-12, 2 times each | Test program |
| A start sequence directly after `DC`, with no pause, is lost. The logo stays, and the images that follow the start sequence do not show. Images that come later show. After a pause of 0.1, 0.3, 0.5, 1, or 2 seconds, the start sequence and the images showed. | Measured: 2026-11-12. No pause: 2 times. Each pause: 1 time. | Test program and camera |
| The device sent no input report in answer to a command in these tests. No acknowledgement for a write was seen. | Measured | Test program, probe |
| With the keep-alive of the device layer each 10 seconds, one connection stayed for 47 minutes. The images stayed, and a key press at the end arrived. | Measured | Probe (`raw --draw`), camera, and owner, 2026-11-11 |
| The SDK and `opendeck-akp05` send `CONNECT` each 10 seconds | Source | Their source code |
| When the device stops the connection, it starts again: it shows the boot animation and then the logo, and the knob lights go off. The images are gone. | Measured | Camera |
| The device is on the bus again 6 to 7 seconds after the stop. The device layer opens it again 7 to 8 seconds after it sees the disconnection, and the probe then draws again. | Measured | Test program: 20 times. Probe and owner: 2 times. |
| The function `shutdown` of `mirajazz` sends `DC` and then `HAN`. Which programs use it is in [prior-art.md](prior-art.md#opendeck-akp05). | Source | `mirajazz`, `src/device.rs` |
| The SDK README refers to `ACK` and `OK` in input reports for newer protocol versions | Source | SDK README |

What the user sees when the host stops:

| Case | Result | Confidence |
|---|---|---|
| The probe stops 12 seconds after `DIS` and sends no clear command (`test --seconds 12 --keep`). Its last `CONNECT` was at 10 seconds. | The images and the knob lights stay. The keys and the knobs do nothing. At 35 seconds after `DIS`, the device starts again and shows the logo. | Measured: camera |
| The probe is killed (`kill -9`) 15 seconds after `DIS` | The same. The logo showed at 36 seconds, and it was the same after 10 minutes. | Measured: camera |
| General | The images stay until 5 seconds after the end of the first period that has no `CONNECT`. That is 20 to 35 seconds after the last `CONNECT` that was in a period and not in the 5 seconds before a stop. | Inference from the rule |
| VSD Craft quits | The displays and the knob lights go off, and the device does not show the logo. It was the same 50 seconds later. The next `DIS` and brightness command showed the last background of VSD Craft again. Which commands VSD Craft sends is not known. | Measured: camera, 1 time |

On 2026-11-12, a test program measured each stop sequence in the next
table one time, with a camera. Before each stop, the slots were green, the
knob lights were blue, and the last `CONNECT` was at 10 seconds after `DIS`.
The program sent the sequence at 11 seconds after `DIS`, closed the device,
and exited. Then the camera recorded the device for the given time. The
program then did a normal start: `DIS`, `LIG`, `CLE 0xFF`, `STP`, the
images, the knob light commands, and `CONNECT` each 10 seconds.

| Stop sequence | Displays | Knob lights | USB connection | A normal start after it | Confidence |
|---|---|---|---|---|---|
| None: the process is killed (`abort`) | The images stay. The logo shows when the device starts again. | On until the device starts again, then off | The device stopped it at 35 seconds after `DIS`, and was on the bus again 7 seconds later | Shows the images and the knob lights | Measured, 50 seconds |
| `CLE 0xFF`, `STP` | Black, then the logo when the device starts again | On until the device starts again, then off | Stopped at 35 seconds after `DIS` | Shows the images | Measured, 50 seconds |
| `HAN` | Dark, then the logo when the device starts again | Off | Stopped at 35 seconds after `DIS` | Shows the images | Measured, 50 seconds |
| `LIG 0` | The images stay, dimmer. Then the logo when the device starts again. | On until the device starts again, then off | Stopped at 35 seconds after `DIS` | Shows the images | Measured, 50 seconds |
| `DC` | The logo, in less than 1 second, for all of the 120 seconds | Off | Not stopped | Shows the images | Measured, 120 seconds. The same result 1 time on 2026-11-11, and at the end of each test on 2026-11-12. |
| `DC`, then `HAN` | Dark, for all of the 120 seconds | Off | Not stopped | Shows the images. The brightness command in the start wakes the displays. | Measured, 120 seconds |
| `HAN`, then `DC` | Dark, for all of the 60 seconds | Off | Not stopped | Shows the images | Measured, 60 seconds |

- The device got `DC` and `HAN` when the process exited directly after the
  two writes, with no pause and no close of the device. The displays were
  dark each time. (Measured: 3 times, 2026-11-12)
- `DC` with byte 1 of the device configuration set to `0xFF` also gives dark
  displays and knob lights that are off. See "Device configuration".
  (Measured)
- The dark result of VSD Craft agrees with `HAN`, and with byte 1 of the
  device configuration set to `0xFF`. Which of the two VSD Craft uses is not
  known. (Inference)

The state that OpenCraft gives the device at its stop is in
[decision 0022](../decisions/0022-device-settings-and-stop-state.md).
`Device::stop` in `crates/core/src/connection.rs` sends `DC` and then `HAN`.
`Link::stop` in `crates/core/src/link.rs` sends them, closes the device, and
then sends no keep-alive and no other command. The engine calls it at its
stop. The probe command `test` sends the same sequence at its end, but not
with `--keep`.

The device layer sends `CONNECT` each 10 seconds while a device is connected:
`Link::next_event` in `crates/core/src/link.rs` sends it, and the constant
`KEEP_ALIVE_INTERVAL` holds the interval. It sends the keep-alive also when
other commands went to the device, because a draw does not start the timer
again. With 10 seconds, the time between two keep-alive commands stays less
than 15 seconds if one keep-alive is less than 5 seconds late. (Inference
from the rule) The interval uses `Instant`, which does not count the time of
a sleep of the computer. Thus after a sleep, the link moves the next
keep-alive earlier by the time of the sleep, or sends it at once. See
[decision 0023](../decisions/0023-wake-from-the-sleep-time-of-the-clocks.md).

## Sleep

A camera recorded these facts on 2026-11-11.

| Fact | Confidence |
|---|---|
| `HAN` switches off the displays and the knob lights | Measured |
| The device keeps the background and the slot images during the sleep | Measured |
| `CONNECT` does not wake the device. A slot image and `STP` do not wake it. | Measured |
| The brightness command switches the displays on again, with the same images. The knob lights stay off. | Measured |
| `DIS` switches the knob lights on again, with the colours that they had | Measured |
| `HAN` does not stop the timer of the keep-alive. With no `CONNECT` after `HAN`, the device stopped the connection at the time that the rule gives. | Measured: 1 time |

Thus the commands to wake the device are the brightness command and `DIS`.

On 2026-11-12, a knob turn after `DC` and `HAN` changed nothing: the displays
and the knob lights stayed off. (Measured: owner.) The key-press result is in
[Input after the dark states (2026-11-14)](#input-after-the-dark-states-2026-11-14).

### Dark-state check (2026-11-13)

A scratch driver outside the repository sent commands to the vendor interface.
A camera took fresh frames after each wait. No person touched the controls.
The driver sent `CONNECT` each 8 seconds except in the stop-state check.

| Fact | Confidence | Source |
|---|---|---|
| With green on all 14 slots, `LIG 0` and `LBLIG 0` left the images visible for 40 seconds. Only the knob lights went off. | Measured | Scratch driver and camera, 1 run |
| `HAN` made the displays and knob lights dark. They stayed dark at 2 and 42 seconds with the keep-alive in operation. | Measured | Same run |
| One more `CONNECT` after that did not wake the display in the next 2 seconds | Measured | Same run |
| A full start (`DIS`, `LIG 80`, clear, images, settings, commit) restored all green slots and blue knob lights in the next 2 seconds | Measured | Same run |
| `DC`, then `HAN`, stayed dark for 45 seconds with no keep-alive. The same full start restored the images and lights. | Measured | Same run |
| Thus brightness 0 cannot implement screen off on this device | Inference | The first row |

The signed development bundle also sent both dark-state actions through
**Test the action** on 2026-11-13. With a scratch green page, the camera saw
dark displays and knob lights after each action. An open-window request
restored the green page and lights after each action. (Measured: one run for
each action.) This checks computer-side wake, not physical input.

### Input after the dark states (2026-11-14)

A test program that is not in the repository drew a test pattern, sent
commands, and read the vendor interface. A camera recorded the device. For
Screen off, the owner check is page 1, step 2 of issue
[#255](https://github.com/shantanugoel/OpenCraft/issues/255). For the stop
state, the owner pressed one key. The program read the interface for 240
seconds and sent no keep-alive after `DC` and `HAN`.

| Fact | Confidence | Source |
|---|---|---|
| After `HAN` with the keep-alive (Screen off), a key press sends an input report. The first gesture wakes the device and starts no action. | Measured | Owner check, 2026-11-14 |
| After `DC` and then `HAN`, a key press sends no input report. The press sends none, and the release sends none. | Measured | Test program, camera, and one key press, 2026-11-14 |
| After that key press, the device shows the firmware logo. The logo was lit in less than 2 seconds. It stayed lit for the next 50 seconds. The device was dark for 80 seconds before the press. | Measured | Same run |
| A key cannot wake the stop state into the profile. Only the computer can wake that state, because the engine cannot see a press that sends no report. | Inference | The two rows above |

The policy is in
[decision 0034](../decisions/0034-device-dark-state.md). This question is not
in the [Not known](#not-known) list: the measurement gives the answer.

## Device configuration

`QUCMD` sets 6 device settings in one command. The haptic feedback is one of
them. We read the sources of the next table on 2026-11-12.

| Fact | Confidence | Source |
|---|---|---|
| The command is `QUCMD` and 6 bytes, one for each setting, in this order: 0 `LedFollowKeyLight`, 1 `KeyLightOnDisconnect`, 2 `CheckUsbPower`, 3 `EnableVibration`, 4 `ResetUsbReport`, 5 `EnableBootVideo` | Source | SDK, `streamdockN4Pro.h` (`N4ProConfigEnumerate`) and `streamdockconfigcmd.h` |
| The SDK values are `0x11` on, `0xFF` off, and `0x1F` "follow". The SDK sends `0x11` for each setting that the caller does not give. | Source | SDK, `streamdockconfigcmd.h` (`ConfigState`, `buildConfigCommand`) |
| `mirajazz` sends `0x1F 0x11 0x00`, then `0x11` (vibration on) or `0xFF` (vibration off), then `0x00 0x11`. It says that these bytes are the bytes of the vendor software. | Source | `mirajazz`, `src/device.rs`, `set_vibration`; `opendeck-akp05`, `src/mappings.rs` (a USB capture on the N4 Pro E) |
| The device layer function `protocol::set_vibration` makes the bytes of `mirajazz` | Measured | `crates/core/src/protocol.rs` |

In this document, "byte N" is the byte at position N in the list of 6.

A test program sent `QUCMD` on 2026-11-12. A camera recorded the device. No
person touched the device.

| Fact | Confidence |
|---|---|
| The device accepts `QUCMD` with byte 3 `0xFF` and with byte 3 `0x11`. The write gives no error, and the device sends no input report in the next 3 seconds. The displays, the knob lights, and the USB connection do not change. | Measured: 2 times each |
| With byte 1 `0xFF`, `DC` makes the displays dark and switches off the knob lights. The device does not show the logo. With byte 1 `0x11` again, `DC` shows the logo again. | Measured: 2 times each |
| The device keeps the configuration when it stops the USB connection and starts again. After byte 1 `0xFF` and a stop, `DC` on the new connection made the displays dark, with no new `QUCMD`. | Measured: 1 time |
| With byte 1 `0xFF`, the device still shows the boot animation and the logo when it starts again after a stop | Measured: 1 time |
| Byte 3 switches the vibration at a key press. With `0xFF`, the device did not vibrate at a key press. With `0x11`, it vibrated. The test program sent the bytes of `mirajazz`, and the owner pressed a key 2 times in each state. | Measured: owner, 2026-11-12 |
| The device keeps byte 3 (vibration) in the same way, because the same command sets it | Inference |
| At the end of the tests, the test program sent the bytes of `mirajazz` with vibration on: `0x1F 0x11 0x00 0x11 0x00 0x11` | Measured |

## Not known

Phase 1 did not answer these questions. Each question has a reason.

| Question | Reason that it stays |
|---|---|
| The exact limit for the size of a slot image. It is between 58 187 and 60 969 bytes. | The largest image that we made with the encoder of the device layer has 45 832 bytes. That is more than 12 000 bytes below the smaller value. See [decision 0007](../decisions/0007-jpeg-for-key-faces-and-bar-panels.md). |
| The exact time until the device shows an image. The times in "Repaint time" include the delay of the camera, which is not measured. | The times are an upper limit, and they are short: less than 0.2 seconds for a full repaint. The time of a page change is in [footprint.md](footprint.md#time-of-a-page-change-2026-11-12). |
| The largest JPEG that the device accepts for `BGPIC` | The device showed a file of 1 191 260 bytes. A background of 800 × 480 at a useful quality is much smaller. |
| Whether an interval of exactly 15 seconds between two keep-alive commands keeps the connection | The device layer uses 10 seconds, as the vendor SDK does. 14 seconds is measured. No design needs a longer interval. |
| What the layer argument of `BGPIC` does. Each value gave the same result. | The device layer sends layer 0 for a full background, as the SDK does. No feature needs a different layer. |
| What `MOD` does on the N4 Pro. No program sent it to our device. | The sources use `MOD` for the N1. The N4 Pro operates with no `MOD`. |
| Whether the device keeps the `QUCMD` configuration after the cable is disconnected | It needs a person. OpenCraft sends the command at each connection. |
| What bytes 0, 2, 4, and 5 of `QUCMD` do, and what the value `0x00` does | OpenCraft sends the bytes of `mirajazz` for them. No feature needs them. Byte 2 (`CheckUsbPower`) and byte 4 (`ResetUsbReport`) can possibly change the USB behaviour, so do not test other values without a reason. |
| Which commands VSD Craft sends when it quits | `DC` and `HAN` give the same dark state. See [decision 0022](../decisions/0022-device-settings-and-stop-state.md). |

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
  - On 2026-11-12: `CPP-SDK/src/HotspotDevice/StreamDockN4Pro/streamdockN4Pro.h`
    and `CPP-SDK/src/DeviceInfo/streamdockconfigcmd.h`
  - On 2026-11-14: `transport_change_mode` in `transport_c.h`,
    `Python-SDK/src/StreamDock/Devices/StreamDockN1.py` (`open`),
    `StreamDockN4.py`, `streamdockN4.cpp`, and `streamdockN4Pro.cpp`
- `mirajazz` (MPL-2.0): <https://github.com/4ndv/mirajazz>. On 2026-11-12 we
  read version 0.16.3, `src/device.rs`. On 2026-11-14 we read `set_mode`,
  `examples/n1.rs`, and CHANGELOG 0.7.0 in the same version.
- `opendeck-akp05`: <https://github.com/ambiso/opendeck-akp05>. On 2026-11-12
  we read version 0.11.1, `src/device.rs`, `src/config.rs`, and
  `src/mappings.rs`.
- The measurements of 2026-11-12 used a test program that is not in the
  repository. It uses the command builders of the device layer.
