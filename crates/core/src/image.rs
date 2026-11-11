//! Images for surfaces.
//!
//! The descriptor gives the size, the image format, and the rotation of each
//! surface. The functions here make an upright image, turn it as the surface
//! needs, and encode it in the format of the surface. See [`encode_for`].

use crate::device::{ImageFormat, Rotation, Surface};
use crate::error::Result;
use image::codecs::jpeg::JpegEncoder;
use image::{imageops::FilterType, Rgb, RgbImage};
use std::io::Cursor;

mod patterns;

pub use patterns::{bar_edge_pattern, icon_image, wide_pattern, EDGE_COLOR};

/// The JPEG quality to use when the caller gives none. The SDK uses this
/// value for the background (`encode_background_frame`).
pub const DEFAULT_JPEG_QUALITY: u8 = 60;

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

/// 5x7 bitmap glyphs for A-Z.
const LETTERS: [[u8; 7]; 26] = [
    [
        0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
    ], // A
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
    ], // B
    [
        0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
    ], // C
    [
        0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
    ], // D
    [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
    ], // E
    [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
    ], // F
    [
        0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
    ], // G
    [
        0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
    ], // H
    [
        0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ], // I
    [
        0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
    ], // J
    [
        0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
    ], // K
    [
        0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
    ], // L
    [
        0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
    ], // M
    [
        0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001,
    ], // N
    [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ], // O
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
    ], // P
    [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
    ], // Q
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
    ], // R
    [
        0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
    ], // S
    [
        0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
    ], // T
    [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ], // U
    [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
    ], // V
    [
        0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
    ], // W
    [
        0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
    ], // X
    [
        0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
    ], // Y
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
    ], // Z
];

/// 5x7 bitmap glyph for ' ' (a space).
const SPACE: [u8; 7] = [0; 7];

/// 5x7 bitmap glyph for '-'.
const DASH: [u8; 7] = [
    0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
];

/// 5x7 bitmap glyph for '.'.
const DOT: [u8; 7] = [
    0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
];

/// The glyph of `c`, or `None` if the font has no glyph for it. The font has
/// the digits, the letters (a lower case letter has the glyph of its upper
/// case letter), and ' ', ',', '-', '.'.
fn glyph(c: char) -> Option<&'static [u8; 7]> {
    match c {
        '0'..='9' => Some(&GLYPHS[c as usize - '0' as usize]),
        'A'..='Z' => Some(&LETTERS[c as usize - 'A' as usize]),
        'a'..='z' => Some(&LETTERS[c as usize - 'a' as usize]),
        ' ' => Some(&SPACE),
        ',' => Some(&COMMA),
        '-' => Some(&DASH),
        '.' => Some(&DOT),
        _ => None,
    }
}

/// Turn the upright image `img` as `surface` needs, and encode it in the
/// format of `surface`. The result is ready to send.
///
/// `img` must have the size of the surface. `jpeg_quality` (1 to 100) applies
/// only to a JPEG surface.
pub fn encode_for(surface: &Surface, img: &RgbImage, jpeg_quality: u8) -> Result<Vec<u8>> {
    encode_as(surface, img, surface.format, jpeg_quality)
}

/// Turn the upright image `img` as `surface` needs, and encode it in `format`.
/// The format of the descriptor does not apply. The descriptor does not
/// change.
///
/// This is for a measurement, for example to send JPEG to a surface that has
/// PNG in the descriptor. The size of `img` can be different from the size of
/// the surface. `jpeg_quality` (1 to 100) applies only to JPEG.
pub fn encode_as(
    surface: &Surface,
    img: &RgbImage,
    format: ImageFormat,
    jpeg_quality: u8,
) -> Result<Vec<u8>> {
    let turned;
    let img = match surface.rotation {
        Rotation::Deg0 => img,
        Rotation::Deg180 => {
            turned = image::imageops::rotate180(img);
            &turned
        }
    };
    match format {
        ImageFormat::Png => {
            let mut out = Cursor::new(Vec::new());
            img.write_to(&mut out, image::ImageFormat::Png)?;
            Ok(out.into_inner())
        }
        ImageFormat::Jpeg => encode_jpeg(img, jpeg_quality),
    }
}

/// Draw `number` on a test image for `surface`, and return the image ready to
/// send.
pub fn render_test_surface(surface: &Surface, number: u8, bg: [u8; 3]) -> Result<Vec<u8>> {
    let (w, h) = (surface.width as u32, surface.height as u32);
    let mut img = RgbImage::from_pixel(w, h, Rgb(bg));

    // A marker in the top left and in the bottom right corner shows the
    // orientation on the device.
    let mark = Rgb([255, 255, 255]);
    let m = (h / 16).max(3);
    for y in 0..m {
        for x in 0..m {
            img.put_pixel(x, y, mark);
            img.put_pixel(w - 1 - x, h - 1 - y, mark);
        }
    }

    // The number, large, at the centre.
    let scale = (h / 14).max(2);
    draw_number(&mut img, number, scale, Rgb([255, 255, 255]));

    encode_for(surface, &img, DEFAULT_JPEG_QUALITY)
}

/// Fill `surface` with one colour, and return the image ready to send.
pub fn render_solid(surface: &Surface, bg: [u8; 3]) -> Result<Vec<u8>> {
    let img = RgbImage::from_pixel(surface.width as u32, surface.height as u32, Rgb(bg));
    encode_for(surface, &img, DEFAULT_JPEG_QUALITY)
}

fn draw_number(img: &mut RgbImage, number: u8, scale: u32, color: Rgb<u8>) {
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

/// Draw the background test pattern for `surface`, and return the image ready
/// to send. `quality` applies if the surface format is JPEG.
pub fn render_background_test(surface: &Surface, quality: u8) -> Result<Vec<u8>> {
    let img = background_test_pattern(surface.width, surface.height);
    encode_for(surface, &img, quality)
}

/// Make an image for `surface` from an image file (PNG or JPEG), and return it
/// ready to send.
///
/// The function changes the image to the size of the surface (the aspect
/// ratio can change, as in the SDK). It puts transparent pixels on black.
/// Then it turns and encodes the image as the surface needs.
pub fn render_background_file(data: &[u8], surface: &Surface, quality: u8) -> Result<Vec<u8>> {
    let src = image::load_from_memory(data)?;
    let src = src
        .resize_exact(
            surface.width as u32,
            surface.height as u32,
            FilterType::Triangle,
        )
        .to_rgba8();
    let img = RgbImage::from_fn(src.width(), src.height(), |x, y| {
        let [r, g, b, a] = src.get_pixel(x, y).0;
        let over_black = |c: u8| ((c as u16 * a as u16 + 127) / 255) as u8;
        Rgb([over_black(r), over_black(g), over_black(b)])
    });
    encode_for(surface, &img, quality)
}

fn fill(img: &mut RgbImage, x0: u32, y0: u32, w: u32, h: u32, color: Rgb<u8>) {
    for y in y0..(y0 + h).min(img.height()) {
        for x in x0..(x0 + w).min(img.width()) {
            img.put_pixel(x, y, color);
        }
    }
}

/// Draw `text` with its top left corner at `x`, `y`. See [`glyph`] for the
/// characters that have a glyph. The function skips each other character.
fn draw_text(img: &mut RgbImage, x: u32, y: u32, text: &str, scale: u32, color: Rgb<u8>) {
    let mut gx = x;
    for c in text.chars() {
        let Some(glyph) = glyph(c) else { continue };
        draw_glyph(img, gx, y, glyph, scale, color);
        gx += 6 * scale;
    }
}

/// The width in pixels of `text` as [`draw_text`] draws it.
fn text_width(text: &str, scale: u32) -> u32 {
    let n = text.chars().filter(|&c| glyph(c).is_some()).count() as u32;
    (n * 6).saturating_sub(1) * scale
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

/// Distinct, readable colours for numbered test images.
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
    use crate::device::{SurfaceId, SurfaceKind};

    /// A background of 800 x 480, JPEG, turned 180°. The values are test
    /// data, not a model.
    const BACKGROUND: Surface = Surface {
        id: SurfaceId::new(SurfaceKind::Background, 1),
        slot: None,
        width: 800,
        height: 480,
        format: ImageFormat::Jpeg,
        rotation: Rotation::Deg180,
    };

    /// A key face of 112 x 112, PNG, turned 180°.
    const KEY_FACE: Surface = Surface {
        id: SurfaceId::new(SurfaceKind::KeyFace, 1),
        slot: Some(6),
        width: 112,
        height: 112,
        format: ImageFormat::Png,
        rotation: Rotation::Deg180,
    };

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
        let jpeg = render_background_test(&BACKGROUND, DEFAULT_JPEG_QUALITY).unwrap();
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
        let low = render_background_test(&BACKGROUND, 30).unwrap();
        let high = render_background_test(&BACKGROUND, 95).unwrap();
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
        src.write_to(&mut png, image::ImageFormat::Png).unwrap();

        let jpeg = render_background_file(png.get_ref(), &BACKGROUND, 90).unwrap();
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
        let jpeg = render_background_file(&jpeg_in, &BACKGROUND, 90).unwrap();
        let img = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_near(&img, 400, 240, [0, 0, 255]);
    }

    #[test]
    fn background_file_refuses_data_that_is_not_an_image() {
        assert!(render_background_file(b"not an image", &BACKGROUND, 60).is_err());
    }

    /// An upright image of the size of `surface`: red in the top left
    /// quarter, blue in the other parts.
    fn quarter_image(surface: &Surface) -> RgbImage {
        let (w, h) = (surface.width as u32, surface.height as u32);
        RgbImage::from_fn(w, h, |x, y| {
            if x < w / 2 && y < h / 2 {
                Rgb([255, 0, 0])
            } else {
                Rgb([0, 0, 255])
            }
        })
    }

    #[test]
    fn encode_for_uses_the_format_and_the_rotation_of_the_surface() {
        // PNG, turned 180°: the red quarter is now at the bottom right.
        let png = encode_for(&KEY_FACE, &quarter_image(&KEY_FACE), 60).unwrap();
        assert_eq!(&png[..4], b"\x89PNG");
        let img = image::load_from_memory(&png).unwrap().to_rgb8();
        assert_eq!(img.dimensions(), (112, 112));
        assert_eq!(img.get_pixel(100, 100).0, [255, 0, 0]);
        assert_eq!(img.get_pixel(10, 10).0, [0, 0, 255]);

        // JPEG, not turned: the red quarter stays at the top left.
        let upright = Surface {
            format: ImageFormat::Jpeg,
            rotation: Rotation::Deg0,
            ..KEY_FACE
        };
        let jpeg = encode_for(&upright, &quarter_image(&upright), 90).unwrap();
        assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF], "JPEG start marker");
        let img = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_near(&img, 10, 10, [255, 0, 0]);
        assert_near(&img, 100, 100, [0, 0, 255]);
    }

    #[test]
    fn encode_as_overrides_the_format_and_keeps_the_rotation() {
        // KEY_FACE has PNG in the descriptor. Ask for JPEG: the data is a
        // JPEG, and it is turned 180° as the surface needs.
        let jpeg = encode_as(&KEY_FACE, &quarter_image(&KEY_FACE), ImageFormat::Jpeg, 90).unwrap();
        assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF], "JPEG start marker");
        let img = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_near(&img, 100, 100, [255, 0, 0]);
        assert_near(&img, 10, 10, [0, 0, 255]);

        // The same call with the format of the surface is encode_for.
        let upright = quarter_image(&KEY_FACE);
        assert_eq!(
            encode_as(&KEY_FACE, &upright, KEY_FACE.format, 60).unwrap(),
            encode_for(&KEY_FACE, &upright, 60).unwrap()
        );

        // A descriptor with JPEG gives PNG on request.
        let png = encode_as(
            &BACKGROUND,
            &quarter_image(&BACKGROUND),
            ImageFormat::Png,
            60,
        );
        assert_eq!(&png.unwrap()[..4], b"\x89PNG");
    }

    #[test]
    fn encode_as_accepts_an_image_that_is_not_the_size_of_the_surface() {
        let wide = RgbImage::from_pixel(352, 112, Rgb([1, 2, 3]));
        let png = encode_as(&KEY_FACE, &wide, ImageFormat::Png, 60).unwrap();
        let img = image::load_from_memory(&png).unwrap();
        assert_eq!((img.width(), img.height()), (352, 112));
    }

    #[test]
    fn font_has_digits_letters_and_a_few_signs() {
        for c in "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcxyz ,-.".chars() {
            assert!(glyph(c).is_some(), "no glyph for {c:?}");
        }
        assert!(glyph('?').is_none());
        // A lower case letter has the glyph of the upper case letter.
        assert_eq!(glyph('q'), glyph('Q'));
        // Each letter is different from each other letter, and has pixels.
        for (i, a) in LETTERS.iter().enumerate() {
            assert!(a.iter().any(|&row| row != 0), "empty glyph {i}");
            for b in &LETTERS[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn test_surface_and_solid_have_the_size_of_the_surface() {
        let bar_panel = Surface {
            id: SurfaceId::new(SurfaceKind::BarPanel, 1),
            width: 176,
            ..KEY_FACE
        };
        for surface in [KEY_FACE, bar_panel] {
            let solid = render_solid(&surface, [10, 20, 30]).unwrap();
            let solid = image::load_from_memory(&solid).unwrap().to_rgb8();
            let numbered = render_test_surface(&surface, 12, [10, 20, 30]).unwrap();
            let numbered = image::load_from_memory(&numbered).unwrap().to_rgb8();
            for img in [&solid, &numbered] {
                assert_eq!(img.dimensions(), (surface.width as u32, 112));
                assert_eq!(img.get_pixel(10, 40).0, [10, 20, 30]);
            }
            assert!(solid.pixels().all(|p| p.0 == [10, 20, 30]));
            let centre = surface.width as u32 / 2;
            assert!((40..72).any(|y| (centre - 16..centre + 16)
                .any(|x| numbered.get_pixel(x, y).0 == [255, 255, 255])));
            assert_eq!(numbered.get_pixel(0, 0).0, [255, 255, 255]);
        }
    }
}
