//! Panel rendering.
//!
//! The firmware expects images already rotated 180 degrees (per the vendor
//! SDK's `key_image_format`), encoded as PNG at the panel's exact size.
//!
//! The background is a JPEG, also turned 180° (per the SDK's
//! `touchscreen_image_format`).

use crate::error::Result;
use image::codecs::jpeg::JpegEncoder;
use image::{imageops::FilterType, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;

/// The JPEG quality that the SDK uses for the background
/// (`encode_background_frame`).
pub const BACKGROUND_JPEG_QUALITY: u8 = 60;

/// 5x7 bitmap glyphs for 0-9, one byte per row (5 significant bits).
const GLYPHS: [[u8; 7]; 10] = [
    [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ],
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
    ],
    [
        0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
    ],
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ],
    [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ],
    [
        0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
    ],
];

/// 5x7 bitmap glyph for ','.
const COMMA: [u8; 7] = [
    0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b00100, 0b01000,
];

/// Render a numbered test panel and return PNG bytes ready to send.
pub fn render_test_panel(width: u16, height: u16, number: u8, bg: [u8; 3]) -> Result<Vec<u8>> {
    let (w, h) = (width as u32, height as u32);
    let mut img = RgbImage::from_pixel(w, h, image::Rgb(bg));

    // Corner markers make orientation obvious on the physical device.
    let mark = image::Rgb([255, 255, 255]);
    let m = (h / 16).max(3);
    for y in 0..m {
        for x in 0..m {
            img.put_pixel(x, y, mark);
            img.put_pixel(w - 1 - x, h - 1 - y, mark);
        }
    }

    // Big centred slot number.
    let scale = (h / 14).max(2);
    draw_number(&mut img, number, scale, image::Rgb([255, 255, 255]));

    let img = image::imageops::rotate180(&img);
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png)?;
    Ok(out.into_inner())
}

/// Render a flat colour panel.
pub fn render_solid(width: u16, height: u16, bg: [u8; 3]) -> Result<Vec<u8>> {
    let img = RgbImage::from_pixel(width as u32, height as u32, image::Rgb(bg));
    let img = image::imageops::rotate180(&img);
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png)?;
    Ok(out.into_inner())
}

fn draw_number(img: &mut RgbImage, number: u8, scale: u32, color: image::Rgb<u8>) {
    let digits: Vec<u8> = number
        .to_string()
        .bytes()
        .filter_map(|b| b.checked_sub(b'0'))
        .collect();

    let glyph_w = 5 * scale;
    let gap = scale;
    let total_w = digits.len() as u32 * glyph_w + (digits.len() as u32 - 1) * gap;
    let total_h = 7 * scale;

    let ox = (img.width().saturating_sub(total_w)) / 2;
    let oy = (img.height().saturating_sub(total_h)) / 2;

    for (i, d) in digits.iter().enumerate() {
        let gx = ox + i as u32 * (glyph_w + gap);
        let glyph = &GLYPHS[(*d as usize).min(9)];
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5u32 {
                if bits & (1 << (4 - col)) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = gx + col * scale + dx;
                            let py = oy + row as u32 * scale + dy;
                            if px < img.width() && py < img.height() {
                                img.put_pixel(px, py, color);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Encode `img` as a JPEG. `quality` is 1 to 100. A value outside this range
/// is moved into it.
///
/// The function does not turn the image. Turn it before the call if the device
/// needs it.
pub fn encode_jpeg(img: &RgbImage, quality: u8) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100)).encode_image(img)?;
    Ok(out)
}

/// Colours of the corners of the background test pattern, upright:
/// top left, top right, bottom left, bottom right.
pub const BACKGROUND_CORNER_COLORS: [[u8; 3]; 4] =
    [[255, 0, 0], [0, 200, 0], [0, 80, 255], [255, 220, 0]];

/// Draw the background test pattern, upright (not turned).
///
/// The pattern helps a person find which part of the image shows on the
/// device, and in which orientation:
/// - A grey line each 40 pixels. Each 4th vertical line (each 160 pixels) and
///   each 2nd horizontal line (each 80 pixels) is brighter.
/// - The coordinates "x,y" as text at each bright crossing, below and to the
///   right of the crossing. The text for 0,0 is to the right of the corner
///   square.
/// - A 40 x 40 square of a different colour in each corner. See
///   [`BACKGROUND_CORNER_COLORS`].
/// - A white line 1 pixel wide at the outer edge.
pub fn background_test_pattern(width: u16, height: u16) -> RgbImage {
    let (w, h) = (width as u32, height as u32);
    let mut img = RgbImage::from_pixel(w, h, Rgb([16, 16, 24]));

    let minor = Rgb([70, 70, 80]);
    let major = Rgb([150, 150, 170]);
    for x in (0..w).step_by(40) {
        let color = if x % 160 == 0 { major } else { minor };
        for y in 0..h {
            img.put_pixel(x, y, color);
        }
    }
    for y in (0..h).step_by(40) {
        let color = if y % 80 == 0 { major } else { minor };
        for x in 0..w {
            img.put_pixel(x, y, color);
        }
    }

    let cell = 40.min(w).min(h);
    let corners = [(0, 0), (w - cell, 0), (0, h - cell), (w - cell, h - cell)];
    for ((cx, cy), color) in corners.into_iter().zip(BACKGROUND_CORNER_COLORS) {
        fill(&mut img, cx, cy, cell, cell, Rgb(color));
    }

    let text = Rgb([255, 255, 255]);
    for y in (0..h).step_by(80) {
        for x in (0..w).step_by(160) {
            // Keep the text out of the top left corner square.
            let tx = if x == 0 && y == 0 { cell + 3 } else { x + 3 };
            draw_text(&mut img, tx, y + 3, &format!("{x},{y}"), 2, text);
        }
    }

    let edge = Rgb([255, 255, 255]);
    for x in 0..w {
        img.put_pixel(x, 0, edge);
        img.put_pixel(x, h - 1, edge);
    }
    for y in 0..h {
        img.put_pixel(0, y, edge);
        img.put_pixel(w - 1, y, edge);
    }
    img
}

/// Render the background test pattern and return JPEG bytes ready to send.
pub fn render_background_test(width: u16, height: u16, quality: u8) -> Result<Vec<u8>> {
    let img = background_test_pattern(width, height);
    encode_jpeg(&image::imageops::rotate180(&img), quality)
}

/// Make a background from an image file (PNG or JPEG) and return JPEG bytes
/// ready to send.
///
/// The function changes the image to `width` x `height` (the aspect ratio can
/// change, as in the SDK). It puts transparent pixels on black, turns the
/// image 180°, and encodes it as a JPEG.
pub fn render_background_file(
    data: &[u8],
    width: u16,
    height: u16,
    quality: u8,
) -> Result<Vec<u8>> {
    let src = image::load_from_memory(data)?;
    let src = src
        .resize_exact(width as u32, height as u32, FilterType::Triangle)
        .to_rgba8();
    let img = RgbImage::from_fn(src.width(), src.height(), |x, y| {
        let [r, g, b, a] = src.get_pixel(x, y).0;
        let over_black = |c: u8| ((c as u16 * a as u16 + 127) / 255) as u8;
        Rgb([over_black(r), over_black(g), over_black(b)])
    });
    encode_jpeg(&image::imageops::rotate180(&img), quality)
}

fn fill(img: &mut RgbImage, x0: u32, y0: u32, w: u32, h: u32, color: Rgb<u8>) {
    for y in y0..(y0 + h).min(img.height()) {
        for x in x0..(x0 + w).min(img.width()) {
            img.put_pixel(x, y, color);
        }
    }
}

/// Draw `text` with its top left corner at `x`, `y`. Only the digits and ','
/// have a glyph. The function skips each other character.
fn draw_text(img: &mut RgbImage, x: u32, y: u32, text: &str, scale: u32, color: Rgb<u8>) {
    let mut gx = x;
    for c in text.chars() {
        let glyph = match c {
            '0'..='9' => &GLYPHS[c as usize - '0' as usize],
            ',' => &COMMA,
            _ => continue,
        };
        draw_glyph(img, gx, y, glyph, scale, color);
        gx += 6 * scale;
    }
}

fn draw_glyph(img: &mut RgbImage, x: u32, y: u32, glyph: &[u8; 7], scale: u32, color: Rgb<u8>) {
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..5u32 {
            if bits & (1 << (4 - col)) != 0 {
                fill(
                    img,
                    x + col * scale,
                    y + row as u32 * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

/// Distinct, readable colours for numbered test panels.
pub fn test_color(i: usize) -> [u8; 3] {
    let hue = (i as f32 * 0.6180339) % 1.0;
    hsv_to_rgb(hue, 0.75, 0.55)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match (i as i32).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The colour at `x`, `y` is near `want`. JPEG changes colours a little.
    fn assert_near(img: &RgbImage, x: u32, y: u32, want: [u8; 3]) {
        let got = img.get_pixel(x, y).0;
        for (g, w) in got.iter().zip(want) {
            assert!(
                g.abs_diff(w) <= 40,
                "pixel ({x}, {y}) is {got:?}, expected near {want:?}"
            );
        }
    }

    #[test]
    fn test_pattern_has_corners_border_and_grid() {
        let img = background_test_pattern(800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let [tl, tr, bl, br] = BACKGROUND_CORNER_COLORS;
        assert_eq!(img.get_pixel(20, 20).0, tl);
        assert_eq!(img.get_pixel(779, 20).0, tr);
        assert_eq!(img.get_pixel(20, 459).0, bl);
        assert_eq!(img.get_pixel(779, 459).0, br);
        for (x, y) in [(0, 0), (799, 0), (0, 479), (799, 479), (400, 0), (0, 240)] {
            assert_eq!(img.get_pixel(x, y).0, [255, 255, 255], "edge at ({x}, {y})");
        }
        // A grid line at x = 120 and at y = 200, and no line at x = 121.
        assert_eq!(img.get_pixel(120, 100).0, [70, 70, 80]);
        assert_eq!(img.get_pixel(100, 200).0, [70, 70, 80]);
        assert_eq!(img.get_pixel(121, 101).0, [16, 16, 24]);
    }

    #[test]
    fn background_test_is_a_jpeg_turned_180_degrees() {
        let jpeg = render_background_test(800, 480, BACKGROUND_JPEG_QUALITY).unwrap();
        assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF], "JPEG start marker");

        let img = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_eq!(img.dimensions(), (800, 480));
        // Turned 180°: the upright top left corner is now at the bottom right.
        let [tl, tr, bl, br] = BACKGROUND_CORNER_COLORS;
        assert_near(&img, 779, 459, tl);
        assert_near(&img, 20, 459, tr);
        assert_near(&img, 779, 20, bl);
        assert_near(&img, 20, 20, br);
    }

    #[test]
    fn higher_quality_gives_more_bytes() {
        let low = render_background_test(800, 480, 30).unwrap();
        let high = render_background_test(800, 480, 95).unwrap();
        assert!(high.len() > low.len());
    }

    #[test]
    fn background_file_is_scaled_turned_and_put_on_black() {
        // A 4 x 2 PNG: the left half is red, the right half is transparent.
        let src = image::RgbaImage::from_fn(4, 2, |x, _| {
            if x < 2 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 255, 0, 0])
            }
        });
        let mut png = Cursor::new(Vec::new());
        src.write_to(&mut png, ImageFormat::Png).unwrap();

        let jpeg = render_background_file(png.get_ref(), 800, 480, 90).unwrap();
        let img = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_eq!(img.dimensions(), (800, 480));
        // Turned 180°: red is now on the right, and black on the left.
        assert_near(&img, 700, 240, [255, 0, 0]);
        assert_near(&img, 100, 240, [0, 0, 0]);
    }

    #[test]
    fn background_file_accepts_a_jpeg() {
        let src = RgbImage::from_pixel(80, 48, Rgb([0, 0, 255]));
        let jpeg_in = encode_jpeg(&src, 90).unwrap();
        let jpeg = render_background_file(&jpeg_in, 800, 480, 90).unwrap();
        let img = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_near(&img, 400, 240, [0, 0, 255]);
    }

    #[test]
    fn background_file_refuses_data_that_is_not_an_image() {
        assert!(render_background_file(b"not an image", 800, 480, 60).is_err());
    }
}
