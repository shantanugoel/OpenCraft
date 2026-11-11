# VSD N4 Pro hardware

Last update: 2026-11-11

The VSD N4 Pro is a Mirabox StreamDock N4 Pro with a different brand name.
VSDinside is the retail brand. The protocol is in
[streamdock-protocol.md](streamdock-protocol.md).

## Identity

| Fact | Confidence | Source |
|---|---|---|
| VID:PID is `0x5548:0x1023` | Measured | `ioreg`, probe |
| Product string is `HOTSPOTEKUSB HID DEMO` | Measured | `ioreg` |
| Firmware version is `V4.VSD N4 Pro.02.010` | Measured | Probe, feature report `0x01` |
| Interface 0 is the vendor control interface: usage page `0xFFA0`, 512 bytes in, 1024 bytes out | Measured | `hidutil`, probe |
| Interface 1 is keyboard emulation: usage page `0x0001`, usage `6`, 9 bytes in | Measured | `hidutil` |
| The vendor interface is not locked for exclusive use. The probe can open it while the vendor software runs. | Measured | Probe |
| `0x5548:0x1008` is the Mirabox N4 Pro. `0x5548:0x1021` is the Mirabox N4 Pro E. | Source | `opendeck-akp05` README |

The descriptors for `0x1008` and `0x1021` are in the code. They are not tested
on hardware.

## Physical layout

| Fact | Confidence | Source |
|---|---|---|
| Top: 10 keys with displays, with a background display behind them | Measured | Owner, 2026-11-11 |
| Middle: one touch bar across the full width. It is a display and a touch sensor. | Measured | Owner, 2026-11-11 |
| Bottom: 4 knobs. The user can turn and push each knob. | Measured | Owner, 2026-11-11 |
| The touch bar is one 2.8 inch touch panel | Source | Mirabox specification |
| Each knob has a ring light with a colour that software can set | Source | VSDinside product text |
| The 10 keys are in 2 rows of 5 | Measured | Owner, 2026-11-11 |
| Top row, from left to right: key slots 11, 12, 13, 14, 15 | Measured | Owner, 2026-11-11: `opencraft-probe test` shows the slot numbers |
| Second row, from left to right: key slots 6, 7, 8, 9, 10 | Measured | Owner, 2026-11-11 |
| Touch bar, from left to right: bar panels 1, 2, 3, 4 | Measured | Owner, 2026-11-11 |

## Surfaces

| Surface | Slots | Size | Format | Rotation | Confidence |
|---|---|---|---|---|---|
| Key face | 6 to 15 | 112 × 112 | PNG | 180° | Measured: the probe paints them |
| Bar panel | 1 to 4 | 176 × 112 | PNG | 180° | Measured: the probe paints them |
| Background | None | 800 × 480 | JPEG | 180° | Measured: the probe paints it. Owner, 2026-11-11 |

The device has one display of 800 × 480 pixels behind the keys and the touch
bar. Each key and the touch bar is a window on this display. The owner
photographed a background test pattern with a grid and coordinates on
2026-11-11. The values below are from that photograph, so they are
approximate. (Measured)

| Window | Part of the 800 × 480 image that shows |
|---|---|
| Keys, top row | y from 0 to approximately 110. The left edge of the first key is near x = 0. The right edge of the fifth key is near x = 800. |
| Keys, second row | y from approximately 165 to approximately 275 |
| Touch bar | The full width, x from 0 to 800. y from approximately 370 to 480. |

- The areas between these windows do not show.
- A key face image and a bar panel image show on top of the background. The
  background stays visible around them. (Measured: owner, 2026-11-11)
- The touch bar shows 800 pixels of width. This agrees with the touch
  coordinates. The 4 bar panels together are 704 pixels wide. (Measured)

- Slot 5 is not used. The SDK image map has no entry for it. (Source)
- The 4 bar panels are the display of the touch bar. The SDK calls them the
  "secondary screen". (Source)
- The device has no display other than these. The SDK has no other draw
  method. (Source)
- 800 × 480 is the size of the full display, not of the touch bar. The SDK method
  `set_touchscreen_image` sets the boot logo, not the background. The method
  name caused an error in the first plan. (Source)
- The SDK method `set_frame_background` sets the background. The SDK calls it
  a temporary background. (Source)
- The device writes the boot logo to ROM. OpenCraft does not send the boot
  logo command. See
  [streamdock-protocol.md](streamdock-protocol.md#background-image). (Source)
- The SDK encodes the background as a JPEG with quality 60. (Source)
- The code calls a bar panel a "knob display" (`PanelKind::Knob`). Phase 1
  changes this name.

## Controls and input codes

All rows are measured on 2026-11-11. The recorded reports are in
`crates/core/tests/fixtures/input/`. The report format is in
[streamdock-protocol.md](streamdock-protocol.md#input-reports).

| Control | Gesture | Codes | State byte | Confidence |
|---|---|---|---|---|
| Key, top row, from left to right (slots 11 to 15) | Press, release | `0x01` to `0x05` | `0x01` press, `0x00` release | Measured |
| Key, second row, from left to right (slots 6 to 10) | Press, release | `0x06` to `0x0A` | `0x01` press, `0x00` release | Measured |
| Knob 1 to 4, from left to right | Turn left / turn right | `0xA0/0xA1`, `0x50/0x51`, `0x90/0x91`, `0x70/0x71` | `0x00` | Measured |
| Knob 1 to 4, from left to right | Push | `0x37`, `0x35`, `0x33`, `0x36` | `0x01` | Measured |
| Touch bar | Finger down | `0x00` | `0x01` | Measured |
| Touch bar | Finger position, while the finger is down | Touch report (`ARX`), one each 35 ms | None | Measured |
| Bar panel 1 to 4, from left to right | Finger up with no move (tap) | `0x40` to `0x43` | `0x00` | Measured |
| Touch bar | Finger up after a move to the right | `0x38` | `0x00` | Measured |
| Touch bar | Finger up after a move to the left | `0x39` | `0x00` | Measured |
| Touch bar | Finger up, gesture not identified | `0xB2` | `0x00` | Measured: 2 times |

Keys:

- A key held for 3 seconds sends one press and one release. It sends no
  repeat. (Measured)
- In one test, 2 adjacent keys were pressed together. The device sent the
  press and the release of one key only, 30 ms apart. One test is not
  sufficient. (Measured: 1 time)

Knobs:

- One click of a knob sends one report. A turn has no release report.
  (Measured)
- A knob push sends one report with the state `0x01`. The device sends no
  report for the release. (Measured: 4 knobs)
- During a fast turn in the two directions, the device sent 398 reports in
  9.5 seconds. The largest rate was 59 reports in 1 second and 21 reports in
  200 ms. The smallest time between two reports was less than 1 ms.
  (Measured)

Touch bar:

- Each touch sends this sequence: one "finger down" report, then touch reports
  while the finger is down, then one "finger up" report. (Measured)
- The "finger down" report and the tap report contain the x coordinate of the
  start of the touch. (Measured)
- The touch coordinates that were seen: x from 8 to 799, y from 6 to 109.
  (Measured) Thus the touch area is possibly 800 × 112. (Inference) This is
  wider than the 4 bar panels together (704 pixels).
- A tap gives the code of the quarter of the touch bar where it starts:
  x = 101 gave `0x40`, 290 gave `0x41`, 510 gave `0x42`, 679 gave `0x43`.
  (Measured) The limits between the quarters are not measured.
- A tap is not one event: it is the sequence above. A finger that stays for
  3 seconds and does not move gives the same tap code when it lifts.
  (Measured)
- Swipe is reliable on this firmware: 20 swipes to the left gave 20 times
  `0x39`, and 20 swipes to the right gave 20 times `0x38`. (Measured)
- `0xB2` came after a move from the top edge to the bottom edge, and after one
  quick move to the left. Its meaning is not known.
- `0x40` to `0x43` are not keys adjacent to the knobs. The first plan was
  incorrect.

## Device settings

The SDK shows these settings. None is tested. (Source)

- Haptic feedback on or off.
- Knob light follows the key light.
- Boot video.
- Reset on USB report.
- Key light on disconnection.

## Not known

Measure these in Phase 1.

- The accurate position of each key window and of the touch bar window on the
  800 × 480 display. The photograph gives approximate values only.
- Where each bar panel image shows in the touch bar window, and whether a bar
  panel image can be wider than 176 pixels. The background can put one image
  across the full touch bar. The vendor software has a "Touch Bar mode" that
  hides the knob icons. See [vsd-craft.md](vsd-craft.md).
- The meaning of the input code `0xB2`.
- The x coordinates at which the tap code changes from one bar panel to the
  next.
- What the device sends when 2 or more keys are pressed together. One test
  showed one key only.
- Whether the knob lights obey the colour command. The probe sends it, but the
  result is not recorded.
- Whether the device keeps its images when the computer sleeps and wakes.
- The time from the cable connection until the device accepts images.

## Sources

- Mirabox StreamDock Device SDK, `StreamDockN4Pro.py`:
  <https://github.com/MiraboxSpace/StreamDock-Device-SDK/blob/main/Python-SDK/src/StreamDock/Devices/StreamDockN4Pro.py>
- Mirabox StreamDock Device SDK, `StreamDock.py` and `PILHelper.py`: the
  background and the boot logo. The other files are in
  [streamdock-protocol.md](streamdock-protocol.md#sources).
- `opendeck-akp05` README: <https://github.com/ambiso/opendeck-akp05>
- Mirabox N4 Pro product page: <https://mirabox.net/products/mirabox-stream-dock-n4-pro>
- VSDinside, N4 Pro and Stream Deck Plus comparison:
  <https://www.vsdinside.com/blogs/blog/n4-pro-vs-stream-deck-plus-knob-ring-lights-touch-bar-and-full-screen-dynamic-background>
