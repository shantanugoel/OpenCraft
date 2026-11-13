# Footprint of OpenCraft

Last update: 2026-11-13

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
  [#143](https://github.com/shantanugoel/OpenCraft/issues/143) measures it.

### Product targets with no window (2026-11-12)

| Target | Value with no window | Result |
|---|---|---|
| Memory in the steady state: less than 80 MB | 21.6 MB after a start with `--login`, 32.9 MB after the window closed | Met |
| Processor use when idle: less than 0.5 % of one core | 0.13 to 0.17 %, and 0.16 % in 30 minutes. No sample of 5 seconds above 1.2 %. | Met |
| Cold start to a ready device: less than 300 ms | 52 ms for a repeated start; 589 ms for the first start of a new executable file; not measured after a start of the computer | Met for a repeated start only. Issue [#143](https://github.com/shantanugoel/OpenCraft/issues/143). |
| Lost input events: none | Not measured again in Phase 5. The counts of Phase 3 and Phase 4 used the engine examples `knob_count` and `touch_count`, which have no window. The window does not change the input path of the engine. | Met in Phase 3 and Phase 4, with no window. Not measured with the Phase 5 application. |

## Phase 6

### The release bundle (2026-11-12)

The application is the bundle `OpenCraft.app` of `scripts/release.sh`, from
the commit `096d30b`: a Developer ID signature, the hardened runtime, and
the identifier `com.shantanugoel.opencraft`
([decision 0025](../decisions/0025-release-form.md)). It is not notarized.
The executable has 22 016 160 bytes. The computer is the same as in
[Phase 2](#phase-2).

- `open -n <bundle> --env OPENCRAFT_PROFILES_DIR=<directory>` started each
  process, with `--args --login` for a start with no window. Thus launchd
  started the process (the parent was process 1), with no terminal, and
  the messages went to the log file
  ([decision 0024](../decisions/0024-log-file-with-no-terminal.md)).
- The profiles directory was a new directory, not a copy of the Phase 4
  directory. It had 2 profiles. The active profile had 2 pages, content on
  22 surfaces (labels and colours, and 4 image files: 3 on the first page),
  and 16 bindings of 9 action kinds. Two bindings had the kind
  `opencraft.volume`. The directory had no file `device-settings.toml` and
  no file `shell.toml`, thus the Dock icon showed. The Phase 5 profile had
  the same numbers of profiles, pages, surfaces, bindings, and volume
  bindings, but different content.
- The device was connected. A camera picture 10 seconds after each start of
  a measurement showed the profile on the device.
- To compare the signature, a copy of the same bundle got an ad-hoc
  signature with no hardened runtime (`codesign --force --deep -s -`). The
  code of the executable stayed the same. `open` started it in the same
  way.
- No person used the device or the window during a measurement. The owner
  used the computer for other work, and a second OpenCraft process, with a
  different profiles directory, ran at some times. It did not hold the device during a measurement. The script
  measures only the given process.

### Memory and processor use (2026-11-12)

`scripts/measure-footprint.sh --pid` made one sample each 5 seconds for 300
seconds, as in [Phase 5](#phase-5). The script got the process ID and not
the name of the application, because a second copy of `OpenCraft.app` was in
`/Applications`, and the name finds that copy first. Each measurement
started 60 seconds after the start of the application. For "Window closed",
the close button of the window got a click through the Accessibility API 60
seconds after the start, and the measurement started 30 seconds after that.
Each column is a new process.

| Measurement | Window open | Window closed | Start with `--login` | Start with `--login`, ad-hoc signature | Confidence |
|---|---|---|---|---|---|
| Processes | 1 | 1 | 1 | 1 | Measured |
| Physical footprint: minimum, median, maximum | 50.5, 50.5, 50.5 MiB | 32.8, 35.0, 35.0 MiB | 24.2, 24.2, 24.2 MiB | 23.9, 24.0, 24.1 MiB | Measured |
| Resident size, median | 110.8 MiB | 112.3 MiB | 93.5 MiB | 95.1 MiB | Measured |
| Processor time in 300 seconds | 0.65 s | 0.65 s | 0.69 s | 0.68 s | Measured |
| Processor use, mean (100 % is one core) | 0.22 % | 0.22 % | 0.23 % | 0.23 % | Measured |
| Processor use, maximum of a sample | 0.4 % | 0.4 % | 0.4 % | 1.6 % | Measured |
| Threads at the start and at the end of the measurement | 15, 15 | 14, 15 | 13, 14 | 13, 13 | Measured: `ps -M` |

- With no window, the footprint is 24.2 MiB (25.4 MB) after a start with
  `--login`, and 35.0 MiB (36.7 MB) after the window closed. With the window
  open, it is 50.5 MiB (53.0 MB). All are below the target of 80 MB.
  (Inference from the measured values)
- The mean processor use is 0.22 to 0.23 % of one core, with and with no
  window. This is below the target of 0.5 %. (Inference from the measured
  values)
- The bundle with the Developer ID signature and the hardened runtime and
  the copy with the ad-hoc signature gave the same values in the start with
  `--login`: 0.2 MiB difference in the footprint, and 0.01 s difference in
  the processor time. Thus the signature and the hardened runtime do not
  change the footprint or the processor use. (Inference from Measured)
- Compared with [Phase 5](#phase-5), the footprint is 3.5 MiB more with the
  window open, 3.6 MiB more after the window closed, and 3.6 MiB more after
  a start with `--login`. The processor use is 0.05 to 0.10 % of one core
  more. The A/B test above shows that the signature is not the cause. The
  code changed between the two builds (commits `836b202` and `096d30b`), the
  profile is different, and the process starts through launchd. Which of
  these is the cause is not known. (Inference)
- In the starts with `--login`, the engine thread used 0.54 s (Developer ID)
  and 0.47 s (ad-hoc) in the 300 seconds, and the volume handler used
  0.14 s in each. In Phase 5, they used 0.31 to 0.38 s and 0.10 s. The main
  thread used 0.00 and 0.02 s. (Measured: `ps -M` at the start and at the
  end of the measurement. Thread names: Inference, as in Phase 5)
- After the window closed, the footprint was 35.0 MiB in samples 1 to 34
  (approximately 3 minutes after the close), and 32.8 MiB from sample 35 to
  the end. In Phase 5 the maximum was also 1.5 MiB above the minimum.
  (Measured)
- One sample of the copy with the ad-hoc signature had 1.6 %: 0.08 s in 5
  seconds, 265 seconds after the start of the measurement, with no window.
  `sample` did not run, thus the cause is not known. The other 239 samples
  with a processor value in the 4 measurements had 0.4 % or less.

### Start time (2026-11-12)

The procedure is different from Phase 5. The bundle must start through
`open` and not from a terminal, and with no terminal the line of
`OPENCRAFT_START_TRACE` in the function `main` goes to the null device.
Thus the times come from the log file:

- The first line of each start gives the time of the system clock and the
  time from the start of the process to that line ("main began *N* ms").
  The shell writes this line after the signal handlers, the lock, and the
  log file, thus a short time after `main` starts. The start of the process
  is the time of the line minus *N*.
- The engine lines `connected` (the device is open) and `full draw done`
  (the last write of the first draw), and, with `OPENCRAFT_START_TRACE` in
  the environment of `open`, the lines `start-trace device ready` (the
  event `Connected`) and `start-trace first frame`.
- The engine lines have a resolution of 1 ms.

A script started the bundle, waited for the last line, stopped the process
with the signal TERM, and waited 3 seconds. A repeated start is the start of
a file that started before. A first start is the first start of a new copy
of the bundle, made with `ditto` from the same bundle, or the first start of
the bundle file in `target/release-bundle` in this test. Each time is from
the start of the process.

| Step | First start: 4 starts with the Developer ID signature | Repeated start with a window: 6 starts, minimum, median, maximum | Repeated start with `--login`: 6 starts, minimum, median, maximum | Confidence |
|---|---|---|---|---|
| The start line ("main began") | 575, 537, 271, 246 ms | 36, 40, 47 ms | 39, 40, 45 ms | Measured |
| `connected` in the engine log | 610, 571, 305, 279 ms | 69, 77, 97 ms | 71, 75, 86 ms | Measured |
| `full draw done` in the engine log | 639, 596, 333, 305 ms | 94, 103, 125 ms | 96, 101, 113 ms | Measured |
| The device is ready (the event `Connected`) | Not recorded, 596, 333, 305 ms | 94, 103, 125 ms | 96, 101, 114 ms | Measured |
| The first frame of the window | Not recorded, 1119, 628 ms, no window | 375, 395, 406 ms | No window | Measured |

- The first start in the table: the bundle file in `target/release-bundle`
  (with a window, with no `OPENCRAFT_START_TRACE`), then new copies 1 and 2
  (with a window), then new copy 3 (with `--login`).
- For a repeated start, the device is ready 94 to 125 ms after the start of
  the process, and the median of the 12 starts is 101 ms. This meets the
  target of 300 ms. (Measured)
- The first start of a new file does not meet the target: 305 to 639 ms to
  the ready device. Most of this time is before the start line: 246 to 575
  ms. (Measured)
- From the start line to the ready device, the time was 56 to 78 ms in the
  12 repeated starts: approximately 35 ms to `connected`, and 25 ms for the
  first draw. In Phase 5, from `main` to the ready device was 44 to 51 ms.
  (Measured)
- From the `open` command to the start of the process, the time was 21 to
  32 ms in a repeated start, and 47 to 74 ms in a first start. This time is
  not in the table. A user starts OpenCraft through the Finder, the Dock, or
  launchd, and that time is also before the start of the process. (Measured)
- The copy with the ad-hoc signature gave the same times. First start of a
  new copy: 550 ms to the start line and 619 ms to the ready device. 6
  repeated starts with a window: 38 to 47 ms to the start line, and 101 to
  111 ms to the ready device. 6 repeated starts with `--login`: 34 to 42 ms
  and 96 to 107 ms. Thus the Developer ID signature and the hardened runtime
  do not change the start time. (Inference from Measured)
- In Phase 5, a repeated start had 6 to 8 ms before `main`. Now it has 34 to
  47 ms before the start line. The signature is not the cause (the row
  above). The process now starts through `open` and launchd, and the start
  line comes after the lock and the log file. The share of each is not
  known. (Inference)
- The first start of a new file is the nearest value to the first start
  after an installation. A copy from the notarized disk image also has the
  quarantine attribute, and Gatekeeper checks it at the first start. That
  start is not measured.

### Lost input events (2026-11-12)

Not measured with the bundle: nobody turned a knob. The counts of
[Phase 3](#phase-3) and [Phase 4](#phase-4) came from the engine examples
`knob_count` (commit `757e400`) and `touch_count` (commit `d1d436a`).

- In `crates/engine/src/engine.rs`, the branch for an input report (count
  the report, decode it, send the event to the shell, and start the bound
  action) has not changed since commit `aa5cb65` (#88), before the count of
  Phase 3. The decoder in `crates/core/src/events.rs` has not changed since
  commit `f58ac34`. (Measured: `git log`)
- The read of the link (`next_event` in `crates/core/src/link.rs`) changed
  after the counts: commit `9af8370` added a check of the clock for a wake
  before each read. The read of a report itself did not change. (Measured:
  `git log` and the code)
- The bundle uses the same engine crate as the examples, and the window
  gets the events from the engine. Thus the input path of the bundle is the
  path that the counts measured, with the wake check added. (Inference)

### Product targets with the release bundle (2026-11-12)

| Target | Value with the release bundle | Result |
|---|---|---|
| Memory in the steady state: less than 80 MB | 25.4 MB after a start with `--login`, 36.7 MB after the window closed, 53.0 MB with the window open | Met |
| Processor use when idle: less than 0.5 % of one core | 0.22 to 0.23 % mean, with and with no window. No sample of 5 seconds above 0.4 % with the Developer ID bundle; one sample of 1.6 % with the ad-hoc copy. | Met |
| Cold start to a ready device: less than 300 ms | 101 ms median (94 to 125 ms) for a repeated start; 305 to 639 ms for the first start of a new file; not measured for the first start of the notarized bundle and after a start of the computer | Met for a repeated start only. Not met for the first start of a new file. Issue [#143](https://github.com/shantanugoel/OpenCraft/issues/143) and issue [#151](https://github.com/shantanugoel/OpenCraft/issues/151). |
| Lost input events: none | Not measured with the bundle. The input path is the path of the Phase 3 and Phase 4 counts. | Met in Phase 3 and Phase 4. Not measured with the bundle. |

## Icons on surfaces (2026-11-13)

The change is the icon set of the engine and the icon picker of
[decision 0033](../decisions/0033-surface-icons.md) (issue
[#190](https://github.com/shantanugoel/OpenCraft/issues/190)). The build is
the release profile of the workspace (`lto = "thin"`, `strip = true`),
`cargo build --release -p opencraft-app`, on the computer of
[Phase 2](#phase-2) (Apple M1 Max, macOS 27.2). The times come from a test
program, not in the repository, in a release build of the engine crate.

| Fact | Confidence |
|---|---|
| The release executable has 23,934,896 bytes before the change (commit `9d7b386`) and 24,563,760 bytes after it: 628,864 bytes (2.6 %) more. A first measurement on commit `fb1d202`, with an earlier version of the change, gave 23,768,848 and 24,464,080 bytes: 695,232 bytes (2.9 %) more. | Measured |
| The icon data `crates/engine/icons/lucide.tsv` has 499,309 bytes. The executable contains it as it is. The rest of the difference, 130,000 to 196,000 bytes, is code: the drawing of an icon with `resvg`, the search, and the icon picker. The two measurements are different, possibly because the link-time optimization puts the code in a different way. | Measured (the sizes); Inference (the parts of the rest and the cause) |
| No crate came into the executable. `resvg` 0.46 and its dependencies were in it before, through GPUI. | Measured: `cargo tree` |
| The engine reads the data in 1.1 ms, at the first use of an icon. | Measured |
| The drawing of one icon of 72 × 72 pixels takes 80 µs, the mean of all 1,818 icons. | Measured |
| The image of a key face of 112 × 112 with an icon and a label takes 0.67 ms. With the label only, it takes 0.23 ms. | Measured: the mean of 100 images |
| One search of all icons ("arrow") takes 0.39 ms. | Measured: the mean of 100 searches |
| The data of the set in memory, after the first use, is approximately 100 KB: one entry of 56 bytes for each icon, with references into the data of the executable. | Inference: the size of the entry |
| The memory and the processor use of the application with icons on 28 surfaces are in [UI overhaul](#memory-and-processor-use-2026-11-13). An icon changes only the time to make an image, as a label does. | Inference |

## UI overhaul (2026-11-13)

The window changed in the UI overhaul (issue
[#173](https://github.com/shantanugoel/OpenCraft/issues/173),
[decision 0029](../decisions/0029-window-layout.md)): three panes, the
library, the command palette, the Settings window, the first start window,
icons on surfaces, and page backgrounds. Issue
[#197](https://github.com/shantanugoel/OpenCraft/issues/197) measured the
release bundle again after the last change of the window.

### The release bundle (2026-11-13)

The application is the bundle `OpenCraft.app` of `scripts/release.sh`, from
the commit `259a20d`, with no `OPENCRAFT_NOTARY_PROFILE`: a Developer ID
signature and the hardened runtime, not notarized. The computer is the same
as in [Phase 2](#phase-2). The procedure is the same as in
[Phase 6](#phase-6), with these differences:

- `open -n <bundle> --env HOME=<directory> --env OPENCRAFT_PROFILES_DIR=<directory>`
  started each process. The home directory was a new directory, thus the
  files of the shell (`shell.toml`, the log file, the lock) were in it, and
  not in the folder of the owner.
- The profiles directory had 2 profiles: the starter profiles "Everyday
  Mac" (active) and "Media and sound" of the first start
  ([decision 0029](../decisions/0029-window-layout.md#18-the-first-start-196)).
  The active profile had 2 pages, content on 28 surfaces, each with a title
  and an icon of the built-in set, and 36 bindings of 5 action kinds. Six
  bindings had the kind `opencraft.volume`. No surface had an image file,
  and no page had a background. The Phase 6 profile had 22 surfaces with
  labels, colours, and 4 image files, and 16 bindings. The directory had no
  file `device-settings.toml`.
- The device was connected. A camera picture 10 seconds after each start,
  and 10 seconds after the start of each measurement, showed the profile
  on the device, also after the window closed.
- No person used the device or the window during a measurement. A lock in
  the scratch directory of the agents kept other OpenCraft processes away
  from the device. Other programs of the owner ran.

| Fact | Value | Confidence |
|---|---|---|
| The executable | 24,966,464 bytes. Phase 6: 22,016,160 bytes. | Measured |
| The bundle | 25,999,055 bytes in its files, 25 MB on the disk (`du`). Phase 6: 22 MB. | Measured |
| The disk image | 12,389,785 bytes. Phase 6: 12 MB. | Measured |

- The executable is 2,950,304 bytes (13 %) larger than in Phase 6. The
  icon data is 499,309 bytes of it, and the icon code is 130,000 to
  196,000 bytes ([Icons on surfaces](#icons-on-surfaces-2026-11-13)). The
  rest, approximately 2.3 MB, is the new window code and the starter
  profiles. (Measured: the sizes. Inference: the parts.)

### Memory and processor use (2026-11-13)

`scripts/measure-footprint.sh --pid` made one sample each 5 seconds for 300
seconds, as in [Phase 6](#phase-6). Each measurement started 60 seconds
after the start of the application. For "Window closed", the close button
got a click through the Accessibility API 60 seconds after the start, and
the measurement started 30 seconds after that. Each column is a new process.

| Measurement | Window open | Window closed | Start with `--login` | Confidence |
|---|---|---|---|---|
| Processes | 1 | 1 | 1 | Measured |
| Physical footprint: minimum, median, maximum | 54.6, 54.6, 54.6 MiB | 36.0, 36.0, 36.0 MiB | 26.4, 26.4, 26.8 MiB | Measured |
| Resident size, median | 117.4 MiB | 115.9 MiB | 101.8 MiB | Measured |
| Processor time in 300 seconds | 0.57 s | 0.67 s | 0.66 s | Measured |
| Processor use, mean (100 % is one core) | 0.19 % | 0.22 % | 0.22 % | Measured |
| Processor use, maximum of a sample | 0.4 % | 0.4 % | 0.6 % | Measured |
| Threads at the start and at the end of the measurement | 15, 15 | 16, 15 | 13, 13 | Measured: `ps -M` |

- The footprint is 57.3 MB with the window open, 37.7 MB after the window
  closed, and 27.7 MB after a start with `--login`. All are below the
  target of 80 MB. (Inference from the measured values)
- The mean processor use is 0.19 to 0.22 % of one core, with and with no
  window. This is below the target of 0.5 %. (Inference from the measured
  values)
- Compared with [Phase 6](#memory-and-processor-use-2026-11-12-2), the
  footprint is 4.1 MiB more with the window open, 1.0 MiB more after the
  window closed (Phase 6: 35.0 MiB, then 32.8 MiB after 3 minutes), and
  2.2 MiB more after a start with `--login`. The processor time is the same
  or less: 0.57 to 0.67 s in 300 seconds, in Phase 6 0.65 to 0.69 s.
  (Measured)
- The cause of the larger footprint is not measured. The profile is
  different (28 icons, no image files), the executable is 2.9 MB larger,
  and the window has more parts. (Inference)
- The engine thread used 0.47 to 0.49 s in 300 seconds, and the volume
  handler used 0.11 to 0.16 s, in all three measurements. The main thread
  used 0.00 to 0.02 s. In Phase 6 the engine thread used 0.47 to 0.54 s, and
  the volume handler 0.14 s. (Measured: `ps -M` at the start and at the end
  of each measurement. Thread names: Inference, as in Phase 5)
- Thus the window does not keep the processor awake when it is closed, and
  a closed window uses no more processor time than an open window. No timer
  of the window runs when no window is open. (Inference from Measured)
- One sample of the start with `--login` had 0.6 %: 0.03 s in 5 seconds.
  The other 179 samples with a processor value had 0.4 % or less.
  (Measured)

### Start time (2026-11-13)

The procedure is the same as in [Phase 6](#start-time-2026-11-12-1): the
start line of the log file, the engine lines `connected` and
`full draw done`, and the lines of `OPENCRAFT_START_TRACE`. A script
started the bundle with `open`, waited for the last line, and stopped the
process with the signal TERM. A first start is the first start of a new
copy of the bundle, made with `ditto`. Each time is from the start of the
process.

| Step | First start: 3 new copies (window, window, `--login`) | Repeated start with a window: 6 starts, minimum, median, maximum | Repeated start with `--login`: 6 starts, minimum, median, maximum | Confidence |
|---|---|---|---|---|
| The start line ("main began") | 582, 295, 292 ms | 35, 42, 49 ms | 33, 46, 52 ms | Measured |
| `connected` in the engine log | 633, 335, 337 ms | 84, 87, 93 ms | 62, 89, 95 ms | Measured |
| `full draw done` in the engine log | 666, 369, 368 ms | 115, 120, 124 ms | 93, 121, 126 ms | Measured |
| The device is ready (the event `Connected`) | 666, 369, 368 ms | 115, 120, 124 ms | 93, 121, 126 ms | Measured |
| The first frame of the window | 1268, 610 ms, no window | 375, 389, 411 ms | No window | Measured |

- For a repeated start, the device is ready 93 to 126 ms after the start of
  the process, and the median of the 12 starts is 120 ms. This meets the
  target of 300 ms. In Phase 6 the median was 101 ms. (Measured)
- From the start line to the ready device, the time was 60 to 82 ms in the
  12 repeated starts. In Phase 6 it was 56 to 78 ms. The first draw
  (`connected` to `full draw done`) was 31 to 33 ms; in Phase 6 it was
  approximately 25 ms. Each surface of this profile has an icon, and an
  image with an icon takes 0.44 ms more than an image with a label only
  ([Icons on surfaces](#icons-on-surfaces-2026-11-13)): 14 images take
  approximately 6 ms more. (Measured: the times. Inference: the cause)
- The first start of a new file does not meet the target: 368 to 666 ms to
  the ready device. Most of this time is before the start line: 292 to 582
  ms. This is the same as in Phase 6. (Measured)
- From the `open` command to the start of the process, the time was 22 to
  42 ms in a repeated start, and 60 to 79 ms in a first start. (Measured)

### Checks with the camera (2026-11-13)

A program for the window (Quartz events, not in the repository) used the
window of the release build with a scratch profiles directory, and the USB
camera recorded the device.

| Fact | Confidence |
|---|---|
| A new fill of a key from the window showed on the device, and ⌘Z put the old fill back on the device | Measured: camera |
| A click on a page in the sidebar showed that page on the device, with its background image behind the key faces and the bar panels with no fill | Measured: camera |
| "Test the action" on the card "Swipe left" of the page ("Change the page", next page) showed the next page on the device | Measured: camera |
| The knob lights in the mode "Colours" showed the four colours that the device inspector set | Measured: camera |
| ⌘Q made the displays and the knob lights dark. The next start showed the profile and the knob light colours again. | Measured: camera |
| With the window closed, the device showed the profile during 300 seconds | Measured: camera at the start of the measurement; the process ran to the end |
| Each key face and bar panel of the starter profile "Everyday Mac" showed its icon and its title on the device | Measured: camera |

### Product targets after the UI overhaul (2026-11-13)

| Target | Value with the release bundle | Result |
|---|---|---|
| Memory in the steady state: less than 80 MB | 27.7 MB after a start with `--login`, 37.7 MB after the window closed, 57.3 MB with the window open | Met |
| Processor use when idle: less than 0.5 % of one core | 0.19 to 0.22 % mean, with and with no window. No sample of 5 seconds above 0.6 %. | Met |
| Cold start to a ready device: less than 300 ms | 120 ms median (93 to 126 ms) for a repeated start; 368 to 666 ms for the first start of a new file | Met for a repeated start only, as in Phase 6. Issue [#143](https://github.com/shantanugoel/OpenCraft/issues/143) and issue [#151](https://github.com/shantanugoel/OpenCraft/issues/151). |
| Lost input events: none | Not measured: nobody turned a knob. The input path of the engine did not change in the overhaul. | Met in Phase 3 and Phase 4. Not measured with the bundle. |

## Not known

- The start time after a start of the computer, when the files of the
  application are not in the memory of the operating system. Issue
  [#143](https://github.com/shantanugoel/OpenCraft/issues/143) measures it.
  Start 1 of [Phase 2](#phase-2) and of [Phase 5](#phase-5), and the first
  starts of new files in [Phase 6](#start-time-2026-11-12-1), are the
  nearest values. A restart of the computer is necessary to empty the file
  cache: `purge` needs `sudo`, and no agent used it.
- The first start of the notarized bundle from the disk image, with the
  quarantine attribute: the time of the Gatekeeper check. This needs the
  notarization, and the notary credentials of the owner
  ([decision 0025](../decisions/0025-release-form.md)). Issue
  [#151](https://github.com/shantanugoel/OpenCraft/issues/151).
- Why a start through `open` has 34 to 47 ms before the start line, and a
  start from a terminal in Phase 5 had 6 to 8 ms before `main`. The
  signature is not the cause. See
  [Start time](#start-time-2026-11-12-1) of Phase 6.
- Why the release bundle uses 3.5 MiB more memory and 0.05 to 0.10 % of one
  core more than the build of Phase 5. The signature is not the cause. See
  [Memory and processor use](#memory-and-processor-use-2026-11-12-2) of
  Phase 6.
- Why the release bundle after the UI overhaul uses 1.0 to 4.1 MiB more
  memory than the bundle of Phase 6. See
  [Memory and processor use](#memory-and-processor-use-2026-11-13) of the
  UI overhaul.
- The cause of the samples with 1.2 % and 1.6 % of one core: see
  [Samples with high processor use](#samples-with-high-processor-use-2026-11-12),
  and the sample with 1.6 % in Phase 6.
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
