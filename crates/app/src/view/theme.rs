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
//! A colour token is a function, for example `bg_canvas()`. Its value comes
//! from the palette of the appearance of the thread: the same name gives the
//! dark value in the dark appearance and the light value in the light
//! appearance (decision 0029, section 21). A new colour token also gets its
//! value in each palette, else the module does not build. A size token is a
//! constant, because its value does not change with the appearance.
//!
//! `crate::appearance` sets the appearance of the thread from the choice of
//! Settings > General (System, Light, or Dark) and from the appearance that
//! macOS gives the window. A test uses [`set_appearance_for_test`], which
//! puts the appearance back at the end of its scope.

use gpui_kit::{point, px, BoxShadow, Pixels, Rgba};
use opencraft_engine::Color;
use std::cell::Cell;

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

// MARK: Appearance
//
// A colour token is a function: its value is the value of the token in the
// appearance of the thread. The window code writes `theme::bg_canvas()`,
// never the value.

/// The appearance of the window.
///
/// A window has one appearance. The values of each colour token in the two
/// appearances are the palettes [`DARK`] and [`LIGHT`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    /// The dark appearance.
    Dark,
    /// The light appearance.
    Light,
}

thread_local! {
    /// The appearance of this thread. GPUI draws a window on one thread.
    /// Each test thread has its own appearance, thus a test that changes
    /// the appearance cannot change the colours of another test.
    static APPEARANCE: Cell<Appearance> = const { Cell::new(Appearance::Dark) };
}

/// The appearance of the window of this thread.
pub fn appearance() -> Appearance {
    APPEARANCE.with(Cell::get)
}

/// Set the appearance of the window of this thread. The window changes it
/// when the user selects an appearance, and when macOS changes the colour
/// scheme while the choice is System (`crate::appearance`). A test uses
/// [`set_appearance_for_test`], which puts the appearance back.
pub(crate) fn set_appearance(appearance: Appearance) {
    APPEARANCE.with(|cell| cell.set(appearance));
}

/// Set the appearance of the window of this thread until the end of a test,
/// also when the test panics. A test of another appearance must not change
/// the colours of the tests that follow on the same thread.
#[cfg(test)]
pub(crate) fn set_appearance_for_test(appearance: Appearance) -> AppearanceGuard {
    let before = self::appearance();
    set_appearance(appearance);
    AppearanceGuard(before)
}

/// Keep the appearance of the thread that it has now, and put it back at the
/// end of a test. A test that changes the appearance through the window
/// (a setting, or the appearance of macOS) uses it, because the window uses
/// [`set_appearance`] directly.
#[cfg(test)]
pub(crate) fn keep_appearance_for_test() -> AppearanceGuard {
    AppearanceGuard(self::appearance())
}

/// Puts back the appearance of the thread that it had before a test.
#[cfg(test)]
pub(crate) struct AppearanceGuard(Appearance);

#[cfg(test)]
impl Drop for AppearanceGuard {
    fn drop(&mut self) {
        set_appearance(self.0);
    }
}

/// The colour tokens of one appearance. Each colour token of this module
/// has a field here, thus a new token needs a value in each palette: the
/// compiler enforces it.
struct Palette {
    bg_canvas: Rgba,
    bg_bar: Rgba,
    bg_sidebar: Rgba,
    bg_panel: Rgba,
    bg_card: Rgba,
    bg_control: Rgba,
    bg_field: Rgba,
    bg_segment_on: Rgba,
    bg_tile: Rgba,
    line_pane: Rgba,
    line_section: Rgba,
    border_field: Rgba,
    border_control: Rgba,
    border_overlay: Rgba,
    border_device_card: Rgba,
    line_sidebar: Rgba,
    text_primary: Rgba,
    text_secondary: Rgba,
    text_tertiary: Rgba,
    text_disabled: Rgba,
    text_on_accent: Rgba,
    text_on_segment: Rgba,
    text_on_tile: Rgba,
    text_on_tint: Rgba,
    accent: Rgba,
    accent_line: Rgba,
    accent_text: Rgba,
    accent_text_hover: Rgba,
    accent_tint: Rgba,
    accent_tint_drag: Rgba,
    success: Rgba,
    success_text: Rgba,
    success_tint: Rgba,
    destructive_text: Rgba,
    destructive_tint: Rgba,
    destructive_line: Rgba,
    warning: Rgba,
    input_glow: Rgba,
    hover_control: Rgba,
    hover_icon: Rgba,
    transparent: Rgba,
    hover_row: Rgba,
    menu_highlight: Rgba,
    selected_unfocused: Rgba,
    pressed: Rgba,
    neutral_tint: Rgba,
    border_sample_other: Rgba,
    switch_knob_colour: Rgba,
    kind_tile_system: Rgba,
    kind_tile_media: Rgba,
    kind_tile_navigation: Rgba,
    kind_tile_logic: Rgba,
    kind_tile_other: Rgba,
    kind_tile_unavailable: Rgba,
    device_body: Rgba,
    device_body_border: Rgba,
    device_display: Rgba,
    device_surface_border: Rgba,
    device_knob: Rgba,
    device_knob_ring: Rgba,
    device_touch_bar_border: Rgba,
    device_knob_dot: Rgba,
    device_key_empty_border: Rgba,
    device_key_empty_icon: Rgba,
    device_touch_bar_hover: Rgba,
    device_text_secondary: Rgba,
    device_text_tertiary: Rgba,
    device_accent_text: Rgba,
    device_touch_bar_bg: Rgba,
    accent_line_hover: Rgba,
    badge_on_image: Rgba,
    accent_tint_drop: Rgba,
    modal_dim: Rgba,
    tile_on_accent: Rgba,
    bg_context_chip: Rgba,
    text_icon_cell: Rgba,
    accent_tint_chip: Rgba,
    picture_panel_empty: Rgba,
    success_fill: Rgba,
    accent_tint_start: Rgba,
    // The colours of the shadows (section 2.8). A shadow function reads its
    // colour here, thus the compiler needs a value in each palette.
    shadow_device_body: Rgba,
    shadow_input_glow: Rgba,
    shadow_drag: Rgba,
    shadow_popover: Rgba,
    shadow_sheet: Rgba,
}

/// The dark appearance: the values of the window today.
const DARK: Palette = Palette {
    bg_canvas: hex(0x18191c),
    bg_bar: hex(0x1f2024),
    bg_sidebar: hex(0x232428),
    bg_panel: hex(0x202125),
    bg_card: hex(0x26272c),
    bg_control: hex(0x2f3036),
    bg_field: hex(0x2a2b30),
    bg_segment_on: hex(0x4a4c53),
    bg_tile: hex(0x2e3036),
    line_pane: hex(0x34363b),
    line_section: hex(0x2c2e33),
    border_field: hex(0x3a3c42),
    border_control: hex(0x42444b),
    border_overlay: hex(0x44474f),
    border_device_card: hex(0x383a40),
    line_sidebar: hex(0x2e3035),
    text_primary: hex(0xececee),
    text_secondary: hex(0xa0a3ab),
    text_tertiary: hex(0x8b8e96),
    text_disabled: hex(0x6b6e76),
    text_on_accent: hex(0xffffff),
    text_on_segment: hex(0xffffff),
    text_on_tile: hex(0xffffff),
    text_on_tint: hex(0xffffff),
    accent: hex(0x1a6fd8),
    accent_line: hex(0x3b95ff),
    accent_text: hex(0x4da3ff),
    accent_text_hover: hex(0x7dbbff),
    accent_tint: hex_alpha(0x1a6fd8, 0.14),
    accent_tint_drag: hex_alpha(0x1a6fd8, 0.18),
    success: hex(0x34c759),
    success_text: hex(0x7ee2a0),
    success_tint: hex_alpha(0x34c759, 0.12),
    destructive_text: hex(0xff8a80),
    destructive_tint: hex_alpha(0xff8a80, 0.12),
    destructive_line: hex_alpha(0xff8a80, 0.35),
    warning: hex(0xf5b65c),
    input_glow: hex(0xffce73),
    hover_control: hex(0x383a40),
    hover_icon: hex_alpha(0xffffff, 0.06),
    transparent: hex_alpha(0x000000, 0.),
    hover_row: hex_alpha(0xffffff, 0.05),
    menu_highlight: hex_alpha(0x1a6fd8, 0.32),
    selected_unfocused: hex_alpha(0xffffff, 0.07),
    pressed: hex(0x42444b),
    neutral_tint: hex_alpha(0xffffff, 0.06),
    border_sample_other: hex(0x5a5c63),
    switch_knob_colour: hex(0xffffff),
    kind_tile_system: hex(0x3a3f47),
    kind_tile_media: hex(0x2350a8),
    kind_tile_navigation: hex(0x1f6f6b),
    kind_tile_logic: hex(0x7a3e2e),
    kind_tile_other: hex(0x4b3b7a),
    kind_tile_unavailable: hex(0x3a3c42),
    device_body: hex(0x0b0c0e),
    device_body_border: hex(0x2e3036),
    device_display: hex(0x0f1724),
    device_surface_border: hex_alpha(0xffffff, 0.06),
    device_knob: hex(0x1c1e23),
    device_knob_ring: hex(0x3a3c42),
    device_touch_bar_border: hex_alpha(0xffffff, 0.10),
    device_knob_dot: hex(0xececee),
    device_key_empty_border: hex(0x3a3f4c),
    device_key_empty_icon: hex(0x5a5f6b),
    device_touch_bar_hover: hex_alpha(0xffffff, 0.10),
    device_text_secondary: hex(0xa0a3ab),
    device_text_tertiary: hex(0x8b8e96),
    device_accent_text: hex(0x4da3ff),
    device_touch_bar_bg: hex_alpha(0xffffff, 0.06),
    accent_line_hover: hex_alpha(0x3b95ff, 0.5),
    badge_on_image: hex_alpha(0x000000, 0.55),
    accent_tint_drop: hex_alpha(0x1a6fd8, 0.28),
    modal_dim: hex_alpha(0x000000, 0.25),
    tile_on_accent: hex_alpha(0xffffff, 0.18),
    bg_context_chip: hex(0x34363c),
    text_icon_cell: hex(0xd0d2d8),
    accent_tint_chip: hex_alpha(0x1a6fd8, 0.25),
    picture_panel_empty: hex(0x1a1f2a),
    success_fill: hex(0x1e7a46),
    accent_tint_start: hex_alpha(0x1a6fd8, 0.16),
    shadow_device_body: hex_alpha(0x000000, 0.45),
    // The glow of a control on the device is `input_glow` at 55 %.
    shadow_input_glow: hex_alpha(0xffce73, 0.55),
    shadow_drag: hex_alpha(0x000000, 0.5),
    shadow_popover: hex_alpha(0x000000, 0.5),
    shadow_sheet: hex_alpha(0x000000, 0.55),
};

/// The light appearance: the same tokens on light surfaces, as macOS uses
/// them in the light appearance. Section 2.9 of docs/DESIGN.md gives the
/// contrast rules of these values. The device drawing keeps the colours of
/// the hardware (the dark body, the display, the knob, and the empty key
/// face); only the border of the body and the shadow on the canvas change.
const LIGHT: Palette = Palette {
    bg_canvas: hex(0xf3f4f6),
    bg_bar: hex(0xfbfbfc),
    bg_sidebar: hex(0xeaecf0),
    bg_panel: hex(0xf6f7f9),
    bg_card: hex(0xffffff),
    bg_control: hex(0xffffff),
    bg_field: hex(0xffffff),
    bg_segment_on: hex(0xd3d7de),
    bg_tile: hex(0xe2e5ea),
    line_pane: hex(0xdcdfe4),
    line_section: hex(0xe4e6ea),
    border_field: hex(0xd2d5db),
    border_control: hex(0xc6cad1),
    border_overlay: hex(0xbfc4cc),
    border_device_card: hex(0xd6dae0),
    line_sidebar: hex(0xe0e3e7),
    text_primary: hex(0x17191d),
    text_secondary: hex(0x545963),
    text_tertiary: hex(0x62676f),
    text_disabled: hex(0x83888f),
    text_on_accent: hex(0xffffff),
    text_on_segment: hex(0x1b1e23),
    text_on_tile: hex(0x20242a),
    text_on_tint: hex(0x0b4fa8),
    accent: hex(0x0b5fcc),
    accent_line: hex(0x1f6fd8),
    accent_text: hex(0x0b5fcc),
    accent_text_hover: hex(0x094fab),
    accent_tint: hex_alpha(0x0b5fcc, 0.10),
    accent_tint_drag: hex_alpha(0x0b5fcc, 0.12),
    success: hex(0x1b8a3c),
    success_text: hex(0x0b6e2e),
    success_tint: hex_alpha(0x1b8a3c, 0.12),
    destructive_text: hex(0xb3261e),
    destructive_tint: hex_alpha(0xb3261e, 0.10),
    destructive_line: hex_alpha(0xb3261e, 0.35),
    warning: hex(0x8a5a00),
    // The glow is on the device drawing, which stays dark in both
    // appearances.
    input_glow: hex(0xffce73),
    hover_control: hex(0xf1f3f5),
    hover_icon: hex_alpha(0x000000, 0.05),
    transparent: hex_alpha(0x000000, 0.),
    hover_row: hex_alpha(0x000000, 0.045),
    menu_highlight: hex_alpha(0x0b5fcc, 0.16),
    selected_unfocused: hex_alpha(0x000000, 0.06),
    pressed: hex(0xe5e8ec),
    neutral_tint: hex_alpha(0x000000, 0.06),
    border_sample_other: hex(0x8a8f98),
    switch_knob_colour: hex(0xffffff),
    // The kind tiles keep a saturated colour in both appearances, as the
    // icons of a macOS toolbar do. The white letter reads on each one.
    kind_tile_system: hex(0x3a3f47),
    kind_tile_media: hex(0x2350a8),
    kind_tile_navigation: hex(0x1f6f6b),
    kind_tile_logic: hex(0x7a3e2e),
    kind_tile_other: hex(0x4b3b7a),
    kind_tile_unavailable: hex(0x3a3c42),
    // The device drawing: the same pixels as the hardware, in both
    // appearances. Only the border of the body follows the canvas.
    device_body: hex(0x0b0c0e),
    device_body_border: hex(0x3c4048),
    device_display: hex(0x0f1724),
    device_surface_border: hex_alpha(0xffffff, 0.06),
    device_knob: hex(0x1c1e23),
    device_knob_ring: hex(0x3a3c42),
    device_touch_bar_border: hex_alpha(0xffffff, 0.10),
    device_knob_dot: hex(0xececee),
    device_key_empty_border: hex(0x3a3f4c),
    device_key_empty_icon: hex(0x5a5f6b),
    device_touch_bar_hover: hex_alpha(0xffffff, 0.10),
    // A name on the device drawing, and the touch bar control, have the
    // colours of the hardware: the same values in both appearances.
    device_text_secondary: hex(0xa0a3ab),
    device_text_tertiary: hex(0x8b8e96),
    device_accent_text: hex(0x4da3ff),
    device_touch_bar_bg: hex_alpha(0xffffff, 0.06),
    accent_line_hover: hex_alpha(0x1f6fd8, 0.55),
    badge_on_image: hex_alpha(0x000000, 0.6),
    accent_tint_drop: hex_alpha(0x0b5fcc, 0.18),
    modal_dim: hex_alpha(0x000000, 0.2),
    tile_on_accent: hex_alpha(0xffffff, 0.22),
    bg_context_chip: hex(0xe5e8ec),
    text_icon_cell: hex(0x33373e),
    accent_tint_chip: hex_alpha(0x0b5fcc, 0.14),
    picture_panel_empty: hex(0xe7eaef),
    success_fill: hex(0x157a3a),
    accent_tint_start: hex_alpha(0x0b5fcc, 0.10),
    // A shadow is weaker on a light canvas.
    shadow_device_body: hex_alpha(0x000000, 0.18),
    shadow_input_glow: hex_alpha(0xffce73, 0.55),
    shadow_drag: hex_alpha(0x000000, 0.18),
    shadow_popover: hex_alpha(0x000000, 0.18),
    shadow_sheet: hex_alpha(0x000000, 0.22),
};

/// The colours of the appearance of this thread.
fn palette() -> &'static Palette {
    match appearance() {
        Appearance::Dark => &DARK,
        Appearance::Light => &LIGHT,
    }
}

// MARK: Colours (section 3.2)

/// The canvas, and the window behind all panes.
pub fn bg_canvas() -> Rgba {
    palette().bg_canvas
}

/// The toolbar and the status bar.
pub fn bg_bar() -> Rgba {
    palette().bg_bar
}

/// The sidebar.
pub fn bg_sidebar() -> Rgba {
    palette().bg_sidebar
}

/// The right panel, and a nested card.
pub fn bg_panel() -> Rgba {
    palette().bg_panel
}

/// A card, and a menu that the window draws.
pub fn bg_card() -> Rgba {
    palette().bg_card
}

/// A button.
pub fn bg_control() -> Rgba {
    palette().bg_control
}

/// A text field, the track of a segmented control, and the device card.
pub fn bg_field() -> Rgba {
    palette().bg_field
}

/// The selected segment of a segmented control.
pub fn bg_segment_on() -> Rgba {
    palette().bg_segment_on
}

/// The tile behind the device icon.
pub fn bg_tile() -> Rgba {
    palette().bg_tile
}

/// The border between two panes.
pub fn line_pane() -> Rgba {
    palette().line_pane
}

/// The line below the toolbar, above the status bar, and below the tab header.
pub fn line_section() -> Rgba {
    palette().line_section
}

/// The border of a text field, of a card, and of a segmented control track.
pub fn border_field() -> Rgba {
    palette().border_field
}

/// The border of a button.
pub fn border_control() -> Rgba {
    palette().border_control
}

/// The border of a menu or of a popover.
pub fn border_overlay() -> Rgba {
    palette().border_overlay
}

/// The border of the device card when it is not selected.
pub fn border_device_card() -> Rgba {
    palette().border_device_card
}

/// The line above the bottom group of the sidebar.
pub fn line_sidebar() -> Rgba {
    palette().line_sidebar
}

/// Normal text.
pub fn text_primary() -> Rgba {
    palette().text_primary
}

/// Form labels, icon buttons, and a value at the right.
pub fn text_secondary() -> Rgba {
    palette().text_secondary
}

/// Section headers, captions, help text, and placeholders.
pub fn text_tertiary() -> Rgba {
    palette().text_tertiary
}

/// A disabled button or icon.
pub fn text_disabled() -> Rgba {
    palette().text_disabled
}

/// Text on an accent fill.
pub fn text_on_accent() -> Rgba {
    palette().text_on_accent
}

/// The text of a selected segment, of a selected tab, and of a toggle button
/// that is on: the text on [`bg_segment_on()`].
pub fn text_on_segment() -> Rgba {
    palette().text_on_segment
}

/// The icon of a tile with [`bg_tile()`], for example a command tile of the
/// palette.
pub fn text_on_tile() -> Rgba {
    palette().text_on_tile
}

/// The text on a tinted background: a selected chip
/// ([`accent_tint_chip()`]).
pub fn text_on_tint() -> Rgba {
    palette().text_on_tint
}

/// The selected row, and the main button.
pub fn accent() -> Rgba {
    palette().accent
}

/// The selection outline, and the focus border of a field.
pub fn accent_line() -> Rgba {
    palette().accent_line
}

/// A link, and the dot of the last input.
pub fn accent_text() -> Rgba {
    palette().accent_text
}

/// A link under the pointer.
pub fn accent_text_hover() -> Rgba {
    palette().accent_text_hover
}

/// The accent at 14 %: the background of a selected card.
pub fn accent_tint() -> Rgba {
    palette().accent_tint
}

/// The accent at 18 %: a row under a drag (section 3.2).
pub fn accent_tint_drag() -> Rgba {
    palette().accent_tint_drag
}

/// The dot of a connected device.
pub fn success() -> Rgba {
    palette().success
}

/// The text of the live state pill.
pub fn success_text() -> Rgba {
    palette().success_text
}

/// The background of the live state pill.
pub fn success_tint() -> Rgba {
    palette().success_tint
}

/// The text of a button that removes something.
pub fn destructive_text() -> Rgba {
    palette().destructive_text
}

/// The background of a message row (section 6.2).
pub fn destructive_tint() -> Rgba {
    palette().destructive_tint
}

/// The line at the top of a message row (section 6.2).
pub fn destructive_line() -> Rgba {
    palette().destructive_line
}

/// The icon of a warning, and a warning text in a gesture card.
pub fn warning() -> Rgba {
    palette().warning
}

/// A control that the user operates on the device glows in this colour.
pub fn input_glow() -> Rgba {
    palette().input_glow
}

/// A button under the pointer: one step lighter than [`bg_control()`].
pub fn hover_control() -> Rgba {
    palette().hover_control
}

/// A transparent icon button under the pointer.
pub fn hover_icon() -> Rgba {
    palette().hover_icon
}

/// No colour: a border that shows only in a different state.
pub fn transparent() -> Rgba {
    palette().transparent
}

/// A row under the pointer.
pub fn hover_row() -> Rgba {
    palette().hover_row
}

/// The highlighted item of a menu that the window draws: the item that a key
/// selects, and the item under the pointer.
pub fn menu_highlight() -> Rgba {
    palette().menu_highlight
}

/// A selected row in a pane that does not have the selection, for example
/// the active page row when the device is the selection.
pub fn selected_unfocused() -> Rgba {
    palette().selected_unfocused
}

/// A pressed button.
pub fn pressed() -> Rgba {
    palette().pressed
}

/// The background of the live state pill when no device is connected.
pub fn neutral_tint() -> Rgba {
    palette().neutral_tint
}

/// The dashed border of the fill sample "Other colour" (section 10.2).
pub fn border_sample_other() -> Rgba {
    palette().border_sample_other
}

/// The border of a toggle button that is on, for example "B" for bold
/// (section 10.2).
pub fn border_toggle_on() -> Rgba {
    border_sample_other()
}

/// The white knob of a switch, and the text on a kind tile.
pub fn switch_knob_colour() -> Rgba {
    palette().switch_knob_colour
}

/// The tile of a kind of each group of section 20, and of a group that is
/// not in that table.
pub fn kind_tile_system() -> Rgba {
    palette().kind_tile_system
}
pub fn kind_tile_media() -> Rgba {
    palette().kind_tile_media
}
pub fn kind_tile_navigation() -> Rgba {
    palette().kind_tile_navigation
}
pub fn kind_tile_logic() -> Rgba {
    palette().kind_tile_logic
}
pub fn kind_tile_other() -> Rgba {
    palette().kind_tile_other
}

/// The tile of an action kind that has no handler (rule 8).
pub fn kind_tile_unavailable() -> Rgba {
    palette().kind_tile_unavailable
}

/// The tile colour of a kind of the group `group` (decision 0029, section
/// 20). The colour comes from the group, thus the window needs no colour for
/// each kind. A group of a plug-in gets [`kind_tile_other()`].
pub fn kind_tile(group: &str) -> Rgba {
    match group {
        "System" => kind_tile_system(),
        "Media" => kind_tile_media(),
        "Navigation" => kind_tile_navigation(),
        "Logic" => kind_tile_logic(),
        _ => kind_tile_other(),
    }
}

/// The body of the device.
pub fn device_body() -> Rgba {
    palette().device_body
}

/// The border of the body of the device.
pub fn device_body_border() -> Rgba {
    palette().device_body_border
}

/// The display when the page has no background image.
pub fn device_display() -> Rgba {
    palette().device_display
}

/// The border of a key face and of a bar panel.
pub fn device_surface_border() -> Rgba {
    palette().device_surface_border
}

/// The inside of a knob.
pub fn device_knob() -> Rgba {
    palette().device_knob
}

/// The ring of a knob.
pub fn device_knob_ring() -> Rgba {
    palette().device_knob_ring
}

/// The border of the "Touch bar" control.
pub fn device_touch_bar_border() -> Rgba {
    palette().device_touch_bar_border
}

/// The mark at the top of a knob.
pub fn device_knob_dot() -> Rgba {
    palette().device_knob_dot
}

/// The dashed border of an empty key face.
pub fn device_key_empty_border() -> Rgba {
    palette().device_key_empty_border
}

/// The plus icon on an empty key face.
pub fn device_key_empty_icon() -> Rgba {
    palette().device_key_empty_icon
}

/// The "Touch bar" control under the pointer.
pub fn device_touch_bar_hover() -> Rgba {
    palette().device_touch_bar_hover
}

/// A name on the device drawing: the name of a control with no surface, and
/// the text of the "Touch bar" control.
pub fn device_text_secondary() -> Rgba {
    palette().device_text_secondary
}

/// A quiet name on the device drawing: the name below a knob.
pub fn device_text_tertiary() -> Rgba {
    palette().device_text_tertiary
}

/// The name below the selected knob.
pub fn device_accent_text() -> Rgba {
    palette().device_accent_text
}

/// The "Touch bar" control when it is not selected.
pub fn device_touch_bar_bg() -> Rgba {
    palette().device_touch_bar_bg
}

/// The outline of a control under the pointer: [`accent_line()`] at 50 %.
pub fn accent_line_hover() -> Rgba {
    palette().accent_line_hover
}

/// The shadow of the body of the device: 0 24 60, black at 45 %.
pub fn device_body_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: palette().shadow_device_body.into(),
        offset: point(px(0.), px(24.)),
        blur_radius: px(60.),
        spread_radius: px(0.),
        inset: false,
    }]
}

/// The glow around a control that the user operates on the device: the
/// colour of [`input_glow()`] at 55 %.
pub fn input_glow_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: palette().shadow_input_glow.into(),
        offset: point(px(0.), px(0.)),
        blur_radius: px(14.),
        spread_radius: px(1.),
        inset: false,
    }]
}

/// The shadow of the drag picture: 0 12 28, black at 50 % (section 3.2).
pub fn drag_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: palette().shadow_drag.into(),
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
        color: palette().shadow_popover.into(),
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
        color: accent_line().into(),
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
        color: accent_line().into(),
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
/// The gesture name of a key or swipe card.
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
/// The backing of a text on an image: the size text of a background preview,
/// and the text "Drop here" of a drop target.
pub fn badge_on_image() -> Rgba {
    palette().badge_on_image
}

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
/// The side of a control that has no surface, in pixels of the display.
/// A row that is wider than the display uses a smaller square.
pub const DEVICE_PLAIN_SIZE: f32 = 96.;
/// The space between those controls, in pixels of the display.
pub const DEVICE_PLAIN_GAP: f32 = 16.;
/// The space above that block. Below the knob labels when the layout has a
/// knob, else below the display.
pub const DEVICE_PLAIN_TOP: f32 = 22.;

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
/// A library row in a drag has [`accent_tint_drag()`].
pub fn accent_tint_drop() -> Rgba {
    palette().accent_tint_drop
}

/// The width of the dashed border of the drop target.
pub const DROP_TARGET_BORDER: Pixels = px(2.);
/// The top-left corner of the drag picture, from the pointer. The picture
/// hangs below the pointer, so that the pointer and the text "Drop here" of
/// the target stay visible.
pub const DRAG_PICTURE_OFFSET: (Pixels, Pixels) = (px(4.), px(14.));
/// The side of the picture of the key face in the drag picture (KEY-14).
pub const SIDE_KEY_DRAG_IMAGE: Pixels = px(24.);

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
/// The width of the segmented control "Appearance" of Settings > General:
/// three segments of 70.
pub const WIDTH_SETTINGS_APPEARANCE: Pixels = px(210.);
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
pub fn modal_dim() -> Rgba {
    palette().modal_dim
}

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
pub fn border_no_colour() -> Rgba {
    border_sample_other()
}

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
pub fn tile_on_accent() -> Rgba {
    palette().tile_on_accent
}

/// The chip with the selected control, for example "Key 8 selected".
pub fn bg_context_chip() -> Rgba {
    palette().bg_context_chip
}

/// The search field of the toolbar: a button that looks like a search
/// field.
pub const WIDTH_TOOLBAR_SEARCH: Pixels = px(240.);
/// The side padding of a key hint, for example "⌘K" (padding 1 5).
pub const KEY_HINT_PADDING_X: Pixels = px(5.);

/// The shadow of a sheet and of the command palette: 0 24 60, black at 55 %.
pub fn sheet_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: palette().shadow_sheet.into(),
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
pub fn text_icon_cell() -> Rgba {
    palette().text_icon_cell
}

/// A cell of the icon grid under the pointer: one step lighter than
/// [`bg_bar()`], with the border of a button.
pub fn hover_icon_cell() -> Rgba {
    bg_control()
}

/// The height of a filter chip, for example the tab "Icons" (section 3.5),
/// and its corner radius.
pub const HEIGHT_CHIP: Pixels = px(24.);
pub const RADIUS_CHIP: Pixels = px(12.);
/// The accent at 25 %: the background of a selected chip.
pub fn accent_tint_chip() -> Rgba {
    palette().accent_tint_chip
}

/// The button "Image file…" of the icon picker.
pub fn bg_image_button() -> Rgba {
    bg_context_chip()
}

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

// MARK: The list pickers

/// The popover of a picker with a search field and a list: the application
/// picker, and the font picker of a label. It aligns to the trailing edge of
/// its button.
pub const LIST_PICKER_WIDTH: Pixels = px(320.);
/// The height of the popover when the window has room for it.
pub const LIST_PICKER_HEIGHT: Pixels = px(360.);
/// The smallest height of the popover in a low window. The list then
/// scrolls in less space.
pub const LIST_PICKER_MIN_HEIGHT: Pixels = px(220.);

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
/// The list of ready profiles on the first start window: its width, and
/// its greatest height. The list grows with its rows, up to this height.
pub const FIRST_START_READY_WIDTH: Pixels = px(360.);
pub const FIRST_START_READY_MAX_HEIGHT: Pixels = px(420.);
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
pub fn picture_panel_empty() -> Rgba {
    palette().picture_panel_empty
}

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
pub fn success_fill() -> Rgba {
    palette().success_fill
}

/// A starter profile card: its corner radius, and the background of the
/// selected card (the accent at 16 %).
pub const RADIUS_STARTER_CARD: Pixels = px(10.);
pub fn accent_tint_start() -> Rgba {
    palette().accent_tint_start
}

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
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
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
    use super::*;
    use gpui_kit::Hsla;
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
        assert!(literals(".w(px(0.)).bg(theme::accent()).h(px(theme::X))").is_empty());
        assert!(literals("images::from_rgb(&image); theme::sample(value)").is_empty());
        assert!(literals("Color::rgb(r, g, b); fn f() -> Rgba {").is_empty());
        assert!(literals("// rgb(0x112233) in a comment").is_empty());
    }

    /// The parts of a colour. GPUI has no equality for `Hsla`, thus a test
    /// compares the parts. A shadow colour is the `Hsla` of a palette value.
    fn parts(colour: Hsla) -> (f32, f32, f32, f32) {
        (colour.h, colour.s, colour.l, colour.a)
    }

    /// The shadows keep the values of section 2.8 of each appearance. A
    /// change of the palette of another appearance must not change them.
    #[test]
    fn the_dark_shadows_keep_their_values() {
        let _scope = set_appearance_for_test(Appearance::Dark);
        assert_eq!(
            parts(device_body_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0x000000, 0.45)))
        );
        assert_eq!(
            parts(input_glow_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0xffce73, 0.55)))
        );
        assert_eq!(
            parts(drag_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0x000000, 0.5)))
        );
        assert_eq!(
            parts(popover_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0x000000, 0.5)))
        );
        assert_eq!(
            parts(sheet_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0x000000, 0.55)))
        );
    }

    /// The light shadows are weaker than the dark ones, because a light
    /// canvas shows a black shadow more.
    #[test]
    fn the_light_shadows_are_weaker_than_the_dark_ones() {
        let _scope = set_appearance_for_test(Appearance::Light);
        assert_eq!(
            parts(device_body_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0x000000, 0.18)))
        );
        assert_eq!(
            parts(drag_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0x000000, 0.18)))
        );
        assert_eq!(
            parts(popover_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0x000000, 0.18)))
        );
        assert_eq!(
            parts(sheet_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0x000000, 0.22)))
        );
        // The glow of a control is on the device drawing, which is dark in
        // both appearances.
        assert_eq!(
            parts(input_glow_shadow()[0].color),
            parts(Hsla::from(hex_alpha(0xffce73, 0.55)))
        );
        for (light, dark) in [
            (LIGHT.shadow_device_body.a, DARK.shadow_device_body.a),
            (LIGHT.shadow_drag.a, DARK.shadow_drag.a),
            (LIGHT.shadow_popover.a, DARK.shadow_popover.a),
            (LIGHT.shadow_sheet.a, DARK.shadow_sheet.a),
            (LIGHT.modal_dim.a, DARK.modal_dim.a),
        ] {
            assert!(light < dark, "{light} is not below {dark}");
        }
    }

    /// The relative luminance of `colour` over `base`, as WCAG defines it.
    fn luminance(colour: Rgba, base: Rgba) -> f32 {
        let over = |c: f32, b: f32| c * colour.a + b * (1. - colour.a);
        let linear = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(over(colour.r, base.r))
            + 0.7152 * linear(over(colour.g, base.g))
            + 0.0722 * linear(over(colour.b, base.b))
    }

    /// `colour`, at its opacity, over the opaque `base`: the colour that the
    /// window paints where the two meet.
    fn over(colour: Rgba, base: Rgba) -> Rgba {
        let blend = |c: f32, b: f32| c * colour.a + b * (1. - colour.a);
        Rgba {
            r: blend(colour.r, base.r),
            g: blend(colour.g, base.g),
            b: blend(colour.b, base.b),
            a: 1.,
        }
    }

    /// The contrast of `colour` over `base` from 1 to 21, as WCAG defines it.
    fn contrast(colour: Rgba, base: Rgba) -> f32 {
        let a = luminance(colour, base);
        let b = luminance(base, base);
        let (high, low) = if a > b { (a, b) } else { (b, a) };
        (high + 0.05) / (low + 0.05)
    }

    /// The rules of section 2.9 of docs/DESIGN.md, for both palettes. A rule
    /// that the dark appearance keeps today also holds for the light one.
    #[test]
    fn both_palettes_meet_the_contrast_rules_of_design() {
        for (name, palette) in [("dark", &DARK), ("light", &LIGHT)] {
            let surfaces = [
                ("bg_canvas", palette.bg_canvas),
                ("bg_bar", palette.bg_bar),
                ("bg_sidebar", palette.bg_sidebar),
                ("bg_panel", palette.bg_panel),
                ("bg_card", palette.bg_card),
                ("bg_field", palette.bg_field),
                ("bg_control", palette.bg_control),
            ];
            let check = |what: &str, colour: Rgba, base: Rgba, least: f32| {
                let ratio = contrast(colour, base);
                assert!(
                    ratio >= least,
                    "{name}: {what} is {ratio:.2}:1, below {least}:1"
                );
            };

            // Body text, labels, and help texts.
            for (_, base) in surfaces {
                check("text_primary", palette.text_primary, base, 7.);
                check("text_secondary", palette.text_secondary, base, 4.5);
                check("accent_line", palette.accent_line, base, 3.);
            }
            for (_, base) in surfaces.iter().take(5) {
                check("text_tertiary", palette.text_tertiary, *base, 4.5);
            }
            check(
                "text_disabled on the canvas",
                palette.text_disabled,
                palette.bg_canvas,
                3.,
            );

            // A link, a status text, and the text of a coloured fill.
            for (_, base) in surfaces.iter().take(5) {
                check("accent_text", palette.accent_text, *base, 4.5);
            }
            for (_, base) in [
                ("bg_canvas", palette.bg_canvas),
                ("bg_bar", palette.bg_bar),
                ("bg_panel", palette.bg_panel),
                ("bg_card", palette.bg_card),
            ] {
                check("success_text", palette.success_text, base, 4.5);
                check("destructive_text", palette.destructive_text, base, 4.5);
                check("warning", palette.warning, base, 4.5);
            }
            check(
                "text_on_accent",
                palette.text_on_accent,
                palette.accent,
                4.5,
            );
            check(
                "text_on_accent on success_fill",
                palette.text_on_accent,
                palette.success_fill,
                4.5,
            );
            check("accent", palette.accent, palette.bg_canvas, 3.);

            // The state of a control with no accent fill.
            check(
                "text_on_segment",
                palette.text_on_segment,
                palette.bg_segment_on,
                4.5,
            );
            check("text_on_tile", palette.text_on_tile, palette.bg_tile, 4.5);
            // The chip has a tint over the surface: the text reads on the
            // tinted chip, not on the surface only.
            for base in [palette.bg_card, palette.bg_panel] {
                let chip = over(palette.accent_tint_chip, base);
                check("text_on_tint", palette.text_on_tint, chip, 4.5);
            }
            // The names on the device drawing. The drawing is dark in both
            // appearances, thus those colours do not change with it.
            check(
                "device_text_secondary on the display",
                palette.device_text_secondary,
                palette.device_display,
                4.5,
            );
            check(
                "device_text_secondary on the touch bar",
                palette.device_text_secondary,
                over(palette.device_touch_bar_bg, palette.device_display),
                4.5,
            );
            check(
                "device_text_secondary on the body",
                palette.device_text_secondary,
                palette.device_body,
                4.5,
            );
            for (what, colour) in [
                ("device_text_tertiary", palette.device_text_tertiary),
                ("device_accent_text", palette.device_accent_text),
            ] {
                check(what, colour, palette.device_body, 4.5);
            }
            check(
                "switch_knob_colour",
                palette.switch_knob_colour,
                palette.bg_segment_on,
                1.3,
            );

            // A border is visible, and a state tint is visible.
            check(
                "border_control",
                palette.border_control,
                palette.bg_card,
                1.4,
            );
            check("border_field", palette.border_field, palette.bg_field, 1.25);
            check(
                "border_overlay",
                palette.border_overlay,
                palette.bg_card,
                1.4,
            );
            for (_, base) in surfaces {
                check("hover_row", palette.hover_row, base, 1.08);
                check("selected_unfocused", palette.selected_unfocused, base, 1.08);
            }
            for base in [palette.bg_card, palette.bg_field, palette.bg_bar] {
                check("menu_highlight", palette.menu_highlight, base, 1.2);
            }

            // A sheet on a light canvas needs less dimming.
            if name == "light" {
                assert!(palette.modal_dim.a < DARK.modal_dim.a);
            }
        }
    }

    /// The device picture keeps the colours of the hardware in both
    /// appearances. Only the border of the body and its shadow follow the
    /// canvas.
    #[test]
    fn the_device_picture_keeps_the_colours_of_the_hardware() {
        for (field, dark, light) in [
            ("device_body", DARK.device_body, LIGHT.device_body),
            ("device_display", DARK.device_display, LIGHT.device_display),
            (
                "device_surface_border",
                DARK.device_surface_border,
                LIGHT.device_surface_border,
            ),
            ("device_knob", DARK.device_knob, LIGHT.device_knob),
            (
                "device_knob_ring",
                DARK.device_knob_ring,
                LIGHT.device_knob_ring,
            ),
            (
                "device_touch_bar_border",
                DARK.device_touch_bar_border,
                LIGHT.device_touch_bar_border,
            ),
            (
                "device_knob_dot",
                DARK.device_knob_dot,
                LIGHT.device_knob_dot,
            ),
            (
                "device_key_empty_border",
                DARK.device_key_empty_border,
                LIGHT.device_key_empty_border,
            ),
            (
                "device_key_empty_icon",
                DARK.device_key_empty_icon,
                LIGHT.device_key_empty_icon,
            ),
            (
                "device_touch_bar_hover",
                DARK.device_touch_bar_hover,
                LIGHT.device_touch_bar_hover,
            ),
            (
                "device_text_secondary",
                DARK.device_text_secondary,
                LIGHT.device_text_secondary,
            ),
            (
                "device_text_tertiary",
                DARK.device_text_tertiary,
                LIGHT.device_text_tertiary,
            ),
            (
                "device_accent_text",
                DARK.device_accent_text,
                LIGHT.device_accent_text,
            ),
            (
                "device_touch_bar_bg",
                DARK.device_touch_bar_bg,
                LIGHT.device_touch_bar_bg,
            ),
            ("input_glow", DARK.input_glow, LIGHT.input_glow),
        ] {
            assert_eq!(dark, light, "{field} changes with the appearance");
        }
        assert_ne!(
            DARK.device_body_border, LIGHT.device_body_border,
            "the border of the body follows the canvas"
        );
        assert_ne!(
            DARK.shadow_device_body, LIGHT.shadow_device_body,
            "the shadow of the body follows the canvas"
        );
    }

    /// A test changes the appearance of its thread only inside a scope, also
    /// when an assertion of the scope fails. The palette of the thread
    /// follows the appearance.
    #[test]
    fn the_appearance_of_a_scope_goes_back_at_its_end() {
        assert_eq!(appearance(), Appearance::Dark);
        assert_eq!(bg_canvas(), DARK.bg_canvas);
        {
            let _scope = set_appearance_for_test(Appearance::Light);
            assert_eq!(appearance(), Appearance::Light);
            assert_eq!(bg_canvas(), LIGHT.bg_canvas);
            assert_ne!(bg_canvas(), DARK.bg_canvas);
        }
        assert_eq!(appearance(), Appearance::Dark);
        assert_eq!(bg_canvas(), DARK.bg_canvas);
        // A second scope leaves the appearance of the first one.
        drop(set_appearance_for_test(Appearance::Light));
        assert_eq!(appearance(), Appearance::Dark);
    }
}
