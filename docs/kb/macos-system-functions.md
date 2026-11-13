# Mac system functions

Last update: 2026-11-13

The implementation is `crates/actions/src/system/macos_functions.rs`.
Each operation uses a public call. There is no AppleScript, System Events
request, private framework or new entitlement. Do Not Disturb is not part
of this action.

## Calls

| Function | Call | Confidence and source |
|---|---|---|
| Mission Control | `/usr/bin/open -b com.apple.exposelauncher` | Measured: the installed bundle has this identifier. One call entered Mission Control; Escape closed it at once. The OpenCraft window became a small preview during the call. |
| Launchpad | Quartz key events for Shift-Fn-A | Source: [Apple shortcuts]. Apple names Launchpad on earlier macOS and Apps on macOS 26 and later. One call returned successfully on macOS 27.2, then Escape reversed it; the visible result was not confirmed. |
| Screenshot | Quartz key events for Shift-Command-5 | Source: [Apple shortcuts]. This opens the system screenshot controls; OpenCraft does not capture pixels or save a screenshot. Not executed in this check. |
| Screen saver | `/usr/bin/open -b com.apple.ScreenSaver.Engine` | Measured: the installed ScreenSaverEngine bundle has this identifier. Inference: launch through the public `open` command starts the screen saver. Not executed; owner check in #255. |
| Sleep | `IOPMFindPowerManagement(MACH_PORT_NULL)`, `IOPMSleepSystem`, then `IOServiceClose` | Source: SDK `IOKit/pwr_mgt/IOPMLib.h`. The console user or root can request sleep. Not executed; owner check in #255. |
| Lock | Quartz key events for Control-Command-Q | Source: [Apple shortcuts]. Not executed; owner check in #255. |
| Show desktop | Quartz key events for Fn-F11 | Measured: one call moved the OpenCraft window outside the display. The same chord reversed it at once and restored the window. Source: [Apple shortcuts]. |

Date of the checks: 2026-11-13. The computer has macOS 27.2. The signed
development window used a scratch profile and no device. A program started
from the shell called the same system functions as the handlers. No picture
of Mission Control, Apps or the desktop was saved. Sleep, lock and screen
saver were tested only through the fake System interface. Native tests
inspect the call selection without executing any call.

## Permissions and compatibility

| Fact | Confidence | Source |
|---|---|---|
| `open -b` is the public command to open an application by bundle identifier. It does not send an Automation request to System Events. | Source | `open(1)` |
| The kind uses the existing Accessibility permission because some choices send key events. It adds no prompt for Automation or Screen Recording. | Inference | The calls above; [macos-permissions.md](macos-permissions.md) |
| The key-based choices use the default system shortcuts. A changed or disabled shortcut can change the result. | Source | [Apple shortcuts] |
| On this computer, there is no `/System/Applications/Launchpad.app`. The action uses the documented shortcut, not that path. | Measured | File check, macOS 27.2 |

## Not known

- Whether the signed release bundle starts the screen saver, requests sleep
  and locks the screen. The owner procedure is in
  [#255](https://github.com/shantanugoel/OpenCraft/issues/255).
- The visible result of the Launchpad shortcut on this computer. The
  one-time agent check did not establish it. The owner procedure is in #255.
- The results on macOS 12 to 26, and with customized system shortcuts.

## Sources

[Apple shortcuts]: <https://support.apple.com/en-us/102650>

- Apple Support, **Mac keyboard shortcuts**, published 2026-09-14.
- macOS SDK: `IOKit/pwr_mgt/IOPMLib.h`, `IOKit/IOKitLib.h`,
  `CoreGraphics/CGEvent.h` and `CoreGraphics/CGEventTypes.h`.
- The bundle identifiers of the installed Mission Control and
  ScreenSaverEngine applications.
- macOS manual page `open(1)`.
