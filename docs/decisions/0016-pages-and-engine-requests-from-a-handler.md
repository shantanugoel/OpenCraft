# 0016: The engine holds the active page, and a handler asks the engine with a message

- Status: Accepted
- Date: 2026-11-12

## Context

Phase 4 adds many pages to a profile, and an action that changes the page.
Until now the active page was the first page.

Rule 8 of [ARCHITECTURE.md](../ARCHITECTURE.md#5-rules) says that all actions
use one handler interface, and that only messages go through it. Thus the
action that changes the page cannot be code in the engine. Rule 10 says that
the engine can draw all surfaces again at a page change.

[Decision 0012](0012-action-handler-interface.md) gives the requests and the
messages of a handler. A handler gets Appear and Disappear for the bindings
of the active page only.

## Decision

**The active page.** The engine holds the number of the active page in
memory. The profile file does not contain it. After a start and after a
change of the profile, the active page is the first page.

**The shell shows the active page.** The window shows and edits the page
that the device shows. The commands that change a surface or a binding
change the active page. To edit a different page, the user selects it, and
the device shows it.

**Commands from the shell.**

| Command | Result |
|---|---|
| Select a page | That page becomes the active page |
| Add a page | A new page after the last page, empty or a copy of a given page. It becomes the active page. |
| Rename a page | The page has the new name |
| Move a page | The page has a new position in the list |
| Remove a page | The page is gone. A profile keeps one page at the minimum. |

**Events to the shell.** The engine sends the list of the page names and the
number of the active page after each change of them. After a change of the
active page, the engine sends the state of each surface and of each binding
of the new active page. The shell keeps the bindings of the active page
only.

**A page change.** The engine does these steps in this order:

1. It sends Disappear for each binding of the old page, and removes all
   feedback.
2. It makes the image of each surface for the new page.
3. It writes to the device only the images that are different from the
   images before, and then one commit. It does not send the start sequence.
4. It sends Appear for each binding of the new page.

If the number of the active page changes because the user moved or removed a
page, the engine does the same steps. Thus a handler never holds a binding
with an old page number.

**A request from a handler to the engine.** A handler can send a new kind of
message: a request to the engine. The first request is "select a page", with
one of these targets: the next page, the previous page, or a page number.
The next page after the last page is the first page, and the reverse.

- The engine obeys the request as it obeys the command of the shell.
- A request names no binding. It applies to the active profile.
- The built-in kind `opencraft.page` sends this request. It is a handler in
  `opencraft-actions`, as each other built-in kind.

## Results

- The engine has no code for a specific action kind (rule 8). A plug-in can
  change the page with the same message.
- The time of a page change is the write time of the images that changed.
  The measured times are in
  [kb/streamdock-protocol.md](../kb/streamdock-protocol.md#repaint-time).
- A binding for the page change is on a page. A new page that is a copy of
  a page has the same bindings. Thus the user makes the bindings for the
  page change one time.
- The user cannot edit a page that the device does not show. This keeps the
  window and the device the same, and it keeps the shell small.
- The feedback of a binding is gone after a page change. The handler sends
  it again after Appear, as after a start.
- More requests can follow with no change to the handler interface: see
  decisions [0017](0017-toggle-and-sequence-actions.md) and
  [0018](0018-many-profiles.md).
- Look at this decision again if the user needs a control that is the same
  on all pages.
