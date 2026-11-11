# Recorded input reports

This directory holds recordings of the input reports of the VSD N4 Pro.
Each file holds the reports for one gesture. Tests in `crates/core` use the
files to check the decoder in `crates/core/src/events.rs`.

The recordings come from the procedure in GitHub issue #11. The files do not
exist until the owner of the device does the procedure.

## How to make the recordings

Connect the device. Quit VSD Craft. Run this command in the root of the
repository:

```sh
scripts/record-input.sh
```

The script builds the probe, shows the slot number on each panel, and asks for
the slot numbers. Then it gives 40 steps, one gesture for each step. The script
shows the instruction for each step. To continue after a stop, add
`--from <step number>`. To write the files in a different directory, give the
directory as the first argument. To print the table below, add `--list`.

You can also record one gesture by hand:

```sh
opencraft-probe raw --save 01-key-06.txt --note "key 6: press and release" --until-enter
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
# note: key 6: press and release
# device: VSD StreamDock N4 Pro
# firmware: V4.VSD N4 Pro.02.010
# columns: time_ms report_hex
0 41434b...
85 41434b...
```

To read a file in a test, use `opencraft_core::recording::parse`.

## Other files

`layout.txt` holds the slot number at each physical position. The owner types
the numbers when the script asks. The lines are `key_row_1`, `key_row_2`, and
`bar_panels`. The numbers go from left to right.

## Files and gestures

A step that the owner skips has no file.

| File | Gesture |
|---|---|
| `01-key-06.txt` | key 6: press and release |
| `02-key-07.txt` | key 7: press and release |
| `03-key-08.txt` | key 8: press and release |
| `04-key-09.txt` | key 9: press and release |
| `05-key-10.txt` | key 10: press and release |
| `06-key-11.txt` | key 11: press and release |
| `07-key-12.txt` | key 12: press and release |
| `08-key-13.txt` | key 13: press and release |
| `09-key-14.txt` | key 14: press and release |
| `10-key-15.txt` | key 15: press and release |
| `11-knob-1-left.txt` | knob 1 (first from the left): turn left about 5 clicks |
| `12-knob-1-right.txt` | knob 1 (first from the left): turn right about 5 clicks |
| `13-knob-1-push.txt` | knob 1 (first from the left): push and release |
| `14-knob-2-left.txt` | knob 2 (second from the left): turn left about 5 clicks |
| `15-knob-2-right.txt` | knob 2 (second from the left): turn right about 5 clicks |
| `16-knob-2-push.txt` | knob 2 (second from the left): push and release |
| `17-knob-3-left.txt` | knob 3 (third from the left): turn left about 5 clicks |
| `18-knob-3-right.txt` | knob 3 (third from the left): turn right about 5 clicks |
| `19-knob-3-push.txt` | knob 3 (third from the left): push and release |
| `20-knob-4-left.txt` | knob 4 (fourth from the left): turn left about 5 clicks |
| `21-knob-4-right.txt` | knob 4 (fourth from the left): turn right about 5 clicks |
| `22-knob-4-push.txt` | knob 4 (fourth from the left): push and release |
| `23-bar-tap-1.txt` | bar panel 1: tap |
| `24-bar-tap-2.txt` | bar panel 2: tap |
| `25-bar-tap-3.txt` | bar panel 3: tap |
| `26-bar-tap-4.txt` | bar panel 4: tap |
| `27-bar-hold-1.txt` | bar panel 1: press and hold for 2 seconds, then release |
| `28-swipe-left-1.txt` | touch bar: swipe left, 1 time |
| `29-swipe-right-1.txt` | touch bar: swipe right, 1 time |
| `30-swipe-left-20.txt` | touch bar: swipe left, 20 times |
| `31-swipe-right-20.txt` | touch bar: swipe right, 20 times |
| `32-slide-left-to-right.txt` | touch bar: one finger moves slowly from the left end to the right end |
| `33-slide-top-to-bottom.txt` | touch bar: one finger moves slowly from the top edge to the bottom edge |
| `34-corner-top-left.txt` | touch bar: touch at the top-left corner |
| `35-corner-top-right.txt` | touch bar: touch at the top-right corner |
| `36-corner-bottom-left.txt` | touch bar: touch at the bottom-left corner |
| `37-corner-bottom-right.txt` | touch bar: touch at the bottom-right corner |
| `38-knob-1-fast.txt` | knob 1 (first from the left): turn as fast as possible for 5 seconds |
| `39-keys-two.txt` | keys 7 and 8: pressed at the same time, then released |
| `40-key-hold.txt` | key 8: press and hold for 3 seconds, then release |

## Rules

- Do not put a device serial number in a file here.
- Do not change a recording by hand. Record the gesture again.
