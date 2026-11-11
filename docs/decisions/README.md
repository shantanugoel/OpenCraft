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
| [0005](0005-gpui.md) | Use GPUI for the GUI | Accepted, with a Phase 5 open item |
| [0006](0006-action-kinds-and-handler-interface.md) | Action kinds have a namespace, and all actions use one handler interface | Accepted |
| [0007](0007-jpeg-for-key-faces-and-bar-panels.md) | Send JPEG to key faces and bar panels | Accepted |
| [0008](0008-crate-boundaries.md) | One crate for the engine and one crate for the GUI application | Accepted |
| [0009](0009-profile-file-format.md) | The profile is a directory with a TOML file and its images | Accepted |
| [0010](0010-core-text-for-surface-labels.md) | Use Core Text for surface labels on macOS | Accepted |
| [0011](0011-engine-thread-model.md) | The engine has one thread, and the shell uses messages | Accepted |

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
