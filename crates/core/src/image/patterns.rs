//! Test images for the measurements of `opencraft-probe`.
//!
//! These images are upright and not encoded. To send one, use
//! [`super::encode_for`] or [`super::encode_as`]. They have no model
//! knowledge: the caller gives the sizes, and the caller takes them from the
//! descriptor, except for a measurement of what the device accepts.

use super::{draw_number, draw_text, fill, hsv_to_rgb, test_color, text_width};
use super::{RgbImage, BACKGROUND_CORNER_COLORS};
use image::Rgb;

/// The colour of the 1-pixel border of the edge patterns.
pub const EDGE_COLOR: [u8; 3] = [255, 255, 255];

/// The side of a corner mark, in pixels, at most.
const MARK_SIDE: u32 = 8;

/// An image that shows the edges of a bar panel.
///
/// - A 1-pixel border in [`EDGE_COLOR`] at the outer edge.
/// - A fill that is different for each `number` (see [`test_color`]).
/// - The `number` at the centre, large.
/// - A square mark of 8 pixels (or less, for a small image) in each corner,
///   inside the border. The colours are [`BACKGROUND_CORNER_COLORS`]: top
///   left, top right, bottom left, bottom right, upright.
/// - The size as text, "176X112" for example, near the bottom edge.
pub fn bar_edge_pattern(width: u16, height: u16, number: u8) -> RgbImage {
    let (w, h) = (width as u32, height as u32);
    let mut img = RgbImage::from_pixel(w, h, Rgb(test_color(number as usize)));

    let side = MARK_SIDE.min(w / 2).min(h / 2).saturating_sub(1).max(1);
    let corners = [
        (1, 1),
        (w - 1 - side, 1),
        (1, h - 1 - side),
        (w - 1 - side, h - 1 - side),
    ];
    for ((x, y), color) in corners.into_iter().zip(BACKGROUND_CORNER_COLORS) {
        fill(&mut img, x, y, side, side, Rgb(color));
    }

    draw_number(&mut img, number, (h / 14).max(2), Rgb([255, 255, 255]));

    let label = format!("{width}X{height}");
    let tw = text_width(&label, 2);
    let tx = w.saturating_sub(tw) / 2;
    draw_text(
        &mut img,
        tx,
        h.saturating_sub(24),
        &label,
        2,
        Rgb([255, 255, 255]),
    );

    border(&mut img, Rgb(EDGE_COLOR));
    img
}

/// An image to send to a surface that is narrower than the image, to learn
/// what the device does with the part that does not fit.
///
/// `unit` is the width of the surface. The pattern has:
/// - A colour that changes along x, so that the part that shows is clear.
/// - A thick line at each multiple of `unit`, and a thin line at each multiple
///   of `unit / 4`.
/// - The x coordinate as text at each multiple of `unit / 2`, at the top and at
///   the bottom.
/// - A 1-pixel border in [`EDGE_COLOR`], and the corner marks of
///   [`bar_edge_pattern`].
pub fn wide_pattern(width: u16, height: u16, unit: u16) -> RgbImage {
    let (w, h) = (width as u32, height as u32);
    let mut img = RgbImage::from_fn(w, h, |x, _| {
        Rgb(hsv_to_rgb(x as f32 / w.max(1) as f32 * 0.9, 0.55, 0.45))
    });

    let unit = unit.max(4) as u32;
    let white = Rgb([255, 255, 255]);
    for x in (0..w).step_by((unit / 4).max(1) as usize) {
        fill(&mut img, x, 0, 1, h, Rgb([200, 200, 200]));
    }
    for x in (0..w).step_by(unit as usize) {
        fill(&mut img, x, 0, 3, h, white);
    }

    for x in (0..w).step_by((unit / 2).max(1) as usize) {
        let text = x.to_string();
        draw_text(&mut img, x + 5, 4, &text, 2, white);
        draw_text(&mut img, x + 5, h.saturating_sub(18), &text, 2, white);
    }

    let side = MARK_SIDE.min(w / 2).min(h / 2).saturating_sub(1).max(1);
    let corners = [
        (1, 1),
        (w - 1 - side, 1),
        (1, h - 1 - side),
        (w - 1 - side, h - 1 - side),
    ];
    for ((x, y), color) in corners.into_iter().zip(BACKGROUND_CORNER_COLORS) {
        fill(&mut img, x, y, side, side, Rgb(color));
    }

    border(&mut img, Rgb(EDGE_COLOR));
    img
}

/// The symbols of [`icon_image`].
#[derive(Clone, Copy)]
enum Symbol {
    Ring,
    Play,
    Pause,
    Plus,
    Disc,
}

const SYMBOLS: [Symbol; 5] = [
    Symbol::Ring,
    Symbol::Play,
    Symbol::Pause,
    Symbol::Plus,
    Symbol::Disc,
];

impl Symbol {
    /// True if the point (`dx`, `dy`) is in the symbol. The origin is the
    /// centre, and `r` is the radius.
    fn contains(self, dx: f32, dy: f32, r: f32) -> bool {
        match self {
            Symbol::Ring => {
                let d = (dx * dx + dy * dy).sqrt();
                d <= r && d >= 0.6 * r
            }
            Symbol::Play => {
                let (x0, x1) = (-0.6 * r, 0.8 * r);
                dx >= x0 && dx <= x1 && dy.abs() <= r * (x1 - dx) / (x1 - x0)
            }
            Symbol::Pause => {
                dy.abs() <= r
                    && ((dx >= -0.7 * r && dx <= -0.2 * r) || (dx >= 0.2 * r && dx <= 0.7 * r))
            }
            Symbol::Plus => {
                (dy.abs() <= 0.2 * r && dx.abs() <= r) || (dx.abs() <= 0.2 * r && dy.abs() <= r)
            }
            Symbol::Disc => dx * dx + dy * dy <= r * r,
        }
    }
}

/// True if the pixel (`x`, `y`) is in the rounded rectangle at (`x0`, `y0`)
/// with the size `w` × `h` and the corner radius `r`.
fn in_round_rect(x: i32, y: i32, x0: i32, y0: i32, w: i32, h: i32, r: i32) -> bool {
    if x < x0 || y < y0 || x >= x0 + w || y >= y0 + h {
        return false;
    }
    let dx = (x0 + r - x).max(x - (x0 + w - 1 - r)).max(0);
    let dy = (y0 + r - y).max(y - (y0 + h - 1 - r)).max(0);
    dx * dx + dy * dy <= r * r
}

/// A realistic icon for a key face or a bar panel: a rounded rectangle with a
/// vertical gradient, a symbol of basic shapes, a line of text (`word`, 10 × 14
/// pixels for each letter), and a small line of text ("SLOT" and the slot
/// number, 5 × 7 pixels for each letter).
///
/// The slot and `variant` select the colour and the symbol. Two images with a
/// different slot, or a different variant, are different. The image is not a
/// flat colour, so the size of its PNG and its JPEG is like the size of a real
/// icon, and a person can judge the quality of the small text.
pub fn icon_image(width: u16, height: u16, slot: u8, word: &str, variant: u8) -> RgbImage {
    let (w, h) = (width as u32, height as u32);
    let background = Rgb([22, 22, 28]);
    let mut img = RgbImage::from_pixel(w, h, background);

    let hue = (slot as f32 * 0.618_034 + variant as f32 * 0.37) % 1.0;
    let top = hsv_to_rgb(hue, 0.55, 0.95);
    let bottom = hsv_to_rgb(hue, 0.85, 0.40);

    let margin = (h / 28).max(1) as i32;
    let (rw, rh) = (w as i32 - 2 * margin, h as i32 - 2 * margin);
    let radius = (h / 9).max(2) as i32;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if in_round_rect(x, y, margin, margin, rw, rh, radius) {
                let t = ((y - margin) as f32 / rh.max(1) as f32).clamp(0.0, 1.0);
                let c = |i: usize| (top[i] as f32 * (1.0 - t) + bottom[i] as f32 * t) as u8;
                img.put_pixel(x as u32, y as u32, Rgb([c(0), c(1), c(2)]));
            }
        }
    }

    let symbol = SYMBOLS[(slot as usize + variant as usize) % SYMBOLS.len()];
    let (cx, cy, r) = (w as f32 / 2.0, h as f32 * 0.34, h as f32 * 0.17);
    for y in (cy - r) as u32..=(cy + r) as u32 + 1 {
        for x in (cx - r) as u32..=(cx + r) as u32 + 1 {
            if x < w && y < h && symbol.contains(x as f32 - cx, y as f32 - cy, r) {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
    }

    let word = word.to_uppercase();
    let x = w.saturating_sub(text_width(&word, 2)) / 2;
    draw_text(&mut img, x, h * 60 / 100, &word, 2, Rgb([255, 255, 255]));
    let small = format!("SLOT {slot}");
    let x = w.saturating_sub(text_width(&small, 1)) / 2;
    draw_text(&mut img, x, h * 78 / 100, &small, 1, Rgb([235, 235, 235]));
    img
}

fn border(img: &mut RgbImage, color: Rgb<u8>) {
    let (w, h) = img.dimensions();
    for x in 0..w {
        img.put_pixel(x, 0, color);
        img.put_pixel(x, h - 1, color);
    }
    for y in 0..h {
        img.put_pixel(0, y, color);
        img.put_pixel(w - 1, y, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn distinct_colors(img: &RgbImage) -> usize {
        let mut seen: Vec<[u8; 3]> = Vec::new();
        for p in img.pixels() {
            if !seen.contains(&p.0) {
                seen.push(p.0);
            }
        }
        seen.len()
    }

    #[test]
    fn bar_edge_pattern_has_border_marks_number_and_own_fill() {
        let img = bar_edge_pattern(176, 112, 3);
        assert_eq!(img.dimensions(), (176, 112));
        for (x, y) in [(0, 0), (175, 0), (0, 111), (175, 111), (88, 0), (0, 56)] {
            assert_eq!(img.get_pixel(x, y).0, EDGE_COLOR, "border at ({x}, {y})");
        }
        let [tl, tr, bl, br] = BACKGROUND_CORNER_COLORS;
        assert_eq!(img.get_pixel(3, 3).0, tl);
        assert_eq!(img.get_pixel(172, 3).0, tr);
        assert_eq!(img.get_pixel(3, 108).0, bl);
        assert_eq!(img.get_pixel(172, 108).0, br);
        // The border has a different colour than the fill, and the fill is
        // the colour of the number.
        assert_eq!(img.get_pixel(20, 56).0, test_color(3));
        // The number is white at the centre.
        assert!(
            (70..106).any(|x| (40..72).any(|y| img.get_pixel(x, y).0 == [255, 255, 255])),
            "no number at the centre"
        );
    }

    #[test]
    fn bar_edge_patterns_have_a_different_fill_for_each_panel() {
        let fills: Vec<[u8; 3]> = (1..=4)
            .map(|n| bar_edge_pattern(176, 112, n).get_pixel(20, 56).0)
            .collect();
        for i in 0..4 {
            for j in i + 1..4 {
                assert_ne!(fills[i], fills[j], "panels {} and {}", i + 1, j + 1);
            }
        }
    }

    #[test]
    fn bar_edge_pattern_works_for_a_small_size() {
        let img = bar_edge_pattern(6, 5, 1);
        assert_eq!(img.dimensions(), (6, 5));
    }

    #[test]
    fn wide_pattern_has_the_size_and_the_lines_of_the_unit() {
        let img = wide_pattern(800, 112, 176);
        assert_eq!(img.dimensions(), (800, 112));
        // A thick line at each multiple of the unit.
        for x in [176u32, 352, 528, 704] {
            for dx in 0..3 {
                assert_eq!(img.get_pixel(x + dx, 56).0, [255, 255, 255], "x = {x}");
            }
        }
        // A thin line at a quarter of the unit, and none at the next pixel.
        assert_eq!(img.get_pixel(44, 56).0, [200, 200, 200]);
        assert_ne!(img.get_pixel(45, 56).0, [200, 200, 200]);
        // The colour changes along x.
        assert_ne!(img.get_pixel(100, 56), img.get_pixel(600, 56));
        // The border is on the outer edge of the whole image.
        assert_eq!(img.get_pixel(799, 56).0, EDGE_COLOR);
        assert_eq!(img.get_pixel(400, 111).0, EDGE_COLOR);
    }

    #[test]
    fn wide_pattern_works_for_a_unit_of_zero() {
        assert_eq!(wide_pattern(40, 20, 0).dimensions(), (40, 20));
    }

    #[test]
    fn icon_image_has_the_size_and_is_not_a_flat_colour() {
        for (w, h) in [(112u16, 112u16), (176, 112)] {
            let img = icon_image(w, h, 12, "Play", 0);
            assert_eq!(img.dimensions(), (w as u32, h as u32));
            assert!(distinct_colors(&img) > 100, "a gradient has many colours");
            // The corner is outside the rounded rectangle.
            assert_eq!(img.get_pixel(0, 0).0, [22, 22, 28]);
            // White text pixels and white symbol pixels exist.
            let white = img.pixels().filter(|p| p.0 == [255, 255, 255]).count();
            assert!(white > 100, "{white} white pixels");
        }
    }

    #[test]
    fn icon_images_differ_by_slot_and_by_variant() {
        let a = icon_image(112, 112, 6, "MUTE", 0);
        assert_eq!(a, icon_image(112, 112, 6, "MUTE", 0));
        assert_ne!(a, icon_image(112, 112, 7, "MUTE", 0));
        assert_ne!(a, icon_image(112, 112, 6, "MUTE", 1));
        assert_ne!(a, icon_image(112, 112, 6, "PLAY", 0));
    }

    #[test]
    fn icon_image_works_for_a_small_size() {
        assert_eq!(
            icon_image(10, 8, 1, "LONG WORD HERE", 0).dimensions(),
            (10, 8)
        );
    }
}
