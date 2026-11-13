# Start at login on macOS

Last update: 2026-11-14

The decision that uses these facts is
[decision 0021](../decisions/0021-start-at-login.md).

## Test arrangement

Date: 2026-11-12. The computer and macOS are the same as in
[gpui.md](gpui.md#release-measurements).

- Two test programs, not in the repository, did the tests. The first is a
  small Swift program. It writes one line to a file and stops: the time, the
  parent process, its arguments, the result of `AXIsProcessTrusted`, and the
  environment variable `XPC_SERVICE_NAME`. The second is the GPUI test
  program of [gpui.md](gpui.md#shell-test-of-phase-5-2026-11-12). It opens no
  device.
- Both programs are bare executables with the ad-hoc signature of the
  linker, as a development build from `cargo build`. One SMAppService test
  also used a small bundle with an ad-hoc signature.
- A test agent is a plist in `~/Library/LaunchAgents`. The test loaded it
  with `launchctl bootstrap gui/<uid> <plist>`.
- After the tests, no test agent was loaded (`launchctl print` found no
  service), no test plist was in `~/Library/LaunchAgents`, and no test
  program ran.
- Nobody logged out. Thus a start at a real login is not measured. The rows
  for the login are Source or Inference.

## A LaunchAgent in the LaunchAgents directory of the user

| Fact | Confidence | Source |
|---|---|---|
| With `RunAtLoad` true, launchd starts the program when it loads the agent. `launchctl bootstrap` loaded the agent and started the program immediately. | Measured | Swift test program, 1 time; GPUI test program, 2 times |
| The parent of the program is launchd (process 1). The program gets the arguments of `ProgramArguments`, for example `--login`. The environment variable `XPC_SERVICE_NAME` has the label of the agent. | Measured | Swift and GPUI test programs |
| The GPUI test program, started by an agent, made its menu bar item and opened no window. It had no Dock icon. Its menu operated. | Measured | GPUI test program, AX checks |
| A bare executable that launchd starts does not get the Accessibility permission of the terminal: `AXIsProcessTrusted` gave false. The same executable, started from the terminal, gave true. | Measured | Swift test program |
| With no `KeepAlive` key, launchd does not start the program again after it stops. 15 seconds after a quit from the menu, `launchctl print` showed the state "not running", 1 run, and exit code 0. | Measured | GPUI test program |
| The removal of the plist file does not stop a program that runs. The job stayed loaded, with the state "running". | Measured | GPUI test program |
| `launchctl bootout` stops the program with SIGTERM. | Measured | GPUI test program: its signal thread got signal 15 |
| A plist that a program writes into `~/Library/LaunchAgents` is not loaded until a login or a `launchctl bootstrap`. `launchctl print` did not find it 8 seconds after the write. | Measured | Swift test program |
| `~/Library/LaunchAgents` holds the "per-user agents provided by the user". | Source | `man 8 launchd`, FILES |
| launchd loads each plist in `~/Library/LaunchAgents` when the user logs in, and then starts the program of an agent with `RunAtLoad`. | Inference | The two rows above, and the measured `RunAtLoad`. Not measured at a real login. |
| `LimitLoadToSessionType` limits an agent to a type of session, for example `Aqua`, the session of a login with a window server. | Source | `man 5 launchd.plist` |
| `ProcessType` `Interactive` gives the job the resource limits of an application, that is none. With no `ProcessType`, launchd limits the processor use and the I/O of the job. | Source | `man 5 launchd.plist` |
| The manual page says that `RunAtLoad` "should be avoided" because it has an effect on the time of a login. A start at login needs it. | Source | `man 5 launchd.plist` |

## OpenCraft with the LaunchAgent file

Date: 2026-11-12. The program is the release build of `opencraft` with the
module `login` of [decision 0021](../decisions/0021-start-at-login.md),
a bare executable. It used a copy of a real profile, and the device was
connected. A helper program read the state through the Accessibility API
and `CGWindowListCopyWindowInfo`, and pressed the menu items. Each result is
from 1 try.

For the start through launchd, the test used a copy of the file with a
different label and the key `EnvironmentVariables`, to give the path of the
copy of the profile. The copy was not in `~/Library/LaunchAgents`.

| Fact | Confidence | Source |
|---|---|---|
| "Start at login" in the menu of the menu bar item wrote `~/Library/LaunchAgents/opencraft.login.plist`. The file was the same, byte for byte, as the content in decision 0021, with the path of the executable. `plutil -lint` found no error. The item then had a check mark. | Measured | File; `cmp`; AX attribute `AXMenuItemMarkChar` |
| A second selection of the item removed the file, and the check mark went away. | Measured | File; AX |
| `launchctl bootstrap` accepted a plist from a directory that is not `~/Library/LaunchAgents`, and started its program. | Measured | `launchctl print`: state "running", 1 run |
| OpenCraft, started by launchd with `--login`, had the parent process 1, no window, and the menu bar item. The engine connected and drew the profile. A camera picture some seconds after the start showed the images of the profile on the device. | Measured | `ps`; window list; AX; log; USB camera picture |
| `launchctl bootout` stopped OpenCraft through its quit path: the log showed signal 15, the stop sequence of the engine, and the quit. 19 seconds later, no OpenCraft process ran. | Measured | Log; `launchctl print`; `pgrep` |
| OpenCraft, started from a terminal with `--login` and `show_dock_icon = false`, had the accessory policy. 71 checks of the Dock in the first 8 seconds did not find the Dock icon. | Measured | Dock items through AX; `NSRunningApplication` |

## Signed bundle and Accessibility (2026-11-13)

The test used macOS 27.2 (26B123) and the development bundle from
`scripts/sign-dev.sh --release`. It used a temporary LaunchAgent with a
different label, a scratch profile and `OPENCRAFT_NO_DEVICE=1`. It did not
change the real login item or the Accessibility list. The GUI lock was held
for each run. All test processes stopped, and each test agent was removed.

| Fact | Confidence | Source |
|---|---|---|
| The signed bundle reported `AXIsProcessTrusted = true` from the shell and when launchd started its executable directly with `--login`. | Measured | The start log, 1 shell run and 2 direct runs |
| A LaunchAgent that ran `/usr/bin/open -n -a <bundle> --args --login` also gave `true`. | Measured | The start log, 1 run |
| In both LaunchAgent forms, **Test the action** for a scratch `f20` hotkey sent a key-down and a key-up (code 90). | Measured | A listen-only Quartz session event tap, 1 pair per form |
| The ad-hoc debug executable, started directly by launchd, reported `false`. Its test sent no F20 event. Settings showed **not given**, the menu showed **Accessibility Permission Not Given…**, and the window and the log gave the missing-permission problem. | Measured | Start log, AX tree, window-only pictures and event tap |
| With Settings open and no device I/O, each of three 20-second idle samples used 0.04 to 0.05 seconds of processor time: 0.20 to 0.25 % of one core. | Measured | Difference of `ps -o time` samples; resolution 0.01 seconds |
| A shell start can inherit the terminal permission. The positive direct launchd result is stronger evidence of the permission of the signed bundle. It gives no evidence that `/usr/bin/open` is necessary for this bundle. | Inference | The comparisons above and the bare-executable test |

The current LaunchAgent mechanism stays. Decision 0035 is not necessary:
the suspected failure was not observed. This test does not prove behaviour
at a real login or in the downloaded release. The owner procedure for a
real login is in
[issue #143](https://github.com/shantanugoel/OpenCraft/issues/143).

## Background Task Management

macOS 13 and later keep a list of login items and background items. The
command `sfltool dumpbtm` prints it.

| Fact | Confidence | Source |
|---|---|---|
| macOS made a record of the type "legacy agent" for each plist in `~/Library/LaunchAgents`, also for a plist that was written and not loaded. The name of the record is the name of the executable. Its disposition is "enabled, allowed, notified". | Measured | `sfltool dumpbtm`, 3 plists |
| Each record stayed in the list after its plist was removed. It was there in each dump of the test, during some minutes. | Measured | `sfltool dumpbtm` |
| `sfltool` has no command that removes one record. `sfltool resetbtm` resets all records of the computer. | Measured | The usage text of `sfltool` |
| `sfltool dumpbtm` sometimes wrote no output. A second try gave the list. | Measured | 2 empty results in 6 tries |
| Later on the same day, the records of the removed test plists were not in the list. A dump less than 2.5 hours after the test showed no "legacy agent" record for them. The cause of the removal is not known. | Measured | `sfltool dumpbtm`, 2026-11-12 |
| 3 minutes after OpenCraft wrote and removed `opencraft.login.plist`, the list had no record for it. The file existed for less than 5 seconds. | Measured | `sfltool dumpbtm`, 2026-11-12 |
| System Settings shows these records in Login Items, in the list "Allow in the Background", and the user can turn an item off there. | Inference | The record types; not looked at in System Settings |
| The disposition "notified" means that macOS told the user about the new item with a notification. | Inference | The name of the flag |

## SMAppService

| Fact | Confidence | Source |
|---|---|---|
| Apple gives `SMAppService.mainApp` to start the main application bundle at login, from macOS 13. | Source | [SMAppService] |
| `SMAppService.mainApp.register()` in a test bundle with an ad-hoc signature gave the status "enabled", and a record of the type "app" with the disposition "enabled". `unregister()` gave "notRegistered", and the record stayed with the disposition "disabled". | Measured | Swift test program in a bundle |
| `register()` in the bare executable also gave "enabled", and a record of the type "app" with the path of the bare executable. `unregister()` gave "notRegistered". | Measured | Swift test program |
| The lock file of the workspace has no crate for the ServiceManagement framework. | Measured | `Cargo.lock` |

## Not known

- A start at a real login. Nobody logged out during the test. Issue
  [#143](https://github.com/shantanugoel/OpenCraft/issues/143) has a check
  by the owner.
- Whether a program in a signed bundle gets the Accessibility permission of
  that bundle when launchd starts its executable directly.
- When macOS removes the record of a plist that was removed, and why. The
  records of the first test stayed for some minutes, and went away in less than
  2.5 hours.
- How a program can find out that the user turned its item off in System
  Settings.
- Whether a record from `SMAppService.mainApp` for a bare executable starts
  that executable at login.

## Sources

[SMAppService]: <https://developer.apple.com/documentation/servicemanagement/smappservice>

- `man 8 launchd` and `man 5 launchd.plist`, on macOS 27.2.
- Apple Developer documentation, [SMAppService].
