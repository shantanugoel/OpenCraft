# 0013: An action changes a surface with feedback data, and the engine makes the image

- Status: Accepted
- Date: 2026-11-12

## Context

Rule 8 says that a handler can send a message to change a surface at any
time. The first use is the sound volume action: the bar panel shows the
level. Rule 10 says that the engine holds the necessary state of each
surface. Rule 11 says that the engine makes the image one time, for the
device and for the shell.

A handler could send pixels. But then each handler needs code for text and
for the size of each surface, and a handler in a different process sends
much data for each change.

A knob has no display. On the N4 Pro, each knob is below one bar panel.

## Decision

**Feedback is data.** A handler sends a Feedback message for one of its
bindings. The feedback has these parts, and each part is optional:

| Part | Meaning |
|---|---|
| Label | A text. It replaces the label of the surface while the feedback applies. |
| Level | A value from 0 to 100. The engine draws it as a bar. |

A Feedback message with no feedback removes the feedback of that binding.

**The surface.** The feedback of a binding shows on the feedback surface of
its control. For a control that has a display, this is its surface. For a
knob, the descriptor names the surface: on the N4 Pro, knob N has bar panel
N. The handler does not name a surface, and it does not know the size of a
surface. If the control has no feedback surface, the engine ignores the
feedback.

**The engine makes the image.** The engine holds the feedback of each
binding in memory. It makes the image of the surface from the content in the
profile and from the feedback, in the same code that makes each other image.
It sends the image to the device and to the shell.

- The feedback is not in the profile file. The handler gets Appear again
  after a start and sends the feedback again.
- If two bindings have the same feedback surface, for example turn left and
  turn right of one knob, the newest feedback shows.
- The engine removes the feedback of a binding when the binding is gone or
  changed.
- A handler that wants the feedback for a short time only removes it after
  that time.

## Results

- The device and the window show one image (rule 11), and the engine can
  draw each surface again at any time (rule 10).
- A handler is small. It needs no image code and no text code.
- All actions look the same on the device.
- A plug-in sends little data.
- A handler cannot draw a picture of its own. When an action needs that, add
  an optional image part to the feedback. The old handlers stay correct.
- The descriptor gets one new field for a control: the feedback surface.
