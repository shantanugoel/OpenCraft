# 0025: The release is a signed and notarized bundle in one disk image for Apple silicon

- Status: Accepted
- Date: 2026-11-12

## Context

Phase 6 makes the first public release. A person who is not the owner must
get OpenCraft in a form that macOS starts with no warning. Before Phase 6,
the only bundle is the development bundle of `scripts/sign-dev.sh`. It has
the identifier `opencraft-dev` and a signature that only the computer of
the owner accepts.

The facts are in [kb/macos-distribution.md](../kb/macos-distribution.md).
The important ones are:

- A bundle with a Developer ID signature, the hardened runtime, and a
  secure timestamp passes `codesign --verify --deep --strict`. Gatekeeper
  rejects it until Apple notarizes it.
- The designated requirement of that signature has the identifier and the
  team, and no version. Thus the permissions stay after an update.
- In a bundle with the hardened runtime and no entitlements, the sound
  volume, the open application, the open URL, and the shell command
  actions operated. An Apple Event from a shell command to the Finder also
  operated.
- GPUI compiles its Metal shaders for macOS 12.0 and later.
- A ticket stapled to a disk image does not go with the application that
  the user copies out of it.

[Decision 0021](0021-start-at-login.md) said: look at `SMAppService.mainApp`
again in Phase 6.

## Decision

### The bundle

| Item | Value | Reason |
|---|---|---|
| Name | `OpenCraft.app` | |
| Executable | `Contents/MacOS/opencraft` | The name of the binary of `crates/app` |
| Bundle identifier | `com.shantanugoel.opencraft` | A stable identifier in reverse-DNS form |
| `CFBundleShortVersionString` and `CFBundleVersion` | The version of `opencraft-app`, that is the workspace version in `Cargo.toml` | One version in one place. The script stops if the version is not three numbers. |
| `LSMinimumSystemVersion` | `12.0` | The Metal shaders of GPUI. See [kb/macos-distribution.md](../kb/macos-distribution.md#the-lowest-macos-version). |
| `LSApplicationCategoryType` | `public.app-category.utilities` | |
| `NSHighResolutionCapable` | true | |
| `NSHumanReadableCopyright` | The copyright line of `LICENSE` | One copyright line in one place |
| `CFBundleIconFile` | `OpenCraft` (`Contents/Resources/OpenCraft.icns`) | |
| `LSUIElement` | Not set | [Decision 0020](0020-menu-bar-item-and-dock-icon.md) shows the Dock icon by default, and hides it at run time with the accessory activation policy. A start with the option "hide" uses a headless start. Thus the key is not necessary. |

- The icon is `assets/macos/OpenCraft.icns`. The script
  `scripts/make-icon.swift` draws it: a dark tile with 6 keys in a grid of 3
  by 2 and one knob. It has no logo and no trademark of a vendor. To change
  the icon, change the script, run it, and commit the script and the
  `.icns` file together.
- If the file `THIRD-PARTY-NOTICES.txt` is in the root of the repository,
  the script copies it to `Contents/Resources/`. Issue
  [#146](https://github.com/shantanugoel/OpenCraft/issues/146) makes that
  file.
- Note (2026-11-13), from the work of
  [#196](https://github.com/shantanugoel/OpenCraft/issues/196): the starter
  profiles of the first start
  ([decision 0029](0029-window-layout.md#18-the-first-start-196)) are
  profile files in `crates/app/assets/starters`. The executable contains
  them (`include_str!`), as it contains the icon of the "About" pane. Thus
  the script copies no more files into `Contents/Resources`, and `cargo
  run`, the development bundle, and the release have the same starter
  profiles.
- The development bundle keeps the identifier `opencraft-dev`. Thus a
  development build and a release do not share permissions, and a change in
  a development build cannot remove a permission of the release.

### The processor architecture

The release is for Apple silicon (`arm64`) only. The computer of the owner
has no Rosetta and cannot start or test an Intel build. The script stops if
the executable is not `arm64`. Look at this again when a person with an
Intel Mac can test a build, or when users ask for one.

### The signature

- The identity is a Developer ID Application identity. The script uses the
  value of `OPENCRAFT_SIGN_IDENTITY`, or else the first "Developer ID
  Application" identity of `security find-identity -v -p codesigning`.
- `codesign --options runtime --timestamp`: the hardened runtime and a
  secure timestamp. Notarization needs both.
- No entitlements. Add an entitlement only when a measurement shows a fault
  that the entitlement corrects. Write the fault and the entitlement in this
  decision.
- The repository holds no identity name, no team ID, and no profile name of
  the owner. The values come from the environment and the keychain.

### The download file

- One disk image: `OpenCraft-<version>-arm64.dmg`, made with `hdiutil`
  (UDZO, HFS+). It holds `OpenCraft.app` and a link `Applications` to
  `/Applications`. It has no background picture.
- The script signs the disk image with the same identity and a secure
  timestamp.
- The last step prints the SHA-256 of the disk image and writes it to
  `OpenCraft-<version>-arm64.dmg.sha256`. The Homebrew cask (issue
  [#147](https://github.com/shantanugoel/OpenCraft/issues/147)) uses it.
- The output is in `target/release-bundle/`.

### Notarization

The script notarizes when `OPENCRAFT_NOTARY_PROFILE` names a keychain profile
of `xcrun notarytool store-credentials`. The steps are:

1. Put the signed bundle in a zip file with `ditto -c -k --keepParent`.
   Submit it with `xcrun notarytool submit --keychain-profile <profile>
   --wait`. Staple the ticket to the bundle with `xcrun stapler staple`.
2. Make the disk image from the stapled bundle, and sign it.
3. Submit the disk image, and staple its ticket.
4. Check with `spctl --assess --type exec` for the bundle and with
   `spctl --assess --type open --context context:primary-signature` for the
   disk image.
5. Write the checksum. The stapler changes the file, thus this is the last
   step.

The bundle gets its own ticket, because a ticket on the disk image does not
go with the copy in `/Applications`. With its own ticket, the first start
needs no network. This costs two submissions.

`notarytool` has no documented exit status for a rejected submission. Thus
the script reads the field `status` of the JSON output. If it is not
"Accepted", the script prints the log of the submission and stops.

If `OPENCRAFT_NOTARY_PROFILE` is not set, the script makes the signed disk
image, says that it is not notarized, and gives the command for the next
stage. `scripts/release.sh --notarize-only` starts from the signed bundle of
an earlier run, with no new build, and does the steps above.

### Changes in the shell

- **The version.** The menu of the menu bar item has a first item
  "OpenCraft *version*", not enabled, with a separator after it. The value
  is `CARGO_PKG_VERSION`, thus it is the same version as in the bundle.
- **The start at login.** The option of
  [decision 0021](0021-start-at-login.md) is on only if the LaunchAgent file
  starts the executable of the process that reads it. After the user moves
  the application, the option shows as off, and one selection writes the new
  path. An upgrade puts the new bundle at the same path, thus the option
  stays on.
- **A read-only location.** If the executable is on a volume that is mounted
  read-only, the shell does not write the LaunchAgent file. It shows a
  problem that tells the user to move OpenCraft to the Applications folder.
  A mounted disk image and a quarantined application that macOS moved to a
  random location (App Translocation) are read-only. Such a path is not there
  at the next login.
- **Not SMAppService now.** The LaunchAgent file operates in the signed
  bundle (measured), and the two changes above correct the problems of a
  move. `SMAppService` needs a new binding crate, and a start at a real
  login is not measured for either mechanism (issue
  [#143](https://github.com/shantanugoel/OpenCraft/issues/143)). Look at
  `SMAppService.mainApp` again if #143 or a user report shows a fault of
  the LaunchAgent file.
- **No change.** The profiles directory, `shell.toml`, the lock file, and the
  log file are in `~/Library/Application Support/OpenCraft`. Their path does
  not use the bundle identifier or the path of the bundle. Thus a
  development bundle and a release share the profiles, and the lock of
  [decision 0019](0019-engine-in-the-gui-process.md) lets only one of them
  operate the device. The menu bar item and the Dock icon option do not use
  the identifier.

## Results

- The owner makes a release with one command. The notary credentials go
  into the keychain one time, with
  `xcrun notarytool store-credentials <profile> --apple-id <Apple ID>
  --team-id <team ID>`. `notarytool` asks for an app-specific password. Then
  `OPENCRAFT_NOTARY_PROFILE=<profile> scripts/release.sh --notarize-only`
  notarizes the last build.
- macOS asks for the Accessibility permission again for the identifier
  `com.shantanugoel.opencraft`. The owner gives it one time. The list in
  System Settings then has two items with the name "OpenCraft": the
  development bundle and the release.
- A Mac with an Intel processor, and a Mac with macOS 11 or earlier, cannot
  use OpenCraft.
- On macOS 27.2, `hdiutil create` prints a warning that it is deprecated. If
  a later macOS removes it, use `diskutil image create`.
- Not measured: a notarization, the first start of a notarized bundle, macOS
  12 to 26, and the actions that need the Accessibility permission in the
  release bundle. See the "Not known" list in
  [kb/macos-distribution.md](../kb/macos-distribution.md#not-known).
