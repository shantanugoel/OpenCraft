# 0043: Installed fonts, label styles, image crop, the clock handler, and profile schema 8

- Status: Accepted
- Date: 2026-11-22

## Context

Phase 11 ([#203](https://github.com/shantanugoel/OpenCraft/issues/203)) makes
a key look correct. The phase adds an image from Finder and from the clipboard
(KEY-01), an installed font for the label (ICON-06), italic and underline
(P11-T1), a default title style for the profile (KEY-04), large sizes for an
emoji (ICON-04), crop and zoom for an image (KEY-02), and a clock key (ACT-31).
This record is the decision P11-D1. It also makes profile schema version 8.

The present state:

- A label has a position, a size, a colour, and a weight
  (`crates/engine/src/profile.rs:117-160`). The size is 10 to 40 pixels. A
  text that does not fit goes down in steps of 2 pixels
  (`crates/engine/src/label.rs:42-52`). The font is the macOS system font, in
  the regular or the emphasized weight (`crates/engine/src/label.rs:159-168`).
  Core Text makes the layout and draws the text
  ([decision 0010](0010-core-text-for-surface-labels.md)).
- The profile schema is 7
  ([decision 0042](0042-gesture-timers.md)). Release 0.6.0 writes version 7.
  Thus Phase 11 cannot add keys to version 7.
- An image is drawn as large as possible with its proportion, at the centre
  (`crates/engine/src/render.rs:505`). The profile stores no crop.
- gpui reads the system pasteboard in this order: file paths, then a string,
  then an image (`gpui-pre-macos-0.3.12/src/pasteboard.rs:57-109`). The engine
  has its own control clipboard (`crates/engine/src/engine.rs:339-346`).
- The handler interface has no request that wakes a handler
  (`crates/engine/src/action.rs:231-267`). A handler that needs time uses its
  own thread; the timer handler does this
  (`crates/actions/src/timer.rs:268-296`).
- Rules 8, 9, 11, and 12 of
  [ARCHITECTURE.md](../ARCHITECTURE.md#5-rules) apply. Rule 8: the engine
  does not read the settings. Rule 9: the engine writes the profile. Rule 11:
  the engine makes one image for the device and the window. Rule 12: the shell
  has no action logic.

The facts that the decision needs, with a source when code or a document gives
them:

| Fact | Confidence | Source |
|---|---|---|
| The installed family list comes from `CTFontManagerCopyAvailableFontFamilyNames`; the binding is `font_manager::copy_available_font_family_names`. | Source | `core-text-22.0.0/src/font_manager.rs:17` |
| `new_from_name` calls `CTFontCreateWithName`. That function takes a PostScript name or a full name and gives a best match, not an error. | Source | `core-text-22.0.0/src/font.rs:205`; Apple documentation |
| A family descriptor comes from `font_descriptor::new_from_attributes` with `kCTFontFamilyNameAttribute`. A font comes from `font::new_from_descriptor`. | Source | `core-text-22.0.0/src/font_descriptor.rs:341,396`; `font.rs:173` |
| `clone_with_symbolic_traits` returns `Option<CTFont>`. It gives `None` when the family has no such face. | Source | `core-text-22.0.0/src/font.rs:295-310` |
| `clone_with_font_size` uses a null matrix. It changes only the size. | Source | `core-text-22.0.0/src/font.rs:287-293` |
| `CGContext::set_text_matrix` takes a `CGAffineTransform`. `CGAffineTransform::new` takes the six values. | Source | `core-graphics-0.25.0/src/context.rs:465`; `core-graphics-types-0.2.0/src/geometry.rs:145-156` |
| `kCTUnderlineStyleAttributeName` is an attribute name. `CFNumber::from(1i32)` makes the value. | Source | `core-text-22.0.0/src/string_attributes.rs:15`; `core-foundation-0.10.1/src/number.rs:96` |
| The current label sizes go down in steps of 2 pixels. | Source | `crates/engine/src/label.rs:42-52` |
| The current image is a fit with its proportion, at the centre. | Source | `crates/engine/src/render.rs:505` |
| The profile writes a label size or a bold value only when it is not the engine default. | Source | `crates/engine/src/store.rs:946-951` |
| The gpui pasteboard reads file paths, then a string, then an image. | Source | `gpui-pre-macos-0.3.12/src/pasteboard.rs:57-109` |
| `image` 0.25.10 already has `tiff` and `image-webp` in the lock, through `gpui-pre-macos`. | Source | `Cargo.lock`; `cargo tree -i tiff` |
| The handler interface has no wake request. | Source | `crates/engine/src/action.rs:231-267` |
| Rust `Instant` on macOS stops during system sleep, thus `recv_timeout` does not count that time. | Inference | std source; ACT-31 measures it |
| The installed family list can give a localized name for the interface. | Inference | Apple documentation says "for user interface display"; ICON-06 measures it |

## Decision

### 1. Installed fonts (`crates/engine/src/label.rs`)

**The list.** The engine gives the shell the installed font families. It uses
`copy_available_font_family_names()` (`CTFontManagerCopyAvailableFontFamilyNames`).
The engine:

- makes the list at the start, and again when the font picker opens;
- removes a name that starts with `.` (a hidden system family);
- removes a duplicate;
- sorts by the family name, with no difference between capital and small
  letters;
- gives each family a display name and a stored name.

**The stored name.** The profile stores the non-localized family name. The
list gives names for the interface, and such a name can be localized. The
stored name comes from `CTFont::family_name()`, which calls `CTFontCopyName`
(`core-text-22.0.0/src/font.rs:317`). `CTFontCopyName` is not
`CTFontCopyLocalizedName`. Thus a profile loads on a computer with a different
language. The picker shows the display name. It sends the stored name. If the
two names are equal, which is the usual case, the picker and the profile show
the same text. ICON-06 measures a language that localizes a family name and
writes the result in [kb/text-rendering.md](../kb/text-rendering.md). The
control socket, the CLI, and the MCP use the stored name, as the profile does.

**The resolution.** To make a font for a family:

1. Test the stored name for membership in the cached set of stored names. A
   missing name uses the system font. Do not ask Core Text to find the family:
   `new_from_name` calls `CTFontCreateWithName`, which takes a PostScript name
   or a full name and gives a best match, not an error. A family name can then
   give a different face, for example Helvetica.
2. Make a descriptor with `kCTFontFamilyNameAttribute` equal to the family
   name, and a font with `font::new_from_descriptor` at the requested size.
3. Make the font one time for each `fit` call. Use `clone_with_font_size` for
   each smaller size. Do not make a new descriptor for each step.
4. If the family is missing, use `new_ui_font_for_language` with the system
   type. Keep the stored name in the profile. The inspector shows a note, not
   a problem: "Not installed: the system font shows". A font that the user
   installs while the application runs shows after the next refresh, when the
   picker opens.

**Bold and italic.** The default weight is bold. A family can have no bold
face. `clone_with_symbolic_traits` gives `None` then. The rules:

- **Bold.** Ask for `kCTFontBoldTrait`. If the family has no bold face, use
  the regular weight. Do not make a synthetic bold. A synthetic bold smears
  small text, and many script and display families have no bold face. A family
  with no bold face shows its regular face.
- **Italic.** Ask for `kCTFontItalicTrait` on the font from the bold step. If
  the family has no italic face, draw with a shear of 0.2. Use
  `CGContext::set_text_matrix` with
  `CGAffineTransform::new(1.0, 0.0, 0.2, 1.0, 0.0, 0.0)`. The layout width
  becomes smaller by `0.2 * ascent`, and each drawn line moves left by half of
  that value. Thus the slanted text stays in the area and stays centred.
- Apply bold first, then italic. Do not ask for the two traits in one call: a
  family with a bold face and no italic face then gives no bold.
- Ask for the italic trait first on the system font. The current macOS system
  font has an italic face (Inference). If the trait fails, the shear applies.
- **Underline.** Set `kCTUnderlineStyleAttributeName` to
  `CFNumber::from(1i32)` (`kCTUnderlineStyleSingle`) on the attributed string.
  The fit height includes the underline position and thickness, thus the line
  stays in the area at `Align::Bottom`. The underline uses the text colour.
- Italic and underline apply to each line. A label with `\n` has two lines, and
  the attributes cover the whole string.

**No font file.** OpenCraft bundles no font file. The binary does not grow
with a font, and the licence notices do not change. Decision 0010 stays true.

**Not macOS.** The label module draws no text
(`crates/engine/src/label.rs`, platform stub). The font list is empty. The
engine builds on Linux. The tests that need a font are
`#[cfg(target_os = "macos")]`.

**The public function.** `label::font_families()` gives the list as data. It is
not a new `Event`, and it is not action logic (rule 12). The shell calls it
when the font picker opens. The list is behind a lock, because the engine
thread draws and the GUI thread reads the list. A font list is not device or
action logic.

### 2. The label style and the inheritance

The window calls the label of a surface the title. The code calls it the
label. This record uses `LabelStyle` for the data and "default title style" for
the profile setting.

**The three forms.** A surface has these keys. Each of position, colour, and
font has three forms:

| File form | Meaning |
|---|---|
| The key is absent | Inherit the default title style of the profile |
| `label_position = "auto"`, `label_color = "auto"`, `label_font = "system"` | The automatic value for this surface, also when the profile default has a value |
| `label_position = "top"`, `label_color = "#RRGGBB"`, `label_font = "Avenir Next"` | The value |

- `label_position`: `"top"`, `"middle"`, `"bottom"`, `"none"`, or `"auto"`.
  `"none"` hides the label and keeps the text.
- `label_color`: `#RRGGBB` or `"auto"`. The automatic colour is black or white
  for the colour behind the label, the fill or the page background
  (`crates/engine/src/render.rs`, `text_color`).
- `label_font`: a family name or `"system"`. A family name is not empty. The
  word `"system"` is reserved. A family with that name cannot be selected.
- `label_size`: 10 to 80 (section 7). `label_bold`, `label_italic`, and
  `label_underline`: `true` or `false`. These four keys have no automatic
  value. An absent key inherits.
- In Rust, the three fields use `Option<Auto<T>>`, where `Auto<T>` is `Auto`
  or `Value(T)`. The other four fields use `Option<T>`. The same form is in
  `SurfaceContent::label_style` and in `Profile::default_label_style`.
- The file words are `"auto"` for position and colour, and `"system"` for
  font. Thus a surface goes back to the automatic value even when the profile
  default has a value. A design with `Option<T>` for every field cannot do
  this: `None` then means both "inherit" and "automatic".

**The profile default.** `Profile::default_label_style: LabelStyle`. The file
has the table `[default_label_style]`. The keys have no `label_` prefix:
`font`, `position`, `size`, `color`, `bold`, `italic`, and `underline`. An
absent key in the table is the engine default. The table is absent when the
profile default has no own value. The engine default is:

| Part | Engine default |
|---|---|
| Font | The system font |
| Position | The automatic position |
| Size | 20 pixels |
| Colour | The automatic colour |
| Bold | `true` |
| Italic | `false` |
| Underline | `false` |

The automatic position is the middle on a surface with no picture, and the
bottom on a surface with an image or an icon
(`LabelStyle::position_for`). The automatic colour is black or white for the
colour behind the label.

**The resolution.** `LabelStyle::resolve(&self, profile_default: &LabelStyle)
-> ResolvedLabelStyle`:

- For each field, an own value wins. Else the value of the profile default
  wins. Else the engine default applies.
- A field of the profile default that is absent gives the engine default.
  There is no third profile level.
- `ResolvedLabelStyle` has a concrete value for each field: a family or
  `None` for the system font, a position or `None` for the automatic position,
  a size, a colour or `None` for the automatic colour, and the three booleans.
- The renderer takes `ResolvedLabelStyle`, not the optional fields of the
  profile. The preview, the device image, and the window use the same
  resolution (rule 11).
- The current `LabelStyle` fields `position` and `color` already use `None`
  for the automatic value. The resolved type can keep that meaning.

**The write rule.** Write a key when the surface or the profile has an own
value. Do not write a key for an inherited value. An own value that is equal to
the engine default is written, because it is an own value.

- Schemas 4 to 7 wrote a label size or a bold value only when it was not the
  engine default (`crates/engine/src/store.rs:946-951`). Thus an old surface
  with "20 pt, bold" has no such keys. After the load it inherits. The profile
  default of an old file is empty, thus the surface gets 20 pt and bold.
  Nothing changes. If the user later sets a profile default, the old surface
  follows it. That is correct: the surface never had an own style.
- The old files load with no loss. A file of schema 7 with a new key has an
  error (section 7).

**The profile command.** `Command::SetProfileDefaultLabelStyle { profile:
String, style: LabelStyle }`. It is a profile setting, as `SetProfileColor` is.
It is not a step of the page history. The engine:

- writes the profile through the saved document;
- puts the new style in `ProfileEntry`, thus `Event::Profiles` carries it;
- redraws each surface of the active page and the open folder that inherits a
  changed field;
- sends `Event::Surface` for each redrawn surface.

A change of the default is not in the undo history. The history covers a style
of one surface, not the profile default.

**The type change one time.** `LabelStyle` loses `Copy` when it has a `String`.
One prep PR (Part of #452) makes the whole change: all fields, `font`,
`resolve`, and no `Copy`. It changes no visible output. All existing tests keep
their expected values. The later PRs add the font list, italic and underline,
the profile default, and the large sizes. Thus the type changes one time, not
three times.

`LabelChange` gets the same three-state form for position, colour, and font,
and `Option` for size, bold, italic, and underline. The undo step names are
"Set Title Font", "Set Title Italic", and "Set Title Underline".

### 3. The label size range (ICON-04)

- `MAX_LABEL_SIZE = 80`. `MIN_LABEL_SIZE = 10`.
- `LABEL_SIZES = [10, 12, 14, 16, 18, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80]`.
  Today the list ends at 32, although the store accepts 40.
- The shrink steps:
  - At a requested size of 40 or less: the current rule. The requested size
    comes first, then 2 pixels less each time to 10. A label of 34 to 40 gives
    the same list as today, thus old labels stay pixel identical.
  - Above 40: the requested size, then each `LABEL_SIZES` value above 40 and
    below the requested size, then 40, then the 2-pixel steps. A label of 80
    has at most 21 layouts: 80, 72, 64, 56, 48, and then 40 to 10 in steps of
    2. Today a label of 40 has 16 layouts.
  - The step above 40 is 8 pixels. It agrees with the list and keeps the worst
    case small.
- Schema 8 accepts 10 to 80. Schemas 4 to 7 accept 10 to 40 (section 7).
- An emoji uses the same label path. There is no separate emoji surface type.
  `fit` measures the ink. ICON-04 verifies that a single emoji at 72 to 80
  pixels fills a 112 × 112 face. It measures the ink area at 80 and at 96
  pixels. The owner can change the maximum to 96 before the release if the
  measurement agrees. No schema 8 file is public before 0.7.0, thus the range
  can change without a new version. The schema test then checks the new
  maximum.

### 4. Image crop and zoom (KEY-02)

**The data.**

```rust
pub struct ImageCrop {
    /// The zoom in percent, from 100 to 400.
    pub zoom: u16,
    /// The pan on the x axis, in per mille of the overflow, from -1000 to 1000.
    pub x: i16,
    /// The pan on the y axis, in per mille of the overflow, from -1000 to 1000.
    pub y: i16,
}
```

- `SurfaceContent::image_crop: Option<ImageCrop>`. `None` is the current fit.
- `ImageCrop::default()` is zoom 100, x 0, y 0. The store normalizes
  `Some(default)` to `None`; the file then has no keys.
- The values are integers. `SurfaceContent` derives `Eq`
  (`crates/engine/src/profile.rs:185`). `Page` has a manual `PartialEq` and
  `Eq` that compare the surfaces (`crates/engine/src/profile.rs:380-390`). A
  float needs a manual `Eq`, and a TOML round trip can change it.
- The file keys: `image_zoom` (100 to 400), `image_offset_x` (-1000 to 1000),
  and `image_offset_y` (-1000 to 1000). The keys are independent: an absent
  zoom is 100, and an absent offset is 0. A key is an error on a face with no
  `image`. An `on` face has its own keys.
- The crop is part of the face. The `SetImage` and `SetIcon` arms of
  `edit_face` (`crates/engine/src/engine.rs:3238-3287`) clear the crop when
  they change the image or the icon. `is_empty` includes it. Copy, Paste,
  Clear, Copy to All Pages, the undo history, and the export and import cover
  it because they copy `SurfaceContent`.

**The meaning.**

- `zoom = 100` is the current `draw_image`: the image is as large as possible
  in the image area after the label band, with its proportion, at the centre
  (`crates/engine/src/render.rs:505`). A `None` crop and a 100/0/0 crop give
  the same pixels.
- `zoom` scales the fitted image: `scale = fit_scale * zoom / 100`. At zoom
  400 the image is four times the fit size.
- The image area is the part of the surface after the label band. The crop
  does not include the label band.
- The image is centred, then moved by the pan. The total overflow on an axis
  is `max(0, scaled - area)` pixels. The movement from the centre to an edge
  is half of the overflow. `x = 1000` moves the image to the right until its
  left edge is at the left edge of the area. `x = -1000` moves it to the left
  until its right edge is at the right edge. `y = 1000` moves it down until
  its top edge is at the top. `y = -1000` moves it up until its bottom edge is
  at the bottom. At 0 the image is centred. A positive x shows more of the
  left of the image. A positive y shows more of the top.
- If the overflow on an axis is 0, the pan on that axis has no effect. Every
  value in the range is valid. The renderer needs no clamp state. A value with
  no effect stays in the profile.
- The renderer uses the source rectangle that the scale and the pan select. At
  zoom 100 it uses the current fit path, thus the existing tests keep their
  pixels.
- The engine sends the decoded image size and the image area size for the
  face that the state shows, in `SurfaceState`: `image_size:
  Option<(u32, u32)>` and `image_area: Option<(u32, u32)>`. The inspector
  warns when `src_w * 100 / zoom < area_w` or `src_h * 100 / zoom <
  area_h`. The image copy is at most 512 pixels on a side
  (`crates/engine/src/store.rs:178`). At 400 % a 512-pixel copy gives 128
  effective pixels: enough for a 112-pixel key face, not enough for a
  176-pixel bar panel. That is the warning case.

**The command and the socket.** `Command::SetImageCrop { surface, crop:
Option<ImageCrop> }`. It is a face edit, as `SetLabelStyle` is.
`Command::OnFace` applies it to the On face. The undo step names are "Set
Image Crop" and "Reset the Image Crop". The control socket adds the three keys
to the surface and to the face, and a `set_image_crop` request. `VERSION` stays
1, because the message only adds fields and a command. The CLI and the MCP get
the fields from the same protocol.

### 5. The image path of KEY-01

**No new engine command.** The shell writes the image bytes to a temporary
file and sends the existing `Command::SetImage { path }`. The engine decodes
and copies the image into `images` with `import_image`. Rule 9 stays true.
KEY-01 and KEY-13 change only `crates/app`. The engine, the control protocol,
the CLI, and the MCP do not change.

- The temporary file is in the system temporary directory, with a name from
  random hexadecimal digits. The shell removes it when the engine sends
  `Event::Surface` for the destination control, or a `Problem` for the
  command, and at the stop. A crash can leave a file for each paste that did
  not complete. The file is in the temporary directory, not in the profile.
- The `image` crate gets the features `bmp`, `webp`, and `tiff`. `tiff` is
  necessary because a copy from Preview or another Cocoa application is often
  TIFF only. `tiff` and `image-webp` are already in `Cargo.lock` through
  `gpui-pre-macos`, and `bmp` needs no crate. Thus the features add no crate.
  The main application binary already has these decoders, thus its size does
  not change. Run `scripts/notices.sh` after the change to `Cargo.toml`, and
  commit `THIRD-PARTY-NOTICES.txt` if it changes.
- A file path from `ExternalPaths` (a drop from Finder) goes to `SetImage` as
  it is. A path to an application bundle is KEY-13, not an image: a drop or
  a paste of a bundle sets the action that opens the application, with the
  icon and the name. KEY-13 finds the kind by its path field of kind
  `Application` in `Event::Kinds` (rule 12).

**The system pasteboard.**

- The app reads `NSPasteboard` through `objc2-app-kit` with the `NSPasteboard`
  feature. That feature adds no crate. The app reads the change count, the file
  paths, and the image types.
- Do not use the entry order of gpui. gpui returns a string before an image
  (`gpui-pre-macos-0.3.12/src/pasteboard.rs:57-109`). A browser can put text
  and an image on the pasteboard together.
- On Paste, when a surface is selected and no text field has the focus, and
  the system pasteboard has an image or a file path to an image: the newer
  item wins. The app stores the `changeCount` of the system pasteboard when
  the engine copies a control. At the paste, if the count is different, the
  system item is newer; paste the system item. If the count is the same, paste
  the control. The control clipboard is in the engine, and a copy does not
  change the system pasteboard.
- If the system pasteboard has no image, a paste on a surface uses the control
  as today. A system item with a file path to an image, or with image bytes,
  sets the image. A text item does not set an image; a text field keeps its
  normal paste. An empty control clipboard and a system image give the system
  image.
- The application icon of KEY-13 is a 256-pixel PNG. The picker icon stays 32
  pixels (`crates/app/src/applications.rs:515`). The icon uses the same
  temporary file and `SetImage` path. The icon extraction is
  `#[cfg(target_os = "macos")]`. If it fails, the Open action is still made.

### 6. The clock handler (ACT-31)

**The kind.** `opencraft.clock` in `crates/actions/src/kinds/clock.rs`. The
handler is `ClockHandler` in `crates/actions/src/clock.rs`. The kind is a key
action with the choice `format`:

| Value | Text | Example |
|---|---|---|
| `24h` | `HH:MM` | `14:05` |
| `12h` | `h:mm AM/PM` | `2:05 PM` |
| `date` | `MMM d` | `Nov 22` |
| `weekday-date` | `EEE\nMMM d` | `Sat\nNov 22` |
| `time-date` | `HH:MM\nMMM d` | `14:05\nNov 22` |

- The default is `24h`. The month and weekday names are English in 0.7.0. The
  interface language of 0.7.0 is English
  ([DESIGN.md](../DESIGN.md#6-text-rules)). A localized clock is a later
  change. The 12-hour form uses `AM` and `PM`.
- The kind description uses the library group "Logic", as the timer kind does.
  `theme::kind_tile("Logic")` gives `KIND_TILE_LOGIC`
  (`crates/app/src/view/theme.rs:160-167`). No new token is necessary. The
  icon is `clock` of the built-in set.
- The handler group is `Group::Clock`, with one worker thread for all clock
  bindings, as the timer handler has one worker.

**The worker.**

- One thread. It keeps a map of the visible clock bindings with their settings
  and their last text.
- With no visible clock binding, it blocks on `recv()`. It wakes only for a
  request.
- With at least one visible clock, it waits with `recv_timeout(d)`. `d` is the
  smaller of 10 seconds and the time to the next minute boundary. Thus the
  minute changes at the boundary when the computer is awake, and a wait is at
  most 10 seconds. There are about 6 wakes each minute while a clock shows.
- At each wake, the worker reads the wall time one time. It calls
  `libc::tzset()` before `libc::localtime_r`. It formats the text for each
  visible binding, and sends `Message::Feedback` only when the text is
  different from the last text of that binding. `Appear` makes a binding a
  visible clock and sends the current text at once. `Invoke` sends the current
  text at once for a binding that is a visible clock. `Disappear` removes the
  binding.
- An `Invoke` has the binding of the outer action when the clock is inside a
  Sequence, a Toggle, an action cycle, or the multi-function knob, and the
  engine sends no `Appear` and no `Disappear` to the clock handler for it. The
  worker does not keep such a binding: it would wake for ever and write a text
  for the outer action.
- The text and the wait to the next turn use the same reading of the wall
  time. The text of one reading with the wait of a second reading can be the
  old minute with the wait to the next minute boundary, thus 10 seconds of a
  stale minute.
- The clock sends a `Feedback` with only a label. The engine draws the label
  with the style of the surface (decision 0013). The engine does not read the
  clock settings (rule 8). No engine change is necessary.
- The 10-second cap is necessary because Rust `Instant` on macOS uses a clock
  that stops during system sleep (Inference from the std source; ACT-31
  measures it and writes the fact in
  [kb/sleep-and-wake.md](../kb/sleep-and-wake.md)). `recv_timeout` uses
  `Instant`. Without the cap, after a sleep the handler continues the old wait,
  and the clock can show a time that is wrong for up to 59 seconds. With the
  cap it is correct within 10 seconds. `tzset` makes a time zone change show at
  the next wake. A wait only to the next minute boundary does not meet this
  limit.
- `libc.workspace = true` is added to `crates/actions`. `Cargo.lock` does not
  change, because `libc` is already in the lock. The clock module is
  `#[cfg(unix)]`; on a different system it sends no text, thus the Linux CI
  build passes. The tests inject the wall clock and the monotonic clock, as
  the timer tests do, and are deterministic.
- The idle cost: about 6 wakes each minute while a clock shows, and one image
  each time the text changes: each minute for a time format, and each day for
  a date format. With no clock, no wake. P11-M2 measures the processor use
  against the exit criterion of 0.5 % of one core.

### 7. Profile schema 8

- `SCHEMA_VERSION = 8`. `pub const FACES_VERSION: u32 = 8`, in the form of
  `GESTURE_TIMERS_VERSION` (`crates/engine/src/store.rs:147-152`).
  `FACES_VERSION` is the first version with the face keys of Phase 11.
- One prep PR under #452 sets both constants, makes the `LabelStyle` type
  change of section 2, and adds the load tests for schemas 1 to 7. It adds no
  key and no visible change. Each feature PR adds its keys with the gate
  `version < FACES_VERSION`. No feature PR changes the two constants. Thus the
  parallel lanes cannot choose two versions.
- The complete schema 8 change:
  - The top-level table `[default_label_style]`: `font`, `position`, `size`,
    `color`, `bold`, `italic`, and `underline`.
  - On a surface and on its `on` face: `label_font`, `label_italic`,
    `label_underline`, `image_zoom`, `image_offset_x`, and `image_offset_y`,
    and `label_size` up to 80.
  - `label_position` and `label_color` get the word `"auto"`.
    `label_font` gets the word `"system"`.
  - The value ranges: `label_size` 10 to 80; `image_zoom` 100 to 400;
    `image_offset_x` and `image_offset_y` -1000 to 1000.
  - `[default_label_style]` is a field of `FileData` before `pages`
    (`crates/engine/src/store.rs:715-740`). The field order makes the TOML
    serializer write the table before the array of page tables. Thus no plain
    key follows a table.
- The version gates:
  - A file of schema 1 to 7 with `[default_label_style]` has an error. A file
    of schema 4 to 7 with `label_font`, `label_italic`, `label_underline`,
    `image_zoom`, `image_offset_x`, or `image_offset_y` has an error. A file
    of schema 4 to 7 with `label_position = "auto"` or `label_color = "auto"`
    has an error. A file of schema 4 to 7 with `label_size > 40` has an
    error.
  - A file of schema 1 to 3 has no label keys and no `[default_label_style]`.
    Such a key has an error there too.
  - `deny_unknown_fields` in `FileData` and `SurfaceData` still refuses a key
    that the engine does not know.
  - A file of schema 8 with a key of schema 9 is not `NewerSchema` when its
    header says 8. It has an error, and the engine keeps it as a backup
    (decision 0009). A file with `schema_version = 9` gives `NewerSchema`, and
    the engine does not touch it.
  - The error gives a backup, as decisions 0036 and 0042 do.
- The load of schemas 1 to 7:
  - Each old file loads with its own defaults. An absent label field is
    `Inherit`. An old surface with no key gets the engine default, because the
    profile default is empty. An old value that the file has stays an own
    value.
  - An old surface with `label_size = 20` or `label_bold = true`, for example
    in a hand-written file, keeps an own value. A file that the engine wrote
    did not write those keys (`crates/engine/src/store.rs:946-951`). Such a
    surface inherits, and the result is 20 pixels and bold.
  - The engine writes version 8 at the next change. A file of version 1 to 7
    that the engine does not change stays on its version.
- The tests:
  - One load test for each of schemas 1 to 7 in the version 8 engine. It
    checks the pages, the surfaces, and the defaults.
  - One file of schema 7 with each new key. Each file has an error and a
    backup.
  - One file of schema 4 to 7 with `label_size = 41`. It has an error. One
    file of schema 8 with `label_size = 80`. It loads.
  - A round trip of schema 8 with the profile default, the label style, and
    the crop.
  - An export and an import of a schema 8 profile. An export file from
    release 0.6.0, with schema 7, imports with no loss.
- The export of [decision 0031](0031-profile-export-file.md) uses
  `store::to_text` and `store::parse`. Thus schema 8 applies with no separate
  format. The `images` table does not change.

## Rejected alternatives

| Alternative | Reason |
|---|---|
| Bundle a font file, or add a font library | The binary grows, the licence notices change, and each script needs a font that has it. Core Text and the installed fonts are already present (decision 0010). |
| Find a family with `new_from_name` or `CTFontCreateWithName` | The name is a PostScript name or a full name. A family name can give a different face, for example Helvetica, and there is no error. The membership test in the installed list gives a deterministic fallback. |
| `Option<T>` for every label field, with `None` for both inherit and automatic | A surface cannot choose the automatic position, the automatic colour, or the system font when the profile default has a value. `None` already means automatic for position and colour. |
| Synthetic bold for a family with no bold face | A false bold smears small text and hides the real face. The regular face is the correct fallback. |
| A separate emoji surface type, or a separate emoji renderer | The label path already draws any text. Core Text uses the emoji font. A second path doubles the image code and the tests. |
| Float crop values | `SurfaceContent` derives `Eq`. A float needs a manual `Eq`, and a TOML round trip can change it. Integer per mille is exact. |
| A new `Command::SetImageBytes` | The engine, the socket, the CLI, and the MCP need a new command. The temporary file and the existing `SetImage` use the current import, and KEY-01 and KEY-13 stay in `crates/app`. |
| A new `Event::Fonts` | The font list is data, not engine state. A public function is enough, and rule 12 permits it. |
| A new theme token or a new library group for the clock | The clock uses the library group "Logic", as the timer does. `KIND_TILE_LOGIC` is present. A new token needs both palettes. |
| A wait only to the next minute boundary | `Instant` stops during system sleep, thus the label can be 59 seconds wrong after a wake. The 10-second cap bounds the error. |
| A localized date formatter in 0.7.0 | The interface is English, and a locale formatter adds code and test cases. A later change can add it. |
| A schema version bump in each feature PR | The parallel lanes race. One prep PR sets 8 and `FACES_VERSION`, and the later PRs add keys to 8. |
| A page-history step for the profile default | The default is a profile setting, as the colour is. A page step puts a global setting in each page history. |
| A font cache with no lock | The engine thread draws and the GUI thread reads the list. A lock prevents a data race. |

## Results

**What becomes easier.**

- A profile names a font with no bundled file. A missing font gives the system
  font and no error, and the profile keeps the name.
- One resolution rule covers a surface and the profile default. The preview,
  the device, and the window use the same style.
- The crop is data. The undo history, Copy, Paste, and the export and import
  cover it with no new code.
- The clock needs no engine change. The handler interface, feedback, and the
  label style do the work.
- Schema 8 is one version for the whole phase. Old profiles load, and one
  prep PR prevents a version race.

**What becomes harder.**

- The font list can change while the application runs. A refresh is necessary,
  and a font that is installed after the last refresh is missing until the
  picker opens.
- A family with no bold face shows its regular weight. The user can think that
  bold is off.
- Synthetic italic needs the smaller layout width and the shifted draw. A test
  must check the right edge and the centre.
- The clock wakes about 6 times a minute while a clock shows. The idle
  measurement must confirm the target.
- The label style type change touches many files one time. The prep PR is
  mechanical but wide.
- Schema 8 fixes the key set for the phase. A later key needs version 9.

**When to look at this decision again.**

- ICON-04 measures the largest size and the emoji ink. The owner can set the
  final maximum before 0.7.0.
- ACT-31 measures the sleep behaviour and the processor use. If a later macOS
  keeps time during sleep, the 10-second cap can go.
- A later phase can add a localized clock or a bundled font. That needs a new
  record.
- If a later macOS changes the font API or the installed family list, measure
  the change again.
- If the picker needs a preview of each font, the window needs a new design
  decision.
