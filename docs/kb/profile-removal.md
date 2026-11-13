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

## Not known

- The cause of the five-second delay, stale pages and failed Close button in
  [issue #321](https://github.com/shantanugoel/OpenCraft/issues/321). The checks
  above did not reproduce that fault. They are not proof that the fault is
  corrected.
