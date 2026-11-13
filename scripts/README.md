# Scripts

This directory holds the scripts for work on the computer of a developer.

| Script | Job |
|---|---|
| `measure-footprint.sh` | Measures the memory and the processor use of all processes of one application |
| `sign-dev.sh` | Builds the GUI application as a bundle and signs it with a stable identity, so that macOS keeps the Accessibility permission |
| `release.sh` | Makes the release: the signed bundle, the disk image, the notarization, and the checksum. See [decision 0025](../docs/decisions/0025-release-form.md). |
| `notices.sh` | Makes and checks `THIRD-PARTY-NOTICES.txt`, the licence notices of the dependencies. See [decision 0026](../docs/decisions/0026-licence-notices.md). |
| `lucide-icons.sh` | Makes the icon data of the engine from one Lucide release. See [decision 0033](../docs/decisions/0033-surface-icons.md). |
| `homebrew-cask.sh` | Writes the Homebrew cask file for one release. See [decision 0027](../docs/decisions/0027-homebrew-cask.md). |
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
| `OpenCraft-<version>-arm64.dmg.sha256` | The SHA-256 of the disk image, for the Homebrew cask. See [homebrew-cask.sh](#homebrew-casksh). |

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
the licence texts of the crates that the release executable contains. At the
end, it adds the works that are not crates: the Lucide icons of the engine,
with `crates/engine/icons/LICENSE`. The
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

## lucide-icons.sh

The script makes the icon data of the engine: `crates/engine/icons/lucide.tsv`
and the licence file `crates/engine/icons/LICENSE`. The decision is
[decision 0033](../docs/decisions/0033-surface-icons.md). The facts are in
[kb/ui-research.md](../docs/kb/ui-research.md#the-lucide-icons-for-surfaces-2026-11-13).

It needs `jq`, `tar`, and `awk`, and `curl` when it downloads the archive.

```sh
scripts/lucide-icons.sh                 # download the archive from GitHub
scripts/lucide-icons.sh lucide.tar.gz   # use an archive that you downloaded
```

The script stops if the SHA-256 of the archive is not the value in the
script. Run it only to change the Lucide version. Then:

1. Change `VERSION` and `SHA256` in the script. Use the version of the icons
   of the GUI toolkit (`lucide.json` of `gpui-kit-assets`).
2. Run the script and `scripts/notices.sh`.
3. Run the tests. The tests of `crates/engine/src/icons.rs` give the number
   of icons and categories, and check the kind icons.
4. Commit the two files, `THIRD-PARTY-NOTICES.txt`, and the changes of the
   tests.

## homebrew-cask.sh

The script writes the Homebrew cask file `opencraft.rb` for one version. It
holds the only copy of the cask text. The decision is
[decision 0027](../docs/decisions/0027-homebrew-cask.md). The facts are in
[kb/homebrew.md](../docs/kb/homebrew.md).

```sh
scripts/homebrew-cask.sh ../homebrew-opencraft/Casks/opencraft.rb
```

With no other argument, the script reads the version and the SHA-256 from the
one file `target/release-bundle/*.dmg.sha256` of `release.sh`. It stops if
the disk image does not have that SHA-256, or if the disk image has no
stapled notarization ticket. The options are:

| Option | Meaning |
|---|---|
| `--version VERSION --sha256 SHA256` | Use these values and do not read `target/release-bundle` |
| `--allow-unnotarized` | Do not check the notarization ticket. Use it only for a test. |

### Publish the Homebrew cask

The owner does these steps. The tap is public only after this repository is
public. The disk image must be the notarized one: the script stops
otherwise.

For the first release:

1. Make this repository public.
2. Make the release. `OPENCRAFT_NOTARY_PROFILE=<profile> scripts/release.sh`
   prints the SHA-256 and writes the disk image.
3. Make the GitHub release with the tag `v<version>`, where `<version>` is
   the version in `target/release-bundle` (for example `0.1.0`). Attach the
   disk image and the `.sha256` file:

   ```sh
   gh release create v<version> target/release-bundle/OpenCraft-<version>-arm64.dmg target/release-bundle/OpenCraft-<version>-arm64.dmg.sha256 --title "OpenCraft <version>"
   ```

4. Download the disk image from the release. Its SHA-256 must be the same
   as the content of the `.sha256` file:

   ```sh
   curl -sL -o /tmp/opencraft-check.dmg https://github.com/shantanugoel/OpenCraft/releases/download/v<version>/OpenCraft-<version>-arm64.dmg
   shasum -a 256 /tmp/opencraft-check.dmg
   cat target/release-bundle/OpenCraft-<version>-arm64.dmg.sha256
   rm /tmp/opencraft-check.dmg
   ```

5. Make the tap repository and a checkout of it:

   ```sh
   cd ..
   gh repo create shantanugoel/homebrew-opencraft --public --description "Homebrew tap for OpenCraft" --clone
   cd OpenCraft
   ```

   The tap is then next to this repository, in `../homebrew-opencraft`.

6. Copy `scripts/homebrew-tap/README.md` of this repository to
   `../homebrew-opencraft/README.md`. After this step, the copy in the tap
   is the only one.
7. Write the cask into the tap, from the root of this repository:

   ```sh
   mkdir -p ../homebrew-opencraft/Casks
   scripts/homebrew-cask.sh ../homebrew-opencraft/Casks/opencraft.rb
   ```

8. Commit and push in the tap:

   ```sh
   cd ../homebrew-opencraft
   git add README.md Casks/opencraft.rb
   git commit -m "Add the cask opencraft <version>"
   git push -u origin HEAD
   ```

9. Test as a user. Quit VSD Craft and OpenCraft first.

   ```sh
   brew install --cask shantanugoel/opencraft/opencraft
   brew audit --cask --online --strict shantanugoel/opencraft/opencraft
   brew style --cask shantanugoel/opencraft/opencraft
   spctl --assess --type exec --verbose /Applications/OpenCraft.app
   ```

   Do not use `brew audit --new`. It checks the numbers of stars for
   `homebrew/cask` ([kb/homebrew.md](../docs/kb/homebrew.md#which-repository-holds-the-cask)),
   which a tap does not need. `spctl` must say `accepted` and
   `source=Notarized Developer ID`. Then start OpenCraft from the
   Applications folder.

For a later release, do steps 2, 3, 4, and 7 in a checkout of the tap that
is up to date, then step 8 with the message "Update the cask opencraft to
<version>", and then step 9. The tap needs no other change.

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
