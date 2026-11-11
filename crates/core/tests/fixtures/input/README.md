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

The script has 4 passes. Each pass is one recording. The script shows a short
list of gestures for the pass. Do the gestures in the order of the list, then
press Enter. The order tells which report comes from which control.

To do one pass again, give its number: `scripts/record-input.sh 2`.

You can also record one gesture by hand:

```sh
opencraft-probe raw --draw --save key.txt --note "one key: press and release" --until-enter
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

| File | Gestures, in this order |
|---|---|
| `session-1-keys.txt` | Each key one time: top row from left to right, then second row from left to right. Then 2 adjacent keys together. Then 1 key held for 3 seconds. |
| `session-2-knobs.txt` | For each knob from left to right: turn left 5 clicks, turn right 5 clicks, push. Then the left knob turned as fast as possible for 5 seconds. |
| `session-3-touch-bar.txt` | 4 taps on the touch bar from left to right. A slow slide from the left end to the right end. A slow slide from the top edge to the bottom edge. A finger held for 2 seconds at the left end. |
| `session-4-swipes.txt` | 20 swipes left, then 20 swipes right. |
