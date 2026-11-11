# 0009: The profile is a directory with a TOML file and its images

- Status: Accepted
- Date: 2026-11-11

## Context

Phase 2 saves the profile and loads it again. Rule 9 of
[ARCHITECTURE.md](../ARCHITECTURE.md#5-rules) applies: the profile file is
the source of truth, the engine writes it, and it has a schema version.
[Decision 0004](0004-gui-from-first-feature-phase.md) adds that no person
writes the file by hand, but the file must stay readable.

The content of a surface can include an image file of the user. The profile
must stay correct when the user moves or deletes that file.

## Decision

**Format.** The profile file is TOML. The engine reads and writes it with the
crates `serde` and `toml`.

**Location.** A profile is a directory. The file is `profile.toml`, and the
images are in the directory `images` next to it. On macOS, the profile of
Phase 2 is `~/Library/Application Support/OpenCraft/profiles/default`. A
caller of the engine can give a different directory, for example for a test.

**Content.** The file has `schema_version`, the name of the profile, and a
list of pages. A page has one table for each surface that has content. The
key of the table is the identity of the surface, for example `key-face-3` or
`bar-panel-1`. The table can have a label, a colour as `#RRGGBB`, and an
image as a path relative to the profile directory. The value of
`schema_version` in Phase 2 is 1.

```toml
schema_version = 1
name = "Default"

[[pages]]
name = "Page 1"

[pages.surfaces.key-face-3]
label = "Mail"
color = "#1E6FD9"
image = "images/4f1c2a9be07d3356.png"
```

**Images.** When the user selects an image file, the engine decodes it and
writes a copy in the directory `images`. The copy is a PNG. If the image is
larger than 512 pixels on a side, the engine makes it smaller first. The name
of the copy comes from a hash of its data. The profile does not refer to the
original file. At the start, the engine removes each file in `images` that no
surface uses.

**Writes.** The engine writes a temporary file in the same directory and then
renames it. Thus a stop during a write does not leave a file that is not
complete.

**A file that the engine cannot use.** If the file has an error, or its
schema version is newer than the engine knows, the engine does not change
it. The engine renames the file to a backup name, tells the shell, and starts
with an empty profile.

**An older schema version.** The engine converts the file when it loads it,
and writes the new version at the next change.

## Results

- A person can read the file and compare two versions of it.
- A profile is complete in one directory. A copy of the directory is a backup.
- An image that cannot be decoded fails when the user selects it, not at a
  later start.
- The engine decodes only small images at the start. This helps the cold
  start target.
- The engine does not keep the original image. An image for a larger surface
  on a different model has at most 512 pixels on a side.
- A profile does not yet name its device model. The content of a surface that
  the connected model does not have stays in the file, and the engine does
  not draw it. Phase 4 adds many profiles: decide then how a profile names
  its model.
- The schema can change freely between phases, because the engine converts
  older versions.
