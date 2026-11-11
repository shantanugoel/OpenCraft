# Scripts

This directory holds the scripts for work on the computer of a developer.

| Script | Job |
|---|---|
| `measure-footprint.sh` | Measures the memory and the processor use of all processes of one application |
| `sign-dev.sh` | Builds the GUI application as a bundle and signs it with a stable identity, so that macOS keeps the Accessibility permission |
| `record-input.sh` | Records the input reports of each control of the N4 Pro. See [the fixture README](../crates/core/tests/fixtures/input/README.md). |

## measure-footprint.sh

### Purpose

The script measures all processes of one application. It measures the main
process, the helper processes, and the plug-in processes. It finds the
processes again on each sample, because processes start and stop. The script
counts these processes:

- Each process whose executable is inside the application bundle.
- Each descendant of those processes. A plug-in can be a Node.js process with an
  executable outside the bundle.

For each process and each sample, the script records:

- The physical footprint, from `footprint`. It is the same number as "Memory"
  in Activity Monitor.
- The resident size, from `ps`. It is a second number. Record both.
- The processor use. The script divides the processor time that a process used
  between two samples by the real time between the samples. It does not use
  the `%cpu` column of `ps`, because that value is a decayed average. 100
  percent is one core.

The script gives a CSV file and a summary. The comment at the start of the
script has the details and the columns of the CSV file.

Limits:

- `footprint` fails for a process of a different user. The field stays empty,
  and the summary shows a warning.
- The processor time has a resolution of 10 ms. Use an interval of 5 seconds or
  more for a good value.
- The first sample has no processor value.

### Use the script for VSD Craft

Use the computer of the owner. Connect the device. Close all other heavy
applications, so the numbers are not disturbed. Write the CSV files to a
directory outside the repository. Do not commit them.

1. Record the version of VSD Craft. Quit VSD Craft.

   ```sh
   defaults read "/Applications/VSD Craft.app/Contents/Info" CFBundleShortVersionString
   defaults read "/Applications/VSD Craft.app/Contents/Info" CFBundleVersion
   ```

2. Measure the time from the start to a ready device. A script cannot see the
   device, so use a stopwatch. Start the stopwatch when you start the
   application. Stop it when the device shows its first page and responds to a
   key. The option `--startup` starts the application and prints the time
   until the process set and its footprint are stable. That time is only an
   aid. It is not the time to a ready device. Use the option as the way to
   start the application:

   ```sh
   scripts/measure-footprint.sh "VSD Craft" --startup
   ```

   Record both times. If you start VSD Craft in a different way, skip this
   command. A camera that records the device can replace the stopwatch: the
   time of the first picture that shows the first page gives the time.

3. Wait 5 minutes and touch no input. Then measure for 5 minutes with the
   window open. Do not touch the device, the keyboard, or the mouse.

   ```sh
   scripts/measure-footprint.sh "VSD Craft" --interval 5 --duration 300 --out vsd-craft-window-open.csv
   ```

4. Close the window of VSD Craft. Do not quit the application. Check that
   VSD Craft still runs (menu bar, or `pgrep -f "VSD Craft"`). If the
   application continues in the background, wait 1 minute and measure again
   for 5 minutes. If the application quits when you close the window, write
   that fact and skip the measurement.

   ```sh
   scripts/measure-footprint.sh "VSD Craft" --interval 5 --duration 300 --out vsd-craft-window-closed.csv
   ```

5. Write the version, the three results (start time, window open, window
   closed), and the summary of each run in the issue
   [#20](https://github.com/shantanugoel/OpenCraft/issues/20). An agent then
   moves the numbers to [kb/vsd-craft.md](../docs/kb/vsd-craft.md) with the
   label "Measured".

### Use the script for OpenCraft

From Phase 2, OpenCraft runs as a process and not as an application bundle.
Give the PID of the process. The script measures the process and its
descendants.

```sh
scripts/measure-footprint.sh --pid "$(pgrep -x <process name>)" --interval 5 --duration 300 --out opencraft.csv
```

Use the same steps and the same times as for VSD Craft. Measure with the window
open and with the window closed. When OpenCraft is a bundle (Phase 6), use the
name of the application, as for VSD Craft.

### Read the output

- The summary shows the number of processes, the total footprint, the total
  resident size, and the total processor use. It also shows the largest 5
  processes by footprint.
- The product target for memory is a footprint. Compare the "Total footprint"
  with the target in the [roadmap](../docs/ROADMAP.md#product-targets).
- In the CSV file, the rows with `TOTAL` in the column `pid` are the totals of
  one sample. All other rows are processes.

## sign-dev.sh

macOS connects the Accessibility permission to the code signature of a
program. The facts are in
[kb/macos-permissions.md](../docs/kb/macos-permissions.md#code-signature-of-a-development-build).
Use this script in place of `cargo build` when you want the permission to
stay after a new build. The script makes the bundle
`target/<profile>/OpenCraft.app`. Start the bundle with `--run`, with `open`,
or from the Finder. Do not start the program in the bundle from a terminal:
then it has the permissions of the terminal.

```sh
scripts/sign-dev.sh --run            # debug build, sign, start
scripts/sign-dev.sh --release --run  # release build, sign, start
```

The script uses the first code signing identity of the keychain. To use a
different identity, set `OPENCRAFT_SIGN_IDENTITY` to its name or to its SHA-1
hash. `security find-identity -v -p codesigning` shows the identities. Do not
write the name of an identity in the repository.
