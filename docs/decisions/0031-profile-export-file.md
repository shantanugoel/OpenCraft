# 0031: A profile goes to one TOML file with its images as Base64 text

- Status: Accepted
- Date: 2026-11-13

## Context

Issue [#195](https://github.com/shantanugoel/OpenCraft/issues/195) adds the
profile settings sheet of
[decision 0029](0029-window-layout.md#16-the-profile-settings-sheet-195). Its
"Share" section has "Export as a file…". The issue asks for one file for
the profile, with its images. The import command is in the sidebar and in
the command palette; its placement is in decision 0029, section 7.3
(amended for issue #322).

A profile is a directory with `profile.toml` and the directory `images`
([decision 0009](0009-profile-file-format.md),
[decision 0018](0018-many-profiles.md)). A copy of a directory is not one
file, and a user cannot easily send a directory.

An import reads a file from a source that OpenCraft does not trust. The file
must not write to a path out of the profile directory, must not use too much
memory, and must give a clear message when it is not correct. The file must
not contain personal data, for example a home path.

## Decision

**One TOML file.** The file has the extension `.opencraft`. It has these top
keys:

| Key | Value |
|---|---|
| `format` | `"OpenCraft profile"`. A file with a different value is not a profile file. |
| `version` | The version of this file format: 1 |
| `profile` | A table with the content of `profile.toml` (decision 0009), with the same schema version and the same keys |
| `images` | A table: for each image path of the profile, for example `images/4f1c2a9be07d3356.png`, the PNG data of the image as Base64 text |

The engine writes the file and reads it (rule 9). The code is in
`crates/engine/src/share.rs`. The commands are `ExportProfile` and
`ImportProfile`.

**The export.** For the active profile, the engine uses the profile in
memory, thus a change that is not in the file yet is in the export. The file
holds each image one time. An image that is missing in the profile directory
is not in the file, and its surface has no image in the file. The file holds
no path of the computer and not the identity of the profile. The engine does
not read the settings of an action (rule 8), thus a setting that the user
typed goes into the file as it is.

Note (2026-11-14): [decision 0037](0037-application-bundle-and-export-path.md)
replaces the sentence above for a path setting. The export writes `~` in
place of the home directory in each path setting. Each other setting stays
as it is.

**The import.** The engine:

1. Refuses a file that is larger than 64 MB, before it reads the file.
2. Refuses a file that is not TOML, or that has no `format` with the
   correct value. A newer `version` or a newer schema version of the profile
   gives the message that a newer OpenCraft made the file.
3. Reads the profile with the checks of a profile file (decision 0009), for
   example each image path must be a file name in `images`.
4. Refuses more than 1024 images, an image key that is not an image path of
   a profile, and an image larger than 4 MB after the Base64 decode. Each
   image path of the profile must have its data in the file.
5. Refuses a profile for a different device model.
6. Decodes each image with limits (4096 pixels on a side, 128 MB of memory),
   and writes a new copy with the steps of an image of the user: a PNG of at
   most 512 pixels on a side, with a name from its data. The profile gets
   the paths of the new copies. No data goes to a path that the file gives.
   A page background gets the larger copy of
   [decision 0032](0032-page-background.md).
7. Adds the profile as a new profile: the identity comes from the name, and
   the name gets a number after it, for example "Work 2", if a profile of
   the model has the name. The new profile becomes the active profile, as
   after "Add a profile". If a step fails, the engine removes the new
   directory and sends a problem.

**The profile settings in the profile file.** The sheet also changes the
colour of a profile and its first page. They are new top keys of
`profile.toml` in schema version 4: `color` (`#RRGGBB`) and `first_page`
(from 1). No release wrote version 4 before, thus the version does not
change. The details are in `crates/engine/src/store.rs`. The mark of the
first page is on a page in memory, so that it moves with the page and the
undo history covers it ([decision 0030](0030-undo-and-redo-in-the-engine.md)).

## Rejected alternatives

| Alternative | Reason |
|---|---|
| A ZIP archive of the profile directory | It needs a new dependency, and each path in the archive must be checked against a path out of the directory. The TOML file has no paths to check, and the engine already reads TOML. |
| A copy of the profile directory | It is not one file. A user cannot easily send it. |
| Keep the image data as it is in the file, with its name | The engine must then trust the data and the name. A new decode and a new copy make sure that each image is a real image of the correct size. |
| Replace the pages of the active profile with the import | The user loses the active profile. An import as a new profile loses nothing, and the user can remove a profile. |

## Results

- A profile with its images is one file that a person can read, and a text
  compare shows the changes of the profile.
- Base64 makes the image data approximately one third larger. An image copy
  is at most 512 pixels on a side, thus a typical profile file is small.
- A file that a newer OpenCraft made gives a clear message. When the format
  changes, increase `version`, and keep the read of version 1.
- An import of an image can give a copy with a different name from the
  original, if the original was not made by the same encoder. The content is
  the same.
- No rule of [ARCHITECTURE.md](../ARCHITECTURE.md) changes.
