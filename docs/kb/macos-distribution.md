# Distribution on macOS

Last update: 2026-11-12

The decision that uses these facts is
[decision 0025](../decisions/0025-release-form.md). The script is
`scripts/release.sh`.

## Test arrangement

Date: 2026-11-12. The computer and macOS are the same as in
[gpui.md](gpui.md#release-measurements): Apple silicon, macOS 27.2. The
computer has no Rosetta. The login keychain has one "Developer ID
Application" identity. For the first facts, the computer had no notary
credentials. Later on the same day, the owner gave an app-specific password,
and `notarytool store-credentials` put a profile in the login keychain. The
rows of the notarization that have the label "Measured" are from that run.

## The bundle and the signature

| Fact | Confidence | Source |
|---|---|---|
| `codesign --force --options runtime --timestamp` with the Developer ID Application identity gives a signature with `flags=0x10000(runtime)`, a `Timestamp=` line, and the authorities "Developer ID Application", "Developer ID Certification Authority", and "Apple Root CA". `codesign --verify --deep --strict --verbose=2` passes. | Measured | `codesign -d --verbose=4` |
| The bundle has no entitlements: `codesign -d --entitlements -` prints nothing. | Measured | `scripts/release.sh` |
| The designated requirement is `identifier "com.shantanugoel.opencraft" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] and certificate leaf[field.1.2.840.113635.100.6.1.13] and certificate leaf[subject.OU] = <team>`. It contains no code hash and no version. | Measured | `codesign -d -r-` |
| A new version with the same identifier and the same team has the same designated requirement. Thus macOS keeps the privacy permissions of OpenCraft after an update. | Inference | The row above, and [macos-permissions.md](macos-permissions.md#code-signature-of-a-development-build) |
| A disk image that `codesign --timestamp` signs has `Format=disk image` and a timestamp. `codesign --verify --strict` passes. | Measured | `codesign -d --verbose=2` |
| Before notarization, `spctl --assess --type exec` rejects the bundle, and `spctl --assess --type open --context context:primary-signature` rejects the disk image. Both give `source=Unnotarized Developer ID`. | Measured | `spctl` |
| The release bundle is 22 MB. The disk image (UDZO) is 12 MB. | Measured | `du -sh`, 2026-11-12 |
| `hdiutil verify` gives a valid checksum. The image has `OpenCraft.app` and a link `Applications` to `/Applications`. | Measured | `hdiutil attach` |
| The volume of the disk image has the name `OpenCraft <version>`, for example `OpenCraft 0.1.0`. It holds only `OpenCraft.app` and the link `Applications`. It has no `.DS_Store` file, thus the Finder shows its default view and no window layout. | Measured | `hdiutil attach -nobrowse` and `ls -la` of the disk image of `main`, 2026-11-12 (new-user test, issue [#152](https://github.com/shantanugoel/OpenCraft/issues/152)) |
| A UDZO disk image mounts read-only, also with no `-readonly` option. | Measured | `mount` |
| On macOS 27.2, `hdiutil create`, `hdiutil attach`, and `hdiutil detach` print a warning that they are deprecated, and give `diskutil image` or `diskutil eject` as the replacement. They still make, mount, and eject the image. | Measured | `hdiutil` |
| `icns` files from `iconutil` are the same, byte for byte, for two runs of `scripts/make-icon.swift`. | Measured | `cmp` |

## The lowest macOS version

| Fact | Confidence | Source |
|---|---|---|
| The executable from `cargo build --release` has `minos 11.0` and `sdk 27.0` in `LC_BUILD_VERSION`. This is the default of the Rust target `aarch64-apple-darwin`. | Measured | `otool -l` |
| GPUI compiles its Metal shaders with `-mmacosx-version-min=12.0`. The crate in `Cargo.lock` is `gpui-pre-apple` 0.3.12. | Source | `build.rs` of `gpui-pre-apple` 0.3.12 |
| The executable links `UniformTypeIdentifiers.framework`, which starts with macOS 11. | Measured, Source | `otool -L`; Apple documentation |
| Thus OpenCraft needs macOS 12.0 or later. `LSMinimumSystemVersion` is `12.0`. | Inference | The rows above |

## The hardened runtime and the actions

Date: 2026-11-12. Nobody can press a key of the device for a test. Thus a
scratch bundle, not in the repository, held the example `system` of
`crates/actions`. It calls the same functions as the built-in action
handlers. The scratch bundle had the same form of signature as the release:
Developer ID, hardened runtime, secure timestamp, and no entitlements. Its
identifier was different. `open` started it, thus it had its own permission
state, not the permissions of the terminal. Each result is from 1 try.

| Fact | Confidence | Source |
|---|---|---|
| `AXIsProcessTrusted` gave false: the bundle had no Accessibility permission. | Measured | Scratch bundle |
| The bundle read the sound volume and the mute, and set them, through Core Audio. | Measured | Scratch bundle |
| The bundle opened an application (Calculator) and a URL (a `file:` URL of a folder) through `/usr/bin/open`. | Measured | Scratch bundle; process list; window list |
| The bundle started a shell command with `/bin/sh -c`. The command wrote its file. | Measured | Scratch bundle |
| A shell command `osascript -e 'tell application "Finder" to get name of startup disk'` from the bundle exited with 0. No permission dialog showed. The bundle has no entitlement `com.apple.security.automation.apple-events`. | Measured | Scratch bundle |
| AppleScript answers the property `name` of an application with no Apple Event. Use a property of an object in the application for a test of Apple Events. | Inference | `get name` operated in each case; `name of startup disk` is a real event |
| The hardened runtime has entitlements for runtime exceptions (for example JIT code and library validation) and for resource access (audio input, camera, location, contacts, calendars, photos, and Apple Events). It has no entitlement for key events or for the Accessibility permission. | Source | [Hardened Runtime] |
| Thus the hotkey, type text, and media key actions need only the Accessibility permission, as in a development build. | Inference | The row above, and [macos-permissions.md](macos-permissions.md#actions) |

The results of the release bundle itself are in
[The release bundle](#the-release-bundle).

## The release bundle

Date: 2026-11-12. The bundle from `scripts/release.sh`, signed and not
notarized. `open --env OPENCRAFT_PROFILES_DIR=<copy>` started it, with a
copy of a real profile, and the device was connected. A helper program read
and pressed the menu bar item through the Accessibility API, and took a
picture of the window. Each result is from 1 try, unless the row gives a
number.

| Fact | Confidence | Source |
|---|---|---|
| The bundle started. Its parent was launchd (process 1). `NSRunningApplication` gave the bundle identifier `com.shantanugoel.opencraft` and the regular activation policy. | Measured | `ps`; `NSRunningApplication` |
| The window opened and showed the profile and "Connected". The log file in the parent directory of the profiles directory got the start line, `connected`, and `full draw done`. | Measured | Window picture; log file |
| The menu bar item had the items of decision 0020 and the first item "OpenCraft 0.1.0", not enabled. | Measured | AX |
| A hotkey binding showed "OpenCraft needs the Accessibility permission to send keys." and the button "Give the permission". The new identifier has no permission until a person gives it. | Measured | Window picture |
| "Start at login" wrote `~/Library/LaunchAgents/opencraft.login.plist` with the path of the executable in the bundle, and the item got a check mark. A second selection removed the file. | Measured | File; AX |
| "Show the Dock icon" changed the activation policy to accessory and back to regular, and wrote `shell.toml`. | Measured | `NSRunningApplication`; file |
| "Quit OpenCraft" stopped the engine with the stop sequence. 3 seconds later, no OpenCraft process ran. | Measured | Log; `pgrep` |
| From the mounted disk image, "Start at login" wrote no file, showed no check mark, and showed a problem that tells the user to move OpenCraft to the Applications folder. | Measured | File; AX; window picture |
| A copy in `/Applications` wrote `/Applications/OpenCraft.app/Contents/MacOS/opencraft` into the LaunchAgent file. | Measured | File |
| A new copy at the same path in `/Applications`, as from an upgrade, showed the check mark at its start. | Measured | AX |
| A copy that was moved to a different directory showed no check mark. A selection wrote its new path into the file. | Measured | AX; file |
| The time from the start of the process to `main` was 253 to 406 ms for 4 starts of new copies, and 31 ms for 1 start. This agrees with the first start of a new executable file in [footprint.md](footprint.md). | Measured | Log file, 5 starts |

## Notarization

| Fact | Confidence | Source |
|---|---|---|
| `notarytool submit` accepts only a UDIF disk image, a signed flat installer package, or a zip file. | Source | [notarytool] |
| With `--wait`, `notarytool submit` returns when the status is "Accepted", "Invalid", or "Rejected". The exit status for "Invalid" is not documented. Thus a script reads the field `status` of the output of `--output-format json`. | Source | [notarytool]; [Apple forum: Invalid] |
| `notarytool store-credentials <profile>` puts the credentials in the keychain: an Apple ID, a team ID, and an app-specific password, or an App Store Connect API key. Then `--keychain-profile <profile>` uses them. | Source | [notarytool] |
| `notarytool log <id>` gives the log of a submission as JSON. | Source | [notarytool] |
| Apple tells developers to sign each item from the inside to the outside. | Source | [Packaging Mac Software] |
| A ticket that is stapled to a disk image does not go with the application when the user copies it out. If the application has no stapled ticket, Gatekeeper checks it on the network at the first start. A computer with no network then cannot check it. | Source | [Apple forum: offline]; [Apple forum: staple in dmg] |
| A stapled ticket changes the file. Thus the checksum of a disk image is correct only after `stapler staple`. | Measured | The SHA-256 of the disk image before and after `scripts/release.sh --notarize-only` |
| `notarytool store-credentials` with an Apple ID, the team ID of the Developer ID certificate, and an app-specific password checked the credentials and saved the profile. | Measured | `notarytool store-credentials` |
| The notary service accepted the zip file of the bundle and the disk image of version 0.1.0. The bundle has the hardened runtime and no entitlements. `notarytool history` shows the status "Accepted" for the two submissions. | Measured | `scripts/release.sh --notarize-only`; `notarytool history` |
| The two submissions, the two `stapler staple` calls, and the new disk image between them took less than 20 minutes in total. The service accepted the zip file of the bundle in less than 9 minutes. | Measured | The time of the run, and `createdDate` in `notarytool history` |
| After notarization and `stapler staple`, `spctl --assess --type exec` accepts the bundle, and `spctl --assess --type open --context context:primary-signature` accepts the disk image. Both give `source=Notarized Developer ID`. `stapler validate` passes for both. | Measured | `spctl`; `stapler validate` |

## Where the application runs from

| Fact | Confidence | Source |
|---|---|---|
| From macOS 10.12, macOS starts a quarantined application from a random read-only location (App Translocation), until the user moves the application with the Finder. | Source | [App Translocation] |
| An application that starts from a mounted disk image runs from `/Volumes/<volume name>/`, which is read-only. | Measured | `statvfs` of the executable in the mounted image |
| `statvfs` gives `ST_RDONLY` for `/` and `/usr/bin/true` (the system volume), and not for `/Applications` and a temporary directory. | Measured | `statvfs` |
| An upgrade of a Homebrew cask with the stanza `app` puts the new bundle at the same path in `/Applications`. | Inference | The stanza `app` moves the bundle to the Applications directory |

## Not known

- The first start of a notarized and stapled bundle: the dialog of macOS
  and its text. This needs the notary credentials of the owner.
- The behaviour on macOS 12 to 26. This computer has only macOS 27.
- An Intel build. This computer has no Rosetta.
- An Apple Event to an application that asks for the Automation
  permission, from the hardened bundle with no entitlement.
- Whether the actions "Send a hotkey", "Type text", and "Media key" operate
  in the release bundle after a person gives the Accessibility permission.
  The fact above is an inference. Nobody gave the permission to the release
  bundle, also not in the new-user test of issue
  [#152](https://github.com/shantanugoel/OpenCraft/issues/152).
- Whether the Accessibility permission of `com.shantanugoel.opencraft`
  stays after an update. The designated requirement says yes. A second
  version is necessary for a measurement.

## Sources

[Hardened Runtime]: <https://developer.apple.com/documentation/security/hardened-runtime>
[notarytool]: <https://keith.github.io/xcode-man-pages/notarytool.1.html>
[Apple forum: Invalid]: <https://developer.apple.com/forums/thread/751743>
[Packaging Mac Software]: <https://developer.apple.com/forums/thread/701581>
[Apple forum: offline]: <https://developer.apple.com/forums/thread/129146>
[Apple forum: staple in dmg]: <https://developer.apple.com/forums/thread/718583>
[App Translocation]: <https://developer.apple.com/forums/thread/724969>

- Apple Developer documentation, [Hardened Runtime].
- `man 1 notarytool`, copy at [notarytool].
- Apple Developer Forums: [Apple forum: Invalid], [Packaging Mac Software],
  [Apple forum: offline], [Apple forum: staple in dmg], [App Translocation].
- `build.rs` of the crate `gpui-pre-apple` 0.3.12.
