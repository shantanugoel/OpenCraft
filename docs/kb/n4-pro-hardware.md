# VSD N4 Pro hardware

Last update: 2026-11-12

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
| Each knob has a ring light with a colour that software can set. See [Knob lights](#knob-lights). | Measured | Camera, 2026-11-11 |
| The 10 keys are in 2 rows of 5 | Measured | Owner, 2026-11-11 |
| Top row, from left to right: key slots 11, 12, 13, 14, 15 | Measured | Owner, 2026-11-11: `opencraft-probe test` shows the slot numbers |
| Second row, from left to right: key slots 6, 7, 8, 9, 10 | Measured | Owner, 2026-11-11 |
| Touch bar, from left to right: bar panels 1, 2, 3, 4 | Measured | Owner, 2026-11-11 |

### Feedback surface (2026-11-12)

| Fact | Confidence | Source |
|---|---|---|
| Knob N is below bar panel N. The descriptor gives bar panel N as the feedback surface of knob N. | Measured | Camera, 2026-11-12: bar panel 1 was green and the light of knob 1 was green, and they were the left panel and the left knob. The light of knob 4 was blue, and it was the right knob. |

## Surfaces

| Surface | Slots | Size | Format | Rotation | Confidence |
|---|---|---|---|---|---|
| Key face | 6 to 15 | 112 × 112 | JPEG | 180° | Measured: the probe paints them |
| Bar panel | 1 to 4 | 176 × 112 | JPEG | 180° | Measured: the probe paints them |
| Background | None | 800 × 480 | JPEG | 180° | Measured: the probe paints it. Owner, 2026-11-11 |

The format is the format that OpenCraft sends. See
[decision 0007](../decisions/0007-jpeg-for-key-faces-and-bar-panels.md). The
device also shows PNG on key faces and bar panels. See
[streamdock-protocol.md](streamdock-protocol.md#image-transfer).

The device has one display of 800 × 480 pixels behind the keys and the touch
bar. Each key and the touch bar is a window on this display. (Measured: owner
and camera, 2026-11-11)

### Positions on the display

A camera and a test program measured these positions on 2026-11-11. The method
is in [How the camera measurements were made](#how-the-camera-measurements-were-made).
The position is the top left corner of the image, in pixels of the upright
800 × 480 display.

| Surface | Slots, from left to right | x of each image | y | Size | Confidence |
|---|---|---|---|---|---|
| Key face, top row | 11, 12, 13, 14, 15 | 6, 176, 346, 514, 682 | 8 | 112 × 112 | Measured |
| Key face, second row | 6, 7, 8, 9, 10 | 6, 176, 346, 514, 682 | 178 | 112 × 112 | Measured |
| Bar panel | 1, 2, 3, 4 | 0, 203, 406, 609 | 358 | 176 × 112 | Measured |

- The descriptor of the N4 Pro in `crates/core/src/device.rs` holds these
  positions.
- The positions are accurate to 1 pixel. A background line of 1 pixel showed
  when it was adjacent to an image, and it did not show when it was 1 pixel
  inside the image. (Measured)
- The camera saw the two vertical edges of 5 key faces and of 3 bar panels,
  and one vertical edge of the other surfaces. It saw the top edge of each
  surface, and the bottom edge of the bar panels only. Each edge that the
  camera saw agrees with the sizes 112 × 112 and 176 × 112. (Measured)
- The device draws an image of the correct size at its real size. A line at a
  known position in the image showed at that position on the display.
  (Measured)
- A space of 27 pixels is between two bar panels. A space of 15 pixels is to
  the right of bar panel 4. The background shows in these spaces. (Measured)
- The horizontal distances from one key face to the next are 170, 170, 168,
  and 168 pixels. They are not all equal. The vertical distance between the
  two rows is 170 pixels. (Measured)
- Each window shows more of the display than its image. Thus the background
  shows around each image. (Measured)

The camera was in front of the device and below its centre. From that
position, each window showed these parts of the display. The values are
approximate, and they change with the position of the eye, because the
display is below the key caps. From that position, the camera did not see the
left edge of the key faces in columns 4 and 5, or the bottom edge of the key
faces. (Measured)

| Window | Part of the display that showed |
|---|---|
| Keys, column 1 to 5 | x from 0 to 116, 164 to 292, 336 to 468, 512 to 640, 690 to 798 |
| Keys, top row | y from 0 to 114 |
| Keys, second row | y from 166 to 290 |
| Touch bar | x from 0 to 798, y from 340 to 476 |

- The areas between these windows do not show. (Measured: owner, 2026-11-11)
- The touch bar window shows the full width of the display. 798 was the last
  position in the camera test. The touch coordinates go to 799. See
  [Controls and input codes](#controls-and-input-codes). (Measured)

### An image that is larger than its surface

The probe command `wide` sent these images on 2026-11-11, and the camera
recorded the result. No image caused an error or a restart of the device.

| Fact | Confidence |
|---|---|
| The device puts the bottom right corner of the image at the bottom right corner of the surface. It draws the image at its real size, to the left and up, across the spaces and across other surfaces. | Measured: slot 2 with 352 × 112, slot 4 with 582, 700, 784, and 785 × 112, slot 12 with 282 × 112, slot 8 with 112 × 282, slot 1 with 176 × 224 |
| An image of 785 × 112 pixels in slot 4 fills the touch bar from x = 0 to x = 784 | Measured: PNG and JPEG |
| If the image does not fit to the left, the device makes it smaller. Slot 1 showed images of 352 × 112 and of 800 × 112 pixels in the width of the bar panel. Slot 11 showed 224 × 112 pixels in the width of the key face. Slot 4 showed 800 × 112 pixels (JPEG) in approximately 785 pixels. | Measured |
| The image stays until a different image or a clear command replaces it. The clear command for all slots removes it. | Measured |

A wide image did not show when it was the first draw after
`opencraft-probe clear` on a new connection. The size of the image was not
the cause. The cause was the commit before the start command. See
[streamdock-protocol.md](streamdock-protocol.md#start-sequence).

### Transparency

- The device draws a PNG that has transparency on top of the background. The
  background shows through the transparent pixels. When the background
  changes, the new background shows through. (Measured: camera, 2026-11-11)
- The SDK sends a PNG only for an image that has transparency. It sends a
  JPEG for all other key images. (Source: SDK, `PILHelper.py`,
  `encode_key_image`)

### Other facts

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
- The device does not report 2 keys at the same time. (Measured: owner and
  probe, 2026-11-11)
  - When 2 adjacent keys go down together, the device sends the press and the
    release of one key, 0 to 31 ms apart. It sends nothing for the other key,
    and nothing when the keys go up. (Measured: 7 times)
  - When one key is down and a second key goes down, the device sends the
    release of the first key. When the second key goes up, the device sends
    the press of the first key again. It sends no report for the second key.
    (Measured: 1 time)
  - Thus a key combination is not possible, and a press report can come from
    a key that was down before.
- The device vibrates when it sends a key press. In the test above, it
  vibrated at the first press and when the second key went up. (Measured:
  owner, 2026-11-11)

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
- A tap gives the code of the part of the touch bar where it starts. 26
  taps from the left end to the right end gave: `0x40` for x from 30 to 171,
  `0x41` for x from 225 to 366, `0x42` for x from 407 to 576, and `0x43` for
  x from 613 to 762. (Measured: owner, 2026-11-11)
- Thus the code changes between x = 171 and 225, between 366 and 407, and
  between 576 and 613. These ranges contain the quarters of the width (200,
  400, 600) and the spaces between the bar panel images. The test cannot
  tell which of the two is the rule. (Inference)
- A tap is not one event: it is the sequence above. A finger that stays for
  3 seconds and does not move gives the same tap code when it lifts.
  (Measured)
- Swipe is reliable on this firmware: 20 swipes to the left gave 20 times
  `0x39`, and 20 swipes to the right gave 20 times `0x38`. (Measured)
- `0xB2` came after a move from the top edge to the bottom edge, and after one
  quick move to the left. Its meaning is not known.
- `0x40` to `0x43` are not keys adjacent to the knobs. The first plan was
  incorrect.

## Knob lights

A camera recorded these facts on 2026-11-11. The commands are in
[streamdock-protocol.md](streamdock-protocol.md#commands).

| Fact | Confidence |
|---|---|
| The colour command sets one colour for each knob. The first colour is for knob 1, the left knob. | Measured: red, green, blue, white, and a second set of 4 colours |
| A knob with the colour black is off, if a different knob has a colour | Measured |
| If all 4 colours are black, the lights show the default effect | Measured |
| The default effect is one colour for all knobs. The colour changes slowly through the colours of the rainbow. It starts with the start command. | Measured |
| The knob light brightness command changes the brightness. 100 is the brightest. 0 is off. | Measured: 100, 50, 10, 0 |
| The lights are off when no host operates the device, and after the sleep command | Measured |
| The command `DELED` ("reset the LED colour" in the SDK) changed nothing that the camera saw, after the lights were white | Measured: 1 time |

## The device with no host

| Fact | Confidence | Source |
|---|---|---|
| After the cable is connected, and after the device stops the USB connection, the device shows a boot animation and then a logo on an animated background. The knob lights are off. | Measured | Camera, 2026-11-11 |
| The boot animation starts less than 0.5 seconds after the cable is connected, and the logo shows approximately 1.5 seconds later | Measured: 2 times | Camera, 2026-11-11 |
| The device is on the bus, and the probe draws on it, 6.4 to 6.8 seconds after the cable is connected | Measured: 2 times | Camera and probe, 2026-11-11 |
| The device continues to show the logo. After 10 minutes it was the same. | Measured | Camera, 2026-11-11 |
| The device shows a slot image that arrives in this state, on top of the animation | Measured | Camera, 2026-11-11 |

What the device does when the host stops is in
[streamdock-protocol.md](streamdock-protocol.md#keep-alive-and-stop).

## How the camera measurements were made

On 2026-11-11, a USB camera recorded the device while a program sent
commands. The camera was in front of the device. No person looked at the
device for these facts.

- The probe sent the commands that the probe has. A test program sent the
  other commands. The test program uses the command builders of the device
  layer, and it is not in the repository.
- Position of an image: the test program drew a green line of 3 pixels on a
  black background, at each second position across the display. Then it drew
  the same line at 3 known positions in each slot image. The line of an image
  and the line of the background are at the same place in the camera picture
  when they are at the same place on the display.
- Edge of an image: each slot had a black image. The test program drew a
  background line of 1 pixel at each position near each edge. The line shows
  only where no slot image covers it.
- A picture from the camera shows colour and position well. It does not show
  small text well. Thus the owner looks at the device for the quality of
  small text.

## Device settings

The SDK shows these settings. None is tested. (Source)

- Haptic feedback on or off.
- Knob light follows the key light.
- Boot video.
- Reset on USB report.
- Key light on disconnection.

## Not known

Phase 1 did not answer these questions. Each question has a reason.

| Question | Reason that it stays |
|---|---|
| The exact part of the display that each window shows to the eye of the user | It changes with the position of the eye. The camera gives approximate values for one position. The positions of the images are exact, and the GUI needs only those. |
| The meaning of the input code `0xB2` | The decoder gives it as the end of a touch with no gesture, and no action uses it. Phase 4 adds the touch bar gestures. Examine the code then, if a gesture needs it. |
| The exact x coordinates at which the tap code changes from one bar panel to the next | The ranges are known: see "Controls and input codes". Each measured tap gave the code of the bar panel below the finger. Phase 4 adds the tap gesture. Measure the exact values then, if the design needs them. |
| What the device sends for other sequences of 2 or more keys: 3 keys, keys that are not adjacent, a key and a knob | The measured cases show that the device does not report 2 keys at the same time. Thus OpenCraft has no key combination, and a binding uses one key. The bindings of Phase 3 did not need more cases. |
| Whether the descriptors for `0x5548:0x1008` and `0x5548:0x1021` are correct | The owner does not have these models. More device models are not in a phase. |
| What the command `DELED` does, and what each device setting in "Device settings" does | No surface or control needs them. Phase 5 has the device settings in its scope: brightness, haptic feedback, and knob lights. |
| Whether the device keeps its images when the computer sleeps and wakes | The owner deferred this check on 2026-11-11. It is issue [#43](https://github.com/shantanugoel/OpenCraft/issues/43), in Phase 5. Rule 10 of the architecture makes the engine draw all surfaces again after a wake, so no earlier phase depends on the answer. |

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
