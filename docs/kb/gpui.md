# GPUI

Last update: 2026-11-11

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
| The PR discusses `gpui-tray`. A direct `NSStatusItem` through `objc2-app-kit` is another possible route. Neither route was tested here. | Source; Inference (direct route) | [tray PR]; macOS AppKit API |
| GPUI has `prompt_for_paths` and `prompt_for_new_path` native file dialogs, and `ActivationPolicy::Accessory` to hide the Dock icon. | Source | Published GPUI `src/platform.rs` and `src/app.rs` |
| `QuitMode::Default` is explicit quit on macOS. The program can keep running after its last window closes. The test uses `QuitMode::Explicit` and confirms this. | Source; Measured | Published GPUI `src/app.rs`; test program, close test |
| The snapshot has separate platform crates. Bare desktop code needs the platform crate and `gpui_platform::application()`, not just the core crate. On macOS, enable `font-kit` for text. The kit sets the platform features, including `runtime_shaders`. | Source; Measured (build) | Published platform and kit manifests; test program |
| The kit supports Linux builds. Its CI builds and tests on Ubuntu. Compile-only CI does not need a window or a device. | Source | [kit CI]; [Linux setup] |
| The upstream Ubuntu 24.04 setup installs `gcc g++ clang libfontconfig-dev libwayland-dev libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev libssl-dev libzstd-dev libasound2-dev vulkan-validationlayers libvulkan1`. This is a conservative package list for the full upstream workspace, not a tested minimum for this program. | Source | [Linux setup] |
| Keep `pkg-config` and `libudev-dev` for the existing workspace. The chosen kit does not need a WebView; its normal dependency tree has no WebKit crate. A smaller Linux package list can omit WebKit after a CI build confirms it. | Source; Inference (smaller list) | OpenCraft `.github/workflows/ci.yml`; test program, `linux-tree.txt` |
| Ubuntu can keep the full-workspace build and clippy commands after the GUI is added, with the native packages installed. There is no source reason to exclude the GUI. macOS still needs a build and native-window check. No Linux build was run in this macOS test. | Inference | Platform manifests; [kit CI]; successful macOS build |

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

- The full application's memory and time to a ready device. This test has no
  engine or device. Repeat the test in issue 58, including startup work.
- Startup with empty OS and Metal caches. This test starts new processes but
  does not reset the computer or purge caches.
- Long-run image replacement and idle use with a focused text field. This
  test bounds image replacement to 650 pairs and measures idle with no focus.
- The work that a future snapshot update needs. One successful version does
  not predict the next API change.
- Whether a menu bar item operates correctly with the GPUI event loop. This
  stays for Phase 5; no menu bar item was made here.
- The minimum Linux system package list and a local Linux build. The source
  and upstream CI support Linux, but this test ran only on macOS.
- Full input-method and accessibility behaviour. The test covers simple text
  entry, not every input method or assistive tool.

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
