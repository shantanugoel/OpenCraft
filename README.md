# OpenCraft

A native, open-source replacement for **VSD Craft** — the stock software for
VSD / Mirabox StreamDock controllers. Rust, no Electron, no embedded Chromium.

> **Status: bring-up verified on real hardware.**
> `opencraft-probe` enumerates a VSD StreamDock N4 Pro, reads its firmware,
> paints all 14 panels, and decodes live input — without the vendor software.

---

## Quick start

```sh
cargo build --release
./target/release/opencraft-probe list        # what is on the USB bus
./target/release/opencraft-probe identify    # open it, dump identity + firmware
./target/release/opencraft-probe test        # paint every panel, then decode input
```

`test` numbers every display with its slot number, so you can map physical
keys to protocol slots. Press keys, turn knobs and swipe the touch bar while it
runs; each event prints with its decoded meaning. Panels are blanked on exit
(add `--keep` to leave the pattern up).

### Probe commands

| Command | What it does |
|---|---|
| `list` | Enumerate HID devices, flag supported controllers |
| `identify` | Open the device, print firmware + panel geometry |
| `test` | Paint all slots, then decode input for `--seconds` |
| `listen` | Decode input only |
| `draw <slot> <rrggbb>` | Paint one slot a solid colour |
| `clear` | Blank every panel |
| `raw` | Hexdump raw input reports (for unknown hardware) |

Options: `--seconds`, `--brightness`, `--mode <0-9>`, `--keep`, `--verbose`.

---

## Verified hardware

| | |
|---|---|
| Device | VSD StreamDock N4 Pro (Mirabox OEM) |
| VID:PID | `0x5548:0x1023` |
| Firmware | `V4.VSD N4 Pro.02.010` |
| Control interface | interface 0, usage page `0xFFA0`, 512 B in / 1024 B out |
| Keyboard interface | interface 1, usage page `0x0001` — **do not open** (triggers TCC) |
| Panels | 10 × 112×112 keys (slots 6–15) + 4 × 176×112 touch-bar panels (slots 1–4) |
| Protocol | StreamDock v3, 1024-byte output reports |

The probe decodes input from the keys, the knobs, and the touch bar. The 4
panels in slots 1–4 are the display of the touch bar. The probe does not draw
the 800×480 background image at this time. The full hardware facts are in
[docs/kb/n4-pro-hardware.md](docs/kb/n4-pro-hardware.md).

---

## Layout

```
crates/core/     device descriptors, HID transport, protocol, panel rendering
crates/cli/      opencraft-probe — hardware bring-up tool
docs/            architecture, roadmap, decision records, knowledge base
```

Start with [docs/README.md](docs/README.md).

`core` is deliberately GUI-free and blocking, so it can be driven from a CLI
today and from a worker thread behind GPUI later.

---

## Notes

- **Quit VSD Craft first.** Two applications driving one device will fight.
- The vendor control interface does **not** need Input Monitoring. Only the
  keyboard-emulation interface does, and we never open it.
- Protocol details were derived from Mirabox's MIT-licensed
  [Device SDK](https://github.com/MiraboxSpace/StreamDock-Device-SDK) and the
  [mirajazz](https://github.com/4ndv/mirajazz) crate (MPL-2.0).
- This is an independent project. Not affiliated with VSDinside or Mirabox.

## License

MIT — see [LICENSE](LICENSE).
