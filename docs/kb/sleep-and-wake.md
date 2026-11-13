# Sleep and wake of the computer: the clocks

Last update: 2026-11-14

This entry gives what the clocks of a process do when the computer sleeps.
The engine uses these facts to find a wake. See
[decision 0023](../decisions/0023-wake-from-the-sleep-time-of-the-clocks.md).
What the device and `hidapi` do at a sleep and a wake of the computer is
not known yet. See "Not known". A screen lock is in
[Checks by the owner (2026-11-14)](#checks-by-the-owner-2026-11-14).

## The clocks on macOS

Date: 2026-11-12.

| Fact | Confidence | Source |
|---|---|---|
| `std::time::Instant` on Apple platforms reads `CLOCK_UPTIME_RAW` | Source | Rust 1.99.1, `library/std/src/sys/time/unix.rs`, `Instant::CLOCK_ID` |
| `CLOCK_UPTIME_RAW` does not count while the computer sleeps. Its value is the value of `mach_absolute_time()`. | Source | macOS manual page `clock_gettime(3)` |
| Thus `Instant` does not count the time of a sleep on macOS. A timer that uses `Instant` is late by the time of the sleep. | Inference | The two rows above |
| `CLOCK_MONOTONIC_RAW` counts at the same rate as `CLOCK_UPTIME_RAW`, and it continues while the computer sleeps. Frequency and time adjustments do not change it. | Source | macOS manual page `clock_gettime(3)`: `CLOCK_UPTIME_RAW` counts "in the same manner as `CLOCK_MONOTONIC_RAW`, but … does not increment while the system is asleep" |
| `CLOCK_MONOTONIC` also continues during a sleep, but it is adjusted. On a computer that did not sleep for 6.9 days, it was 5.4 seconds less than `CLOCK_UPTIME_RAW`. | Source, and Measured | Manual page; the computer of the owner, 2026-11-12 |
| On the same computer, `CLOCK_MONOTONIC_RAW` minus `CLOCK_UPTIME_RAW` was less than 0.001 ms after 6.9 days with no sleep | Measured | The computer of the owner, 2026-11-12 |
| Thus `CLOCK_MONOTONIC_RAW` minus `CLOCK_UPTIME_RAW` is the total time of all sleeps since the start of the computer. It does not change while the computer is awake. | Inference | The rows above |
| The wall clock (`SystemTime`, `CLOCK_REALTIME`) counts during a sleep, but a person or the time service can change it | Source | macOS manual page `clock_gettime(3)` |

## The clocks on Linux

Date: 2026-11-12.

| Fact | Confidence | Source |
|---|---|---|
| `std::time::Instant` on Linux reads `CLOCK_MONOTONIC` | Source | Rust 1.99.1, `library/std/src/sys/time/unix.rs` |
| `CLOCK_MONOTONIC` does not count while the system is suspended. `CLOCK_BOOTTIME` is the same clock, but it also counts the time of a suspension. | Source | Linux manual page `clock_gettime(2)` |
| Thus `CLOCK_BOOTTIME` minus `CLOCK_MONOTONIC` is the total time of all suspensions | Inference | The row above |

The device layer function `computer_sleep_time` in
`crates/core/src/link.rs` gives these differences.

## Display notices (2026-11-13)

| Fact | Confidence | Source |
|---|---|---|
| The shell observer of `NSWorkspaceScreensDidSleepNotification` received a notice after `pmset displaysleepnow`. The engine made all displays and knob lights dark. | Measured | Development bundle, scratch profile, camera |
| After 8 seconds, `caffeinate -u -t 2` woke the computer displays. The shell received `NSWorkspaceScreensDidWakeNotification`, and the device showed the full green page and knob lights again. | Measured | Same run |
| The display sleep in this check did not lock the screen. It does not test a computer sleep or screen lock. | Measured | Same run; no lock or computer-sleep command was sent |

The shell also registers NSWorkspace will-sleep and did-wake notices and
the distributed names `com.apple.screenIsLocked` and
`com.apple.screenIsUnlocked`. The screen lock of the owner is in
[Checks by the owner (2026-11-14)](#checks-by-the-owner-2026-11-14).
A sleep of the computer stays in
[#143](https://github.com/shantanugoel/OpenCraft/issues/143).

## Checks by the owner (2026-11-14)

Date: 2026-11-14. Release build of the development bundle, one N4 Pro,
macOS 27.

| Fact | Confidence | Source |
|---|---|---|
| At a lock of the screen, the device went dark. After the sign-in, the device showed the profile again. | Measured | Owner check, [issue #255](https://github.com/shantanugoel/OpenCraft/issues/255) |
| After the action "Screen off", a press of a device key showed the profile again. | Measured | Owner check, issue #255 |
| With the screen-off timer at 1 minute, the device went dark, and a key press woke it. | Measured | Owner check, issue #255 |
| After the stop state (`DC`, then `HAN`), a key press lit the logo of the firmware, and no input report arrived at the computer. | Measured | Owner key press with a test program, not in the repository; [issue #323](https://github.com/shantanugoel/OpenCraft/issues/323) |
| The computer of the owner has the system setting `SleepDisabled` on. `IOPMSleepSystem` gives `kIOReturnNotPermitted` there, and the Apple menu shows Sleep as not available. A real sleep of that computer was not possible in the check. | Measured | `opencraft.log` of the check; `pmset -g` |

## Not known

- What macOS and the device do at a sleep and a wake: whether macOS removes
  the device and adds it again, and whether the device keeps its images.
  See [hidapi-macos.md](hidapi-macos.md#not-known) and
  [n4-pro-hardware.md](n4-pro-hardware.md#not-known). The check is issue
  [#143](https://github.com/shantanugoel/OpenCraft/issues/143).
- Whether the process operates for a short time during a sleep (for
  example Power Nap), and what the device gets then.
- The value of `CLOCK_MONOTONIC_RAW` minus `CLOCK_UPTIME_RAW` after a real
  sleep. The check of issue
  [#143](https://github.com/shantanugoel/OpenCraft/issues/143) gives it in
  the log line `wake detected`.

## Sources

- Rust standard library 1.99.1, `library/std/src/sys/time/unix.rs`, in the
  `rust-src` component of the toolchain.
- macOS manual page `clock_gettime(3)`.
- Linux manual page `clock_gettime(2)`:
  <https://man7.org/linux/man-pages/man2/clock_gettime.2.html>
