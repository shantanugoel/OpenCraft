# VSD Craft, the vendor software

Last update: 2026-11-11

VSD Craft is the vendor software that OpenCraft replaces.

## Footprint

The size on disk was measured on the computer of the owner, on or before
2026-11-10. The version of VSD Craft was not recorded.

| Fact | Confidence |
|---|---|
| `/Applications/VSD Craft.app` is 1.3 GB | Measured |
| `defaultData/` is 338 MB: default profiles and plug-ins | Measured |
| `videoBackground/` is 43 MB: MP4 and GIF backgrounds | Measured |
| `motionGraphLib/` is 9.4 MB: an animation runtime | Measured |
| `plugins/` is 15 MB: 29 `.sdPlugin` plug-ins | Measured |
| The application contains Qt 6 with QtQuick and QtMultimedia | Measured |
| The application contains QCefView, which is an embedded Chromium | Measured |
| Each plug-in is a Node.js process that connects through a WebSocket | Inference |
| The memory use comes from Chromium, the QML renderer, video decode, and the plug-in processes | Inference |

## Behaviour

| Fact | Confidence | Source |
|---|---|---|
| The software has a "Touch Bar mode", which the user selects for each scene | Source | Mirabox tutorial |
| "Touch Bar mode does not follow pages." Each scene has one touch bar. | Source | Mirabox tutorial |
| "When Touchbar Mode is enabled, knob icons will not be displayed on the device interface." | Source | Mirabox tutorial |
| The screen can show a dynamic background behind dynamic icons | Source | VSDinside product text |
| The plug-in namespace of the vendor is `com.hotspot.streamdock.*` | Inference | First plan. Not examined. |
| The profiles are folders for each device model, with `.sdPlugin` JSON files | Inference | First plan. Not examined. |

## Not known

- The memory use and the processor use of VSD Craft when it runs. Measure them
  one time. They are the reference for the product targets.
- The profile format. This is necessary only for an import function, which is
  not in a phase.
- How "Touch Bar mode" draws on the touch bar. See
  [n4-pro-hardware.md](n4-pro-hardware.md#not-known).

## Sources

- Mirabox tutorial, touch bar on the N4 Pro:
  <https://mirabox.net/blogs/tutorial/how-to-set-up-the-touch-bar-function-on-n4-pro>
- VSDinside, N4 Pro and Stream Deck Plus comparison:
  <https://www.vsdinside.com/blogs/blog/n4-pro-vs-stream-deck-plus-knob-ring-lights-touch-bar-and-full-screen-dynamic-background>
