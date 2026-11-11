# GPUI

Last update: 2026-11-11

GPUI is the GUI toolkit of the Zed editor. The decision that uses these facts
is [decision 0005](../decisions/0005-gpui.md). These facts change quickly.
Examine them again at the start of Phase 2.

## Crates

| Fact | Confidence | Source |
|---|---|---|
| `gpui` on crates.io is at 0.2.2, released 2025-10-22. No newer release exists. | Source | crates.io |
| `gpui-pre` is a snapshot of the Zed repository, released about one time each week. Version 0.3.12 is from 2026-11-09. | Source | crates.io |
| A member of Longbridge releases `gpui-pre`. It is not an official Zed release. | Source | crates.io |
| `gpui-kit` 0.9.1 is a widget library from Longbridge. It needs exactly `gpui-pre` 0.3.12. | Source | crates.io |
| `gpui-kit` and `gpui-component` have the same repository and the same version | Source | crates.io |
| `gpui-kit` and `gpui-component` are one project with two names | Inference | |
| The `gpui-kit` README says "75+ documented components" and gives AccessKit support | Source | README. Not examined. |

## Functions

| Fact | Confidence | Source |
|---|---|---|
| GPUI has no API for a menu bar item. The pull request for it (44047) was closed without a merge on 2025-12-12. | Source | Zed repository |
| The alternatives are the `gpui-tray` crate as a Git dependency, or `NSStatusItem` through `objc2-app-kit` | Source | Pull request text |
| GPUI has native file dialogs: `prompt_for_paths` and `prompt_for_new_path` | Source | `platform.rs` |
| GPUI can hide the Dock icon: `ActivationPolicy::Accessory` | Source | `platform.rs` |
| On macOS, a GPUI application stays open when the last window closes | Source | `QuitMode` default |

The first plan said that menu bar support was added in December 2025 and that
GPUI had no file dialogs. The two statements were incorrect.

## Licence

| Fact | Confidence | Source |
|---|---|---|
| `gpui`, `gpui-pre`, and `gpui-kit` are Apache-2.0 | Source | crates.io |
| The crates that `gpui` needs are Apache-2.0. The GPL and AGPL crates of Zed are not dependencies of `gpui`. | Source | Zed repository. No full audit. |
| An MIT application can use them. The released application must include the Apache-2.0 notices. | Inference | |

## Not known

Measure these in Phase 2.

- The memory use of a small GPUI application with one window open.
- The cold start time of a small GPUI application.
- The processor use of a GPUI application when the window is hidden.
- How much work an update of `gpui-pre` needs.
- Whether a menu bar item operates correctly with the GPUI event loop.

## Sources

- `gpui` on crates.io: <https://crates.io/crates/gpui>
- `gpui-pre` on crates.io: <https://crates.io/crates/gpui-pre>
- `gpui-kit`: <https://github.com/longbridge/gpui-kit>
- Zed pull request 44047: <https://github.com/zed-industries/zed/pull/44047>
- GPUI `platform.rs`:
  <https://github.com/zed-industries/zed/blob/main/crates/gpui/src/platform.rs>
