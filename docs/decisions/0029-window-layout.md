# 0029: The main window has a sidebar, a canvas, and a right panel, and each new feature has one place in it

- Status: Accepted
- Date: 2026-11-12

## Context

The window of Phase 6 is one screen: the device picture and an editor
panel. The page row, the profile menu, and the device settings are buttons
at the top. Each new feature adds more buttons at the top. Other software
for these devices has plug-ins, folders, double press and hold, a profile
switch by application, more devices, icon packs, and animated faces. The
window has no place for them.

Issue [#173](https://github.com/shantanugoel/OpenCraft/issues/173) proposed
a new layout. The wireframes are on a design canvas with 15 boards. The owner
approved the proposal as the base for the overhaul. The research facts are
in [kb/ui-research.md](../kb/ui-research.md). The facts about the vendor
software and its web view are in [kb/vsd-craft.md](../kb/vsd-craft.md).

This record is the text specification for the tasks of #173. An agent does
each task with this record and the issue only. Thus the record is longer
than other records, and it gives sizes, colours, and text. If this record
and an issue disagree, this record is correct. Section 24 gives the sections
for each task.

## Decision

### 1. How to read this record

- All sizes are in points. One point is one pixel of the wireframes, which
  show a window of 1440 × 900.
- "Section N" refers to a numbered section of this record.
- The wireframes show the dark appearance. This record gives the values for
  the dark appearance only. See section 21 for the light appearance.
- A **LATER place** is a place for a feature that comes later. The
  wireframes show it as an orange dashed box with the word LATER. **The
  application does not draw a LATER place**, unless this record says so. The
  code keeps the place: the list, the tab row, or the section stack takes one
  more entry with no change to the layout. Section 21 lists all LATER places.
- The text in quotation marks is the text on the window. Use it exactly.
- The wireframes use the kind name "Open an app". The window uses the name
  that the engine gives in `KindDescription`, which is "Open an
  application". The window never has its own list of kind names (rule 8).
- The wireframes use the font Inter as a substitute. The window uses the
  system font (section 3.1).

### 2. Words

| Word | Meaning |
|---|---|
| Sidebar | The pane on the leading (left) side. It shows the device, the profiles, and the pages. |
| Canvas | The pane in the centre. It shows the device and the active page. |
| Right panel | The pane on the trailing (right) side. It has the tabs "Inspector" and "Library". |
| Inspector | The tab of the right panel that changes the selection |
| Library | The tab of the right panel that lists the action kinds |
| Selection | The thing that the inspector shows: one control, the page, or the device (section 10.1) |
| Gesture card | The part of the inspector for one gesture of the selected control (section 10.3) |
| Accent | The colour of the selection and of the main button (section 3.2) |

### 3. Visual constants

**Replaced (2026-11-13).** [DESIGN.md](../DESIGN.md) replaces this section
for the visual constants, the states, and the text rules. The values are in
`crates/app/src/view/theme.rs` only. If this section and DESIGN.md
disagree, DESIGN.md and `theme.rs` are correct. This section stays as the
history of the first values.

These constants give one style to all parts of the window. Issue
[#209](https://github.com/shantanugoel/OpenCraft/issues/209) put each one
into `crates/app/src/view/theme.rs` and wrote `docs/DESIGN.md`. Do not
write a colour or a size as a number in more than one place.

#### 3.1 Font and text sizes

The font is the system user interface font (San Francisco). In GPUI, use the
system font name, not a font file. Do not bundle a font.

| Name | Size | Weight | Use |
|---|---|---|---|
| `badge` | 10 | 400 to 700 | The mark "ON DEVICE", the hex value of a knob light, the size text of a background preview |
| `section` | 11 | 600, letter spacing 0.06 em | Section headers. The code writes the text in capital letters, for example "APPEARANCE". |
| `caption` | 11 | 400 or 500 | A number at the right of a row, a hint at the right of a section header, a knob label |
| `help` | 11.5 | 400 | Status bar, help text below a control, the live state pill |
| `label` | 12 | 400 or 500 | The labels of a form, small buttons, segmented controls, a page number text |
| `control` | 12.5 | 400 or 500 | Text in a popup button, in a search field, in a step row |
| `body` | 13 | 400, 500, or 600 | Rows in the sidebar and the library, text fields, the toolbar title |
| `heading` | 15 | 600 | The page name in the page header, the name in the inspector header |
| `sheet-title` | 17 | 600 | The title of a sheet; the text in the command palette field (weight 400) |
| `display` | 22 | 600, letter spacing −0.01 em | The title of the first start window |

Line height is 1.45 for text of more than one line. Else use the default
line height of GPUI.

#### 3.2 Colours (dark appearance)

**Surfaces of the window**

| Name | Value | Use |
|---|---|---|
| `bg-canvas` | `#18191C` | The canvas, the window behind all panes |
| `bg-bar` | `#1F2024` | The toolbar, the status bar, a text field in a card, the first start window |
| `bg-sidebar` | `#232428` | The sidebar, the footer of a sheet, the title area of the Settings window |
| `bg-panel` | `#202125` | The right panel, a nested card |
| `bg-card` | `#26272C` | A gesture card, a group box of the Settings window, a sheet |
| `bg-control` | `#2F3036` | A button, a popup button |
| `bg-field` | `#2A2B30` | A text field, a search field, the track of a segmented control, a small square button, a popover, the command palette, the device card |
| `bg-segment-on` | `#4A4C53` | The selected segment of a segmented control, a toggle button that is on |
| `bg-tile` | `#2E3036` | The tile behind the device icon, a tile of a command or a page in the command palette |

**Lines**

| Name | Value | Use |
|---|---|---|
| `line-pane` | `#34363B` | The border between two panes, the line between the head and the body of a gesture card |
| `line-section` | `#2C2E33` | The line above a section, below the toolbar, above the status bar, below the tab header |
| `line-sidebar` | `#2E3035` | The line above the bottom group of the sidebar |
| `border-field` | `#3A3C42` | The border of a text field, of a card, of a segmented control track |
| `border-control` | `#42444B` | The border of a button and of a popup button |
| `border-overlay` | `#44474F` | The border of a popover, of the command palette, of a sheet; a dashed empty card |
| `border-device-card` | `#383A40` | The border of the device card when it is not selected |

**Text**

| Name | Value | Use |
|---|---|---|
| `text-primary` | `#ECECEE` | Normal text |
| `text-secondary` | `#A0A3AB` | Form labels, icon buttons, a value at the right |
| `text-tertiary` | `#8B8E96` | Section headers, captions, help text, placeholders |
| `text-disabled` | `#6B6E76` | A disabled button or icon |
| `text-on-accent` | `#FFFFFF` | Text on an accent fill |
| `text-icon-cell` | `#D0D2D8` | An icon in the icon picker grid |

**Accent and states**

| Name | Value | Use |
|---|---|---|
| `accent` | `#1A6FD8` | The selected row, the main button, the on state of a switch, the filled part of a slider |
| `accent-line` | `#3B95FF` | The selection outline, the focus border of a field, the border of a selected chip |
| `accent-text` | `#4DA3FF` | A link, the label of a selected knob, the dot of the last input |
| `accent-text-hover` | `#7DBBFF` | A link under the pointer |
| `accent-tint` | `#1A6FD8` at 14 % (a selected card), 16 % (a selected start card), 18 % (a library row in a drag), 25 % (a selected chip), 28 % (a drop target) | Accent backgrounds |
| `success` | `#34C759` | The dot of a connected device, the mark "ON DEVICE" |
| `success-text` | `#7EE2A0` | The text of the live state pill |
| `success-tint` | `#34C759` at 12 % | The background of the live state pill |
| `success-fill` | `#1E7A46` | The check badge of the first start |
| `destructive-text` | `#FF8A80` | The text of "Remove…" and "Remove the profile…" |
| `warning` | `#F5B65C` | The text "not given" of a permission, the icon of a warning |
| `input-glow` | The current colour of the live input glow (`ACTIVE`, `#FFCE73`, in `crates/app/src/view.rs`) | A control that the user operates on the device |

The wireframes also use `#F5B65C` for LATER places. The application does
not draw LATER places (section 1), thus this use does not occur.

**The device drawing**

| Name | Value | Use |
|---|---|---|
| `device-body` | `#0B0C0E`, border 1 `#2E3036` | The body of the device |
| `device-display` | `#0F1724` | The display when the page has no background image |
| `device-key-empty-border` | `#3A3F4C`, dashed | The border of an empty key face |
| `device-key-empty-icon` | `#5A5F6B` | The plus icon on an empty key face |
| `device-surface-border` | `#FFFFFF` at 6 % | The border of a key face and of a bar panel |
| `device-knob` | `#1C1E23` | The inside of a knob |
| `device-knob-ring` | `#3A3C42` | The ring of a knob when the knob light mode is not "Colours" |
| `device-knob-dot` | `#ECECEE` | The mark at the top of a knob |

**Colour samples that are data.** The fill colours of a surface are the
current 12 samples (`COLOURS` in `crates/app/src/view.rs`). The knob light
colours are the current 9 samples (`KNOB_COLOURS` in
`crates/app/src/view/device.rs`). The colours of the samples in the
wireframes are only examples. Do not change the stored colours.

**Shadows.** A popover: 0 18 40, black at 50 %. The command palette and a
sheet: 0 24 60, black at 55 %. The drag picture: 0 12 28, black at 50 %. The
device body: 0 24 60, black at 45 %.

#### 3.3 Spacing steps

Use only these values for padding and gaps: 1, 2, 3, 4, 6, 8, 10, 12, 14,
16, 18, 20, 24, 28, 32.

| Place | Value |
|---|---|
| The padding of a section of the right panel | 14 at the top, 16 at the sides, 16 at the bottom |
| The gap between the columns of a form, and between its rows | 10 (8 in a gesture card) |
| The gap between gesture cards | 8 |
| The gap between buttons in a row | 6 |
| The gap between the toolbar items | 6 |
| The gap between rows of the sidebar | 1 |
| The side padding of the sidebar row group | 10 |

#### 3.4 Corner radii

| Value | Use |
|---|---|
| 4 | A badge, the box of a key hint ("⌘K") |
| 5 | A segment, a fill sample, a sidebar "+" button, a kind tile of 20 |
| 6 | A button, a text field, a popup button, a sidebar row, a kind tile of 24 |
| 7 | A segmented control track, a search field, a library row, a main button |
| 8 | A nested card, a row of the command palette, an icon cell, a bar panel on the canvas |
| 9 | A gesture card, the device card |
| 10 | A group box of the Settings window, a starter profile card, a face button of a toggle |
| 11 | A face of a toggle state, the device icon tile of the device inspector |
| 12 | The window, a popover, a key face on the canvas and in the inspector header |
| 14 | A sheet, the command palette |
| Full | A switch, a chip (radius 12 at height 24), a dot, a knob, a colour sample of a profile |

#### 3.5 Control heights

| Height | Control |
|---|---|
| 18 | A switch in the inspector (width 30) |
| 19 | A switch in the Settings window (width 32) |
| 22 | A segment in a form, a fill sample (22 × 22), the "◀ Touch bar ▶" control |
| 24 | A filter chip, a badge, the live state pill, a menu item of the menu bar item |
| 26 | A segment of the tab control, the page header buttons, "Test the action" |
| 28 | A button, a text field, a sidebar row, a toolbar button, the toolbar search field, a square button (28 × 28) |
| 30 | The action popup of a gesture card, a search field of a panel, a field of a sheet, a main button of a sheet |
| 34 | A library row, a step row |
| 36 | A row of the command palette, the footer of the command palette |
| 40 | An empty gesture card |

#### 3.6 Icons

- The window uses line icons from Lucide, through `gpui_kit::assets::IconName`
  ([kb/ui-research.md](../kb/ui-research.md#the-icon-set-in-the-gui-toolkit)).
  Embed only the icons that the window uses, with `icon_assets!`. Do not
  register `AllAssets` for the window chrome.
- Icon sizes: 12 in a small button, 14 in a search field or a library tile,
  15 in a sidebar row, 16 in a toolbar button, 17 for the pane buttons, 20 in
  a Settings tab, 26 in the device inspector header.
- The icon colour is the text colour of its control.
- The icon set for the key faces is the decision of issue #190. It can be the
  same set. Note (2026-11-13): it is the same set, Lucide 1.43.0, and the
  engine draws it ([decision 0033](0033-surface-icons.md)).

#### 3.7 States

| State | Look |
|---|---|
| Hover on a button | Background one step lighter: `bg-control` changes to `#383A40`; a transparent icon button gets `#FFFFFF` at 6 % |
| Hover on a row (sidebar, library, menu) | Background `#FFFFFF` at 5 % |
| Pressed | Background `#42444B` |
| Selected row | Background `accent`, text `text-on-accent` |
| Selected row in a pane that does not have the selection | Background `#FFFFFF` at 7 %, normal text (example: the active page row when the device is selected) |
| Selected segment | Background `bg-segment-on`, text `text-on-accent`. The other segments are transparent with `text-secondary`. |
| Selected chip | Border `accent-line`, background `accent-tint` 25 %, text `text-on-accent`. The other chips: border `border-field`, transparent, `text-secondary`. |
| Selected sample (colour, icon) | Outline 2 `accent-line`, 2 outside the sample |
| Disabled | Text and icon `text-disabled`, no hover, no pointer change |
| Focus | Text field: border `accent-line`. Other control: outline 2 `accent-line`. |
| Destructive button | Normal button with `destructive-text` |
| Main button | Background `accent`, no border, text `text-on-accent`, weight 600 |
| Link | `accent-text`, no underline; hover `accent-text-hover` |

A button that opens a dialog, a sheet, a popover, or a file panel has "…" at
the end of its label, for example "Image file…". A button that does the
work at once has no "…".

Each icon button has a tooltip. The tooltip text is the accessible label in
this record, for example "Hide the inspector".

### 4. The window and the panes

```
+------------+---------------------------------------------+------------+
| (window    | toolbar (52)                                             |
|  buttons)  +---------------------------------------------+------------+
|            | page header (48)                            | tab header |
|  sidebar   |                                             |            |
|  (240)     |            canvas: the device               | right      |
|            |                                             | panel      |
|            |            help line                        | (360)      |
|            +---------------------------------------------+            |
|            | status bar (28)                             |            |
+------------+---------------------------------------------+------------+
```

- **Order.** Sidebar on the leading side, then the canvas, then the right
  panel on the trailing side.
- **Sidebar.** Full window height. Default width 240, minimum 200,
  maximum 320.
- **Right panel.** From the bottom of the toolbar to the bottom of the
  window. Default width 360, minimum 320, maximum 480.
- **Toolbar.** Height 52. It is above the canvas and the right panel, not
  above the sidebar (section 5).
- **Status bar.** Height 28. It is below the canvas only, not below the
  right panel (section 6).
- **Canvas.** All the other space. Minimum width 480.
- **Dividers.** A line of 1 (`line-pane`) between two panes. The pointer area
  of a divider is 6 wide. A drag on the divider changes the width of the
  pane between its minimum and its maximum.
- **Default window size.** 1440 × 900. If the screen is smaller, the window
  takes the visible area of the screen. At 1440 × 900 the canvas is 840 wide,
  and the device shows at the scale 0.925 (section 8.2).
- **Minimum window size.** 1080 × 700. Thus the panes do not overlap when
  both side panes are visible at their default widths.
- **Title bar.** The window has a transparent title bar. The system draws the
  window buttons in the top-left corner, on the sidebar, at their centre
  height 26. The sidebar and the toolbar fill the area of the title bar. A
  drag on an empty part of the toolbar or of the top of the sidebar moves
  the window.
- **Show and hide.** The View menu has these commands. The title of each
  command agrees with the state of the pane.

| Command | Key command | Also from |
|---|---|---|
| "Hide Sidebar" / "Show Sidebar" | ⌃⌘S | The button "Hide the sidebar" at the top of the sidebar, and "Show the sidebar" in the toolbar |
| "Hide Inspector" / "Show Inspector" | ⌥⌘I | The button "Hide the inspector" / "Show the inspector" at the end of the toolbar |

- When the sidebar is hidden, the toolbar starts at the leading edge of the
  window. Its leading padding becomes 78, so the window buttons have space.
  The button "Show the sidebar" (icon: a rectangle with a line at the left)
  is the first toolbar item.
- When the right panel is hidden, the canvas and the status bar fill its
  space. The toolbar stays full width.
- **Kept between two starts.** The shell keeps these values in `shell.toml`
  ([decision 0020](0020-menu-bar-item-and-dock-icon.md#the-dock-icon-option)):
  `sidebar_visible` (default `true`), `inspector_visible` (default `true`),
  `sidebar_width` (default 240), `inspector_width` (default 360), and
  `right_panel_tab` (`"inspector"` or `"library"`, default `"inspector"`).
  A missing or incorrect value gives the default.

### 5. The toolbar

Height 52, background `bg-bar`, line `line-section` at the bottom. Padding 0
at the top and the bottom, 18 at the leading side, 14 at the trailing side.
Gap 6. The items, from the leading side:

1. **Title block.** Two lines, gap 1, 10 to the next item:
   - The page name: `body`, weight 600, `text-primary`. Example: "Main".
   - The profile and the number of pages: `caption`, `text-tertiary`.
     Example: "Work · 3 pages". With one page: "Work · 1 page".
2. **Undo** (#186). Icon button 30 × 28, radius 6, transparent, icon 16
   (undo arrow), `text-secondary`. Disabled when the undo list is empty.
   Tooltip: "Undo" and the name of the change, for example "Undo Set Title".
3. **Redo** (#186). The same as Undo, with the redo arrow. Disabled when the
   redo list is empty.
4. A flexible space.
5. **Live state pill.** Height 24, radius 12, padding 0 10, gap 6, `help`
   weight 500. A dot of 6 at the start.

| State | Text | Dot | Text colour | Background |
|---|---|---|---|---|
| Connected | "Live on the device" | `success` | `success-text` | `success-tint` |
| Not connected | "Not connected" | `text-disabled` | `text-secondary` | `#FFFFFF` at 6 % |

6. **Search field** (#192). A button that looks like a search field: 240 ×
   28, radius 7, background `bg-field`, border `border-field`, padding 0 8,
   gap 8, 8 from the pill. A search icon 14, the text "Search" (`control`
   weight 400, `text-tertiary`), and at the end the key hint "⌘K" (`caption`,
   padding 1 5, border 1 `border-overlay`, radius 4). A click opens the
   command palette (section 15). Accessible label: "Search actions,
   controls, pages and commands". Until #192, the toolbar keeps the space
   empty.
7. **Hide the inspector.** Icon button 30 × 28, icon 17 (a rectangle with a
   line at the right). Tooltip "Hide the inspector", or "Show the inspector"
   when the right panel is hidden.

Until #177, the current page row and profile menu stay in the toolbar,
between the title block and the flexible space. Until #180, the button
"Device settings" stays there also. #177 and #180 remove them.

The toolbar has no other buttons. A new command goes into a menu, into the
command palette, or into the inspector (section 22).

### 6. The status bar and the messages

#### 6.1 The status bar

Height 28, background `bg-bar`, line `line-section` at the top. Padding 0 16,
gap 8, `help`, `text-secondary`.

- **Left side: the last input.** A dot of 6 in `accent-text`, then the text
  "*control* · *gesture* · *time*".
  - Examples: "Key 1 · Press · now", "Knob 1 · Turn right × 3 · now".
  - The control names are the names of section 10.1.
  - When the same gesture on the same control comes again within 1 second,
    the text shows "× *count*" after the gesture.
  - The time is "now" for 10 seconds after the input. Then it is the local
    time as "HH:MM". Use one timer for each input, not a timer that runs all
    the time.
  - Before the first input: "No input yet", with no dot.
- **Left side: a message.** Some operations show a message in place of the
  last input, for 4 seconds, with no dot. Then the last input shows again.
  The messages:
  - After a drop or a click in the library: "*Control* gets “*kind name*”
    for *gesture*". Example: "Key 8 gets “Open an application” for Press".
  - After a click in the library with no selected control: "Select a control
    first".
  - A later task can add a message of this form. It does not add a new bar.
- **Right side: the save state.** "All changes saved". The engine writes the
  profile (rule 9), and it sends no event when the file is written. Thus the
  text means: the engine has each change. A failed write shows as a message
  (section 6.2). The shell does not read the text of a message to change the
  save state. If a later engine event gives the save state, the text follows
  that event.

#### 6.2 Messages

The current messages of the window (an `Event::Problem` text with a "Close"
button, as in [docs/user/faults.md](../user/faults.md#messages-in-the-window))
stay. They show at the bottom of the canvas, directly above the status bar,
one row for each message:

- Row: padding 8 16, gap 10, background `#FF8A80` at 12 %, line
  `#FF8A80` at 35 % at the top. A warning icon 14 in `warning`, the text
  (`label`, `text-primary`, more than one line if necessary), and a "Close"
  button (height 26) at the end.
- The behaviour of the messages does not change.

### 7. The sidebar

Width section 4. Background `bg-sidebar`, line `line-pane` at the trailing
side. Accessible label: "Devices, profiles and pages". The parts, from the
top:

#### 7.1 The top area

Height 52. The window buttons are at the leading side (section 4). At the
trailing side, 12 from the edge: the button "Hide the sidebar", 30 × 28,
icon 17 (a rectangle with a line at the left), `text-secondary`.

#### 7.2 The device card

In a box with padding 2 at the top, 10 at the sides, 8 at the bottom. The
card is a button of full width: padding 8, radius 9, gap 10.

- An icon tile 32 × 32, radius 8, background `bg-tile`, with a device icon
  18 in `text-primary`.
- Two lines, gap 2:
  - The model name: `body` weight 600. Example: "VSD N4 Pro". The name comes
    from the layout of the engine (`Event::Layout`), not from a list in the
    shell (rule 4).
  - The state: `caption`, gap 5, a dot of 6. "Connected" with a `success`
    dot, or "Not connected" with a `text-disabled` dot.
- An up and down chevron 14 at the end. It is the device switcher. A click
  opens a menu with one item for each device that the engine knows. Now it
  lists the one device. More devices are a LATER place (section 21).

| State | Background | Border | Text |
|---|---|---|---|
| Not selected | `bg-field` | `border-device-card` | `text-primary`; state in `text-secondary` |
| Selected (the device is the selection) | `accent` | `accent-line` | `text-on-accent` |

A click on the card selects the device (section 10.1). The inspector then
shows the device settings (section 10.7). Accessible label:
"Device VSD N4 Pro, connected. Select to show the device settings."

#### 7.3 The "Profiles" section

- **Header.** Padding 12 at the top, 12 at the trailing side, 6 at the
  bottom, 18 at the leading side. The text "PROFILES" (`section`,
  `text-tertiary`), and at the end a "+" icon button 24 × 22, radius 5,
  icon 14, accessible label "Add a profile". A click opens a native command
  menu: "New Profile" starts an edit of the name in a new row (section 7.5);
  "Import a Profile…" opens the file dialog. Both commands come from
  `app_menu::commands` and are also in the command palette. The application
  has no File menu or profile commands in the menu bar.
- **Rows.** A group with padding 0 10, gap 1. Two levels at the maximum:
  profiles, and the pages of the active profile.

**Profile row.** Height 28, padding 0 8, radius 6, gap 6, `body` weight 500,
`text-primary`.

1. A disclosure chevron 12, `text-tertiary`: down for the active profile,
   right for the other profiles.
2. A profile icon 15 (Lucide "layers"), `text-secondary`. After #195, the
   icon has the colour of the profile, if the profile has a colour.
3. The name, which takes the free width. A long name ends with "…".
4. On the active profile only: the mark "ON DEVICE" (`badge` weight 600,
   `success`).

A click on a row of a profile that is not active selects that profile
(`Command::SelectProfile`). The engine then sends the pages of that profile,
and that profile shows its pages. The engine sends the pages of the active
profile only (`Event::Pages`), thus only the active profile has page rows.
A click on the chevron of the active profile hides or shows its page rows.
The first profile row after the page rows of the active profile has 4 more
space at the top.

**Page row.** Height 28, padding 0 8 at the trailing side and 50 at the
leading side, radius 6, gap 8, `body` weight 400, `text-primary`.

- The page name, then at the end the page number (`caption`,
  `text-tertiary`), starting at 1.
- The active page is the selected row (section 3.7): background `accent`,
  text and number `text-on-accent`, weight 500. When the device is the
  selection, the active page row has the look "selected row in a pane that
  does not have the selection".
- A click selects the page (`Command::SelectPage`). The command carries
  the profile of the frame that the user clicked (`Command::InProfile`,
  issue #321).

**"Add a page" row.** Below the page rows of the active profile. Height 28,
padding 50 at the leading side, gap 6, `control` weight 400,
`text-tertiary`, a plus icon 12. A click adds a page at the end, as the
current command does.

#### 7.4 The bottom group

A flexible space, then a group with a line `line-sidebar` at the top,
padding 8 10 14, gap 1:

- **"Settings" row.** Height 28, padding 0 8, gap 8, `body` weight 400,
  `text-secondary`. A gear icon 15, the text "Settings", and at the end
  "⌘," (`caption`, `text-tertiary`). A click opens the Settings window
  (section 17). The row is disabled until #193.
- The "Plug-ins" row of the wireframes is a LATER place, above "Settings".

The bottom group contains no critical command
([kb/ui-research.md](../kb/ui-research.md#apple-human-interface-guidelines)).

#### 7.5 Menus and edits in the sidebar

| Row | Right-click menu |
|---|---|
| Profile | "Rename", "Duplicate", "Profile Settings…" (#195), separator, "Remove…" |
| Page | "Rename", "Duplicate", "Move Up", "Move Down", separator, "Remove…" |

- "Rename" and a double-click on a name start an edit in the row: the name
  becomes a text field of the same size. Return keeps the name. Escape
  cancels. A click outside keeps the name. An empty name is not possible:
  Return then does nothing.
- "Remove…" asks for a confirmation, as now. The confirmation names the
  profile or the page.
- "Move Up" is disabled on the first page. "Move Down" is disabled on the
  last page.
- A drag of a page row to change the order is a LATER place.
- The engine gets the same commands as from the current controls.

### 8. The canvas

Background `bg-canvas`. From the top: the page header, the device area, the
messages (section 6.2), the status bar.

#### 8.1 The page header

Height 48, padding 0 20, gap 10. No line.

1. The page name: `heading`. Example: "Main".
2. "Page *N* of *M*": `label`, `text-tertiary`. Example: "Page 1 of 3".
3. A flexible space.
4. "Previous page": a square button 28 × 26, radius 6, background
   `bg-field`, border `border-field`, chevron left 14. Disabled on the first
   page.
5. "Next page": the same, chevron right. Disabled on the last page.
6. "Page settings": a button, height 26, padding 0 10, `label` weight 500.
   A click selects the page (section 10.1) and shows the Inspector tab.

The folder path of the wireframe ("Main › OBS") is a LATER place, after the
page name.

#### 8.2 The device area

The device area is a column, centred in both directions, gap 18: the device
drawing, then the help line.

**Scale.** The drawing uses one scale *s* for all of its parts. *s* is the
largest value between 0.5 and 1.0 for which the device body fits in the
device area, with 24 of space at each side and the help line below. At the
default window size, *s* is (840 − 48) / 856 = 0.925: the width of the
canvas sets it. (The wireframes show the scale 0.84, with more space at the
sides. The rule above gives 0.925, and the application obeys the rule.)
The values below are in device pixels of
the 800 × 480 display. Multiply each value by *s* to get points.

**The parts of the drawing**

| Part | Size and position (device pixels) | Look |
|---|---|---|
| Body | The display with a margin of 28 at the left, the right, and the top. Below the display: the knobs and their labels (below), and a margin of 44 below the labels. For the N4 Pro: about 857 × 690. | `device-body`, radius 33, shadow (section 3.2) |
| Display | 800 × 480, at the margin | The background image of the page from the engine (#191). With no background: `device-display`. Radius 12. |
| Key face, bar panel | At the position and size of each surface in `Event::Layout`. For the N4 Pro: [kb/n4-pro-hardware.md](../kb/n4-pro-hardware.md#positions-on-the-display) | The image of the engine only (rule 11). Border 1 `device-surface-border`. Radius 14 for a key face, 10 for a bar panel. |
| Knob | Diameter 90. The centre is below the centre of its feedback bar panel (from the layout). The top is 22 below the display. A knob with no feedback surface: the knobs share the width of the display in equal parts. | Inside `device-knob`, ring 4. The ring has the colour of the knob light when the knob light mode is "Colours", else `device-knob-ring`. A mark of 7 at the top centre, `device-knob-dot`. |
| Knob label | Centred below the knob, 8 below it | "Knob *N*", `caption` weight 500 (not scaled), `text-tertiary`; `accent-text` when the knob is selected |

The shell does not check for a model (rule 4). All positions come from the
layout.

**An empty key face.** A key face with no label, no image, no icon, and no
binding gets a dashed border 1 in `device-key-empty-border` and a plus icon
of 28 × *s* in `device-key-empty-icon`, on top of the engine image.

**The "◀ Touch bar ▶" control.** A small control, not scaled: height 22,
radius 11, padding 0 10, gap 6, `caption` weight 500. A chevron left 12, the
text "Touch bar", a chevron right 12. It is centred on the display
horizontally, and centred in the space between the lowest key face and the
highest bar panel vertically. Normal: background `#FFFFFF` at 6 %, border 1
`#FFFFFF` at 10 %, `text-secondary`. Selected: background `accent`, border
`accent-line`, `text-on-accent`. Accessible label: "Touch bar: swipe
gestures". A click selects the page and scrolls the inspector to "Touch bar
gestures" (section 10.6).

Note (2026-11-14), from issue #158. A knob has its own element. The touch
bar has its own element. Each other control that has no surface has one
element below the display.

The elements are in rows. A lower row number is higher on the device. In
one row, a lower column number is further to the left. The controls of one
row share the width of the display. Each element is a square of
`DEVICE_PLAIN_SIZE` (96 pixels of the display). A row that does not fit
uses a smaller square. The gap is `DEVICE_PLAIN_GAP` (16).
`DEVICE_PLAIN_TOP` (22) is the space above the block. The space is below
the knob labels when the layout has a knob. The space is below the display
when the layout has no knob.

The element has a dashed border and the name of the control, for example
"Key 7". The selection outline is the outline of a key face. A click
selects that control. The click does not select the device. The inspector
shows the gestures of the control. It shows no "Appearance" section,
because the control has no display.

The descriptor has no place for a knob relative to the display. Each knob
stays in one row below the display. Measure that place before it becomes
descriptor data, and write the measurement in `docs/kb/`.

#### 8.3 Selection and pointer on the canvas

| Click on | Selects |
|---|---|
| A key face | That key |
| A bar panel | That bar panel |
| A knob | That knob |
| The "◀ Touch bar ▶" control | The page, with the touch bar section (section 10.6) |
| An empty part of the display | The page (until #181 the inspector shows nothing for the page) |
| The body of the device, outside the display and the knobs | The device (#180) |

- **Selected control.** An outline of 2 in `accent-line`, 3 outside the
  control (4 for a knob). Selected device: the same outline around the body,
  4 outside it.
- **Feedback surface of a selected knob.** A dashed outline of 2 in
  `accent-line`, 3 outside the bar panel.
- **Hover on a control.** An outline of 1 in `accent-line` at 50 %, 3
  outside the control. The pointer is a hand.
- **Live input.** A control that the user operates on the device glows in
  `input-glow`, as now. The touch point shows on the touch bar, as now. The
  live input shows also when the control is selected.
- **Right-click on a control** (#187): a menu with "Copy", "Paste", "Clear".
- The selection stays when the page changes: the same control on the new
  page is selected.
- At the start, the selection is the first key of the layout (the top row,
  first key).

#### 8.4 The help line

One line below the device: `label`, `text-tertiary`, centred. The text
depends on the state:

| State | Text |
|---|---|
| Normal | "Click a control to change it · Drag an action onto a control · Right-click for Copy, Paste and Clear" |
| A knob is selected | "The dashed outline shows the bar panel that gives feedback for Knob *N*" |
| A drag from the library is in progress | "Drop on a key, a bar panel or a knob. Drop on a key with an action to replace it. ⌘Z undoes it." |

The line names only functions that operate. Until #185 the normal text has
no part "Drag an action onto a control". Until #187 it has no part
"Right-click for Copy, Paste and Clear". Until #186 the drag text has no
part "⌘Z undoes it.".

### 9. The right panel and its tabs

Width section 4. Background `bg-panel`, line `line-pane` at the leading side.

- **Tab header.** Padding 12 16, line `line-section` at the bottom. A
  segmented control of full width: track `bg-field`, radius 7, padding 2;
  two segments of equal width, height 26, radius 5, `label` weight 500:
  "Inspector" and "Library". The selected segment has the look of section
  3.7.
- **Content.** Below the tab header. It scrolls vertically when it is higher
  than the panel. The tab header does not scroll.
- The tab stays when the selection changes. "Page settings" (section 8.1)
  and ⌘↵ in the command palette change the tab to "Inspector".
- Until #184, "Library" shows an empty state: the text "The action library
  comes in a later version." in `label`, `text-tertiary`, centred, 40 from
  the top.

### 10. The inspector

#### 10.1 The inspector states

The content follows the selection.

| Selection | Name in the header | Second line | Sections |
|---|---|---|---|
| A key | "Key *N*" | The position, for example "Top row, first key" | Appearance, Gestures (10.2, 10.3) |
| A bar panel | "Bar panel *N*" | The position, for example "Touch bar, first panel" | Appearance, Gestures |
| A knob | "Knob *N*" | "Feedback on bar panel *N*" | Gestures, Bar panel *N* (10.4) |
| The page | "Page “*name*”" | "Page *N* of *M* · Profile “*name*”" | Page, Background, Touch bar gestures (10.6) |
| The device | The model name | The connection state | Display, Feedback, Knob lights (10.7) |

- The numbers of the controls are the numbers of the user guide
  ([docs/user/install.md](../user/install.md#keys-and-bar-panels-the-numbers)),
  as `control_name` in `crates/app/src/view.rs` gives them now.
- The position text: "Top row" or "Second row", and "first key" to "fifth
  key", from the order of the surfaces in the layout. For a bar panel:
  "Touch bar, first panel" to "fourth panel". Use the words "first",
  "second", "third", "fourth", "fifth"; above five, use "key *N*".
- With no selection, the inspector shows "Select a control, the page or the
  device." (`label`, `text-tertiary`, centred, 40 from the top).

#### 10.2 The header, the sections, the form, and "Appearance"

**Header.** Padding 16, gap 12, no line at the top.

- A picture of 64 × 64: the image of the surface from the engine, radius 12
  (key face); the image of the bar panel scaled to a width of 64 (bar panel);
  a knob picture (section 10.4); a page picture (section 10.6); the device
  icon tile (section 10.7).
- Two lines, gap 3: the name (`heading`), the second line (`label`,
  `text-tertiary`).
- At the end, for a control: the "⋯" button, 28 × 28, radius 6, background
  `bg-field`, border `border-field`, icon 16 (three dots). Accessible label:
  "More for Key 1". A click opens a menu: "Copy", "Paste", "Clear",
  separator, "Copy to All Pages" (#187). Until #187, the button is disabled.

**Section.** Padding (section 3.3), line `line-section` at the top.

- Header: the text in `section`, `text-tertiary`, 12 above the content. A
  section can have a hint at the end of the header line: `caption`,
  `text-tertiary`, for example "What the key does". With a hint, the space
  below the header is 4, and the content has a gap of 8.

**Form.** A grid of two columns: the label column (64 wide; 72 in the page
and the device inspector; 110 in a sheet) and the value column (the rest).
Gap 10 between columns and between rows. Rows are centred vertically.
Labels: `label`, `text-secondary`. A label that is wider than the label
column is above its control, gap 6, so that it does not break into short
lines (#179).

**Text field.** Height 28, padding 0 8, radius 6, background `bg-field`
(`bg-bar` in a gesture card), border `border-field`, `body` weight 400.
Focus: border `accent-line`.

**"Appearance" section** of a key face and of a bar panel. The rows, in this
order:

| Row | Content | Task |
|---|---|---|
| "Icon" | A tile 28 × 28, radius 6, with the icon in the fill colour; "Choose icon…" (opens section 14); "Image file…" | #190 |
| "Title" | A text field with the label text | #179 |
| "Position" | A segmented control in a track with border `border-field`, segments 22 high: "Top", "Middle", "Bottom", "None" | #189 |
| "Text" | A popup button with the size, for example "12 pt"; a colour button 28 × 28 that shows the text colour; a toggle button "B" 28 × 28 (`body` weight 700) for bold, on = `bg-segment-on` with border `#5A5C63` | #189 |
| "Fill" | The fill samples (section 3.2), 22 × 22, radius 5, gap 6, the row wraps. The selected sample has the look of section 3.7. The first sample is "No fill", as in the wireframe: background `bg-field`, border `border-overlay`. It removes the fill colour. The last sample is "Other colour": dashed border `#5A5C63`, the text "+". It opens a popover with a field for `#RRGGBB`. | #179 |

Until #190, the first row is "Image" with the buttons "Image file…" and
"Remove" ("Remove" is disabled when the surface has no image). #190 changes
it to the "Icon" row. Then the button "No icon" in the icon picker also
removes an image.

#### 10.3 The gesture card

Make the gesture card one component. The key, bar panel, knob, page,
toggle, and sequence editors use it.

**"Gestures" section.** Header "GESTURES" with the hint "What the key does"
(a key), "What the bar panel does" (a bar panel), or "What the knob does" (a
knob). Then one card for each gesture of the control kind, in the order of
`state::gestures`, gap 8.

**A card with an action, closed.** Border 1 `border-field`, radius 9,
background `bg-card`. One head row: padding 8 10, gap 8.

- The gesture name, 64 wide: `label` weight 600, `text-primary`. Examples:
  "Press", "Release", "Tap", "Turn", "Push", "Swipe left".
- The action popup: the rest of the width, height 30, padding 0 8, radius 6,
  background `bg-control`, border `border-control`, gap 8, `control` weight
  500. A kind tile 20 × 20, radius 5, with the kind icon 13 in white (section
  20); the kind name; an up and down chevron 12 at the end.

**A card with an action, open.** The head row, and below it a body: line 1
`line-pane` at the top, padding 10 at the top and the sides, 12 at the
bottom. The body is a form (section 10.2) with the label column 64 and a gap
of 8. Its rows are the fields of the `KindDescription` of the kind, in their
order, with the field labels. A field that
[decision 0038](0038-field-shown-for-the-values-that-use-it.md) hides is not
a row:

| Input type (decision 0014) | Control in the value column |
|---|---|
| `Text` | A text field |
| `LongText` | A text field of three lines, height 60 |
| `Integer` | A text field 56 wide that takes digits only, in the range |
| `Choice` | A popup button with the option labels, height 28 |
| `RuntimeChoice` | The shared choice popup. It asks the handler when the form opens. A missing saved value shows with “not available” and stays in the settings (decision 0014). |
| `RuntimeChoiceList` | The same choice popup, with checks for the ordered selections. A click adds at the end or removes the value. Saved unavailable values can be removed in the menu. |
| `Colour` | The shared fill samples, without “No fill”, and a hex field below them. The field has the shared hex help text. |
| `Toggle` | A switch 30 × 18 |
| `Hotkey` | The current hotkey recorder. It shows the hotkey with the symbols of the short value of section 10.5, for example "⇧⌘A". The profile keeps the text of decision 0014. |
| `Path` | A text field and a button "Choose…" |
| `Action` | A nested card (section 10.5) |
| `ActionList` | A step list (section 10.5) |

A path of kind Application uses the same text field and the button
"Choose…". "Choose…" opens the list of installed applications. The list
shows the name and the icon of each application, and a search field. The
last row is "Choose a different application…". It opens the file dialog.
The field stores the path of the application that the user picks. A typed
name stays as the user typed it. The dialog has the label "Choose an
application".

- The last row of the body: an empty label cell, and at the trailing end the
  button "Test the action" (#188): height 26, padding 0 10, a play icon 11,
  `label` weight 500. Until #188, the row is not there.
- If the handler sends `Status::NeedsUser`, the body shows its text
  (`label`, `warning`) and a button with its button text, above "Test the
  action".

**A card with no action.** A button of full width: height 40, padding 0 10,
gap 8, border 1 dashed `border-overlay`, radius 9, transparent. The gesture
name (64 wide, `label` weight 600, `text-primary`), a plus icon 12, and
"Add an action" (`label` weight 500, `text-secondary`). A click opens the
kind menu.

**The kind menu.** A menu at the popup button or at the empty card. One
section for each group of the library (section 11), with the group name as a
section title, and one item for each kind: the kind tile and the kind name.
A card with an action has at the end a separator and "Remove the action". A
choice sets the kind with its default settings, and opens the card.

**An action with no handler** (rule 8). The card has the head row with the
popup text "Not available" in `text-tertiary`, and the kind tile with a
question mark. Its body shows the kind identifier (`label`, `text-tertiary`)
and the text "This version of OpenCraft does not have this action. The
binding stays in the profile." (`label`, `warning`). The popup menu has
the kinds and "Remove the action".

**Open and close.**

- One card is open at a time in one gesture section.
- A click on the gesture name or on the empty part of the head row opens the
  card, and closes the open card. A click on the open card's head row closes
  it.
- The inspector remembers the open card of each control, while the
  application runs.
- When a control is selected for the first time, the first card with an
  action is open. If no card has an action, no card is open.

The LATER places "Double press" and "Hold" (keys) and "Push and turn"
(knobs) are more cards in this list (section 22, rule 2).

#### 10.4 The knob inspector (#182)

**Header.** The knob picture: a circle 64, background `device-knob`, ring 4
in the ring colour of the knob on the canvas (section 8.2), a mark 6 at the
top centre. The name "Knob *N*". The second line "Feedback on bar panel *N*",
from the feedback surface of the layout. With no feedback surface: "No bar
panel".

**"Gestures" section**, hint "What the knob does". The cards:

1. **"Turn"**, when the two bindings "Turn left" and "Turn right" are one
   pair: the same kind, the kind has a direction description (section 20),
   the direction values agree with one of the two choices below, and all
   other settings are the same. Also when the two bindings are empty. The
   body of the open card:
   - Row "Direction": a segmented control with two segments. The first is
     "Right is *value*", the second is "Left is *value*". *value* is the
     label of the choice option for a turn to the right, in lower case. For
     "Sound volume": "Right is up" and "Left is up". The first segment writes
     the right value to "Turn right" and the left value to "Turn left". The
     second segment writes them the other way.
   - The other fields of the kind, as in section 10.3. A change writes the
     two bindings.
   - The last row: a link "Set left and right apart" (`label`,
     `accent-text`). A click shows the two cards "Turn left" and "Turn right"
     for this knob. The bindings do not change.
   - A kind with no direction description on "Turn" writes the same settings
     to the two bindings.
2. **"Turn left" and "Turn right"**, two cards, when the bindings are not one
   pair, or after "Set left and right apart". If the bindings are one pair,
   the last row of each card is a link "Join left and right". A click shows
   the one "Turn" card again.
3. **"Push"**.

The shell writes two `SetBinding` commands for one change of "Turn". This is
data from the handler (rule 12), not action logic in the shell.

**"Bar panel *N*" section.** The header is "BAR PANEL *N*". The form, label
column 64:

| Row | Content |
|---|---|
| "Face" | The image of the bar panel from the engine, 148 × 94, radius 8, border 1 `line-pane` |
| "Title", "Fill", "Image" | As in "Appearance" (section 10.2) |

Below the form, gap 8: the gesture card "Tap" of this bar panel.

The ring light colour of the knob for each page ("Ring light on this page")
is a LATER place. The setting "Show the level on bar panel *N*" in the
wireframe is a LATER place: the handler shows the feedback now (decision
0013).

#### 10.5 Toggle and sequence (#183)

A toggle and a sequence use the gesture card inside the gesture card. They
can contain each other, as now
([decision 0017](0017-toggle-and-sequence-actions.md)).

**"Appearance for each state" section.** It takes the place of "Appearance"
when the "Press" binding of the key is a kind with two states (section 20).

- Two face buttons next to each other, gap 14, each of the half width:
  padding 10 0, radius 10, gap 8, a column. A face picture 72 × 72,
  radius 11, then the text (`label` weight 500).
  - The first button is "Off", the second is "On". The button of the current
    state of the toggle has the text "Off · now" or "On · now".
  - The selected button (the state that the form below edits): background
    `accent-tint` 14 %, border `accent-line`, `text-primary`. The other:
    transparent, border `border-field`, `text-secondary`.
  - Both pictures are images of the engine (rule 11). If the engine sends
    only the image of the current state, #183 adds the image of the other
    state to the engine events.
- Below, 14 lower: the form.
  - "Off" selected: the rows of "Appearance" (section 10.2).
  - "On" selected: the row "Title", which edits the field "Label for on" of
    the toggle. Below it, the help text "The On face uses the fill and the
    image of the Off face, with a frame." (`help`, `text-tertiary`).
- The second line of the header: "Toggle · the state now is Off" (or "On").

**The toggle in the "Press" card.** The body shows, for each field of input
type `Action`, the field label (`label`, `text-secondary`) and below it,
gap 6, a nested card. #183 changes the labels of the toggle fields in the
description to "From Off to On" (field `on`) and "From On to Off" (field
`off`), so that the shell shows them from data.

**Nested card.** Border 1 `border-field`, radius 8, background `bg-panel`.

- Head row: padding 6 8, gap 8, a line `line-pane` at the bottom when the
  card is open. A kind tile 18 × 18, radius 4, icon 11. The kind name
  (`control` weight 500). At the end a summary (`caption`, `text-tertiary`),
  for example "2 steps".
- Closed: one row of height 32, radius 6, background `bg-control`, border
  `border-control`, with the tile, the kind name, and at the end a short
  value (`label`, `text-secondary`), for example "⌘⇧A". A click opens it.
- Open: the fields of the kind, as in section 10.3.
- An empty nested card is a dashed card "Add an action" (section 10.3), of
  height 32.

**Step list** (input type `ActionList` or `TimedActionList`).

For `TimedActionList` (#282), the open step starts with the shared integer
field "Delay before, in milliseconds". The rules for its data and defaults
are in the [amendment of decision 0017](0017-toggle-and-sequence-actions.md#amendment-a-delay-before-each-step-282).

- One row for each step: height 34, padding 0 8, gap 8, a line
  `line-section` at the bottom. A drag handle (six dots, 12 × 14,
  `text-disabled`, accessible label "Drag to reorder"), the step number (16
  wide, `caption`, `text-tertiary`), the kind tile 18, the kind name
  (`control`), and at the end the short value (`label`, `text-secondary`).
- A click on a step opens it as a nested card in its place.
- A right-click, or a "⋯" button at the end of the open step, shows the menu
  "Move Up", "Move Down", separator, "Remove".
- A drag of the handle moves the step.
- The last row: padding 6 8, the button "Add a step": height 24, padding 0 8,
  radius 6, dashed border `border-overlay`, a plus icon 11, `label`
  weight 500, `text-secondary`. A click opens the kind menu.
- Below the step list, the field "Delay between two actions, in
  milliseconds" stays, as a normal row. It is the default for inherited
  delays in an old sequence.
- The short value of a step: the value of its first field, as text. For a
  hotkey, the keys with symbols, for example "⇧⌘A" (the order of macOS: ⌃ ⌥ ⇧ ⌘, and then the
  key). For a choice, the
  option label. For a nested toggle or sequence: "2 steps" or "Toggle".

A "Wait" step is a LATER place: one more kind in the kind menu (section 22,
rule 1).

#### 10.6 The page inspector (#181, #191)

**Header.** A page picture 72 × 44, radius 6, background `device-display`,
border 1 `line-pane`: the images of the canvas for this page, scaled down
(rule 11). The name "Page “*name*”". The second line "Page *N* of *M* ·
Profile “*name*”".

**"Page" section.** Label column 72.

| Row | Content |
|---|---|
| "Name" | A text field. Return or a click outside renames the page. |
| "Order" | Two square buttons 28 × 28, "Move the page up" (chevron up) and "Move the page down" (chevron down), disabled at the first or the last page |
| (no label) | The buttons "Duplicate" and "Remove…" (`destructive-text`). "Remove…" asks for a confirmation, as in the sidebar. |

The text "or drag it in the sidebar" of the wireframe is a LATER place: it
comes with the drag of a page row.

**"Background" section** (#191).

- A segmented control, 12 above the next row: "None", "Colour", "Image",
  and "Video". "Video" is disabled: it is the one LATER place that the
  window shows (issue #191).
- "Image": a preview 140 × 84, radius 8, border `border-field`, with the
  image, and the text "800 × 480" (`badge` weight 400, `text-secondary`) at
  the bottom left, 6 from the edges. Next to it, gap 12, a column of two
  controls, gap 6: the button "Image file…" and a popup button for the fit
  ("Fill" or "Fit"). The default is "Fill":
  [decision 0032](0032-page-background.md).
- "Colour": the fill samples of section 10.2, with no "No fill" sample:
  "None" removes the background.
- "None": no other control.
- The help text below, 10 lower: "The background shows around and behind
  each key face and bar panel." (`help`, `text-tertiary`).
- A model with no background has no "Background" section.

**"Touch bar gestures" section.** Header "TOUCH BAR GESTURES" with the hint
"On this page". The gesture cards "Swipe left" and "Swipe right" (section
10.3), with the gesture name column 84 wide. A swipe has no position, thus
it belongs to the page. When the "◀ Touch bar ▶" control selected the page,
the inspector scrolls so that this section is visible.

LATER places: "Use the same swipes on all pages of this profile", "One
image across the touch bar", "Knob light colours for this page".

#### 10.7 The device inspector (#180)

**Header.** Padding 16 16 12. A tile 48 × 48, radius 11, background
`bg-tile`, a device icon 26. The model name (`heading`). The state
(`label`, `text-secondary`, a dot of 6): "Connected by USB" (`success` dot)
or "Not connected" (`text-disabled` dot).

The "Device" section shows the firmware version from the engine. It shows
"Not available" when no device is connected or the read failed. The model
name stays in the header. No serial number is shown.

The sections show only the settings that the model has
([decision 0022](0022-device-settings-and-stop-state.md#the-descriptor)).

**"Display" section.** Gap 12.

- A line: "Brightness" (`control`, `text-primary`) and at the end the value,
  for example "80 %" (`text-secondary`).
- A slider, gap 8 below: height 16, track 4 high, radius 2, `border-field`;
  the filled part `accent`; a white thumb of 16. Range 15 to 100. The window
  sends the commands as now (`SETTINGS_INTERVAL`).

Note (2026-11-13), from #262: "Screen off after" is a popup button below
brightness. Never is the default. Its choices and engine policy are in
[decision 0034](0034-device-dark-state.md). It uses the shared popup controls
and the token `WIDTH_SCREEN_OFF_MENU`.

**"Feedback" section.** Gap 10.

- A line: "Haptic feedback" and at the end a switch 30 × 18 (on: `accent`;
  off: `bg-segment-on`; a white knob of 14).
- The help text "The device vibrates a little when you operate a control."
  (`help`, `text-tertiary`).

**"Knob lights" section.** Gap 12.

- A segmented control (segments 22 high): "Effect", "Colours", "Off".
- In "Colours": a grid of 4 columns, gap 8. One button for each knob light:
  padding 8 0, radius 8, background `bg-card`, border `border-field`, a
  column with gap 6: a ring 26 (border 4 in the colour, inside
  `device-knob`), "Knob *N*" (`caption` weight 500), and the hex value
  (`badge` weight 400, `text-tertiary`), for example "#14B8A6". Accessible
  label "Knob 1 light colour #14B8A6". A click opens a popover with the 9
  knob light samples (section 3.2) and a field for `#RRGGBB`.
- "Light brightness" with its value, and a slider as above, range 1 to 100.
  In the mode "Off" the slider is disabled, and the value is kept.

**Note at the bottom.** After a flexible space, a line `line-section` at the
top, padding 12 16 16: "These settings are for this device. A change of the
profile or the page does not change them. You can change them when no
device is connected." (`help`, `text-tertiary`, line height 1.45).

"Release the device to other software" is a LATER place below the note.

### 11. The library (#184)

**Header** (below the tab header). Padding 12 16, gap 10, line
`line-section` at the bottom.

- The filter chips (height 24, radius 12, padding 0 10, `label` weight 500,
  gap 4) are a LATER place: "Icons", "Backgrounds", and "Starter pages" come
  later. While "Actions" is the only filter, the window does not draw the
  chip row.
- A search field: height 30, padding 0 8, radius 7, background `bg-field`,
  border `border-field`, gap 8, a search icon 14 in `text-tertiary`. The
  placeholder "Search *N* actions", with *N* the number of kinds in the
  library (now "Search 11 actions"). Focus: border `accent-line`. Accessible
  label "Search actions".

**List.** Padding 6 10 12. It scrolls.

- One group for each group name in `KindDescription` (section 20). The
  groups are in the order of their first kind in `Event::Kinds`. The kinds
  are in the order of that event. The shell has no list of kinds (rule 8).
- Group header: padding 8 at the top of the group, then 4 6, the name in
  `section`, `text-tertiary`.
- Row: a button of full width, height 34, padding 0 8, radius 7, gap 10,
  `body` weight 500, transparent, border 1 transparent. A kind tile 24 × 24,
  radius 6, with the kind icon 14 in white; the kind name; at the end the
  help text (`caption`, `text-tertiary`). The pointer is an open hand.
- A row in a drag: background `accent-tint` 18 %, border 1 dashed
  `accent-line`, text `text-secondary`.
- A kind with no handler is not in the library.

The built-in kinds, as the engine must send them:

| Group | Kinds, in order | Help text |
|---|---|---|
| "System" | Open an application, Open a URL, Run a shell command, Send a hotkey, Type text | "Accessibility" on Send a hotkey and Type text |
| "Media" | Media key, Sound volume | "Accessibility" on Media key; "Knob" on Sound volume |
| "Navigation" | Change the page, Select a profile | — |
| "Logic" | Toggle, Sequence of actions | — |

The LATER places "Open a folder" (Navigation), "Wait" (Logic), and a group
for each plug-in come as data (section 22, rules 1 and 4).

Note (2026-11-14): decision [0036](0036-folders.md) uses the place "Open a
folder". The Navigation group also has "Back". "Wait" stays a later place.

**Search.** The text filters the kinds of all groups: a kind shows if its
name contains the text, with no difference between capital and small
letters. A group with no kind does not show. With no result: "No action
matches “*text*”." (`label`, `text-tertiary`, centred).

**Click to assign.** A click on a row sets the kind, with its default
settings, on the open gesture card of the selected control. If no card is
open, it uses the first gesture of the control (section 12). The status bar
shows the message of section 6.1. With no selected control (the page or the
device is the selection), the click does nothing and the status bar says
"Select a control first". The right panel stays on "Library".

### 12. Drag and drop (#185)

- Use the method that #175 found
  ([kb/gpui.md](../kb/gpui.md)).
- **Drag picture.** A chip at the pointer: padding 7 12 7 8, gap 8, radius 8,
  background `bg-field`, border 1 `bg-segment-on`, the shadow of section
  3.2, `control` weight 500. The kind tile 22 and the kind name. The
  wireframe turns it by −3°; do this only if GPUI can do it.
- **Drop target.** A key face, a bar panel, or a knob under the pointer:
  background `accent-tint` 28 % on top of the image, a dashed border 2 in
  `accent-line`, and the text "Drop here" (`caption` weight 600, white) in
  the centre. Only the control under the pointer shows this.
- **The drop** sets the kind, with its default settings, on the first
  gesture of the control: "Press" for a key, "Tap" for a bar panel, "Turn"
  for a knob (section 10.4; before #182, "Turn right"). The drop selects the
  control and opens that card. The right panel stays on its tab.
- A drop on a control with an action replaces the action. With #186 closed,
  ⌘Z restores it.
- A drop outside a control does nothing.
- The help line shows the drag text (section 8.4). The status bar shows the
  message of section 6.1.

### 13. Menus, key commands, undo, and the clipboard

**Application menus.** The window has these menus. Each task adds its items.

| Menu | Items | Task |
|---|---|---|
| "OpenCraft" | "About OpenCraft" (opens Settings at "About"), separator, "Settings…" ⌘, , separator, "Quit OpenCraft" ⌘Q | #193 adds "About" and "Settings…" |
| "Edit" | "Undo *name*" ⌘Z, "Redo *name*" ⇧⌘Z, separator, "Copy" ⌘C, "Paste" ⌘V, "Clear" (Delete), separator, "Search…" ⌘K | #186, #187, #192 |
| "View" | "Hide Sidebar" ⌃⌘S, "Hide Inspector" ⌥⌘I | #176 |

**Focus rule.** When a text field has the keyboard focus, ⌘Z, ⇧⌘Z, ⌘C, ⌘V,
and Delete go to the text field. Else they go to the profile and the
selected control.

**Undo and redo** (#186). The decision of #186 says where the history is.
These parts are fixed:

- The names of the changes, in the form of the Edit menu: "Set Title", "Set
  Fill", "Set Image", "Set Action", "Remove the Action", "Add Page", "Rename
  Page", "Move Page", "Remove Page", "Duplicate Page", "Paste", "Clear",
  "Copy to All Pages", "Drop Action". #186 can add more.
- One change to a text field is one step: from the first character to the
  Return, or to the moment when the field loses the focus.
- With an empty list, the menu item is "Undo" (or "Redo") and disabled.

**Clipboard** (#187). "Copy" copies the content of the selected control:
its appearance and its bindings. "Paste" is possible only to a control of
the same kind; else it is disabled. "Clear" removes the appearance and the
bindings of the control. "Copy to All Pages" puts the content on the same
control on each page of the profile.

The content of a knob is its bindings and its bar panel: the content of
its feedback surface and the bindings of the bar panel control of that
surface, because the knob inspector shows that bar panel (section 10.4).
The engine keeps the clipboard (rule 12), and a copy stays when the
profile changes: a paste copies the image into the active profile. The
clipboard is not the clipboard of macOS (#187).

### 14. The icon picker (#190)

A popover attached to the button "Choose icon…", with an arrow of 16 that
points to the button.

- Size 360 × 550. Radius 12, background `bg-field`, border 1
  `border-overlay`, the popover shadow. Accessible label "Choose an icon".
- **Header.** Padding 12 12 10, gap 10, line `border-field` at the bottom.
  - The tab chips (section 3.7): "Icons" and "Recent". "Packs" and "GIF"
    are LATER places in this row.
  - A search field: height 30, radius 7, background `bg-bar`, border
    `border-field` (focus `accent-line`), placeholder "Search icons", and at
    the end the number of icons, for example "1,818 icons" (`caption`,
    `text-tertiary`). The field has the focus when the popover opens.
- **Grid.** Padding 12, 7 columns, rows 40 high, gap 6. It scrolls. A cell:
  radius 8, background `bg-bar`, border 1 `line-pane`, the icon 20 in
  `text-icon-cell`. The icon of the surface: background `accent`, border
  `accent-line`, white. Each cell has the icon name as accessible label.
- **"Recent".** The last 21 icons that the user chose, newest first. The
  shell keeps them in `shell.toml` (`recent_icons`).
- **Footer.** Padding 10 12, line `border-field` at the top, gap 10.
  - A row: "Icon colour" (64 wide, `label`, `text-secondary`), then 6
    circles of 20, gap 8: `#FFFFFF`, `#111214` (border `#5A5C63`),
    `#F59E0B`, `#22C55E`, `#FF6B6B`, `#60A5FA`. The selected circle has the
    look of section 3.7.
  - A row: "Image file…" (background `#34363C`, border `bg-segment-on`) at
    the start, and "No icon" (transparent, border `bg-segment-on`) at the
    end.
- A click on an icon or a colour applies it at once (one undo step). The
  popover stays open. Escape or a click outside closes it.

### 15. The command palette (#192)

A modal layer in the centre of the window, 120 below the top of the window.
Behind it, the window gets a layer of black at 25 %. A click on that layer
closes the palette.

- Size 680 × 460. Radius 14, background `bg-field`, border 1
  `border-overlay`, the sheet shadow. Accessible label "Search actions,
  controls, pages and commands".
- **Search row.** Height 56, padding 0 18, gap 12, line `border-field` at the
  bottom. A search icon 18 in `text-secondary`; the field (`sheet-title`
  weight 400, no border), with the focus; at the end the context chip, for
  example "Key 8 selected" (`help`, `text-secondary`, padding 3 8, radius 6,
  background `#34363C`). With no selected control the chip is not there.
- **Results.** Padding 6 8. It scrolls. Groups in this order: "Actions"
  (from the library), "Controls" (the controls of the active page, by name
  and by label), "Pages", "Profiles", "Commands".
  - Group header: padding 8 10 4, `section`, `text-tertiary`.
  - Row: height 36, padding 0 10, radius 8, gap 10. A tile 24, radius 6,
    icon 14 (the kind tile for an action, `bg-tile` for the others); the name
    (`body`); at the end a hint (`label`, `text-secondary`).
  - Hints: an action "Action"; a control its label or "Empty"; a page
    "*profile* · page *N*"; a profile "On device" for the active profile; a
    command its key command, if it has one.
  - The selected row: background `accent`, text white, the tile background
    white at 18 %.
  - A row shows if its name contains the text (no difference between capital
    and small letters). Each group shows 5 rows at the most. With an empty
    field, each group shows its first 5 rows. With no result: "No result for
    “*text*”."
- **Commands** in the group: "Add a page", "Page settings", "Device
  settings", "Add a profile", "Settings…", "Hide Sidebar" or "Show Sidebar",
  "Hide Inspector" or "Show Inspector". A later task adds its commands here.
  Issue #322 adds "Import a profile…" (section 7.3).
- **Keys.** Up and Down move the selected row, across the groups. Return does
  the main operation: assign the action to the open gesture card of the
  selected control (section 11), select the control, select the page, select
  the profile, or do the command. ⌘↵ selects the result and shows it in the
  Inspector tab. Escape closes the palette.
- **Footer.** Height 36, padding 0 18, gap 16, line `border-field` at the
  top, `help`, `text-secondary`; each key in `text-primary` weight 600. "↵
  *main operation*", for example "↵ Assign to Key 8", "↵ Select", "↵ Do the
  command"; "⌘↵ Open in the inspector"; at the end "esc Close".

### 16. The profile settings sheet (#195)

"Profile Settings…" in the right-click menu of a profile opens a sheet on
the main window.

- Width 600, height from the content. Radius 14, background `bg-card`,
  border 1 `border-overlay`, the sheet shadow. Accessible label "Profile
  settings for *name*".
- **Title.** Padding 20 24 14. "Profile settings" (`sheet-title`), and below
  it, gap 4, "Profile “*name*” · *N* pages" (`label`, `text-secondary`).
- **Form.** Padding 0 24, label column 110, gap 12 between rows and 14
  between columns, labels `label` (12.5) in `text-secondary`.

| Row | Content |
|---|---|
| "Name" | A text field: height 30, padding 0 10, radius 7, background `bg-bar` |
| "Colour" | 7 circles of 20, gap 8: "None" (transparent, border `#5A5C63`), `#3B82F6`, `#14B8A6`, `#22C55E`, `#F59E0B`, `#EF4444`, `#A855F7`. The selected circle has the look of section 3.7. The colour shows on the profile icon in the sidebar. |
| "First page" | A popup button 200 × 30 with the page names. The device shows this page when the profile becomes active. |

- **"Share" section.** 16 below the form, padding 0 24, gap 8. Header
  "SHARE" (`section`). "Export as a file…" (height 28, padding 0 12).
  Import is in the sidebar's profile add menu (section 7.3), not in this
  sheet: it adds a new profile and changes no setting of this profile
  (decision 0031). This placement amends the original sheet for issue #322.
- **Footer.** After a flexible space: padding 14 24, line `line-pane` at the
  top, background `bg-sidebar`. At the start "Remove the profile…" (height
  30, padding 0 12, radius 7, transparent, border `border-control`,
  `destructive-text`). At the end the main button "Done" (height 30, padding
  0 18).
- Each change applies at once, as in the inspector. "Done", Return, and
  Escape close the sheet. "Remove the profile…" asks for a confirmation.
- "Switch to this profile automatically" is a LATER place between the form
  and "Share". "Import from VSD Craft" is a LATER place in the sidebar's
  profile add menu, because it also adds a profile.
- The profile colour and the first page are new data in the profile. #195
  adds them (decision 0009 is the base).

### 17. The Settings window (#193)

A standard macOS settings window
([kb/ui-research.md](../kb/ui-research.md#apple-human-interface-guidelines)).

- It opens from "Settings…" (⌘,) in the application menu, from the sidebar
  row "Settings", and from the menu bar item. If it is open, it comes to the
  front.
- Width 640. Its height follows the content of the pane. The user cannot
  change its size. The minimize and zoom buttons are disabled.
- **Title area.** Background `bg-sidebar`, line `line-section` at the
  bottom, padding 0 16 8. A title bar of 34 with the window buttons and the
  name of the pane in the centre (`body` weight 600), for example
  "General".
- **Tab row.** Centred, gap 4. One button for each pane: 78 × 50, radius 7,
  a column, gap 3: an icon 20 and the name (`caption` weight 500). Selected:
  background `bg-segment-on`, white. Others: transparent,
  `text-secondary`. The tabs: "General" (gear), "Shortcuts" (keyboard),
  "About" (information). "Plug-ins" and "Updates" are LATER places after
  "Shortcuts". The window opens at the pane that was visible last
  (`settings_pane` in `shell.toml`).
- **Content.** Padding 18 40, gap 14. Background `bg-bar`.
- **Group box.** Radius 10, background `bg-card`, border 1 `line-pane`. Rows:
  padding 10 14, gap 12, a line `line-pane` between two rows. A row has two
  lines on the leading side (the name in `body`, a description in `help`,
  `text-tertiary`) and a control at the end.

**"General" pane.**

1. A group box with three switches (32 × 19, knob 15):

| Name | Description | Value |
|---|---|---|
| "Start at login" | "The device operates after you log in, with no window" | [Decision 0021](0021-start-at-login.md) |
| "Show the Dock icon" | "Off: OpenCraft is only in the menu bar" | `show_dock_icon` ([decision 0020](0020-menu-bar-item-and-dock-icon.md)) |
| "Show the window at start" | "Off: the window opens from the menu bar item" | New: `show_window_at_start` in `shell.toml`, default `true`. A start at login never opens the window (decision 0021). |

2. A group box for the permission: padding 12 14, gap 12. A shield icon 20
   (`warning` when not given, `success` when given). The text "Accessibility
   permission: **not given**" (the last words in `warning` weight 600) or
   "Accessibility permission: **given**" (`success`). Below it: "Necessary
   for *kind names*", with the names of the kinds that need the permission
   (section 20), joined with ", " and " and ". Example: "Necessary for Send
   a hotkey, Type text and Media key". At the end, the button "Open System
   Settings", which opens the Accessibility pane of System Settings.
3. A group box with two rows: "Profiles folder" with the folder path (the
   home folder as "~") and the button "Show in Finder"; "Log file" with
   "Attach it when you report a fault" and the button "Show in Finder"
   (height 26).

"Change the profile when the front app changes" is a LATER place below the
last group box.

**"Shortcuts" pane.** One group box with one row for each key command of the
window: the command name at the start (`body`), the keys at the end (`body`,
`text-secondary`). The list: Search ⌘K, Undo ⌘Z, Redo ⇧⌘Z, Copy ⌘C, Paste
⌘V, Clear Delete, Hide or show the sidebar ⌃⌘S, Hide or show the inspector
⌥⌘I, Settings ⌘, , Quit OpenCraft ⌘Q. The list shows only commands that
exist. The user cannot change them.

**"About" pane.** Centred: the application icon 64, "OpenCraft"
(`sheet-title`), the version (`label`, `text-secondary`), and the button
"Licence notices", which opens the notices file
([decision 0026](0026-licence-notices.md)). Below it, "Copy diagnostics"
copies the local support report. Its contents and privacy filter are in
[decision 0024](0024-log-file-with-no-terminal.md). Below the buttons, the
links "User guide" and "Report a fault" open fixed GitHub pages in the
user\'s default browser. OpenCraft fetches nothing for this pane.

The options that move here leave the menu bar item (section 19).

### 18. The first start (#196)

At the first start (no profile file), a window shows before the main window.

- Size 800 × 560. The user cannot change its size. Background `bg-bar`. A
  title bar of 40 with the window buttons only.
- **Left column.** 300 wide, a line `line-section` at the trailing side,
  content centred, gap 18.
  - A small device picture 220 × 170: radius 16, `device-body`, border 1
    `#2E3036`, padding 12, gap 8. A grid of 10 key tiles (5 columns, gap 6,
    height 28, radius 5), a row of 4 bar panel tiles (height 22, radius 4,
    `#1A1F2A`), and a row of 4 knob rings (22, ring 3). The tiles have the
    fill colours of the selected starter profile.
  - With a device: a check badge at the top right of the picture, 10
    outside it: a circle 28, `success-fill`, border 3 `bg-bar`, a white
    check 14. Below the picture, gap 4: "*model name* found" (14, weight
    600), for example "VSD N4 Pro found", and "Connected by USB" (`label`,
    `text-secondary`).
  - With no device: no badge, the picture at 50 % opacity, "Connect your
    device" (14, weight 600), and "OpenCraft finds it when you connect it.
    You can also continue now." (`label`, `text-secondary`, centred).
- **Right column.** Padding 28 32 22.
  - The title (`display`): "Your device is ready", or "Welcome to OpenCraft"
    with no device. 6 below it: "Choose a profile to start with. You can
    change each key later." (`body`, `text-secondary`, line height 1.5), 20
    above the cards.
  - A grid of 2 × 2 starter profile cards, gap 12. A card is a button:
    padding 12, radius 10, a column, gap 6, background `bg-card`, border
    `border-field`. Selected: background `accent-tint` 16 %, border
    `accent-line`. In the card: a key picture (5 columns of cells 14 high,
    radius 3, gap 3, with the fill colours of the profile), the name (`body`
    weight 600), and the description (`help`, `text-secondary`).

| Card | Description |
|---|---|
| "Everyday Mac" (selected first) | "Apps, web pages, sleep, page change" |
| "Media and sound" | "Play, next track, volume on the knobs" |
| "Video calls" | "Mute toggle, camera, leave the call" |
| "Empty" | "One empty page" |

  - A flexible space, then a box 16 above the buttons: padding 10 12,
    radius 9, background `bg-card`, border `line-pane`, gap 10, a shield
    icon 16 in `text-secondary`, and "Three actions need the Accessibility
    permission: Send a hotkey, Type text and Media key. OpenCraft asks for
    it when you add the first one." (`label`, `text-secondary`, line height
    1.45). The kind names come from data (section 20).
  - The main button "Start" at the end of the last row (height 30, padding
    0 16). Return does "Start".
- "Start" makes the selected profile, closes this window, and opens the
  main window. The starter profiles are profile files in the bundle.
- The wireframe shows two step dots and a button "Back". The first start
  has one step, thus the window has no dots and no "Back".

**The Accessibility request.** The first start does not ask for the
permission. When the user sets the first binding with a kind that needs it,
and the permission is not given, a sheet opens on the main window:

- The title "OpenCraft needs the Accessibility permission" (`sheet-title`).
- The text "“*kind name*” sends keys to other apps. macOS lets OpenCraft do
  this only with the Accessibility permission. The binding stays in the
  profile. The action operates after you give the permission."
- The buttons "Not now" and the main button "Open System Settings".
- The sheet shows one time in each run of the application.

Note (2026-11-13), from the work of #196. The code does this:

- The pictures of the keys and the bar panels are the images of the engine
  (rule 11), made with `opencraft_engine::page_preview` from the first page
  of each starter profile. A surface with no content is a tile in
  `bg-tile` (a bar panel: `#1A1F2A`). The key tiles of the device picture
  are square (34 × 34), because they show the square image of a key face;
  the bar panel tiles are 28 high. The cells of a card follow the rule
  below.
- **The cells of a card (#251).** A cell of a card shows the fill colour of
  its key and nothing else: no image, no icon, and no title. The cell is
  14 high with radius 3, as the wireframe showed. The cause is that a key
  image of the engine is square and has an icon and a title (#239). A band of
  14 high from the middle of that image cuts the icon. The colour comes from
  the profile through the engine: the window reads the `color` of the key
  face on the first page of the starter profile, and converts it with
  `theme::colour`. The window has no second list of colours. A key with no
  fill, or with no content, shows `bg-tile`, the colour of the cells of the
  card "Empty". The device picture at the left stays as it is: it shows the
  full key images of the engine for the selected card.
- The box about the permission shows only when the permission is not given.
- "Start" with a starter profile that has an action that needs the
  permission (for example "Media and sound") opens the main window with the
  Accessibility request, if the permission is not given. The first start
  window does not ask.
- The request also opens for an action in a toggle or in a sequence. It
  opens when the engine sends back a binding that the window sent, thus the
  bindings of a page change do not open it. The sheet has a shield icon at
  the start of the title.
- The card "Empty" makes the profile "Default" with one empty page.
- The arrow keys move the selection of the cards, and a double-click on a
  card starts. If the user closes the window with no "Start", the next
  "Open OpenCraft" shows it again.
- The starter profiles are in the executable (decision 0025). The video
  call keys send the key commands of Zoom, Google Meet, and Microsoft Teams,
  one page for each.

Note (2026-11-14), from the work of #340. The window size stays 800 × 560.
The right column has the button "Ready profiles…" on the same row as
"Start". The button opens a list above it. Each row of the list has a
switch. A switch that is on adds that ready profile when the user clicks
"Start". The switches are off at the start, so "Start" adds only the
selected starter profile. The engine adds the ready profiles first, then
the starter profile, so the starter profile is the active profile. The
same ready profiles are in the submenu "Ready Profiles" of the profile
add menu. A ready profile keeps a link only when no profile already has
it (decision 0035, section 5).

### 19. The menu bar item (#194)

The native menu of [decision 0020](0020-menu-bar-item-and-dock-icon.md)
stays. The items, in this order:

| Item | Kind | Operation |
|---|---|---|
| "*model name* · connected" or "*model name* · not connected", with a dot image (`success` or `text-disabled`) | Text, not enabled | The connection state |
| Separator | | |
| "Profile: *name*" | Submenu with one item for each profile, a check mark on the active profile. *name* is the active profile. | Selects the profile |
| "Page: *name*" | Submenu with one item for each page of the active profile, a check mark on the active page | Selects the page |
| Separator | | |
| The brightness row | One custom view: "Brightness" and the value ("80 %") on one line; below it a small sun icon, a slider, and a large sun icon. Width 270, padding 4 10 8. | Sets the display brightness, as the device inspector does |
| Separator | | |
| "Accessibility Permission Not Given…" (only when missing; added by #258) | Command | Opens Settings at "General"; the state comes from the action kinds |
| "Open OpenCraft" | Command, ⌘O | As now |
| "Settings…" | Command, ⌘, | Opens the Settings window |
| Separator | | |
| "About and licence notices" | Command | Opens the Settings window at "About" |
| Separator | | |
| "Quit OpenCraft" | Command, ⌘Q | The quit path of decision 0020 |

- The items "OpenCraft *version*", "Show the Dock icon", "Start at login",
  and "Licence notices" leave the menu. The version and the notices move to
  the "About" pane, and the two options to the "General" pane (section 17).
- The custom view is the only custom view in the menu. It does not make a
  timer.
- "Release the device" is a LATER place after "Settings…".

Note (2026-11-13), from the work of #194. The tests showed these facts
([kb/gpui.md](../kb/gpui.md#the-menu-with-the-brightness-row-2026-11-13)),
and the code does this:

- `NSMenuItem.image` shows nothing in this menu. The dot of the first item is
  a coloured character in the title (green for "connected", the system
  tertiary label colour for "not connected"), at the left of the text, in the
  column of the item titles. It is not in the column of the check marks.
- The item "Profile: *name*" is a submenu, in the same way as the pages.
  *name* is the active profile. The items of the profiles have no icon.
- The custom view follows the width of the menu. Its text is 14 points from
  the edge, in the column of the check marks, and the right edge has the same
  space. The slider has the lowest value of the device model, not 0.
- The menu has no "Page" submenu and no "Profile" submenu before the engine
  sends the pages and the profiles. It has no brightness row before the engine
  sends the device settings.
- A selection of the active profile sends no command.
- The slider sends at most one command in each 50 ms, and the last value
  always goes to the engine. A change from the window moves the slider when
  the menu opens again. The engine saves the value.
- "Open OpenCraft" has the key equivalent ⌘O, and works while the menu is
  open. The window does not bind ⌘O.

Note (2026-11-14), from issue
[#355](https://github.com/shantanugoel/OpenCraft/issues/355). The profiles
are one submenu. Before this, each profile was an item under the section
title "Profile".

Note (2026-11-14), from issue
[#371](https://github.com/shantanugoel/OpenCraft/issues/371). A choice of
the page that is shown sends the page command. The engine closes the open
folders when that page is the page under the folders
([decision 0036](0036-folders.md#4-the-pages-and-the-folders)). A choice of
a different page operates as before.

### 20. Data that the shell needs from the engine

The shell has no device logic and no action logic (rule 12), and no list of
kinds (rule 8). Thus some tasks add data to the engine messages. Each task
updates [decision 0014](0014-action-settings-description.md) for a change to
`KindDescription`.

| Data | Where | Task | Use |
|---|---|---|---|
| The group of a kind, as text (for example "System") | `KindDescription` | #184 | Library groups, kind menu |
| A short help text of a kind (optional) | `KindDescription` | #184 | Library row hint |
| The kind needs the Accessibility permission (yes or no) | `KindDescription` | #184 | The permission text in section 17 and section 18, the request of section 18 |
| The direction description: the choice field, the value for a turn to the right, the value for a turn to the left | `KindDescription` | #182 | The "Turn" card |
| The icon of a kind: a name in the icon set of #190 | `KindDescription` | #190 | Kind tiles. Until #190, the tile shows the first letter of the kind name in white. |
| The kind has two states, the field of the label for on, and the image of each state | `KindDescription` and the surface events | #183 | Section 10.5 |
| The profile colour and the first page | Profile and `Event::Profiles` | #195 | Sections 7.3 and 16 |
| The page background | Profile and `Event::Background` ([decision 0032](0032-page-background.md)) | #191 | Sections 8.2 and 10.6 |

**Kind tiles.** The tile colour comes from the group, so the shell needs no
colour for each kind. The wireframes use a colour for each kind; this
record uses one colour for each group:

| Group | Tile colour |
|---|---|
| "System" | `#3A3F47` |
| "Media" | `#2350A8` |
| "Navigation" | `#1F6F6B` |
| "Logic" | `#7A3E2E` |
| Any other group | `#4B3B7A` |

**Kind icons** for the built-in kinds (Lucide names): Open an application
"app-window", Open a URL "globe", Run a shell command "terminal", Send a
hotkey "keyboard", Type text "type", Media key "play", Sound volume
"volume-2", Change the page "arrow-right", Select a profile "layers",
Toggle "toggle-left", Sequence of actions "list".

The shell must not compare a kind identifier with a built-in identifier, for
example `opencraft.toggle`. If a task needs to know a property of a kind, it
adds the property to the description.

### 21. Places kept for later

The application does not draw these places (section 1), except "Video" in
the page background. A later feature uses its place and needs no new
screen.

| Area | Place | Where it goes |
|---|---|---|
| Sidebar | More devices; a virtual deck | Items of the device switcher (section 7.2) |
| Sidebar | A mark for a profile that changes with the front application (for example "OBS") | At the end of a profile row, in place of "ON DEVICE" for a profile that is not active |
| Sidebar | "Plug-ins"; a store | A row in the bottom group, above "Settings" |
| Sidebar | Drag a page row to change the order | The page rows |
| Toolbar | "Share a profile"; "Update available" | Between the live state pill and the search field |
| Canvas | The folder path, for example "Main › OBS" | The page header, after the page name. Decision [0036](0036-folders.md) uses this place: the path replaces the page name while a folder is open. |
| Canvas | Animated faces (GIF, video); one image across the touch bar; other device models | The device drawing, from the layout and the engine images |
| Canvas | Select many controls | The selection of section 8.3 |
| Inspector | "Double press" and "Hold" on a key; "Push and turn" on a knob | Gesture cards (section 10.3) |
| Inspector | A state from an application on a key face | The "Appearance" section |
| Inspector | "Ring light on this page" (knob); "Knob light colours for this page" (page) | A section at the end of the knob and the page inspector |
| Inspector | "Show the level on bar panel *N*" | A row in the "Turn" card |
| Inspector | The settings forms of a plug-in | Gesture cards, from the settings description (section 22, rule 4) |
| Inspector | "Use the same swipes on all pages of this profile"; "One image across the touch bar"; "Video" background | The page inspector. "Video" shows as a disabled segment now. |
| Inspector | "Release the device to other software" | The device inspector |
| Inspector | A "Wait" step | The kind menu of a step list |
| Library | Filters "Icons", "Backgrounds", "Starter pages"; icon packs; video backgrounds | The chip row (section 11) |
| Library | "Open a folder", "Wait"; a group for each plug-in, with "Browse plug-ins…" | Rows and groups (section 11). Decision [0036](0036-folders.md) uses "Open a folder" and adds "Back". "Wait" and the plug-in group stay later places. |
| Status bar | Faults of a plug-in | A message of section 6.1 |
| Icon picker | "Packs", "GIF" | Tab chips (section 14) |
| Settings window | "Plug-ins", "Updates"; "Change the profile when the front app changes" | Tabs; a group box of "General" (section 17) |
| Profile sheet | "Switch to this profile automatically", with a list of applications and "Go back to the last profile when the app is not in front" | Section 16 |
| Sidebar profile add menu | "Import from VSD Craft" | Section 7.3 |
| Menu bar item | "Release the device" | Section 19 |
| Window | The light appearance | A second set of values for the colour tokens of section 3.2. The layout does not change. |

### 22. Rules for a new feature

1. A new action kind is one row in the library and one form in the
   inspector. The form comes from the settings description of the action
   ([decision 0014](0014-action-settings-description.md)).
2. A new gesture is one more card in "Gestures".
3. A new device model needs no new screen. The canvas draws it from the
   descriptor, and the device switcher lists it.
4. A plug-in gets a group in the library, a tab in Settings, and native
   forms in the inspector. No web view in the main window, so that the
   footprint stays small.
5. Do not add a top-level screen for one feature.

### 23. No web view in the main window

The main window, the Settings window, the first start window, and the menu
bar item contain no web view. All forms are native GPUI elements that the
shell makes from data (rule 8, decision 0014). A plug-in describes its
settings as fields; it does not send HTML.

The reason: the web view is the largest part of the footprint of the vendor
software ([kb/vsd-craft.md](../kb/vsd-craft.md)), and "Small" is quality
goal 2. This affects the deferred decision about Stream Deck plug-ins, whose
settings panels are HTML pages
([kb/prior-art.md](../kb/prior-art.md#stream-deck-plug-in-model)). A record
for that decision must show the cost of a web view, and it must not put the
web view in the main window.

### 24. Tasks and their sections

| Task | Sections |
|---|---|
| [#175](https://github.com/shantanugoel/OpenCraft/issues/175) GPUI test | 4, 9, 12, 14, 15 (what GPUI must do) |
| [#176](https://github.com/shantanugoel/OpenCraft/issues/176) Three panes | 3, 4, 5, 6, 9 |
| [#177](https://github.com/shantanugoel/OpenCraft/issues/177) Sidebar | 7 |
| [#178](https://github.com/shantanugoel/OpenCraft/issues/178) Canvas | 8 |
| [#179](https://github.com/shantanugoel/OpenCraft/issues/179) Inspector cards | 10.1, 10.2, 10.3 |
| [#180](https://github.com/shantanugoel/OpenCraft/issues/180) Device inspector | 7.2, 10.7 |
| [#181](https://github.com/shantanugoel/OpenCraft/issues/181) Page inspector | 8.2 (touch bar control), 10.6 |
| [#182](https://github.com/shantanugoel/OpenCraft/issues/182) Knob inspector | 10.4, 20 |
| [#183](https://github.com/shantanugoel/OpenCraft/issues/183) Toggle and sequence | 10.5, 20 |
| [#184](https://github.com/shantanugoel/OpenCraft/issues/184) Library | 11, 20 |
| [#185](https://github.com/shantanugoel/OpenCraft/issues/185) Drag and drop | 12, 8.4 |
| [#186](https://github.com/shantanugoel/OpenCraft/issues/186) Undo and redo | 5, 13 |
| [#187](https://github.com/shantanugoel/OpenCraft/issues/187) Copy and paste | 8.3, 10.2 (the "⋯" menu), 13 |
| [#188](https://github.com/shantanugoel/OpenCraft/issues/188) Test an action | 10.3 |
| [#189](https://github.com/shantanugoel/OpenCraft/issues/189) Label options | 10.2 |
| [#190](https://github.com/shantanugoel/OpenCraft/issues/190) Icons | 3.6, 10.2, 14, 20 |
| [#191](https://github.com/shantanugoel/OpenCraft/issues/191) Background image | 8.2, 10.6, 20 |
| [#192](https://github.com/shantanugoel/OpenCraft/issues/192) Command palette | 5, 13, 15 |
| [#193](https://github.com/shantanugoel/OpenCraft/issues/193) Settings window | 7.4, 13, 17 |
| [#194](https://github.com/shantanugoel/OpenCraft/issues/194) Menu bar item | 19 |
| [#195](https://github.com/shantanugoel/OpenCraft/issues/195) Profile sheet | 7.5, 16, 20 |
| [#196](https://github.com/shantanugoel/OpenCraft/issues/196) First start | 18 |
| [#209](https://github.com/shantanugoel/OpenCraft/issues/209) Design language | 3, 21, 22 |
| [#197](https://github.com/shantanugoel/OpenCraft/issues/197) User guide and footprint | All sections; 23 for the footprint |

## Rejected alternatives

| Alternative | Reason |
|---|---|
| Keep the current single screen, and add buttons at the top | Each feature adds a button. The window has no place for plug-ins, folders, more gestures, or more devices. |
| The library as a fourth column | The device gets smaller. At 1440 × 900 the canvas would be about 600 wide. The library is a tab of the right panel, as in Loupedeck and Stream Deck the action list is on one side ([kb/ui-research.md](../kb/ui-research.md)). |
| A top-level screen for each feature (for example a "Device" screen, a "Plug-ins" screen) | The user loses the device from view. The HIG puts the settings of the selected thing in the inspector, and the general settings in the Settings window. |
| A third sidebar level for folders | The HIG says two levels at the maximum. A folder goes into the page header as a path (section 21). Decision [0036](0036-folders.md) keeps two levels: a folder row is at the same level as a page row, and the path is in the page header. |
| One card for each binding, as a list of bindings | A gesture with no action has no place, so the user does not see what is possible. One card for each gesture shows the empty gestures as "Add an action". |
| Device settings in a separate window | The device is a thing that the user selects, as a key. Its settings belong in the inspector. The Settings window holds the options of the application. |
| Draw the LATER places as disabled controls | A user sees controls that do nothing and asks when they come. The code keeps the places; the window does not show them. |
| Web views for plug-in settings, as in VSD Craft and Stream Deck | The footprint. See section 23. |
| Bundle the font Inter of the wireframes | A native application uses the system font. A font file adds to the bundle. |
| A light appearance in this overhaul | The wireframes give the dark values only. The tokens of section 3.2 make a light set a later change with no layout change. |

## Results

- Each task of #173 has a section to refer to (section 24). An agent can do
  a task with this record and its issue.
- The visual constants of section 3 give one style. #209 moves them into one
  module and writes `docs/DESIGN.md` from the code.
- A new feature has one place (section 21) and five rules (section 22). The
  toolbar and the sidebar do not get one button for each feature.
- Some tasks add data to the engine messages (section 20). Each of them
  updates decision 0014 or decision 0009 in the same pull request.
- The window stays dark only. A light appearance needs a second set of
  values for the tokens. Look at this record again when the owner asks for
  it.
- The device drawing uses the descriptor. A second device model with a
  different layout must show correctly with no change to the shell. Look at
  section 8.2 again when OpenCraft operates a second model or more than one
  device.
- The sizes come from wireframes. #175 tests what GPUI can do. If GPUI
  cannot do a part (for example the turned drag picture, or a popover with
  an arrow), the task uses the nearest form and writes the limit in
  [kb/gpui.md](../kb/gpui.md). It does not change the layout.
- No rule of [ARCHITECTURE.md](../ARCHITECTURE.md) changes.
