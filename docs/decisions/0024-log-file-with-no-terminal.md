# 0024: The messages go to a log file when the process has no terminal

- Status: Accepted
- Date: 2026-11-12

## Context

The shell and the engine write their messages to the standard error: a
connection, a disconnection, a wake, a full draw, the stop, and each
problem of the start. A start through launchd
([decision 0021](0021-start-at-login.md)) or through the Finder gives the
null device as the standard error. Then nobody can read the messages. With
no window open ([decision 0019](0019-engine-in-the-gui-process.md)), the
messages are the only record of what the engine did, for example after a
sleep and a wake of the computer.

## Decision

At the start, before each other message, the shell looks at its standard
error.

- If the standard error is the null device, the shell connects it to the
  file `opencraft.log` in the directory of the shell (the parent directory
  of the profiles directory, where `shell.toml` is). Each start adds its
  lines at the end of the file.
- If the standard error is a terminal, a pipe, or a file, nothing changes.
  A person who starts OpenCraft from a terminal sees the messages there.
- If the file is larger than 1 MiB at a start, the shell renames it to
  `opencraft.log.1`, and starts a new file. Only one old file stays.
- The first line of each start gives the time of the system clock, and the
  time from the start of the process to `main`. The time before `main` is a
  part of the start time (see [kb/footprint.md](../kb/footprint.md)).

The code is in `crates/app`, in the module `log_file`, for `cfg(unix)`. It
uses `libc`, and no new dependency. The engine does not change: it writes
to the standard error, as before.

The file contains no label, no binding, and no input of the user. The
engine writes a line only for an event that is not frequent.

## Results

- The owner and an agent can read what the engine did after a start at
  login, with no terminal.
- The shell now writes two files: `shell.toml` and the log file.
- The size limit applies at a start only. A process that runs for a long
  time and writes many lines can make the file larger than the limit. Look
  at this decision again if a message becomes frequent.
- A logging library, levels, and a setting for the log are not necessary
  now. Look at this decision again in Phase 6, when users who are not the
  owner need a way to report a fault.
