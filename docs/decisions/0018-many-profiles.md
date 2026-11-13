# 0018: Each profile is a directory in the profiles directory, and names its device model

- Status: Accepted
- Date: 2026-11-12

## Context

Phase 4 adds many profiles and a control to select one.
[Decision 0009](0009-profile-file-format.md) made a profile a directory with
a TOML file and its images. It left one question for this phase: how a
profile names its device model. The definition in
[ARCHITECTURE.md](../ARCHITECTURE.md#3-terms) says that a profile is for one
device model.

Until now the engine got the path of one profile directory. On macOS that
was `~/Library/Application Support/OpenCraft/profiles/default`.

## Decision

**The profiles directory.** The engine gets the path of the profiles
directory. Each directory in it that has a profile file is one profile.
The content of a profile directory stays as in decision 0009. The profile of
the earlier phases is in this directory, with the directory name `default`,
and it needs no move.

**The identity of a profile.** The identity is the name of its directory.
The engine makes the directory name from the name that the user gives: small
letters, digits, and `-`, and a number at the end if the name exists. The
directory name does not change when the user renames the profile. The name
for the user is in the profile file.

**The device model.** The profile file gets the key `model`. Its value is
the model identifier of the descriptor, for example `n4-pro`. The
descriptors of models with the same layout have the same model identifier.
The schema version becomes 3. A file of version 2 has no model: the engine
gives it the model of the first descriptor, which is the only model that the
earlier phases supported.

The engine offers only the profiles of the model of the device. The profiles
of a different model stay in the directory.

**The active profile.** The file `state.toml` in the profiles directory
holds the directory name of the active profile for each model. The engine
writes it when the active profile changes. If the file is missing or names a
profile that does not exist, the engine uses the first profile of the model
in the order of the directory names. If the model has no profile, the engine
makes one.

**Commands from the shell.**

| Command | Result |
|---|---|
| Select a profile | The engine writes the active profile if it changed, and loads the selected profile |
| Add a profile | A new profile, empty or a copy of a given profile. It becomes the active profile. |
| Rename a profile | The name for the user changes |
| Remove a profile | The engine moves the directory into the directory `removed` in the profiles directory. One profile for the model stays at the minimum. |
| Add a starter profile | Added on 2026-11-13 for the first start ([decision 0029](0029-window-layout.md#18-the-first-start-196)). The shell gives a profile. It becomes the active profile. If the active profile is not in a file and has no change, for example the profile that the engine made because the model had no profile, the new profile takes its place, and the engine does not write the old profile. |

**Events to the shell.** The engine sends the list of the profiles of the
model, each with its identity and its name, and the identity of the active
profile, after each change of them.

**A change of the active profile.** The engine does the steps of a page
change of [decision 0016](0016-pages-and-engine-requests-from-a-handler.md),
with the first page of the new profile as the new page.

**A request from a handler.** The request "select a profile" names a
profile by its name for the user. The built-in kind `opencraft.profile`
sends it. If no profile of the model has the name, the engine tells the
shell about the problem.

## Results

- A profile stays complete in one directory, and a copy of the directory is
  a backup.
- The engine holds only the active profile in memory. A change of the
  profile reads one file and the images of its first page.
- The engine does not delete a profile of the user. The user can get a
  removed profile back from the directory `removed`, by hand. The engine
  does not clean that directory.
- A profile is for one model. When a second model is supported, look at
  this decision again: a user with two models can want one profile for the
  two.
- The engine does not see a change that a different program makes in the
  profiles directory while the engine runs.
- Two profiles can have the same name for the user. Then the request from a
  handler selects the first one in the order of the directory names.
