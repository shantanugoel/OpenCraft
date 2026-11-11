# VSD Craft, the vendor software

Last update: 2026-11-11

VSD Craft is the vendor software that OpenCraft replaces.

## Footprint when it runs

The agent measured VSD Craft 3.10.201.0605 on the computer of the owner on
2026-11-11, with the script
[measure-footprint.sh](../../scripts/README.md#measure-footprintsh). The device
was connected, and nobody touched it. The active scene had a video background
and 10 key icons. Other applications were open on the computer. Each
measurement was 5 minutes, with a sample each 5 seconds.

| Measurement | Window open | Window closed | Confidence |
|---|---|---|---|
| Processes | 19 | 19 | Measured |
| Total footprint: minimum, median, maximum | 694.0, 698.8, 712.0 MiB | 674.8, 692.4, 701.0 MiB | Measured |
| Total resident size, median | 2229.6 MiB | 2242.6 MiB | Measured |
| Processor use, mean (100 % is one core) | 35.5 % | 45.4 % | Measured |
| Processor use, maximum of a sample | 38.0 % | 47.8 % | Measured |

The measurement with the window open started 5 minutes after the start of
the application. The measurement with the window closed started 1 minute
after the window closed.

The processes, with the window open:

| Processes | Number | Footprint, sum of the medians | Processor use, mean | Confidence |
|---|---|---|---|---|
| Chromium renderer (`CefViewWing (Renderer)`) | 12 | 363.3 MiB | 0.9 % | Measured |
| Main process (`VSD Craft`) | 1 | 159.5 MiB | 34.5 % | Measured |
| Node.js (`node20`) | 3 | 80.5 MiB | 0.0 % | Measured |
| Chromium GPU process | 1 | 65.5 MiB | 0.0 % | Measured |
| Other Chromium helper processes | 2 | 31.4 MiB | 0.0 % | Measured |

| Fact | Confidence | Source |
|---|---|---|
| VSD Craft continues to run when its window closes. No window is on the screen, and the 19 processes stay. | Measured | Window list of macOS, 2026-11-11 |
| The main process uses almost all of the processor time. With the window closed, it used more: 44.2 %. | Measured | Script |
| The cause of the processor use is possibly the video background, which the application sends to the device as frames | Inference | The camera showed a moving background on the device |
| The device shows the first page 4.2 seconds after the start of the application (`open -a`) | Measured: 1 time | Camera, 10 pictures each second |
| 11.8 seconds after the start, the set of processes and its total footprint were stable, at 738.2 MiB | Measured: 1 time | Script, option `--startup` |
| When VSD Craft quits, the displays and the knob lights of the device go off | Measured: 1 time | Camera. See [streamdock-protocol.md](streamdock-protocol.md#keep-alive-and-stop). |

The time to the first page is not the full "time to a ready device" of the
product targets: no person pressed a key to see if the device responded.

## Size on disk

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
| The plug-ins run in Node.js processes that connect through a WebSocket. 3 Node.js processes ran for 29 plug-ins. | Inference (the WebSocket), Measured (the 3 processes) |
| Most of the memory is in the Chromium processes: 460 of 699 MiB | Measured |

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

- The processor use of VSD Craft with a scene that has no video background.
- The time until the device responds to a key after the start of VSD Craft.
- The profile format. This is necessary only for an import function, which is
  not in a phase.
- How "Touch Bar mode" draws on the touch bar. See
  [n4-pro-hardware.md](n4-pro-hardware.md#not-known).

## Sources

- Mirabox tutorial, touch bar on the N4 Pro:
  <https://mirabox.net/blogs/tutorial/how-to-set-up-the-touch-bar-function-on-n4-pro>
- VSDinside, N4 Pro and Stream Deck Plus comparison:
  <https://www.vsdinside.com/blogs/blog/n4-pro-vs-stream-deck-plus-knob-ring-lights-touch-bar-and-full-screen-dynamic-background>
