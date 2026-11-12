# GPUI

Last update: 2026-11-12

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
| The application uses `gpui-kit` 0.9.1. Its unbounded `async_channel` receiver wakes a `Context::spawn_in` task. The engine sink uses `try_send`, so it does not wait for the window. Events sent before window creation stay in the queue. | Measured | `crates/app/src/main.rs`, `view.rs`; window tests |
| `InputState::set_value` does not emit `InputEvent::Change`. It resets selection and undo state. The application calls it when a surface is selected, not on each engine reply. A reply during typing does not change the field or send a new command. | Source; Measured | `gpui-base` 0.9.1 `input/base/state.rs`; application window tests |
| A Kit glob import with `test-support` also imports its `test` macro. This can make generated `#[test]` attributes expand again. Explicit imports prevent this. `debug_bounds` needs a static selector registered with `debug_selector`, not only an element ID. | Measured | Application test build and `crates/app/src/view/tests.rs` |
| A native file dialog selected a transparent PNG. The engine read and copied it. A key face showed its label, colour, and image in the window and on the device. A bar panel also showed its label, colour, and image in both places. Removing all three parts cleared the bar panel in both places. The arrow stayed upright, with red on the left and blue on the right. | Measured | Release application; native window screenshots and USB camera pictures, kept outside the repository |
| The native window fits at 900 × 640 content pixels. A restart shows the saved profile. Normal window close ends the process and writes the last label change. | Measured | Release application; native screenshots, process check, and test profile file, kept outside the repository |
| The macOS platform quits with `NSApplication::terminate`; code after the run loop is not a sufficient stop hook. GPUI awaits `on_app_quit` tasks for at most 200 ms. The application also delays normal window close until a background task has stopped the engine, with no time limit on that path. | Source | `gpui-pre-macos` 0.3.12 `platform.rs`; `gpui-pre` 0.3.12 `app.rs`; `crates/app/src/main.rs` |

## Window checks of Phase 4 (2026-11-12)

| Fact | Confidence | Source |
|---|---|---|
| A second `hover` style on one element stops a debug build with the message "hover style already set". It is a `debug_assert`, so a release build keeps one of the two styles with no message. Give each element one hover style only. | Source; Measured (debug panic) | `gpui-pre` 0.3.12 `src/elements/div.rs`, `hover`; application window tests |
| An action from `Window::dispatch_action` does not get to an input that is not in the window yet, for example an input that the same click adds. `InputState::select_all` and `InputState::focus` operate on the entity directly, and the selected text is replaced by the next typed text. | Measured | Application window tests, `crates/app/src/view/pages.rs`, `edit_name` |
| `InputEvent::PressEnter` comes when the user presses Enter in a single-line `Input`. | Source; Measured | `gpui-base` 0.9.1 `input/base/state.rs`; application window tests |
| `deferred(element).with_priority(n)` paints an element after all other elements, and a higher number paints later. A menu with priority 2 over a full-window layer with priority 1 operates in the window tests. Each of the two needs `occlude`: with the default hitbox, a click also gets to the elements behind. | Source; Measured | `gpui-pre` 0.3.12 `src/elements/deferred.rs` and `src/elements/div.rs`, `occlude`; application window tests |
| `ClickEvent::click_count` gives the count of clicks of the mouse-up event. The page tabs use it for a double click. | Source | `gpui-pre` 0.3.12 `src/interactive.rs` |
| The no-display test platform does not show the result of a double click, of a tooltip, or of the visual style. The window tests check the bounds of the elements and the commands only. | Inference | `TestAppContext` API; application window tests |

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
| The button of the test shows a text title. An image on the button was not tested. | Measured | Test program |

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
| An accessory application does not show in the list of Cmd-Tab. | Inference | It has no Dock icon. Not measured. |

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
- Whether an accessory application shows in the list of Cmd-Tab.
- What occurs at a logout. macOS asks each application to quit, and the
  Inference is that this calls `NSApplication::terminate` and the
  `on_app_quit` closures. Nobody logged out during the test.
- Whether an image on the button of the menu bar item shows correctly in
  the light and the dark menu bar.
- Full input-method and accessibility behaviour. The test covers simple text
  entry, not every input method or assistive tool.
- Whether a double click on a page tab gives a click count of 2 in the
  native window. The window tests do not simulate a double click.

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
