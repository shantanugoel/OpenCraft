# 0042: Gesture timers, hold, two-state faces, multi-step feedback, and knob functions

- Status: Accepted
- Date: 2026-11-21

## Context

Phase 10 ([#202](https://github.com/shantanugoel/OpenCraft/issues/202)) makes
one key or one knob do more than one action. This record is the decision
P10-D1. It gives the gesture timers, the hold, the two-state key faces, the
multi-step feedback, the multi-function knob, and the profile schema
version 7. The features are KEY-10, P10-T1, KEY-09, AUTO-10, PLUG-26, and
KNOB-03.

The device reports a press and a release for a key. Its reports have these
limits:

- A key that is held sends one press and one release. It sends no repeat.
- A knob push sends one report. The device sends no report for the release.
- Two adjacent keys that go down together give a press and a release 0 to
  31 ms apart. The device sends nothing for the second key.
- A press report can come from a key that was down before.

These facts are in
[kb/n4-pro-hardware.md](../kb/n4-pro-hardware.md#controls-and-input-codes).
Thus the long press, the double press, and the hold are state and timers in
the engine. The device cannot make them.

The present interface has one gesture for one binding: press and release of a
key ([decision 0012](0012-action-handler-interface.md)). Feedback is data,
and the engine makes the image
([decision 0013](0013-action-feedback-on-a-surface.md)). A kind describes
its settings as fields
([decision 0014](0014-action-settings-description.md)). A toggle keeps its
state in its handler
([decision 0017](0017-toggle-and-sequence-actions.md)). The window has one
canvas and one inspector for each control
([decision 0029](0029-window-layout.md)). A different program can change the
profile through a socket ([decision 0040](0040-control-socket.md)).

Rule 8 of [ARCHITECTURE.md](../ARCHITECTURE.md#5-rules) says that the engine
does not read the settings. Rule 12 says that the shell contains no action
logic. A gesture timer must know what a key can do before the user releases
the key. Thus the engine gets these facts as static data of the kind, not from
the settings.

## Decision

### 1. The gesture timers (`crates/engine/src/gestures.rs`)

**Form.** The module is a state machine with no I/O, as `front_switch` is. Its
inputs are one input event, `now`, and the facts of the key. Its outputs are a
list of dispatches and the next deadline. The engine sends the requests.

**The wait in the engine.** `Engine::poll` waits for
`min(timeout, idle_deadline, gestures.deadline())`. After the wait, the engine
first runs each timer with a deadline at or before `now`, then it handles the
event. The tests use `FakeClock`.

**The constants.** Phase 10 has no user setting and no profile key for them.

| Constant | Value | Counted from |
|---|---|---|
| `LONG_PRESS_THRESHOLD` | 500 ms | The Press report. The long press runs while the key is still down, not at the Release. |
| `DOUBLE_PRESS_WINDOW` | 300 ms | The first Release report, not the first Press. |

The window counts from the Release, thus its length does not depend on the
time of the first press. The worst jitter is 31 ms, thus the user has at least
269 ms. A normal gap between two presses is 100 to 200 ms. A window of 350 ms
would add 50 ms to each single press on each key with a double-press binding,
for a small gain. P10-M1 measures the latency, and it can change these two
values.

**The facts of a key.** The engine reads them at Press, from the active
location.

| Fact | True when |
|---|---|
| `L` | The key has a `LongPress` binding, or the kind of its Press binding has `handles_long_press` |
| `D` | The key has a `DoublePress` binding |
| `H` | The kind of its Press binding has `receives_release` |

**What each set of facts does.**

| Facts | At Press | At Release | When a timer ends |
|---|---|---|---|
| `None` | Press at once, 0 ms | Release at once | No timer |
| `H` | `HoldStart` at once | `HoldEnd`, then the Release binding | No timer |
| `L` only | Start the long timer. Nothing runs. | Before the timer: Press, then Release | Long timer: `LongPress`. The Release after it runs nothing. |
| `D` only | Wait. Nothing runs. | Open the window. Nothing runs. | Window: Press, then Release |
| `L` and `D` | Start the long timer | Before the timer: open the window | Long timer: `LongPress`, and no window. Window: Press, then Release |
| Second Press in the window | `DoublePress` at once. The key then ignores input until its Release. | Nothing runs | No timer |

- A long press replaces the whole short gesture. After `LongPress`, neither
  the Press action nor the Release action runs. This meets the exit criterion
  "the short-press action does not run".
- `H` wins over `L` and `D`. The engine keeps the two bindings in the profile,
  but they never run. The engine reports this as
  `BindingState::inert_by: Option<GestureKind>`, here `Some(Press)`. The shell
  shows "Does not run: the Press action holds the key" for such a binding. The
  shell does not know the rule (rule 12).
- The Release belongs to the Press before it. A Release runs only while
  the key is still in the Down state of that Press. If a transition or
  another cancel ends that Down state (for example when the Press ran
  "Next page"), the engine drops the Release, thus it does not run the
  Release action of the new page.

**The reports of the device that need a rule.**

| Report | Result |
|---|---|
| Any input on a different control (a key, a knob, or a touch) while a double-press window is open | The window ends first as a single press: Press, then Release. Then the engine handles the new input. The order of the gestures of decision 0012 stays correct. |
| A Press on a key that is already down | Ignore it. The state is idempotent. A second `Invoke` of a toggle is worse than a lost press. |
| A Press on key B while key A is down, with no Release of A | Treat it as a lost Release of A: `HoldEnd` if A holds, then cancel the rest. No Press action and no Release action of A runs. Then the engine handles B. |
| A Release with no Down state for that key | Drop it. This covers a Release after a cancel, after a wake from the dark state, and at the start. |
| The 0 to 31 ms press and release pair of two adjacent keys | It is a very short tap. Do not debounce it: quality goal 1 says that the application does not lose input. |
| Down A, down B, release A, up B, press A again | Handle it as reported: the hold ends and starts again, and a double press can occur. This is a known limit of the device. See [kb/n4-pro-hardware.md](../kb/n4-pro-hardware.md#controls-and-input-codes). |

**What cancels a pending gesture.** Cancel means: the timers stop, the
waiting Press does not run, the key ignores input until its Release, and an
active hold gets `HoldEnd`.

- `transition_with`, for each key with a binding in `gone` or `arrived`. This
  covers a change of the page, the folder, or the profile, undo and redo, and
  a change of the model.
- `set_binding`, a paste, and a copy to all pages, for the controls that they
  change.
- A `DisplayState` that is not Awake, and a lock or a sleep.
- `LinkEvent::Disconnected`.
- `Engine::finish`.

A Press that runs "Next page" causes a transition. Thus the engine drops its
own Release, as the rule above says.

**The idle timer.** A key that is down keeps the device awake. The idle
deadline cannot end while a key is down, thus a long push-to-talk does not dim
the device.

**`GestureKind`.**

- Append `LongPress` and `DoublePress` at the end of the enum. The `Ord` order
  and the `BTreeMap` order do not change.
- The text form is `long-press` and `double-press`. The socket protocol of
  [decision 0040](0040-control-socket.md) uses `long_press` and
  `double_press`.
- `of_control(Key)` gives `&[Press, Release, LongPress, DoublePress]`. `Press`
  stays first, because `view/states.rs` and `library::first_gesture` need it.
- `from_gesture` never returns the two new variants. Only the engine makes
  them.
- The key inspector shows "Long press" and "Double press" in the gesture name
  column with `GESTURE_NAME_WIDE` (84 px). The two names then fit with no wrap
  and no clip.

### 2. The hold of a hotkey (P10-T1)

**The interface.**

- `KindDescription::receives_release: bool`. It is static data.
- `Request::HoldStart { binding, action }`.
- `Request::HoldEnd { binding }`. It has no action: the handler releases what
  it pressed for that binding.

**A separate kind.** The kind is `opencraft.hold-hotkey` ("Hold a hotkey"), in
`Group::Key`. It gives `receives_release: true` and
`states: Some(TwoStates { on_label: None })`. The reasons:

- If the hold were a setting of `opencraft.hotkey`, the engine would read a
  setting to know if the Press waits. That breaks rule 8.
- A static flag on `opencraft.hotkey` would make each normal hotkey Press
  block the long press and the double press.
- A separate kind still gives "key down at Press, key up at Release, in one
  binding".

**A hold cannot stay down.** Four rules give this:

1. Only `gestures.rs` sends `HoldStart`, and only for the Press binding of a
   key. A hold kind on a different gesture, for example Release, LongPress,
   or DoublePress, gets `Invoke`.
2. `Invoke` is always a complete action. For a hold kind it is a tap: key
   down, then key up. `TestAction`, a nested `EngineRequest::Invoke`, and a
   binding on Release, LongPress, or DoublePress never leave a key down.
3. The engine keeps `held: BTreeMap<ControlId, (BindingId, String)>`. Each exit
   path sends one `HoldEnd` to the same kind: the Release, a cancel, a lost
   Release, a disconnect, and `Engine::finish`. `HoldEnd` goes before the
   `Disappear`. At the stop, the runtime delivers the queued requests before
   it joins the handler threads.
4. The handler is a second guard. A `HoldStart` on a binding that already
   holds releases first. A `Disappear` of a holding binding releases it. A
   `Drop` releases all keys. An `Invoke` on a holding binding does nothing.
   The release order is the key first, then the modifiers.

A property test sends random events and cancels. For each `HoldStart`,
exactly one `HoldEnd` must follow.

**The behaviour.** A hold never outlives its binding on the device. A change
of the page, the folder, or the profile, a removal of the binding, the dark
state, a disconnect, and the stop end the hold at that moment and drop the
Release after it.

The kind declares `states`, and it sends `On` while it holds and `Off` at the
end. With section 4, the user can give a push-to-talk key its own face.

### 3. The long press that resets a timer (PLUG-26)

**The interface.**

- `KindDescription::handles_long_press: bool`.
- `Request::LongPress { binding, action }`. The `binding` is the Press binding,
  with the gesture `Press`, thus the handler finds its memory and its feedback
  surface.

**The rules.**

- The flag applies only to a kind on the Press binding of a key. It makes `L`
  true for that key.
- An explicit `LongPress` binding wins. The engine then runs that binding, and
  the handler gets no `Request::LongPress`. A binding that the user made is
  stronger than a behaviour of a kind.
- The Press card can show a fixed note from the `help` of the kind.

### 4. The two faces of a two-state key (KEY-09)

**The data.**

- `SurfaceContent::on: Option<Box<SurfaceContent>>`.
- The inner `on` is always `None`. The store refuses a nested `on`, thus the
  file has an error (decision 0009). `Page::update` removes one.
- The file form is `[pages.surfaces.key-face-3.on]`, with the keys of a face:
  label, label options, colour, image or icon, and icon colour. A folder uses
  the same form.
- The store accepts `on` on each surface. In Phase 10 the window edits it for
  the key faces only.

**The meaning.**

- The On face fully replaces the base face. It is not an overlay of the base
  face, because an overlay cannot say "no image in On".
- The engine command that makes an On face copies the base face. The shell
  thus has no copy rule.
- `on` is part of `is_empty`, `image_paths`, `image_uses`, `map_images` (the
  export and the import), and the removal of unused images. The undo history
  and copy and paste copy the content, thus they cover it.
- The engine uses `content.on` when the newest feedback on the surface has
  `state == On` and `content.on.is_some()`. Else it uses the base face.
- `StateImages` makes the off image from the base face and the on image from
  the On face. Rule 11 stays true.

**The frame rule. The 6 px `FRAME` draws only when the surface has no On face.
An On face replaces the frame.**

- [Decision 0017](0017-toggle-and-sequence-actions.md) added the frame because
  the two states otherwise looked the same. An On face removes that reason.
- The frame takes 12 px from each side of 112 px, about 20 % of the area. A
  custom image would change size between the two states.
- The automatic black or white frame can clash with the colours of the user.
- A rule "only when On has no custom colour" is hard to explain: is a face
  with only a new label custom?
- An option needs a schema key and a control in the window, for a small gain.

**The labels.**

- `Feedback::label` replaces the label of the face that shows, base or On. This
  is the rule of decision 0013, not changed.
- `toggle.on_label` stays a feedback label, thus it wins over the label of the
  On face. The window keeps the editor of the On label on the `on_label`
  setting.
- `microphone_mute` and `hold-hotkey` declare
  `states: Some(TwoStates { on_label: None })`, thus the window shows the face
  of each state for them. `microphone_mute` keeps its "Microphone muted" and
  "Microphone on" labels.
- The level bar draws on top of either face.

### 5. The multi-step feedback (AUTO-10) and the multi-function knob (KNOB-03)

**The step part.** `Feedback::step: Option<usize>` is the index from 0 of the
step that the next `Invoke` runs. The engine does not draw it. It is editor
data, not image data, thus rule 11 does not change.

**The propagation.** When the step part of the feedback of a binding changes,
the engine sends `Event::Step { binding, step }`. It sends `step: None` when
the feedback is removed or the binding disappears. The engine forwards the
data and reads no settings (rule 8).

**The step field of a kind.** `KindDescription::steps: Option<String>` names
the list field (`ActionList`, `TimedActionList`, or `TurnList`) whose item
number is `step`. The shell marks that row ("Next" for a step list, or the
active function for a turn list). It does this only when the top-level kind
declares `steps`. Without the field the shell ignores the step, which also
covers a cycle in another action (rule 12).

**`opencraft.cycle`.**

- The handler stores the position as a number, keyed by `(binding, settings)`,
  as the toggle does. It never searches a step by equality, thus two equal
  steps stay apart. A change of the settings sets the position to 0.
- The kind gives `steps: Some("steps")`.

**`opencraft.knob-functions`.**

- `KindDescription::knob_group: bool` says that the kind takes TurnLeft,
  TurnRight, and Push with one action.
- The knob inspector shows one "Functions" card in place of the Turn card and
  the Push card. A change writes the same action to the three bindings in one
  `Command::Group`. A removal removes all three. If the three bindings are not
  equal, for example after an edit through the socket, the inspector shows the
  normal cards, as for `turn_pair`.
- The key inspector does not offer the kind.
- `steps: Some("functions")` lets the editor mark the active function.
- The new `InputType::TurnList` is a list of
  `{ label, left = { kind, settings }, right = { kind, settings } }`. The
  window edits an item with the existing turn-card code (`turn_actions` with
  `Direction`). The handler reads `left` and `right` only, thus it needs no
  direction logic.
- The handler keys its memory by `(page, folder, control)` with no gesture.
  Push adds 1 to the index, wraps around, and sends the feedback `{ label,
  step }`. A turn sends
  `EngineRequest::Invoke { binding: <turn binding>, action: left or right }`.
  At Appear the handler sends the current label and step.
- The engine does not change. There are three ordinary bindings, thus the rule
  "the newest feedback shows" of decision 0013 applies: a volume level
  replaces the function label during a turn, and a push shows the label again.

### 6. The schema version 7

- `SCHEMA_VERSION = 7`. The constants `GESTURE_TIMERS_VERSION = 7` and
  `STATE_FACES_VERSION = 7` name the two changes.
- Version 7 adds `long-press` and `double-press` under `bindings.<control>`,
  and the table `surfaces.<surface>.on`.
- Versions 1 to 6 load with no loss: each keeps its defaults, and a page with
  no `id` gets N+1.
- A file of version 6 or lower with one of the new keys has an error, and the
  engine keeps the file as a backup (decision 0009). This is the pattern of
  decisions [0036](0036-folders.md) and
  [0040](0040-control-socket.md).
- The engine always writes version 7. A version above 7 has an error. Release
  0.5.0 writes version 6 and cannot read version 7, and the engine has no way
  to write version 6 again.
- The export of [decision 0031](0031-profile-export-file.md) uses the same
  rules, thus an older export imports.
- The tests are: one load test for each of versions 1 to 6 in the version 7
  engine (an exit criterion), one file of version 6 with each new key that has
  an error, and a round trip of version 7 with both new parts.
- `Counters::invokes_sent` counts the requests that a user gesture sends. A
  test that asserts `invokes_sent == input_events` uses keys with no `L` and
  no `D`, because a Press on such a key gives one request at once.

## Results

**What becomes easier.**

- One key does four things: press, long press, double press, and hold. The
  device sends only a press and a release for each of them. A new key action
  needs no change to the device layer.
- The shell gets the new facts as data: `inert_by`, `step`, `steps`,
  `knob_group`, `receives_release`, `handles_long_press`, and `TurnList`. It
  still has no action logic (rule 12).
- A hold cannot stay down. One module sends `HoldStart`, one map holds it, and
  each exit path sends `HoldEnd`. A new kind that receives the release uses
  the same interface.
- The two faces of a key are content of the profile. The export, the undo
  history, and copy and paste thus cover them with no new code.
- Schema version 7 adds keys only. Each old profile loads with no change.

**What becomes harder.**

- The engine now has time in the input path. A key with `L` or `D` waits, and
  a wrong constant is a delay that the user feels. P10-M1 measures it.
- A key with `D` runs its Press action at the end of the window, 300 ms after
  the Release.
- A device test counts one request for one press only on a key with no `L`
  and no `D`.
- The state machine has cases that the device causes: a lost Release, a
  duplicate Press, and a short pair of adjacent keys. Each case has a rule and
  a test.
- A hold adds one request pair on each press of a hold key, and one more place
  that must release the keys.

**When to look at this decision again.**

- P10-M1 measures the latency of a single press with and without a `D`
  binding. A result can change `DOUBLE_PRESS_WINDOW`, or make the wait start at
  the Press.
- A user setting for the two constants can come after Phase 10, if the
  measurements show that one value does not fit each use.
- The plug-in protocol needs the same requests. This record is then the base,
  as decision 0012 is the base for the handler interface.
- A long press that must stop at the Release, for example "hold to repeat", is
  a different gesture. It needs a new record.
