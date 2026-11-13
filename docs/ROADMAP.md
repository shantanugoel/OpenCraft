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
| 3 | Actions for keys and knobs | Complete |
| 4 | Pages and the touch bar | Complete |
| 5 | Background operation | Complete |
| 6 | First public release | Complete |
| — | UI overhaul, after Phase 6 | Complete, with owner checks open |

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

## Phase 3: Actions for keys and knobs (complete)

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

- [x] Each action kind operates from a key.
- [x] A knob changes the sound volume, and the bar panel shows the level.
- [x] No input event is lost during a fast knob turn.
- [x] A new action kind needs no change to the engine (rule 8).
- [x] Each built-in action uses the handler interface only (rule 8).
- [x] The engine keeps a binding that has an action kind with no handler (rule 8).

**Decisions that this phase forces.** Interface of an action handler. How an
action changes a surface. How an action describes its settings to the GUI. Make
each decision so that it is also correct for a plug-in. See
[decision 0006](decisions/0006-action-kinds-and-handler-interface.md). The
answers are in decisions [0012](decisions/0012-action-handler-interface.md),
[0013](decisions/0013-action-feedback-on-a-surface.md), and
[0014](decisions/0014-action-settings-description.md).

**Result.** All exit criteria are true on 2026-11-12, and the
[checks for each phase](#checks-for-each-phase) are done.
[Issue #83](https://github.com/shantanugoel/OpenCraft/issues/83) has the
record of the checks.

- The owner did three checks on the device: each action kind, the sound
  volume from a knob with the level on the bar panel, and the count of input
  events during fast knob turns. The record is in
  [kb/footprint.md](kb/footprint.md#checks-by-the-owner-1).
- The Accessibility permission stays after a new signed build. See
  [kb/macos-permissions.md](kb/macos-permissions.md#code-signature-of-a-development-build).
- For three criteria, the proof is in tests that run with no device. A new
  action kind: the engine test
  `a_new_kind_dispatches_settings_without_engine_changes`. The handler
  interface only: the crate `opencraft-actions` depends on `opencraft-engine`
  only, and the engine code contains no name of a built-in kind. A kind with
  no handler: the engine test
  `an_unavailable_kind_stays_in_the_file_and_reports_its_state`.
- The measured footprint is in [kb/footprint.md](kb/footprint.md#phase-3).
  The memory and the processor use are below their targets.
- Decision 0011 asked for a new look in this phase, because a handler can
  block. Decision 0012 gives each handler its own thread. The engine test
  `a_blocked_handler_does_not_stop_the_runtime_or_another_handler` shows that
  the thread model of decision 0011 stays correct.
- This phase had one more exit criterion: the owner uses OpenCraft as the
  daily driver for one week with no blocking fault. On 2026-11-12 the owner
  moved it to Phase 4, so that the work on Phase 4 can start during that
  week. The week started on 2026-11-12.

## Phase 4: Pages and the touch bar (complete)

**Goal.** The profile has more than one page, and the touch bar is a full control.

**Scope.**

- Many pages in a profile. An action to change the page.
- Swipe on the touch bar as a gesture.
- Tap on a bar panel as a gesture.
- An action with two states (toggle). An action that is a sequence of actions.
- Many profiles, and a control to select one.

**Exit criteria.**

- [x] A page change repaints the device in a time that the owner accepts. Write the time in `kb/`.
- [x] Swipe and tap operate reliably, or the knowledge base gives the limit.
- [x] A toggle shows its state on the key face.

This phase had one more exit criterion, which came from Phase 3: the owner
uses OpenCraft as the daily driver for one week with no blocking fault. On
2026-11-12 the owner removed it.

**Decisions that this phase forces.** One image across the full touch bar, or
four bar panels only. The answer is four bar panels only. See
[decision 0015](decisions/0015-four-bar-panels-and-touch-gestures.md). The
other decisions of the phase are
[0016](decisions/0016-pages-and-engine-requests-from-a-handler.md),
[0017](decisions/0017-toggle-and-sequence-actions.md), and
[0018](decisions/0018-many-profiles.md).

**Result.** All exit criteria are true on 2026-11-12, and the
[checks for each phase](#checks-for-each-phase) are done.
[Issue #108](https://github.com/shantanugoel/OpenCraft/issues/108) has the
record of the checks.

- The owner did the last checks on the device: taps and swipes through the
  engine, the page change, the toggle from a key and from a bar panel, a
  sequence, and a change of the profile. The record is in
  [kb/footprint.md](kb/footprint.md#checks-by-the-owner-2).
- The time of a page change is in
  [kb/footprint.md](kb/footprint.md#time-of-a-page-change-2026-11-12): the
  camera showed a new page less than 0.2 seconds after the start of the
  change.
- Tap and swipe were reliable in the count of the owner. See "Touch bar" in
  [kb/n4-pro-hardware.md](kb/n4-pro-hardware.md#controls-and-input-codes).
- The measured footprint is in [kb/footprint.md](kb/footprint.md#phase-4).
  The memory and the mean processor use are below their targets.
- The toggle, the sequence, the page change, and the selection of a profile
  are handlers in `opencraft-actions`. The engine contains no name of a
  built-in kind (rule 8).

## Phase 5: Background operation (complete)

**Goal.** OpenCraft operates all day with no window open.

**Scope.**

- A menu bar item. An option to hide the Dock icon.
- Start at login.
- Recovery after computer sleep and wake.
- Device settings: brightness, haptic feedback, knob lights.
- A defined device state when OpenCraft stops.

**Exit criteria.**

- [x] The device operates with no window open.
- [x] The device operates after sleep and wake with no user action. This is
  true in the engine tests, with a sleep that the test makes. The check with
  a real sleep of the computer is not done.
- [x] The product targets are met with no window open. This is true for the
  memory and for the processor use. The start time is below its target for a
  repeated start only.

On 2026-11-12 the owner moved three checks out of this phase: a real sleep
and wake, a start at a real login, and the cold start after a start of the
computer. The owner does them later, and they block no phase. They are in
[issue #143](https://github.com/shantanugoel/OpenCraft/issues/143).

**Decisions that this phase forces.** Engine in the GUI process or in a
different process. The answer is the GUI process: see
[decision 0019](decisions/0019-engine-in-the-gui-process.md). How to make
the menu bar item. The answer is AppKit code in the shell: see
[decision 0020](decisions/0020-menu-bar-item-and-dock-icon.md). The other
decisions of the phase are
[0021](decisions/0021-start-at-login.md),
[0022](decisions/0022-device-settings-and-stop-state.md),
[0023](decisions/0023-wake-from-the-sleep-time-of-the-clocks.md), and
[0024](decisions/0024-log-file-with-no-terminal.md).

**Result.** The phase is complete on 2026-11-12, with the limits that the
exit criteria give. The [checks for each phase](#checks-for-each-phase) are
done. [Issue #129](https://github.com/shantanugoel/OpenCraft/issues/129) has
the record of the checks.

- With no window open, the engine operated the device for 30 minutes after
  a start with `--login`, and the camera showed the profile on the device
  after a close of the window. See
  [kb/footprint.md](kb/footprint.md#phase-5) and
  [kb/gpui.md](kb/gpui.md).
- The measured footprint with no window is in
  [kb/footprint.md](kb/footprint.md#phase-5). The memory and the processor
  use are below their targets.
- A quit from the menu bar item, Cmd-Q, and a stop signal each make the
  device dark. See "Keep-alive and stop" in
  [kb/streamdock-protocol.md](kb/streamdock-protocol.md#keep-alive-and-stop).
- The owner did the check of the haptic feedback and of the dark state on
  the device. The camera measured the brightness and the knob lights.
- The start at login operated through launchd with a test file. The facts
  are in [kb/macos-start-at-login.md](kb/macos-start-at-login.md).
- What the link and the engine do at a wake is in
  [kb/sleep-and-wake.md](kb/sleep-and-wake.md).

## Phase 6: First public release (complete)

**Goal.** A person who is not the owner can install and use OpenCraft.

**Scope.**

- Signed and notarized application. Installation through Homebrew.
- User documentation.
- Licence notices for the dependencies.
- A procedure to add a new device model as data.

**Exit criteria.**

- [x] All product targets are met. This is true for the memory and for the
  processor use. The start time is below its target for a repeated start
  only. The count of lost input events is from Phase 3 and Phase 4.
- [x] A new user installs OpenCraft and configures a key with the
  documentation only. An agent that had only the user guide did this. No
  person other than the owner did it.

On 2026-11-12 the owner accepted these limits: the start time as in Phase 5,
and no check by a second person. The owner also moved two checks out of this
phase: the first start of the downloaded application, and the three actions
that need the Accessibility permission in the release bundle. They block no
phase. They are in
[issue #170](https://github.com/shantanugoel/OpenCraft/issues/170).

**Decisions that this phase forces.** The form of the release: see
[decision 0025](decisions/0025-release-form.md). The licence notices: see
[decision 0026](decisions/0026-licence-notices.md). The installation through
Homebrew: see [decision 0027](decisions/0027-homebrew-cask.md). One more
decision of the phase is
[0028](decisions/0028-second-start-opens-the-window.md).

**Result.** The phase is complete on 2026-11-12, with the limits that the
exit criteria give. The [checks for each phase](#checks-for-each-phase) are
done. [Issue #153](https://github.com/shantanugoel/OpenCraft/issues/153) has
the record of the checks.

- Version 0.1.0 is a signed and notarized bundle in one disk image for Apple
  silicon. The facts are in
  [kb/macos-distribution.md](kb/macos-distribution.md).
- A Homebrew cask in a tap of the owner installs the release. The facts are
  in [kb/homebrew.md](kb/homebrew.md).
- The user guide is in [user/](user/README.md). The result of the test with
  the guide only is in
  [issue #152](https://github.com/shantanugoel/OpenCraft/issues/152).
- The licence notices are in the bundle, and the menu bar item opens them.
  The facts are in [kb/dependency-licences.md](kb/dependency-licences.md).
- The procedure for a new device model is in
  [ADDING-A-DEVICE-MODEL.md](ADDING-A-DEVICE-MODEL.md). A test adds a second
  model as data only. No second real device was connected.
- The measured footprint of the release bundle is in
  [kb/footprint.md](kb/footprint.md#phase-6).

## UI overhaul, after Phase 6 (complete, with owner checks open)

**Goal.** The main window has a place for each later feature, and it looks
and operates like a good native macOS application. The umbrella issue is
[#173](https://github.com/shantanugoel/OpenCraft/issues/173). The milestone
"UI overhaul" groups its tasks.

**Scope.** Three panes (the sidebar, the device picture, the right panel
with the inspector and the library), one card for each gesture, drag and
drop from the library, undo and redo, copy and paste of a control, the
command palette, the icon picker with a built-in icon set, "Test the
action", the label options, a page background, the first start window with
starter profiles, the profile settings sheet with export and import, the
Settings window, the menu bar item with the profile and the brightness, the
design language, the keyboard operation, and the accessible labels.

**Decisions.** The layout is in
[decision 0029](decisions/0029-window-layout.md), and the design language is
in [DESIGN.md](DESIGN.md). The other decisions are
[0030](decisions/0030-undo-and-redo-in-the-engine.md),
[0031](decisions/0031-profile-export-file.md),
[0032](decisions/0032-page-background.md), and
[0033](decisions/0033-surface-icons.md).

**Result.** The tasks are complete on 2026-11-13, and the
[checks for each phase](#checks-for-each-phase) are done.
[Issue #197](https://github.com/shantanugoel/OpenCraft/issues/197) has the
record of the checks.

- The user guide in [user/](user/README.md) and its pictures show the new
  window.
- The measured footprint of the release bundle is in
  [kb/footprint.md](kb/footprint.md#ui-overhaul-2026-11-13). The memory and
  the processor use are below their targets, with and with no window. The
  start time is below its target for a repeated start only, as in Phase 6.
- The camera showed the changes from the window on the device: a fill, a
  page change, a page background, the knob light colours, "Test the
  action", and the dark device after a quit.
- The checks that need the hands of the owner are in a comment of issue
  #197: a key, a knob, and the touch bar from the device, a disconnection
  with a page background (#191), VoiceOver (#243), and a drag of the window
  with a real mouse (#176).

## Not in a phase

These items are not scheduled. The architecture keeps them possible.

- Plug-in host, and compatibility with Stream Deck plug-ins.
- Integrations with other applications, for example OBS.
- More device models.
- Windows and Linux.
- Import of profiles from the vendor software.
- Animation on surfaces.
