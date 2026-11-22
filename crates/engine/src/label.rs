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

use crate::profile::{Color, DEFAULT_LABEL_SIZE, LABEL_SIZES, MAX_LABEL_SIZE, MIN_LABEL_SIZE};
use image::RgbImage;

/// One installed font family (decision 0043, section 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFamily {
    /// The family name that the profile stores: the non-localized name of
    /// `CTFontCopyName`. A profile with this name loads on a computer with a
    /// different interface language.
    pub name: String,
    /// The family name for the interface. The list of Core Text can localize
    /// it. The picker shows this name and sends [`Self::name`].
    pub display: String,
}

/// The installed font families, sorted by the display name with no
/// difference between capital and small letters. A hidden system family (a
/// name that starts with `.`) and a duplicate are not in the list.
///
/// The function makes the list again at each call, thus a font that the user
/// installs while OpenCraft runs shows at the next call. The shell calls it
/// when the font picker opens. The engine makes the list at the first label
/// that names a family. The list is behind a lock, because the engine thread
/// draws and the GUI thread reads the list.
///
/// The list is empty on an operating system that is not macOS.
pub fn font_families() -> Vec<FontFamily> {
    platform::font_families()
}

/// The font of a label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    /// The size that the label tries first, in pixels.
    pub size: u8,
    /// Bold, or the regular weight.
    pub bold: bool,
    /// The family that the profile names. `None` is the system font. A
    /// family that is not installed draws in the system font, and the
    /// profile keeps the name (decision 0043, section 1).
    pub family: Option<String>,
    /// Italic. A family with no italic face is drawn with a shear of
    /// [`SLANT`] (decision 0043, section 1).
    pub italic: bool,
    /// A single underline, in the colour of the text.
    pub underline: bool,
}

impl Default for Font {
    /// The font of a label of the earlier versions.
    fn default() -> Self {
        Self {
            size: DEFAULT_LABEL_SIZE,
            bold: true,
            family: None,
            italic: false,
            underline: false,
        }
    }
}

impl Font {
    /// The font sizes that the label tries, in pixels, from the first to the
    /// smallest (decision 0043, section 3). At a size of 40 or less, the
    /// font size and then 2 pixels smaller each time to [`MIN_LABEL_SIZE`].
    /// Above 40, the font size, then each size of [`LABEL_SIZES`] above 40
    /// and below the font size, then 40, then the 2-pixel steps.
    pub fn sizes(&self) -> Vec<f64> {
        let size = self.size.clamp(MIN_LABEL_SIZE, MAX_LABEL_SIZE);
        if size <= 40 {
            let mut sizes: Vec<f64> = (MIN_LABEL_SIZE..=size)
                .rev()
                .step_by(2)
                .map(f64::from)
                .collect();
            if sizes.last() != Some(&f64::from(MIN_LABEL_SIZE)) {
                sizes.push(f64::from(MIN_LABEL_SIZE));
            }
            return sizes;
        }
        let mut sizes = vec![f64::from(size)];
        for large in LABEL_SIZES.into_iter().rev() {
            if large > 40 && large < size {
                sizes.push(f64::from(large));
            }
        }
        sizes.push(40.0);
        let mut step = 38;
        while step >= MIN_LABEL_SIZE {
            sizes.push(f64::from(step));
            step -= 2;
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

/// The shear of a synthetic italic, as a part of the ascent. A family with
/// no italic face is drawn with this shear, and no false italic face
/// (decision 0043, section 1).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const SLANT: f64 = 0.2;

/// The room that a single underline needs in the height of its line, as a
/// part of the font size: the position of the underline and its thickness.
/// Core Text has no binding for the two values, thus the module measures the
/// underline (see `docs/kb/text-rendering.md`): its bottom is 0.10 to 0.25 of
/// the size below the ink of the text, and 0.2 keeps a room for the fonts of
/// the measurement.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const UNDERLINE_PART: f64 = 0.2;

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
pub fn band_height(text: &str, font: &Font, width: u32, surface_height: u32) -> u32 {
    let limit = (f64::from(surface_height) * BAND_PART) as u32;
    let text_height = platform::fit_height(text, font, width, limit.saturating_sub(2 * BAND_SPACE));
    (text_height + 2 * BAND_SPACE).min(limit)
}

/// Draw `text` in `font` and `color` in `area` on `canvas`, at the vertical
/// place `align`. The parts of `area` that are not on the canvas are
/// ignored.
pub fn draw(
    canvas: &mut RgbImage,
    area: Area,
    text: &str,
    font: &Font,
    color: Color,
    align: Align,
) {
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
    use super::{Align, Area, Color, Font, FontFamily, RgbImage};

    /// No Core Text: the engine offers no font family.
    pub fn font_families() -> Vec<FontFamily> {
        Vec::new()
    }

    pub fn fit_height(_text: &str, _font: &Font, _width: u32, limit: u32) -> u32 {
        limit
    }

    pub fn draw(
        _canvas: &mut RgbImage,
        _area: Area,
        _text: &str,
        _font: &Font,
        _color: Color,
        _align: Align,
    ) {
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{Align, Area, Color, Font, FontFamily, RgbImage, SLANT, UNDERLINE_PART};
    use core_foundation::attributed_string::CFMutableAttributedString;
    use core_foundation::base::{CFRange, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::{CFString, CFStringRef};
    use core_graphics::base::{kCGBitmapByteOrder32Big, kCGImageAlphaPremultipliedLast, CGFloat};
    use core_graphics::color::CGColor;
    use core_graphics::color_space::CGColorSpace;
    use core_graphics::context::CGContext;
    use core_graphics::geometry::{CGAffineTransform, CGPoint, CGRect, CGSize};
    use core_graphics::path::CGPath;
    use core_graphics::sys::CGColorRef;
    use core_text::font::{
        kCTFontEmphasizedSystemFontType, kCTFontSystemFontType, new_from_descriptor,
        new_ui_font_for_language, CTFont,
    };
    use core_text::font_descriptor::{
        kCTFontBoldTrait, kCTFontFamilyNameAttribute, kCTFontItalicTrait, new_from_attributes,
    };
    use core_text::font_manager;
    use core_text::framesetter::CTFramesetter;
    use core_text::line::CTLine;
    use core_text::string_attributes::{
        kCTFontAttributeName, kCTForegroundColorAttributeName,
        kCTForegroundColorFromContextAttributeName, kCTUnderlineColorAttributeName,
        kCTUnderlineStyleAttributeName,
    };
    use std::sync::{Mutex, MutexGuard};

    /// A height for the layout frame that no label needs. The layout then
    /// has each line of the text.
    const TALL: f64 = 10_000.0;

    use core_graphics::sys::CGColorSpaceRef;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        /// The device RGB colour space of this Mac.
        fn CGColorSpaceCreateDeviceRGB() -> CGColorSpaceRef;
        fn CGColorSpaceRelease(space: CGColorSpaceRef);
        /// A colour of a colour space, from four components. The colour
        /// keeps the space, thus the caller releases its own reference.
        fn CGColorCreate(space: CGColorSpaceRef, components: *const CGFloat) -> CGColorRef;
    }

    /// The size for a font that the list of families reads. The family and
    /// not the size is the answer.
    const LIST_SIZE: f64 = 12.0;

    /// The installed font families of the last [`font_families`] call, or
    /// `None` before the first call. See [`installed_family`].
    static FAMILIES: Mutex<Option<Vec<FontFamily>>> = Mutex::new(None);

    pub fn font_families() -> Vec<FontFamily> {
        let mut cache = lock();
        let families = installed_families();
        *cache = Some(families.clone());
        families
    }

    fn lock() -> MutexGuard<'static, Option<Vec<FontFamily>>> {
        FAMILIES.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// The installed families from Core Text: the display name of each
    /// family, with the family name of the profile file. A hidden family
    /// (a name that starts with `.`) and a duplicate are not in the list.
    fn installed_families() -> Vec<FontFamily> {
        let names = font_manager::copy_available_font_family_names();
        let mut families: Vec<FontFamily> = Vec::new();
        for display in names.iter().map(|name| name.to_string()) {
            if display.starts_with('.') {
                continue;
            }
            // The list gives the name for the interface. The profile stores
            // the non-localized family name, which CTFontCopyName gives.
            let name = family_font(&display, LIST_SIZE).family_name();
            if families.iter().any(|family| family.name == name) {
                continue;
            }
            families.push(FontFamily { name, display });
        }
        families.sort_by(|a, b| {
            a.display
                .to_lowercase()
                .cmp(&b.display.to_lowercase())
                .then_with(|| a.display.cmp(&b.display))
        });
        families
    }

    /// `family` is installed: it is in the list of the last
    /// [`font_families`] call.
    ///
    /// The test is necessary because `CTFontCreateWithName` and a font
    /// descriptor take a name and give a best match, not an error. A missing
    /// family gives Helvetica. See decision 0043, section 1.
    fn installed_family(family: &str) -> bool {
        let mut cache = lock();
        let families = cache.get_or_insert_with(installed_families);
        families.iter().any(|known| known.name == family)
    }

    /// The font of `family` at `size`, from the family descriptor. The font
    /// descriptor takes a family name, thus it gives the family and not a
    /// single face of it.
    pub(super) fn family_font(family: &str, size: f64) -> CTFont {
        let key =
            unsafe { CFString::wrap_under_get_rule(kCTFontFamilyNameAttribute as CFStringRef) };
        let value = CFString::new(family);
        let attributes = CFDictionary::from_CFType_pairs(&[(key, value.as_CFType())]);
        new_from_descriptor(&new_from_attributes(&attributes), size)
    }

    /// A colour in the colour space of the bitmap context, as a `CGColor`.
    /// A colour of another space is converted: a pure blue of the generic
    /// space becomes `(4, 51, 255)` on a P3 display (see
    /// `docs/kb/text-rendering.md`). Thus the drawn colour is the value of
    /// the profile.
    fn device_colour(color: Color) -> CGColor {
        let [r, g, b] = color.0.map(|value| f64::from(value) / 255.0);
        let components: [CGFloat; 4] = [r, g, b, 1.0];
        // SAFETY: the space is a device RGB space that this call owns, the
        // four components are red, green, blue, and alpha, and the colour
        // keeps the space.
        unsafe {
            let space = CGColorSpaceCreateDeviceRGB();
            let colour = CGColor::wrap_under_create_rule(CGColorCreate(space, components.as_ptr()));
            CGColorSpaceRelease(space);
            colour
        }
    }

    /// The system font at `size`, in the weight for emphasis (`bold`) or in
    /// the regular weight.
    fn system_font(size: f64, bold: bool) -> CTFont {
        // The system font. Bold is the weight for emphasis: a label on a
        // small display is easier to read in this weight.
        let kind = if bold {
            kCTFontEmphasizedSystemFontType
        } else {
            kCTFontSystemFontType
        };
        new_ui_font_for_language(kind, size, None)
    }

    /// The font that the label of `font` starts with, at its size, and
    /// whether the label needs the shear of a synthetic italic.
    ///
    /// - A family that is installed gives the font of that family. The bold
    ///   face is the font with the bold trait. A family with no bold face
    ///   keeps its regular face: there is no synthetic bold (decision 0043,
    ///   section 1).
    /// - The italic face is the bold step with the italic trait. A family
    ///   with no italic face is drawn with the shear [`SLANT`], and no false
    ///   italic face.
    /// - A family that is missing gives the system font, and no error.
    fn base_font(font: &Font) -> (CTFont, bool) {
        let size = f64::from(font.size.max(super::MIN_LABEL_SIZE));
        let Some(family) = font.family.as_deref().filter(|name| installed_family(name)) else {
            let system = system_font(size, font.bold);
            return with_italic(system, font.italic);
        };
        let regular = family_font(family, size);
        let bold = if font.bold {
            match regular.clone_with_symbolic_traits(kCTFontBoldTrait, kCTFontBoldTrait) {
                Some(bold) => bold,
                // A family with no bold face shows its regular face.
                None => regular,
            }
        } else {
            regular
        };
        with_italic(bold, font.italic)
    }

    /// The italic face of `font`, or `font` with the shear of a synthetic
    /// italic. The italic trait is asked for on the font of the bold step,
    /// not together with the bold trait: a family with a bold face and no
    /// italic face keeps its bold face (decision 0043, section 1).
    fn with_italic(font: CTFont, italic: bool) -> (CTFont, bool) {
        if !italic {
            return (font, false);
        }
        match font.clone_with_symbolic_traits(kCTFontItalicTrait, kCTFontItalicTrait) {
            Some(italic) => (italic, false),
            None => (font, true),
        }
    }

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
        /// The room that the shear of a synthetic italic needs on the right,
        /// in pixels. `0` for an upright line.
        shear: f64,
    }

    fn layout(
        text: &str,
        base: &CTFont,
        shear: bool,
        font: &Font,
        size: f64,
        width: u32,
        color: Option<Color>,
    ) -> Layout {
        // The base font is the family at its size. A smaller size is a copy
        // of the same font: no new font descriptor for each step.
        let face = base.clone_with_font_size(size);
        // A synthetic italic draws each line with a shear. The shear needs
        // its own room on the right, and the drawn line moves left by half of
        // it (decision 0043, section 1).
        let shear = if shear { SLANT * face.ascent() } else { 0.0 };
        let limit = (f64::from(width) - shear).max(1.0);
        let mut styled = CFMutableAttributedString::new();
        styled.replace_str(&CFString::new(text), CFRange::init(0, 0));
        let range = CFRange::init(0, styled.char_len());
        // The colour of the text and of its underline. Core Text takes the
        // colour of an underline from the foreground attribute, not from the
        // context: with the attribute "from context" the underline is black.
        // Thus the drawing gives the colour itself, and a layout with no
        // colour (the height of a band) uses the context (decision 0043,
        // section 1).
        let text_colour = color.map(device_colour);
        // SAFETY: the names are attribute names of Core Text, and each value
        // has the type that its attribute needs: a font, a CGColor, a
        // Boolean, and the number of the underline style. `1` is a single
        // underline.
        unsafe {
            styled.set_attribute(range, kCTFontAttributeName, &face);
            match &text_colour {
                Some(colour) => {
                    styled.set_attribute(range, kCTForegroundColorAttributeName, colour);
                    if font.underline {
                        styled.set_attribute(range, kCTUnderlineColorAttributeName, colour);
                    }
                }
                None => styled.set_attribute(
                    range,
                    kCTForegroundColorFromContextAttributeName,
                    &CFBoolean::true_value(),
                ),
            }
            if font.underline {
                styled.set_attribute(range, kCTUnderlineStyleAttributeName, &CFNumber::from(1i32));
            }
        }
        let setter = CTFramesetter::new_with_attributed_string(styled.as_concrete_TypeRef());
        let path = CGPath::from_rect(
            CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(limit, TALL)),
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
        let width = out.iter().map(|l| l.width).fold(0.0, f64::max) + shear;
        // An underline draws below the descent. The block is higher by that
        // space, thus the line stays in the area at the bottom (decision
        // 0043, section 1).
        let underline = if font.underline {
            UNDERLINE_PART * size
        } else {
            0.0
        };
        Layout {
            width,
            height: top - bottom + underline,
            shear,
            lines: out,
        }
    }

    fn fits(layout: &Layout, width: u32, height: u32) -> bool {
        layout.width <= f64::from(width) + 0.5 && layout.height <= f64::from(height) + 0.5
    }

    /// The layout of `text` for an area of `width` × `height`: at the first
    /// font size at which the text fits, or at the smallest size with the end
    /// of the text cut.
    fn fit(text: &str, font: &Font, width: u32, height: u32, color: Option<Color>) -> Layout {
        let text = text.trim();
        // The font of the family is made one time for this fit. Each smaller
        // size is a copy of it (decision 0043, section 1).
        let (base, shear) = base_font(font);
        let sizes = font.sizes();
        for &size in &sizes {
            let layout = layout(text, &base, shear, font, size, width, color);
            if fits(&layout, width, height) {
                return layout;
            }
        }
        // Find the largest number of characters that fits with `…` after
        // them. A cut can divide a group of characters that make one sign.
        let smallest = sizes[sizes.len() - 1];
        let ends: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        if ends.is_empty() {
            return layout("", &base, shear, font, smallest, width, color);
        }
        let cut = |n: usize| format!("{}…", text[..ends[n]].trim_end());
        let (mut low, mut high) = (0, ends.len() - 1);
        while low < high {
            let middle = (low + high).div_ceil(2);
            if fits(
                &layout(&cut(middle), &base, shear, font, smallest, width, color),
                width,
                height,
            ) {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        layout(&cut(low), &base, shear, font, smallest, width, color)
    }

    pub fn fit_height(text: &str, font: &Font, width: u32, limit: u32) -> u32 {
        if width == 0 || limit == 0 {
            return limit;
        }
        (fit(text, font, width, limit, None).height.ceil() as u32).min(limit)
    }

    pub fn draw(
        canvas: &mut RgbImage,
        area: Area,
        text: &str,
        font: &Font,
        color: Color,
        align: Align,
    ) {
        let layout = fit(text, font, area.width, area.height, Some(color));
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
        // A synthetic italic: the shear is in the text matrix, and the line
        // moves left by half of the room that the shear needs, thus the
        // slanted text stays centred in its area.
        if layout.shear > 0.0 {
            context.set_text_matrix(&CGAffineTransform::new(1.0, 0.0, SLANT, 1.0, 0.0, 0.0));
        }
        for line in &layout.lines {
            context.set_text_position(
                ((f64::from(area.width) - line.width - layout.shear) / 2.0).round(),
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
    use crate::label::platform;
    use core_text::font_descriptor::kCTFontBoldTrait;
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
        draw(canvas, area, text, &Font::default(), color, Align::Middle);
    }

    fn drawn_with(text: &str, font: &Font, align: Align) -> RgbImage {
        let mut image = canvas(112, 112);
        let area = full(&image);
        draw(&mut image, area, text, font, Color::WHITE, align);
        image
    }

    #[test]
    fn the_sizes_go_down_in_steps_of_two_to_the_smallest() {
        let sizes = |size| {
            Font {
                size,
                ..Font::default()
            }
            .sizes()
        };
        assert_eq!(sizes(20), [20.0, 18.0, 16.0, 14.0, 12.0, 10.0]);
        assert_eq!(sizes(15), [15.0, 13.0, 11.0, 10.0]);
        assert_eq!(sizes(10), [10.0]);
        assert_eq!(sizes(4), [10.0]);
        // Above 40 the steps are 8 pixels down to 40, then 2 pixels.
        assert_eq!(
            sizes(40),
            (10..=40)
                .rev()
                .step_by(2)
                .map(f64::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            sizes(80),
            [
                80.0, 72.0, 64.0, 56.0, 48.0, 40.0, 38.0, 36.0, 34.0, 32.0, 30.0, 28.0, 26.0, 24.0,
                22.0, 20.0, 18.0, 16.0, 14.0, 12.0, 10.0
            ]
        );
        assert_eq!(sizes(50)[..4], [50.0, 48.0, 40.0, 38.0]);
        assert_eq!(sizes(96)[0], 80.0, "the maximum is 80");
    }

    /// The ink of `text` at `size`, as left, top, right, bottom.
    #[cfg(target_os = "macos")]
    fn emoji_ink(text: &str, size: u8) -> (u32, u32, u32, u32) {
        let font = Font {
            size,
            ..Font::default()
        };
        ink(&drawn_with(text, &font, Align::Middle))
            .unwrap_or_else(|| panic!("no ink for {text} at {size}"))
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn an_emoji_fills_the_face_at_a_large_size_and_does_not_touch_an_edge() {
        // 60 % of a 112-pixel key face.
        let least = 68;
        for text in ["😀", "🔇", "👨‍💻"] {
            for size in [72, 80] {
                let (left, top, right, bottom) = emoji_ink(text, size);
                let (width, height) = (right - left + 1, bottom - top + 1);
                // The two-person ZWJ glyph has wider side bearings: its ink
                // is 64 pixels wide at 72, and 71 at 80. The height and the
                // 80-pixel width are above 60 %.
                let least_width = if text == "👨‍💻" && size == 72 {
                    60
                } else {
                    least
                };
                assert!(
                    width >= least_width && height >= least,
                    "{text} at {size} is {width} × {height}"
                );
                assert!(
                    left > 0 && top > 0 && right < 111 && bottom < 111,
                    "{text} at {size} touches an edge: {left} {top} {right} {bottom}"
                );
                let (x, y) = ((left + right) / 2, (top + bottom) / 2);
                assert!(
                    x.abs_diff(56) <= 4 && y.abs_diff(56) <= 4,
                    "{text} at {size} is not centred: {x} {y}"
                );
            }
        }
    }

    #[test]
    fn the_alignment_puts_the_text_at_the_top_the_middle_or_the_bottom() {
        let font = Font::default();
        let (_, top, _, _) = ink(&drawn_with("Mail", &font, Align::Top)).unwrap();
        let (_, middle_top, _, middle_bottom) =
            ink(&drawn_with("Mail", &font, Align::Middle)).unwrap();
        let (_, _, _, bottom) = ink(&drawn_with("Mail", &font, Align::Bottom)).unwrap();
        assert!(top < 8, "{top}");
        assert!(bottom > 104, "{bottom}");
        assert!(
            middle_top > 35 && middle_bottom < 77,
            "{middle_top} {middle_bottom}"
        );
    }

    #[test]
    fn the_size_and_the_weight_change_the_text() {
        let font = |size, bold| Font {
            size,
            bold,
            ..Font::default()
        };
        let width = |font: &Font| {
            let (left, _, right, _) = ink(&drawn_with("Mail", font, Align::Middle)).unwrap();
            right - left
        };
        let pixels = |font: &Font| {
            drawn_with("Mail", font, Align::Middle)
                .pixels()
                .filter(|p| **p != DARK)
                .count()
        };
        let regular = font(20, false);
        assert!(width(&font(32, false)) > width(&regular) + 10);
        assert!(width(&font(12, false)) + 10 < width(&regular));
        assert!(pixels(&font(20, true)) > pixels(&regular) * 11 / 10);
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

    /// A font with the family `family` and the default weight.
    fn family_font(family: &str, bold: bool) -> Font {
        Font {
            family: Some(family.into()),
            bold,
            ..Font::default()
        }
    }

    /// The width of the ink of `font` on "Mail".
    fn ink_width(font: &Font) -> u32 {
        let (left, _, right, _) = ink(&drawn_with("Mail", font, Align::Middle)).unwrap();
        right - left
    }

    /// The number of pixels of the ink of `font` on "Mail".
    fn ink_pixels(font: &Font) -> usize {
        drawn_with("Mail", font, Align::Middle)
            .pixels()
            .filter(|pixel| **pixel != DARK)
            .count()
    }

    #[test]
    fn the_font_list_has_no_hidden_name_no_duplicate_and_no_dots() {
        let families = font_families();
        assert!(!families.is_empty(), "the system has font families");
        for family in &families {
            assert!(!family.display.is_empty(), "{family:?}");
            assert!(!family.display.starts_with('.'), "{family:?}");
            assert!(!family.name.is_empty(), "{family:?}");
        }
        let mut names: Vec<&str> = families.iter().map(|f| f.name.as_str()).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "a stored name is in the list two times");
        // The order is the display name, with no difference between capital
        // and small letters.
        let displays: Vec<String> = families.iter().map(|f| f.display.to_lowercase()).collect();
        let mut sorted = displays.clone();
        sorted.sort();
        assert_eq!(displays, sorted);
        // The families that the tests need are installed.
        assert!(
            families.iter().any(|family| family.name == "Menlo"),
            "Menlo is installed"
        );
    }

    #[test]
    fn an_installed_font_is_not_the_system_font() {
        let system = Font::default();
        let menlo = family_font("Menlo", true);
        assert!(ink(&drawn_with("Mail", &menlo, Align::Middle)).is_some());
        assert_ne!(
            drawn_with("Mail", &menlo, Align::Middle).as_raw(),
            drawn_with("Mail", &system, Align::Middle).as_raw()
        );
        // The two fonts have different widths at the same size.
        assert_ne!(ink_width(&menlo), ink_width(&system));
    }

    #[test]
    fn a_font_that_is_not_installed_draws_in_the_system_font() {
        let system = Font::default();
        for missing in [
            "No Such Font Family 12345",
            "Menlo-Regular",
            "menlo",
            ".AppleSystemUIFont",
        ] {
            let font = family_font(missing, true);
            if missing == ".AppleSystemUIFont" {
                // A hidden family is not in the list, thus it is missing.
                assert!(
                    !font_families().iter().any(|family| family.name == missing),
                    "{missing} is in the list"
                );
            }
            assert_eq!(
                drawn_with("Mail", &font, Align::Middle).as_raw(),
                drawn_with("Mail", &system, Align::Middle).as_raw(),
                "{missing}"
            );
        }
    }

    #[test]
    fn a_font_family_uses_its_bold_face() {
        let regular = family_font("Menlo", false);
        let bold = family_font("Menlo", true);
        assert!(
            ink_pixels(&bold) > ink_pixels(&regular) * 11 / 10,
            "{} {}",
            ink_pixels(&bold),
            ink_pixels(&regular)
        );
    }

    #[test]
    fn a_family_with_no_bold_face_shows_its_regular_face() {
        // A family with no bold face keeps its regular face: no synthetic
        // bold (decision 0043, section 1). Not every system has one, thus the
        // test looks for one and stops with no family to test.
        let without = font_families().into_iter().find(|family| {
            platform::family_font(&family.name, 20.0)
                .clone_with_symbolic_traits(kCTFontBoldTrait, kCTFontBoldTrait)
                .is_none()
        });
        let Some(family) = without else {
            return;
        };
        let regular = family_font(&family.name, false);
        let bold = family_font(&family.name, true);
        assert_eq!(
            drawn_with("Mail", &regular, Align::Middle).as_raw(),
            drawn_with("Mail", &bold, Align::Middle).as_raw(),
            "{}",
            family.name
        );
    }

    /// A font of the family `family` (`None` is the system font) at 20
    /// pixels, with the three styles.
    fn with_style(family: Option<&str>, bold: bool, italic: bool, underline: bool) -> Font {
        Font {
            size: 20,
            bold,
            family: family.map(Into::into),
            italic,
            underline,
        }
    }

    #[test]
    fn italic_and_bold_italic_change_the_face() {
        let bold = with_style(Some("Menlo"), true, false, false);
        let italic = with_style(Some("Menlo"), true, true, false);
        let bold_italic = with_style(Some("Menlo"), false, true, false);
        let upright_regular = with_style(Some("Menlo"), false, false, false);
        let image = |font: &Font| drawn_with("Mail", font, Align::Middle).as_raw().clone();
        assert_ne!(image(&bold), image(&italic), "bold and bold italic");
        assert_ne!(image(&upright_regular), image(&bold_italic), "italic");
        assert_ne!(image(&italic), image(&bold_italic), "bold italic");
        assert_ne!(image(&bold), image(&upright_regular), "bold");
    }

    #[test]
    fn a_family_with_no_italic_face_is_drawn_with_a_shear() {
        // Impact has a bold face and no italic face (measured, see
        // `docs/kb/text-rendering.md`). The label is then slanted, and it
        // stays in its area and at the centre.
        let upright = with_style(Some("Impact"), true, false, false);
        let slanted = with_style(Some("Impact"), true, true, false);
        let image = |font: &Font| drawn_with("Mail", font, Align::Middle).as_raw().clone();
        assert_ne!(image(&upright), image(&slanted));
        let (left, _, right, _) = ink(&drawn_with("Mail", &slanted, Align::Middle)).unwrap();
        assert!(left >= 1 && right <= 110, "{left} {right}");
        assert!(
            (left as i32 - (111 - right) as i32).abs() <= 4,
            "the slanted label is at the centre: {left} {right}"
        );
        // A long label at the smallest size keeps the room of the shear.
        let text = "Volume up and down";
        let (left, _, right, _) = ink(&drawn_with(text, &slanted, Align::Middle)).unwrap();
        assert!(left >= 1 && right <= 110, "{left} {right}");
    }

    #[test]
    fn an_underline_is_drawn_below_the_text_and_stays_in_the_area() {
        let plain = with_style(None, true, false, false);
        let underlined = with_style(None, true, false, true);
        // At the top the text starts at the same place with and without the
        // line, thus the line is below the ink of the text.
        let upright = drawn_with("Mail", &plain, Align::Top);
        let line = drawn_with("Mail", &underlined, Align::Top);
        let (_, _, _, plain_bottom) = ink(&upright).unwrap();
        let (_, _, _, bottom) = ink(&line).unwrap();
        assert!(
            bottom > plain_bottom,
            "the underline is below the text: {bottom} {plain_bottom}"
        );
        // The underline has the colour of the text.
        let mut red = canvas(112, 112);
        let area = full(&red);
        draw(
            &mut red,
            area,
            "Mail",
            &underlined,
            Color::rgb(255, 0, 0),
            Align::Top,
        );
        assert!(red
            .enumerate_pixels()
            .any(|(_, y, p)| y > plain_bottom && p[0] > 240 && p[1] < 40 && p[2] < 60));
        // At the bottom of the area the underline stays inside it.
        let mut image = canvas(112, 112);
        let area = full(&image);
        draw(
            &mut image,
            area,
            "Mail",
            &underlined,
            Color::WHITE,
            Align::Bottom,
        );
        let (_, _, _, bottom) = ink(&image).unwrap();
        assert!(bottom <= 110, "{bottom}");
        // The band of an underlined label is higher.
        assert!(band_height("Mail", &underlined, 104, 112) > band_height("Mail", &plain, 104, 112));
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
        let one = band_height("Mail", &font, 104, 112);
        let two = band_height("Volume up and down", &font, 104, 112);
        let many = band_height(&"word ".repeat(50), &font, 104, 112);
        assert!((16..=30).contains(&one), "{one}");
        assert!(two > one, "{one} {two}");
        assert!(two <= 44 && many <= 44, "{two} {many}");
        // A smaller font needs a lower band.
        let small = band_height(
            "Mail",
            &Font {
                size: 10,
                bold: true,
                ..Font::default()
            },
            104,
            112,
        );
        assert!(small < one, "{small} {one}");
    }
}
