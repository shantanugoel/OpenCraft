# 0023: The link finds a wake from the sleep time of the clocks, and the engine then draws all surfaces again

- Status: Accepted
- Date: 2026-11-12

## Context

Quality goal 1 and rule 10 of the
[architecture](../ARCHITECTURE.md#5-rules) need a recovery after the
computer sleeps and wakes, with no user action. Task
[#127](https://github.com/shantanugoel/OpenCraft/issues/127) implements it.

What macOS and the device do at a sleep is not measured
([#43](https://github.com/shantanugoel/OpenCraft/issues/43)). Thus the engine
must recover in each of these cases:

1. macOS removes the device at the sleep and adds it again at the wake.
2. The handle stays, but the device got no keep-alive during the sleep. It
   stopped its USB connection, or it lost its images, and possibly the host
   sees no error.
3. The handle stays, and the device kept all.
4. The writes fail after the wake.

The facts about the clocks are in
[kb/sleep-and-wake.md](../kb/sleep-and-wake.md). `Instant` does not count the
time of a sleep on macOS and on Linux. Thus the keep-alive of the link, which
uses `Instant`, is late after a sleep by the time of the sleep. The timer of
the device continues, if the device has power.

## Decision

**Find the wake in the device layer.** `Clock::asleep` gives the total time
that the computer slept. `SystemClock` reads it with `clock_gettime`:
`CLOCK_MONOTONIC_RAW` minus `CLOCK_UPTIME_RAW` on macOS, and
`CLOCK_BOOTTIME` minus `CLOCK_MONOTONIC` on Linux. `Link::next_event` reads
it in each turn of its loop.

- If the value increased, the link moves the next keep-alive earlier by the
  increase. Thus the keep-alive follows the time of the device.
- If the value increased by more than `WAKE_THRESHOLD` (2 seconds), the
  link sends the keep-alive at once and gives `LinkEvent::Woke { slept }`.
  With no device connected, it tries to connect at once.

**The engine does the same as after a connection.** At `LinkEvent::Woke`,
if a device is connected, the engine sends `Event::Disconnected` to the
shell, sends the start sequence and all images (rule 10), and then sends
`Event::Connected`. The shell drops its input marks at
`Event::Disconnected`, because a release during the sleep is lost.

**The other cases use the paths that exist.**

- Case 1, and a device that stops its USB connection some seconds after the
  wake: the link sees the disconnection, connects again, and the engine
  draws all surfaces.
- Case 4: the link counts the write errors (`ERROR_LIMIT`), drops the
  device, and connects again. The engine draws all surfaces at the
  connection.

**Log lines.** The engine writes one line to standard error for each wake
(with the sleep time), connection, disconnection, full draw, and stop. The
probe prints the same events. A person can read the result of a test from a
log file.

## Rejected alternatives

| Alternative | Reason |
|---|---|
| A sleep and wake notification of macOS (IOKit `IORegisterForSystemPower`, `NSWorkspace`) | It needs platform code with a run loop or an AppKit observer, in the engine or in the shell. The shell must not hold device logic (rule 12). The clocks give the same information on one thread, with no run loop, also on Linux. |
| Find a wake from a gap of the wall clock between two turns of the loop | A change of the time of day by a person or by the time service would look like a wake. A long block of the engine thread would also look like a wake. The difference of two monotonic clocks changes only during a sleep. |
| Close the device and open it again at each wake | The device then shows a disconnection that did not occur, and the open can fail while macOS adds the device again. The link already connects again when a write fails. |
| A higher limit, for example 15 seconds | Whether the device keeps its state during a short sleep is not known. A full draw needs less than 50 ms. The sleep time does not change while the computer is awake (measured), so a low limit gives no false wake. |

## Results

- The engine draws all surfaces after each sleep of more than 2 seconds,
  also when the device kept all. The device shows the start sequence for
  less than 0.2 seconds.
- A timer of the engine that uses `Instant`, for example `SAVE_DELAY`,
  counts only the time awake. A pending write of the profile occurs after
  the wake. It is not lost.
- A device write that blocks after a wake blocks the engine thread. The
  transport has no time limit for a write. See
  [kb/hidapi-macos.md](../kb/hidapi-macos.md#not-known).
- On a platform with no such clocks, `Clock::asleep` gives `None`, and only
  the disconnection path recovers.
- Look at this decision again when issue
  [#43](https://github.com/shantanugoel/OpenCraft/issues/43) gives the facts.
  If macOS always removes the device at a sleep, the wake event only adds a
  log line.
