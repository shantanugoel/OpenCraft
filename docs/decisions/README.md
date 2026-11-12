# Decision records

A decision record gives one design decision and its reason. Write a record when
a phase forces a decision. Do not write a record for a decision that can wait.

## Rules

- One decision in one file. The file name is `NNNN-short-title.md`.
- Keep the record short. Give the context, the decision, and the results.
- Do not change the decision in an accepted record. Write a new record that
  replaces it, and set the old status to "Replaced by NNNN".
- Put the facts in [../kb/](../kb/README.md) and link to them. A record contains
  the reason, not the research.

## Index

| Number | Decision | Status |
|---|---|---|
| [0001](0001-native-application.md) | Make a native application, not an OpenDeck plug-in | Accepted |
| [0002](0002-hidapi-transport.md) | Use `hidapi` directly for the transport | Accepted |
| [0003](0003-blocking-device-layer.md) | The device layer blocks and has no async runtime | Accepted |
| [0004](0004-gui-from-first-feature-phase.md) | The GUI is in each feature phase | Accepted |
| [0005](0005-gpui.md) | Use GPUI for the GUI | Accepted. The open item is closed by 0020. |
| [0006](0006-action-kinds-and-handler-interface.md) | Action kinds have a namespace, and all actions use one handler interface | Accepted |
| [0007](0007-jpeg-for-key-faces-and-bar-panels.md) | Send JPEG to key faces and bar panels | Accepted |
| [0008](0008-crate-boundaries.md) | One crate for the engine and one crate for the GUI application | Accepted |
| [0009](0009-profile-file-format.md) | The profile is a directory with a TOML file and its images | Accepted |
| [0010](0010-core-text-for-surface-labels.md) | Use Core Text for surface labels on macOS | Accepted |
| [0011](0011-engine-thread-model.md) | The engine has one thread, and the shell uses messages | Accepted |
| [0012](0012-action-handler-interface.md) | An action handler gets requests and sends messages, on its own thread | Accepted |
| [0013](0013-action-feedback-on-a-surface.md) | An action changes a surface with feedback data, and the engine makes the image | Accepted |
| [0014](0014-action-settings-description.md) | An action kind describes its settings as a list of fields | Accepted |
| [0015](0015-four-bar-panels-and-touch-gestures.md) | The touch bar shows four bar panels, and the device identifies the touch gestures | Accepted |
| [0016](0016-pages-and-engine-requests-from-a-handler.md) | The engine holds the active page, and a handler asks the engine with a message | Accepted |
| [0017](0017-toggle-and-sequence-actions.md) | An action can contain actions, and the engine draws the state of a toggle | Accepted |
| [0018](0018-many-profiles.md) | Each profile is a directory in the profiles directory, and names its device model | Accepted |
| [0019](0019-engine-in-the-gui-process.md) | The engine stays in the process of the GUI application | Accepted. 0028 changes what a second start does. |
| [0020](0020-menu-bar-item-and-dock-icon.md) | The shell makes the menu bar item with AppKit, and has one quit path | Accepted |
| [0021](0021-start-at-login.md) | A LaunchAgent file starts OpenCraft at login, with the argument `--login` | Accepted. 0025 changes the state of the option, and keeps the LaunchAgent file. |
| [0022](0022-device-settings-and-stop-state.md) | Device settings are for each device model, outside the profile, and the device goes dark at the stop | Accepted |
| [0023](0023-wake-from-the-sleep-time-of-the-clocks.md) | The link finds a wake from the sleep time of the clocks, and the engine then draws all surfaces again | Accepted |
| [0024](0024-log-file-with-no-terminal.md) | The messages go to a log file when the process has no terminal | Accepted |
| [0025](0025-release-form.md) | The release is a signed and notarized bundle in one disk image for Apple silicon | Accepted. 0026 makes the notices file necessary. |
| [0026](0026-licence-notices.md) | cargo-about makes a committed notices file, CI checks it, and the menu bar item opens it | Accepted |
| [0027](0027-homebrew-cask.md) | Homebrew installs OpenCraft with a cask in a tap of the owner | Accepted |
| [0028](0028-second-start-opens-the-window.md) | A second start opens the window of the OpenCraft that runs | Accepted |
| [0029](0029-window-layout.md) | The main window has a sidebar, a canvas, and a right panel, and each new feature has one place in it | Accepted. It is the text specification of the UI overhaul, thus it is longer than the other records. |
| [0030](0030-undo-and-redo-in-the-engine.md) | The engine keeps the history of undo and redo, as copies of the pages | Accepted |

## Template

```markdown
# NNNN: Title

- Status: Proposed | Accepted | Replaced by NNNN
- Date: YYYY-MM-DD

## Context

What made this decision necessary.

## Decision

What we do.

## Results

What becomes easier. What becomes harder. When to look at this decision again.
```
