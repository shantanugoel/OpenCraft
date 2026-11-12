# 0019: The engine stays in the process of the GUI application

- Status: Accepted
- Date: 2026-11-12

## Context

The goal of Phase 5 is that OpenCraft operates all day with no window open.
The [deferred decisions](../ARCHITECTURE.md#7-deferred-decisions) asked if
the engine operates in the process of the GUI application, or in a
different process. Rules 2 and 3 of
[ARCHITECTURE.md](../ARCHITECTURE.md#5-rules) keep the two options open.

The facts:

- The application of Phase 4 is one process. It uses 47.5 MiB with the
  window open, 45.0 MiB when it is hidden, and 0.10 % of one core when it is
  hidden. See [kb/footprint.md](../kb/footprint.md#phase-4).
- A GPUI test program with no engine uses approximately 29 to 31 MiB after
  its window closes, and 18 MiB when it starts with no window. It uses
  0.003 % of one core with no window. See
  [kb/gpui.md](../kb/gpui.md#footprint-with-no-window).
- The [product targets](../ROADMAP.md#product-targets) are less than 80 MB
  and less than 0.5 % of one core.
- The menu bar item needs AppKit in a process that runs all the time. See
  [decision 0020](0020-menu-bar-item-and-dock-icon.md).
- The Accessibility permission attaches to the code signature of the
  program that sends the key events. See
  [kb/macos-permissions.md](../kb/macos-permissions.md#code-signature-of-a-development-build).
  The action handlers send these events, and they are in the engine
  process.

## Decision

**One process.** The engine, the built-in action handlers, and the shell
stay in the process of the GUI application, as in Phases 2 to 4. The
process continues when no window is open. The engine thread starts before
the GUI toolkit, as now.

**One process for each user.** Before the engine starts, the application
opens the file `opencraft.lock` in the parent directory of the profiles
directory. It locks the file with `flock(fd, LOCK_EX | LOCK_NB)` from the
`libc` crate, which the workspace already uses. If a different process
holds the lock, the application writes "OpenCraft is already running." to
the standard error and stops with the exit status 0. It does not start the
engine. The operating system removes the lock when the process stops.
Note (2026-11-12): if the application cannot open or lock the file for a
different reason, it writes a message and continues with no lock
(`lock_or_stop` in `crates/app/src/main.rs`).

The reasons:

- One process meets the targets with a large margin. With no window, the
  estimate is approximately 35 MiB after a window closed, and less after a
  start with no window. (Inference: the GPUI value with no window, plus the
  difference between the Phase 4 application and the GPUI test programs.
  Issue [#128](https://github.com/shantanugoel/OpenCraft/issues/128)
  measures the real value.) The measured value is in
  [kb/footprint.md](../kb/footprint.md#phase-5).
- A second process saves only a part of the GPUI memory, because the
  process with the menu bar item also needs AppKit. (Inference)
- A second process needs a protocol between the processes, a second
  executable, a start order, and a recovery when one process stops. The
  engine process would also need its own signature and its own
  Accessibility permission.
- The lock prevents a second engine, for example after a start at login and
  a start by hand. The device layer opens the device for exclusive use (see
  [kb/hidapi-macos.md](../kb/hidapi-macos.md)), thus a second engine would
  only show "not connected" and look for the device each second.

## Results

- The application needs no code for a connection between processes.
- The shell continues to use the engine through `EngineHandle` and messages
  only ([decision 0011](0011-engine-thread-model.md)). Thus a later move to
  two processes changes the handle and not the messages.
- A fault in the GUI code stops the engine too, and the device stops. Look
  at this decision again if this occurs in daily use.
- Look at this decision again if the measurement of issue #128 does not
  meet the targets with no window open, or when a plug-in host is designed.
- A second start of the bundle through the Finder or `open` activates the
  process that runs, and macOS starts no second process. (Inference: this
  is the normal behaviour of macOS for an application bundle. Not
  measured.) A second start from a terminal stops with the message.
