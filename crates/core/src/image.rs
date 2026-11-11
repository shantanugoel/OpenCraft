//! Panel rendering.
//!
//! The firmware expects images already rotated 180 degrees (per the vendor
//! SDK's `key_image_format`), encoded as PNG at the panel's exact size.

use crate::error::Result;
use image::{ImageFormat, RgbImage};
use std::io::Cursor;

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
