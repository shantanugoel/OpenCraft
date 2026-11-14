# 0037: An application action stores the bundle identifier, and an export hides a home path

- Status: Accepted
- Date: 2026-11-14

## Context

Issue [#350](https://github.com/shantanugoel/OpenCraft/issues/350). A pick
from the list of installed applications stored the absolute path of the
bundle. A path in the home folder is personal data.
[Decision 0031](0031-profile-export-file.md) says that an export file must
not contain a home path. It also says that the engine does not read the
settings of an action, so a setting went into the file as it was.

The owner decided the design on 2026-11-14: store the bundle identifier,
keep the path to tell two copies apart, store the path as it is in the
profile, and replace the home directory with `~` in each path setting of
an export.

A link of [decision 0035](0035-front-application-selects-the-profile.md)
already stores the bundle identifier. The facts about the registered copies
are in [macos-open-and-quit.md](../kb/macos-open-and-quit.md).

## Decision

**Open an application.** The setting `bundle_id` stores the bundle
identifier. The setting `application` stores the path, or a name, as the
user gave it. The path is the fallback.

The description of the path field names the `bundle_id` setting. The shell
writes it when the user picks an application that has a bundle identifier,
from the list or from the file dialog. A file, a folder, or a typed name
has no bundle identifier. A later edit of the path removes `bundle_id`.

When the action runs:

- One registered copy of the bundle identifier: open that copy. A move of
  the application does not matter.
- More than one registered copy: open the copy whose path is the stored
  path.
- No bundle identifier, or no registered copy: open the stored name or path.

The profile file keeps the path as it is. A path in the home folder stays
a home path in the profile file.

**The export.** The engine replaces the home directory with `~` in each
path setting, in the export copy only. A path setting is a field of input
type Path in the kind description, including a path inside a contained
action and a path in a folder. The engine does not change any other
setting. A command stays as the user typed it. The profile file does not
change. An imported profile keeps `~`. The action expands a path that
starts with `~/`.

This is the one place where the engine reads action settings. It reads
them only to find path text. It does not decide what the action does.
Rule 8 stays for every other use.

## Results

- An export has no home path in a path setting.
- An application action still opens the application after a move, when one
  copy of the bundle identifier is registered.
- Two copies of one bundle identifier stay distinct.
- A setting that is not a path can still contain a home path, because the
  export does not change it.
