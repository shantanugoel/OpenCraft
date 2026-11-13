# Removal of an active profile

Last update: 2026-11-14

## Checks of issue #321

| Fact | Confidence | Source |
|---|---|---|
| With the source at `609ea21`, the debug and release windows removed the active three-page checks profile from page 2 and page 3. The next profile, Default, had one page. The window showed Default and page 1, with no problem. The release check also used the N4 Pro. | Measured | Scratch profiles; `docs/checks/phase-7-8-checks.opencraft`; native window actions and screenshots |
| In one release check with no device, temporary timing code measured 3.4 microseconds for the window to send RemoveProfile. The engine took 8.5 milliseconds for the command. The save took less than 0.1 millisecond; the directory move completed within 0.4 millisecond of the start. | Measured | Temporary timing code in `main.rs` and `Engine::remove_profile`; the code was removed after the check |
| The window sends the command through an unbounded queue. The engine does the save, directory move, profile load and draw on `opencraft-engine`, not on the window thread. Removal and normal selection both call `change_profile`. It sends Profiles before Pages. | Source | `crates/app/src/main.rs`; `crates/engine/src/runtime.rs`; `crates/engine/src/engine.rs` |
| Each handler has a separate request thread. Removal does not wait for Disappear or Appear. The microphone and sound-output handlers can join their audio worker on their handler thread. The permission worker waits up to five seconds between checks, but its cancellation channel wakes it at once. | Source | `runtime::start_handlers`; `actions/src/audio.rs`, `audio_output.rs` and `handlers.rs` |
| The engine test compares removal with normal selection. The window tests check removal from the third page to a one-page profile, and a mouse click on Close in a problem row. These tests passed before a production code change. | Measured | `removing_an_active_three_page_profile_sends_the_same_new_state_as_a_selection`; `removal_of_the_active_larger_profile_replaces_its_pages`; `a_problem_closes_by_mouse_and_stays_closed_in_the_shell` |

## Second analysis of issue #321 (2026-11-14)

| Fact | Confidence | Source |
|---|---|---|
| The problem "The profile has no page 1." comes from `Engine::is_page`. It writes the page number from 0. Thus the window sent a command for the second page to a profile with one page. | Source | `crates/engine/src/engine.rs` |
| The owner removed the profile at the end of the check procedure, directly after Show desktop, Mission Control, the screen saver, a screen lock and unlock, and a refused system sleep. | Source | `docs/checks/phase-7-8-checks.md`; the report of the owner in the issue |
| A window that GPUI takes as covered draws no frame after a change of its state. Its clicks then act on the rows of its last frame. While the screen of this computer was locked, the windows stayed in that condition. At the end of the lock, they drew again with no help. See [gpui.md](gpui.md#frames-after-sleep-lock-and-occlusion-2026-11-14). | Measured | Frame log and window pictures |
| A stale frame that stays after the unlock explains each part of the report: no change in the window after Remove, a message that seems not to close, a click on the old row "Page 2" that gives "The profile has no page 1.", and new pages in "Default" from clicks on rows that the window did not show. | Inference | The facts above |
| With the source of the correction, a click on a page row, on "Add a page", on a page item of the right-click menu, on the page arrows of the canvas, or on "Remove…" of a page sends the profile of the frame with the command (`Command::InProfile`). The engine does nothing, and gives a problem, if that profile is not the active profile. | Measured | Engine test `a_command_for_a_profile_that_is_not_active_changes_nothing`; window test `a_click_in_an_old_frame_carries_the_profile_of_that_frame` |
| A removal of the active profile does not write the file of the next profile, also after the delay of the save and after the stop. | Measured | Engine test `removing_the_active_profile_does_not_change_the_file_of_the_next_profile` |

## Not known

- Whether the screen lock and unlock of the owner made the stale frame of
  [issue #321](https://github.com/shantanugoel/OpenCraft/issues/321). One
  lock on this computer ended with a correct frame, also with no guard. The
  owner step in issue #255 asks for this.
- The cause of the delay of about 5 seconds in the report.
