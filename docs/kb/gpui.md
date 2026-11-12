# GPUI

Last update: 2026-11-13

GPUI is the GUI toolkit of the Zed editor. The decision that uses these facts
is [decision 0005](../decisions/0005-gpui.md).

## Crates and sources

| Fact | Confidence | Source |
|---|---|---|
| `gpui` on crates.io is still at 0.2.2. `gpui-pre` is at 0.3.12. | Source | `cargo search` and `cargo info`, 2026-11-11; [registry] |
| `gpui-pre` 0.3.12 names Zed revision `d2a4fb6ded7c470be25d1541281da3882495b65b`. It is a third-party snapshot, not an official Zed release. | Source | Published `Cargo.toml`, `package.metadata.gpui-pre`; [publish script] |
| Longbridge's release workflow checks for a new snapshot each Monday. The script also changes crate names and macro paths. It does not copy Zed without changes. | Source | [release workflow]; [publish script] |
| `gpui-kit` 0.9.1 pins `gpui-pre` and `gpui-pre-platform` to exactly 0.3.12. Its default features add `gpui-component` and `gpui-kit-assets`. It always adds `gpui-base`. | Source | Published `gpui-kit` 0.9.1 `Cargo.toml`; [kit] |
| `gpui-kit` is a facade. `gpui-component` is its styled widget layer. They are different crates in one repository, not two names for one crate. The old repository name redirects to `longbridge/gpui-kit`. | Source | `gh api` for both repository names; published manifests; [kit] |
| The kit README states that it has 75+ components and primitives, and AccessKit support. This test did not audit accessibility. | Source | [kit README] |
| Both test forms build on installed stable Rust 1.99.1, on macOS arm64. A separate manifest with only the kit, `image`, and `async-channel` also builds. No Zed Git dependency was necessary. | Measured | Test program: `Cargo.toml`, `facade-check/Cargo.toml`, build logs |

## Phase 2 API test

**Measured.** A test program, not in the repository, has two release forms.
The bare form uses `gpui-pre = 0.3.12` and `gpui-pre-platform = 0.3.12`.
The widget form adds `gpui-kit = 0.9.1`, with default features. The program
uses no engine or device layer. It does not open a device. Source: test
program, `src/main.rs` and `Cargo.lock`.

The API names below are for these versions. The kit re-exports GPUI names
at `gpui_kit::*`.

| Fact | Confidence | Source |
|---|---|---|
| `div`, fixed sizes, flex rows, and `rounded_full` make the 10 squares, 4 wide bar panels, and 4 circles. Each has `on_click`, `on_mouse_down`, `on_mouse_up`, and `on_mouse_up_out`. State changes set the border and background, then call `Context::notify`. | Measured | Test program, `src/main.rs`, `Spike::control` and `Render` |
| Both forms pass real window-dispatch tests for all 18 click handlers, selected state, pressed state, release, colour choice, and typed text. These are synthetic GUI events, not device input events. | Measured | Test program, `src/self_test.rs`; `bare-measure.log`, `widgets-measure.log` |
| `img(Arc<RenderImage>)` shows pixels from memory. `RenderImage::new([image::Frame::new(buffer)])` accepts a frame whose bytes are in BGRA order. It does not accept RGB bytes directly. | Measured | Test program, `src/main.rs`, `pixels`; published GPUI `src/assets.rs` and `src/elements/img.rs` |
| A worker makes an `image::RgbImage` of 112 x 112 and another of 176 x 112. The GUI converts them to BGRA with opaque alpha. It shows both, and replaces both at 10 Hz. The first image is in a square. | Measured | Test program, `src/main.rs`, `produce`, `pixels`, and `receive` |
| This path needs no image encoding or decoding. It is not zero-copy: it allocates and fills a BGRA buffer, then uploads new image data to the GPU atlas. The BGRA data is 50,176 bytes for the square and 78,848 bytes for the bar panel. A pair at 10 Hz is 1,290,240 bytes per second before atlas overhead. | Source; Inference (byte counts) | Published GPUI `src/window.rs`, `paint_image`; `gpui-pre-apple` 0.3.12 `src/metal_atlas.rs`; 4 bytes per pixel |
| Call `Window::drop_image(old)` when replacing a `RenderImage`. Dropping its `Arc` alone does not remove the GPU atlas entry. Keep the queue bounded. | Source | Published GPUI `src/window.rs`, `drop_image` and `paint_image` |
| Each form received 650 image pairs. A 60-second replacement run had no steady footprint growth. Weak references showed no retained old CPU images after the final frame. The code also removes old GPU atlas entries. This is a bounded test, not proof for an unlimited run. | Measured | Test program, `measure.py`, `measurements.json`, and both `*-measure.log` files |
| Use `Context::spawn_in(window, async move \|view, cx\| ...)` for a foreground task. Await `async_channel::Receiver::recv`, then use `WeakEntity::update_in` and `Context::notify`. The worker uses `Sender::send_blocking`. The channel wakes the GPUI executor. The program has no queue-poll timer and no async runtime of its own. | Measured | Test program, `src/main.rs`, `Spike::new` and `receive` |
| A bare single-line text field needs `EntityInputHandler`, `ElementInputHandler`, focus, text shaping, selection, UTF-16 ranges, editing actions, and clipboard code. The adapted GPUI example is 715 lines. It accepts typed text in this test. | Measured | Test program, `src/bare_input.rs`, adapted from GPUI 0.3.12 `examples/input.rs`; `src/self_test.rs` |
| The widget form uses `input::InputState::new(window, cx)` in an entity, then `Input::new(&state)`. `InputState::value()` reads the label. Call `gpui_kit::init` before opening the window. Use `gpui_kit::open_window`, or mount `gpui_kit::base::Root` yourself. Set a text colour that contrasts with the field background. | Measured | Test program, `src/main.rs`, widget branches; `src/self_test.rs`; native window inspection |
| Four clickable colour samples are sufficient for a small colour control. A full picker is not necessary for this test. | Measured | Test program, `src/main.rs`, colour row; `src/self_test.rs` |
| `App::prompt_for_paths(PathPromptOptions)` opens the native file dialog. Set `files = true`, `directories = false`, and `multiple = false`. Its receiver gives `Result<Result<Option<Vec<PathBuf>>>>`: cancellation can give `Ok(Ok(None))`. The options have no extension filter. Validate the selected image after the dialog. | Measured | Test program, `src/main.rs`, `dialog`; published GPUI `src/platform.rs` |
| Both forms selected a PNG through the native dialog, decoded it on `background_executor`, and displayed it. The logs contain `DIALOG_LOADED`. | Measured | Test program, `bare-run.log`, `widgets-run.log`; native dialog test |
| In the combined test manifest, GPUI macros find the optional `gpui-kit` dependency even with its feature off. The bare form needs a crate alias for that test arrangement. A separate bare manifest need not have this arrangement. | Source; Measured (alias) | Published `gpui-pre-macros` 0.3.12 `src/gpui_pre_facade_paths.rs`; test program, `src/main.rs` |

## Application checks (2026-11-11)

| Fact | Confidence | Source |
|---|---|---|
| The application uses `gpui-kit` 0.9.1. Its unbounded `async_channel` receiver wakes a `Context::spawn_in` task. The engine sink uses `try_send`, so it does not wait for the window. Events sent before window creation stay in the queue. From Phase 5, the receiver is in the `Shell` entity, in a task from `Context::spawn`, also when no window is open. A window gets each event from that entity through `Context::subscribe_in`. | Measured | `crates/app/src/main.rs`, `shell.rs`, `view.rs`; window tests |
| `InputState::set_value` does not emit `InputEvent::Change`. It resets selection and undo state. The application calls it when a surface is selected, not on each engine reply. A reply during typing does not change the field or send a new command. | Source; Measured | `gpui-base` 0.9.1 `input/base/state.rs`; application window tests |
| A Kit glob import with `test-support` also imports its `test` macro. This can make generated `#[test]` attributes expand again. Explicit imports prevent this. `debug_bounds` needs a static selector registered with `debug_selector`, not only an element ID. | Measured | Application test build and `crates/app/src/view/tests.rs` |
| The value of `PathPromptOptions::prompt` is the text of the default button of the macOS file dialog. The image dialog of OpenCraft shows **Select an image**. The options have no start folder: the dialog opened in the Documents folder in one test and in the Applications folder in a different test, thus macOS selects the folder. | Measured | Window list and window picture of the release build in the development bundle, 2026-11-12; new-user test, issue [#152](https://github.com/shantanugoel/OpenCraft/issues/152) |
| A native file dialog selected a transparent PNG. The engine read and copied it. A key face showed its label, colour, and image in the window and on the device. A bar panel also showed its label, colour, and image in both places. Removing all three parts cleared the bar panel in both places. The arrow stayed upright, with red on the left and blue on the right. | Measured | Release application; native window screenshots and USB camera pictures, kept outside the repository |
| The native window fits at 900 × 640 content pixels. A restart shows the saved profile. In the application of Phases 2 to 4, a normal window close ended the process and wrote the last label change. From Phase 5, a window close does not end the process: see [Application checks of Phase 5](#application-checks-of-phase-5-2026-11-12). | Measured | Release application; native screenshots, process check, and test profile file, kept outside the repository |
| The macOS platform quits with `NSApplication::terminate`; code after the run loop is not a sufficient stop hook. GPUI awaits `on_app_quit` tasks for at most 200 ms. | Source | `gpui-pre-macos` 0.3.12 `platform.rs`; `gpui-pre` 0.3.12 `app.rs`; `crates/app/src/main.rs` |

## Window checks of Phase 4 (2026-11-12)

| Fact | Confidence | Source |
|---|---|---|
| A second `hover` style on one element stops a debug build with the message "hover style already set". It is a `debug_assert`, so a release build keeps one of the two styles with no message. Give each element one hover style only. | Source; Measured (debug panic) | `gpui-pre` 0.3.12 `src/elements/div.rs`, `hover`; application window tests |
| An action from `Window::dispatch_action` does not get to an input that is not in the window yet, for example an input that the same click adds. `InputState::select_all` and `InputState::focus` operate on the entity directly, and the selected text is replaced by the next typed text. | Measured | Application window tests, `crates/app/src/view/pages.rs`, `edit_name` |
| `InputEvent::PressEnter` comes when the user presses Enter in a single-line `Input`. | Source; Measured | `gpui-base` 0.9.1 `input/base/state.rs`; application window tests |
| `deferred(element).with_priority(n)` paints an element after all other elements, and a higher number paints later. A menu with priority 2 over a full-window layer with priority 1 operates in the window tests. Each of the two needs `occlude`: with the default hitbox, a click also gets to the elements behind. | Source; Measured | `gpui-pre` 0.3.12 `src/elements/deferred.rs` and `src/elements/div.rs`, `occlude`; application window tests |
| `ClickEvent::click_count` gives the count of clicks of the mouse-up event. The page tabs use it for a double click. | Source | `gpui-pre` 0.3.12 `src/interactive.rs` |
| The no-display test platform does not show the result of a tooltip or of the visual style. The window tests of Phase 4 check the bounds of the elements and the commands only. The Kit test API can send a double click: see [Window tests of these parts](#window-tests-of-these-parts). | Inference; Measured (double click) | `TestAppContext` API; application window tests; `sidebar_renames_in_the_row` of the window layout test |

## Shell test of Phase 5 (2026-11-12)

**Measured.** A test program, not in the repository, uses `gpui-kit` 0.9.1
and has no engine. It opens no device. It makes a menu bar item with
`NSStatusItem` from `objc2-app-kit`, in the GPUI event loop. It opens and
closes one small window, changes the activation policy, and gets commands
from its menu, from a named pipe, and from signals. A helper program read
the state through the Accessibility API, and pressed the menu items, the
close button, and the Dock icon. Each result below is from 1 try, if the
row gives no count. The program ran in two forms: the executable in a small
bundle, started from a shell, and the bare executable from `cargo build`.
The bare form did these tests again with the same results: the start with
no window, the menu, the Dock icon, the click on the Dock icon, and Cmd-Q.
The computer is the same as in
[Release measurements](#release-measurements). The test of a start through
launchd is in [macos-start-at-login.md](macos-start-at-login.md).

### Menu bar item

| Fact | Confidence | Source |
|---|---|---|
| `NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength)` in the callback of `Application::run` makes the menu bar item. The callback runs on the main thread, thus `MainThreadMarker::new()` gives a marker. | Measured | Test program |
| A menu item calls a method of an Objective-C class that `objc2::define_class!` makes. The method sends a command on an `async_channel`, and a GPUI foreground task from `App::spawn` receives it. The menu items operated with a window open and with no window. | Measured | Test program; AX press on each item |
| While the menu is open, the GPUI foreground task continues. It did two commands from the pipe. One command changed the title of a menu item, and the menu showed the new title. | Measured | Test program log; AX titles |
| A menu item with no action and `setEnabled(false)` shows a line of text that the user cannot select. `setState(NSControlStateValueOn)` shows a check mark. | Measured | AX attributes |
| The menu bar item stays when the activation policy changes. | Measured | AX |
| The test needs no new crate. Its `objc2` 0.6.5, `objc2-foundation` 0.3.2, and `objc2-app-kit` 0.3.2 are the versions that GPUI uses in the lock file of the workspace. | Measured | The two lock files |
| The features of `objc2-app-kit` for the test are `std`, `NSStatusBar`, `NSStatusItem`, `NSStatusBarButton`, `NSButton`, `NSControl`, `NSView`, `NSResponder`, `NSMenu`, `NSMenuItem`, `NSCell`, `NSApplication`, and `NSRunningApplication`, with no default features. The features of `objc2-foundation` are `std`, `NSString`, `NSObject`, and `NSArray`. | Measured | Test program manifest; build |
| The button of the test shows a text title. The application shows an SF Symbol as a template image on the button: see [Application checks of Phase 5](#application-checks-of-phase-5-2026-11-12). | Measured | Test program |

### Windows, Dock icon, and activation

| Fact | Confidence | Source |
|---|---|---|
| With `QuitMode::Explicit`, the process continues after the last window closes, through the close button or through `Window::remove_window`. | Measured | Test program; AX close button |
| `Application::on_reopen` runs when the user clicks the Dock icon and no window is open. With a window open, GPUI does not call it, and macOS only activates the application. | Measured; Source | AX press on the Dock icon; `gpui-pre-macos` 0.3.12 `platform.rs`, `should_handle_reopen` |
| `App::set_activation_policy(ActivationPolicy::Accessory)` removes the Dock icon at run time. The window stays on the screen, and the application becomes inactive. | Measured | Dock items through AX; window list |
| An accessory application that is active is the front application, but the menu bar stays with the application that was active before. Thus its application menu is not visible. | Measured | `NSWorkspace` `frontmostApplication` and `menuBarOwningApplication` |
| `App::set_activation_policy(ActivationPolicy::Regular)` shows the Dock icon again. If the application is active at that time, the menu bar stays with the other application. | Measured: 3 of 3 tries | `menuBarOwningApplication` |
| `NSApplication::deactivate` before the change to the regular policy does not correct this, with an activation 0, 50, or 200 ms later. | Measured: 4 tries | `menuBarOwningApplication` |
| This sequence corrects it: activate the Dock application (bundle identifier `com.apple.dock`) with `NSRunningApplication::activateWithOptions`, set the regular policy, and call `App::activate(true)` 50 ms or 200 ms later. | Measured: 4 of 4 tries | `menuBarOwningApplication` |
| If the application is not active at the change to the regular policy, its menu bar shows when it becomes active. | Measured | `menuBarOwningApplication` |
| `Application::with_windowing(WindowingRequest::Headless)` sets the accessory policy before the run loop. During a start with no window, 30 checks of the Dock in the first seconds did not find the Dock icon. | Measured; Source | Dock items through AX; `gpui-pre-macos` 0.3.12 `platform.rs`, `run` |
| `App::activate(true)` with no window did not make the application active. | Measured | `NSRunningApplication` `isActive` |
| An accessory application does not show in the list of Cmd-Tab, also with a window open. With the regular policy, the same application shows in the list. | Measured: 1 try for each policy | The application of Phase 5; the items of the list `AXProcessSwitcherList` of the Dock process, read while Cmd-Tab showed the list |

### Quit

| Fact | Confidence | Source |
|---|---|---|
| The application of Phase 4 sets no application menu and binds no key to quit. In the test program with the same configuration, Cmd-Q did nothing. With an application menu that has a Quit item, and a key binding `cmd-q` to the same action, Cmd-Q quit the program. | Measured | Test program; `CGEventPostToPid` with Cmd-Q; `crates/app/src/main.rs` |
| Thus the missing menu and key binding are the cause of the result of Phase 4: Cmd-Q through System Events did not quit the application. | Inference | The row above |
| Cmd-Q quits through the GPUI key binding when the window has the keyboard focus, also with the accessory policy. | Measured | Test program |
| `App::quit` calls `NSApplication::terminate`. Then GPUI calls each `on_app_quit` closure and waits for the futures for at most 200 ms (see [Application checks](#application-checks-2026-11-11)). A future that needed 500 ms did not complete before the process stopped. Work of 500 ms in the closure itself, before the closure returns its future, completed, and then the process stopped. | Measured | Test program log |
| SIGTERM to a program with no signal handler stops it immediately. The `on_app_quit` closures do not run. | Measured | Test program log |
| A program can block SIGTERM with `pthread_sigmask` as the first step of `main`, before it starts other threads. Then one thread can wait with `sigwait` and send a quit command to the GPUI foreground task. With this, SIGTERM from `kill` and from `launchctl bootout` called `App::quit`, and the `on_app_quit` closures ran. | Measured | Test program log, 3 times |
| But a child process that `std::process::Command` starts gets the blocked signal mask. A `/bin/sleep` child of the program did not stop after SIGTERM. | Measured; Source | `ps -o stat`; `man 2 execve`: "Blocked signals remain blocked" |
| A signal handler from `libc::signal` that writes one byte to a pipe, and one thread that reads the pipe and sends the quit command, also called `App::quit` for SIGTERM and for SIGINT. A child process got the default action and stopped after SIGTERM. | Measured; Source | Test program log; `ps -o stat`; `man 2 execve`: a caught signal is "set to default action in the new process image" |
| A quit from the menu bar item with no window open calls `App::quit`, and the `on_app_quit` closures run. | Measured | Test program log |

### Footprint with no window

The procedure is the same as in [Release measurements](#release-measurements):
`footprint -p` and the processor time of the process each 5 seconds, for 60
seconds. The test program is a release build with thin LTO. It has the menu
bar item and 14 small elements in its window.

| Measurement | Physical footprint: minimum, median, maximum | Processor use, mean (100 % is one core) | Threads | Confidence |
|---|---|---|---|---|
| Window open | 36.50, 36.50, 36.84 MiB | 0.013 % | 5 | Measured |
| Window closed | 28.34, 29.16, 29.16 MiB | 0.004 % | 5 | Measured |
| Window open again | 34.00, 34.00, 34.13 MiB | 0.021 % | 7 | Measured |
| Window closed again | 28.92, 28.92, 28.92 MiB | 0.004 % | 6 | Measured |
| Window closed, after 5 more cycles of open and close | 29.56, 29.56, 29.56 MiB | 0.002 % | 6 | Measured |
| No window and accessory policy, after 3 windows | 30.81, 30.81, 30.81 MiB | 0.003 % | 8 | Measured |
| Bare executable, start with no window and the accessory policy, no window opened yet | 17.95, 18.00, 18.00 MiB | 0.003 % | 5 | Measured |

- After a window closes, the footprint is approximately 7 MiB less than with
  the window open. It does not go back to the value before the first window.
  (Measured)
- 5 more cycles of open and close added 0.6 MiB. This is a short test, not
  proof for many cycles. (Measured)

## Application checks of Phase 5 (2026-11-12)

**Measured.** The application is the release build of `opencraft` with the
shell of [decision 0020](../decisions/0020-menu-bar-item-and-dock-icon.md),
started from a shell as a bare executable. It used a copy of a real profile,
and the device was connected. A helper program read the state through the
Accessibility API and `CGWindowListCopyWindowInfo`, and pressed the menu
items, the close button, and the Dock icon. The menu bar was dark. Each
result is from 1 try, if the row gives no count.

| Fact | Confidence | Source |
|---|---|---|
| After the window closed, the process continued and had no window. 84 seconds after the close, the device showed the images of the profile. | Measured | Window list; process state; USB camera picture |
| "Open OpenCraft" in the menu of the menu bar item opened a new window. 250 ms after the press, the window showed the device as connected, and the labels and the colours of the profile. | Measured | Window screenshot |
| The menu showed "Device: connected", "Profile: Default", and the check mark of "Show the Dock icon". The text lines were not enabled. | Measured | AX titles and attributes |
| The button shows the SF Symbol `square.grid.3x2` as a template image, in white on the dark menu bar. Its accessibility description is "OpenCraft". | Measured | Screenshot of the item only; AX description |
| "Show the Dock icon" removed the Dock icon and set the accessory policy. The window stayed open. The application wrote `show_dock_icon = false` to `shell.toml`. | Measured | Dock items through AX; window list; file |
| With the accessory policy, the application active, and the window open, "Show the Dock icon" showed the Dock icon. After 1.2 s, the application was active and had the menu bar. | Measured | `menuBarOwningApplication` |
| A start with `show_dock_icon = false` opened the window with the accessory policy. 55 checks of the Dock in the first 6 seconds did not find the Dock icon. | Measured | Dock items through AX |
| A press on the Dock icon with no window open opened a new window. | Measured | AX press on the Dock item; window list |
| With the accessory policy and the window open behind a window of a different application, "Open OpenCraft" made the application active in 1 of 2 tries. A new window from "Open OpenCraft" made the application active in 2 of 2 tries, 1 with each policy. | Measured | `NSRunningApplication` `isActive`; window order |
| Each quit request ended the process in less than 1 second: "Quit OpenCraft" with no window and with a window, Cmd-Q, SIGTERM, SIGINT, and SIGHUP. Each time, the application printed that the engine stopped, after `EngineHandle::stop` returned. | Measured | Process state; standard error |
| Cmd-Q to the active application quit it with the accessory policy. A label change 30 ms before the Cmd-Q, and a label change directly before SIGTERM, were in the profile file after the quit. The save delay of the engine is 500 ms, thus the engine wrote the file at the stop. | Measured | `CGEventPostToPid` key events; profile file |
| A second start while the application ran printed "OpenCraft is already running." and ended with the exit status 0. This was the bare executable, before [decision 0028](../decisions/0028-second-start-opens-the-window.md). See [Second start](#second-start-2026-11-12). | Measured | Terminal output; exit status |

## Second start (2026-11-12)

**Measured.** The application is the release build in the development bundle
of `scripts/sign-dev.sh` (identifier `opencraft-dev`), with the change of
[decision 0028](../decisions/0028-second-start-opens-the-window.md). The
device was connected. A helper program read the window list with
`CGWindowListCopyWindowInfo`, and closed the window with a click on its close
button. After each second start, it read the process list each 0.25 s for
2 s, to find a second process. Each result is from 1 try.

| Fact | Confidence | Source |
|---|---|---|
| GPUI answers `applicationShouldHandleReopen:hasVisibleWindows:` with YES. It calls the `on_reopen` closure only when `hasVisibleWindows` is NO. | Source | `gpui-pre-macos` 0.3.12, `src/platform.rs`, `should_handle_reopen` |
| `open OpenCraft.app` while the bundle ran, with its window closed, started no second process. The window of the process that ran opened. The log file had no line about a signal, thus the reopen event opened the window. | Measured | Process list; window list; log file |
| `open --env NAME=value` to a bundle that runs prints "was already running and so the additional environment variables could not be set", and starts no second process | Measured | Terminal output |
| The same occurred with the Dock icon hidden (`show_dock_icon = false`, accessory policy) | Measured | Process list; window list |
| A click on the Dock icon, with the window closed, opened the window | Measured | AX press on the Dock item; window list |
| The executable in the bundle, started from a terminal, counts as the running bundle for macOS. A later `open OpenCraft.app` started no second process, and the window opened. | Measured | Process list; window list |
| A start of the executable from a terminal while OpenCraft ran started a second process. That process stopped at the lock, sent SIGUSR1, and wrote "OpenCraft is already running. It shows its window now.". The first process wrote that it got the signal, and its window opened. This was the same with the bundle and with the debug build. | Measured | Terminal output; log file; window list |
| A start with `--login` while OpenCraft ran stopped at the lock, sent no signal, and opened no window | Measured | Terminal output; window list |

## Device settings area (2026-11-12)

| Fact | Confidence | Source |
|---|---|---|
| `SliderState` emits `SliderEvent::Change` for each move of a drag and for a click on the track, and `SliderEvent::Release` one time at the end of a drag. `SliderState::set_value` emits no event. The minimum is set when the state is made, so a different minimum needs a new `SliderState`. | Source; Measured | `gpui-base` 0.9.1 `src/slider.rs`; application window tests |
| In the release build, a drag of the brightness slider with 40 mouse moves in approximately 0.5 s changed the device. The window showed 15 % and 100 %, the settings file had the same values, and the camera saw the display brightness change. | Measured: 3 drags | Window screenshot; `device-settings.toml`; USB camera |

## Window layout test (2026-11-12)

**Measured.** A test program, not in the repository, makes the seven parts
of the new window layout of
[issue 175](https://github.com/shantanugoel/OpenCraft/issues/175). It uses
`gpui-kit` 0.9.1 with a copy of the lock file of the workspace, and no
OpenCraft crate. It opens no device. It has 17 window tests on the
no-display test platform, and all 17 pass. A helper program also operated
the real window with Quartz mouse and key events, read the menu bar through
the Accessibility API, and took pictures of the window only. The computer is
the same as in [Release measurements](#release-measurements). The window
used the dark theme of the kit. The source column names the files and the
tests of the test program: `src/spike.rs`, `src/tests.rs`, and
`src/main.rs`. The pictures are kept outside the repository.

In the tables, `component::` is `gpui_kit::component::`. The pattern blocks
are short forms of the code of the test program.

### Window tests of these parts

| Fact | Confidence | Source |
|---|---|---|
| The Kit test API `gpui_kit::test::{TestWindowExt, TestEventExt}` operates on the `Window` of a `VisualTestContext` from `TestAppContext::add_window_view`. `find`, `try_find`, `click`, `double_click`, `right_click`, `drag`, `drag_to`, `press`, and `input` send real window events with real hit testing. `pointer_move`, `pointer_down`, and `pointer_up` give the steps of a drag. | Measured | `src/tests.rs`; Kit `TESTING.md` |
| `find` gets an element only if the element has `.id(...)` and `.test_support()` (`gpui_kit::base::TestSupportExt`). Some Kit parts register their own IDs: each tab of a `TabBar` is `Integer(ix)` in the scope of the bar ID, a `Toggle` uses its own ID, and a drawn menu is `"popup-menu"` with its items as `Integer(ix)`. | Measured | `src/tests.rs` |
| `within(id)` stops the test when two elements in one path have the same ID. `TabBar::new("tabs")` has this problem, because the inner `Tabs` element also uses the ID `"tabs"`. Give the bar a different ID, for example `"right-tabs"`. | Measured | `tabs_change_the_content` |
| The test window is not active. GPUI then sends an empty previous focus path to the focus listeners, so `InputEvent::Blur` does not come. Call `window.activate_window()` before a test that needs a blur. | Source; Measured | `gpui-pre` 0.3.12 `src/window.rs`, `draw`; `sidebar_renames_in_the_row` |
| The test window opens at a size that is not the size of `simulate_resize`. Thus each test has a window size change at its start. | Measured | `open` in `src/tests.rs` |
| The test platform shows no pixels. The drag picture, the colour of a highlight, the look of a popover, and the native menu need a check in the real window. | Measured | All parts below |

### 1. Sidebar with a tree of two levels

| Fact | Confidence | Source |
|---|---|---|
| Plain `div` rows make a tree of two levels: a profile row with a disclosure triangle, and page rows below it. A click on a row selects it. A click on the triangle opens or closes the profile, and does not change the selection, because its handler calls `cx.stop_propagation()`. | Measured | `Spike::sidebar_row`; `sidebar_selects_rows_and_the_triangle_alone_expands`; real window |
| The triangle is `Icon::new(IconName::ChevronRight)`, and `.rotate(gpui_kit::percentage(0.25))` turns it down when the profile is open. The icon needs an asset source: see part 3. | Measured | Real window picture |
| In the real window, a double click on a row gave `ClickEvent::click_count()` 2, and the edit field opened. | Measured | Real window; event log |
| Edit of a name in the row: put `Input::new(&state)` in the row in place of the label. At the start, call `set_value`, `select_all`, and `focus` on the `InputState` entity. The old name was selected, and typed text replaced it. Enter (`InputEvent::PressEnter`) and a click out of the field (`InputEvent::Blur`) keep the new name. A `capture_key_down` handler on the row gets Escape before the field and cancels. | Measured | `start_rename`, `commit_rename`, `cancel_rename`; `sidebar_renames_in_the_row`; real window pictures |
| After the edit, give the focus back to the workspace element. The key bindings of part 6 need the workspace element in the focus path. | Measured (with the focus call); Inference (without it) | `commit_rename`; `sidebar_renames_in_the_row` |
| A right click can show a native AppKit menu: `component::native_menu::NativeMenu::new().menu("Rename", Box::new(Rename)).separator().show(event.position, window, cx)` in `on_mouse_down(MouseButton::Right, ...)`. The menu opened at the pointer. Arrow keys with Return, and a mouse click, chose an item. | Measured | `MenuKind::Native`; real window picture; event log |
| A `NativeMenu` item sends its action with `Window::dispatch_action`, thus to the focused element. Focus the workspace element before `show`, and keep the row of the click in the view state. The workspace handler then knows the row. | Source; Measured | `gpui-component` 0.9.1 `src/native_menu/mod.rs`; real window |
| The window tests use a drawn menu: `ContextMenuExt::context_menu` with `PopupMenuItem::new(label).on_click(...)`. It opens at the pointer. The test did not call `NativeMenu::show` on the no-display platform. | Measured | `MenuKind::Drawn`; `sidebar_drawn_context_menu_operates` |
| The Kit tree (`component::tree::{tree, TreeState, TreeItem}`) has arrow keys in the key context `"Tree"`, accessibility roles, and a virtual list. But a click on any part of a folder row selects the row and also opens or closes it. It has no public method that opens one folder, `set_items` clears the selection, and its only events are `TreeEvent::Expanded` and `TreeEvent::Collapsed`. | Source; Measured (row click) | `gpui-base` 0.9.1 `src/tree.rs`; `kit_tree_toggles_a_folder_on_a_row_click` |
| `component::sidebar::SidebarMenuItem` has a trailing caret, `click_to_toggle`, `active`, and `context_menu`. Its label is text only, thus it cannot hold an edit field. | Source | `gpui-component` 0.9.1 `src/sidebar/menu.rs` |
| A selection that is an index into the rows becomes incorrect after an insert: after "Duplicate", the new copy showed as selected. Keep the selection as a stable ID. | Measured | Real window picture |

```rust
// A profile row. `row` is Row::Profile(p) or Row::Page(p, i).
div().id(row_id).test_support().h(px(26.)).flex().items_center().rounded_md()
    .when(selected, |d| d.bg(theme.accent))
    .child(div().id(("disclosure", p)).test_support()
        .child(Icon::new(IconName::ChevronRight).when(open, |i| i.rotate(percentage(0.25))))
        .on_click(cx.listener(move |v, _, _, cx| { cx.stop_propagation(); v.toggle(p, cx) })))
    .child(if editing { Input::new(&v.rename).xsmall().into_any_element() } else { name })
    .on_click(cx.listener(move |v, e: &ClickEvent, window, cx| {
        v.select(row, cx);
        if e.click_count() == 2 { v.start_rename(row, window, cx) }
    }))
    .on_mouse_down(MouseButton::Right, cx.listener(move |v, e: &MouseDownEvent, window, cx| {
        v.context_row = Some(row);
        window.focus(&v.focus, cx);
        NativeMenu::new().menu("Rename", Box::new(Rename)).show(e.position, window, cx);
    }))
```

**Window test.** `double_click(row_id)`, then `input("Intro")` and
`press("enter")` or `press("escape")`. For a click out, call
`activate_window` first. For the menu, use the drawn menu:
`right_click(row_id)`, then `within("popup-menu").click(1usize)`.

### 2. Three panes with dividers

| Fact | Confidence | Source |
|---|---|---|
| `component::h_resizable(id).with_state(&Entity<ResizableState>)` with three `component::resizable_panel()` children makes the panes. `.size(px)` gives the first width, `.size_range(min..max)` the limits, and `.visible(bool)` hides a pane. The default minimum is 100 px. | Source; Measured | `gpui-base` 0.9.1 `src/resizable/`; `Spike::render` |
| A drag of a divider changes the width and stops at the limits: +60 px gave 280, a long drag right gave the maximum 320, and a long drag left gave the minimum 180. The right divider also operates. `on_resize` runs at the mouse-up, and `ResizableState::sizes()` gives the widths. | Measured | `panes_resize_with_the_divider_and_keep_the_limits`; real window event log |
| In the real window, the divider shows a short bar when the pointer is on it, and a longer bar during the drag. | Measured | Real window pictures |
| Without `.fixed()`, a change of the window size changes all panes in proportion. With `.fixed()` on the two side panes, only the centre pane changes: at 900 and 1400 px the widths were 220, 380, 300 and 220, 880, 300. | Measured | `panes_keep_the_side_widths_when_the_window_changes_size`, `panes_with_no_fixed_sides_change_with_the_window` |
| Without `.fixed()`, `sizes()` can hold a width below the minimum. The panel applies the limits only when it draws. Thus clamp a width from `sizes()` before you save it. | Measured | Same tests: stored 128.3 px, drawn 180 px |
| `.visible(false)` keeps the index and the width of the pane. After hide and show, the pane had its last width again, also after a divider drag. | Measured | `panes_hide_and_show`; real window pictures |
| GPUI does not stop the window at the sum of the pane widths. At 760 px, with fixed side panes of 220 and 300 px and a centre minimum of 320 px, the right pane went 80 px out of the window. Set `window_min_size` to the sum of the shown panes, or hide a side pane when the window is too narrow. | Measured | `side_widths_after_resize`; real window picture |
| View menu items that send `ToggleSidebar` and `ToggleInspector` operate. The native menu showed the key equivalents of the key bindings ⌃⌘S and ⌥⌘I, also when the bindings have a key context. | Measured | Real window; AX `AXMenuItemCmdChar` and `AXMenuItemCmdModifiers` |

```rust
h_resizable("panes").with_state(&self.panes)
    .on_resize(|state, _, cx| save(state.read(cx).sizes()))
    .child(resizable_panel().size(px(220.)).size_range(px(180.)..px(320.))
        .visible(self.show_sidebar).fixed().child(sidebar))
    .child(resizable_panel().size_range(px(320.)..Pixels::MAX).child(canvas))
    .child(resizable_panel().size(px(300.)).size_range(px(260.)..px(420.))
        .visible(self.show_inspector).fixed().child(inspector))
```

**Window test.** Get the right edge of the sidebar from
`find("sidebar").bounds()`, then `window.drag(edge, edge + dx)`. Read the
widths from `bounds()` of each pane or from `ResizableState::sizes()`. Use
`simulate_resize` for a window size change.

### 3. Popover and modal layer

| Fact | Confidence | Source |
|---|---|---|
| `component::popover::Popover::new(id).trigger(button).content(...)` opens below the trigger with its left edge on the left edge of the trigger (`Anchor::TopLeft`, the default) and a gap of 0.25 rem. It painted above the sidebar and the pane divider. | Measured | `popover_opens_below_its_button_and_closes_on_a_choice`; real window picture |
| `.open(bool)` with `.on_open_change(...)` makes the popover controlled. Escape and a click out of the popover close it and call `on_open_change`. A click on an item of the content does not close it: set the state to closed in the item handler. | Measured | Same test; real window event log |
| The `content` closure runs on each draw. Do not make entities in it. | Source | `gpui-component` 0.9.1 `src/popover.rs` |
| Kit icons need an asset source: `gpui_kit::application().with_assets(gpui_kit::assets::Assets)`. The test program has it, and the icons showed. The application registers only the icons that the window uses: see [Three panes in the application](#three-panes-in-the-application-2026-11-13). | Measured | `src/main.rs`; real window picture |
| `WindowExt::open_dialog(cx, \|dialog, window, cx\| ...)` shows a modal layer. The Kit `Root` draws the layer. The Root of `gpui_kit::open_window`, and the Root that the application makes, are sufficient. | Measured | `open_palette`; real window |
| When the layer opens, the Root moves the focus into it. The workspace key bindings stopped: ⌃⌘S did nothing. The View menu items were not enabled while the layer was open. A click out of the dialog closed it, and the element below did not get the click. Escape closed it and gave the focus back to the earlier element. | Measured | `command_palette_is_modal_and_has_the_focus`; real window; AX `enabled` |
| The background did not become darker in the picture of the dark theme. | Measured | Real window picture |
| The command palette is `component::command::{Command, CommandItem, CommandState}` in the dialog. Call `CommandState::focus` one time, on the first draw of the dialog. Typing filters the items. Up, Down, Enter, and Escape operate (key context `"Command"`). Escape clears a query first, and closes the dialog at the second press. | Measured | `open_palette`; real window pictures |
| The filter runs in a task. In a window test, call `run_until_parked` after `input` and before `press("enter")`. | Measured | `command_palette_is_modal_and_has_the_focus` |
| `CommandItem::action` sends the action to the focus in the dialog. The workspace element is not in that path, so the workspace does not get the action. Use `on_confirm`: close the dialog, focus the workspace, then call `window.dispatch_action` again. With this, the palette showed and hid the panes. | Source; Measured | `gpui-component` 0.9.1 `src/command/state.rs`, `confirm`; `open_palette` |
| A `CommandState` keeps its query between two openings. Make a new state for each opening, as the Kit example of `open_dialog` does, or call `set_query("")`. Both operated. | Measured | `open_palette`, `SPIKE_REUSE_PALETTE`; real window |
| The palette showed no key equivalent next to an item. It looks for the binding from the focus in the dialog, and the bindings have the key context `"Workspace"`. | Source; Measured | `src/command/state.rs`, `item_binding`; real window picture |

```rust
pub fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let palette = cx.new(|cx| CommandState::new(window, cx));
    let first = Cell::new(true);
    let view = cx.entity();
    window.open_dialog(cx, move |dialog, window, cx| {
        if first.replace(false) { palette.update(cx, |s, cx| s.focus(window, cx)) }
        let view = view.clone();
        dialog.w(px(480.)).margin_top(px(80.)).close_button(false).child(
            Command::new(&palette).items(items()).on_confirm(move |ix, window, cx| {
                window.close_dialog(cx);
                view.update(cx, |v, cx| window.focus(&v.focus, cx));
                window.dispatch_action(action_of(ix.row), cx);
            }))
    });
}
```

**Window test.** `click("icon-button")`, then compare `find("icon-grid")`
and `find("icon-button")` bounds. For the dialog: `press("cmd-k")`,
`window.has_active_dialog(cx)`, and
`palette.read(cx).focus_handle(cx).contains_focused(window, cx)`.

### 4. Drag and drop from the right pane to the centre pane

| Fact | Confidence | Source |
|---|---|---|
| The source row has `.on_drag(value, \|value, offset, window, cx\| cx.new(\|_\| Preview { .. }))`. The target has `.drag_over::<T>(\|style, value, window, cx\| style.bg(..).border_color(..))` and `.on_drop(cx.listener(\|view, value: &T, window, cx\| ..))`. The drop handler got the data of the row. | Measured | `Spike::library`, `Spike::canvas`; `drag_from_the_library_to_a_key`; real window |
| A drag starts when the pointer moves more than 2 px with the left button down. A shorter move is a click. | Source; Measured | `gpui-pre` 0.3.12 `src/elements/div.rs`, `DRAG_THRESHOLD`; same test |
| GPUI puts the top-left corner of the drag picture at the pointer minus `offset`, and `offset` is the point of the press in the source element. A small picture from a wide row thus shows far from the pointer: 149 px to the left in the picture. Add a padding of `offset` in the picture, and the picture stays at the pointer. | Source; Measured | `src/window.rs`, `cursor_offset`; real window pictures with and without `SPIKE_PLAIN_PREVIEW` |
| `drag_over` uses the position of the pointer, not of the picture. | Measured | Real window picture |
| A drop out of all targets sends no event and changes nothing. | Measured | `drag_from_the_library_to_a_key` |
| The `accent` colour of the dark Kit theme is grey, so a highlight with it was hard to see. A blue (`0x0a84ff`) border and background showed clearly. | Measured | Real window pictures |
| One element can have one `on_drag` only. | Source | `src/elements/div.rs`, `debug_assert` |

```rust
// Source row in the library.
div().id(("action", i)).on_drag(ActionDrag { name }, |drag, offset, _, cx| {
    cx.new(|_| DragPreview { name: drag.name.clone(), offset })
})
// The picture: padding keeps the pill at the pointer.
div().pl(self.offset.x - px(8.)).pt(self.offset.y - px(8.)).child(pill)
// Target in the canvas.
div().id(("key", k))
    .drag_over::<ActionDrag>(|s, _, _, _| s.bg(rgba(0x0a84ff33)).border_color(rgb(0x0a84ff)))
    .on_drop(cx.listener(move |v, d: &ActionDrag, window, cx| v.drop_on(k, d, window, cx)))
```

**Window test.** `drag_to("action-3", "key-0")`, or `pointer_down`, eight
`pointer_move` steps with `Some(MouseButton::Left)`, and `pointer_up`.
`cx.has_active_drag()` is true during the drag. The test cannot see the
picture or the highlight.

### 5. Controls in a two-column grid

| Fact | Confidence | Source |
|---|---|---|
| `component::form::{h_form, field}` makes the two columns: `h_form().label_width(px(84.)).child(field().label("Size").child(control))`. The left edges of all five controls were at the same x. | Measured | `Spike::inspector`; `controls_in_the_form` |
| The form puts the labels on the left side of the label column. `label_fn` with `text_right` did not move the text, because the label is in a container of its own width. A label element with a fixed width and `flex().justify_end()` was on the right side. | Source; Measured | `gpui-component` 0.9.1 `src/form/field.rs`; real window pictures |
| Segmented control: `ToggleGroup::new(id).segmented().single()` with `Toggle::new((id, i)).label(..).checked(..)` and `.on_click(\|checked: &Vec<bool>, ..\|)`. A click on the checked item clears all items, so keep the old value when no item is checked. In the dark theme, it had no outline. | Measured | `controls_in_the_form`; real window picture |
| Two other forms look more like a macOS segmented control: `ButtonGroup::new(id).outline()` with `Button::new(..).label(..).selected(..)` (import `component::Selectable`), and `TabBar::new(id).segmented()`. Both operated. | Measured | Real window picture; event log |
| Switch: `Switch::new(id).checked(b).on_click(\|checked: &bool, ..\|)`. The owner keeps the value. | Measured | `controls_in_the_form`; real window |
| Slider: `Slider::new(&state)` with `SliderState::new().min(..).max(..).step(..).default_value(..)`. A drag gave `SliderEvent::Change` for each move and one `Release`. | Measured | `controls_in_the_form`; real window event log |
| Text field: `Input::new(&state).small()`. The input of "Mail" gave four `InputEvent::Change` events, and `value()` in the subscriber gave "Mail" each time, because the events were in a queue. Use the event, not a count of events. | Measured | `controls_in_the_form` |
| Popup button: `Select::new(&state).id("font").small()` with `SelectState::new(vec_of_shared_strings, Some(IndexPath::new(0)), window, cx)`. `SelectEvent::Confirm(Option<value>)` comes after a choice. The list opened below the field, with a check mark at the current item. Down, Down, Enter chose the third item in the test, and a click chose it in the real window. | Measured | `controls_in_the_form`; real window picture |
| `div().grid().grid_cols(n)` also makes a grid. The test program uses it for the keys and the icons. | Measured | `Spike::canvas`, `icon_popover` |

```rust
h_form().label_width(px(84.))
    .child(field().label("Title").child(Switch::new("show-title").checked(on)
        .on_click(move |c: &bool, _, cx| view.update(cx, |v, cx| v.set_title(*c, cx)))))
    .child(field().label("Size").child(Slider::new(&self.size)))
    .child(field().label("Font").child(Select::new(&self.font).id("font").small()))
    .child(field()
        .label_fn(|_, _| div().w(px(72.)).flex().justify_end().child("Style"))
        .child(ButtonGroup::new("style").outline().small().children(buttons)))
```

**Window test.** `click(("style", 2usize))`, `click("show-title")`, a
`drag` across the slider bounds, `click` on the field and `input("Mail")`,
`click("font")` and `press("down")`, `press("enter")`. Compare
`bounds().origin.x` for the column.

### 6. Key commands in the window

| Fact | Confidence | Source |
|---|---|---|
| Pattern: `actions!(ns, [..])`, `cx.bind_keys([KeyBinding::new("cmd-z", Undo, Some("Workspace")), ..])`, and on the root element of the view `.key_context("Workspace").track_focus(&focus).on_action(cx.listener(..))`. Focus that element when the window opens, and after a click on the canvas, an edit, or a dialog. ⌘K, ⌘Z, ⇧⌘Z, ⌘C, ⌘V, Delete (`backspace`), and forward delete (`delete`) operated in the tests and in the real window. | Measured | `src/main.rs`, `bind_keys`; `key_commands_on_the_canvas`; real window event log |
| A binding with no key context gets the depth of the deepest context. It then has the same depth as the bindings of a text field (key context `"Input"`), and the binding that was added last wins. With bindings of no context, a focused field lost Backspace, ⌘Z, ⌘C, and ⌘V to the workspace: Backspace removed the action of the selected key. With the key context `"Workspace"`, the field kept all four. | Source; Measured | `gpui-pre` 0.3.12 `src/keymap.rs`, `binding_enabled`; `a_focused_text_field_keeps_its_keys_with_global_bindings` and `..._with_context_bindings`; real window |
| ⌘K opened the palette also from a focused field, because the field has no binding for it. | Measured | Same tests |
| The Kit `Root` binds ⌘C in the key context `"Root"` for text selection. A binding in `"Workspace"` is deeper and wins. | Source; Measured | `gpui-base` 0.9.1 `src/root.rs`; `key_commands_on_the_canvas` |
| `cx.set_menus` with `MenuItem::action("Undo", Undo)` shows the key equivalents of the bindings in the Edit menu. A key press in a focused field goes to the field and not to the menu. | Measured | Real window; AX attributes; event log |
| But a click on a menu item sends the workspace action also when a field has the focus. Edit > Undo undid the last change of the canvas while the user typed. In the workspace handler, if `window.focused_input(cx)` (`component::WindowExt`) is `Some`, send `component::input::{Undo, Redo, Copy, Paste, Delete}` to the field. With this, Edit > Undo, Redo, and Delete changed the text of the field. | Measured | `forward_to_field`; real window event log |
| The clipboard is `cx.write_to_clipboard(ClipboardItem::new_string(text))` and `cx.read_from_clipboard().and_then(\|c\| c.text())`. | Measured | `Spike::copy`, `Spike::paste` |

```rust
cx.bind_keys([
    KeyBinding::new("cmd-k", CommandPalette, Some("Workspace")),
    KeyBinding::new("cmd-z", Undo, Some("Workspace")),
    KeyBinding::new("cmd-shift-z", Redo, Some("Workspace")),
    KeyBinding::new("backspace", DeleteSelection, Some("Workspace")),
]);
div().key_context("Workspace").track_focus(&self.focus)
    .on_action(cx.listener(|v, _: &Undo, window, cx| {
        if window.focused_input(cx).is_some() {
            window.dispatch_action(Box::new(input::Undo), cx); // from the Edit menu
        } else {
            v.undo(cx)
        }
    }))
```

**Window test.** `press("cmd-shift-z")` and the other keys. For a field,
call `state.update(cx, |s, cx| s.focus(window, cx))`, then `input` and
`press`, and check that the workspace state did not change. A window test
does not use the native menu: test the menu items in the real window.

### 7. Tabs in a pane

| Fact | Confidence | Source |
|---|---|---|
| `component::tab::{TabBar, Tab}`: `TabBar::new("right-tabs").segmented().small().selected_index(ix).child(Tab::new().label("Inspector")).on_click(cx.listener(\|view, ix: &usize, ..\|))`. The owner keeps the index. An `on_click` on the bar replaces the handlers of the tabs. | Source; Measured | `Spike::right_panel`; `tabs_change_the_content`; real window |
| The bar has the forms `segmented`, `underline`, `pill`, `outline`, and `folder`. | Source | `gpui-component` 0.9.1 `src/tab/tab_bar.rs` |

**Window test.** `within("right-tabs").click(1usize)`, then
`within("right-tabs").find(1usize).selected()`.

### Real window checks

| Fact | Confidence | Source |
|---|---|---|
| `screencapture -x -o -l <id>` gives a picture of one window at 1 pixel for each point. A native menu is a different window of the process, at layer 101. | Measured | Helper program; window list |
| A Quartz key event takes the modifier state of the earlier event if its flags are not set. After ⌘K, typed letters were Command keys, and no text came. Set the flags of each key event, also to 0. | Measured | Helper program |
| System Events can read the items of a menu (`enabled`, `AXMenuItemCmdChar`, `AXMenuItemCmdModifiers`) and click an item. | Measured | Helper program |

## Three panes in the application (2026-11-13)

**Measured.** The application is the release build of `opencraft` with the
three panes of
[issue 176](https://github.com/shantanugoel/OpenCraft/issues/176), on the
computer of [Release measurements](#release-measurements), with a copy of a
real profile and the device connected. A helper program operated the real
window with Quartz events and System Events, and took pictures of the window
only. The pictures are kept outside the repository.

| Fact | Confidence | Source |
|---|---|---|
| `gpui_kit::assets::icon_assets!(pub WindowIcons, [PanelLeft, ...])` makes an asset source with only the listed icons, and `with_assets(WindowIcons)` shows them. An icon that is not in the list shows nothing. No crate came into `Cargo.lock`. | Measured | `crates/app/src/view/theme.rs`, `WindowIcons`; real window picture |
| `gpui_kit::component::IconName` has only the icons of the components. `gpui_kit::assets::IconName` has all Lucide icons, for example `Grid3x2`. `component::Icon::new` accepts both. | Source; Measured (build error with `component::IconName::Grid3x2`) | `gpui-component` 0.9.1 `src/icon.rs` |
| `TitlebarOptions { appears_transparent: true, traffic_light_position: Some(point(px(18.), px(19.))), .. }` puts the centre of the window buttons approximately 26 below the top of the window, and the close button approximately 25 from the left edge. | Measured | Window picture |
| With `WindowOptions::app_owns_titlebar_drag`, `Window::titlebar_double_click` in a mouse-down handler with `click_count` 2 zoomed the window and zoomed it back. | Measured | Window list |
| `Window::start_window_move` in a mouse-move handler during a drag wrote "Window move completed without beginning" for drags from Quartz events, and the window did not move. A drag with Quartz events on the title bar of a Finder window also did not move that window. Thus a test with Quartz events cannot show if the window moves with a real mouse. | Measured; Inference (the cause) | Window list; standard error |
| System Events `set size of window 1` to 900 × 600 gave 1080 × 700: the window stops at `window_min_size`. | Measured | Window list |
| A divider made of an element with `on_drag` and `cursor_col_resize`, and a parent with `on_drag_move` (capture phase, for all pointer positions) and `on_drop`, changes the width of a pane. A width in the view state, with limits, needs no `ResizableState`. | Measured | `crates/app/src/view/panes.rs`; window tests; real window picture |
| The View menu items that `cx.set_menus` sets again after each change show the new titles ("Show Sidebar", "Hide Inspector"). A click on an item through System Events did the command. | Measured | AX menu item names; window pictures |
| A key binding in the key context `"Workspace"` does nothing when no element has the focus. The application focuses the root element when the window opens, and after a click that takes the focus from a text field, in place of `Window::blur`. | Inference; Measured (the window tests with the focus in the root element) | `gpui-pre` 0.3.12 `src/window.rs`, dispatch path; `crates/app/src/view.rs`, `focus_workspace` |
| `TestPlatform::get_menus` gives the menus of `set_menus`, thus a window test can read the titles of the View menu. | Source; Measured | `gpui-pre` 0.3.12 `src/platform/test/platform.rs`; `view/tests/panes.rs` |

## Gesture cards in the inspector (2026-11-13)

**Measured.** The application is the debug build of `opencraft` with the
gesture cards of
[issue 179](https://github.com/shantanugoel/OpenCraft/issues/179), on the
computer of [Release measurements](#release-measurements), with a copy of a
real profile and the device connected. A helper program operated the real
window with Quartz events and took pictures of the window only. The
pictures are kept outside the repository.

| Fact | Confidence | Source |
|---|---|---|
| The text style of GPUI has no letter spacing. A section header of decision 0029 (letter spacing 0.06 em) has the normal spacing of the font. | Source | `gpui-pre` 0.3.12 `src/style.rs`, `TextStyle` |
| `Input::new(&state).appearance(false)` draws no background, no border, and no focus ring of the kit. A frame element around it can draw the field with the sizes and colours of the window, and the frame gets the focus border when `state.focus_handle(cx).is_focused(window)`. | Source; Measured | `gpui-component` 0.9.1 `src/input/input.rs`, `render`; `crates/app/src/view/controls.rs`, `text_field`; window picture |
| The single-line input keeps the vertical padding of its size after `.h(..)`. With `.h(px(26.))` and no `.py_0()`, the text was approximately 3 below the centre of a field of 28. With `.py_0()` and a frame with `items_center`, the text was in the centre. | Measured | Window pictures |
| `Textarea::new(&state).small()` has the editor padding 8 at the sides and 2 at the top and the bottom. A padding on the element adds to it. | Source; Measured | `gpui-component` 0.9.1 `src/sizing.rs`, `input_px` and `input_py`; window picture |
| `anchored()` with no position uses its own place in the layout as the anchor. With `.offset(point(px(0.), h))` from the top of a button of height `h`, the element shows below the button. If it does not fit below, the default fit mode `SwitchAnchor` uses the other corner at the anchor and drops the offset, so the element shows above the button and does not cover it. With `SnapToWindowWithMargin`, the element moved up and covered the button. | Source; Measured | `gpui-pre` 0.3.12 `src/elements/anchored.rs`, `prepaint`; window pictures |
| A menu in `deferred(anchored()..)` inside the scrolled inspector moves with the scroll position of its button. A second `deferred` element at the window origin, of the size of the viewport, with `occlude()` and `on_mouse_down`, closes the menu on a click out of it. The element below does not get the click. | Measured | `crates/app/src/view/controls.rs`, `popup_menu`; `a_menu_closes_with_escape_and_with_a_click_out_of_it`; window pictures |
| `window.text_system().shape_line(text, size, &[run], None).width` gives the width of a label before the layout. The inspector uses it to put a label that is wider than the label column above its control. | Source; Measured | `gpui-pre` 0.3.12 `src/text_system.rs`; `crates/app/src/view/form.rs`, `fitted_row`; window picture |
| `cx.debug_bounds(id)` of `VisualTestContext` takes a `&'static str`. A test cannot give it an identifier from `format!`. | Source | Build error in `view/tests/actions.rs` |
| In a window test, a click goes to the centre of the element. The centre of the head row of a gesture card is on its popup button, so the test clicks the gesture name (`{id}-name`) to open the card. | Measured | `cards_open_set_a_kind_change_a_setting_and_clear_an_action` |

## Sidebar in the application (2026-11-13)

**Measured.** The application is the debug build of `opencraft` with the
sidebar of [issue 177](https://github.com/shantanugoel/OpenCraft/issues/177),
on the computer of [Release measurements](#release-measurements), with a
scratch copy of the profiles (two profiles, three pages) and the device
connected. A helper program operated the real window with Quartz events and
took pictures of the window only. The pictures are kept outside the
repository. The window tests are in `crates/app/src/view/tests/sidebar.rs`.

| Fact | Confidence | Source |
|---|---|---|
| In the application, a double click on a page row and on a profile row gave `ClickEvent::click_count()` 2, and the name field opened in the row. | Measured | Window pictures |
| A Kit `Input` with one line handles Return with `cx.propagate()` and then emits `InputEvent::PressEnter`. On the test platform, the window then put the key into the text: a selected name became empty before the subscriber read it. A `capture_key_down` handler on the parent element that handles `"enter"` and calls `cx.stop_propagation()` keeps the name, in the tests and in the real window. | Source; Measured (test platform); Measured (the handler, real window) | `gpui-base` 0.9.1 `src/input/base/state.rs`, `enter`; `a_page_name_is_edited_in_the_row` |
| `Input` applies the style of the caller after its own style. `.h(..)`, `.px(..)`, `.bg(..)`, `.rounded(..)`, and `.text_size(..)` on `Input::new(..)` changed the field. A negative leading margin on the parent (`ml(-inset)`) put the text of the field at the same x as the name in the other rows. | Source; Measured | `gpui-component` 0.9.1 `src/input/input.rs`, `refine_style`; window picture |
| `Window::prompt(PromptLevel::Warning, message, Some(detail), &[PromptButton::ok("Remove"), PromptButton::cancel("Cancel")], cx)` shows a native alert in the centre of the window. Return chose "Remove", and Escape chose "Cancel". The receiver gives the index of the button. | Measured | Window pictures |
| On the test platform, `VisualTestContext::pending_prompt()` gives the message and the detail of the alert, and `simulate_prompt_answer("Remove")` chooses a button by its label. | Source; Measured | `gpui-pre` 0.3.12 `src/app/test_context.rs`; `the_page_menu_sends_the_page_commands` |
| On the test platform, `NativeMenu::show` shows nothing and returns: the test window has no raw window handle, thus the menu finds no view. A window test does the right click, and then sends the action of the menu item with `dispatch_action`. | Source; Measured (no panic) | `gpui-component` 0.9.1 `src/native_menu/macos.rs`, `ns_view_ptr`; `gpui-pre` 0.3.12 `src/platform/test/window.rs` |
| `NativeMenu::menu_with_disabled` shows a disabled item, for example "Move Up" on the first page. `menu_with_check` shows a check mark. | Measured | Window pictures |
| A native menu opens with its first item under the pointer, and that item is highlighted. Then the Up arrow key did not move the highlight. Move the pointer out of the menu before the arrow keys in a helper program. | Measured | Window pictures |

## Device drawing on the canvas (2026-11-13)

**Measured.** The application draws the device of
[issue 178](https://github.com/shantanugoel/OpenCraft/issues/178) on the
canvas. The window tests and the real window on the computer of
[Release measurements](#release-measurements) gave these facts. A camera
picture of the device agreed with the window picture: the positions of the
key faces, the bar panels, and the knobs, and the colours of the knob lights.

| Fact | Confidence | Source |
|---|---|---|
| `border_dashed()` with `rounded(..)` draws a dashed line that follows the rounded corners. A dashed line of 1 and of 2 shows in the real window. | Measured | `crates/app/src/view/canvas.rs`; real window picture |
| `img(image).rounded(r)` clips the image to the rounded corners. | Measured | Real window picture |
| GPUI rounds the layout to device pixels. In the test window, an element at 5.55 points from its parent was at 5.5. A test of positions that come from a scale needs a tolerance of one device pixel. | Measured | `view/tests/canvas.rs`, `near` |
| A click handler that calls `cx.stop_propagation()` stops the click: the click handler of the parent element does not run. Thus a click on a control selects the control, and a click on the empty canvas around it selects the page. | Measured | `a_click_on_an_empty_part_of_the_canvas_selects_the_page` |
| `group(name)` on a control and `group_hover(name, ..)` on an outline element outside the bounds of the control show the outline while the pointer is on the control. | Measured | Real window picture |
| An outline outside a control is an absolute child with negative `left` and `top`. The parent has no border: the drawing puts each border on a separate layer of `size_full`, so that the positions of the children start at the edge of the parent. | Measured | `crates/app/src/view/canvas.rs`, `outline` |

## Release measurements

**Measured.** The computer has an Apple M1 Max, 10 CPU cores, and 32 GiB of
memory. It runs macOS 27.2. Rust is stable 1.99.1. Other applications were
open. Source: `system_profiler`, `sw_vers`, `rustc --version`.

**Measured.** The test uses `cargo build --release --locked`, stripped
symbols, and no LTO. This is not the workspace release profile, which uses
thin LTO. Both forms use the same GPUI platform features: `font-kit`, `x11`,
`wayland`, and `runtime_shaders`. The widget form initializes the library and
mounts a Root. It uses a plain Input, not the icon catalog. Source: test
program, manifests and build logs.

**Measured.** `measure.py` calls `footprint -p PID -f bytes` and `ps` directly.
It records physical footprint, RSS, and cumulative CPU time each 5 seconds.
CPU use is the change in CPU time divided by elapsed time. It is not the
smoothed `%cpu` from `ps`. Each idle run lasts 60 seconds, with 13 memory
samples and 12 CPU intervals. The worker stops before the idle run. The text
field has no focus. The window stays on the screen for the visible run.
`App::hide` hides the application before the hidden run. Source: test
program, `measure.py` and `measurements.json`.

| Measurement | Bare GPUI | With widgets | Confidence and source |
|---|---|---|---|
| Visible footprint: minimum, median, maximum | 39.50, 39.50, 43.36 MiB | 42.17, 42.19, 47.13 MiB | Measured; `measurements.json` |
| Hidden footprint: minimum, median, maximum | 37.95, 37.95, 37.95 MiB | 40.58, 40.58, 40.58 MiB | Measured; `measurements.json` |
| Visible RSS, median | 94.69 MiB | 102.66 MiB | Measured; `measurements.json` |
| Hidden RSS, median | 94.86 MiB | 102.92 MiB | Measured; `measurements.json` |
| Visible CPU use: mean, maximum | 0.000, 0.000 % | 0.000, 0.000 % | Measured; `measurements.json` |
| Hidden CPU use: mean, maximum | 0.017, 0.200 % | 0.017, 0.200 % | Measured; `measurements.json` |
| Footprint during image replacement: minimum, median, maximum | 41.31, 41.63, 42.11 MiB | 43.80, 43.94, 45.63 MiB | Measured; 65-second sample run, `measurements.json` |
| Footprint 10 seconds after the window closes, one sample | 24.86 MiB | 27.41 MiB | Measured; process still alive, `measurements.json` |
| Process start to first submitted frame: minimum, median, maximum, 5 starts | 196, 231, 347 ms | 377, 397, 553 ms | Measured; `first-present.json` |
| Stripped release binary | 9.66 MiB | 17.94 MiB | Measured; file size |
| Clean release build, elapsed | 56.16 s | 90.14 s | Measured; `/usr/bin/time -p`, `clean-bare.log`, `clean-widgets.log` |
| Distinct dependency packages, normal and build edges, native target, excluding the test package | 311 | 411 | Measured; `cargo tree`, `bare-tree.txt`, `widgets-tree.txt` |

**Measured.** The clean builds use separate empty target directories.
Registry sources are already downloaded. The package count includes build
scripts and procedural macros, but not development dependencies. The widget
cost in this test is 100 more packages, 33.98 seconds more clean build time,
8.28 MiB more binary data, and 2.69 MiB more visible median footprint.
Source: build logs, dependency trees, and the table above.

**Measured.** For startup, the parent records a timestamp just before process
creation. Idle results and startup results came from separate runs. The program
schedules a foreground task from its first `Window::on_next_frame` callback. The task records a timestamp after the
frame callback returns and submits the first scene. The difference includes
process creation and GPUI initialization. `on_next_frame` alone runs before
presentation in this version; its name is not a post-presentation guarantee.
This test measures submission, not GPU completion or display scanout. Each
start is a new process. The test does not purge the OS or Metal caches.
Source: test program, `src/main.rs`, `measure.py`, `first-present.json`;
published GPUI `src/window.rs`, frame callback and `present`.

**Inference.** These are not full product measurements. There is no engine,
profile, device, or time to a ready device. The widget form leaves memory
space below the product target, but its first submitted frame already takes
more than 300 ms. The integrated application must improve startup and repeat
the measurements in [issue 58](https://github.com/shantanugoel/OpenCraft/issues/58).
The 0.017 % idle result is one 10 ms CPU-time step in about 60 seconds. It
does not show continuous work. These shorter runs are not the same procedure
as the 5-minute [vendor reference](vsd-craft.md#footprint-when-it-runs).
Source: measurements above; [product targets](../ROADMAP.md#product-targets).

## Platform functions and CI

| Fact | Confidence | Source |
|---|---|---|
| GPUI has no built-in menu bar item API in this snapshot. Pull request 44047 closed without a merge on 2025-12-12. | Source | Published platform API; [tray PR] |
| The PR discusses `gpui-tray`. A direct `NSStatusItem` through `objc2-app-kit` is another route. The direct route operates: see [Shell test of Phase 5](#shell-test-of-phase-5-2026-11-12). `gpui-tray` was not tested. | Source; Measured (direct route) | [tray PR]; test program |
| GPUI has `prompt_for_paths` and `prompt_for_new_path` native file dialogs, and `ActivationPolicy::Accessory` to hide the Dock icon. | Source | Published GPUI `src/platform.rs` and `src/app.rs` |
| `QuitMode::Default` is explicit quit on macOS. The program can keep running after its last window closes. The test uses `QuitMode::Explicit` and confirms this. | Source; Measured | Published GPUI `src/app.rs`; test program, close test |
| The snapshot has separate platform crates. Bare desktop code needs the platform crate and `gpui_platform::application()`, not just the core crate. On macOS, enable `font-kit` for text. The kit sets the platform features, including `runtime_shaders`. | Source; Measured (build) | Published platform and kit manifests; test program |
| The kit supports Linux builds. Its CI builds and tests on Ubuntu. Compile-only CI does not need a window or a device. | Source | [kit CI]; [Linux setup] |
| The upstream Ubuntu 24.04 setup installs `gcc g++ clang libfontconfig-dev libwayland-dev libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev libssl-dev libzstd-dev libasound2-dev vulkan-validationlayers libvulkan1`. This is a conservative package list for the full upstream workspace, not a tested minimum for this program. | Source | [Linux setup] |
| The application CI install list is `libudev-dev pkg-config clang libfontconfig-dev libwayland-dev libxkbcommon-x11-dev libx11-xcb-dev libvulkan1`. It adds no WebKit, OpenSSL, Zstd, ALSA, or Vulkan validation package. The GitHub Ubuntu 26.04 image already has the usual C and C++ build tools. This is a working CI list, not a package list for a clean Linux desktop. | Measured | `.github/workflows/ci.yml`; [application CI run](https://github.com/shantanugoel/OpenCraft/actions/runs/43079214508), package and build logs |
| All four full-workspace checks pass on Ubuntu with that list. The eight application tests include three GPUI window tests with no display or device. A native Linux window was not tested. | Measured | [Application CI run](https://github.com/shantanugoel/OpenCraft/actions/runs/43079214508), fmt, clippy, build, and test logs |

## Licences and update cost

| Fact | Confidence | Source |
|---|---|---|
| The GPUI snapshot crates, kit, base, and component declare Apache-2.0. Not all transitive dependencies use that licence. | Source | Published manifests; test program, `cargo metadata` |
| The checked native dependency metadata has no GPL or AGPL dependency. It includes MPL-2.0 declarations for `cbindgen` and `option-ext`. This is not a complete licence audit. | Source | Test program, `cargo metadata` and dependency trees |
| The asset crate declares Apache-2.0 for code. Its Lucide icons use ISC, with MIT for Feather-derived icons. | Source | `gpui-kit-assets` 0.9.1 README and `LICENSE-LUCIDE` |
| An MIT application can use these dependencies. Distribution must keep the required licence texts, notices, and attributions. Review MPL obligations for the files that are distributed. | Inference | Declared licences above |
| An exact kit pin keeps one matching GPUI family. Commit the application lock file too: the kit's base and component dependencies use version ranges. A later update must test the image path, channel wake, text entry, native dialog, and footprint again. | Inference | Published kit manifest; test program |
| A Zed Git revision would not remove the widget compatibility work. It could also make Cargo resolve two GPUI families unless all related crates are aligned. There is no measured blocker that needs this route. | Inference | Exact snapshot requirements in the published widget manifests; successful snapshot tests |

## Not known

- Startup with empty OS and Metal caches. This test starts new processes but
  does not reset the computer or purge caches.
- Long-run image replacement and idle use with a focused text field. This
  test bounds image replacement to 650 pairs and measures idle with no focus.
- The work that a future snapshot update needs. One successful version does
  not predict the next API change.
- What occurs at a logout. macOS asks each application to quit, and the
  Inference is that this calls `NSApplication::terminate` and the
  `on_app_quit` closures. Nobody logged out during the test.
- Whether the image on the button of the menu bar item shows correctly in
  the light menu bar. Only the dark menu bar was tested.
- Full input-method and accessibility behaviour. The test covers simple text
  entry, not every input method or assistive tool.
- How VoiceOver reads a sidebar of plain `div` rows, a drag and drop, and
  the command palette. The window layout test did not use an assistive
  tool. Plain rows have no tree role until the code adds one.
- Arrow keys in a sidebar of plain `div` rows. The test program has no key
  bindings for them. The Kit tree has them.
- A drag out of the window, and a drag from a different application. The
  test only dragged inside the window.
- The look of the Kit parts in the light theme. The window layout test used
  the dark theme only.

## Sources

[registry]: <https://crates.io/crates/gpui-pre>
[kit]: <https://github.com/longbridge/gpui-kit/tree/v0.9.1>
[kit README]: <https://github.com/longbridge/gpui-kit/blob/38a54f0220024f7394dd07d7e6685688168d64e6/README.md>
[publish script]: <https://github.com/longbridge/gpui-kit/blob/38a54f0220024f7394dd07d7e6685688168d64e6/script/bump-gpui.ts>
[release workflow]: <https://github.com/longbridge/gpui-kit/blob/38a54f0220024f7394dd07d7e6685688168d64e6/.github/workflows/release-gpui.yml>
[kit CI]: <https://github.com/longbridge/gpui-kit/blob/38a54f0220024f7394dd07d7e6685688168d64e6/.github/workflows/ci.yml>
[Linux setup]: <https://github.com/longbridge/gpui-kit/blob/38a54f0220024f7394dd07d7e6685688168d64e6/script/install-linux.sh>
[tray PR]: <https://github.com/zed-industries/zed/pull/44047>

- GPUI 0.3.12 published source:
  <https://docs.rs/crate/gpui-pre/0.3.12/source/>
- GPUI platform 0.3.12 published source:
  <https://docs.rs/crate/gpui-pre-platform/0.3.12/source/>
- GPUI Component 0.9.1 published source:
  <https://docs.rs/crate/gpui-component/0.9.1/source/>
- GPUI Base 0.9.1 published source:
  <https://docs.rs/crate/gpui-base/0.9.1/source/>
- GPUI Kit 0.9.1 published source and its testing guide `TESTING.md`:
  <https://docs.rs/crate/gpui-kit/0.9.1/source/>
