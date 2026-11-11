//! Draw the label of a surface.
//!
//! On macOS, Core Text makes the layout and draws the text. It selects the
//! fonts for each script, so text that is not Latin shows correctly. The
//! reasons are in `docs/decisions/0010-core-text-for-surface-labels.md`.
//!
//! The label is at the centre of its area. A text that is too wide goes to
//! more lines. If the text does not fit, the font becomes smaller. If the
//! text does not fit at the smallest size, the end is cut and gets `…`.
//!
//! On an operating system that is not macOS, the module draws no text. No
//! type of Core Text is in the interface of the module.

use crate::profile::Color;
use image::RgbImage;

/// The font sizes that the label tries, in pixels, from the first to the
/// smallest.
pub const FONT_SIZES: [f64; 6] = [20.0, 18.0, 16.0, 14.0, 12.0, 10.0];

/// The largest part of the height of a surface that the band of a label can
/// have, when the surface also has an image.
const BAND_PART: f64 = 0.4;

/// The space above and below the text in a band, in pixels.
const BAND_SPACE: u32 = 3;

/// A rectangle on a surface image, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The height of the band at the bottom of a surface that the label `text`
/// needs, for a surface with an image. `width` is the width for the text, and
/// `surface_height` is the height of the surface.
pub fn band_height(text: &str, width: u32, surface_height: u32) -> u32 {
    let limit = (f64::from(surface_height) * BAND_PART) as u32;
    let text_height = platform::fit_height(text, width, limit.saturating_sub(2 * BAND_SPACE));
    (text_height + 2 * BAND_SPACE).min(limit)
}

/// Draw `text` in `color` at the centre of `area` on `canvas`. The parts of
/// `area` that are not on the canvas are ignored.
pub fn draw(canvas: &mut RgbImage, area: Area, text: &str, color: Color) {
    let x = area.x.min(canvas.width());
    let y = area.y.min(canvas.height());
    let area = Area {
        x,
        y,
        width: area.width.min(canvas.width() - x),
        height: area.height.min(canvas.height() - y),
    };
    if area.width == 0 || area.height == 0 || text.trim().is_empty() {
        return;
    }
    platform::draw(canvas, area, text, color);
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{Area, Color, RgbImage};

    pub fn fit_height(_text: &str, _width: u32, limit: u32) -> u32 {
        limit
    }

    pub fn draw(_canvas: &mut RgbImage, _area: Area, _text: &str, _color: Color) {}
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{Area, Color, RgbImage, FONT_SIZES};
    use core_foundation::attributed_string::CFMutableAttributedString;
    use core_foundation::base::{CFRange, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::CFString;
    use core_graphics::base::{kCGBitmapByteOrder32Big, kCGImageAlphaPremultipliedLast};
    use core_graphics::color_space::CGColorSpace;
    use core_graphics::context::CGContext;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    use core_graphics::path::CGPath;
    use core_text::font::{kCTFontEmphasizedSystemFontType, new_ui_font_for_language};
    use core_text::framesetter::CTFramesetter;
    use core_text::line::CTLine;
    use core_text::string_attributes::{
        kCTFontAttributeName, kCTForegroundColorFromContextAttributeName,
    };

    /// A height for the layout frame that no label needs. The layout then
    /// has each line of the text.
    const TALL: f64 = 10_000.0;

    /// One line of a layout.
    struct Line {
        line: CTLine,
        width: f64,
        /// The baseline, as the distance down from the top of the text block.
        baseline: f64,
    }

    /// The lines of a text at one font size, wrapped to a width.
    struct Layout {
        lines: Vec<Line>,
        width: f64,
        height: f64,
    }

    fn layout(text: &str, size: f64, width: u32) -> Layout {
        // The system font, in the weight for emphasis: a label on a small
        // display is easier to read in this weight.
        let font = new_ui_font_for_language(kCTFontEmphasizedSystemFontType, size, None);
        let mut styled = CFMutableAttributedString::new();
        styled.replace_str(&CFString::new(text), CFRange::init(0, 0));
        let range = CFRange::init(0, styled.char_len());
        // SAFETY: the two names are attribute names of Core Text, and each
        // value has the type that its attribute needs: a font, and a Boolean.
        unsafe {
            styled.set_attribute(range, kCTFontAttributeName, &font);
            styled.set_attribute(
                range,
                kCTForegroundColorFromContextAttributeName,
                &CFBoolean::true_value(),
            );
        }
        let setter = CTFramesetter::new_with_attributed_string(styled.as_concrete_TypeRef());
        let path = CGPath::from_rect(
            CGRect::new(
                &CGPoint::new(0.0, 0.0),
                &CGSize::new(f64::from(width), TALL),
            ),
            None,
        );
        let frame = setter.create_frame(CFRange::init(0, 0), &path);
        let lines = frame.get_lines();
        let origins = frame.get_line_origins(None);

        // The origins are from the bottom of the frame, and y goes up.
        let mut top = 0.0;
        let mut bottom = 0.0;
        let mut out = Vec::with_capacity(lines.len());
        for (i, (line, origin)) in lines.into_iter().zip(&origins).enumerate() {
            let bounds = line.get_typographic_bounds();
            if i == 0 {
                top = origin.y + bounds.ascent;
            }
            bottom = origin.y - bounds.descent;
            out.push(Line {
                line,
                width: bounds.width,
                baseline: top - origin.y,
            });
        }
        Layout {
            width: out.iter().map(|l| l.width).fold(0.0, f64::max),
            height: top - bottom,
            lines: out,
        }
    }

    fn fits(layout: &Layout, width: u32, height: u32) -> bool {
        layout.width <= f64::from(width) + 0.5 && layout.height <= f64::from(height) + 0.5
    }

    /// The layout of `text` for an area of `width` × `height`: at the first
    /// font size at which the text fits, or at the smallest size with the end
    /// of the text cut.
    fn fit(text: &str, width: u32, height: u32) -> Layout {
        let text = text.trim();
        for size in FONT_SIZES {
            let layout = layout(text, size, width);
            if fits(&layout, width, height) {
                return layout;
            }
        }
        // Find the largest number of characters that fits with `…` after
        // them. A cut can divide a group of characters that make one sign.
        let smallest = FONT_SIZES[FONT_SIZES.len() - 1];
        let ends: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        if ends.is_empty() {
            return layout("", smallest, width);
        }
        let cut = |n: usize| format!("{}…", text[..ends[n]].trim_end());
        let (mut low, mut high) = (0, ends.len() - 1);
        while low < high {
            let middle = (low + high).div_ceil(2);
            if fits(&layout(&cut(middle), smallest, width), width, height) {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        layout(&cut(low), smallest, width)
    }

    pub fn fit_height(text: &str, width: u32, limit: u32) -> u32 {
        if width == 0 || limit == 0 {
            return limit;
        }
        (fit(text, width, limit).height.ceil() as u32).min(limit)
    }

    pub fn draw(canvas: &mut RgbImage, area: Area, text: &str, color: Color) {
        let layout = fit(text, area.width, area.height);
        let (w, h) = (area.width as usize, area.height as usize);
        let mut context = CGContext::create_bitmap_context(
            None,
            w,
            h,
            8,
            w * 4,
            &CGColorSpace::create_device_rgb(),
            kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big,
        );

        // Copy the area to the bitmap, draw on it, and copy it back. The
        // first row of the bitmap data is the top row.
        let (canvas_width, x0, y0) = (canvas.width() as usize, area.x as usize, area.y as usize);
        let row = |y: usize| {
            let start = ((y0 + y) * canvas_width + x0) * 3;
            start..start + w * 3
        };
        for y in 0..h {
            let source = &canvas.as_raw()[row(y)];
            let target = &mut context.data()[y * w * 4..(y + 1) * w * 4];
            let (source, target) = (source.as_chunks::<3>().0, target.as_chunks_mut::<4>().0);
            for (rgb, rgba) in source.iter().zip(target) {
                *rgba = [rgb[0], rgb[1], rgb[2], 255];
            }
        }

        let [r, g, b] = color.0.map(|v| f64::from(v) / 255.0);
        context.set_rgb_fill_color(r, g, b, 1.0);
        context.set_should_antialias(true);
        context.set_should_smooth_fonts(false);
        // The text position is from the bottom of the bitmap, and y goes up.
        let top = (f64::from(area.height) - layout.height) / 2.0;
        for line in &layout.lines {
            context.set_text_position(
                ((f64::from(area.width) - line.width) / 2.0).round(),
                (f64::from(area.height) - top - line.baseline).round(),
            );
            line.line.draw(&context);
        }

        for y in 0..h {
            let source = &context.data()[y * w * 4..(y + 1) * w * 4];
            let target = &mut canvas.as_mut()[row(y)];
            let (source, target) = (source.as_chunks::<4>().0, target.as_chunks_mut::<3>().0);
            for (rgb, rgba) in target.iter_mut().zip(source) {
                *rgb = [rgba[0], rgba[1], rgba[2]];
            }
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use image::Rgb;

    const DARK: Rgb<u8> = Rgb([20, 30, 50]);

    fn canvas(width: u32, height: u32) -> RgbImage {
        RgbImage::from_pixel(width, height, DARK)
    }

    fn full(image: &RgbImage) -> Area {
        Area {
            x: 0,
            y: 0,
            width: image.width(),
            height: image.height(),
        }
    }

    /// The smallest rectangle that contains each pixel that is not the
    /// colour of the canvas: left, top, right, bottom. `None` if no pixel
    /// changed.
    fn ink(image: &RgbImage) -> Option<(u32, u32, u32, u32)> {
        let mut bounds: Option<(u32, u32, u32, u32)> = None;
        for (x, y, pixel) in image.enumerate_pixels() {
            if *pixel != DARK {
                let (l, t, r, b) = bounds.unwrap_or((x, y, x, y));
                bounds = Some((l.min(x), t.min(y), r.max(x), b.max(y)));
            }
        }
        bounds
    }

    fn drawn(width: u32, height: u32, text: &str) -> RgbImage {
        let mut image = canvas(width, height);
        let area = full(&image);
        draw(&mut image, area, text, Color::WHITE);
        image
    }

    #[test]
    fn a_label_is_at_the_centre_and_in_the_colour() {
        let image = drawn(112, 112, "Mail");
        let (left, top, right, bottom) = ink(&image).expect("the label shows");
        // The space on the two sides, and above and below, is almost equal.
        assert!(
            (left as i32 - (111 - right) as i32).abs() <= 3,
            "{left} {right}"
        );
        assert!(
            (top as i32 - (111 - bottom) as i32).abs() <= 6,
            "{top} {bottom}"
        );
        // One line of the first font size.
        assert!(bottom - top < 24, "{top} {bottom}");
        assert!(image
            .pixels()
            .any(|p| p[0] > 240 && p[1] > 240 && p[2] > 240));

        let mut red = canvas(112, 112);
        let area = full(&red);
        draw(&mut red, area, "Mail", Color::rgb(255, 0, 0));
        assert!(red.pixels().any(|p| p[0] > 240 && p[1] < 40 && p[2] < 60));
    }

    #[test]
    fn a_long_label_goes_to_more_lines_and_stays_in_the_area() {
        let image = drawn(112, 112, "Volume up and down");
        let (left, top, right, bottom) = ink(&image).unwrap();
        assert!(bottom - top > 30, "more than one line: {top} {bottom}");
        assert!(left >= 1 && right <= 110 && top >= 1 && bottom <= 110);
    }

    #[test]
    fn a_label_that_does_not_fit_is_cut_and_stays_in_the_area() {
        let text = "This label is much too long for a key face. ".repeat(20);
        let mut image = canvas(112, 112);
        let area = Area {
            x: 10,
            y: 20,
            width: 80,
            height: 50,
        };
        draw(&mut image, area, &text, Color::WHITE);
        let (left, top, right, bottom) = ink(&image).unwrap();
        assert!(left >= 10 && right < 90, "{left} {right}");
        assert!(top >= 20 && bottom < 70, "{top} {bottom}");
        // The text uses most of the area.
        assert!(right - left > 50 && bottom - top > 30);
    }

    #[test]
    fn one_long_word_stays_in_the_area() {
        let image = drawn(112, 112, &"W".repeat(60));
        let (left, top, right, bottom) = ink(&image).unwrap();
        assert!(left >= 1 && right <= 110 && top >= 1 && bottom <= 110);
    }

    #[test]
    fn text_that_is_not_latin_shows() {
        // Devanagari, CJK, Arabic, and an emoji.
        for text in ["नमस्ते", "音量", "مرحبا", "🔊"] {
            assert!(ink(&drawn(112, 112, text)).is_some(), "{text}");
        }
    }

    #[test]
    fn an_empty_label_and_an_empty_area_draw_nothing() {
        for text in ["", "   ", "\n"] {
            assert_eq!(ink(&drawn(112, 112, text)), None, "{text:?}");
        }
        let mut image = canvas(112, 112);
        for area in [
            Area {
                x: 0,
                y: 0,
                width: 0,
                height: 50,
            },
            Area {
                x: 0,
                y: 0,
                width: 50,
                height: 0,
            },
            Area {
                x: 200,
                y: 200,
                width: 50,
                height: 50,
            },
        ] {
            draw(&mut image, area, "Mail", Color::WHITE);
        }
        assert_eq!(ink(&image), None);
        // An area that is larger than the canvas is cut to the canvas.
        let area = Area {
            x: 100,
            y: 100,
            width: 500,
            height: 500,
        };
        draw(&mut image, area, "Mail", Color::WHITE);
    }

    #[test]
    fn the_band_is_as_high_as_the_text_needs_and_not_more_than_its_limit() {
        let one = band_height("Mail", 104, 112);
        let two = band_height("Volume up and down", 104, 112);
        let many = band_height(&"word ".repeat(50), 104, 112);
        assert!((16..=30).contains(&one), "{one}");
        assert!(two > one, "{one} {two}");
        assert!(two <= 44 && many <= 44, "{two} {many}");
    }
}
