# The design language of the window

Read all of this document before you change a part of the window. It gives
the rules that keep the main window, the Settings window, the first start
window, and the menus consistent. Each rule names the place in `crates/app`
that holds it. If the code and this document disagree, correct one of them in
the same pull request ([README.md](README.md#rules)).

- The layout (where each part is, and where a new feature goes) is in
  [decision 0029](decisions/0029-window-layout.md). This document does not
  repeat it.
- The facts about GPUI and its traps are in [kb/gpui.md](kb/gpui.md).
- This document replaces section 3 of decision 0029 for the visual constants.
  The values are in `crates/app/src/view/theme.rs` only.

## 1. Principles

| Principle | What it means for a change |
|---|---|
| The device is the centre | The canvas shows the real device with the images of the engine (rule 11). A change does not make the device smaller or hide it behind a screen. |
| Select, then change | The user selects a thing (a control, the page, the device). The inspector shows its settings. Do not add a second place to change the same setting. |
| A native Mac application | System font, dark surfaces, small radii, quiet colours, the accent only for selection and the main action. It must look like a good macOS application, not like a web page. |
| Each change applies at once | No "Apply" and no "Save" button. Each change goes to the engine at once and is one step of the undo history. |
| Quiet until used | Hover, focus, and selection show a state. No animation, no colour for decoration, no control that does nothing (a LATER place is not drawn). |
| One way to look, many ways to do | One control has one look in all parts. One operation has one code path for the mouse, the menu, the keys, and the palette. |
| Data, not lists | The window shows what the engine sends. It has no list of action kinds, gestures, models, or commands of its own (section 9). |

## 2. Tokens

All tokens are in `crates/app/src/view/theme.rs`. Use the name. Do not write
a value.

- **Do not** write a literal colour (`rgb(0x…)`, `0xRRGGBB`, `hsla(…)`) or a
  `px(number)` in window code outside `theme.rs`. The test
  `view::theme::tests::no_literal_colour_or_size_outside_the_theme` fails if
  you do. It reads `view/`, `settings_window*`, and `first_start*`, with no
  test code.
- **A new value:** add one constant to `theme.rs`, in the `MARK` section of
  its part, with a comment that says where it is used. If a token with the
  same purpose exists, use it. Do not make a second name for the same
  purpose.
- **A colour of the engine** (a fill, a knob light, a profile colour) goes
  through `theme::colour(Color)`. A colour sample of the data goes through
  `theme::sample(u32)`.
- The window is dark only. A light appearance is a second set of values for
  the same names (decision 0029, section 21).

### 2.1 Colours

| Token | Use |
|---|---|
| `BG_CANVAS` | The canvas, the window behind all panes |
| `BG_BAR` | The toolbar, the status bar, a text field in a gesture card, the Settings content, the first start window |
| `BG_SIDEBAR` | The sidebar, the button row of a sheet, the Settings title area |
| `BG_PANEL` | The right panel, a nested card |
| `BG_CARD` | A gesture card, a group box, a sheet, a menu that the window draws |
| `BG_CONTROL` | A button, a popup button |
| `BG_FIELD` | A text field, a search field, a segmented control track, a square button, a popover, the palette, the device card |
| `BG_SEGMENT_ON` | The selected segment or tab, a toggle button that is on, a switch that is off |
| `BG_TILE` | The tile behind a device icon, a command tile of the palette |
| `LINE_PANE` | The line between two panes, the line in a card, a group box border |
| `LINE_SECTION` | The line above a section, below the toolbar, above the status bar, below the tab header, a menu separator |
| `LINE_SIDEBAR` | The line above the bottom group of the sidebar |
| `BORDER_FIELD` | A text field, a card, a square button, a chip that is not selected |
| `BORDER_CONTROL` | A button, a popup button |
| `BORDER_OVERLAY` | A menu, a popover, the palette, a sheet, a dashed empty card |
| `TEXT_PRIMARY` | Normal text |
| `TEXT_SECONDARY` | A form label, an icon button, a value at the end of a row |
| `TEXT_TERTIARY` | A section title, a caption, a help text, an empty state, a placeholder |
| `TEXT_DISABLED` | A disabled control, a dot of "Not connected" |
| `TEXT_ON_ACCENT` | Text on `ACCENT` or on `BG_SEGMENT_ON` |
| `ACCENT` | The selected row, the main button, a switch that is on, a slider fill |
| `ACCENT_LINE` | The selection outline, the focus border of a field, the focus ring, the border of a control with an open menu or popover |
| `ACCENT_TEXT`, `ACCENT_TEXT_HOVER` | A link; the label of a selected knob; the dot of the last input |
| `ACCENT_TINT`, `ACCENT_TINT_START`, `ACCENT_TINT_CHIP`, `ACCENT_TINT_DRAG`, `ACCENT_TINT_DROP` | The accent at 14 % (selected card), 16 % (selected starter card), 25 % (selected chip), 18 % (library row in a drag), 28 % (drop target) |
| `SUCCESS`, `SUCCESS_TEXT`, `SUCCESS_TINT`, `SUCCESS_FILL` | Connected; the live state pill; the check badge of the first start |
| `WARNING` | A warning icon, a warning text, "not given" |
| `DESTRUCTIVE_TEXT`, `DESTRUCTIVE_TINT`, `DESTRUCTIVE_LINE` | A button that removes; the message rows above the status bar |
| `INPUT_GLOW`, `input_glow_shadow()` | A control that the user operates on the device |
| `KIND_TILE_*`, `kind_tile(group)` | The tile of an action kind, from its group |
| `DEVICE_*` | The device drawing only |
| `FILL_COLOURS`, `TITLE_COLOURS`, `KNOB_COLOURS`, `PROFILE_COLOURS`, `ICON_COLOURS` | Colour samples. These are data: the profile keeps them. Do not change a value. |

### 2.2 States

| Token | State |
|---|---|
| `HOVER_CONTROL` | A button or a square button under the pointer (one step lighter than `BG_CONTROL`) |
| `HOVER_ICON` | A transparent icon button under the pointer |
| `HOVER_ROW` | A row, a segment, a chip, or a dashed card under the pointer |
| `PRESSED` | Each pressed control (`.active`) |
| `MENU_HIGHLIGHT` | The highlighted row of a menu that the window draws |
| `SELECTED_UNFOCUSED` | A selected row in a pane that does not have the selection |
| `NEUTRAL_TINT` | The live state pill and the touch bar control when they are not active |
| `ACCENT_LINE_HOVER` | The outline of a canvas control under the pointer |
| `MAIN_HOVER_OPACITY`, `MAIN_PRESSED_OPACITY` | The main button under the pointer, and pressed |
| `MODAL_DIM` | The layer behind a sheet or the palette |
| `focus_ring_outside()`, `focus_ring_inside()`, `FOCUS_RING` | The focus ring of a control with the keyboard focus: 2 in `ACCENT_LINE`, outside the edge of a control with a fill, inside the edge of a control with no fill (section 5) |

### 2.3 Text

The font is the system font. The window has no font file.

| Token | Size | Weight | Use |
|---|---|---|---|
| `TEXT_BADGE` | 10 | 400 to 600 | "ON DEVICE", a hex value, the size text of a preview |
| `TEXT_SECTION` | 11 | 600, capital letters | A section title or a group title. Use `controls::section_title`. |
| `TEXT_CAPTION` | 11 | 400 or 500 | A number at the end of a row, a hint, a knob label, a key hint |
| `TEXT_HELP` | 11.5 | 400 | The status bar, a help text, the live state pill, a Settings description |
| `TEXT_LABEL` | 12 | 400 or 500 | A form label, a small button, a segment, a link, an empty state |
| `TEXT_CONTROL` | 12.5 | 400 or 500 | A popup button, a search field, a menu row, a step row, a sheet label |
| `TEXT_BODY` | 13 | 400, 500, or 600 | The default: rows, text fields, the toolbar title, a main button |
| `TEXT_DEVICE_STATE` | 14 | 600 | "*model* found" of the first start |
| `TEXT_HEADING` | 15 | 600 | The page name in the page header, the name in the inspector header |
| `TEXT_SHEET_TITLE` | 17 | 600 | A sheet title, "OpenCraft" in "About" |
| `TEXT_DISPLAY` | 22 | 600 | The title of the first start window |

- Weight: 400 for normal text, 500 for a control text and a row, 600 for a
  name, a title, and a gesture name. 700 only for "B" (bold) and the letter
  of a kind tile.
- Line height: the default of GPUI for one line. `LINE_HEIGHT_TEXT` (1.45)
  for text of more than one line. `FIRST_START_LINE_HEIGHT` (1.5) only below
  the first start title. `SETTINGS_LINE_HEIGHT` (1.25) only in the Settings
  window, where each row has fixed lines.
- GPUI has no letter spacing. A section title uses the normal spacing.

### 2.4 Spacing

| Token | Use |
|---|---|
| `SPACE_1` to `SPACE_32` | The only steps for padding and gaps: 1, 2, 3, 4, 6, 8, 10, 12, 14, 16, 18, 20, 24, 28, 32 |
| `GAP_ITEMS` | Between the toolbar items, and between buttons in a row |
| `SECTION_PADDING_TOP`, `_SIDE`, `_BOTTOM` | A section of the right panel: 14, 16, 16 |
| `SECTION_HEADER_GAP`, `SECTION_HINT_GAP` | Below a section title: 12; with a hint: 4, then a content gap of 8 |
| `FORM_GAP`, `CARD_FORM_GAP` | Between the columns and the rows of a form: 10; in a gesture card: 8 |
| `CARD_GAP` | Between two gesture cards |
| `EMPTY_STATE_TOP` | Above the text of an empty state in a pane |
| `FORM_LABEL`, `FORM_LABEL_WIDE`, `SHEET_LABEL` | The label column: 64 (inspector, card), 72 (page and device inspector), 110 (sheet) |

### 2.5 Radii

| Token | Value | Use |
|---|---|---|
| `RADIUS_BADGE` | 4 | A badge, a key hint |
| `RADIUS_SEGMENT` | 5 | A segment, a fill sample, the sidebar "+" button |
| `RADIUS_CONTROL` | 6 | A button, a text field, a popup button, a sidebar row, a menu row |
| `RADIUS_TRACK` | 7 | A segmented control track, a search field, a library row, a sheet button, a main button |
| `RADIUS_CARD` | 8 | A nested card, a palette row, an icon cell |
| `RADIUS_GESTURE_CARD`, `RADIUS_DEVICE_CARD` | 9 | A gesture card, the device card |
| `RADIUS_GROUP` | 10 | A group box of the Settings window |
| `RADIUS_POPOVER` | 12 | A menu, a popover |
| `RADIUS_SHEET` | 14 | A sheet, the palette |
| `rounded_full()` | Full | A switch, a chip, a dot, a colour circle, a knob |

### 2.6 Heights and sizes

| Token | Value | Use |
|---|---|---|
| `HEIGHT_SEGMENT_INSPECTOR` | 22 | A segment of a form |
| `HEIGHT_PILL`, `HEIGHT_CHIP`, `HEIGHT_ADD_STEP` | 24 | The live state pill, a chip, "Add a step" |
| `HEIGHT_SEGMENT` | 26 | A tab segment, a small button, the page header buttons |
| `HEIGHT_CONTROL`, `HEIGHT_MENU_ROW` | 28 | A button, a text field, a sidebar row, a toolbar button, a square button, a menu row |
| `HEIGHT_ACTION_POPUP`, `HEIGHT_SEARCH_FIELD`, `HEIGHT_SHEET_CONTROL` | 30 | The action popup, a search field of a panel, a field and a button of a sheet, a main button |
| `HEIGHT_LIBRARY_ROW`, `HEIGHT_STEP_ROW` | 34 | A library row, a step row |
| `HEIGHT_PALETTE_ROW` | 36 | A palette row |
| `HEIGHT_EMPTY_CARD` | 40 | A gesture card with no action |
| `SWITCH_INSPECTOR`, `SWITCH_SETTINGS` | 30 × 18, 32 × 19 | A switch in the inspector, in the Settings window |
| `WIDTH_ICON_BUTTON` | 30 | A transparent icon button (toolbar, sidebar top) |
| `WIDTH_SQUARE_BUTTON`, `SAMPLE`, `COLOUR_CIRCLE` | 28, 22, 20 | A square button; a fill sample; a colour circle |
| `BORDER`, `OUTLINE_WIDTH`, `HOVER_OUTLINE_WIDTH` | 1, 2, 1 | A border; the selection outline; the hover outline |
| `LINK_FOCUS_PADDING`, `SLIDER_FOCUS_PADDING_X`, `_Y` | 2; 8, 4 | The space between a link or a slider and its focus ring |
| `FIRST_START_READY_WIDTH`, `FIRST_START_READY_MAX_HEIGHT` | 360, 420 | The list of ready profiles on the first start window |
| `DEVICE_PLAIN_SIZE`, `DEVICE_PLAIN_GAP`, `DEVICE_PLAIN_TOP` | 96, 16, 22 | A control with no surface, below the display. The values are pixels of the display (decision 0029, section 8.2). |

### 2.7 Icons

- The icons are Lucide, through `gpui_kit::assets::IconName`. Add each new
  icon to `theme::WindowIcons`, else it shows nothing. The kind tiles and the
  icon picker use the icon set of the engine (`theme::surface_icon_path`).
- The icon colour is the text colour of its control.

| Token | Size | Use |
|---|---|---|
| `ICON_PLAY`, `ICON_KIND_NESTED` | 11 | "Test the action"; a kind in a tile of 18 |
| `ICON_SMALL` | 12 | A chevron, a "+" in a row or a card |
| `ICON_KIND` | 13 | A kind in a tile of 20 |
| `ICON_BUTTON`, `ICON_SEARCH`, `ICON_MESSAGE`, `ICON_KIND_LIBRARY` | 14 | A square button, a search field, a message row, a tile of 24 |
| `ICON_ROW` | 15 | A sidebar row |
| `ICON_TOOLBAR`, `ICON_MORE`, `ICON_APPLICATION` | 16 | Undo and Redo, the "⋯" button, an application in the application picker |
| `ICON_PANE` | 17 | "Hide the sidebar", "Hide the inspector" |
| `ICON_TAB` | 20 | A tab of the Settings window |

### 2.8 Shadows

| Function | Use |
|---|---|
| `popover_shadow()` | A menu, a popover, the icon picker, the application picker |
| `sheet_shadow()` | A sheet, the palette |
| `drag_shadow()` | The drag picture |
| `device_body_shadow()` | The body of the device |

## 3. Components

Use a shared component before you make a new one. A new component that two
parts use goes into `view/controls.rs` (a control) or `view/form.rs` (an
inspector part). Do not copy a component and change it at the call site.

### 3.1 Controls (`crates/app/src/view/controls.rs`)

Each control in this table that the user operates takes the keyboard focus
when it is enabled (`keyboard_focus`, section 5). The "States" column does
not repeat "focus" for them. A text field shows its focus with its border.

| Function | Use it for | Do not use it for | States |
|---|---|---|---|
| `button`, `button_if` | A normal command button, 28 high: "Duplicate", "Open System Settings", "Export as a file…" | A button in a sheet button row (`sheet_button`), a small button in a row (`small_button`) | hover, pressed, disabled (`button_if`) |
| `small_button`, `small_icon_button` | A button of 26 in a row or a message: "Close", "Show in Finder", "Page settings", "Test the action" | The main action of a sheet | hover, pressed, disabled |
| `main_button` | The one main action of a sheet or a window: "Done", "Start", "Open System Settings" in a sheet | More than one button in one place | hover, pressed (opacity) |
| `sheet_button` | Other buttons of a sheet button row; with `destructive`, "Remove the profile…" | A button in the inspector | hover, pressed, disabled, destructive |
| `icon_button` | A transparent pane button with icon 17: show and hide the sidebar and the inspector | Any other icon button | hover, pressed |
| `toolbar_button` | A transparent toolbar button with icon 16: Undo, Redo | A button in a pane | hover, pressed, disabled. The ID holds the label, so the tooltip changes. |
| `square_button`, `square_icon_button` | A square button with a border and one icon: "Previous page", "Move the page up", the "⋯" button (`form::more_button`) | A button with text | hover, pressed, disabled |
| `popup_button` + `popup_menu` + `menu_row` + `menu_separator` | A choice from a list (a run-time choice list uses checks, adds at the end and removes a selected value): the action popup, a choice field, the label size, the fit, "First page" | A command menu at the pointer (use a native menu) | hover, pressed, open (`ACCENT_LINE` border); the menu has the keys of `view/menu.rs` |
| `segmented` + `segment`, `segment_if` | Two to five choices that all show: the tabs of the right panel, "Position", "Background", "Knob lights", the direction of a turn | More than five choices (use a popup button), a command | selected, hover, disabled |
| `switch`, `switch_if` | An on/off setting that applies at once. `switch_if` can be disabled: no focus, and the caller adds no click | A choice between two named values (use a segmented control) | on, off, disabled |
| `slider` | A number in a range: brightness | A number that the user types | disabled (no thumb); Left, Right, Up, and Down move it one step |
| `text_field`, `text_field_sized` (`FieldSize::sheet`) | Each text field: the inspector, a card (`BG_BAR`), a popover, a sheet | The name field in a sidebar row (it has the size of the row) | focus (`ACCENT_LINE` border) |
| `search_field` | The search field of a panel: the library, the icon picker, the application picker | The toolbar search (a button that opens the palette) | focus |
| `colour_sample` | A fill sample of 22, "No fill" with `None` | A profile colour or an icon colour (use `colour_circle`) | selected (outline 2, 2 outside) |
| `colour_circle` | A colour of a profile or of an icon, 20 | A fill sample | selected |
| `colour_button` | A square button that shows a colour and opens a popover | A colour sample | open (`ACCENT_LINE`, no tooltip) |
| `toggle_button` | A square toggle with a short text, for example "B" | An on/off setting (use a switch) | on, off, hover, pressed |
| `chip` | A tab chip or a filter chip: "Icons", "Recent" | The tabs of a pane (use a segmented control) | selected, hover |
| `link` | An operation that changes the view, not the data: "Set left and right apart"; a fixed browser link in Settings > About | A command that changes the profile | hover |
| `section_title` | The text of each section title and group title | Other text in capital letters | none |
| `group_title` | A group title in a list or a menu: the library, the kind menu | A section of the inspector (`form::section`) | none |
| `key_hint` | A key command in a box: "⌘K" | A key command in a row (plain text) | none |
| `dot` | A state dot of 6 | Decoration | none |
| `form::colour_palette` | The shared fill samples, in a fill row or an action colour field. The caller supplies the change handler. A colour field has the hex field below the samples, with `HEX_HELP`. | — | selected, hover, focus |
| `HEX_HELP` | The help text below a field for `#RRGGBB` | — | — |
| `keyboard_focus`, `Ring` | Each new control, row, or card with a click handler: Tab reaches it, Space and Return operate it, and it has the focus ring | A disabled control, a menu row (the menu has its own keys), a control of the device drawing | focus ring |
| `keyboard_focus_tracked` | A group in which the arrow keys move the selection and the focus: the cards of the first start window | Other controls | focus ring |
| `popover_focus` | Each drawn menu and popover (`popup_menu` adds it, the icon picker and the application picker call it): a focus trap, a click in it keeps the focus, and the focus goes back to the control that a key used to open it (`view/focus_return.rs`) | A sheet (`Modal` has its own trap and the same return of the focus) | — |

The device inspector uses the shared popup button and menu for **Screen off
after**. `WIDTH_SCREEN_OFF_MENU` is the width of that button and menu. The
engine holds the inactivity deadline; the view makes no timer.

### 3.2 Inspector parts (`crates/app/src/view/form.rs`)

| Function | Use it for | Do not use it for |
|---|---|---|
| `header` | The top of each inspector: picture 64, name, second line, "⋯" | A section |
| `more_button` | The "⋯" button of a control, with a native menu | Other commands |
| `section` | A section with a line at the top, a title, and an optional hint | A group of the library (`group_title`) |
| `form`, `row` | A two-column form | A row in a sheet (`modal::sheet_row`) or in the Settings window (`settings_window/parts.rs`) |
| `fitted_row` | A row with a label from data (a field label): the label goes above the value if it does not fit | A fixed label that fits |
| `top_row` | A value of more than one line | — |
| `stacked_row` | A value of full width below its label: a nested card | — |
| `end_row` | A control at the end with no label: "Test the action" | — |
| `help` | A help text below a control | A message (status bar) |

### 3.3 Larger components

| Component | File | Use it for |
|---|---|---|
| `GestureCard`, `KindLabel` | `view/gesture_card.rs` | Each gesture of a control, a swipe of the page, a nested action, a step. The kind tile, the kind content, and the drag chip of each kind. |
| `Modal`, `sheet_title`, `sheet_row`, `sheet_section_header`, `button_row` | `view/modal.rs` | Each sheet and the command palette: the dim layer, the focus, Escape, Return |
| `MenuKeys`, `MenuRow` | `view/menu.rs` | The keys of a drawn menu. `popup_menu` adds them. |
| `group`, `row`, `line_row` | `settings_window/parts.rs` | The group boxes of the Settings window |
| Icon picker popover | `view/icon_picker.rs` | Only the icon picker (it has an arrow) |
| Application picker | `view/application_picker.rs` | A path field of kind Application, and "Add an application…" in the profile sheet. The list shows the name and the icon of each installed application, a search field, and "Choose a different application…". For a link, a row whose application is linked to a different profile shows "Linked to “*profile*”" at its end, in `TEXT_CAPTION` and `TEXT_TERTIARY`, at most 55 % of the row, with "…" (decision 0035, section 5). The row label for VoiceOver is "*name*, Linked to “*profile*”". It has no arrow. |
| Native menu (`NativeMenu`) | `view/sidebar.rs`, `view/clipboard.rs`, `view/nested.rs` | A command menu at the pointer: a right-click menu, the "⋯" menu, the device switcher |
| Native alert (`window.prompt`) | `view/pages.rs` (`confirm_remove`) | The confirmation of a destructive command |

### 3.4 Role and accessible label (`controls::Accessible`)

VoiceOver reads only an element that has an ID and a role. Plain text and
an element with no role are not in the accessibility tree
([kb/gpui.md](kb/gpui.md#accessibility-tree-of-the-window-2026-11-13)).

- Each shared control of `controls.rs`, `form.rs`, `gesture_card.rs`,
  `menu.rs`, and `modal.rs` sets its role and its label. A new control that
  uses them gets both. Do not set them again at the call site.
- The label is the visible name or the tooltip of the control. Do not write
  a second text for it. A control with a value from a row label gets the
  row label as a parameter: `switch`, `text_field`, `popup_button`,
  `search_field`. Use one constant for the row label and the control.
- A new clickable element that is not a shared control: call
  `.accessible(role, label)`. A text that VoiceOver must read, for example a
  title or a state: call `.readable_text(text)`. A control that is on or
  off: call `.checked(on)`. Add `aria_selected`, `aria_expanded`, or
  `aria_description` for a state or a hint.
- A part with children that the user operates is a group (`Role::Group`),
  not a button: VoiceOver goes into a group, for example the device body
  and the head of a gesture card.

| Element | Role | Label |
|---|---|---|
| A button, a square button, an icon button, a sheet button, the main button | `Button` | The text or the tooltip |
| A toggle button ("B") | `Button` with `checked` | The tooltip |
| A switch | `Switch` with `checked` | The row label |
| A segment, a chip, a colour sample, a colour circle, a starter card, a tab of the Settings window | `RadioButton` with `checked` | The text or the tooltip |
| A segmented control | `RadioGroup` | None |
| A popup button | `ComboBox` with the value | The row label |
| A menu, a menu row | `Menu`, `MenuItem` | The text of the row (`MenuRow::child` or `MenuRow::label`) |
| A text field, a search field | The role of the kit field | The row label |
| A link | `Link` | The text |
| An inspector header, a section | `Group` | The name, the section title; the hint is the description |
| A sheet, the palette, the icon picker, the application picker | `Dialog` | The label of decision 0029. The application picker uses "Choose an application". |
| The sidebar, its rows | `Navigation`, `Tree` with `TreeItem` rows | "Devices, profiles and pages"; the name of the row |
| The toolbar, the right panel, the status bar | `Toolbar`, `Complementary`, `Status` | None, the tab name, none |
| A key face, a bar panel, a knob, the touch bar | `Button` with `aria_selected` | The control name and the title: "Key 2, “Mail”" |
| A control with no surface, other than a knob or the touch bar | `Button` with `aria_selected` | The control name, for example "Key 7" |

## 4. Layout rules

- Each part and each new feature has its place in
  [decision 0029](decisions/0029-window-layout.md): sections 4 to 19 for the
  parts, section 21 for the LATER places, section 22 for the rules for a new
  feature.
- Do not add a button to the toolbar or a row to the sidebar for one
  feature. A new command goes into a menu, the command palette, or the
  inspector (decision 0029, section 22).
- Do not draw a LATER place. The code keeps its place: one more entry in a
  list, a tab row, or a section stack.
- The window must show all controls at the minimum size 1080 × 700
  (`WINDOW_MIN`). A long text ends with "…" (`truncate`), it does not push
  other parts.
- The Settings window has a fixed width and takes its height from the
  content of its pane (`Fit` in `settings_window.rs`). A new row, a new
  text size, or a new button size needs no number for the height. Do not
  write a height for a pane. The window changes its height before it shows
  a new pane, and it opens at the height of its content (`docs/kb/gpui.md`,
  "Height of the Settings window from its content").

## 5. Behaviour rules

| Subject | Rule | Where |
|---|---|---|
| Selection | One selection: a control, the page, or the device. A selected control has an outline of `OUTLINE_WIDTH` in `ACCENT_LINE`. A selected row is `ACCENT` with `TEXT_ON_ACCENT`. A selected row in a pane without the selection is `SELECTED_UNFOCUSED`. The selection stays when the page changes. | `state.rs`, `view/canvas.rs`, `view/sidebar.rs` |
| Hover | Each clickable element has a hover look and the pointer hand. A disabled element has neither. | `controls.rs` |
| Pressed | Each button has `.active(PRESSED)`. A main button changes its opacity. | `controls.rs` |
| Disabled | `TEXT_DISABLED`, no hover, no pointer, no click handler. Show a disabled control, do not hide it, when the user expects it at that place (for example "Move Up" on the first page). | `controls.rs` |
| Focus | A text field with the focus has the `ACCENT_LINE` border. Each other control that the user operates takes the keyboard focus through `controls::keyboard_focus`: a shared control does this when it is enabled, and a row or a card with a click handler calls it. The control with the focus shows the focus ring (2, `ACCENT_LINE`) only when the focus came from the keyboard, as macOS does: GPUI `focus_visible` applies when the last input was a key. Outside the edge for a control with a fill (`Ring::Outside`: a button, a popup button, a switch, a sample), inside the edge for a control with no fill and for a colour circle (`Ring::Inside`: an icon button, a segment, a chip, a link, a row, a card). A click does not give a control the focus: the focus goes to the owner (`controls::FocusOwner`): the root element, the card of a sheet, or, in a popover, it stays where it is. After a click that ends an edit, call `focus_workspace`, not `window.blur`: the key commands need the focus in the root element. `focus_workspace` keeps the focus on a control that has it. When the element with the focus goes away, the focus goes to the root element (`on_focus_lost`). A popover with a field, the icon picker, the application picker, a sheet, and the command palette give the focus back to the control that had the keyboard focus when a key opened them (Space on the button, ⌘K): after Escape, Return, a choice, or a click outside, that control has the focus and the focus ring, and the next Tab goes to the next control. After a click that opened them, no control had the focus, so the focus goes to the root element and no ring shows. The code is in `view/focus_return.rs`: `popover_focus` and `Modal` do it, and a new popover or sheet needs no code. | `view/controls.rs`, `view.rs`, `view/focus_return.rs` |
| Keyboard | Tab and Shift-Tab move the focus in the order of the window: the sidebar, the toolbar, the page header, the tabs of the right panel, the inspector, the status bar. A sheet, the palette, and a popover are focus traps. The cards of the first start window are one stop: the arrow keys move the selection and the focus. The controls of the device drawing and the rows of the library are not in the order: ⌘K selects a control and sets an action. Space and Return operate the control with the focus. In a sheet, Return does the main action also when a button has the focus, and Space operates that button, as in a sheet of macOS. The key commands are in the key context `Workspace`. They operate also when a control has the focus. A focused text field keeps ⌘Z, ⇧⌘Z, ⌘C, ⌘V, and Delete. With the focus on a button, Delete clears the selected control, as **Edit > Clear** does (decision 0029, "Focus rule"); undo restores it. A drawn menu has Up, Down, Home, End, Return, Space, and letters; Tab does nothing while it is open. The application picker uses Up and Down to move the highlight, and Return to select the row. Space stays in its search field. Escape closes the menu, the popover, or the sheet. | `app_menu.rs`, `view/controls.rs`, `view/menu.rs`, `view/modal.rs`, `view/application_picker.rs` |
| Command menu | A menu of commands at the pointer is a native menu: right-click, "⋯", the device switcher. Native menu items use title case. | `view/sidebar.rs`, `view/clipboard.rs` |
| Choice menu | A choice from a popup button is a drawn menu (`popup_menu`). It shows below the button, else above, else covers the button. The first row is highlighted. A click outside closes it and does nothing else. | `controls.rs` |
| Popover | A colour popover uses `popup_menu` with one row of content. A click on a sample applies at once, and the popover stays open. Escape, Return, a click outside, or a second click on its button closes it, and the focus goes back to its button if a key opened it (section 5, "Focus"). A control with an open popover has no tooltip. | `view/inspector.rs`, `view/device_inspector.rs` |
| Sheet | A sheet uses `Modal`. Each change in it applies at once. "Done", Return, and Escape close it. A click on the dim layer does not close a sheet; it closes the palette. The key commands of the window do not operate while a sheet is open. | `view/modal.rs` |
| Destructive command | "Remove…" of a page or a profile asks with a native alert that names the thing. The alert has "Remove" and "Cancel". A change that undo can restore (Clear, a replaced action) does not ask. | `view/pages.rs` |
| Undo | Each change of the pages of the profile is one step with a name in title case ("Set Title"). One edit of a text field is one step: call `text_edit_event` in the subscription of a new field. A command that changes more than one binding sends one `Command::Group`. Profile add, rename, remove, select, and the device settings are not steps. Each profile keeps its history while OpenCraft runs. | [decision 0030](decisions/0030-undo-and-redo-in-the-engine.md), `view/history.rs` |
| Empty state | A pane with nothing to show has one sentence, `TEXT_LABEL`, `TEXT_TERTIARY`, centred, `EMPTY_STATE_TOP` below the top. A gesture with no action is a dashed card "Add an action". | `view/inspector.rs`, `view/library.rs` |
| Error | A missing Accessibility permission also shows the native menu command "Accessibility Permission Not Given…", which opens Settings at General. It follows the engine statuses. A problem of the engine is a message row above the status bar, with "Close" (`DESTRUCTIVE_TINT`, a warning icon). The Settings window shows its problems below its pane. Do not use a modal alert for an error. | `view/status_bar.rs`, `settings_window.rs` |
| Status bar | The last input at the start, "All changes saved" at the end. A short result of an operation is a message for `MESSAGE_TIME` (4 s) with `show_message`. Do not add a second bar. | `view/status_bar.rs` |
| Drag and drop | The drag picture is `KindLabel::drag_chip` at `DRAG_PICTURE_OFFSET`. Only the control under the pointer shows the drop target. A drop and a click use one path (`assign_to`). | `view/library.rs`, `view/canvas.rs` |
| Test command | “Test the action” ignores an input event that this process sent. A synthetic click on the test button must not invoke the command again. All action kinds use the same guard; no kind check. | `view/synthetic_input.rs`, `view/gesture_card.rs` |
| One code path | The mouse, the menu, the keys, and the palette call the same function for the same operation. A command of the window is one row of `app_menu::commands`: the menus, the "Shortcuts" pane, and the palette read that table. An action kind goes to a control through `assign_kind`. | `app_menu.rs`, `view/library.rs` |
| Tooltip and label | Each icon button has a tooltip. The tooltip text is also the accessible label; the shared controls set both. Each clickable element has a role and a label (section 3.4). | `controls.rs` |

## 6. Text rules

- **Sentence case** for all text on the window: labels, buttons, tooltips,
  section titles in the code, messages. **Title case** only for the items of
  a native menu (menu bar, right-click menu, "⋯" menu) and for the names of
  undo steps, as macOS does: "Hide Sidebar", "Move Up", "Copy to All Pages",
  "Profile Settings…", "Undo Set Title".
- **"…"** at the end of a command that opens a dialog, a sheet, a popover, a
  file panel, or a confirmation: "Remove…", "Choose icon…", "Image file…",
  "Settings…". No "…" on a command that does the work at once, and on a link.
  A state that waits ("Push a key…", "Record hotkey…") also ends with "…".
- **Form of a label:** a button is a verb phrase ("Add an action", "Test the
  action", "Show in Finder"). A tooltip of an icon button names the
  operation ("Hide the sidebar", "Previous page"). A form label is a noun
  ("Title", "Fill").
- **Lists:** no comma before "and" or "or": "a key, a bar panel or a knob".
- **Status bar message:** a statement with no full stop: "Key 8 gets “Open a
  URL” for Press", "Key 1 is copied", "Select a control first".
- **Empty state and help text:** a full sentence with a full stop.
- **Quotation marks:** “ ” around a name from the user or a kind name in a
  sentence: Page “Main”.
- **Key commands:** symbols in the order of macOS (⌃ ⌥ ⇧ ⌘, then the key),
  with no space: "⇧⌘A". The menus, the "Shortcuts" pane, the key hints, and
  the hotkey values use this form (`settings_window::shortcuts::keystroke_text`,
  `view::hotkey::symbols`). A right-side modifier has `R` before its symbol.
  Fn follows Command. The recorder completes a modifier-only chord when
  all modifiers are released.
- **A key of the keyboard:** the verb is "push", as in the user guide: "Type
  #RRGGBB and push Return." "Press" is the name of a gesture.
- **Numbers:** a space before "%" and "pt": "80 %", "20 pt". "×" for a size
  or a count: "800 × 480", "Turn right × 3".

### 6.1 Word list

Use one name for each thing. The user guide uses the same names.

| Use | Do not use | Meaning |
|---|---|---|
| key | button | A pressable control with a key face |
| key face | key image, LCD key | The display area of a key |
| bar panel | strip panel, segment | One of the four display areas of the touch bar |
| touch bar | strip, slider | The touch area below the keys, with swipes |
| knob | dial, encoder | A turnable control |
| control | element, widget | A key, a bar panel, a knob, or the touch bar |
| title | label (on the window) | The text on a key face or a bar panel. `label` is the name in the profile file. |
| fill | background colour | The colour of a key face or a bar panel |
| background | wallpaper | The image or the colour of a page |
| page | folder, screen | One set of key faces and bindings |
| profile | preset, layout | A set of pages |
| action, action kind | plug-in action, macro | What a gesture does; its kind ("Open a URL") |
| gesture | event, trigger | Press, Release, Tap, Turn, Turn left, Turn right, Push, Swipe left, Swipe right |
| inspector | properties | The tab of the right panel that edits the selection |
| library | actions panel | The tab of the right panel with the action kinds |
| command palette | search window | The window of ⌘K |
| Settings window | preferences | The window of ⌘, |
| connected, not connected | online, offline | The state of the device |

The names of the action kinds and the gestures come from the engine
(`KindDescription::name`, `state::gestures`). Do not write them in the
window code, except as test data.

## 7. Parts from data

| Rule | Example |
|---|---|
| The shell has no list of action kinds (rule 8). The library, the kind menu, the palette, and the permission texts read `Event::Kinds`. | `library::groups` |
| The shell does not compare a kind identifier with a built-in identifier. A property of a kind is a field of `KindDescription`. | `direction`, `states`, `needs_accessibility` |
| The settings form of an action comes from its description (decision 0014). A timed action list adds the shared integer field of decision 0017 to each open step. | `view/actions.rs` |
| The device drawing comes from `Event::Layout`. No check for a model (rule 4). | `view/canvas.rs` |
| The commands of the menus (also the sidebar's profile add menu), the "Shortcuts" pane, and the palette come from one table. | `app_menu::commands` |
| The key text of a command comes from its key binding. | `settings_window::shortcuts::keys` |

## 8. Do and do not

These are mistakes that the tasks of the overhaul made, and their
corrections.

| Do not | Do |
|---|---|
| Copy a shared control and change it (a second square button, a second segmented control, six copies of a section title, five copies of a colour conversion) | Use the function in `controls.rs` or `form.rs`; add a parameter if necessary |
| Write `px(2.)`, `0x34363c`, or `transparent_black()` in a part | Use or add a token in `theme.rs` |
| Keep `#![allow(dead_code)]` "for later tasks" | Add a token or a function when a part uses it |
| Use the default size of a kit field | Use `controls::text_field` |
| Let a drawn menu take the text style of its button | `popup_menu` sets its own text style |
| Let a menu scroll and hide its last row | `popup_menu` moves up and covers its button |
| Show a popover over the label of its row | Align the popover to the trailing edge (`MenuPlace::align_end`) |
| Show a tooltip on a control with an open popover | Remove the tooltip while it is open |
| Keep a tooltip text in a stable ID when the text changes | Put the text in the element ID (`toolbar_button`) |
| Let a long label break into short lines | Use `form::fitted_row` |
| Let the Kit field get Return after its own event | Handle Return before the field (`sidebar::name_field`) |
| Put two key bindings on one menu command | One binding: the menu shows the first one |
| Copy a wireframe text with "…" for a link that acts at once | Obey section 6 ("…" only for a dialog) |
| Write key commands in two forms ("⇧ ⌘ A" and "⇧⌘A") | One form: "⇧⌘A" |
| Give one thing two names ("Label" and "Title", "Choose image…" and "Image file…") | Use the word list |
| Draw a picture that hides a text of a drop target | Keep the text readable (no plus icon during a drag) |
| Show text on an image with no backing | Use `BADGE_ON_IMAGE` behind it |
| Make a menu item that the palette also needs in two places | Add one row to `app_menu::commands` |
| Use a different button size for the disabled state | `button_if(…, false)` keeps the size |
| Move the focus to the root element at the close of a popover or a sheet, in the code of the part | Call `focus_workspace` as before: it gives the focus to the control that opened the popover, if a key opened it |
| Make a row or a card with a click handler that the keyboard cannot reach | Call `controls::keyboard_focus` with the `Ring` of its look |
| Put a changing text in the ID of an element that takes the focus | Keep the ID stable: the focus handle lives in the state of the element (`toolbar_button` puts the tooltip in a child) |

## 9. Checklist for a pull request that changes the window

- [ ] I read this document and the sections of decision 0029 for my part.
- [ ] The part uses the tokens of `theme.rs` and the shared components. A
      new token or component has a comment, and this document lists it.
- [ ] The test `no_literal_colour_or_size_outside_the_theme` passes.
- [ ] Each new text obeys section 6 and the word list.
- [ ] Each new clickable element has a role and a label (section 3.4), and
      `view/tests/accessibility.rs` reads the labels of a new part.
- [ ] Each change of the profile is one undo step, and the same operation
      from the mouse, the menu, the keys, and the palette uses one path.
- [ ] Window tests (`crates/app/src/view/tests/`) cover the new behaviour.
- [ ] The four commands of `AGENTS.md` pass.
- [ ] Visual check: I started the real window with a scratch profile
      directory and the GUI lock, and looked at pictures of each changed
      state at 1440 × 900 and at 1080 × 700. Hover, selected, disabled,
      focus, and open menus agree with this document. The pull request has a
      picture.
- [ ] The user guide (`docs/user`) and its pictures show the change.
- [ ] The pull request lists each decision that this document or the record
      did not give.

## 10. Known differences

These parts do not obey this document yet. Each one has an issue.

| Difference | Issue |
|---|---|
| Nobody listened to the window with VoiceOver. The accessibility tree was read with the Accessibility API only. The procedure for the owner is in the issue. | [#243](https://github.com/shantanugoel/OpenCraft/issues/243) |
| A disabled control has no disabled state in the accessibility tree: GPUI 0.3.12 has no setter for it. It has no click action. | Accepted until GPUI has the setter |
| The knob name below a knob is a second click target of the knob. It has no role, so that VoiceOver reads the knob one time. | Accepted |
| Three highlight looks for a row that the keys select: `MENU_HIGHLIGHT` in a drawn menu, `ACCENT` in the palette, the system accent in a native menu. | Accepted: the palette row is a selection, the menu row is a highlight. |
