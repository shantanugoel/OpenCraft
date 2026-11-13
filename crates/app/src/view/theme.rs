//! The visual constants of the window: section 3 of decision 0029.
//!
//! This module is the one place for a colour, a text size, a spacing step, a
//! corner radius, a pane width, and a control height. The window code uses
//! these names and writes no number for them. Issue #209 adds the rest of
//! section 3 here, and writes `docs/DESIGN.md` from this module.
//!
//! The values are for the dark appearance. A light appearance is a second
//! set of values for the same names (decision 0029, section 21).

// The tasks of #173 use the names that the window does not use yet.
#![allow(dead_code)]

use gpui_kit::{point, px, BoxShadow, Pixels, Rgba};

/// A colour from its value `0xRRGGBB`, with full opacity.
const fn hex(value: u32) -> Rgba {
    hex_alpha(value, 1.)
}

/// A colour from its value `0xRRGGBB` and its opacity from 0 to 1.
const fn hex_alpha(value: u32, alpha: f32) -> Rgba {
    Rgba {
        r: ((value >> 16) & 0xff) as f32 / 255.,
        g: ((value >> 8) & 0xff) as f32 / 255.,
        b: (value & 0xff) as f32 / 255.,
        a: alpha,
    }
}

// MARK: Colours (section 3.2)

/// The canvas, and the window behind all panes.
pub const BG_CANVAS: Rgba = hex(0x18191c);
/// The toolbar and the status bar.
pub const BG_BAR: Rgba = hex(0x1f2024);
/// The sidebar.
pub const BG_SIDEBAR: Rgba = hex(0x232428);
/// The right panel, and a nested card.
pub const BG_PANEL: Rgba = hex(0x202125);
/// A card, and a menu that the window draws.
pub const BG_CARD: Rgba = hex(0x26272c);
/// A button.
pub const BG_CONTROL: Rgba = hex(0x2f3036);
/// A text field, the track of a segmented control, and the device card.
pub const BG_FIELD: Rgba = hex(0x2a2b30);
/// The selected segment of a segmented control.
pub const BG_SEGMENT_ON: Rgba = hex(0x4a4c53);
/// The tile behind the device icon.
pub const BG_TILE: Rgba = hex(0x2e3036);

/// The border between two panes.
pub const LINE_PANE: Rgba = hex(0x34363b);
/// The line below the toolbar, above the status bar, and below the tab header.
pub const LINE_SECTION: Rgba = hex(0x2c2e33);
/// The border of a text field, of a card, and of a segmented control track.
pub const BORDER_FIELD: Rgba = hex(0x3a3c42);
/// The border of a button.
pub const BORDER_CONTROL: Rgba = hex(0x42444b);
/// The border of a menu or of a popover.
pub const BORDER_OVERLAY: Rgba = hex(0x44474f);
/// The border of the device card when it is not selected.
pub const BORDER_DEVICE_CARD: Rgba = hex(0x383a40);
/// The line above the bottom group of the sidebar.
pub const LINE_SIDEBAR: Rgba = hex(0x2e3035);

/// Normal text.
pub const TEXT_PRIMARY: Rgba = hex(0xececee);
/// Form labels, icon buttons, and a value at the right.
pub const TEXT_SECONDARY: Rgba = hex(0xa0a3ab);
/// Section headers, captions, help text, and placeholders.
pub const TEXT_TERTIARY: Rgba = hex(0x8b8e96);
/// A disabled button or icon.
pub const TEXT_DISABLED: Rgba = hex(0x6b6e76);
/// Text on an accent fill.
pub const TEXT_ON_ACCENT: Rgba = hex(0xffffff);

/// The selected row, and the main button.
pub const ACCENT: Rgba = hex(0x1a6fd8);
/// The selection outline, and the focus border of a field.
pub const ACCENT_LINE: Rgba = hex(0x3b95ff);
/// A link, and the dot of the last input.
pub const ACCENT_TEXT: Rgba = hex(0x4da3ff);
/// The accent at 14 %: the background of a selected card.
pub const ACCENT_TINT: Rgba = hex_alpha(0x1a6fd8, 0.14);
/// The dot of a connected device.
pub const SUCCESS: Rgba = hex(0x34c759);
/// The text of the live state pill.
pub const SUCCESS_TEXT: Rgba = hex(0x7ee2a0);
/// The background of the live state pill.
pub const SUCCESS_TINT: Rgba = hex_alpha(0x34c759, 0.12);
/// The text of a button that removes something.
pub const DESTRUCTIVE_TEXT: Rgba = hex(0xff8a80);
/// The background of a message row (section 6.2).
pub const DESTRUCTIVE_TINT: Rgba = hex_alpha(0xff8a80, 0.12);
/// The line at the top of a message row (section 6.2).
pub const DESTRUCTIVE_LINE: Rgba = hex_alpha(0xff8a80, 0.35);
/// The icon of a warning, and a warning text in a gesture card.
pub const WARNING: Rgba = hex(0xf5b65c);
/// A control that the user operates on the device glows in this colour.
pub const INPUT_GLOW: Rgba = hex(0xffce73);

/// A button under the pointer: one step lighter than [`BG_CONTROL`].
pub const HOVER_CONTROL: Rgba = hex(0x383a40);
/// A transparent icon button under the pointer.
pub const HOVER_ICON: Rgba = hex_alpha(0xffffff, 0.06);
/// A row under the pointer.
pub const HOVER_ROW: Rgba = hex_alpha(0xffffff, 0.05);
/// A selected row in a pane that does not have the selection, for example
/// the active page row when the device is the selection.
pub const SELECTED_UNFOCUSED: Rgba = hex_alpha(0xffffff, 0.07);
/// A pressed button.
pub const PRESSED: Rgba = hex(0x42444b);
/// The background of the live state pill when no device is connected.
pub const NEUTRAL_TINT: Rgba = hex_alpha(0xffffff, 0.06);
/// The dashed border of the fill sample "Other colour" (section 10.2).
pub const BORDER_SAMPLE_OTHER: Rgba = hex(0x5a5c63);
/// The border of a toggle button that is on, for example "B" for bold
/// (section 10.2).
pub const BORDER_TOGGLE_ON: Rgba = BORDER_SAMPLE_OTHER;
/// The white knob of a switch, and the text on a kind tile.
pub const SWITCH_KNOB_COLOUR: Rgba = hex(0xffffff);
/// The tile of a kind of each group of section 20, and of a group that is
/// not in that table.
pub const KIND_TILE_SYSTEM: Rgba = hex(0x3a3f47);
pub const KIND_TILE_MEDIA: Rgba = hex(0x2350a8);
pub const KIND_TILE_NAVIGATION: Rgba = hex(0x1f6f6b);
pub const KIND_TILE_LOGIC: Rgba = hex(0x7a3e2e);
pub const KIND_TILE_OTHER: Rgba = hex(0x4b3b7a);
/// The tile of an action kind that has no handler (rule 8).
pub const KIND_TILE_UNAVAILABLE: Rgba = hex(0x3a3c42);

/// The tile colour of a kind of the group `group` (decision 0029, section
/// 20). The colour comes from the group, thus the window needs no colour for
/// each kind. A group of a plug-in gets [`KIND_TILE_OTHER`].
pub fn kind_tile(group: &str) -> Rgba {
    match group {
        "System" => KIND_TILE_SYSTEM,
        "Media" => KIND_TILE_MEDIA,
        "Navigation" => KIND_TILE_NAVIGATION,
        "Logic" => KIND_TILE_LOGIC,
        _ => KIND_TILE_OTHER,
    }
}

/// The body of the device.
pub const DEVICE_BODY: Rgba = hex(0x0b0c0e);
/// The border of the body of the device.
pub const DEVICE_BODY_BORDER: Rgba = hex(0x2e3036);
/// The display when the page has no background image.
pub const DEVICE_DISPLAY: Rgba = hex(0x0f1724);
/// The border of a key face and of a bar panel.
pub const DEVICE_SURFACE_BORDER: Rgba = hex_alpha(0xffffff, 0.06);
/// The inside of a knob.
pub const DEVICE_KNOB: Rgba = hex(0x1c1e23);
/// The ring of a knob.
pub const DEVICE_KNOB_RING: Rgba = hex(0x3a3c42);
/// The border of the "Touch bar" control.
pub const DEVICE_TOUCH_BAR_BORDER: Rgba = hex_alpha(0xffffff, 0.10);
/// The mark at the top of a knob.
pub const DEVICE_KNOB_DOT: Rgba = hex(0xececee);
/// The dashed border of an empty key face.
pub const DEVICE_KEY_EMPTY_BORDER: Rgba = hex(0x3a3f4c);
/// The plus icon on an empty key face.
pub const DEVICE_KEY_EMPTY_ICON: Rgba = hex(0x5a5f6b);
/// The "Touch bar" control under the pointer.
pub const DEVICE_TOUCH_BAR_HOVER: Rgba = hex_alpha(0xffffff, 0.10);
/// The outline of a control under the pointer: [`ACCENT_LINE`] at 50 %.
pub const ACCENT_LINE_HOVER: Rgba = hex_alpha(0x3b95ff, 0.5);

/// The shadow of the body of the device: 0 24 60, black at 45 %.
pub fn device_body_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hex_alpha(0x000000, 0.45).into(),
        offset: point(px(0.), px(24.)),
        blur_radius: px(60.),
        spread_radius: px(0.),
        inset: false,
    }]
}

/// The glow around a control that the user operates on the device.
pub fn input_glow_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hex_alpha(0xffce73, 0.55).into(),
        offset: point(px(0.), px(0.)),
        blur_radius: px(14.),
        spread_radius: px(1.),
        inset: false,
    }]
}

/// The shadow of a popover and of a menu that the window draws: 0 18 40,
/// black at 50 %.
pub fn popover_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hex_alpha(0x000000, 0.5).into(),
        offset: point(px(0.), px(18.)),
        blur_radius: px(40.),
        spread_radius: px(0.),
        inset: false,
    }]
}

// MARK: Text sizes (section 3.1)

/// The mark "ON DEVICE", and other small marks.
pub const TEXT_BADGE: Pixels = px(10.);
/// A number at the right of a row, a hint, a knob label.
pub const TEXT_CAPTION: Pixels = px(11.);
/// The status bar, help text, and the live state pill.
pub const TEXT_HELP: Pixels = px(11.5);
/// The labels of a form, small buttons, and segmented controls.
pub const TEXT_LABEL: Pixels = px(12.);
/// Text in a popup button, in a search field, in a step row.
pub const TEXT_CONTROL: Pixels = px(12.5);
/// Rows, text fields, and the toolbar title. The default size of the window.
pub const TEXT_BODY: Pixels = px(13.);
/// The name in the inspector header.
pub const TEXT_HEADING: Pixels = px(15.);
/// A section header, in capital letters, weight 600, for example "PROFILES"
/// in the sidebar. GPUI has no letter spacing, thus the header has the
/// normal spacing of the font.
pub const TEXT_SECTION: Pixels = px(11.);

// MARK: Spacing steps (section 3.3)
//
// Padding and gaps use only these steps.

pub const SPACE_1: Pixels = px(1.);
pub const SPACE_2: Pixels = px(2.);
pub const SPACE_3: Pixels = px(3.);
pub const SPACE_4: Pixels = px(4.);
pub const SPACE_6: Pixels = px(6.);
pub const SPACE_8: Pixels = px(8.);
pub const SPACE_10: Pixels = px(10.);
pub const SPACE_12: Pixels = px(12.);
pub const SPACE_14: Pixels = px(14.);
pub const SPACE_16: Pixels = px(16.);
pub const SPACE_18: Pixels = px(18.);
pub const SPACE_20: Pixels = px(20.);
pub const SPACE_24: Pixels = px(24.);
pub const SPACE_28: Pixels = px(28.);
pub const SPACE_32: Pixels = px(32.);

/// The gap between the toolbar items, and between the buttons in a row.
pub const GAP_ITEMS: Pixels = SPACE_6;
/// The padding of a section of the right panel: top, sides, bottom.
pub const SECTION_PADDING_TOP: Pixels = SPACE_14;
pub const SECTION_PADDING_SIDE: Pixels = SPACE_16;
pub const SECTION_PADDING_BOTTOM: Pixels = SPACE_16;
/// The space below a section header, and below a header with a hint.
pub const SECTION_HEADER_GAP: Pixels = SPACE_12;
pub const SECTION_HINT_GAP: Pixels = SPACE_4;
/// The gap between the columns of a form and between its rows; in a
/// gesture card.
pub const FORM_GAP: Pixels = SPACE_10;
pub const CARD_FORM_GAP: Pixels = SPACE_8;
/// The gap between two gesture cards.
pub const CARD_GAP: Pixels = SPACE_8;

// MARK: Corner radii (section 3.4)

/// A badge.
pub const RADIUS_BADGE: Pixels = px(4.);
/// A segment, and a colour sample.
pub const RADIUS_SEGMENT: Pixels = px(5.);
/// A button, a text field, a popup button, a sidebar row.
pub const RADIUS_CONTROL: Pixels = px(6.);
/// A segmented control track, a search field, and a library row.
pub const RADIUS_TRACK: Pixels = px(7.);
/// A nested card, and a bar panel on the canvas.
pub const RADIUS_CARD: Pixels = px(8.);
/// A gesture card, and the device card.
pub const RADIUS_DEVICE_CARD: Pixels = px(9.);
pub const RADIUS_GESTURE_CARD: Pixels = RADIUS_DEVICE_CARD;
/// A popover, a menu, a key face on the canvas and in the inspector header.
pub const RADIUS_POPOVER: Pixels = px(12.);

// MARK: Control heights (section 3.5)

/// A segment of the tab control, and a small button.
pub const HEIGHT_SEGMENT: Pixels = px(26.);
/// A button, a text field, a toolbar button, a sidebar row.
pub const HEIGHT_CONTROL: Pixels = px(28.);
/// The live state pill.
pub const HEIGHT_PILL: Pixels = px(24.);
/// The width of an icon button in the toolbar and in the sidebar top area.
pub const WIDTH_ICON_BUTTON: Pixels = px(30.);
/// The diameter of a state dot.
pub const DOT: Pixels = px(6.);
/// The space above the text of an empty state in a pane.
pub const EMPTY_STATE_TOP: Pixels = px(40.);
/// The label column of a form, and the gesture name of a gesture card.
pub const FORM_LABEL: Pixels = px(64.);
/// The label column of a form in the page and the device inspector.
pub const FORM_LABEL_WIDE: Pixels = px(72.);
/// The gesture name of a swipe card (section 10.6).
pub const GESTURE_NAME_WIDE: Pixels = px(84.);
/// The action popup of a gesture card.
pub const HEIGHT_ACTION_POPUP: Pixels = px(30.);
/// A gesture card with no action.
pub const HEIGHT_EMPTY_CARD: Pixels = px(40.);
/// A nested card with no action, and a closed nested card (section 10.5).
pub const HEIGHT_EMPTY_NESTED: Pixels = px(32.);
/// A text field of three lines.
pub const HEIGHT_LONG_TEXT: Pixels = px(60.);
/// The text field of an integer.
pub const WIDTH_INTEGER_FIELD: Pixels = px(56.);
/// The size of a switch (section 3.5): width, height, and the diameter of
/// its knob.
#[derive(Clone, Copy)]
pub struct SwitchSize {
    pub width: Pixels,
    pub height: Pixels,
    pub knob: Pixels,
}
/// A switch in the inspector.
pub const SWITCH_INSPECTOR: SwitchSize = SwitchSize {
    width: px(30.),
    height: px(18.),
    knob: px(14.),
};
/// A switch in the Settings window.
pub const SWITCH_SETTINGS: SwitchSize = SwitchSize {
    width: px(32.),
    height: px(19.),
    knob: px(15.),
};
/// The picture in the inspector header.
pub const HEADER_PICTURE: Pixels = px(64.);
/// The page picture in the header of the page inspector (section 10.6),
/// and the corner radius of a key face or a bar panel in it.
pub const PAGE_PICTURE_WIDTH: Pixels = px(72.);
pub const PAGE_PICTURE_HEIGHT: Pixels = px(44.);
pub const RADIUS_PAGE_PICTURE_ITEM: Pixels = px(2.);
/// The ring of the knob picture in the inspector header.
pub const HEADER_KNOB_RING: Pixels = px(4.);
/// The mark at the top of the knob picture in the inspector header.
pub const HEADER_KNOB_MARK: Pixels = px(6.);
/// A kind tile in a gesture card and in the kind menu, and in a nested card.
pub const KIND_TILE: Pixels = px(20.);
pub const KIND_TILE_NESTED: Pixels = px(18.);
/// A kind tile in a library row (section 11).
pub const KIND_TILE_LIBRARY: Pixels = px(24.);
/// A row of the library (section 11).
pub const HEIGHT_LIBRARY_ROW: Pixels = px(34.);
/// A search field of a panel, for example in the library (section 11).
pub const HEIGHT_SEARCH_FIELD: Pixels = px(30.);
/// The width of a menu that the window draws.
pub const MENU_WIDTH: Pixels = px(240.);
/// The highest menu. A longer menu, or a menu that is higher than the
/// window, scrolls. The kind menu with the group titles of the library is
/// approximately 465 high.
pub const MENU_MAX_HEIGHT: Pixels = px(520.);
/// The height of a row of a menu that the window draws.
pub const HEIGHT_MENU_ROW: Pixels = px(28.);
/// The space between a popup button and its menu, and between the menu and
/// the edge of the window.
pub const MENU_OFFSET: Pixels = SPACE_4;
pub const MENU_WINDOW_MARGIN: Pixels = SPACE_8;
/// The selection outline of a fill sample: its width, and the space
/// between the outline and the sample (section 3.7).
pub const SAMPLE_OUTLINE: Pixels = px(2.);
pub const SAMPLE_OUTLINE_GAP: Pixels = px(2.);
/// The popover of "Other colour", with its field for `#RRGGBB`.
pub const WIDTH_COLOUR_POPOVER: Pixels = px(200.);
/// A fill sample.
pub const SAMPLE: Pixels = px(22.);
/// The popup button of the label size, for example "20 pt", and its menu
/// (section 10.2).
pub const WIDTH_SIZE_POPUP: Pixels = px(76.);
pub const WIDTH_SIZE_MENU: Pixels = px(100.);
/// The icon tile of the device card.
pub const DEVICE_TILE: Pixels = px(32.);
/// The icon tile in the header of the device inspector (section 10.7).
pub const DEVICE_HEADER_TILE: Pixels = px(48.);
/// The corner radius of that tile.
pub const RADIUS_HEADER_TILE: Pixels = px(11.);
/// The device icon in that tile.
pub const ICON_HEADER_DEVICE: Pixels = px(26.);
/// A segment of a segmented control in an inspector.
pub const HEIGHT_SEGMENT_INSPECTOR: Pixels = px(22.);
/// The button of one knob light in the device inspector. The height is fixed,
/// so that the popover of the button has its place.
pub const HEIGHT_KNOB_LIGHT_BUTTON: Pixels = px(80.);
/// The ring of the knob light colour in that button.
pub const KNOB_LIGHT_RING: Pixels = px(26.);
/// The width of the colour of that ring.
pub const KNOB_LIGHT_RING_WIDTH: Pixels = px(4.);
/// The popover of a knob light: nine samples in one row, and a field.
pub const WIDTH_KNOB_POPOVER: Pixels = px(250.);
/// The line height of the note at the bottom of the device inspector.
pub const NOTE_LINE_HEIGHT: f32 = 1.45;
/// The widest title block in the toolbar. A longer page name ends with "…".
pub const TOOLBAR_TITLE_MAX: Pixels = px(220.);
/// The height of the page header at the top of the canvas (section 8.1).
pub const PAGE_HEADER_HEIGHT: Pixels = px(48.);
/// The "+" button of a section header of the sidebar.
pub const SIDEBAR_ADD_WIDTH: Pixels = px(24.);
pub const SIDEBAR_ADD_HEIGHT: Pixels = px(22.);
/// The text field that replaces a name in a sidebar row: its height, and
/// the space from its edge to its text (padding and border).
pub const ROW_FIELD_HEIGHT: Pixels = px(22.);
pub const ROW_FIELD_INSET: Pixels = px(5.);
/// The leading padding of a page row and of the "Add a page" row.
pub const SIDEBAR_PAGE_INDENT: Pixels = px(50.);
/// The width of a small square button, for example "Previous page" in the
/// page header.
pub const WIDTH_SQUARE_BUTTON: Pixels = px(28.);
/// The space around the device drawing in the canvas.
pub const DEVICE_MARGIN: f32 = 24.;

// MARK: Icon sizes (section 3.6)

/// An icon in a small button.
pub const ICON_SMALL: Pixels = px(12.);
/// An icon in a square button and in a message row.
pub const ICON_BUTTON: Pixels = px(14.);
/// The icon of a search field of a panel.
pub const ICON_SEARCH: Pixels = px(14.);
/// An icon in a message row.
pub const ICON_MESSAGE: Pixels = px(14.);
/// An icon in a sidebar row.
pub const ICON_ROW: Pixels = px(15.);
/// The icon of a toolbar button, for example Undo.
pub const ICON_TOOLBAR: Pixels = px(16.);
/// The icon of a pane button.
pub const ICON_PANE: Pixels = px(17.);
/// The device icon of the device card.
pub const ICON_DEVICE: Pixels = px(18.);
/// The "⋯" icon of the inspector header.
pub const ICON_MORE: Pixels = px(16.);
/// The kind icon in a kind tile of 20, and of 18.
pub const ICON_KIND: Pixels = px(13.);
pub const ICON_KIND_NESTED: Pixels = px(11.);
/// The play icon of "Test the action" (#188).
pub const ICON_PLAY: Pixels = px(11.);

// MARK: The window and the panes (section 4)

/// The height of the toolbar, and of the top area of the sidebar.
pub const TOOLBAR_HEIGHT: Pixels = px(52.);
/// The height of the status bar.
pub const STATUS_BAR_HEIGHT: Pixels = px(28.);
/// The leading padding of the toolbar when the sidebar is hidden: space for
/// the window buttons.
pub const TOOLBAR_PADDING_NO_SIDEBAR: Pixels = px(78.);
/// The point of the close button of the window, from the top-left corner.
/// The centre of the window buttons is then 26 below the top.
pub const WINDOW_BUTTONS: (Pixels, Pixels) = (px(18.), px(19.));

/// The width of the sidebar: default, minimum, maximum.
pub const SIDEBAR_WIDTH: f32 = 240.;
pub const SIDEBAR_MIN: f32 = 200.;
pub const SIDEBAR_MAX: f32 = 320.;
/// The width of the right panel: default, minimum, maximum.
pub const INSPECTOR_WIDTH: f32 = 360.;
pub const INSPECTOR_MIN: f32 = 320.;
pub const INSPECTOR_MAX: f32 = 480.;
/// The minimum width of the canvas.
pub const CANVAS_MIN: f32 = 480.;
/// The width of the pointer area of a divider between two panes.
pub const DIVIDER_AREA: Pixels = px(6.);

/// The default size of the window. A smaller screen gives its visible area.
pub const WINDOW_SIZE: (f32, f32) = (1440., 900.);
/// The minimum size of the window. Both side panes fit at their default
/// widths, with the canvas at its minimum width.
pub const WINDOW_MIN: (f32, f32) = (1080., 700.);

/// The device drawing uses a scale between these values (section 8.2).
pub const DEVICE_SCALE_MIN: f32 = 0.5;
pub const DEVICE_SCALE_MAX: f32 = 1.0;

// MARK: The device drawing (section 8.2)
//
// The values in pixels of the display. The canvas multiplies each one by
// the scale of the drawing.

/// The margin of the body at the left, the right, and the top of the
/// display. With no knob, also below the display.
pub const DEVICE_BODY_MARGIN: f32 = 28.;
/// The margin of the body below the knob labels.
pub const DEVICE_BODY_BOTTOM: f32 = 44.;
/// The corner radius of the body.
pub const DEVICE_BODY_RADIUS: f32 = 33.;
/// The corner radius of the display.
pub const DEVICE_DISPLAY_RADIUS: f32 = 12.;
/// The corner radius of a key face.
pub const DEVICE_KEY_RADIUS: f32 = 14.;
/// The corner radius of a bar panel.
pub const DEVICE_PANEL_RADIUS: f32 = 10.;
/// The size of the plus icon on an empty key face.
pub const DEVICE_KEY_EMPTY_ICON_SIZE: f32 = 28.;
/// The diameter of a knob.
pub const DEVICE_KNOB_DIAMETER: f32 = 90.;
/// The space between the display and the top of a knob.
pub const DEVICE_KNOB_TOP: f32 = 22.;
/// The width of the ring of a knob.
pub const DEVICE_KNOB_RING_WIDTH: f32 = 4.;
/// The diameter of the mark at the top of a knob.
pub const DEVICE_KNOB_MARK: f32 = 7.;
/// The space between the mark and the ring of a knob.
pub const DEVICE_KNOB_MARK_GAP: f32 = 5.;
/// The space between a knob and its label.
pub const DEVICE_KNOB_LABEL_GAP: f32 = 8.;

// These values are in points. They do not change with the scale.

/// The line height of a knob label.
pub const DEVICE_KNOB_LABEL_HEIGHT: f32 = 14.;
/// The width of the box of a knob label.
pub const DEVICE_KNOB_LABEL_WIDTH: Pixels = px(80.);
/// The "◀ Touch bar ▶" control: height and corner radius.
pub const DEVICE_TOUCH_BAR_HEIGHT: Pixels = px(22.);
pub const DEVICE_TOUCH_BAR_RADIUS: Pixels = px(11.);
/// The diameter of the mark of the touch point.
pub const DEVICE_TOUCH_POINT: f32 = 12.;
/// The width of the outline of a selected control.
pub const OUTLINE_WIDTH: f32 = 2.;
/// The space between a control and its outline: for a key face, a bar
/// panel, and a hover outline.
pub const OUTLINE_GAP: f32 = 3.;
/// The space between a knob or the body and its outline.
pub const OUTLINE_GAP_ROUND: f32 = 4.;
/// The gap between the device drawing and the help line (section 8.2).
pub const DEVICE_AREA_GAP: Pixels = SPACE_18;
/// The line height of the help line below the device.
pub const HELP_LINE_HEIGHT: f32 = 16.;

// MARK: The Settings window (section 17)

/// The title of a sheet, and the name of the application in the pane "About".
pub const TEXT_SHEET_TITLE: Pixels = px(17.);
/// A group box of the Settings window.
pub const RADIUS_GROUP: Pixels = px(10.);
/// The width of the Settings window. Its height follows its pane.
pub const SETTINGS_WIDTH: f32 = 640.;
/// The height of the title bar of the Settings window, with the window
/// buttons and the name of the pane.
pub const SETTINGS_TITLE_HEIGHT: Pixels = px(34.);
/// The point of the close button of the Settings window, from the top-left
/// corner. The centre of the window buttons is then in the middle of the
/// title bar.
pub const SETTINGS_WINDOW_BUTTONS: (Pixels, Pixels) = (px(13.), px(10.));
/// A tab of the Settings window.
pub const SETTINGS_TAB: (Pixels, Pixels) = (px(78.), px(50.));
/// The icon of a tab of the Settings window.
pub const ICON_TAB: Pixels = px(20.);
/// The padding of the content of a pane: top and bottom, and the sides.
pub const SETTINGS_CONTENT_Y: Pixels = SPACE_18;
pub const SETTINGS_CONTENT_X: Pixels = px(40.);
/// The application icon in the pane "About".
pub const APP_ICON: Pixels = px(64.);
/// The height of the Settings window for each pane, in points: the title
/// bar, the row of tabs, and the content of the pane with its padding. The
/// content of a pane has fixed rows, thus the heights are constants. The
/// pane "Shortcuts" adds one row height for each key command.
pub const SETTINGS_HEIGHT_GENERAL: f32 = 480.;
pub const SETTINGS_HEIGHT_SHORTCUTS: f32 = 132.;
pub const SETTINGS_SHORTCUT_ROW: f32 = 37.;
/// The pane "About" with the notices file, and with the text that tells
/// where the notices are.
pub const SETTINGS_HEIGHT_ABOUT: f32 = 300.;
pub const SETTINGS_HEIGHT_ABOUT_NO_NOTICES: f32 = 332.;
/// The height that a message adds below the content of the pane, and the
/// highest text of the message (two lines).
pub const SETTINGS_MESSAGE_HEIGHT: f32 = 52.;
pub const SETTINGS_MESSAGE_TEXT_MAX: Pixels = px(32.);
/// The line height of the Settings window, as a multiple of the text size.
/// Each row has one or two lines, so the window does not use a taller
/// line height.
pub const SETTINGS_LINE_HEIGHT: f32 = 1.25;

// MARK: Icons (section 3.6)

// The window embeds only the icons in this list. A new icon in the window
// needs a new name here: a missing icon shows nothing.
gpui_kit::assets::icon_assets!(pub WindowIcons, [
    Check,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronsUpDown,
    Copy,
    Ellipsis,
    Grid3x2,
    Info,
    Keyboard,
    Layers,
    PanelLeft,
    PanelRight,
    Pencil,
    Play,
    Plus,
    Redo2,
    Search,
    Settings,
    ShieldAlert,
    ShieldCheck,
    Trash,
    TriangleAlert,
    Undo2,
]);
