# Checks of Phase 11 on the device

This procedure is for the owner. It covers the physical checks of
[Phase 11](https://github.com/shantanugoel/OpenCraft/issues/203) in one
session. Answer each question with yes or no, and write the answers in
[#459](https://github.com/shantanugoel/OpenCraft/issues/459) and
[#203](https://github.com/shantanugoel/OpenCraft/issues/203).

An agent does the other checks of Phase 11: the footprint, the camera, the
accessibility tree, and the keyboard order. This procedure holds only what a
person must do.

The checks use the connected N4 Pro and the OpenCraft window. The import file
`phase-11-checks.opencraft` in this directory has no personal data.

## Make ready

1. Use the checkout of Phase 11, or release 0.7.0. The profile schema is
   version 8, and release 0.6.0 cannot read it.
2. Quit the installed OpenCraft from its menu bar item. Only one process can
   own the device.
3. Open a terminal at the root of this repository.
4. Run the guided script, or do the steps below by hand:

   ```sh
   scripts/phase-11-check.sh
   ```

   The script copies your profiles to a temporary directory and adds the
   profile of this check. It never opens your own profiles. The new build
   writes schema version 8, and release 0.6.0 cannot read that version.

   The script asks each step in order. It moves to the next step when you
   press `y` or `n`. It saves your answers to
   `/tmp/opencraft-phase-11-answers.txt`. At the end, the script removes the
   copy. It starts your installed OpenCraft again, also if you quit it in
   step 2. If you press Ctrl-C, the script does the same and stops.

The check profile has five keys on page 1:

| Key | Face |
|---|---|
| 1 (top row, first) | Empty. Drop the image here in step 1. |
| 2 (top row, second) | Empty. Drop the application here in step 1. |
| 3 | `Menlo`, an installed font, italic and underlined, 32 pt |
| 4 | The emoji 😀 at 80 pt |
| 5 | The clock, 24-hour |

## 1. Drop a file from Finder (KEY-01, KEY-13)

Issues: [#448](https://github.com/shantanugoel/OpenCraft/issues/448) and
[#449](https://github.com/shantanugoel/OpenCraft/issues/449).

A drag from Finder is a system drag-and-drop session. An agent cannot do it
in the background.

1. Drag one image file from Finder onto key 1. Do the window and the device
   show the picture? yes/no
2. Drag one `.app` from Finder onto key 2. Do the window and the device show
   the icon of the application? yes/no

## 2. The four keys on the device and in the window (ICON-06, P11-T1, ICON-04, ACT-31)

Issues: [#452](https://github.com/shantanugoel/OpenCraft/issues/452),
[#453](https://github.com/shantanugoel/OpenCraft/issues/453),
[#455](https://github.com/shantanugoel/OpenCraft/issues/455), and
[#456](https://github.com/shantanugoel/OpenCraft/issues/456).

Look at key 1 (the image), key 3 (`Menlo`, italic and underlined), key 4
(the emoji), and key 5 (the clock). Look at them on the device and in the
window.

1. Are the colours of the four keys the same on the device and in the window?
   yes/no
2. Is the small text sharp on the device (the clock and the label `Menlo`)?
   yes/no

The camera pictures of these keys are in
[kb/footprint.md](../kb/footprint.md#phase-11-the-clock-key-and-three-installed-fonts-2026-11-22).
The colours of a black fill with white text and the picture agreed in the
agent check. The questions above are for the look with your eyes.

## 3. Move a key by a drag in the window (KEY-14)

Issue: [#451](https://github.com/shantanugoel/OpenCraft/issues/451).

A drag in a GPUI window is a GPUI drag, not a system drag. An agent cannot
start it in the background.

1. Drag key 3 onto key 6 in the window. Do the window and the device show the
   `Menlo` key at place 6? yes/no
2. Press Command-Z in the window. Does the `Menlo` key return to place 3?
   yes/no

## 4. Optional: the appearance follows macOS (APP-21)

Issue: [#457](https://github.com/shantanugoel/OpenCraft/issues/457). This
check does not stop the release.

1. Switch macOS to the Light appearance and back, with the window open
   (System Settings > Appearance). Does the window change with it? yes/no

## 5. Optional: VoiceOver (P11-M1)

Issue: [#458](https://github.com/shantanugoel/OpenCraft/issues/458). This
check does not stop the release. An agent must not turn on VoiceOver: it
speaks and moves the cursor on the desktop of the owner.

1. Turn VoiceOver on, press Tab and Space a few times in the window, then
   turn VoiceOver off. Does it speak sensible labels for the controls?
   yes/no

## 6. Optional: look-ahead for Phase 12, VSD Craft

Issue: [Phase 12](https://github.com/shantanugoel/OpenCraft/issues/203). This
check does not stop the release. VSD Craft opens the device, thus OpenCraft
must stop first. The script stops it for this step.

1. Open VSD Craft, export two scenes to files, then quit VSD Craft. Did it
   write two files? yes/no

The next phase can ask you if an agent may read the `profiles` folder of VSD
Craft. That folder is in its application support directory. When you agree,
this step is not necessary.

## Fact 14 of Phase 10: a new line in a title (answered, no step)

The look-ahead in [phase-10-checks.md](phase-10-checks.md#look-ahead-for-phase-11-a-new-line-in-a-label-fact-14)
asked if a title can have two lines. An agent answered it in the real window:
the Title field is one line, and Option-Return inserts no character. A label
with two lines needs a value that already has a new line. The clock formats
"Weekday and date" and "Time and date" have a new line. The device shows the
two lines. See
[text-rendering.md](../kb/text-rendering.md#a-new-line-in-a-label-fact-14-2026-11-22).
You do not do this check.

## Finish

1. Quit the Phase 11 build of OpenCraft, if it still runs.
2. Start your installed OpenCraft again. The script does this for you.
3. Tell the agent that `/tmp/opencraft-phase-11-answers.txt` is ready.
