# Add a new device model

This document gives the procedure to add a StreamDock-class device model to
OpenCraft. A new model is new data in the descriptor table (architecture
[rule 4](ARCHITECTURE.md#device-layer)). The engine and the shell do not
change.

Use this procedure if you have a device that OpenCraft does not know. The
example in each step is the VSD N4 Pro. Its facts are in
[kb/n4-pro-hardware.md](kb/n4-pro-hardware.md). The protocol facts are in
[kb/streamdock-protocol.md](kb/streamdock-protocol.md).

## What is proven, and what is not

- **Proven by tests.** The tests add a second model through the descriptor
  data only: the test model in `crates/core/tests/support/test_model.rs`. It
  is not a real device, and it is not in the table of the real models. It has
  6 keys, 1 knob, 1 knob light, key faces of 96 × 96 as PNG that are not
  turned, other slots, other input codes, and no bar panel, touch bar, or
  background. With no other code change:
  - the device layer finds the model in a table, selects only its vendor
    interface, decodes its input with its own codes, and sends images to its
    own slots (`crates/core/tests/test_model.rs`);
  - the rules of `crates/core/tests/descriptor.rs` are true for it;
  - the engine makes a profile and device settings for the model, draws all
    its surfaces, sends only the setting commands that the model has, and
    dispatches its bindings (`crates/engine/tests/engine/test_model.rs`);
  - the window shows each of its controls from the layout message, and no
    other control (`crates/app/src/view/tests/test_model.rs`).
- **Not proven.** No second real device was ever connected to OpenCraft. The
  descriptors for `0x5548:0x1008` and `0x5548:0x1021` are not tested on
  hardware. A real device can need a protocol detail that the N4 Pro does not
  need. See [Limits](#limits).

## Before you start

- Read [CONTRIBUTING.md](CONTRIBUTING.md), [ARCHITECTURE.md](ARCHITECTURE.md),
  and [kb/README.md](kb/README.md).
- Make an issue for the model, with the label `task` and `needs-hardware`.
  Write in it what the device is and where you bought it.
- Build the probe: `cargo build --release -p opencraft-cli`. The executable
  is `target/release/opencraft-probe`.

## Safety rules

Obey these rules in each step. Each rule prevents damage to the device or to
your data.

1. **Quit the vendor software** (for example VSD Craft) before each probe
   command. Two applications must not operate one device at the same time.
   See [rule 1](ARCHITECTURE.md#ownership).
2. **Do not send the boot logo command** (`LOG`). The device writes the boot
   logo to permanent memory. The device layer has no builder for it, and a
   test fails if `protocol.rs` or `connection.rs` contains its code. Do not
   add one. See
   [kb/streamdock-protocol.md](kb/streamdock-protocol.md#background-image).
3. **Do not open the keyboard interface.** OpenCraft opens only the vendor
   interface of a known device ([rule 6](ARCHITECTURE.md#device-layer)). If
   the vendor interface of your device has a usage page that is not in
   `VENDOR_USAGE_PAGES` in `crates/core/src/device.rs`, add the page to that
   list. Do not add the usage page of a keyboard (`0x0001`) or of consumer
   controls (`0x000C`).
4. **Send only the commands of the probe.** Do not send other bytes of
   `QUCMD` than the bytes in the code. Some of them can change the USB
   behaviour. See "Not known" in
   [kb/streamdock-protocol.md](kb/streamdock-protocol.md#not-known).
5. **Write no personal data** ([rule 10](CONTRIBUTING.md#rules-for-work-on-an-issue)).
   The output of `opencraft-probe identify` and of `ioreg` contains the serial
   number of the device. Do not copy it into an issue, a document, a
   recording, or a commit. A recording file of the probe never has the serial
   number.

## 1. Find the identity

1. Connect the device. Quit the vendor software.
2. Run `opencraft-probe list`. This command does not open a device.
   - Find the line of your device. Write down the `VID:PID`, the interface
     number, and the usage page of each interface.
   - `known vendor` in the column `STATE` means that the vendor ID is in
     `KNOWN_VIDS`. If your vendor ID is not in the list, add it.
   - One interface has a vendor usage page, for example `0xFFA0` on the N4
     Pro. That is the vendor interface. The other interface is usually the
     keyboard emulation.
3. Find the report sizes of the vendor interface: run
   `ioreg -r -c IOHIDDevice -l` and read `MaxInputReportSize` and
   `MaxOutputReportSize` of that interface. The N4 Pro has 512 in and 1024
   out (with the report ID, 1025).
   - The device layer knows only protocol version 3: 512 bytes in and 1024
     bytes out. If your device has other sizes, stop here and write the sizes
     in the issue. See [Limits](#limits).
4. Add a first descriptor to `DESCRIPTORS` in `crates/core/src/device.rs`:
   the name, the model identifier, the VID, the PID, `packet_size:
   protocol::PACKET_SIZE`, no surface, no control, and no setting
   (`lowest_brightness: 0`, `knob_lights: 0`, `haptic: false`). The probe
   opens only a device that is in the table.
5. Run `opencraft-probe identify`. It opens the vendor interface, and it reads
   the firmware version (feature report `0x01`). It sends no output report.
   Write down the firmware version.

## 2. Record the input of each control

The input report format is the protocol. Only the input codes are model
data. The format is in
[kb/streamdock-protocol.md](kb/streamdock-protocol.md#input-reports).

1. For each gesture of each control, record one file:

   ```sh
   opencraft-probe raw --save key-row1-col1.txt \
     --note "Key, top row, 1st from the left: press it 1 time" --until-enter
   ```

   Do the gesture, then push Enter. The time limit is 10 seconds. For a long
   gesture, add `--seconds 60`. If no report comes, add `--draw`: then the
   probe sends the start sequence and draws the slot numbers first.
2. Record each gesture that a control can have:
   - a key: one press, a key held for 3 seconds, and 2 keys together;
   - a knob: turn left 5 clicks, turn right 5 clicks, one push, and a fast
     turn;
   - a touch bar: a tap on each part, a slow move from the left end to the
     right end, a move from the top edge to the bottom edge, a held finger,
     and swipes to the left and to the right, 1 time and 20 times.
3. Read the code at byte 9 and the state at byte 10 of each report. Compare
   them with the codes of the N4 Pro in
   [kb/n4-pro-hardware.md](kb/n4-pro-hardware.md#controls-and-input-codes).
4. If a report does not agree with the protocol, write it in the issue. Do
   not change the decoder for one model.

`opencraft-probe record` gives one prompt for each gesture, but its list of
steps is for the N4 Pro only. See
[issue #157](https://github.com/shantanugoel/OpenCraft/issues/157). The
`raw` files have the same format as the files of `record`.

## 3. Measure the surfaces

A surface has a slot, a position, a size, an image format, and a rotation.
The probe commands draw only on the surfaces of the descriptor. Thus add a
guess to the descriptor first, then measure, then correct the guess.

1. Find a first guess for each surface in a source: the vendor SDK class for
   the model, `mirajazz`, or the product page. Write the source in the kb
   entry. See [kb/prior-art.md](kb/prior-art.md).
2. Add one surface for each slot of the guess, with the guessed size,
   `ImageFormat::Jpeg`, and the guessed rotation.
3. Use the probe commands. Each command answers one question:

   | Command | Question that it answers |
   |---|---|
   | `opencraft-probe test --keep` | Which slot is at which position. Each surface shows its slot number. Write the numbers of each row from left to right, as in `crates/core/tests/fixtures/input/layout.txt`. If a number is upside down, the rotation is wrong. |
   | `opencraft-probe draw <slot> <rrggbb>` and `clear [slot]` | Whether one slot shows an image, and where. The N4 Pro does not use slot 5. |
   | `opencraft-probe bars` | The edges, the gaps, and the rotation of each bar panel. The corner marks show how the device turns the image. |
   | `opencraft-probe wide <slot> --size <w>x<h>` | The real size of a surface. The N4 Pro puts the bottom right corner of a large image at the bottom right corner of the surface. See [kb/n4-pro-hardware.md](kb/n4-pro-hardware.md#an-image-that-is-larger-than-its-surface). |
   | `opencraft-probe background --with-surfaces` | The size of the background, and the approximate position of each surface on it. The test pattern has a line each 40 pixels and the coordinates as text. |
   | `opencraft-probe repaint --format png` and `--format jpeg` | Which image formats the device shows, and the time of a repaint. |
   | `opencraft-probe send brightness 0 5 10 15 20 --each 5` | The lowest value of the brightness command that changes the light. |
   | `opencraft-probe send knob-colors <rrggbb>...` | The number of knob lights and their order. The command sends one colour for each knob of the descriptor. |
   | `opencraft-probe send haptic off` and `send haptic on` | Whether the model has haptic feedback. |

4. The probe gives positions that are approximate. The N4 Pro positions are
   accurate to 1 pixel. They came from a camera and a test program that is
   not in the repository. The method is in
   [kb/n4-pro-hardware.md](kb/n4-pro-hardware.md#how-the-camera-measurements-were-made).
   If you cannot measure to 1 pixel, write the accuracy in the kb entry.
5. Find the largest slot image that the device shows. The N4 Pro does not
   show a slot image that is too large, and it gives no error. The limit is in
   [kb/streamdock-protocol.md](kb/streamdock-protocol.md#image-transfer).
   The test `hard_slot_images_stay_below_the_largest_size_that_the_device_showed`
   in `crates/core/tests/descriptor.rs` checks the images of the N4 Pro
   against that limit. Do the same check for the new model.

## 4. Write the descriptor

Write the final entry in `DESCRIPTORS` in `crates/core/src/device.rs`. Use the
N4 Pro entry as the example. Do not write a check for the model in other code.

| Field | Meaning | N4 Pro |
|---|---|---|
| `name` | The name for the user | `"VSD StreamDock N4 Pro"` |
| `model` | The stable model identifier. A profile and the device settings name the model with it. Small letters, digits, and `-`. Give two descriptors the same identifier only if their surfaces, controls, and settings are the same. See [decision 0018](decisions/0018-many-profiles.md). | `"n4-pro"` |
| `vid`, `pid` | The USB identity from step 1 | `0x5548`, `0x1023` |
| `packet_size` | The output report payload size. It is the protocol, not a measurement of the model. | `protocol::PACKET_SIZE` (1024) |
| `surfaces` | One `Surface` for each key face, bar panel, and background | 10 key faces, 4 bar panels, 1 background |
| `surfaces[].id` | The kind and the number. The numbers of each kind are 1, 2, 3, and so on, from left to right and from top to bottom. | key face 1 is the top left key |
| `surfaces[].slot` | The slot from step 3. A background has no slot. | key face 1 is slot 11 |
| `surfaces[].x`, `y` | The top left corner of the image on the upright display | key face 1: 6, 8 |
| `surfaces[].width`, `height` | The size of the image | key face: 112 × 112 |
| `surfaces[].format` | The format that OpenCraft sends. See [decision 0007](decisions/0007-jpeg-for-key-faces-and-bar-panels.md). | `ImageFormat::Jpeg` |
| `surfaces[].rotation` | The turn that the device needs | `Rotation::Deg180` |
| `controls` | One `Control` for each key, knob, bar panel, and touch bar | 10 keys, 4 knobs, 4 bar panels, 1 touch bar |
| `controls[].number` | The number among the controls of the same kind, from 1 | knob 1 is the left knob |
| `controls[].codes` | The input codes from step 2. The kind of the control comes from the codes. | knob 1: `0xA0`, `0xA1`, `0x37` |
| `controls[].position` | The row and the column among the controls of the same kind. Row 1 is the top row. | key 6: row 2, column 1 |
| `controls[].surface` | The surface of the control, if it has a display | key 1: key face 1 |
| `controls[].feedback_surface` | The surface that shows feedback for a control with no display. See [decision 0013](decisions/0013-action-feedback-on-a-surface.md). | knob 1: bar panel 1 |
| `settings.lowest_brightness` | The lowest brightness value that changes the light | `15` |
| `settings.knob_lights` | The number of knob lights, knob 1 first. 0: none. | `4` |
| `settings.haptic` | True if the model has haptic feedback | `true` |

Rules that the tests check for each descriptor:

- Each USB identity selects one descriptor.
- No two controls share an input code. No control uses `0xFF`, the
  acknowledgement.
- Each slot is used one time. Only the background has no slot.
- The surfaces are on the display and do not overlap.
- A control more to the right has a surface more to the right.
- A knob light belongs to a knob.

## 5. Add fixtures and tests

1. Put the recordings of step 2 in a new directory,
   `crates/core/tests/fixtures/<model identifier>/`, with a `README.md` and a
   `layout.txt`. Use `crates/core/tests/fixtures/input/` as the example. The
   file format is in its [README](../crates/core/tests/fixtures/input/README.md).
2. Add a decoder test for the model, as in
   `crates/core/tests/input_decoder.rs`. Decode each recording with the new
   descriptor, and compare the events with the gestures of the notes.
3. Add tests for the measured facts of the model to
   `crates/core/tests/descriptor.rs`, as for the N4 Pro: the counts, the
   sizes, the slots, the positions, and the settings. The rules for each
   descriptor run on the new entry with no change.
4. Extend `the_decoder_source_has_no_input_code_of_the_n4_pro` to the codes
   of the new model.
5. Do not change the expected values of the tests of the N4 Pro. The bytes
   that go to the N4 Pro must stay the same.

The engine tests and the window tests need no change. The tests with the test
model show that they operate a model that is new data.

## 6. Record the facts in the knowledge base

1. Make a new entry, `docs/kb/<model identifier>-hardware.md`, from the
   template in [kb/README.md](kb/README.md#template). Use the sections of
   [kb/n4-pro-hardware.md](kb/n4-pro-hardware.md): identity, physical layout,
   surfaces, positions on the display, controls and input codes, knob lights,
   device settings, not known, and sources.
2. Give each fact a confidence label and a source. A value from the vendor SDK
   that you did not see on the device is **Source**, not **Measured**.
3. Add the entry to the index in [kb/README.md](kb/README.md#index).
4. Write a protocol fact that is new or different in
   [kb/streamdock-protocol.md](kb/streamdock-protocol.md), not in the model
   entry.
5. Put each question that you could not answer in the "Not known" list.

## 7. What a profile for the new model needs

Nothing in the code. The profile and the device settings use the model
identifier of the descriptor.

- When a device of the new model connects, the engine selects the profiles
  of that model. If the model has no profile, the engine makes a new profile
  with one empty page. See [decision 0018](decisions/0018-many-profiles.md).
- A profile of the N4 Pro does not show on the new model, and a profile of
  the new model does not show on the N4 Pro. The profiles of the other model
  stay in the profiles directory.
- The device settings of the model are in their own table of `state.toml`.
  The engine sends only the setting commands that the model has. See
  [decision 0022](decisions/0022-device-settings-and-stop-state.md).
- A binding names a control by its kind and number, for example `key-6` or
  `knob-1`. A surface has the same form, for example `key-face-6`. See
  [decision 0009](decisions/0009-profile-file-format.md) and the module
  documentation of `crates/engine/src/store.rs`.

## 8. The pull request

1. Do the work on a branch. Write `Closes #N` with the number of your issue.
2. Run the four commands of [AGENTS.md](../AGENTS.md#commands).
3. Put in the pull request:
   - the output of `opencraft-probe test` on the new device, with no serial
     number ([rule 9](CONTRIBUTING.md#rules-for-work-on-an-issue));
   - a photo of the device with the slot numbers of `test --keep`, if you
     can;
   - the result of the GUI application with the new device: the window shows
     the layout, and a binding on a key operates.
4. Ask the owner to run `opencraft-probe test` on the N4 Pro, because the
   change is in `crates/core`.

## Limits

These limits are known. The test model does not have these needs, so the
tests do not find them. Each limit has an issue, or it says when to make one.

| Limit | Where | Issue |
|---|---|---|
| Before a device connects, the engine shows the layout and the profiles of the first descriptor, the N4 Pro. When the device connects, the engine changes to the correct model. | `crates/engine/src/runtime.rs` | [#156](https://github.com/shantanugoel/OpenCraft/issues/156) |
| `opencraft-probe record` has the steps of the N4 Pro only. Use `raw`. | `crates/cli/src/record.rs`, `scripts/record-input.sh` | [#157](https://github.com/shantanugoel/OpenCraft/issues/157) |
| The window shows no element for a control that has no surface, other than a knob and the touch bar. The window puts the knobs in one row below the display. | `crates/app/src/view.rs` | [#158](https://github.com/shantanugoel/OpenCraft/issues/158) |
| Only protocol version 3 (512 bytes in, 1024 bytes out) is known. A device with other report sizes possibly has other commands and another report format. | `crates/core/src/protocol.rs`, `events.rs` | Make an issue with the sizes of step 1 |
| A surface can turn by 0° or 180° only. | `Rotation` in `crates/core/src/device.rs` | Make an issue if a device needs a different turn |
| The JPEG quality (90 for a slot image, 60 for the background) and the largest slot image come from the N4 Pro. A larger surface can give a slot image that the device does not show. | `crates/core/src/image.rs`, [decision 0007](decisions/0007-jpeg-for-key-faces-and-bar-panels.md) | Make an issue if the check in step 3, item 5, fails |
| The size of the label text, and the frame of 6 pixels for a toggle in the state "on", are for surfaces of approximately 112 pixels | `crates/engine/src/label.rs`, `render.rs` | Make an issue if the text on a small surface is not clear |
| The engine draws the key faces and the bar panels. It does not draw the background. | `crates/engine/src/engine.rs` | Not for a model: it is the same for the N4 Pro |
| The window puts a touch point at the top of the bar panels plus the y coordinate of the touch. This agrees with the N4 Pro only. | `touch_position` in `crates/app/src/state.rs` | Make an issue if the touch mark is in the wrong place |

These parts are the protocol, not the model. They stay the same for each
model of protocol version 3: the command format, the start sequence, the
keep-alive interval of 10 seconds, the stop sequence (`DC`, then `HAN`), the
input report format, the acknowledgement code `0xFF`, the image header and the
parts of 1024 bytes, and the firmware version in feature report `0x01`. If a
measurement on your device does not agree, write it in the kb and make an
issue. Do not add a check for the model.
