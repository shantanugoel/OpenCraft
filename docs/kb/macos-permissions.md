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
| A media key event is lost if the program stops immediately after it sends the event. A program that stays for 300 ms does not lose it. | Measured |
| `CGPreflightPostEventAccess` gives the same answer as `AXIsProcessTrusted` in the two cases | Measured |
| An application needs the Accessibility permission to send key events to other applications | Inference |
| An application needs no permission to open an application or a URL | Inference |

The code is in `crates/actions/src/system/macos.rs`.

The first plan put the permission work after the action work. That order is
incorrect. The hotkey action needs the Accessibility permission on the first
day.

## Code signature of a development build

Date: 2026-11-12. Tool: `codesign -d -r-`, on the program `opencraft`.

| Fact | Confidence | Source |
|---|---|---|
| The linker gives each build an ad-hoc signature. The designated requirement of such a build is its code hash (`cdhash`). | Measured | 2 builds |
| A new build with a changed source file has a different code hash. Thus its designated requirement is different. | Measured | 2 builds |
| A build that `codesign` signs with an identity of the keychain and a fixed identifier has a designated requirement of the identifier and the certificate. It contains no code hash. | Measured | `scripts/sign-dev.sh` |
| That designated requirement is the same after a new build with a changed source file | Measured | 2 builds |
| `codesign` signs with an identity of the login keychain and shows no prompt | Measured | 3 signatures |
| macOS identifies a program for a privacy permission by its designated requirement. Thus the permission stays for builds from `scripts/sign-dev.sh`, and it does not stay for builds with the ad-hoc signature. | Inference | The facts above |

| A program that starts from a terminal has the Accessibility permission of the terminal application: `AXIsProcessTrusted` gave true in a test program with no permission of its own | Measured | A test program, not in the repository |
| The same test program in an application bundle, started with `open`, has its own permission state: `AXIsProcessTrusted` gave false | Measured | The same test program |

`scripts/sign-dev.sh` makes a bundle with the stable signature. See
[scripts/README.md](../../scripts/README.md#sign-devsh).

## Not known

- Whether a development build with a stable signature keeps the Accessibility
  permission after each build. The designated requirement stays the same:
  see "Code signature of a development build". The test with the permission
  itself needs the owner, who gives the permission one time.
