# OpenCraft architecture

This document gives the stable design of OpenCraft. It contains only the
decisions that are expensive to change. All other decisions wait for the phase
that needs them. See [Deferred decisions](#7-deferred-decisions).

A change to a rule in this document needs a record in
[decisions/](decisions/README.md).

## 1. Purpose

OpenCraft is a native application that operates StreamDock-class controllers
without the vendor software. The first device is the VSD N4 Pro. The first
platform is macOS.

## 2. Quality goals

The goals are in priority order. If two goals conflict, the higher goal wins.

1. **Reliable.** The application does not lose input. It recovers from a
   disconnection and from computer sleep without user action.
2. **Small.** The application uses little memory and almost no processor time
   when idle. This is the primary difference from the vendor software.
3. **Extensible.** A new device model is new data. A new action kind is new code
   in one place. The engine does not change for either.

## 3. Terms

Use these terms in the code and in the documents. Do not use a different word
for the same thing.

| Term | Definition |
|---|---|
| Device | One physical controller that is connected to the computer. |
| Descriptor | The static data that describes one device model. |
| Surface | An area of the device on which the application can draw. Examples: a key face, a bar panel, the background. |
| Control | A part of the device that the user operates. Examples: a key, a knob, a bar panel, the touch bar. |
| Gesture | One thing that the user does to a control. Examples: press, release, turn left, turn right, tap, swipe. |
| Input event | One gesture on one control, decoded from a device report. |
| Action | A unit of work, given as data: a kind and its settings. |
| Binding | The link from one gesture on one control to one action. |
| Page | One full set of bindings and surface content for a device. |
| Profile | A named set of pages for one device model. |
| Engine | The component that owns the device, the profile, and the actions. |
| Shell | A user interface that is a client of the engine. The GUI application is a shell. |

## 4. Layers

```
  +---------------------------+     +---------------------+
  |  GUI application (shell)  |     |  Probe (bring-up)   |
  +-------------+-------------+     +----------+----------+
                | messages                     |
  +-------------v-------------+                |
  |  Engine                   |                |
  |  profile, bindings,       |                |
  |  actions, rendering       |                |
  +-------------+-------------+                |
                |                              |
  +-------------v------------------------------v----------+
  |  Device layer                                         |
  |  descriptors, protocol, transport                     |
  +---------------------------+---------------------------+
                              |
                           USB HID
```

A layer can use the layer below it. A layer cannot use a layer above it.

| Layer | Knows about | Does not know about |
|---|---|---|
| Device layer | Device models, reports, surfaces, controls | Profiles, actions, the GUI |
| Engine | Profiles, bindings, actions, images for surfaces | The GUI toolkit |
| Shell | The engine messages, the GUI toolkit | Device reports |

The probe is a bring-up tool. It uses the device layer directly, and it takes
the place of the engine. Do not operate the probe and the engine on one device
at the same time.

### Code map

| Layer | Crate | State |
|---|---|---|
| Device layer | `crates/core` | Exists |
| Probe | `crates/cli` | Exists |
| Engine | `crates/engine` | Starts in Phase 2 |
| GUI application | `crates/app` | Starts in Phase 2 |

The reason for these crates is in
[decision 0008](decisions/0008-crate-boundaries.md). The GUI application does
not depend on `crates/core`.

## 5. Rules

These rules stay true in all phases.

### Ownership

1. **One owner.** Only the engine opens a device. One engine owns one device at
   a time.
2. **Messages.** A shell sends commands to the engine and receives events from
   the engine. A shell does not call the device layer.
3. **No window necessary.** The engine can operate when no window is open. This
   rule keeps a background agent possible.

### Device layer

4. **The descriptor is data.** The descriptor contains the surfaces, the
   controls, the input codes, and the capabilities of a device model. Code
   outside the descriptor table does not contain a check for a specific model.
5. **Device I/O does not block the user interface.** Device reads and writes
   occur on threads that are not the user interface thread.
6. **Do not open the keyboard interface.** The device has a keyboard-emulation
   interface. OpenCraft opens only the vendor interface. See
   [kb/macos-permissions.md](kb/macos-permissions.md).
7. **Tests without hardware.** The transport is behind an interface that has a
   fake implementation. All code above the transport can run in a test with no
   device.

### Engine

8. **An action is data, and all actions use one handler interface.** See
   [decision 0006](decisions/0006-action-kinds-and-handler-interface.md).
   - An action has a kind and settings. The kind is an identifier with a
     namespace, for example `opencraft.hotkey`.
   - The engine sends the settings to the handler for that kind and does not
     read them.
   - If no handler exists for a kind, the engine keeps the binding and its
     settings in the profile. The shell shows that the action is not available.
   - A built-in action uses the same handler interface that a plug-in will
     use. Only messages go through this interface.
   - A handler can send a message to change a surface at any time, not only
     after a gesture.
9. **The profile file is the source of truth.** The shell changes the profile
   through engine commands. The engine writes the file. The file has a schema
   version. The format is in
   [decision 0009](decisions/0009-profile-file-format.md).
10. **The engine holds the necessary state of each surface.** The engine can
    draw all surfaces again at any time. It does this after a connection, a
    page change, and a wake from sleep.
11. **One image, two destinations.** The engine makes the image for a surface
    one time. The device and the GUI show that same image. The GUI does not
    make its own copy.

### Shell

12. **The shell contains no device logic and no action logic.** It shows the
    state of the engine and sends commands. A different shell can replace it.

## 6. Data flow

**Input.** The device sends a report. The device layer decodes the report into
an input event. The engine finds the binding for that event on the active page.
The engine sends the action to its handler. The engine also sends the input
event to the shell, so that the shell can show it.

**Output.** A command or an action changes the necessary state of a surface. The
engine makes a new image for that surface. The engine sends the image to the
device layer and to the shell.

## 7. Deferred decisions

These decisions are not made. Each one waits for the phase that needs it. When
you make one, write a decision record and remove the row.

| Decision | Phase | Note |
|---|---|---|
| Form of the GPUI dependency, and the widget library | 2 | See [kb/gpui.md](kb/gpui.md). |
| Library for text and icons on surfaces | 2 | |
| Thread model in the engine | 2 | Rule 5 is the limit. |
| Engine in the GUI process, or in a different process | 5 | Rules 2 and 3 keep the two options open. |
| Menu bar item | 5 | GPUI has no API for this. |
| How an action describes its settings to the GUI | 3 | The answer must also be correct for a plug-in. |
| Plug-in protocol, language, package format, and installation | After the first release | Rule 8 keeps this possible. |
| Compatibility with Stream Deck plug-ins | After the first release | This is a different feature from our own plug-ins. See [kb/prior-art.md](kb/prior-art.md#stream-deck-plug-in-model). |
| Async runtime | When a need shows | See [decision 0003](decisions/0003-blocking-device-layer.md). |
| Animation on surfaces | After the first release | |
| More device models, Windows, Linux | After the first release | Rule 4 keeps new models low in cost. |
