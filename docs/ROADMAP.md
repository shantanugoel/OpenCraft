# OpenCraft roadmap

The work is in phases. A phase has a goal, a scope, and exit criteria. A phase
has no calendar date. A phase is complete when all its exit criteria are true.
The [roadmap issue](https://github.com/shantanugoel/OpenCraft/issues/1) links to
the issues of each phase.

Each phase from Phase 2 adds a small feature through all layers: device layer,
engine, and GUI. The GUI is the only editor of the profile. See
[decision 0004](decisions/0004-gui-from-first-feature-phase.md).

The first use is a daily driver for one person. The aim after that is an
application that other persons can adopt.

## Status

| Phase | Name | State |
|---|---|---|
| 0 | Hardware gate | Complete |
| 1 | Device layer foundation | Complete |
| 2 | First GUI: see and paint the device | Complete |
| 3 | Actions for keys and knobs | Next |
| 4 | Pages and the touch bar | Not started |
| 5 | Background operation | Not started |
| 6 | First public release | Not started |

## Checks for each phase

Do these checks before you close a phase.

- [ ] All tests pass on a computer with no device.
- [ ] The documents agree with the code.
- [ ] New facts are in [kb/](kb/README.md).
- [ ] Each design decision of the phase has a record in [decisions/](decisions/README.md).
- [ ] From Phase 2: the footprint is measured and written in `kb/`.

## Product targets

Measure these from Phase 2. They are exit criteria for Phase 6.

| Target | Value |
|---|---|
| Cold start to a ready device | Less than 300 ms |
| Memory in the steady state | Less than 80 MB |
| Processor use when idle | Less than 0.5 % of one core |
| Lost input events | None |

The measured values are in [kb/footprint.md](kb/footprint.md).

## Phase 0: Hardware gate (complete)

**Goal.** Show that OpenCraft can operate the N4 Pro without the vendor software.

**Result.** The probe finds the device, reads the firmware version, paints all
14 key faces and bar panels, and decodes knob input. The facts are in
[kb/n4-pro-hardware.md](kb/n4-pro-hardware.md).

## Phase 1: Device layer foundation (complete)

**Goal.** The device layer is complete, correct, and testable for the N4 Pro.
This phase has no GUI. It is small, and it makes the GUI phases reliable.

**Scope.**

- Move the input codes from `events.rs` into the descriptor (rule 4).
- Give each control and each surface an identity in the descriptor. Connect
  each key control to its key surface.
- Change the names in the code to the terms in
  [ARCHITECTURE.md](ARCHITECTURE.md#3-terms).
- Put the transport behind an interface, and add a fake transport (rule 7).
- Add tests for the commands and for the input decoder. Use recorded reports.
- Draw the 800 × 480 background image.
- Detect a disconnection and connect again.
- Measure each item in the "Not known" lists of
  [kb/n4-pro-hardware.md](kb/n4-pro-hardware.md) and
  [kb/streamdock-protocol.md](kb/streamdock-protocol.md).

**Exit criteria.**

- [x] The probe draws on each surface and shows each gesture of each control.
- [x] The device layer has no check for a specific model outside the descriptor table.
- [x] The tests run with the fake transport.
- [x] The probe continues to operate after the device is disconnected and connected again.
- [x] The "Not known" lists for the device and the protocol are empty, or each remaining item has a reason.

**Decisions that this phase forces.** Image format for surfaces (PNG or JPEG).
The answer is JPEG. See
[decision 0007](decisions/0007-jpeg-for-key-faces-and-bar-panels.md).

**Result.** All exit criteria are true on 2026-11-11, and the
[checks for each phase](#checks-for-each-phase) are done.
[Issue #2](https://github.com/shantanugoel/OpenCraft/issues/2) has the record
of the checks.

- The measured facts are in [kb/n4-pro-hardware.md](kb/n4-pro-hardware.md)
  and [kb/streamdock-protocol.md](kb/streamdock-protocol.md). Each question
  that stays has a reason in the "Not known" list of its entry.
- One check of the phase is deferred: the device after the computer sleeps
  and wakes. It is issue
  [#43](https://github.com/shantanugoel/OpenCraft/issues/43) in Phase 5, which
  has the recovery after sleep in its scope.
- The reference for the product targets is measured. See
  [kb/vsd-craft.md](kb/vsd-craft.md#footprint-when-it-runs).

## Phase 2: First GUI: see and paint the device (complete)

**Goal.** A window shows the device. The user changes how a key looks, and the
device shows the change.

**Scope.**

- An engine that owns the device and obeys the rules in
  [ARCHITECTURE.md](ARCHITECTURE.md#5-rules).
- A window that shows the device layout: keys, touch bar, and knobs.
- The window shows live input. A pressed key shows as pressed.
- The user sets the label, the colour, and the image of a key face and of a bar panel.
- The profile is saved and loaded again. One profile, one page.

**Exit criteria.**

- [x] A change in the window shows on the device.
- [x] The device shows the saved profile after the application starts again.
- [x] The device shows the saved profile after a disconnection and a new connection.
- [x] The window stays responsive during a full repaint of the device.
- [x] The engine tests run with no window and no device.

**Decisions that this phase forces.** Crate boundaries. Form of the GPUI
dependency and the widget library. Profile file format. Library for text and
icons. Thread model in the engine. The answers are in decisions
[0005](decisions/0005-gpui.md), [0008](decisions/0008-crate-boundaries.md),
[0009](decisions/0009-profile-file-format.md),
[0010](decisions/0010-core-text-for-surface-labels.md), and
[0011](decisions/0011-engine-thread-model.md).

**Result.** All exit criteria are true on 2026-11-12, and the
[checks for each phase](#checks-for-each-phase) are done.
[Issue #59](https://github.com/shantanugoel/OpenCraft/issues/59) has the
record of the checks.

- The owner did the last checks on the device: the live input in the window,
  a change from the window, and the cable disconnected and connected again
  while the window moved.
- The measured footprint is in [kb/footprint.md](kb/footprint.md). The
  memory and the processor use are below their targets. The time to a ready
  device is below its target when the files of the application are in the
  memory of the operating system. The first start of a new build was not.
- One product target is not measured: lost input events. Phase 3 has an exit
  criterion for it.

## Phase 3: Actions for keys and knobs

**Goal.** The device does useful work. The owner uses OpenCraft each day in
place of the vendor software.

**Scope.**

- Bindings for key gestures and knob gestures.
- Action kinds, in this order: open an application, open a URL, send a hotkey,
  type text, media keys, sound volume, run a shell command.
- The user selects and configures an action in the window.
- Onboarding for the Accessibility permission. See
  [kb/macos-permissions.md](kb/macos-permissions.md).
- A stable code signature for development builds, so that the permission stays.

**Exit criteria.**

- [ ] Each action kind operates from a key.
- [ ] A knob changes the sound volume, and the bar panel shows the level.
- [ ] No input event is lost during a fast knob turn.
- [ ] A new action kind needs no change to the engine (rule 8).
- [ ] Each built-in action uses the handler interface only (rule 8).
- [ ] The engine keeps a binding that has an action kind with no handler (rule 8).
- [ ] The owner uses OpenCraft as the daily driver for one week with no blocking fault.

**Decisions that this phase forces.** Interface of an action handler. How an
action changes a surface. How an action describes its settings to the GUI. Make
each decision so that it is also correct for a plug-in. See
[decision 0006](decisions/0006-action-kinds-and-handler-interface.md).

## Phase 4: Pages and the touch bar

**Goal.** The profile has more than one page, and the touch bar is a full control.

**Scope.**

- Many pages in a profile. An action to change the page.
- Swipe on the touch bar as a gesture.
- Tap on a bar panel as a gesture.
- An action with two states (toggle). An action that is a sequence of actions.
- Many profiles, and a control to select one.

**Exit criteria.**

- [ ] A page change repaints the device in a time that the owner accepts. Write the time in `kb/`.
- [ ] Swipe and tap operate reliably, or the knowledge base gives the limit.
- [ ] A toggle shows its state on the key face.

**Decisions that this phase forces.** One image across the full touch bar, or
four bar panels only.

## Phase 5: Background operation

**Goal.** OpenCraft operates all day with no window open.

**Scope.**

- A menu bar item. An option to hide the Dock icon.
- Start at login.
- Recovery after computer sleep and wake.
- Device settings: brightness, haptic feedback, knob lights.
- A defined device state when OpenCraft stops.

**Exit criteria.**

- [ ] The device operates with no window open.
- [ ] The device operates after sleep and wake with no user action.
- [ ] The product targets are met with no window open.

**Decisions that this phase forces.** Engine in the GUI process or in a
different process. How to make the menu bar item.

## Phase 6: First public release

**Goal.** A person who is not the owner can install and use OpenCraft.

**Scope.**

- Signed and notarized application. Installation through Homebrew.
- User documentation.
- Licence notices for the dependencies.
- A procedure to add a new device model as data.

**Exit criteria.**

- [ ] All product targets are met.
- [ ] A new user installs OpenCraft and configures a key with the documentation only.

## Not in a phase

These items are not scheduled. The architecture keeps them possible.

- Plug-in host, and compatibility with Stream Deck plug-ins.
- Integrations with other applications, for example OBS.
- More device models.
- Windows and Linux.
- Import of profiles from the vendor software.
- Animation on surfaces.
