# 0012: An action handler gets requests and sends messages, on its own thread

- Status: Accepted
- Date: 2026-11-12

## Context

Phase 3 adds the actions. [Decision 0006](0006-action-kinds-and-handler-interface.md)
and rule 8 of [ARCHITECTURE.md](../ARCHITECTURE.md#5-rules) set the limits:
an action is data, all actions use one handler interface, and only messages
go through this interface. The interface must also be correct for a plug-in,
which can be a different process.

[Decision 0011](0011-engine-thread-model.md) gives the engine one thread, and
that thread must not do long work. Some actions can block, for example a
shell command. The device can send 59 reports in 1 second during a fast knob
turn: see [kb/n4-pro-hardware.md](../kb/n4-pro-hardware.md#controls-and-input-codes). The engine
must not lose one of them (quality goal 1).

[Decision 0008](0008-crate-boundaries.md) said that a built-in action starts
as a module of the engine, and that this decision looks at that again.

## Decision

**The data.**

- An action has a kind and settings. The kind is text with a namespace, for
  example `opencraft.hotkey`. The settings are a table of values: text,
  integer, boolean, list, and table. The engine keeps the settings and does
  not read them.
- A binding connects one gesture of one control on one page to one action. A
  binding names the gesture with no position data. Phase 3 binds these
  gestures: press and release of a key, and turn left, turn right, and push
  of a knob.
- The identity of a binding is the page, the control, and the gesture. Each
  message about a binding contains this identity.

**Requests: from the engine to a handler.**

| Request | When | Content |
|---|---|---|
| Appear | A binding of the kind is on the active page: at the start, and after the user makes or changes the binding | The binding and the settings |
| Disappear | The binding is gone or changed, or its page is not active | The binding |
| Invoke | The user did the gesture of the binding | The binding and the settings |
| Resolve | The user asked to correct the status of a kind. See [decision 0014](0014-action-settings-description.md). | The kind |

A change of the settings is a Disappear and then an Appear.

**Messages: from a handler to the engine.**

| Message | Content |
|---|---|
| Feedback | The binding, and what the surface shows. See [decision 0013](0013-action-feedback-on-a-surface.md). |
| Status | A kind, and its status. See [decision 0014](0014-action-settings-description.md). |
| Problem | A text for the user |

A handler can send a message at any time, not only after a request.

**The handler.** A handler is an object with three functions:

- It gives the description of each kind that it has. The description is data:
  see [decision 0014](0014-action-settings-description.md).
- It gets, one time at the start, a sender for its messages.
- It gets one request.

A handler has no other access to the engine. It cannot read the profile, the
device, or the images.

**Threads.** Each handler has its own thread and a queue of requests with no
size limit. The engine thread puts a request in the queue and continues. It
never calls the code of a handler, and it never waits for a handler. The
messages of all handlers go into one queue. The engine thread reads this
queue in its loop, with the commands. Thus a message waits for the poll
interval at most.

- A handler gets its requests in the order of the gestures.
- A slow handler delays only its own kinds. It cannot delay the keep-alive,
  the input, or a different handler.
- A handler that starts long work, for example a shell command, does that
  work on a thread of its own, so that its next request does not wait.

The engine logic stays separate from the threads, as in decision 0011. The
logic sends a request to a port and gets a message through a function. The
runtime connects each port to the thread of a handler. A test connects a
fake handler and uses no thread.

**No handler.** If no handler has the kind of a binding, the engine keeps the
binding and its settings, sends no request, and tells the shell that the
action is not available.

**The crate.** The built-in handlers are in a new crate, `opencraft-actions`
(`crates/actions`). It depends on `opencraft-engine` and uses only the
handler interface. `opencraft-engine` does not depend on it. The program
gives the list of handlers to the engine when it starts the engine. This
replaces the sentence in decision 0008 that a built-in action starts as a
module of the engine.

Note (2026-11-14): issue
[#366](https://github.com/shantanugoel/OpenCraft/issues/366).
Each built-in kind that has moved is one module in `opencraft-actions`.
The module holds the kind name, the description, a typed read of the
settings, and the work. A registry in that crate lists the modules. The
order in the registry is the order of the kinds in the library. Kinds that
share a handler keep one handler, one thread, and one queue. The handler
interface does not change. The engine does not change. A kind that is not
in the registry stays in the old tables. The first modules are the four
kinds of the open handler: open an application, quit an application, open
a URL, and a shell command. The five kinds of the key handler are modules
too: send a hotkey, type text, a media key, a mouse event, and a Mac
system function (issue
[#367](https://github.com/shantanugoel/OpenCraft/issues/367)). They keep
one handler, one thread, and one poll of the Accessibility permission.
The sound volume kind is a module too. It keeps the volume handler and
its poll.

## Results

- A new action kind is a new handler. The engine does not change (rule 8).
- The compiler refuses a built-in action that uses the internals of the
  engine, because those are not public.
- A plug-in host later is one more handler: it sends the same requests to a
  plug-in process and gets the same messages. The messages contain data only.
- The engine thread stays quick. Decision 0011 stays correct with handlers
  that block.
- The program that starts the engine names the handlers. For the GUI
  application, this is one line in `main`, and it is not action logic
  (rule 12).
- Each handler is one thread that sleeps when it has no request. Measure the
  footprint at the end of Phase 3.
- A message from a handler can wait 20 ms. If feedback during a knob turn
  looks slow, wake the engine thread for a message. The interface does not
  change for that.
- Look at this decision again when the plug-in protocol is designed.
