//! The visual constants of the window.
//!
//! Read `docs/DESIGN.md` before you change the window. It gives each token of
//! this module and when to use it.
//!
//! This module is the one place for a colour, a text size, a spacing step, a
//! corner radius, a pane width, a control height, and a colour sample of the
//! data. The window code uses these names and writes no number for them: the
//! test `no_literal_colour_or_size_outside_the_theme` fails for a literal
//! colour or `px(..)` size in other window code. A new value gets a name here,
//! with a comment that says where it is used.
//!
//! The values are for the dark appearance. A light appearance is a second
//! set of values for the same names (decision 0029, section 21).

use gpui_kit::{point, px, BoxShadow, Pixels, Rgba};
use opencraft_engine::Color;

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

/// A colour of the engine (a fill, a knob light, a profile colour) as a
/// colour of the window. All window code uses this one conversion.
pub fn colour(colour: Color) -> Rgba {
    let [r, g, b] = colour.0;
    hex(u32::from_be_bytes([0, r, g, b]))
}

/// A colour sample of the data (`0xRRGGBB`) as a colour of the window.
pub const fn sample(value: u32) -> Rgba {
    hex(value)
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
/// A link under the pointer.
pub const ACCENT_TEXT_HOVER: Rgba = hex(0x7dbbff);
/// The accent at 14 %: the background of a selected card.
pub const ACCENT_TINT: Rgba = hex_alpha(0x1a6fd8, 0.14);
/// The accent at 18 %: a row under a drag (section 3.2).
pub const ACCENT_TINT_DRAG: Rgba = hex_alpha(0x1a6fd8, 0.18);
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
/// No colour: a border that shows only in a different state.
pub const TRANSPARENT: Rgba = hex_alpha(0x000000, 0.);
/// A row under the pointer.
pub const HOVER_ROW: Rgba = hex_alpha(0xffffff, 0.05);
/// The highlighted item of a menu that the window draws: the item that a key
/// selects, and the item under the pointer.
pub const MENU_HIGHLIGHT: Rgba = hex_alpha(0x1a6fd8, 0.32);
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

/// The shadow of the drag picture: 0 12 28, black at 50 % (section 3.2).
pub fn drag_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hex_alpha(0x000000, 0.5).into(),
        offset: point(px(0.), px(12.)),
        blur_radius: px(28.),
        spread_radius: px(0.),
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

/// The width of the focus ring of a control that has the keyboard focus
/// (decision 0029, section 3.7: "outline 2 `accent-line`").
pub const FOCUS_RING: Pixels = px(2.);
/// The space between the text of a link and its focus ring.
pub const LINK_FOCUS_PADDING: Pixels = SPACE_2;
/// The space between a slider and its focus ring: at the sides, the space
/// for the thumb at the ends of the track.
pub const SLIDER_FOCUS_PADDING_X: Pixels = SPACE_8;
pub const SLIDER_FOCUS_PADDING_Y: Pixels = SPACE_4;

/// The focus ring outside the edge of a control with a fill, for example a
/// button or a switch (`controls::Ring::Outside`). GPUI has no outline:
/// a shadow with no blur draws the ring. The fill of the control covers the
/// part of the shadow below it.
pub fn focus_ring_outside() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: ACCENT_LINE.into(),
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: FOCUS_RING,
        inset: false,
    }]
}

/// The focus ring inside the edge of a control with no fill, for example
/// an icon button or a row (`controls::Ring::Inside`). A shadow outside the
/// edge would fill all of a transparent control.
pub fn focus_ring_inside() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: ACCENT_LINE.into(),
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: FOCUS_RING,
        inset: true,
    }]
}

// MARK: Colour samples that are data
//
// The profile keeps the colour that the user selects. Do not change a stored
// value: a profile with the old value then shows no selected sample.

/// The fill colours of a surface (section 10.2) and of a page background.
pub const FILL_COLOURS: [u32; 12] = [
    0x20242b, 0x596273, 0xd7dee8, 0xb83f4c, 0xcc7135, 0xc4a53e, 0x3d875a, 0x298886, 0x356ab6,
    0x6454ac, 0x9357a3, 0xb65c86,
];
/// The text colours of the popover of the title colour, before the fill
/// colours: white and black.
pub const TITLE_COLOURS: [u32; 2] = [0xffffff, 0x000000];
/// The colours of a knob light. A knob light shows a pure colour best.
pub const KNOB_COLOURS: [u32; 9] = [
    0xffffff, 0xff0000, 0xff8000, 0xffff00, 0x00ff00, 0x00ffff, 0x0000ff, 0xff00ff, 0x000000,
];

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
/// The kind button in the head of an open nested card: the tile and the
/// kind name, which open the kind menu (section 10.5).
pub const HEIGHT_NESTED_KIND: Pixels = px(22.);
/// A row of a step list (section 10.5).
pub const HEIGHT_STEP_ROW: Pixels = px(34.);
/// The button "Add a step" (section 10.5).
pub const HEIGHT_ADD_STEP: Pixels = px(24.);
/// The column of the step number in a step list.
pub const WIDTH_STEP_NUMBER: Pixels = px(16.);
/// The drag handle of a step: its width and its height (six dots).
pub const WIDTH_DRAG_HANDLE: Pixels = px(12.);
pub const HEIGHT_DRAG_HANDLE: Pixels = px(14.);
/// The vertical padding of the drag picture (section 12).
pub const DRAG_PICTURE_PADDING_Y: Pixels = px(7.);
/// The kind tile in the drag picture (section 12).
pub const KIND_TILE_DRAG: Pixels = px(22.);
/// A face button of the appearance of each state (section 10.5): the
/// gap between the two buttons, the radius of a button, the face picture,
/// and its radius.
pub const STATE_FACE_GAP: Pixels = SPACE_14;
pub const RADIUS_STATE_BUTTON: Pixels = px(10.);
pub const STATE_FACE: Pixels = px(72.);
pub const RADIUS_STATE_FACE: Pixels = px(11.);
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
/// The preview of the background image in the page inspector (section
/// 10.6), and the space from its edges to the size text.
pub const BACKGROUND_PREVIEW_WIDTH: Pixels = px(140.);
pub const BACKGROUND_PREVIEW_HEIGHT: Pixels = px(84.);
pub const BACKGROUND_BADGE_INSET: Pixels = SPACE_6;
/// The backing of the size text on a background image.
pub const BADGE_ON_IMAGE: Rgba = hex_alpha(0x000000, 0.55);
/// The menu of the fit of the background image.
pub const WIDTH_FIT_MENU: Pixels = px(140.);
/// The device inactivity popup button and its menu.
pub const WIDTH_SCREEN_OFF_MENU: Pixels = px(160.);
/// The ring of the knob picture in the inspector header.
pub const HEADER_KNOB_RING: Pixels = px(4.);
/// The mark at the top of the knob picture in the inspector header.
pub const HEADER_KNOB_MARK: Pixels = px(6.);
/// The width of the image of the bar panel in the "Bar panel" section of
/// the knob inspector (section 10.4). The height follows the bar panel: 94
/// for the bar panels of the N4 Pro.
pub const KNOB_PANEL_FACE: Pixels = px(148.);
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
/// The line height of a text of more than one line (section 3.1): a note, a
/// help text that wraps, the box about the permission, the description of a
/// starter profile card, and the text of a sheet.
pub const LINE_HEIGHT_TEXT: f32 = 1.45;
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
/// An application icon in the application picker and in its field.
pub const ICON_APPLICATION: Pixels = px(16.);
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

/// The highest height of the messages above the status bar (section 6.2).
/// More messages scroll.
pub const MESSAGES_HEIGHT: f32 = 120.;

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
/// The width of the outline of a control under the pointer.
pub const HOVER_OUTLINE_WIDTH: f32 = 1.;
/// The space between a control and its outline: for a key face, a bar
/// panel, and a hover outline.
pub const OUTLINE_GAP: f32 = 3.;
/// The space between a knob or the body and its outline.
pub const OUTLINE_GAP_ROUND: f32 = 4.;
/// The gap between the device drawing and the help line (section 8.2).
pub const DEVICE_AREA_GAP: Pixels = SPACE_18;
/// The line height of the help line below the device.
pub const HELP_LINE_HEIGHT: f32 = 16.;

// MARK: Drag and drop (sections 11 and 12)

/// The drop target on a control: the accent at 28 %, on top of the image.
/// A library row in a drag has [`ACCENT_TINT_DRAG`].
pub const ACCENT_TINT_DROP: Rgba = hex_alpha(0x1a6fd8, 0.28);
/// The width of the dashed border of the drop target.
pub const DROP_TARGET_BORDER: Pixels = px(2.);
/// The top-left corner of the drag picture, from the pointer. The picture
/// hangs below the pointer, so that the pointer and the text "Drop here" of
/// the target stay visible.
pub const DRAG_PICTURE_OFFSET: (Pixels, Pixels) = (px(4.), px(14.));

// MARK: The Settings window (section 17)

/// The title of a sheet, and the name of the application in the pane "About".
pub const TEXT_SHEET_TITLE: Pixels = px(17.);
/// A group box of the Settings window.
pub const RADIUS_GROUP: Pixels = px(10.);
/// The width of the Settings window. Its height follows the content of its
/// pane.
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
/// The height of the title area of the Settings window: the title bar, the
/// row of tabs with its padding, and the line below. The window of the
/// measurement has this height, and a window that cannot measure its content
/// opens with it.
pub fn settings_header_height() -> Pixels {
    SETTINGS_TITLE_HEIGHT + SETTINGS_TAB.1 + SPACE_8 + BORDER
}
/// The icon of a tab of the Settings window.
pub const ICON_TAB: Pixels = px(20.);
/// The padding of the content of a pane: top and bottom, and the sides.
pub const SETTINGS_CONTENT_Y: Pixels = SPACE_18;
pub const SETTINGS_CONTENT_X: Pixels = px(40.);
/// The application icon in the pane "About".
pub const APP_ICON: Pixels = px(64.);
/// The padding above and below the text of the message below a pane. The
/// height of the message follows its text (at most two lines, see
/// [`SETTINGS_MESSAGE_TEXT_MAX`]).
pub const SETTINGS_MESSAGE_PADDING: Pixels = SPACE_10;
/// The highest text of the message below a pane (two lines).
pub const SETTINGS_MESSAGE_TEXT_MAX: Pixels = px(32.);
/// The longest time that the Settings window waits for the platform to
/// change its height before it shows the new pane. The platform does this in
/// a few milliseconds. This time covers a platform that refuses the height.
pub const SETTINGS_RESIZE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);
/// The largest difference between the height of the content of the Settings
/// window and the height of the window that counts as equal, in points.
pub const SETTINGS_HEIGHT_TOLERANCE: Pixels = px(0.5);
/// The line height of the Settings window, as a multiple of the text size.
/// Each row has one or two lines, so the window does not use a taller
/// line height.
pub const SETTINGS_LINE_HEIGHT: f32 = 1.25;

// MARK: Sheets and the modal layer (sections 15, 16, and 18)

/// The layer that covers the window behind a sheet or the command palette:
/// black at 25 %.
pub const MODAL_DIM: Rgba = hex_alpha(0x000000, 0.25);
/// The corner radius of a sheet and of the command palette.
pub const RADIUS_SHEET: Pixels = px(14.);
/// The space from the top of the window to the top of a sheet: the sheet
/// shows below the toolbar, as a sheet of macOS does.
pub const SHEET_TOP: Pixels = px(76.);
/// The space from the bottom of the window to the bottom of a sheet at the
/// smallest window height.
pub const SHEET_BOTTOM: Pixels = SPACE_24;
/// The width of the profile settings sheet (section 16).
pub const PROFILE_SHEET_WIDTH: Pixels = px(600.);
/// The label column of the form of a sheet.
pub const SHEET_LABEL: Pixels = px(110.);
/// A text field, a popup button, and a button of the button row of a
/// sheet.
pub const HEIGHT_SHEET_CONTROL: Pixels = px(30.);
/// The popup button "First page" of the profile settings sheet.
pub const WIDTH_SHEET_POPUP: Pixels = px(200.);
/// A colour circle of a profile (section 16), and of the icon picker
/// (section 14).
pub const COLOUR_CIRCLE: Pixels = px(20.);
/// The border of the colour circle "None".
pub const BORDER_NO_COLOUR: Rgba = BORDER_SAMPLE_OTHER;
/// The colours of a profile (section 16). These are data: the profile keeps
/// the colour that the user selects.
pub const PROFILE_COLOURS: [u32; 6] = [0x3b82f6, 0x14b8a6, 0x22c55e, 0xf59e0b, 0xef4444, 0xa855f7];

// MARK: The command palette and the toolbar search field (sections 5 and 15)

/// The size of the command palette, with its border, and the space from
/// the top of the window to the palette.
pub const PALETTE_WIDTH: Pixels = px(680.);
pub const PALETTE_HEIGHT: Pixels = px(460.);
pub const PALETTE_TOP: Pixels = px(120.);
/// The width of a border of 1 (`border_1`). A field inside a frame with a
/// border is two of these less high than the frame.
pub const BORDER: Pixels = px(1.);
/// The width of the border of the palette, of a sheet, and of a popover.
pub const BORDER_OVERLAY_WIDTH: Pixels = BORDER;
/// The search row at the top of the palette, and the text field in it.
pub const HEIGHT_PALETTE_SEARCH: Pixels = px(56.);
pub const HEIGHT_PALETTE_FIELD: Pixels = px(36.);
/// The search icon of the palette.
pub const ICON_PALETTE_SEARCH: Pixels = px(18.);
/// A row of the palette, and the footer of the palette.
pub const HEIGHT_PALETTE_ROW: Pixels = px(36.);
pub const HEIGHT_PALETTE_FOOTER: Pixels = px(36.);
/// The tile of a row of the palette, its corner radius, and its icon.
pub const PALETTE_TILE: Pixels = px(24.);
pub const RADIUS_PALETTE_TILE: Pixels = RADIUS_CONTROL;
pub const ICON_PALETTE_TILE: Pixels = px(14.);
/// A row of the palette.
pub const RADIUS_PALETTE_ROW: Pixels = RADIUS_CARD;
/// The tile of the selected row: white at 18 %.
pub const TILE_ON_ACCENT: Rgba = hex_alpha(0xffffff, 0.18);
/// The chip with the selected control, for example "Key 8 selected".
pub const BG_CONTEXT_CHIP: Rgba = hex(0x34363c);
/// The search field of the toolbar: a button that looks like a search
/// field.
pub const WIDTH_TOOLBAR_SEARCH: Pixels = px(240.);
/// The side padding of a key hint, for example "⌘K" (padding 1 5).
pub const KEY_HINT_PADDING_X: Pixels = px(5.);

/// The shadow of a sheet and of the command palette: 0 24 60, black at 55 %.
pub fn sheet_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hex_alpha(0x000000, 0.55).into(),
        offset: point(px(0.), px(24.)),
        blur_radius: px(60.),
        spread_radius: px(0.),
        inset: false,
    }]
}

// MARK: The icon picker (section 14)

/// The popover of the icon picker: its size.
pub const ICON_PICKER_WIDTH: Pixels = px(360.);
pub const ICON_PICKER_HEIGHT: Pixels = px(550.);
/// The arrow of the popover: a triangle of 22 × 11, the half of a square of
/// 16 turned by 45°.
pub const ICON_PICKER_ARROW_WIDTH: f32 = 22.;
pub const ICON_PICKER_ARROW_HEIGHT: f32 = 11.;
/// The smallest height of the popover in a low window. The grid then
/// scrolls in less space.
pub const ICON_PICKER_MIN_HEIGHT: Pixels = px(300.);
/// The space between the popover and the edge of the window.
pub const ICON_PICKER_WINDOW_MARGIN: Pixels = SPACE_8;

/// The columns of the icon grid, the height of a cell, and the icon in a
/// cell.
pub const ICON_GRID_COLUMNS: usize = 7;
pub const ICON_CELL_HEIGHT: Pixels = px(40.);
pub const ICON_CELL_ICON: Pixels = px(20.);
/// An icon in a cell of the icon grid.
pub const TEXT_ICON_CELL: Rgba = hex(0xd0d2d8);
/// A cell of the icon grid under the pointer: one step lighter than
/// [`BG_BAR`], with the border of a button.
pub const HOVER_ICON_CELL: Rgba = BG_CONTROL;
/// The height of a filter chip, for example the tab "Icons" (section 3.5),
/// and its corner radius.
pub const HEIGHT_CHIP: Pixels = px(24.);
pub const RADIUS_CHIP: Pixels = px(12.);
/// The accent at 25 %: the background of a selected chip.
pub const ACCENT_TINT_CHIP: Rgba = hex_alpha(0x1a6fd8, 0.25);
/// The button "Image file…" of the icon picker.
pub const BG_IMAGE_BUTTON: Rgba = BG_CONTEXT_CHIP;
/// The colours of an icon (section 14). These are data: the profile keeps
/// the colour that the user selects. The second colour has a border, so
/// that it shows on the popover.
pub const ICON_COLOURS: [u32; 6] = [0xffffff, 0x111214, 0xf59e0b, 0x22c55e, 0xff6b6b, 0x60a5fa];
pub const ICON_COLOUR_WITH_BORDER: u32 = 0x111214;
/// The number of icons in the tab "Recent".
pub const RECENT_ICONS: usize = 21;
/// The tile of the row "Icon" in "Appearance" (section 10.2), and the icon
/// in it.
pub const ICON_TILE: Pixels = px(28.);
pub const ICON_TILE_ICON: Pixels = px(18.);
/// The kind icon in a kind tile of 24 (a library row) and of 22 (the drag
/// picture).
pub const ICON_KIND_LIBRARY: Pixels = px(14.);
pub const ICON_KIND_DRAG: Pixels = px(13.);

// MARK: The application picker

/// The popover of the application picker. It aligns to the trailing edge
/// of the button "Choose…".
pub const APPLICATION_PICKER_WIDTH: Pixels = px(320.);
/// The height of the popover when the window has room for it.
pub const APPLICATION_PICKER_HEIGHT: Pixels = px(360.);
/// The smallest height of the popover in a low window. The list then
/// scrolls in less space.
pub const APPLICATION_PICKER_MIN_HEIGHT: Pixels = px(220.);

// MARK: The first start window and the Accessibility request (section 18)

/// The title of the first start window.
pub const TEXT_DISPLAY: Pixels = px(22.);
/// The text "*model name* found" and "Connect your device".
pub const TEXT_DEVICE_STATE: Pixels = px(14.);
/// The size of the first start window. The user cannot change it.
pub const FIRST_START_SIZE: (f32, f32) = (800., 560.);
/// The title bar of the first start window, with the window buttons only.
pub const FIRST_START_TITLE_HEIGHT: Pixels = px(40.);
/// The point of the close button, from the top-left corner. The centre of
/// the window buttons is then in the middle of the title bar.
pub const FIRST_START_WINDOW_BUTTONS: (Pixels, Pixels) = (px(16.), px(13.));
/// The left column with the device picture.
pub const FIRST_START_LEFT: Pixels = px(300.);
/// The widest text below the device picture.
pub const FIRST_START_LEFT_TEXT: Pixels = px(240.);
/// The padding of the right column: top, sides, bottom.
pub const FIRST_START_RIGHT_TOP: Pixels = SPACE_28;
pub const FIRST_START_RIGHT_SIDE: Pixels = SPACE_32;
pub const FIRST_START_RIGHT_BOTTOM: Pixels = px(22.);
/// The line height of the text below the title.
pub const FIRST_START_LINE_HEIGHT: f32 = 1.5;
/// The small device picture: its size, corner radius, and padding.
pub const PICTURE_SIZE: (Pixels, Pixels) = (px(220.), px(170.));
pub const RADIUS_PICTURE: Pixels = px(16.);
pub const PICTURE_PADDING: Pixels = SPACE_12;
/// A key tile of the device picture. The tile is square, because it shows
/// the image of the key face (rule 11). The record gives 28 high in a column
/// of 34; the square of the column fits in the picture.
pub const PICTURE_KEY: Pixels = px(34.);
pub const RADIUS_PICTURE_KEY: Pixels = px(5.);
/// A bar panel tile of the device picture: the height for the proportion
/// of the bar panel image in its column.
pub const PICTURE_PANEL_HEIGHT: Pixels = px(28.);
pub const RADIUS_PICTURE_PANEL: Pixels = px(4.);
/// A bar panel tile with no content.
pub const PICTURE_PANEL_EMPTY: Rgba = hex(0x1a1f2a);
/// A knob ring of the device picture: the diameter and the ring width.
pub const PICTURE_KNOB: Pixels = px(22.);
pub const PICTURE_KNOB_RING: Pixels = px(3.);
/// The device picture with no device.
pub const PICTURE_NO_DEVICE_OPACITY: f32 = 0.5;
/// The check badge of the device picture: the diameter, its border, the
/// check icon, and the space out of the picture.
pub const BADGE_CHECK: Pixels = px(28.);
pub const BADGE_CHECK_BORDER: Pixels = px(3.);
pub const ICON_BADGE_CHECK: Pixels = px(14.);
pub const BADGE_CHECK_OUTSIDE: Pixels = px(-10.);
/// The main button under the pointer, and pressed: the opacity of the
/// accent fill.
pub const MAIN_HOVER_OPACITY: f32 = 0.9;
pub const MAIN_PRESSED_OPACITY: f32 = 0.8;
/// The check badge of the first start.
pub const SUCCESS_FILL: Rgba = hex(0x1e7a46);
/// A starter profile card: its corner radius, and the background of the
/// selected card (the accent at 16 %).
pub const RADIUS_STARTER_CARD: Pixels = px(10.);
pub const ACCENT_TINT_START: Rgba = hex_alpha(0x1a6fd8, 0.16);
/// A cell of the key picture of a starter profile card, and its radius. A
/// cell shows the fill colour of its key only (decision 0029, section 18).
pub const STARTER_CELL: Pixels = px(14.);
pub const RADIUS_STARTER_CELL: Pixels = px(3.);
/// The box about the Accessibility permission: its corner radius, and the
/// shield icon.
pub const RADIUS_NOTE_BOX: Pixels = px(9.);
pub const ICON_NOTE: Pixels = px(16.);
/// The width of the Accessibility request sheet.
pub const PERMISSION_SHEET_WIDTH: Pixels = px(480.);
/// The shield icon of the Accessibility request, and its tile.
pub const ICON_PERMISSION: Pixels = px(20.);
pub const PERMISSION_TILE: Pixels = px(36.);

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
    CircleDot,
    ClipboardPaste,
    Copy,
    CopyPlus,
    Ellipsis,
    Eraser,
    GripVertical,
    Grid3x2,
    Info,
    Keyboard,
    Layers,
    PanelLeft,
    PanelRight,
    PanelTop,
    Pencil,
    Play,
    Plus,
    Power,
    RectangleHorizontal,
    Redo2,
    Search,
    Settings,
    Shield,
    ShieldAlert,
    ShieldCheck,
    Square,
    Trash,
    TriangleAlert,
    Undo2,
]);

/// The asset source of the window: the icons of [`WindowIcons`], and each
/// icon of the icon set of the engine at the path of [`surface_icon_path`]
/// (decision 0033). The icon picker and the kind tiles show the icons of the
/// engine, so the window and the key faces use the same drawings, and the
/// executable holds them one time.
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowAssets;

/// The start of the asset path of an icon of the engine.
const SURFACE_ICON_PREFIX: &str = "surface-icons/";

/// The asset path of the icon `name` of the icon set of the engine.
pub fn surface_icon_path(name: &str) -> gpui_kit::SharedString {
    format!("{SURFACE_ICON_PREFIX}{name}.svg").into()
}

impl gpui_kit::AssetSource for WindowAssets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if let Some(name) = path
            .strip_prefix(SURFACE_ICON_PREFIX)
            .and_then(|p| p.strip_suffix(".svg"))
        {
            let svg = opencraft_engine::icon_set().get(name).map(|i| i.svg());
            return Ok(svg.map(|svg| std::borrow::Cow::Owned(svg.into_bytes())));
        }
        WindowIcons.load(path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<gpui_kit::SharedString>> {
        WindowIcons.list(path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    /// The window code: the main window (`view`), the Settings window, and
    /// the first start window, with no test code.
    fn window_files() -> Vec<PathBuf> {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        let mut dirs = vec![
            src.join("view"),
            src.join("settings_window"),
            src.join("first_start"),
        ];
        for name in ["view.rs", "settings_window.rs", "first_start.rs"] {
            files.push(src.join(name));
        }
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if name == "tests" || name == "tests.rs" || name == "theme.rs" {
                    continue;
                }
                if path.is_dir() {
                    dirs.push(path);
                } else if name.ends_with(".rs") {
                    files.push(path);
                }
            }
        }
        files
    }

    /// The literal colours and sizes in `code`: a colour function, a colour
    /// value `0xRRGGBB`, or `px(..)` of a number other than 0.
    fn literals(code: &str) -> Vec<String> {
        let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let mut found = Vec::new();
        for (number, line) in code.lines().enumerate() {
            let line = line.split("//").next().unwrap_or("");
            for pattern in [
                "rgb(",
                "rgba(",
                "hsla(",
                "hex(",
                "hex_alpha(",
                "Rgba { r",
                "px(",
            ] {
                for (at, _) in line.match_indices(pattern) {
                    if line[..at].chars().next_back().is_some_and(ident) {
                        continue;
                    }
                    // A function of a number: a literal value.
                    let rest = line[at + pattern.len()..].trim_start_matches('-');
                    let literal =
                        rest.starts_with(|c: char| c.is_ascii_digit()) || pattern == "Rgba { r";
                    let zero = rest.starts_with("0.)") || rest.starts_with("0.0)");
                    if literal && !(pattern == "px(" && zero) {
                        found.push(format!("{}: {}", number + 1, line.trim()));
                    }
                }
            }
            for (at, _) in line.match_indices("0x") {
                let digits = line[at + 2..]
                    .chars()
                    .take_while(|c| c.is_ascii_hexdigit())
                    .count();
                if digits == 6 && !line[..at].chars().next_back().is_some_and(ident) {
                    found.push(format!("{}: {}", number + 1, line.trim()));
                }
            }
        }
        found
    }

    /// `docs/DESIGN.md`, "Tokens": a colour or a size of the window has one
    /// name in this module. Window code outside it writes no literal colour
    /// and no `px(..)` of a number.
    #[test]
    fn no_literal_colour_or_size_outside_the_theme() {
        let mut found = Vec::new();
        for path in window_files() {
            let code = std::fs::read_to_string(&path).unwrap();
            // The unit tests at the end of a file can use literal values.
            let code = code.split("#[cfg(test)]").next().unwrap_or("");
            for item in literals(code) {
                found.push(format!("{}:{item}", path.display()));
            }
        }
        assert!(
            found.is_empty(),
            "Use a name of `view/theme.rs` (docs/DESIGN.md):\n{}",
            found.join("\n")
        );
    }

    #[test]
    fn the_check_finds_literal_colours_and_sizes() {
        assert_eq!(literals("d.bg(gpui_kit::rgb(0x112233))").len(), 2);
        assert_eq!(literals(".w(px(12.))").len(), 1);
        assert_eq!(literals(".top(px(-4.))").len(), 1);
        assert!(literals(".w(px(0.)).bg(theme::ACCENT).h(px(theme::X))").is_empty());
        assert!(literals("images::from_rgb(&image); theme::sample(value)").is_empty());
        assert!(literals("Color::rgb(r, g, b); fn f() -> Rgba {").is_empty());
        assert!(literals("// rgb(0x112233) in a comment").is_empty());
    }
}
