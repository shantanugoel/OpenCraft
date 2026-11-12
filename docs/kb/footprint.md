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

### Checks by the owner

The owner used the application and the device on 2026-11-12.

| Fact | Confidence |
|---|---|
| The window showed each key press, each knob turn and push, and each tap and swipe on the touch bar | Measured: owner |
| A label and a colour from the window showed on the device at once, and the small text was good to read | Measured: owner |
| The cable was disconnected for 3 seconds and connected again, while the owner moved the window. The window stayed responsive, and the device showed the profile again after approximately 7 seconds. | Measured: owner |

## Phase 3

The application is the program `opencraft` of Phase 3: the engine, the
built-in action handlers, and one window, in one process. The build, the
computer, and the procedure are the same as in [Phase 2](#phase-2). The
profile had content on 7 surfaces and 9 bindings. One binding had the kind
`opencraft.volume`, thus the volume handler read the sound volume one time
each 2 seconds. No person used the device or the window during a measurement.

| Measurement | Window open | Application hidden | Confidence |
|---|---|---|---|
| Processes | 1 | 1 | Measured |
| Physical footprint: minimum, median, maximum | 43.8, 43.8, 44.3 MiB | 43.8, 43.8, 43.8 MiB | Measured |
| Resident size, median | 107.4 MiB | 107.2 MiB | Measured |
| Processor time in 300 seconds | 0.53 s | 0.21 s | Measured |
| Processor use, mean (100 % is one core) | 0.18 % | 0.07 % | Measured |
| Processor use, maximum of a sample | 0.4 % | 0.2 % | Measured |
| Threads, with the application hidden | 10 | | Measured: `ps -M` |

- The handlers add 2 threads and 0.10 seconds of processor time in 300
  seconds, with the application hidden. The cause of the processor time is
  the volume handler, which reads the sound volume one time each 2 seconds.
  (Inference from the two phases)
- The footprint is 55 % of the target of 80 MB. The processor use is below
  the target of 0.5 % of one core. (Inference from the measured values)
- The footprint is not larger than in Phase 2. The difference of 2.5 MiB is
  in the range that a different profile can cause. (Inference)
- The release binary has 21 441 440 bytes. (Measured)
- The start time was not measured again in this phase.

### Checks by the owner

The owner used the application and the device on 2026-11-12. The application
was the bundle from `scripts/sign-dev.sh --release`, with the Accessibility
permission.

| Fact | Confidence |
|---|---|
| Each of the 7 built-in action kinds operated: 6 kinds from a key press, and the sound volume from a knob | Measured: owner |
| A turn of knob 1 changed the sound volume, a push muted the sound, and bar panel 1 showed the level | Measured: owner |
| During fast turns and pushes of the knobs, no input event was lost, and the order was the same: the engine example `knob_count` compared the input events that the shell got with the requests that a handler got | Measured: owner |

- `knob_count` also reads the counters of the engine: reports received, input
  events, and requests sent. The owner recorded the result line of the
  program and not the counts.

## Phase 4

### Memory and processor use (2026-11-12)

The application is the program `opencraft` of Phase 4: the engine, the
built-in action handlers, and one window, in one process. The build, the
computer, and the procedure are the same as in [Phase 2](#phase-2). The
profiles directory had 2 profiles. The active profile had 2 pages, content
on 22 surfaces, and 16 bindings. Two bindings had the kind
`opencraft.volume`. No person used the device or the window during a
measurement.

| Measurement | Window open | Application hidden | Confidence |
|---|---|---|---|
| Processes | 1 | 1 | Measured |
| Physical footprint: minimum, median, maximum | 47.5, 47.5, 47.5 MiB | 45.0, 45.0, 45.0 MiB | Measured |
| Resident size, median | 108.1 MiB | 108.3 MiB | Measured |
| Processor time in 300 seconds | 0.62 s | 0.31 s | Measured |
| Processor use, mean (100 % is one core) | 0.21 % | 0.10 % | Measured |
| Processor use, maximum of a sample | 1.6 % | 0.2 % | Measured |
| Threads, with the application hidden | 14 | | Measured: `ps -M` |

- The footprint is 59 % of the target of 80 MB with the window open. The
  mean processor use is below the target of 0.5 % of one core. (Inference
  from the measured values)
- One sample of 5 seconds with the window open had 1.6 %. The cause is not
  known. Each other sample had 0.4 % or less.
- The footprint is 1.2 to 3.7 MiB larger than in [Phase 3](#phase-3), and
  the application has 4 more threads. The 4 new action kinds are in 3 new
  handlers, and each handler has one thread. The cause of the fourth thread
  is not known. (Inference from the two phases and the code)
- The engine holds only the active profile and the images of the active
  page. Thus the count of profiles and pages does not change the footprint
  much. (Inference: decisions 0016 and 0018. Not measured with many pages.)
- The release binary has 21 758 368 bytes. (Measured)
- The start time was not measured again in this phase.

### Time of a page change (2026-11-12)

A test program, not in the repository, used the engine logic with the real
device. It made a profile in a temporary directory, and it called the page
change of the engine each 1.2 seconds. A USB camera with 30 pictures each
second recorded the device.

- The engine time is the time of the function for the page change: it makes
  the 14 images, encodes the images that changed, writes them, and writes
  the commit. The program is a release build.
- The camera time is from the start of the page change to the first picture
  with the new colour of key face 3. It includes the delay of the camera,
  which is not measured, and one picture is 33 ms. Thus it is an upper
  limit.

| Change | Count | Engine time: minimum, median, maximum | Camera time: minimum, median, maximum | Confidence |
|---|---|---|---|---|
| Page change, all 14 surfaces are different | 20 | 24, 34, 43 ms | 111, 147, 169 ms | Measured |
| Page change, 1 surface is different | 20 | 7, 12, 19 ms | 108, 141, 156 ms | Measured |
| Profile change, all 14 surfaces are different | 10 | 30, 43, 48 ms | 131, 146, 165 ms | Measured |

- The page was a label and a colour on each surface, with no image file.
  A page with image files was not measured.
- Key face 1, key face 3, and bar panel 1 showed the new page in the same
  camera picture or in adjacent pictures: the medians were 144, 147, and
  130 ms. Thus the device shows a new page as one change, not surface by
  surface. (Measured)
- A page change adds 12 ms of engine time when one image changed. This is
  the time to make the 14 images and to compare them. (Measured)
- The time from a finger on the device to the page change is not in these
  values. The device sends the report of a swipe when the finger lifts.

### Checks by the owner

The owner used the device on 2026-11-12.

| Fact | Confidence |
|---|---|
| 20 taps, 10 swipes to the left, and 13 swipes to the right each gave the correct gesture and one Invoke at a handler. The record is in [n4-pro-hardware.md](n4-pro-hardware.md#controls-and-input-codes). | Measured: owner, the engine example `touch_count` |

## Not known

- The start time after a start of the computer, when the files of the
  application are not in the memory of the operating system. Start 1 above
  is the nearest value.
- Lost input events between the hand and the device report. The measurement
  of Phase 3 starts at the reports that the device sends. No measurement
  counted the detents of a knob that a person turned.
- The footprint with no device connected. The engine then looks for the
  device one time each second.
- The footprint after many hours.

## Sources

- `scripts/measure-footprint.sh`: see [scripts/README.md](../../scripts/README.md).
