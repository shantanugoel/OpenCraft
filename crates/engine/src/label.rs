//! Draw the label of a surface.
//!
//! On macOS, Core Text makes the layout and draws the text. It selects the
//! fonts for each script, so text that is not Latin shows correctly. The
//! reasons are in `docs/decisions/0010-core-text-for-surface-labels.md`.
//!
//! The label is at the top, at the centre, or at the bottom of its area,
//! and at the centre from left to right. A text that is too wide goes to
//! more lines. If the text does not fit at the size of its [`Font`], the font
//! becomes smaller in steps of 2 pixels. If the text does not fit at the
//! smallest size, the end is cut and gets `…`.
//!
//! On an operating system that is not macOS, the module draws no text. No
//! type of Core Text is in the interface of the module.

use crate::profile::{Color, DEFAULT_LABEL_SIZE, MIN_LABEL_SIZE};
use image::RgbImage;

/// The font of a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Font {
    /// The size that the label tries first, in pixels.
    pub size: u8,
    /// Bold, or the regular weight.
    pub bold: bool,
}

impl Default for Font {
    /// The font of a label of the earlier versions.
    fn default() -> Self {
        Self {
            size: DEFAULT_LABEL_SIZE,
            bold: true,
        }
    }
}

impl Font {
    /// The font sizes that the label tries, in pixels, from the first to the
    /// smallest: the size of the font, and then 2 pixels smaller each time,
    /// to [`MIN_LABEL_SIZE`].
    pub fn sizes(self) -> Vec<f64> {
        let mut sizes: Vec<f64> = (MIN_LABEL_SIZE..=self.size.max(MIN_LABEL_SIZE))
            .rev()
            .step_by(2)
            .map(f64::from)
            .collect();
        if sizes.last() != Some(&f64::from(MIN_LABEL_SIZE)) {
            sizes.push(f64::from(MIN_LABEL_SIZE));
        }
        sizes
    }
}

/// Where the text block is in its area, from top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Top,
    Middle,
    Bottom,
}

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

/// The height of the band above or below the image of a surface that the
/// label `text` needs. `width` is the width for the text, and
/// `surface_height` is the height of the surface.
pub fn band_height(text: &str, font: Font, width: u32, surface_height: u32) -> u32 {
    let limit = (f64::from(surface_height) * BAND_PART) as u32;
    let text_height = platform::fit_height(text, font, width, limit.saturating_sub(2 * BAND_SPACE));
    (text_height + 2 * BAND_SPACE).min(limit)
}

/// Draw `text` in `font` and `color` in `area` on `canvas`, at the vertical
/// place `align`. The parts of `area` that are not on the canvas are
/// ignored.
pub fn draw(canvas: &mut RgbImage, area: Area, text: &str, font: Font, color: Color, align: Align) {
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
    platform::draw(canvas, area, text, font, color, align);
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{Align, Area, Color, Font, RgbImage};

    pub fn fit_height(_text: &str, _font: Font, _width: u32, limit: u32) -> u32 {
        limit
    }

    pub fn draw(
        _canvas: &mut RgbImage,
        _area: Area,
        _text: &str,
        _font: Font,
        _color: Color,
        _align: Align,
    ) {
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{Align, Area, Color, Font, RgbImage};
    use core_foundation::attributed_string::CFMutableAttributedString;
    use core_foundation::base::{CFRange, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::CFString;
    use core_graphics::base::{kCGBitmapByteOrder32Big, kCGImageAlphaPremultipliedLast};
    use core_graphics::color_space::CGColorSpace;
    use core_graphics::context::CGContext;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    use core_graphics::path::CGPath;
    use core_text::font::{
        kCTFontEmphasizedSystemFontType, kCTFontSystemFontType, new_ui_font_for_language,
    };
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

    fn layout(text: &str, size: f64, bold: bool, width: u32) -> Layout {
        // The system font. Bold is the weight for emphasis: a label on a
        // small display is easier to read in this weight.
        let kind = if bold {
            kCTFontEmphasizedSystemFontType
        } else {
            kCTFontSystemFontType
        };
        let font = new_ui_font_for_language(kind, size, None);
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
    fn fit(text: &str, font: Font, width: u32, height: u32) -> Layout {
        let text = text.trim();
        let sizes = font.sizes();
        for &size in &sizes {
            let layout = layout(text, size, font.bold, width);
            if fits(&layout, width, height) {
                return layout;
            }
        }
        // Find the largest number of characters that fits with `…` after
        // them. A cut can divide a group of characters that make one sign.
        let smallest = sizes[sizes.len() - 1];
        let ends: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        if ends.is_empty() {
            return layout("", smallest, font.bold, width);
        }
        let cut = |n: usize| format!("{}…", text[..ends[n]].trim_end());
        let (mut low, mut high) = (0, ends.len() - 1);
        while low < high {
            let middle = (low + high).div_ceil(2);
            if fits(
                &layout(&cut(middle), smallest, font.bold, width),
                width,
                height,
            ) {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        layout(&cut(low), smallest, font.bold, width)
    }

    pub fn fit_height(text: &str, font: Font, width: u32, limit: u32) -> u32 {
        if width == 0 || limit == 0 {
            return limit;
        }
        (fit(text, font, width, limit).height.ceil() as u32).min(limit)
    }

    pub fn draw(
        canvas: &mut RgbImage,
        area: Area,
        text: &str,
        font: Font,
        color: Color,
        align: Align,
    ) {
        let layout = fit(text, font, area.width, area.height);
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
        let space = (f64::from(area.height) - layout.height).max(0.0);
        let top = match align {
            Align::Top => 0.0,
            Align::Middle => space / 2.0,
            Align::Bottom => space,
        };
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

    /// Draw with the default font, at the middle.
    fn put(canvas: &mut RgbImage, area: Area, text: &str, color: Color) {
        draw(canvas, area, text, Font::default(), color, Align::Middle);
    }

    fn drawn_with(text: &str, font: Font, align: Align) -> RgbImage {
        let mut image = canvas(112, 112);
        let area = full(&image);
        draw(&mut image, area, text, font, Color::WHITE, align);
        image
    }

    #[test]
    fn the_sizes_go_down_in_steps_of_two_to_the_smallest() {
        let sizes = |size| Font { size, bold: true }.sizes();
        assert_eq!(sizes(20), [20.0, 18.0, 16.0, 14.0, 12.0, 10.0]);
        assert_eq!(sizes(15), [15.0, 13.0, 11.0, 10.0]);
        assert_eq!(sizes(10), [10.0]);
        assert_eq!(sizes(4), [10.0]);
    }

    #[test]
    fn the_alignment_puts_the_text_at_the_top_the_middle_or_the_bottom() {
        let font = Font::default();
        let (_, top, _, _) = ink(&drawn_with("Mail", font, Align::Top)).unwrap();
        let (_, middle_top, _, middle_bottom) =
            ink(&drawn_with("Mail", font, Align::Middle)).unwrap();
        let (_, _, _, bottom) = ink(&drawn_with("Mail", font, Align::Bottom)).unwrap();
        assert!(top < 8, "{top}");
        assert!(bottom > 104, "{bottom}");
        assert!(
            middle_top > 35 && middle_bottom < 77,
            "{middle_top} {middle_bottom}"
        );
    }

    #[test]
    fn the_size_and_the_weight_change_the_text() {
        let font = |size, bold| Font { size, bold };
        let width = |font| {
            let (left, _, right, _) = ink(&drawn_with("Mail", font, Align::Middle)).unwrap();
            right - left
        };
        let pixels = |font| {
            drawn_with("Mail", font, Align::Middle)
                .pixels()
                .filter(|p| **p != DARK)
                .count()
        };
        let regular = font(20, false);
        assert!(width(font(32, false)) > width(regular) + 10);
        assert!(width(font(12, false)) + 10 < width(regular));
        assert!(pixels(font(20, true)) > pixels(regular) * 11 / 10);
    }

    fn drawn(width: u32, height: u32, text: &str) -> RgbImage {
        let mut image = canvas(width, height);
        let area = full(&image);
        put(&mut image, area, text, Color::WHITE);
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
        put(&mut red, area, "Mail", Color::rgb(255, 0, 0));
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
        put(&mut image, area, &text, Color::WHITE);
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
            put(&mut image, area, "Mail", Color::WHITE);
        }
        assert_eq!(ink(&image), None);
        // An area that is larger than the canvas is cut to the canvas.
        let area = Area {
            x: 100,
            y: 100,
            width: 500,
            height: 500,
        };
        put(&mut image, area, "Mail", Color::WHITE);
    }

    #[test]
    fn the_band_is_as_high_as_the_text_needs_and_not_more_than_its_limit() {
        let font = Font::default();
        let one = band_height("Mail", font, 104, 112);
        let two = band_height("Volume up and down", font, 104, 112);
        let many = band_height(&"word ".repeat(50), font, 104, 112);
        assert!((16..=30).contains(&one), "{one}");
        assert!(two > one, "{one} {two}");
        assert!(two <= 44 && many <= 44, "{two} {many}");
        // A smaller font needs a lower band.
        let small = band_height(
            "Mail",
            Font {
                size: 10,
                bold: true,
            },
            104,
            112,
        );
        assert!(small < one, "{small} {one}");
    }
}
