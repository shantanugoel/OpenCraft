# 0033: The engine draws a Lucide icon on a surface from a name in the profile

- Status: Accepted
- Date: 2026-11-13

## Context

Issue [#190](https://github.com/shantanugoel/OpenCraft/issues/190) lets the
user give a key face or a bar panel an icon from a built-in set of line
icons, with no image file. [Decision 0029](0029-window-layout.md) gives the
window parts: the row "Icon" of "Appearance" (section 10.2), the icon picker
(section 14), and the kind icons on the kind tiles (section 20). Section 3.6
leaves the icon set of the key faces to this issue.

The issue asks for the set, its licence, its cost in the bundle, and how the
engine draws an icon. These rules apply:

- Rule 11 ([ARCHITECTURE.md](../ARCHITECTURE.md#engine)): the engine makes
  the image of a surface one time, and the device and the window show that
  same image.
- The engine does not know the GUI toolkit (the layer table of
  ARCHITECTURE.md).
- Quality goal 2: the application is small.

The facts:

- The GUI toolkit contains all 1,818 icons of Lucide 1.43.0 in the crate
  `gpui-kit-assets`, with the ISC licence (and MIT for the icons from
  Feather). That crate depends on GPUI
  ([kb/ui-research.md](../kb/ui-research.md#the-icon-set-in-the-gui-toolkit)).
- The SVG renderer `resvg` 0.46 is already in the executable, because GPUI
  uses it ([kb/ui-research.md](../kb/ui-research.md#the-lucide-icons-for-surfaces-2026-11-13)).
- The measured costs are in [kb/footprint.md](../kb/footprint.md#icons-on-surfaces-2026-11-13).

## Decision

### The icon set is Lucide 1.43.0

- The set is [Lucide](https://lucide.dev) 1.43.0: 1,818 line icons on a grid
  of 24 × 24, with a stroke of width 2, and 42 categories. It is the version
  of the GUI toolkit, so that the window and the key faces have the same
  drawings.
- The licence is ISC. The icons from the Feather project also have the MIT
  licence. Both texts are in `crates/engine/icons/LICENSE`.
- `scripts/lucide-icons.sh` makes the data from the source archive of the
  release, and stops if the SHA-256 of the archive is not the fixed value. It
  is the archive that `gpui-kit-assets` 0.9.1 names. The data is
  `crates/engine/icons/lucide.tsv`: one line for each icon with its name, its
  first category, its tags, and its SVG elements. All icons have the same
  element `<svg>`, thus the file does not repeat it.
- The engine module `crates/engine/src/icons.rs` embeds the file and reads it
  one time, at the first use. It gives the icons in the order of their
  names, the categories, a search by name and by tag, and the SVG text of
  each icon.

### The engine draws the icon (rule 11)

- The engine draws the SVG of the icon with `resvg`, with no default
  feature, into the image of the surface. It is the same image that goes to
  the device and to the window. The window does not draw an icon on the
  canvas or in the header picture.
- An icon is a picture of the surface, as an image is. The label rules of an
  image apply to it ([`render`](../../crates/engine/src/render.rs)): a label
  at the top or at the bottom has a band, and a label at the middle is on the
  icon. The automatic position of the label is the bottom.
- The icon is a square at the centre of its area. Its side is the largest
  multiple of 24 pixels that is not more than three quarters of the shorter
  side of the area. Thus each line of the grid of the icon is on the edge of
  a pixel, and the lines stay sharp after the JPEG encode of the device. On a
  key face of 112 pixels, the side is 72 with no band and 48 with a band.
  The side stays the same inside the frame of the state "on".
- The colour of the icon is the icon colour of the profile, or black or
  white for the fill, as for the label. On a surface with no fill, it is
  black or white for the page background
  ([decision 0032](0032-page-background.md)).
- The window uses the same SVG data for the pictures that are not surface
  images: the grid of the icon picker, the tile of the row "Icon", and the
  kind tiles. Its asset source gives the path `surface-icons/<name>.svg`
  from the engine. Thus the executable has one copy of the data. GPUI draws
  these pictures in one colour.

### The profile names the icon

- Schema version 4 gets two keys of a surface, `icon` (the name, for example
  `"globe"`) and `icon_color` (`#RRGGBB`, only with `icon`). No release
  wrote version 4 before, thus the version does not change. The details are
  in `crates/engine/src/store.rs`.
- A surface has an image or an icon, not both. `Command::SetIcon` removes
  the image, and `Command::SetImage` with a file removes the icon.
  `Command::SetIcon` with no icon removes both: it is "No icon" in the
  window, which section 10.2 asks to remove an image also.
- A name has only small letters, digits, and hyphens. A name that the set of
  this engine does not have stays in the file, and the surface shows no
  icon, as an action with no handler stays (rule 8). The command gives a
  problem for a name that the set does not have.
- The icon is a part of `SurfaceContent`. Thus the undo history, Copy,
  Paste, Clear, Copy to All Pages, and the profile export and import cover it
  with no more code. The history names are "Set Icon" and "Remove the Icon".

### The kind icons are data

`KindDescription` gets an optional icon name
([decision 0014](0014-action-settings-description.md)). The built-in kinds
use the names of decision 0029, section 20. The shell has no list of kinds
(rule 8).

### The icon picker

The picker obeys decision 0029, section 14. These parts were not in the
record:

- With no search text, the tab "Icons" shows the icons in groups: each icon
  one time, in its first category, and the groups in the order of their
  titles. 40 of the 42 categories are the first category of an icon. A search shows the icons that agree with all
  words of the text, with no groups: first the icons with the text as their
  name, then the icons whose name starts with the text, then the icons that
  have the words in their name, and then the icons that have them in a tag.
- The number at the end of the search field is the number of icons in the
  grid.
- A colour with no icon on the surface does not change the profile. The next
  icon gets the colour.
- "No icon" and "Image file…" close the popover. "Image file…" then opens the
  file panel of the image.
- The popover goes below the button "Choose icon…" if it fits there, else
  above it. In a low window, it goes to the side with more space and is less
  high. The arrow points to the centre of the button.
- The shell keeps the recent icons in `shell.toml`, as `recent_icons`.

### The licence notices

The icons are not a crate. `scripts/notices.sh` adds the section "Works that
are not crates" at the end of `THIRD-PARTY-NOTICES.txt`, with
`crates/engine/icons/LICENSE` ([decision 0026](0026-licence-notices.md)).
No crate comes into the executable: `Cargo.lock` gets only the dependency of
the engine on `resvg`.

## Rejected alternatives

| Alternative | Reason |
|---|---|
| The engine uses the icons of `gpui-kit-assets` | That crate depends on GPUI. The engine must not know the GUI toolkit. |
| The window draws the icon on top of the image of the engine | The device would not show the icon. Rule 11: one image, two destinations. |
| The engine draws the paths with Core Graphics and its own SVG path code | More code to write and to test, and Linux needs a second path. `resvg` is already in the executable. |
| An icon font | It needs a font file and a second text path. Lucide gives the font as a separate package. |
| A copy of each SVG file | The 1,818 files repeat the same element `<svg>`. The data file is 499,309 bytes, and the SVG files of the release are approximately 731 KiB. |
| Compressed data | It needs code to expand it, and the expanded data stays in memory. The data in the executable is in memory pages that macOS can take back. |
| The icon as an image file of the profile, made at the choice | The colour cannot change later with no new file, the profile becomes larger, and the profile does not know the name of the icon. |

## Results

- A user can give a surface one of 1,818 icons in 6 colours (and black or
  white for the fill) with no file. The icon goes with the profile, its
  copy, its undo, and its export.
- The release executable is 628,864 bytes larger (2.6 %). The data is
  499,309 bytes of it. The details are in
  [kb/footprint.md](../kb/footprint.md#icons-on-surfaces-2026-11-13).
- A different version of Lucide is one run of `scripts/lucide-icons.sh` with
  a new version and checksum. A later version can remove or rename an icon:
  then a surface with the old name shows no icon, and the name stays in the
  profile. Look at the names of the kind icons, and at the version of the
  GUI toolkit, before a change.
- More icon sets (the LATER tab "Packs") can come as more data files and a
  set name in the profile key. The key `icon` then needs a set name, for
  example `lucide:globe`; a name with no set stays Lucide.
- No rule of [ARCHITECTURE.md](../ARCHITECTURE.md) changes.
