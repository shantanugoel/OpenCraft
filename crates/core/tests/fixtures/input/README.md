# Recorded input reports

This directory holds recordings of the input reports of the VSD N4 Pro.
Tests in `crates/core` use the files to check the decoder in
`crates/core/src/events.rs`.

The recordings come from the procedure in GitHub issue #11.

## How to make the recordings

Connect the device. Quit VSD Craft. Run this command in the root of the
repository:

```sh
scripts/record-input.sh
```

The probe draws the panels, then it shows one line for each gesture. Do the
gesture on the device, then wait for the next line. The probe goes to the next
line when the device sends no more reports. Do not use the keyboard. Each
gesture goes to one file. A gesture that gives no report has no file.

The probe sends the keep-alive command each 10 seconds. Without it, the device
stops the connection. See
[the protocol entry](../../../../../docs/kb/streamdock-protocol.md#keep-alive-and-stop).

To continue after a stop, give the step number:
`scripts/record-input.sh --from 13`.

You can also record by hand, with no prompts:

```sh
opencraft-probe raw --draw --keep-alive 10 --save key.txt --note "one key: press and release" --until-enter
```

## File format

- The file is a text file. Each line ends with a line break.
- A line that starts with `#` is a comment.
- The first line is `# opencraft-input-recording v1`.
- The next comment lines give `note` (the gesture), `device` (the model name),
  and `firmware` (the firmware version), in the form `# key: value`.
  The file never has the serial number.
- Each other line is one report: the time in milliseconds from the start of the
  recording, one space, and all bytes of the report as hexadecimal with no
  separators. A report has up to 512 bytes.

Example (the bytes are not real):

```text
# opencraft-input-recording v1
# note: one key: press and release
# device: VSD StreamDock N4 Pro
# firmware: V4.VSD N4 Pro.02.010
# columns: time_ms report_hex
0 41434b...
85 41434b...
```

To read a file in a test, use `opencraft_core::recording::parse`.

## Other files

`layout.txt` holds the slot number at each physical position. The owner read
the numbers from the panels. The lines are `key_row_1`, `key_row_2`, and
`bar_panels`. The numbers go from left to right.

## Files and gestures

The file name is the step number and a short name, for example
`13-knob-1-left.txt`. The `note` line in each file gives the gesture. In a
file, the time of the first report is 0.

| Files | Gestures |
|---|---|
| `01` to `10` | Each key, 1 press: top row from left to right, then second row from left to right |
| `11`, `12` | 2 keys together. 1 key held for 3 seconds. |
| `13` to `24` | For each knob from left to right: turn left 5 clicks, turn right 5 clicks, push |
| `25` | The left knob turned as fast as possible |
| `26` to `29` | A tap on each quarter of the touch bar, from left to right |
| `30` to `32` | A slow move from the left end to the right end. A slow move from the top edge to the bottom edge. A finger held for 2 seconds. |
| `33` to `36` | 1 swipe left, 1 swipe right, 20 swipes left, 20 swipes right. A knob push ends the steps with 20 swipes. That push is not in the file. |
