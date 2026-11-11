# Footprint of OpenCraft

Last update: 2026-11-12

This entry holds the measured values for the
[product targets](../ROADMAP.md#product-targets). The reference is the vendor
software: see [vsd-craft.md](vsd-craft.md#footprint-when-it-runs). Measure
again at the end of each phase.

## Phase 2

The application is the program `opencraft` of Phase 2: the engine and one
window, in one process. The build is the release profile of the workspace.
The computer has an Apple M1 Max processor and macOS 27.2. Other applications
were open. The device was connected, and the profile had content on 8
surfaces. No person used the device or the window during a measurement.

### Memory and processor use

`scripts/measure-footprint.sh --pid` made one sample each 5 seconds for 300
seconds. The first measurement started 60 seconds after the start of the
application. The second measurement started 30 seconds after the application
was hidden.

| Measurement | Window open | Application hidden | Confidence |
|---|---|---|---|
| Processes | 1 | 1 | Measured |
| Physical footprint: minimum, median, maximum | 46.3, 46.3, 46.3 MiB | 46.5, 46.5, 46.5 MiB | Measured |
| Resident size, median | 110.5 MiB | 110.7 MiB | Measured |
| Processor time in 300 seconds | 0.43 s | 0.11 s | Measured |
| Processor use, mean (100 % is one core) | 0.14 % | 0.04 % | Measured |
| Processor use, maximum of a sample | 0.2 % | 0.2 % | Measured |
| Threads, with the application hidden | 8 | | Measured: `ps -M` |

- The engine thread wakes 50 times each second
  ([decision 0011](../decisions/0011-engine-thread-model.md)). With this, the
  full application uses 0.04 % of one core when it is hidden. (Measured)
- The footprint is 58 % of the target of 80 MB, and 7 % of the footprint of
  the vendor software (698.8 MiB). (Inference from the measured values)
- The release binary has 20 608 128 bytes. (Measured)

### Start time

The application prints the time of 3 steps when the environment variable
`OPENCRAFT_START_TRACE` is set. A test program, not in the repository,
recorded the time immediately before it started the process, and started the
application 6 times. Each time is from that moment.

| Step | Start 1 | Start 2 to 6: minimum, median, maximum | Confidence |
|---|---|---|---|
| The function `main` starts | 526.5 ms | 14.3, 21.5, 30.6 ms | Measured |
| The device is ready: the engine sent the start sequence and all 14 images | 599.0 ms | 75.0, 92.0, 104.0 ms | Measured |
| The first frame of the window | 854.0 ms | 320.0, 331.0, 345.7 ms | Measured |

- "The device is ready" is the event `Connected` of the engine. The engine
  sends it after the last write of the first draw. The device shows the last
  image less than 130 ms after that write: see
  [streamdock-protocol.md](streamdock-protocol.md#repaint-time). (Measured)
- The engine starts before the GUI toolkit, on its own thread. Thus the
  device is ready approximately 240 ms before the first frame of the window.
  (Measured)
- Start 1 was the first start of a new build of the program. The time before
  `main` was 526.5 ms. In the other starts it was 14 to 31 ms. Thus the
  operating system used approximately 0.5 seconds for the first start of the
  new file. (Inference)
- From `main` to the ready device, the time was 61 to 76 ms in all 6 starts.
  (Measured)
- The profile had 2 surfaces with an image of the user. The engine decodes
  each image at the start.

## Not known

- The start time after a start of the computer, when the files of the
  application are not in the memory of the operating system. Start 1 above
  is the nearest value.
- Lost input events. No measurement counted the gestures of a person and the
  events of the engine. Phase 3 has the exit criterion "No input event is
  lost during a fast knob turn".
- The footprint with no device connected. The engine then looks for the
  device one time each second.
- The footprint after many hours.

## Sources

- `scripts/measure-footprint.sh`: see [scripts/README.md](../../scripts/README.md).
