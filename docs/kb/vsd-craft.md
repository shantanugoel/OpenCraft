# VSD Craft, the vendor software

Last update: 2026-11-13

VSD Craft is the vendor software that OpenCraft replaces.

## Footprint when it runs

The agent measured the installed VSD Craft on the computer of the owner on
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
| `Info.plist` says version 3.10.201.0605. The running code and the Settings page say version 3.10.201.0625. | Source, Measured | `Info.plist` (Source). The running application and the Settings page (Measured). Analysis, 2026-11-12. |

The time to the first page is not the full "time to a ready device" of the
product targets: no person pressed a key to see if the device responded.

## Size on disk

The size on disk was measured on the computer of the owner, on or before
2026-11-10. The version of VSD Craft was not recorded. The analysis of
2026-11-12 corrected the rows for `motionGraphLib/` and for the plug-ins.

| Fact | Confidence |
|---|---|
| `/Applications/VSD Craft.app` is 1.3 GB | Measured |
| `defaultData/` is 338 MB: default profiles and plug-ins | Measured |
| `videoBackground/` is 43 MB: MP4 and GIF backgrounds | Measured |
| `motionGraphLib/` is 9.4 MB: 24 GIF files for key-press effects. It is not an animation runtime. | Measured, Source |
| `Resources/plugins/` holds 28 native plug-ins. The plug-ins have no code. | Measured, Source |
| The application contains Qt 6 with QtQuick and QtMultimedia | Measured |
| The application contains QCefView, which is an embedded Chromium | Measured |
| The application has 41 plug-ins. 3 of them use Node.js. The footprint measurement counted 3 Node.js processes. | Measured, Source |
| Most of the memory is in the Chromium processes. The sum of their medians is 460.2 MiB. The median of the total footprint is 698.8 MiB. | Measured |

## Behaviour

| Fact | Confidence | Source |
|---|---|---|
| Touch Bar mode is a setting of the device. It is not a setting of a scene. | Measured | The window of VSD Craft 3.10.201. Analysis, 2026-11-12. |
| The pills of Touch Bar mode are the small key actions on the touch bar. They belong to the scene. | Measured | The same window. |
| A Node.js plug-in opens a WebSocket on the local computer. | Source | The plug-in launch arguments. Analysis, 2026-11-12. |
| "When Touchbar Mode is enabled, knob icons will not be displayed on the device interface." | Source | Mirabox tutorial |
| The screen can show a dynamic background behind dynamic icons | Source | VSDinside product text |
| The plug-in namespace of the vendor is `com.hotspot.streamdock.*` | Inference | First plan. Not examined. |
| A scene is a folder. The folder name ends with `.sdProfile`. The folder holds `manifest.json`. | Measured, Source | The profile folders. Analysis, 2026-11-12. |
| An export of a scene is a ZIP archive of that folder. | Measured, Source | An export file. Analysis, 2026-11-12. |

## Device settings (2026-11-12)

The agent read the files of VSD Craft 3.10 on the computer of the owner. It
did not operate VSD Craft for these facts.

| Fact | Confidence | Source |
|---|---|---|
| The file `StreamDockConfig.plist` is INI text, not a property list. It keeps the display brightness and the knob light brightness for each device, in the sections `DeviceBrightness` and `DeviceLightBrightness`. The key of each value names the device model and the serial number of the device. | Measured, Source | The configuration file. Analysis, 2026-11-12. |
| The same file has a screen-off time for each device (`ScreenOffTime`) | Measured | The configuration file |
| A text search of the profile files found no brightness value | Measured | The profile files |
| The settings of VSD Craft include "Activate vibration", and a "Lighting control" with the choices "Light" and "Follow the software" | Source | The English text resources in the application binary |
| The function names `addQUCMDCommand` and `sendQUCMDCommand` are in a library of VSD Craft | Source | The symbol names in `libSDLibrary` |

## Scenes, folders, and the switch by application (2026-11-14)

These facts come from the analysis of VSD Craft 3.10.201 in issue #198,
sections 8 and 9. The analysis read a copy of the data folder and operated
the window. The labels are those of the analysis.

| Fact | Confidence | Source |
|---|---|---|
| A scene links up to three applications ("App 1" to "App 3"). The user picks each one from a list of the running applications, or with "Other..." (a file panel for any `.app`). | Measured | #198, §9 |
| The scene manifest keeps the full path of each linked application bundle in `AppIdentifier0` to `AppIdentifier2`, not the bundle identifier. | Measured, Source | #198, §8 and §9 |
| One scene is the default scene. The manifest marks it with `AppIdentifier` `"*"`. The device shows it when the front application has no linked scene. | Measured | #198, §9 |
| The switch occurs when the front application changes, also when the window of VSD Craft is open behind a different application. A close of the window alone causes no switch. The text in Settings says that the switch occurs only when the window is closed; that text is not correct for 3.10.201. | Measured | #198, §9 |
| The device shows the new scene 0.47 s to 1.10 s after the change. The time includes the activation of the application by macOS. The editor window also shows the new scene. | Measured | #198, §9 |
| If two scenes link the same application, the first scene loses the link, and the application shows a warning. | Source, Review | #198, §9 |
| The action "Create Folder" turns a key into a folder. In the folder, key 1 is a fixed "Go back" key, and the other keys are free. The page buttons stay visible. | Measured, Source | #198, §8 |
| A folder in a folder works. A folder is a child page in the directory `profiles/` of the scene, linked by a `ProfileUUID`. | Measured, Source | #198, §8 |
| In the analysed data folder, 4 of 16 folder actions pointed to a child page that did not exist. | Measured | #198, §8 |
| A drop of an action on a folder key replaced the folder with no question, and the child pages of the folder were deleted from the disk. | Measured | #198, §17 |
| The action "Scene Shift" selects the previous scene, the next scene, or a named scene, and can open the scene at a page. | Measured (UI), Source | #198, §6 and §8 |

## Not known

- The maximum depth of folders in folders, and whether one folder can hold
  more than one page. Public texts give both answers.

- The processor use of VSD Craft with a scene that has no video background.
- The time until the device responds to a key after the start of VSD Craft.
- How "Touch Bar mode" draws on the touch bar. Two methods can draw one
  image across the touch bar: an image that is larger than its surface, and
  the background. See
  [n4-pro-hardware.md](n4-pro-hardware.md#an-image-that-is-larger-than-its-surface).
  Which method VSD Craft uses is not examined.

## Sources

- Mirabox tutorial, touch bar on the N4 Pro:
  <https://mirabox.net/blogs/tutorial/how-to-set-up-the-touch-bar-function-on-n4-pro>
- VSDinside, N4 Pro and Stream Deck Plus comparison:
  <https://www.vsdinside.com/blogs/blog/n4-pro-vs-stream-deck-plus-knob-ring-lights-touch-bar-and-full-screen-dynamic-background>
