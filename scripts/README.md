# Scripts

This directory holds the scripts for work on the computer of a developer.

| Script | Job |
|---|---|
| `measure-footprint.sh` | Measures the memory and the processor use of all processes of one application |
| `sign-dev.sh` | Builds the GUI application as a bundle and signs it with a stable identity, so that macOS keeps the Accessibility permission |
| `release.sh` | Makes the release: the signed bundle, the disk image, the notarization, and the checksum. See [decision 0025](../docs/decisions/0025-release-form.md). |
| `notices.sh` | Makes and checks `THIRD-PARTY-NOTICES.txt`, the licence notices of the dependencies. See [decision 0026](../docs/decisions/0026-licence-notices.md). |
| `make-icon.swift` | Draws the application icon `assets/macos/OpenCraft.icns` |
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
open and with the window closed. From Phase 5, also measure a start with
`--login`, which opens no window. See
[kb/footprint.md](../docs/kb/footprint.md#phase-5). When OpenCraft is a bundle
(Phase 6), use the name of the application, as for VSD Craft.

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
then it has the permissions of the terminal. A start with `open` or from the
Finder has no terminal, thus the messages go to the log file. See
[decision 0024](../docs/decisions/0024-log-file-with-no-terminal.md).

```sh
scripts/sign-dev.sh --run            # debug build, sign, start
scripts/sign-dev.sh --release --run  # release build, sign, start
```

The script uses the first code signing identity of the keychain. To use a
different identity, set `OPENCRAFT_SIGN_IDENTITY` to its name or to its SHA-1
hash. `security find-identity -v -p codesigning` shows the identities. Do not
write the name of an identity in the repository.

The development bundle has the identifier `opencraft-dev`. The release of
`release.sh` has a different identifier. Thus each one has its own
permissions.

## release.sh

The script makes the release on the computer of a developer with Apple
silicon. The form of the release is in
[decision 0025](../docs/decisions/0025-release-form.md). The facts are in
[kb/macos-distribution.md](../docs/kb/macos-distribution.md). The output is
in `target/release-bundle/`:

| File | Content |
|---|---|
| `OpenCraft.app` | The bundle with the Developer ID signature |
| `OpenCraft-<version>-arm64.dmg` | The disk image to publish |
| `OpenCraft-<version>-arm64.dmg.sha256` | The SHA-256 of the disk image, for the Homebrew cask |

The version is the workspace version in `Cargo.toml`.

### Before the first release

1. Put a "Developer ID Application" identity in the login keychain. The
   script uses the first one. To use a different identity, set
   `OPENCRAFT_SIGN_IDENTITY` to its name or to its SHA-1 hash.
2. Put the notary credentials in the keychain one time. Use an
   app-specific password of the Apple ID. `notarytool` asks for it:

   ```sh
   xcrun notarytool store-credentials <profile> --apple-id <Apple ID> --team-id <team ID>
   ```

Do not write the name of the identity, the team ID, the Apple ID, or the
profile in the repository.

### Make a release

```sh
OPENCRAFT_NOTARY_PROFILE=<profile> scripts/release.sh
```

The script builds the release profile, makes and signs the bundle, makes
and signs the disk image, notarizes and staples the bundle and the disk
image, and prints the SHA-256 of the disk image. The two notarizations can
take some minutes each.

With no `OPENCRAFT_NOTARY_PROFILE`, the script stops after the signed disk
image and says that it is not notarized. Do not publish that disk image. To
notarize it later with no new build:

```sh
OPENCRAFT_NOTARY_PROFILE=<profile> scripts/release.sh --notarize-only
```

### Check the result

The script checks each item and stops at the first error. It runs these
checks:

- `codesign --verify --deep --strict --verbose=2` on the bundle, and
  `codesign -d --entitlements -`, which shows no entitlements.
- The identifier, the hardened runtime, the secure timestamp, the Developer
  ID authority, and the version of the bundle.
- `codesign --verify --strict` on the disk image.
- After notarization: `xcrun stapler validate`, and `spctl --assess` on the
  bundle and on the disk image. Each must say `accepted` and
  `source=Notarized Developer ID`.

### The licence notices

Before the build, the script compares the key in `THIRD-PARTY-NOTICES.txt`
with the key of the crates of `Cargo.lock` (`scripts/notices.sh --key`). It
stops if the file is missing or the keys are not the same. Then run
`scripts/notices.sh` and commit the file. The script copies the file into
`OpenCraft.app/Contents/Resources/`, and stops if the bundle has no file.

## notices.sh

The script makes `THIRD-PARTY-NOTICES.txt` in the root of the repository:
the licence texts of the crates that the release executable contains. The
decision is [decision 0026](../docs/decisions/0026-licence-notices.md). The
facts are in [kb/dependency-licences.md](../docs/kb/dependency-licences.md).

It needs `cargo-about` and `jq`. Install `cargo-about` one time, and fetch
the crates of all platforms one time:

```sh
cargo install cargo-about --locked --features cli
cargo fetch --locked
```

```sh
scripts/notices.sh           # write THIRD-PARTY-NOTICES.txt
scripts/notices.sh --check   # compare; CI uses this
scripts/notices.sh --key     # print the key; release.sh uses this
```

Run the script and commit the file when `Cargo.lock` or `about.toml`
changes. CI fails if the file is not correct.

If `cargo-about` stops with a checksum error for a clarification, a crate
changed its licence file. Read the new file, and change the entry in
`about.toml`. If `cargo-about` or `cargo deny` stops for a licence that is
not accepted, read the duties of that licence first. Write them in
[kb/dependency-licences.md](../docs/kb/dependency-licences.md), and add the
licence to `deny.toml` and to `about.toml` only if the owner accepts it.

## make-icon.swift

The script draws the application icon at each size and writes
`assets/macos/OpenCraft.icns` with `iconutil`. It needs only the Xcode
command line tools.

```sh
swift scripts/make-icon.swift assets/macos/OpenCraft.icns
```

A second argument names a directory for PNG copies at 1024, 256, 64, 32,
and 16 pixels, to look at the icon. To change the icon, change the script,
run it, and commit the script and the `.icns` file together.
