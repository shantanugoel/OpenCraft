# Checks of Phase 10 on the device

This procedure is for the owner. It covers the physical checks of
[Phase 10](https://github.com/shantanugoel/OpenCraft/issues/202) in one
session, with the Phase 9 look-ahead recording and the Phase 11 look-ahead
check. Answer each question with yes or no, and write the answers in
[#202](https://github.com/shantanugoel/OpenCraft/issues/202) and
[#437](https://github.com/shantanugoel/OpenCraft/issues/437).

The checks use the connected N4 Pro, the OpenCraft window, a plain text
editor, and one call application that supports push-to-talk with the space
bar (for example Zoom, Google Meet, Slack, or Discord). The import file
`phase-10-checks.opencraft` in this directory has no personal data.

## Make ready

A Phase 10 build writes profiles in schema version 7. Release 0.5.0 reads
schema version 6 only. Use a scratch profiles directory until you install a
release with Phase 10.

1. Quit the installed OpenCraft from its menu bar item. Only one process can
   own the device.
2. Open a terminal at the root of this repository.
3. Run the guided script, or do the steps below by hand:

   ```sh
   scripts/phase-10-check.sh
   ```

   The script records the press times and starts the Phase 10 build with a
   scratch profiles directory. It prompts each step in order and moves to the
   next step when you press `y` or `n`. It saves your answers to
   `/tmp/opencraft-phase-10-answers.txt`.

## Step 0: Record the press and release times (P10-M1)

Issue: [#437](https://github.com/shantanugoel/OpenCraft/issues/437) (carried
forward from [phase-9-checks.md](phase-9-checks.md#look-ahead-for-phase-10-the-times-of-a-key-press)).

Do this step with no OpenCraft process running:

1. Run:

   ```sh
   cargo run --release -p opencraft-cli -- raw --seconds 60 --until-enter --save /tmp/opencraft-press.txt
   ```

2. Press key 1 short, one time. Wait 2 seconds.
3. Press key 1 and hold it for approximately 1 second. Wait 2 seconds.
4. Press key 1 two times fast. Wait 2 seconds.
5. Press Enter to stop the command. Keep `/tmp/opencraft-press.txt` for the
   agent to record in [kb/footprint.md](../kb/footprint.md).

## Start the Phase 10 build and import the check profile

1. Start OpenCraft with a scratch profiles directory:

   ```sh
   rm -rf /tmp/opencraft-phase-10 && mkdir -p /tmp/opencraft-phase-10
   OPENCRAFT_PROFILES_DIR=/tmp/opencraft-phase-10 cargo run --release -p opencraft-app
   ```

2. Give the Accessibility permission to OpenCraft if macOS asks.
3. In the sidebar, click **+** at the right of **PROFILES**, click
   **Import a Profile…**, and select `docs/checks/phase-10-checks.opencraft`.
4. Confirm that the device shows six keys on page 1 (**Press only**,
   **Gestures**, **PTT Off**, **Focus**, **Cycle**, **00:05**) and
   **Volume** on bar panel 1.
5. Open a plain text editor with an empty document and put the text cursor in
   it.

## 1. Single press with no added delay (KEY-10)

Issue: [#431](https://github.com/shantanugoel/OpenCraft/issues/431).

1. Press key 1 (**Press only**) one time. Does `now ` appear in the text
   editor immediately when the key goes down, with no visible pause? yes/no

## 2. Short, long, and double press on one key (KEY-10)

Issue: [#431](https://github.com/shantanugoel/OpenCraft/issues/431).

1. Press key 2 (**Gestures**) short, one time. Wait half a second. Does
   `short ` appear in the text editor, and not `long ` or `double `? yes/no
2. Press and hold key 2 (**Gestures**) for approximately 1 second, then
   release. Does `long ` appear while the key is still down, and does
   `short ` not appear on release? yes/no
3. Press key 2 (**Gestures**) two times fast. Does `double ` appear, and does
   `short ` not appear? yes/no

## 3. Hold a hotkey for push-to-talk (P10-T1)

Issue: [#432](https://github.com/shantanugoel/OpenCraft/issues/432).

1. Click key 3 (**PTT Off**) in the OpenCraft window. Does the **Long press**
   card show the notice "Does not run: the Press action holds the key"?
   yes/no
2. Open a call application with mute on (where holding the space bar opens
   the microphone). Press and hold key 3 (**PTT Off**) on the device.
   Does the key change to red **PTT Live** while down, and does the call
   application keep the microphone open while you hold the key? yes/no
3. Release key 3. Does the microphone mute again, and does the key return to
   **PTT Off** while the space bar does not stay pressed? yes/no

## 4. Two-state key faces (KEY-09)

Issue: [#433](https://github.com/shantanugoel/OpenCraft/issues/433).

1. Click in the text editor. Press key 4 (**Focus**) one time. Does key 4
   change to green **Break** with no border frame on both the device and the
   window? yes/no
2. Press key 4 (**Break**) a second time. Does key 4 return to blue **Focus**
   on both the device and the window? yes/no

## 5. Multi-step action cycle (AUTO-10)

Issue: [#434](https://github.com/shantanugoel/OpenCraft/issues/434).

1. Click key 5 (**Cycle**) in the OpenCraft window. Confirm step 1 shows
   **Next**.
2. Click in the text editor and press key 5 three times. Does the editor show
   `one one two `, and does **Next** move from step 1 to step 2, then step 3,
   then back to step 1? yes/no

## 6. Timer countdown, bell, and long-press reset (PLUG-26)

Issue: [#435](https://github.com/shantanugoel/OpenCraft/issues/435).

1. Press key 6 (**00:05**) short, one time. Does key 6 show a border frame
   and count down second by second to `00:00`, and does the system bell sound
   at `00:00`? yes/no
2. Press key 6 short to start again, wait 2 seconds, then press and hold
   key 6 for 1 second. Does key 6 reset to `00:05` with no border frame and
   no bell? yes/no

## 7. Multi-function knob (KNOB-03)

Issue: [#436](https://github.com/shantanugoel/OpenCraft/issues/436).

1. Turn knob 1 to the right and to the left. Does the Mac sound volume change
   and show on bar panel 1? yes/no
2. Push knob 1. Does bar panel 1 change to **Brightness**? yes/no
3. Turn knob 1 to the right and to the left. Does the brightness of the N4 Pro
   change? yes/no
4. Push knob 1 again. Does bar panel 1 return to **Volume**? yes/no

## Look-ahead for Phase 11: a new line in a label (fact 14)

Issue: [#203](https://github.com/shantanugoel/OpenCraft/issues/203).

1. In the OpenCraft window, select any empty key (for example key 7).
2. Try to type a two-line title in **Title** (for example press Option-Return
   or paste two lines of text). Does the title field keep a new line, or does
   it turn it into one line? Write what you saw on the device and in the
   window in [#203](https://github.com/shantanugoel/OpenCraft/issues/203).

## Finish

1. Quit the Phase 10 build of OpenCraft.
2. Start your installed OpenCraft again.
3. Tell the agent that `/tmp/opencraft-press.txt` and your answers are ready.
