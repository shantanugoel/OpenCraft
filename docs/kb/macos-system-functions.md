# Mac system functions

Last update: 2026-11-14

The implementation is `crates/actions/src/system/macos_functions.rs`.
Each operation uses a public call. There is no AppleScript, System Events
request, private framework or new entitlement. Do Not Disturb is not part
of this action.

## Calls

| Function | Call | Confidence and source |
|---|---|---|
| Mission Control | `/usr/bin/open -b com.apple.exposelauncher` | Measured: the installed bundle has this identifier. One call entered Mission Control; Escape closed it at once. The OpenCraft window became a small preview during the call. |
| Launchpad | Quartz key events for Shift-Fn-A | Source: [Apple shortcuts]. Apple names Launchpad on earlier macOS and Apps on macOS 26 and later. One call returned successfully on macOS 27.2, then Escape reversed it; the visible result was not confirmed. The owner check is in [Owner check (2026-11-14)](#owner-check-2026-11-14). |
| Screenshot | Quartz key events for Shift-Command-5 | Source: [Apple shortcuts]. This opens the system screenshot controls; OpenCraft does not capture pixels or save a screenshot. The agent check did not execute it. The owner check is in [Owner check (2026-11-14)](#owner-check-2026-11-14). |
| Screen saver | `/usr/bin/open -b com.apple.ScreenSaver.Engine` | Measured: the installed ScreenSaverEngine bundle has this identifier. Inference: launch through the public `open` command starts the screen saver. The owner check is in [Owner check (2026-11-14)](#owner-check-2026-11-14). |
| Sleep | `IOPMFindPowerManagement(MACH_PORT_NULL)`, `IOPMSleepSystem`, then `IOServiceClose` | Source: SDK `IOKit/pwr_mgt/IOPMLib.h`. The console user or root can request sleep. The owner check is in [Sleep that macOS refuses (2026-11-14)](#sleep-that-macos-refuses-2026-11-14). |
| Lock | Quartz key events for Control-Command-Q | Source: [Apple shortcuts]. The owner check is in [Owner check (2026-11-14)](#owner-check-2026-11-14). |
| Show desktop | Quartz key events for Fn-F11 | Measured: one call moved the OpenCraft window outside the display. The same chord reversed it at once and restored the window. Source: [Apple shortcuts]. |

Date of the checks: 2026-11-13. The computer has macOS 27.2. The signed
development window used a scratch profile and no device. A program started
from the shell called the same system functions as the handlers. No picture
of Mission Control, Apps or the desktop was saved. Sleep, lock and screen
saver were tested only through the fake System interface. Native tests
inspect the call selection without executing any call.

## Sleep that macOS refuses (2026-11-14)

The log of the owner sitting of #255 had "the system refused the operation
(code -536870174)" for Sleep. Issue
[#324](https://github.com/shantanugoel/OpenCraft/issues/324). No real sleep
was requested for these facts.

| Fact | Confidence | Source |
|---|---|---|
| The code -536870174 is `0xE00002E2`, `kIOReturnNotPermitted`. | Source | SDK `IOKit/IOReturn.h` |
| `IOPMSleepSystem` goes to `IOPMrootDomain::privateSleepSystem`. It returns `kIOReturnNotPermitted` when `checkSystemSleepEnabled` is false. That check is false when the sleep switch of user space is on (`userDisabledAllSleep`, the root domain property "SleepDisabled"), when sleep is turned off in the kernel configuration, or during a restart or a shutdown. | Source | [xnu IOPMrootDomain] |
| A `PreventUserIdleSystemSleep` assertion does not cause this result. The SDK says that the system can still sleep from the Apple menu with that assertion. | Source | SDK `IOKit/pwr_mgt/IOPMLib.h` |
| `kIOPMAssertionTypePreventSystemSleep` is deprecated since 10.9, and not supported. | Source | SDK `IOKit/pwr_mgt/IOPMLib.h` |
| On the computer of the owner, `pmset -g` shows `SleepDisabled 1`, and the root domain has "SleepDisabled" = Yes. The command `pmset disablesleep 1` (root) sets it. A keep-awake application can set it through its helper. | Measured | `pmset -g`; `ioreg -n IOPMrootDomain -r -d1`; `pmset(1)` |
| On the same computer, the item "Sleep" of the Apple menu is disabled. | Measured | Accessibility tree of the menu bar, read with cua-driver |
| No public call makes the computer sleep while "SleepDisabled" is on. A change of the switch needs root, which OpenCraft does not have and does not ask for. | Inference | The facts above |
| The action gives the message "macOS does not allow sleep now. A setting turns off all sleep, …" for this code. Other codes keep the general message with the code. | Measured | Tests `a_refused_sleep_names_the_cause` and `a_sleep_that_macos_refuses_gives_a_message_with_the_cause` |

## Owner check (2026-11-14)

The owner pressed the keys of page 3 in the procedure of that date
([checks/phase-7-8-checks.md](../checks/phase-7-8-checks.md) at commit
`8cb7fd8`). The bundle was the signed development release build. The
computer has macOS 27. Sleep is in the section above. The device result of
Lock is in
[sleep-and-wake.md](sleep-and-wake.md#checks-by-the-owner-2026-11-14).

| Fact | Confidence | Source |
|---|---|---|
| Mission Control showed the windows. | Measured | Owner check, [issue #255](https://github.com/shantanugoel/OpenCraft/issues/255) |
| Show desktop moved the windows away. A second press brought them back. | Measured | Owner check, issue #255 |
| Launchpad, or Apps, opened. | Measured | Owner check, issue #255 |
| The screenshot controls showed. | Measured | Owner check, issue #255 |
| The screen saver started. | Measured | Owner check, issue #255 |
| Lock locked the screen. | Measured | Owner check, issue #255 |

## Permissions and compatibility

| Fact | Confidence | Source |
|---|---|---|
| `open -b` is the public command to open an application by bundle identifier. It does not send an Automation request to System Events. | Source | `open(1)` |
| Mission Control and the screen saver open a bundle with `open -b`. Sleep calls `IOPMSleepSystem`. These calls send no key event. They need no Accessibility permission. | Inference | The call table; [macos-permissions.md](macos-permissions.md#actions) |
| Launchpad, Screenshot, Lock, and Show desktop send a key event. They need the Accessibility permission. | Inference | The call table; [macos-permissions.md](macos-permissions.md#actions) |
| The kind adds no prompt for Automation or Screen Recording. | Inference | The calls above |
| The key-based choices use the default system shortcuts. A changed or disabled shortcut can change the result. | Source | [Apple shortcuts] |
| On this computer, there is no `/System/Applications/Launchpad.app`. The action uses the documented shortcut, not that path. | Measured | File check, macOS 27.2 |

## Not known

- The results on macOS 12 to 26, and with customized system shortcuts.
- A sleep of this computer. The setting `SleepDisabled` is on, so the owner
  check could not sleep it. The open check is
  [issue #143](https://github.com/shantanugoel/OpenCraft/issues/143). The
  measured refusal is in
  [Sleep that macOS refuses (2026-11-14)](#sleep-that-macos-refuses-2026-11-14).

## Sources

[Apple shortcuts]: <https://support.apple.com/en-us/102650>
[xnu IOPMrootDomain]: <https://github.com/apple-oss-distributions/xnu/blob/main/iokit/Kernel/IOPMrootDomain.cpp>

- Apple Support, **Mac keyboard shortcuts**, published 2026-09-14.
- macOS SDK: `IOKit/pwr_mgt/IOPMLib.h`, `IOKit/IOKitLib.h`,
  `CoreGraphics/CGEvent.h` and `CoreGraphics/CGEventTypes.h`.
- The bundle identifiers of the installed Mission Control and
  ScreenSaverEngine applications.
- macOS manual page `open(1)`.
