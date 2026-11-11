# macOS privacy permissions

Last update: 2026-11-11

## Device access

| Fact | Confidence | Source |
|---|---|---|
| The vendor interface (interface 0, usage page `0xFFA0`) needs no privacy permission | Measured | Probe |
| The keyboard-emulation interface (interface 1, usage page `0x0001`) needs the Input Monitoring permission | Measured | Probe |

Thus OpenCraft does not open the keyboard-emulation interface. This is rule 6
of the [architecture](../ARCHITECTURE.md#5-rules). OpenCraft does not need the
Input Monitoring permission.

## Actions

These facts are not tested in this project. Test them in Phase 3.

| Fact | Confidence |
|---|---|
| An application needs the Accessibility permission to send key events to other applications | Inference |
| An application needs no permission to open an application or a URL | Inference |
| macOS connects a permission to the code signature. A build with a different signature can lose the permission. | Inference |

The first plan put the permission work after the action work. That order is
incorrect. The hotkey action needs the Accessibility permission on the first
day.

## Not known

- Whether a development build with a stable signature keeps the Accessibility
  permission after each build.
- Which permission the media key action and the sound volume action need.
