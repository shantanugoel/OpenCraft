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
| With the application and a check profile, each of these steps operated: a swipe to the left and to the right changed the page; a press of a key with a toggle showed the frame and the label for "on", and a second press removed them; a tap on a bar panel with a toggle did the same; a sequence showed the second page and then the first page again; a key selected a different profile, and a key selected the first profile again | Measured: owner |
| The owner did the page changes and reported no problem with their time. The measured time is in [Time of a page change](#time-of-a-page-change-2026-11-12). | Measured: owner |

## Phase 5

### Memory and processor use (2026-11-12)

The application is the program `opencraft` of Phase 5: the engine, the
built-in action handlers, the menu bar item, and the window, in one process
([decision 0019](../decisions/0019-engine-in-the-gui-process.md)). The build
is the release profile of the workspace at commit `836b202`. The computer is
the same as in [Phase 2](#phase-2). The profiles directory was a copy of the
profiles directory of [Phase 4](#phase-4): 2 profiles, and the active profile
had 2 pages, content on 22 surfaces, and 16 bindings. The profiles
directory had no file `device-settings.toml`, thus the engine used the
default device settings. The device was connected. No person used the device
or the window during a measurement.

`scripts/measure-footprint.sh --pid` made one sample each 5 seconds for 300
seconds. Each measurement started 60 seconds after the start of the
application. For "Window closed", the window closed 60 seconds after the
start through its close button, and the measurement started 30 seconds after
that. A start with `--login` opens no window
([decision 0021](../decisions/0021-start-at-login.md)). The Dock icon option
is `show_dock_icon` in `shell.toml`
([decision 0020](../decisions/0020-menu-bar-item-and-dock-icon.md)). Each
column is a new process.

| Measurement | Window open | Window closed, Dock icon shown | Start with `--login`, Dock icon shown | Start with `--login`, Dock icon hidden | Confidence |
|---|---|---|---|---|---|
| Processes | 1 | 1 | 1 | 1 | Measured |
| Physical footprint: minimum, median, maximum | 46.9, 47.0, 47.0 MiB | 31.4, 31.4, 32.9 MiB | 20.5, 20.6, 20.6 MiB | 20.4, 20.5, 20.6 MiB | Measured |
| Resident size, median | 112.1 MiB | 111.1 MiB | 86.3 MiB | 86.3 MiB | Measured |
| Processor time in 300 seconds | 0.49 s | 0.50 s | 0.46 s | 0.40 s | Measured |
| Processor use, mean (100 % is one core) | 0.16 % | 0.17 % | 0.15 % | 0.13 % | Measured |
| Processor use, maximum of a sample | 0.4 % | 0.2 % | 0.2 % | 0.2 % | Measured |
| Threads | 15 | 15 | 13 | 13 | Measured: `ps -M` |

- With no window, the footprint is 20.6 MiB (21.6 MB) after a start with
  `--login`, and 31.4 MiB (32.9 MB) after the window closed. This is 27 %
  and 41 % of the target of 80 MB. (Inference from the measured values)
- With no window, the mean processor use is 0.13 to 0.17 % of one core. This
  is below the target of 0.5 %. (Inference from the measured values)
- The Dock icon option does not change the footprint. (Measured)
- A closed window does not decrease the processor use. The engine thread and
  the volume handler use almost all of the processor time, also when the
  window is open. (Measured: the processor time of each thread from
  `ps -M`, each 5 seconds)
- The engine thread used 0.31 to 0.38 s in 300 seconds (0.10 to 0.13 % of
  one core). The volume handler used 0.10 s in 300 seconds, as in
  [Phase 3](#phase-3). (Measured. The names of the threads come from the
  order of the threads in `sample`: Inference)
- The footprint after the window closed is 15.6 MiB less than with the
  window open, but 10.8 MiB more than after a start with no window. The
  GPUI test program has the same behaviour: see
  [gpui.md](gpui.md#footprint-with-no-window). (Measured)
- With the window open, the footprint is 0.5 MiB less than in Phase 4. The
  processor use is lower than in Phase 4 (0.21 %), and no sample has 1 % or
  more. (Measured)
- The release binary has 22 109 792 bytes. (Measured)
- The owner used the computer during the measurements, thus other programs
  operated. The script measures only the OpenCraft process, but the other
  programs can change its values a small quantity. The idle wake-ups of the
  process were not measured, as in Phase 4.

### Long run with no window (2026-11-12)

The application started with `--login`, with the Dock icon shown. The
measurement started 60 seconds after the start and continued for 1800
seconds, with one sample each 5 seconds. A second script read the processor
time of each thread with `ps -M` each 5 seconds. It was set to run `sample`
on the process when the process used 0.04 s or more in 5 seconds.

| Measurement | Value | Confidence |
|---|---|---|
| Samples with a processor value | 360 | Measured |
| Physical footprint: minimum, median, maximum | 20.6, 20.6, 20.7 MiB | Measured |
| Processor time in 1800 seconds | 2.84 s | Measured |
| Processor use, mean | 0.16 % | Measured |
| Samples with 0.0 %, 0.2 %, and 0.4 % | 79, 278, 3 | Measured |
| Samples with 1 % or more | 0 | Measured |
| Threads at the start and at the end | 13, 13 | Measured: `ps -M` |

- The engine thread used 2.23 s and the volume handler used 0.60 s of the
  2.84 s. (Measured. Thread names: Inference, as above)
- The footprint did not increase in 30 minutes. (Measured)

### Samples with high processor use (2026-11-12)

The sample with 1.6 % in [Phase 4](#phase-4) did not occur again in the 5
measurements above: 600 samples with a processor value, each with 0.4 %
or less. One more
measurement with the window closed stopped after 205 seconds, because a
time limit of the measurement tool stopped it. Its values are not in the
tables. It had one sample with 1.2 %, and one adjacent sample with 0.8 %:

- The window had closed approximately 75 seconds before.
- In these 10 seconds, the process used 0.10 s. The main thread used 0.03 s.
  New, short threads with a high priority (46 and 47) used the remaining
  time. The engine thread used 0.01 s, the same as in the other samples.
  (Measured: `ps -M`)
- `sample` did not run at that time, because the limit for `sample` was too
  high in that run. Thus the code that used the time is not known.
- The work was on the main thread and on new threads, not on the engine
  thread. Thus GPUI or AppKit did the work, and not the engine. (Inference)

### Start time (2026-11-12)

The procedure is the same as in [Phase 2](#phase-2): the environment
variable `OPENCRAFT_START_TRACE`, and the time immediately before the start
of the process. A Perl command recorded the time and then replaced itself
with the application (`exec`), thus the process ID stayed the same. The
application started 6 times with a window, and then 6 times with `--login`.
The engine log line `full draw done` gives the time of the last write of
the first full draw. Each time is from the start of the process.

| Step | Window, start 1 | Window, starts 2 to 6: minimum, median, maximum | `--login`, starts 1 to 6: minimum, median, maximum | Confidence |
|---|---|---|---|---|
| The function `main` starts | 541.2 ms | 6.3, 6.7, 7.1 ms | 6.6, 6.8, 7.6 ms | Measured |
| `full draw done` in the engine log | 588.7 ms | 51.0, 52.7, 54.4 ms | 50.7, 51.9, 52.5 ms | Measured |
| The device is ready (the event `Connected`) | 588.3 ms | 51.3, 53.0, 54.0 ms | 50.9, 51.9, 52.2 ms | Measured |
| The first frame of the window | 872.7 ms | 321.4, 328.4, 334.3 ms | No window | Measured |

- Window start 1 was the first start of the release build. One more start
  with `--login`, from a new copy of the same file, gave 537.5 ms before
  `main` and 588.6 ms to the ready device. Thus the first start of a new
  executable file adds approximately 0.5 seconds before `main`. (Measured)
- From `main` to the ready device, the time was 44 to 51 ms in all 13
  starts. In Phase 2 it was 61 to 76 ms. (Measured)
- A start with `--login` gives a ready device at the same time as a start
  with a window. The window does not delay the device. (Measured)
- A repeated start meets the target of 300 ms. The first start of a new
  executable file does not meet it. The start after a start of the computer
  is not measured. Issue
  [#137](https://github.com/shantanugoel/OpenCraft/issues/137) measures it.

### Product targets with no window (2026-11-12)

| Target | Value with no window | Result |
|---|---|---|
| Memory in the steady state: less than 80 MB | 21.6 MB after a start with `--login`, 32.9 MB after the window closed | Met |
| Processor use when idle: less than 0.5 % of one core | 0.13 to 0.17 %, and 0.16 % in 30 minutes. No sample of 5 seconds above 1.2 %. | Met |
| Cold start to a ready device: less than 300 ms | 52 ms for a repeated start; 589 ms for the first start of a new executable file; not measured after a start of the computer | Met for a repeated start only. Issue [#137](https://github.com/shantanugoel/OpenCraft/issues/137). |
| Lost input events: none | Not measured again. The window does not change the input path of the engine. | Not measured with no window |

## Not known

- The start time after a start of the computer, when the files of the
  application are not in the memory of the operating system. Issue
  [#137](https://github.com/shantanugoel/OpenCraft/issues/137) measures it.
  Start 1 of [Phase 2](#phase-2) and of [Phase 5](#phase-5) are the nearest
  values.
- The cause of the samples with 1.2 % and 1.6 % of one core: see
  [Samples with high processor use](#samples-with-high-processor-use-2026-11-12).
- Why the engine thread uses more processor time than the full application
  of Phase 2 (0.11 s in 300 seconds, hidden). The engine thread of Phase 5
  uses 0.31 to 0.38 s. Phases 2 to 4 did not measure each thread.
- Lost input events between the hand and the device report. The measurement
  of Phase 3 starts at the reports that the device sends. No measurement
  counted the detents of a knob that a person turned.
- The footprint with no device connected. The engine then looks for the
  device one time each second.
- The footprint after many hours. In 30 minutes with no window, it did not
  increase: see [Long run with no window](#long-run-with-no-window-2026-11-12).

## Sources

- `scripts/measure-footprint.sh`: see [scripts/README.md](../../scripts/README.md).
