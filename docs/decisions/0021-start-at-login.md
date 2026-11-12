# 0021: A LaunchAgent file starts OpenCraft at login, with the argument `--login`

- Status: Accepted
- Date: 2026-11-12

## Context

Phase 5 adds a start at login. After a start at login, the device operates
and no window opens. OpenCraft has no signed and notarized bundle before
Phase 6. A development build is a bare executable from `cargo build`, or
the bundle from `scripts/sign-dev.sh` that keeps the Accessibility
permission. The mechanism must operate for the two forms.

The facts are in [kb/macos-start-at-login.md](../kb/macos-start-at-login.md).
The important ones are:

- A plist in `~/Library/LaunchAgents` with `RunAtLoad` starts its program
  when launchd loads it. The program gets the arguments of the plist.
- With no `KeepAlive`, launchd does not start the program again after a
  quit.
- `launchctl bootstrap` starts the program immediately, and
  `launchctl bootout` stops it with SIGTERM.
- A bare executable that launchd starts does not have the Accessibility
  permission of the terminal.
- `SMAppService.mainApp` is the documented way for an application bundle.
  The workspace has no crate for it.

## Decision

**The mechanism.** The shell writes and removes one LaunchAgent file:
`~/Library/LaunchAgents/opencraft.login.plist`. The code is in
`crates/app`, in the module `login`, for `target_os = "macos"` only. It
needs no new dependency. The option is the item "Start at login" in the
menu of the menu bar item ([decision 0020](0020-menu-bar-item-and-dock-icon.md)).

The content of the file is this, with the path of the executable in place
of `EXECUTABLE`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>opencraft.login</string>
    <key>ProgramArguments</key>
    <array>
        <string>EXECUTABLE</string>
        <string>--login</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>LimitLoadToSessionType</key>
    <string>Aqua</string>
</dict>
</plist>
```

- `EXECUTABLE` is the value of `std::env::current_exe()`, with `&`, `<`,
  and `>` written as XML entities. For the bundle from `scripts/sign-dev.sh`,
  this is the executable in the bundle.
- The file has no `KeepAlive` key. Thus a quit is final until the next
  login.
- `ProcessType` `Interactive` prevents the limits that launchd puts on a
  background job.

**Turn the option on.** The shell makes the directory if it does not exist.
It writes the content to a temporary file in the same directory, and then
renames it to `opencraft.login.plist`. The shell does not call
`launchctl bootstrap`, because that starts a second OpenCraft immediately.

**Turn the option off.** The shell removes the file. It does not call
`launchctl bootout`, because that stops OpenCraft with SIGTERM if launchd
started it.

**The state of the option.** The option is on if the file exists. The
shell reads this at the start and after each change. The file names the
executable of the build that turned the option on. To change the
executable, the user turns the option off and on again.

Note (2026-11-12): [decision 0025](0025-release-form.md) changes this. The
option is on only if the file starts the executable of the process. A
selection then writes the file for that executable. The shell does not
write the file for an executable on a read-only volume, for example a disk
image.

**After a start at login.** The argument `--login` is the only difference
from a normal start:

- The engine starts as at a normal start.
- No window opens.
- The menu bar item shows.
- The Dock icon follows the option of
  [decision 0020](0020-menu-bar-item-and-dock-icon.md). With the option
  "hide", the Dock icon does not show during the start.
- The lock of [decision 0019](0019-engine-in-the-gui-process.md) applies as
  at a normal start.

A person can also start OpenCraft with `--login` by hand, to do the same
start. The application does not use `XPC_SERVICE_NAME` to find a start
through launchd.

**The development build.** For the actions that need the Accessibility
permission, the owner turns the option on from the bundle of
`scripts/sign-dev.sh`. A bare executable that launchd starts has no
Accessibility permission.

**Not SMAppService now.** Look at `SMAppService.mainApp` again in Phase 6,
when OpenCraft has a signed and notarized bundle. It is the documented way
for a bundle, and System Settings then shows the name of the application.
For Phase 5 it has these problems: its behaviour for a bare executable is
not documented, and it needs a new binding crate. Note (2026-11-12): Phase 6
looked at it again.
[Decision 0025](0025-release-form.md#changes-in-the-shell) keeps the
LaunchAgent file.

## Results

- The mechanism operates for each form of the build, with no new crate. The
  module `login` can take the directory as a parameter, thus a test can
  write and remove the file in a temporary directory with no device.
- macOS adds a record for the file to its list of background items, with
  the name of the executable (`opencraft`). The record stays after the file
  is removed. Note (2026-11-12): a later measurement found that the records
  went away some time after the removal of the file. See
  [kb/macos-start-at-login.md](../kb/macos-start-at-login.md#background-task-management).
  The user can turn the item off in System Settings. Then the
  menu still shows the option as on, because the file exists. See the "Not
  known" list in [kb/macos-start-at-login.md](../kb/macos-start-at-login.md#not-known).
- A start at a real login is not measured. The owner checks it in
  [issue #126](https://github.com/shantanugoel/OpenCraft/issues/126): log
  out and log in, and then see the menu bar item, no window, a device that
  operates, and an action that needs the Accessibility permission, for
  example a hotkey.
- If the bundle from `scripts/sign-dev.sh` does not get its Accessibility
  permission when launchd starts its executable directly, look at this
  decision again. A possible change is the program `/usr/bin/open` with the
  arguments `-a`, the bundle, `--args`, and `--login`.
- If the executable moves or is removed, the start at login fails with no
  message. The user turns the option off and on again. Note (2026-11-12):
  after [decision 0025](0025-release-form.md), the moved application shows
  the option as off, and one selection turns it on again.
