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
| The 10 keys are in 2 rows of 5 | Inference | The key codes are in two groups of 5 |

## Surfaces

| Surface | Slots | Size | Format | Rotation | Confidence |
|---|---|---|---|---|---|
| Key face | 6 to 15 | 112 × 112 | PNG | 180° | Measured: the probe paints them |
| Bar panel | 1 to 4 | 176 × 112 | PNG | 180° | Measured: the probe paints them |
| Background | None | 800 × 480 | JPEG | 180° | Source: SDK. Not tested. |

- Slot 5 is not used. The SDK image map has no entry for it. (Source)
- The 4 bar panels are the display of the touch bar. The SDK calls them the
  "secondary screen". (Source)
- The device has no display other than these. The SDK has no other draw
  method. (Source)
- 800 × 480 is not the size of the touch bar. The SDK method
  `set_touchscreen_image` sets the boot logo and the background. The method
  name caused an error in the first plan. (Source)
- The code calls a bar panel a "knob display" (`PanelKind::Knob`). Phase 1
  changes this name.

## Controls and input codes

| Control | Gesture | Codes | Confidence |
|---|---|---|---|
| Knob 1 to 4 | Turn left / turn right | `0xA0/0xA1`, `0x50/0x51`, `0x90/0x91`, `0x70/0x71` | Measured |
| Knob 1 to 4 | Push | `0x37`, `0x35`, `0x33`, `0x36` | Measured |
| Key slots 11 to 15 | Press, release | `0x01` to `0x05` | Source: SDK |
| Key slots 6 to 10 | Press, release | `0x06` to `0x0A` | Source: SDK |
| Bar panel 1 to 4 | Tap | `0x40` to `0x43` | Source: SDK |
| Touch bar | Swipe left / swipe right | `0x38` / `0x39` | Source: SDK |
| Touch bar | Touch point | Packet `ACK` … `ARX`, x and y at bytes 10 to 13, big-endian | Source: SDK |

- The record of the bring-up names knob input only. The other codes agree with
  the SDK, but no record shows a test on our hardware.
- A tap on a bar panel sends one event only. It does not send a press and then
  a release. (Source: SDK)
- `0x40` to `0x43` are not keys adjacent to the knobs. The first plan was
  incorrect.
- The `opendeck-akp05` plug-in has swipe disabled. Its author found that swipe
  was not reliable. (Source)

## Device settings

The SDK shows these settings. None is tested. (Source)

- Haptic feedback on or off.
- Knob light follows the key light.
- Boot video.
- Reset on USB report.
- Key light on disconnection.

## Not known

Measure these in Phase 1.

- Which key slot is at which physical position.
- Where the 800 × 480 background shows: behind the keys only, or behind the
  touch bar also.
- Whether the touch bar can show one image that is wider than the 4 bar panels
  together (4 × 176 = 704 pixels). The vendor software has a "Touch Bar mode"
  that hides the knob icons. See [vsd-craft.md](vsd-craft.md).
- The range of the touch point coordinates, and when the device sends a touch
  point.
- Whether swipe is reliable on this firmware.
- Whether the knob lights obey the colour command. The probe sends it, but the
  result is not recorded.
- The largest rate of knob events during a fast turn.

## Sources

- Mirabox StreamDock Device SDK, `StreamDockN4Pro.py`:
  <https://github.com/MiraboxSpace/StreamDock-Device-SDK/blob/main/Python-SDK/src/StreamDock/Devices/StreamDockN4Pro.py>
- `opendeck-akp05` README: <https://github.com/ambiso/opendeck-akp05>
- Mirabox N4 Pro product page: <https://mirabox.net/products/mirabox-stream-dock-n4-pro>
- VSDinside, N4 Pro and Stream Deck Plus comparison:
  <https://www.vsdinside.com/blogs/blog/n4-pro-vs-stream-deck-plus-knob-ring-lights-touch-bar-and-full-screen-dynamic-background>
