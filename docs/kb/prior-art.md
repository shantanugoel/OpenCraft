# Other software for this device family

Last update: 2026-11-12

The decision that uses these facts is
[decision 0001](../decisions/0001-native-application.md).

## Projects

| Project | Stack | Facts | Confidence |
|---|---|---|---|
| Mirabox StreamDock Device SDK | C++, Python | The vendor SDK. Licence MIT. It has a class for the N4 Pro. | Source |
| `mirajazz` | Rust | A transport crate for Mirabox and Ajazz devices. Licence MPL-2.0. Version 0.16.3 added haptic feedback for the N4 Pro family. | Source |
| OpenDeck | Rust, Tauri, Svelte | A host for Stream Deck plug-ins. Device support comes from plug-ins. | Source |
| `opendeck-akp05` | Rust | An OpenDeck plug-in for the N4 family. See below. | Source |
| Bitfocus Companion | Node.js | Supports the Mirabox N4. Support for the N4 Pro is not confirmed. | Source |
| StreamController | Python, GTK | We found no plug-in for the N4 Pro. | Source |

## `opendeck-akp05`

| Fact | Confidence |
|---|---|
| The README lists "VSDInside N4 Pro (5548:1023)", "Mirabox N4 Pro (5548:1008)", and "Mirabox N4 Pro E (5548:1021)" | Source |
| It draws 176 × 112 JPEG images on the 4 bar panels, and it reads knob turn and knob push | Source |
| It does not draw the 800 × 480 background, and it does not use touch points | Source |
| It has swipe disabled, because swipe was not reliable | Source |
| The README says for macOS: "Zero effort, no tests before release" | Source |
| The repository had 36 stars and 22 forks on 2026-11-11 | Source |
| It keeps its device settings in one configuration file for the plug-in (`config.toml`), not in an OpenDeck profile: haptic feedback on or off, and the knob lights (brightness, and one colour for each knob) | Source: README and `src/config.rs`, version 0.11.1, read 2026-11-12 |
| It sends these settings at each connection. For a setting with no value, it sends no command, and the device keeps its own state. With no knob light setting, the knob lights show the default effect of the device. | Source: `src/device.rs`, README |
| At each connection it sets the display brightness to 50. OpenDeck can change it later. | Source: `src/device.rs`, `src/main.rs` |
| At its stop it sends `DC` and then `HAN` (the `mirajazz` function `shutdown`), with a time limit of 2 seconds | Source: `src/device.rs` |
| We did not operate the plug-in with our device | — |

## OpenDeck issue 404

The issue is open. A user of an Ajazz AKP05E Pro asked for a swipe on the touch
bar to change the profile. The title is now "Support for pages within profiles
including touch bar swipe support". OpenDeck has no pages at this time.
(Source)

The first plan said that this issue showed no support for the touch bar. That
was incorrect. The issue is about pages and swipe, not about the display.

## Stream Deck plug-in model

These facts are not examined in this project. Examine them before the plug-in
decision.

| Fact | Confidence |
|---|---|
| A Stream Deck plug-in is a different process that connects to the host through a WebSocket | Inference |
| The current Elgato SDK makes plug-ins that run on Node.js | Inference |
| The settings panel of a plug-in (the "property inspector") is an HTML page | Inference |
| A host that shows these settings panels needs a web view | Inference |
| VSD Craft and OpenDeck use this plug-in model | Inference |

A web view is the largest part of the footprint of the vendor software. See
[vsd-craft.md](vsd-craft.md). Thus compatibility with Stream Deck plug-ins can
conflict with the "Small" quality goal.

## Conclusion

OpenCraft is not the only open-source software that can operate the N4 Pro.
The differences that stay are the footprint, tests on macOS, the background
display, and the touch bar as a full control.

## Sources

- Mirabox SDK: <https://github.com/MiraboxSpace/StreamDock-Device-SDK>
- `mirajazz`: <https://github.com/4ndv/mirajazz>
- OpenDeck: <https://github.com/nekename/OpenDeck>
- OpenDeck issue 404: <https://github.com/nekename/OpenDeck/issues/404>
- `opendeck-akp05`: <https://github.com/ambiso/opendeck-akp05>
- Bitfocus Companion, Mirabox surfaces:
  <https://companion.free/user-guide/v4.1/surfaces/mirabox-streamdock>
