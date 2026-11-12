# 0015: The touch bar shows four bar panels, and the device identifies the touch gestures

- Status: Accepted
- Date: 2026-11-12

## Context

Phase 4 makes the touch bar a full control. The roadmap forces one decision:
one image across the full touch bar, or four bar panels only. The phase also
adds tap and swipe as gestures of a binding.

The facts are in [kb/n4-pro-hardware.md](../kb/n4-pro-hardware.md):

- The touch bar is a window on the display. A slot image for a bar panel has
  176 × 112 pixels. The background shows in the spaces between the bar
  panels.
- One slot image can go across the touch bar, but it does not fill the full
  width. One background image can fill the area of the touch bar.
- Each knob is below one bar panel. Bar panel N is the feedback surface of
  knob N ([decision 0013](0013-action-feedback-on-a-surface.md)).
- The device identifies the gesture when the finger lifts. A tap gives the
  code of one bar panel. A swipe gives a code for the direction and no
  position.

## Decision

**Surfaces.** The touch bar shows four bar panels only. A page has no image
across the full touch bar.

- The user edits a bar panel as the user edits a key face: label, colour,
  and image.
- The feedback of a knob changes one bar panel. The engine writes one small
  image for each change. With one image across the touch bar, each click of
  a knob would write the full image.
- A tap has one bar panel. Thus the image and the binding are on the same
  part of the touch bar.

**Controls and gestures.** A binding can have these touch gestures:

| Control | Gesture | Text in the profile file |
|---|---|---|
| Bar panel N | Tap | `tap` |
| Touch bar | Swipe to the left | `swipe-left` |
| Touch bar | Swipe to the right | `swipe-right` |

- A binding names the gesture with no position data, as in
  [decision 0012](0012-action-handler-interface.md).
- The engine uses the gesture that the device identifies. The engine does
  not calculate a gesture from the touch points.
- The start of a touch, the touch points, and the end of a touch with no
  identified gesture are input events for the shell only. A binding cannot
  have them.
- The feedback surface of a bar panel control is its bar panel surface. The
  touch bar control has no feedback surface.

## Results

- The engine, the profile file, and the window use one model for a key face
  and for a bar panel.
- No new protocol command is necessary.
- The spaces between the bar panels stay black. A picture across the full
  touch bar is not possible in this phase. If it becomes necessary, the
  background surface can show it behind the bar panels: see "Background
  image" in [kb/streamdock-protocol.md](../kb/streamdock-protocol.md#background-image).
- A gesture that the device does not identify does nothing. The knowledge
  base gives how frequently this occurs.
- A swipe has no position. Thus a swipe cannot be different for each bar
  panel.
