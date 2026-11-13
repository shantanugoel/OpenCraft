# 0032: Each page has a background, and a surface with no fill shows it

- Status: Accepted
- Date: 2026-11-13

## Context

Issue [#191](https://github.com/shantanugoel/OpenCraft/issues/191) adds a
background image to each page. The window parts are in
[decision 0029](0029-window-layout.md), sections 8.2 and 10.6. The issue
asks for a decision about the fit of an image that is not 800 × 480.

The facts of the device are in
[kb/streamdock-protocol.md](../kb/streamdock-protocol.md#background-image)
and in [kb/n4-pro-hardware.md](../kb/n4-pro-hardware.md#surfaces):

- The device has one display of 800 × 480 behind the keys and the touch
  bar. Each key face and each bar panel is an image on this display.
- `BGPIC` sends a background JPEG of 800 × 480. `BGCLE` removes it. The
  key faces and the bar panels stay on top of the background.
- The window of each key shows only a few pixels around the image of its
  key face. The touch bar has spaces of 27 pixels between the bar panels.
- A key face image is a JPEG with no transparency
  ([decision 0007](0007-jpeg-for-key-faces-and-bar-panels.md)). A key face
  with no content is black.

Thus, if a key face covers the background with black, the user sees almost
none of the background.

## Decision

**The profile.** A page has an optional background: one colour, or an
image with a fit. It is the table `background` of the page in
`profile.toml`, schema version 4 (no release wrote version 4 before). The
keys are in `crates/engine/src/store.rs`. The background is a part of the
page, thus the undo history, a copy of a page, and the export and the
import of a profile ([decision 0031](0031-profile-export-file.md)) include
it with no special steps.

**The fit.** The fit tells how an image goes on the display when its
proportion is not that of the display:

| Fit | Result |
|---|---|
| "Fill" (the default) | The image covers all of the display, at the centre. The parts out of the display do not show. |
| "Fit" | All of the image shows, at the centre. Black fills the rest of the display. |

"Fill" is the default because the device then shows no black bands. Most
photos (3:2, 16:9) are near the proportion of the display (5:3), thus Fill
removes only a small part of them.

**The image copy.** A background copy is a PNG with no transparency (a
transparent pixel goes on black). It is the smallest size that covers the
background, and it is at most 2 times the background on a side. Thus each
fit has all the pixels that it needs, and the copy stays smaller than the
limit of an import (4 MB). A surface image copy stays at most 512 pixels
on a side (decision 0009). An import makes the copy of each image with the
limits of its use. An image with the two uses gets two copies.

**A surface with no fill shows the background.** The engine makes the
image of each surface from the part of the background at its position,
when the surface has no fill colour. The label, the image of the surface,
the level, and the frame of a toggle go on top of it. The automatic label
colour is black or white for the mean colour of that part. A surface with
a fill colour covers the background. Thus the background shows behind and
around each key face and bar panel, and the key faces still use JPEG.

**The engine.** The engine makes the background image one time. The device
and the window show that image (rule 11):

- `Command::SetBackground` sets or removes the background of a page.
  `Command::SetBackgroundFit` changes the fit. The names of the steps are
  "Set Background", "Remove the Background", and "Set Background Fit".
- `Event::Background` gives the background of the active page and its
  image, before the surfaces, when it changes.
- A full draw sends the background after the device settings: `BGPIC`
  for a page with a background, else `BGCLE 0x03`. It is not known if the
  device keeps a background from an earlier run of OpenCraft (kb), thus a
  page with no background removes it.
- A page change, an undo, and a change of the background send `BGPIC` or
  `BGCLE` only if the background changed, then the images of the surfaces
  that changed, then one commit.
- A model with no background in its descriptor has no background command
  and no "Background" section in the window.

**The window.** The canvas shows the engine image in the display, behind
the surfaces. The page picture of the page inspector shows it too. The
"Background" section of the page inspector follows decision 0029, section
10.6. Before the user picks a colour or an image, the segments "Colour" and
"Image" show their controls and send nothing.

## Rejected alternatives

| Alternative | Reason |
|---|---|
| Send a PNG with transparency for a surface with no fill, as the SDK does | Decision 0007 sends JPEG to each slot. A PNG of a key face can be larger than the largest slot image that the device showed. The image in the window would then not be the image on the device (rule 11). |
| Keep the black key faces on top of the background | The user sees almost none of the background on the keys. |
| "Fit" as the default | The display shows black bands at two sides for most images. |
| "Stretch" (change the proportion) | A photo looks wrong. The SDK does this, but the user cannot correct it. |
| Keep the original image file | The profile must not refer to a file of the user (decision 0009), and a large photo makes the profile directory and the export large. |

## Results

- A page change to or from a page with a background writes the background
  and most surfaces again. The camera measured 110 to 180 ms from the
  commit to a new background (kb). This is a little slower than a page
  change with no background.
- The engine decodes the background image of the active page at the start
  and at a page change. An 800 × 480 PNG decodes in a few milliseconds.
- A video background is a LATER place (decision 0029, section 21). The SDK
  sends one `BGPIC` for each frame of a video.
- No rule of [ARCHITECTURE.md](../ARCHITECTURE.md) changes.
