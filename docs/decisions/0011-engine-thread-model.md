# 0011: The engine has one thread, and the shell uses messages

- Status: Accepted
- Date: 2026-11-11

## Context

The engine owns the device (rule 1), and device I/O must not block the user
interface (rule 5). The device layer blocks and has no async runtime
([decision 0003](0003-blocking-device-layer.md)). These facts set limits:

- The device needs a keep-alive in each period of 15 seconds. `Link` sends it
  from `Link::next_event`. See
  [kb/streamdock-protocol.md](../kb/streamdock-protocol.md#keep-alive-and-stop).
- The `hidapi` crate does not permit the use of one handle from two threads
  at the same time: its handle type is `Send` and not `Sync`. Thus a read on
  one thread and a write on a different thread are not possible with one
  handle. `Link::next_event` blocks during a read.
- A full repaint writes for less than 50 ms. One key face needs approximately
  2 ms. See [kb/streamdock-protocol.md](../kb/streamdock-protocol.md#repaint-time).

## Decision

**One engine thread.** The engine has one thread. This thread owns the
`Link`, the profile, and the images of the surfaces. It does all device I/O,
makes the images, and writes the profile file. No other thread touches these.

**The loop.** The engine thread does this in a loop:

1. It obeys each command that waits.
2. It calls `Link::next_event` with a short time limit, the poll interval. The
   poll interval is 20 ms.
3. It does the work for the event: an input event, a connection, or a
   disconnection.
4. If the profile changed and no change came for 500 ms, it writes the file.

An input event returns from the read at once. Thus the poll interval adds no
delay to input. A command waits for the poll interval at most.

**Messages.** A shell holds a handle to the engine.

- Commands go through a channel of the standard library to the engine
  thread. A send does not block.
- Events go to a function that the shell gives at the start. The engine
  thread calls it. The function must return quickly: it puts the event in a
  queue of the shell and wakes the user interface thread.
- A message contains data only.

**The logic is separate from the thread.** The engine logic is a type that
gets a command or a link event and gives writes and events. The thread loop is
a thin layer around it. The tests call the logic directly, with the fake
transport and a fake clock, and they use no thread and no real wait
(rule 7).

**Long work.** The engine thread does no work that can take more than
approximately 1 second. Such work would make the keep-alive late and would
delay input. An action handler of Phase 3 that can block must have its own
thread.

**The stop.** When the handle goes away, or the shell sends the stop command,
the engine thread writes the profile if it changed, and stops.

## Results

- No lock is necessary: one thread owns all state of the engine.
- The order of writes to the device is the order in which the engine thread
  makes them. A draw cannot be in the middle of a different draw.
- The user interface thread never waits for the device or for a file.
- The engine thread wakes 50 times each second when idle. Each wake is small.
  Measure the processor use in Phase 2 and write it in `kb/`. If the value is
  too high for the target "almost zero", the alternatives are a longer poll
  interval, or a transport that can read and write on two threads. The change
  is inside the engine thread loop and the device layer. The messages do not
  change.
- The engine can go to a different process in Phase 5: the channel and the
  function become a connection between two processes, and the messages stay.
- Look at this decision again in Phase 3, when handlers can block, and in
  Phase 5, with the measured processor use with no window.
