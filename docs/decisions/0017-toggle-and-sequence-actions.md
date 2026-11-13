# 0017: An action can contain actions, and the engine draws the state of a toggle

- Status: Accepted
- Date: 2026-11-12

## Context

Phase 4 adds an action with two states (toggle) and an action that is a
sequence of actions. An exit criterion is that a toggle shows its state on
the key face.

These two actions contain other actions. Rule 8 of
[ARCHITECTURE.md](../ARCHITECTURE.md#5-rules) applies: a handler has no
access to a different handler, and only messages go through the handler
interface. [Decision 0013](0013-action-feedback-on-a-surface.md) says that a
handler sends feedback as data and that the engine makes the image.
[Decision 0014](0014-action-settings-description.md) says that a kind
describes its settings as a list of fields, and that a missing input type is
added to the list.

## Decision

**An action in the settings.** The settings of an action can contain a
different action. In the settings, an action is a table with the keys `kind`
and `settings`. The engine does not read it, as it does not read each other
value of the settings.

**Two new input types** for a field:

| Input type | The user gives | Value in the settings |
|---|---|---|
| Action | One action: a kind and its settings | A table with `kind` and `settings` |
| Action list | A list of actions, in an order | A list of such tables |

The shell makes the form for a contained action from the fields of its kind,
with the same code as for the action of a binding.

**A request to do an action.** A handler can send the request "do this
action" to the engine, with its binding and the action. This is a request
from a handler to the engine, as in
[decision 0016](0016-pages-and-engine-requests-from-a-handler.md).

- The engine sends Invoke to the handler of the kind of that action, with
  the binding of the handler that asked.
- If no handler has the kind, the engine tells the shell about the problem.
- The handler that asked does not get a result. It does not know when the
  action is complete.

**The toggle.** The kind `opencraft.toggle` has these settings: the action
for "on", the action for "off", and an optional label for the state "on".

- The handler holds the state of each toggle binding in memory. A toggle
  starts in the state "off".
- At each Invoke, the handler changes the state, asks the engine to do the
  action for the new state, and sends the feedback.
- The handler keeps the state while the application runs. After a page
  change and a change back, the toggle shows the state from before. If the
  settings of the binding changed, the state is "off" again.
- The state is not in the profile file. It is the state of the toggle, not
  the state of the computer: the handler does not read the result of the
  two actions.

**The feedback of a state.** The feedback gets one more optional part:

| Part | Meaning |
|---|---|
| State | On or off. For "on", the engine draws a frame around the surface, in the colour of the label. For "off", it draws no frame. |

The handler of the toggle sends the state, and the label for "on" if the
user gave one.

Note (2026-11-13): issue
[#183](https://github.com/shantanugoel/OpenCraft/issues/183) added one more
optional part, so that the window can show the face of each state
([decision 0029](0029-window-layout.md#105-toggle-and-sequence-183),
section 10.5, and rule 11):

| Part | Meaning |
|---|---|
| Other state | The feedback of the state that the surface does not show now. |

- The engine makes the image of the surface from the feedback, as before,
  and also the image of the other state. With no other state part, the
  image of the other state has the same parts with the other state.
- The device gets only the image of the state now. The surface event to the
  shell has the image of each state, and the state now.
- The handler of the toggle sends the other state with each feedback. Thus
  in the state off, the shell can show the face on with its label.

**The sequence.** The kind `opencraft.sequence` has these settings: a list
of actions, and a delay between two actions in milliseconds.

- At an Invoke, the handler asks the engine to do each action in the order
  of the list, and it waits for the delay between two actions. It does this
  on a thread of its own, as [decision 0012](0012-action-handler-interface.md)
  tells a handler to do for long work.
- The sequence does not wait until an action is complete. The delay is the
  only tool for an action that needs time, for example the start of an
  application.

## Amendment: a delay before each step (#282)

The input type `TimedActionList` adds timing to an action list. The
sequence uses this type. Each step keeps the optional integer
`delay_before_ms` in its settings. The range is 0 to 10000 milliseconds.
This key is metadata of the containing list, not a field of the child's
kind. The sequence removes it before it asks the engine to do the child.

- The delay is **before** the step. This also permits a wait before the
  first step. There is no wait after the last step.
- A missing value is zero for the first step. For each later step, it is
  the sequence's existing `delay_ms`, with its existing default of 100.
  Thus an old profile has exactly the same timing, with no migration.
- The window reads the list type from the description, not the kind
  identifier. It adds a shared integer field to the open step's form.
- A new step gets an explicit delay. Before a move or removal, the window
  makes inherited delays explicit. A replacement kind keeps the delay.
  Copy and paste keep the settings, and undo keeps the whole action.
- No profile schema change is necessary: settings already keep arbitrary
  integer values. The table form of an action does not change.

## Results

- The toggle and the sequence are handlers in `opencraft-actions`. The
  engine has no code for them (rule 8). A plug-in can contain actions in
  the same way.
- A contained action can have each kind, also a toggle or a sequence. The
  settings are data of a limited size, so a sequence always ends.
- A handler does not get Appear for an action that a different action
  contains. A kind that needs Appear to show its feedback, for example the
  sound volume, shows no feedback there before its first Invoke.
- The feedback of a contained action shows on the surface of the binding
  that contains it.
- All toggles look the same on the device. If a toggle needs a different
  image for each state, add an image part to the feedback
  ([decision 0013](0013-action-feedback-on-a-surface.md)).
- The state of a toggle can disagree with the computer, for example after
  the user changes the same setting by a different method. A kind that
  reads the real state is a different kind with its own handler.
- A shell that does not know the two new input types shows a text field, as
  decision 0014 says. Such a shell cannot edit these actions.
