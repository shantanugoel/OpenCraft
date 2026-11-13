# macOS privacy permissions

Last update: 2026-11-12

## Device access

| Fact | Confidence | Source |
|---|---|---|
| The vendor interface (interface 0, usage page `0xFFA0`) needs no privacy permission | Measured | Probe |
| The keyboard-emulation interface (interface 1, usage page `0x0001`) needs the Input Monitoring permission | Measured | Probe |

Thus OpenCraft does not open the keyboard-emulation interface. This is rule 6
of the [architecture](../ARCHITECTURE.md#5-rules). OpenCraft does not need the
Input Monitoring permission.

## Actions

Date: 2026-11-12. A test program, not in the repository, did each
operation two times: started from a terminal that has the Accessibility
permission, and as an application bundle with no permission.

| Fact | Confidence |
|---|---|
| A program reads and sets the sound volume and the mute of the default output through Core Audio with no permission | Measured |
| A media key event from a program with no Accessibility permission does nothing: the "sound up" key did not change the volume. With the permission, it changed the volume. | Measured |
| A key event, a text event, or a media key event is lost if the program stops immediately after it sends the event. A program that stays for 300 ms does not lose it. | Measured |
| With the Accessibility permission, a program types text into the application in front, also letters with accents, a currency sign, an emoji, and a text of more than 20 UTF-16 units. A hotkey (`cmd+a`) operates in that application. | Measured: the label field of the OpenCraft window got the text |
| `CGPreflightPostEventAccess` gives the same answer as `AXIsProcessTrusted` in the two cases | Measured |
| An application needs the Accessibility permission to send key events to other applications | Inference |
| An application needs no permission to open an application or a URL | Inference |

The code is in `crates/actions/src/system/macos.rs`.

The first plan put the permission work after the action work. That order is
incorrect. The hotkey action needs the Accessibility permission on the first
day.

## Input mute (2026-11-15)

Fact 13 of the calibration notes of issue #198: a Core Audio property test
found four input devices. Three, including the default input, had a writable
master mute property. The fourth had no master mute property. The test read
the device list, input streams, default input and property flags only. It
opened no input stream and showed no microphone permission prompt.
Confidence: **Measured**, a local Swift test with CoreAudio. No device name
or unique device identifier was recorded.

A signed development build then used “Test the action” to set the default
input mute. A separate Core Audio read gave mute on. A separate test restored
the old value, and the window removed the state frame through the property
listener, with no gesture. The build used a scratch profile and the device
lock, with the real controller connected. No microphone prompt appeared.
Confidence: **Measured**. The action opens no audio input stream.

## Code signature of a development build

Date: 2026-11-12. Tool: `codesign -d -r-`, on the program `opencraft`.

| Fact | Confidence | Source |
|---|---|---|
| The linker gives each build an ad-hoc signature. The designated requirement of such a build is its code hash (`cdhash`). | Measured | 2 builds |
| A new build with a changed source file has a different code hash. Thus its designated requirement is different. | Measured | 2 builds |
| A build that `codesign` signs with an identity of the keychain and a fixed identifier has a designated requirement of the identifier and the certificate. It contains no code hash. | Measured | `scripts/sign-dev.sh` |
| That designated requirement is the same after a new build with a changed source file | Measured | 2 builds |
| `codesign` signs with an identity of the login keychain and shows no prompt | Measured | 3 signatures |
| macOS identifies a program for a privacy permission by its designated requirement. Thus the permission does not stay for builds with the ad-hoc signature. | Inference | The facts above |
| The Accessibility permission stays after a new build from `scripts/sign-dev.sh` with a changed source file. The owner gave the permission one time. After the new build, a hotkey action operated, and macOS did not ask again. | Measured: owner | 2026-11-12, 2 builds |
| A program that starts from a terminal has the Accessibility permission of the terminal application: `AXIsProcessTrusted` gave true in a test program with no permission of its own | Measured | A test program, not in the repository |
| The same test program in an application bundle, started with `open`, has its own permission state: `AXIsProcessTrusted` gave false | Measured | The same test program |

`scripts/sign-dev.sh` makes a bundle with the stable signature. See
[scripts/README.md](../../scripts/README.md#sign-devsh).

The permission of a program that launchd starts is in
[macos-start-at-login.md](macos-start-at-login.md#a-launchagent-in-the-launchagents-directory-of-the-user).

## Right-side modifiers and Fn (2026-11-13)

| Fact | Confidence | Source |
|---|---|---|
| GPUI 0.3.12 gives one Boolean for both sides of each modifier. Its modifier-change event has no key code. | Source | `platform/keystroke.rs`, `interactive.rs` of GPUI |
| The local AppKit event has device-dependent flags for the left and right modifiers. Reading `NSApplication.currentEvent` adds no event monitor. | Source | AppKit `NSEvent.h`; IOKit `IOLLEvent.h` |
| Quartz has the Fn flag and key code `0x3F`. Right Control, Option, Shift and Command have separate virtual key codes. | Source | Core Graphics `CGEventTypes.h`; Carbon `Events.h` |
| The recorder reads right Control and Fn from local synthetic modifier events and saves a modifier-only chord on release. A shell-started sender with right Shift types an uppercase letter into the OpenCraft title field. | Measured | Signed development window, scratch profile, issue #275 |

## Mouse events (2026-11-13)

| Fact | Confidence | Source |
|---|---|---|
| Quartz creates mouse, drag and scroll events with public Event Services calls. Line scroll uses `CGEventCreateScrollWheelEvent2`. | Source | Core Graphics `CGEvent.h` |
| A shell-started sender selects key 2 in the OpenCraft window with a left click. A line scroll moves the mouse form in that window at both test sizes. No other application is a receiver. | Measured | Signed development window and scratch profiles, issue #281 |
| A mouse click that the action sends has the Unix process ID of the OpenCraft process in its event source. The local AppKit event keeps that ID. | Measured | Passive event tap, issue #281 |
| A click action tested on its own test button sends one click only. The shared test command ignores the event that this process sent. | Measured | Signed development window, issue #281 |

## Not known

- Whether the permission stays after a change of the signing identity, for
  example a new certificate.
