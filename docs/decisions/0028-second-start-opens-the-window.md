# 0028: A second start opens the window of the OpenCraft that runs

- Status: Accepted
- Date: 2026-11-12

## Context

[Decision 0019](0019-engine-in-the-gui-process.md) permits one OpenCraft
process for each user. A second start writes "OpenCraft is already running."
to the standard error and stops. A person who starts OpenCraft from the
Finder, the Dock, or the Applications folder does not see the standard
error. Issue [#162](https://github.com/shantanugoel/OpenCraft/issues/162)
asks that a second start shows the window of the OpenCraft that runs, or a
message that the user can see.

The facts are in [kb/gpui.md](../kb/gpui.md#second-start-2026-11-12):

- A start of the bundle while it runs, through `open`, the Finder, or the
  Dock, starts no second process. macOS sends the reopen event to the
  process that runs. With no window open, the closure `on_reopen` of
  [decision 0020](0020-menu-bar-item-and-dock-icon.md) opens the window.
  This is also true when the Dock icon is hidden, and when the process that
  runs started from a terminal.
- A start of the executable from a terminal starts a second process. That
  process stops at the lock, and no window opens.

Thus only a start of the executable needs a new mechanism. The shell
already has a pipe and a thread for signals (decision 0020).

## Decision

**The process number in the lock file.** The process that gets the lock
writes its process number, as one line of decimal digits, into
`opencraft.lock`. It first removes the old content of the file.

**SIGUSR1 opens the window.** The shell sets a handler for SIGUSR1 with the
handler of SIGTERM, SIGINT, and SIGHUP. The thread of the signals sends the
shell command Open for SIGUSR1. This command is the same as "Open OpenCraft"
in the menu bar item. The shell sets the handlers before it locks the file.
Without a handler, SIGUSR1 stops the process.

**A second start.** A process that does not get the lock reads the number.
If a different process has that number, and the operating system gives the
name `opencraft` for it, the second process sends SIGUSR1 to it. Then it
writes "OpenCraft is already running. It shows its window now." to the
standard error, and stops with the exit status 0. If it cannot send the
signal, it writes a message that tells the user to use the menu bar item.

**A start at login sends no signal.** A start with `--login`
([decision 0021](0021-start-at-login.md)) must not open a window. It stops
at the lock, as before.

The code is in `crates/app`: `lock.rs`, `signals.rs`, and `lock_or_stop` in
`main.rs`. It uses `libc` and no new dependency.

The reasons:

- The reopen event of macOS covers each start of the bundle. The signal is
  only for a start of the executable, thus a small mechanism is sufficient.
- The pipe and the thread for the signals exist. SIGUSR1 adds one branch.
- A distributed notification of macOS needs more AppKit code, and it does
  not operate on Linux. A socket needs a new thread and a protocol.

## Results

- Each start of OpenCraft while it runs shows the window, except a start at
  login.
- The lock file now has content. A number from an earlier start stays in the
  file only if the process that holds the lock did not write its number, for
  example a build from before this decision. The check of the process name
  prevents a signal to a different program with that number.
- Each process of the same user can open the window with SIGUSR1. This has
  no risk: the menu bar item does the same.
- [Decision 0019](0019-engine-in-the-gui-process.md) stays correct for the
  lock. This decision replaces only what a second start does.
