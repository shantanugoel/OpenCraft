# 0020: The shell makes the menu bar item with AppKit, and has one quit path

- Status: Accepted
- Date: 2026-11-12

## Context

Phase 5 adds a menu bar item and an option to hide the Dock icon. GPUI has
no API for a menu bar item. This was the open item of
[decision 0005](0005-gpui.md). The application of Phase 4 also has two
problems for an application with no window:

- A close of the window stops the engine and quits the application.
- Cmd-Q does not quit the application.

A test program with GPUI and AppKit, and no engine, gave the facts. They are
in [kb/gpui.md](../kb/gpui.md#shell-test-of-phase-5-2026-11-12). The process
model is in [decision 0019](0019-engine-in-the-gui-process.md).

## Decision

### Code and dependencies

The menu bar item and the Dock icon are shell code. They are in
`crates/app`, in the module `menu_bar`, for `target_os = "macos"` only. The
engine and `opencraft-actions` do not change. Only this module contains
`unsafe` code for AppKit.

Add these dependencies to `crates/app`, for macOS only. They are the
versions that GPUI already uses in `Cargo.lock`:

| Crate | Version | Features |
|---|---|---|
| `objc2` | `0.6` (the workspace dependency) | default |
| `objc2-foundation` | `0.3`, `default-features = false` | `std`, `NSString`, `NSObject`, `NSArray` |
| `objc2-app-kit` | `0.3`, `default-features = false` | `std`, `NSStatusBar`, `NSStatusItem`, `NSStatusBarButton`, `NSButton`, `NSControl`, `NSView`, `NSResponder`, `NSMenu`, `NSMenuItem`, `NSCell`, `NSApplication`, `NSRunningApplication`, `NSImage` |

`crates/app` also gets these workspace dependencies: `libc`, for
`cfg(unix)`, for the signals below and the lock of decision 0019; and
`serde` and `toml`, for all platforms, for the file `shell.toml` below. No
new crate comes into `Cargo.lock`. (Inference: the lock file of the test
program has the same versions.)

`NSImage` is for the image on the button. The test program did not use it.

### The menu bar item

The shell makes the item in the callback of `Application::run`, on the main
thread, with
`NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength)`.
The shell keeps the item, the menu, and the target object until the process
stops. The button shows an SF Symbol as a template image, with the
accessibility description "OpenCraft". If the image is not available, the
button shows the title "OpenCraft".

The menu has these items, in this order:

| Item | Kind | Operation |
|---|---|---|
| "Device: connected" or "Device: not connected" | Text, not enabled | Shows the last `Connected` or `Disconnected` event of the engine |
| "Profile: *name*" | Text, not enabled | Shows the name of the active profile. Not shown before the engine sends the profiles. |
| Separator | | |
| "Open OpenCraft" | Command | Opens the window. If the window is open, brings it and the application to the front. |
| Separator | | |
| "Show the Dock icon" | Check mark | Shows or hides the Dock icon. See below. |
| "Start at login" | Check mark | See [decision 0021](0021-start-at-login.md). |
| Separator | | |
| "Quit OpenCraft" | Command, key equivalent Cmd-Q | The quit path below |

Note (2026-11-12): [decision 0025](0025-release-form.md) adds the first
item "OpenCraft *version*", text, not enabled, with a separator after it.
[Decision 0026](0026-licence-notices.md) adds the command "Licence notices"
after it, before the separator, and the features `NSWorkspace` and `NSURL`.

Note (2026-11-13): [decision 0029](0029-window-layout.md), section 17,
moves the version, the licence notices, "Show the Dock icon", and "Start at
login" into the Settings window. The menu then has "Settings…" and "About
and licence notices", which open that window. The options keep their
effects, which this record gives.

The menu has no control to select a profile or a page. The window and the
device do this. Look at this again if the owner asks for it.

Note (2026-11-13): the owner asked for it. [Decision
0029](0029-window-layout.md), section 19, gives the menu a list of the
profiles, a submenu of the pages, and a slider for the display brightness.
The tested facts about a custom view in a menu are in
[kb/gpui.md](../kb/gpui.md#the-menu-with-the-brightness-row-2026-11-13).

Note (2026-11-14): the profiles are one submenu. Section 19 of
[decision 0029](0029-window-layout.md) gives the items.

### Commands from the menu

A class that `objc2::define_class!` makes is the target of each menu item.
Each method sends a shell command on an `async_channel`. One GPUI
foreground task from `App::spawn` receives the commands and does the work.
The thread for the signals sends its quit command on the same channel. The
same task changes the titles and the check marks of the menu when the state
changes.

### State with no window

The engine events go to one receiver for the process, not to the window.
The `ViewState` of `crates/app/src/state.rs` moves into an entity that the
shell makes in the callback of `Application::run`. That entity receives the
events, also when no window is open. The window view reads the entity, thus
a new window shows the current state immediately. The menu reads the same
entity. The engine needs no new command.

Note (2026-11-12): the implementation gives a new window a copy of the state
of the entity. Then the window gets each event from the entity through a GPUI
subscription (`DeviceView` in `crates/app/src/view.rs`). The window keeps its
own copy, because it also does work for each event: the images, the text
fields, and the changes that wait for a reply. A new window shows the current
state immediately, as the decision says.

### The window

- The application uses `QuitMode::Explicit`. A close of the window closes
  only the window. The engine continues.
- "Open OpenCraft", and `Application::on_reopen` (a click on the Dock icon
  when no window is open), open a new window if no window exists.
  Otherwise they activate the window and the application.
- A normal start opens the window. A start at login opens no window
  ([decision 0021](0021-start-at-login.md)).

### Application menu and Cmd-Q

The shell sets an application menu with `App::set_menus`. It has one menu,
"OpenCraft", with the item "Quit OpenCraft" for a GPUI action `Quit`. The
shell binds `cmd-q` to `Quit` with `App::bind_keys`. The action starts the
quit path. The test program showed that this corrects the Cmd-Q result of
Phase 4.

### The Dock icon option

| Option | Activation policy |
|---|---|
| Show the Dock icon (the default) | `ActivationPolicy::Regular` |
| Hide the Dock icon | `ActivationPolicy::Accessory` |

- At the start, if the option is "hide", the shell calls
  `Application::with_windowing(WindowingRequest::Headless)` before
  `Application::run`. Thus the Dock icon does not show during the start.
- To hide the icon, the shell calls
  `App::set_activation_policy(ActivationPolicy::Accessory)`. An open window
  stays open.
- To show the icon, the shell sets `ActivationPolicy::Regular`. If
  `NSApplication::isActive` is true at that time, the shell first activates
  the Dock application (bundle identifier `com.apple.dock`) with
  `NSRunningApplication::activateWithOptions`, and it calls
  `App::activate(true)` 200 ms after the change. Without this, the menu bar
  stays with a different application.
- The shell keeps the option in its own file `shell.toml`, in the parent
  directory of the profiles directory, with the key `show_dock_icon`. The
  shell writes the file when the user changes the option. If the file is
  missing, the value is `true`. This is not engine state: a different shell
  has different settings (rule 12). Thus the shell writes this file, and
  the comment "Only the engine ... writes files" in `crates/app/src/main.rs`
  changes. Note (2026-11-12): the shell writes more files from later
  decisions. See the note in
  [decision 0024](0024-log-file-with-no-terminal.md#results).

### The quit path

All quit requests go to one path:

- "Quit OpenCraft" in the menu bar item,
- the `Quit` action, from the application menu and from Cmd-Q,
- SIGTERM and SIGINT, also from `launchctl bootout`,
- a terminate from macOS, for example at a logout or from the menu of the
  Dock icon.

Note (2026-11-12): [decision 0022](0022-device-settings-and-stop-state.md)
adds SIGHUP. The code sets the handler of "Signals" below for SIGTERM,
SIGINT, and SIGHUP (`crates/app/src/signals.rs`).

Each request calls `App::quit`. `App::quit` calls
`NSApplication::terminate`, and then GPUI calls the `on_app_quit` closures.
A terminate from macOS goes into the same closures. The closure of the shell calls
`EngineHandle::stop` in the closure itself, and then returns a future that
is ready. It does not stop the engine in the future, because GPUI waits for
a future for at most 200 ms. The main thread waits until the engine has
stopped. The device state at the stop is the work of
[issue #125](https://github.com/shantanugoel/OpenCraft/issues/125); it
occurs in `EngineHandle::stop`. The stop of the engine must not wait for
the main thread. The code after `Application::run` stays as a fallback.

**Signals.** At the start of `main`, the shell makes a pipe and sets a
handler with `libc::signal` for SIGTERM and for SIGINT. The handler only
writes one byte to the pipe. One thread reads the pipe and sends the quit
command on the channel of the shell. Do not block the signals with
`pthread_sigmask` and `sigwait`: a child process, for example from the
shell command action, gets the blocked mask and then does not stop at
SIGTERM.

## Results

- The shell has a small amount of `unsafe` AppKit code in one module. The
  GPUI update cost of [decision 0005](0005-gpui.md) now includes a test of
  the menu bar item and of the Dock icon.
- The menu bar item operates with no window and with a window open. The
  window can open and close many times while the engine continues.
- The engine stops in the same way for each quit request, also for SIGTERM.
  SIGKILL and a crash do not stop the engine. Then the device gets no stop
  commands. It stops the USB connection some seconds after the last
  keep-alive
  ([kb/streamdock-protocol.md](../kb/streamdock-protocol.md#keep-alive-and-stop)).
- A quit waits for the engine with no time limit. If the stop of the engine
  can be long, issue #125 must limit it. Note (2026-11-12): issue #125 added
  the limit. `EngineHandle::stop` waits for `STOP_TIMEOUT` (3 seconds) at
  most, and then the quit continues (`crates/engine/src/runtime.rs`).
- The test program did not measure a logout, the list of Cmd-Tab, or the
  image on the button. See the "Not known" list in
  [kb/gpui.md](../kb/gpui.md#not-known). Issue
  [#123](https://github.com/shantanugoel/OpenCraft/issues/123) checks them.
  Note (2026-11-12): issue #123 measured the list of Cmd-Tab
  ([kb/gpui.md](../kb/gpui.md#windows-dock-icon-and-activation)) and the
  image in the dark menu bar
  ([kb/gpui.md](../kb/gpui.md#application-checks-of-phase-5-2026-11-12)).
  A logout and the light menu bar stay in the "Not known" list.
- A different platform has no `menu_bar` module. The application continues
  to build on Linux in CI with no menu bar item.
