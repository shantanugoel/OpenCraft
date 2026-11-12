# OpenCraft

OpenCraft is a small, native, open-source application for StreamDock
controllers. It replaces **VSD Craft**, the vendor software of VSD and
Mirabox. It is written in Rust. It has no Electron and no embedded web
browser.

> **Status: version 0.1.0 is the first public release.** OpenCraft operates the
> VSD StreamDock N4 Pro on macOS: keys, knobs, the touch bar,
> pages, profiles, and device settings. It has no plug-ins yet. The
> [roadmap](docs/ROADMAP.md#status) gives the state of each phase.

This is an independent project. It is not affiliated with VSDinside or
Mirabox.

## Supported device and platform

| | |
|---|---|
| Device | VSD StreamDock N4 Pro. The Mirabox N4 Pro and N4 Pro E have the same layout, but nobody tested them. |
| Platform | macOS, on a Mac with Apple silicon. See [What you need](docs/user/install.md#what-you-need). |

The limits are in the [user guide](docs/user/faults.md#limits).

## Install

With Homebrew:

```sh
brew install --cask shantanugoel/opencraft/opencraft
```

Or download `OpenCraft-<version>-arm64.dmg` from the
[releases page](https://github.com/shantanugoel/OpenCraft/releases), open it,
and drag OpenCraft onto the shortcut Applications.

Quit VSD Craft before you start OpenCraft. Two applications must not operate
one device at the same time. The full procedure is in
[Install and first start](docs/user/install.md).

## Your first key

1. Connect the device, and open OpenCraft.
2. In the window, click a key face.
3. Type a label, and click a colour.
4. Click **No action ▾**, and select an action, for example **Open a URL**.
   Type a URL.
5. Push the key on the device.

The [user guide](docs/user/README.md) gives all procedures: the actions,
the knobs, the touch bar, pages, profiles, the device settings, the menu
bar item, the start at login, and the faults.

## For developers

Build and run the application from source:

```sh
cargo build -p opencraft-app --release
cargo run -p opencraft-app --release
```

The variable `OPENCRAFT_PROFILES_DIR` sets a different profiles directory,
for example for a test.

CI runs four checks on each pull request. Run the same checks on your
computer before you push. No device is necessary. CI also checks the
licences of the dependencies and the file `THIRD-PARTY-NOTICES.txt`. See
[notices.sh](scripts/README.md#noticessh).

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace
```

On Linux, install the system packages in
[the CI workflow](.github/workflows/ci.yml) first.

```
crates/core/     device descriptors, transport (HID and fake), protocol, input decoding, surface images
crates/cli/      opencraft-probe — hardware bring-up tool
crates/engine/   profile, bindings, handler interface, surface images, device owner, messages
crates/app/      opencraft — native GUI shell, menu bar item, start at login
crates/actions/  built-in action handlers
scripts/         measurement, signing, release, licence notices, Homebrew cask, icon, and input recording scripts
assets/          the application icon
docs/            architecture, roadmap, decision records, knowledge base, user guide
.github/         CI workflow and issue forms
```

| Document | Contents |
|---|---|
| [docs/README.md](docs/README.md) | Start here: which document holds which fact |
| [crates/cli/README.md](crates/cli/README.md) | The probe `opencraft-probe`: its commands, the measurement commands, and the verified hardware |
| [docs/user/README.md](docs/user/README.md) | The user guide |
| [docs/ADDING-A-DEVICE-MODEL.md](docs/ADDING-A-DEVICE-MODEL.md) | The procedure to add a new device model |
| [scripts/README.md](scripts/README.md) | The scripts for measurement, signing, the release, the licence notices, and the Homebrew cask |
| [CHANGELOG.md](CHANGELOG.md) | The changes for each version |
| [SECURITY.md](SECURITY.md) | How to report a vulnerability |
| [AGENTS.md](AGENTS.md) | The rules for agents |
| [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt) | The licence notices of the dependencies. `scripts/notices.sh` makes it from `about.toml`. `deny.toml` holds the accepted licences. |

The protocol details came from the MIT-licensed
[Device SDK](https://github.com/MiraboxSpace/StreamDock-Device-SDK) of
Mirabox and the [mirajazz](https://github.com/4ndv/mirajazz) crate (MPL-2.0).

## Licence

MIT. See [LICENSE](LICENSE). The licences of the dependencies are in
[THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt). The menu bar item of
OpenCraft also opens them.
