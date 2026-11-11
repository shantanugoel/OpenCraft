//! Make the image of a surface from its content.
//!
//! The engine makes the image of a surface one time. The device and the shell
//! show that same image (architecture rule 11). The image is upright and has
//! the size of the surface. The device layer turns and encodes it for the
//! device.
//!
//! The layout of a surface:
//!
//! - The colour fills the surface. With no colour, the surface is black.
//! - The image is as large as possible in its area, with its proportion, at
//!   the centre. The colour shows through the transparent parts.
//! - With a label and no image, the label is at the centre of the surface.
//! - With a label and an image, the label is in a band at the bottom, and
//!   the image has the area above the band.

use crate::label::{self, Area};
use crate::profile::{Color, SurfaceContent};
use image::imageops::{self, FilterType};
use image::{DynamicImage, Rgb, RgbImage};

/// The colour of a surface that has no colour.
pub const DEFAULT_COLOR: Color = Color::BLACK;

/// The space between the edge of the surface and the label, in pixels.
const MARGIN: u32 = 4;

/// The size of a surface image, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// Make the upright image of a surface of the size `size` for `content`.
///
/// `image` is the decoded image of the content, if the content has one. The
/// caller loads it: see [`crate::store::ProfileStore::load_image`].
pub fn surface_image(
    size: Size,
    content: &SurfaceContent,
    image: Option<&DynamicImage>,
) -> RgbImage {
    let color = content.color.unwrap_or(DEFAULT_COLOR);
    let mut canvas = RgbImage::from_pixel(size.width, size.height, Rgb(color.0));
    if size.width == 0 || size.height == 0 {
        return canvas;
    }
    let text = content.label.as_deref().filter(|t| !t.trim().is_empty());

    // The band of the label, when the surface also has an image.
    let mut image_height = size.height;
    if let (Some(text), Some(_)) = (text, image) {
        let band = label::band_height(text, size.width.saturating_sub(2 * MARGIN), size.height);
        image_height = size.height - band;
        let area = Area {
            x: MARGIN,
            y: image_height,
            width: size.width.saturating_sub(2 * MARGIN),
            height: band.saturating_sub(MARGIN / 2),
        };
        label::draw(&mut canvas, area, text, text_color(color));
    } else if let Some(text) = text {
        let area = Area {
            x: MARGIN,
            y: MARGIN,
            width: size.width.saturating_sub(2 * MARGIN),
            height: size.height.saturating_sub(2 * MARGIN),
        };
        label::draw(&mut canvas, area, text, text_color(color));
    }

    if let Some(image) = image {
        draw_image(&mut canvas, image, size.width, image_height);
    }
    canvas
}

/// Draw `image` on `canvas`, as large as possible in the area from 0, 0 with
/// the size `width` × `height`, with its proportion, at the centre.
fn draw_image(canvas: &mut RgbImage, image: &DynamicImage, width: u32, height: u32) {
    if width == 0 || height == 0 || image.width() == 0 || image.height() == 0 {
        return;
    }
    let scaled = if (image.width(), image.height()) == (width, height) {
        image.to_rgba8()
    } else {
        image
            .resize(width, height, FilterType::CatmullRom)
            .into_rgba8()
    };
    let x = (width - scaled.width().min(width)) / 2;
    let y = (height - scaled.height().min(height)) / 2;
    // Put the pixels on the colour with their transparency.
    let mut under = DynamicImage::ImageRgb8(std::mem::take(canvas)).into_rgba8();
    imageops::overlay(&mut under, &scaled, i64::from(x), i64::from(y));
    *canvas = DynamicImage::ImageRgba8(under).into_rgb8();
}

/// The colour of the label on a surface with the colour `background`: white
/// on a dark colour, and black on a light colour.
pub fn text_color(background: Color) -> Color {
    let [r, g, b] = background.0.map(|v| {
        let v = f32::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    // The relative luminance of the colour. At 0.179, black text and white
    // text have the same contrast with the colour.
    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if luminance > 0.179 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    const KEY: Size = Size {
        width: 112,
        height: 112,
    };
    const BAR: Size = Size {
        width: 176,
        height: 112,
    };
    const RED: Color = Color::rgb(200, 30, 30);
    const GREEN: Rgb<u8> = Rgb([0, 200, 0]);

    fn content(label: Option<&str>, color: Option<Color>) -> SurfaceContent {
        SurfaceContent {
            label: label.map(str::to_string),
            color,
            image: None,
        }
    }

    fn green(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(width, height, Rgba([0, 200, 0, 255])))
    }

    /// True if a pixel in the rows from `top` to `bottom` has the colour
    /// `want`.
    #[cfg(target_os = "macos")]
    fn has_color(image: &RgbImage, top: u32, bottom: u32, want: Rgb<u8>) -> bool {
        image
            .enumerate_pixels()
            .any(|(_, y, p)| (top..bottom).contains(&y) && *p == want)
    }

    #[test]
    fn a_surface_with_no_content_is_black_and_has_the_size_of_the_surface() {
        for size in [KEY, BAR] {
            let image = surface_image(size, &SurfaceContent::default(), None);
            assert_eq!(image.dimensions(), (size.width, size.height));
            assert!(image.pixels().all(|p| *p == Rgb([0, 0, 0])));
        }
    }

    #[test]
    fn the_colour_fills_the_surface() {
        let image = surface_image(BAR, &content(None, Some(RED)), None);
        assert!(image.pixels().all(|p| *p == Rgb(RED.0)));
    }

    #[test]
    fn an_image_is_as_large_as_possible_with_its_proportion_at_the_centre() {
        // A wide image on a key face: the full width, and the colour above
        // and below.
        let image = surface_image(KEY, &content(None, Some(RED)), Some(&green(200, 100)));
        assert_eq!(image.get_pixel(0, 56), &GREEN);
        assert_eq!(image.get_pixel(111, 56), &GREEN);
        assert_eq!(image.get_pixel(56, 30), &GREEN);
        assert_eq!(image.get_pixel(56, 81), &GREEN);
        assert_eq!(image.get_pixel(56, 26), &Rgb(RED.0));
        assert_eq!(image.get_pixel(56, 85), &Rgb(RED.0));

        // A small square image on a bar panel: the full height, and the
        // colour on the left and on the right.
        let image = surface_image(BAR, &content(None, Some(RED)), Some(&green(16, 16)));
        assert_eq!(image.get_pixel(88, 0), &GREEN);
        assert_eq!(image.get_pixel(88, 111), &GREEN);
        assert_eq!(image.get_pixel(33, 56), &GREEN);
        assert_eq!(image.get_pixel(30, 56), &Rgb(RED.0));
        assert_eq!(image.get_pixel(145, 56), &Rgb(RED.0));
    }

    #[test]
    fn the_colour_shows_through_the_transparent_parts_of_an_image() {
        let source = RgbaImage::from_fn(112, 112, |x, _| {
            if x < 56 {
                Rgba([0, 200, 0, 255])
            } else {
                Rgba([0, 200, 0, 0])
            }
        });
        let image = surface_image(
            KEY,
            &content(None, Some(RED)),
            Some(&DynamicImage::ImageRgba8(source)),
        );
        assert_eq!(image.get_pixel(10, 56), &GREEN);
        assert_eq!(image.get_pixel(100, 56), &Rgb(RED.0));
    }

    #[test]
    fn the_label_has_a_colour_that_shows_on_the_colour_of_the_surface() {
        assert_eq!(text_color(Color::BLACK), Color::WHITE);
        assert_eq!(text_color(RED), Color::WHITE);
        assert_eq!(text_color(Color::rgb(20, 40, 220)), Color::WHITE);
        assert_eq!(text_color(Color::WHITE), Color::BLACK);
        assert_eq!(text_color(Color::rgb(255, 210, 30)), Color::BLACK);
        assert_eq!(text_color(Color::rgb(0, 200, 0)), Color::BLACK);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_label_with_no_image_is_at_the_centre() {
        let image = surface_image(KEY, &content(Some("Mail"), Some(RED)), None);
        let white = Rgb([255, 255, 255]);
        assert!(
            has_color(&image, 40, 72, white),
            "the label is in the middle"
        );
        assert!(!has_color(&image, 0, 30, white) && !has_color(&image, 82, 112, white));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_label_with_an_image_is_in_a_band_below_the_image() {
        let image = surface_image(
            KEY,
            &content(Some("Mail"), Some(RED)),
            Some(&green(112, 112)),
        );
        let white = Rgb([255, 255, 255]);
        // The image is above, and it is smaller than the surface. The label
        // is in the band at the bottom, on the colour.
        assert!(has_color(&image, 0, 70, GREEN));
        assert!(!has_color(&image, 0, 70, white));
        assert!(has_color(&image, 80, 112, white));
        assert!(!has_color(&image, 86, 112, GREEN));
        assert_eq!(image.get_pixel(1, 110), &Rgb(RED.0));
    }

    #[test]
    fn a_label_of_spaces_is_no_label() {
        let with = surface_image(KEY, &content(Some("  "), Some(RED)), Some(&green(112, 112)));
        let without = surface_image(KEY, &content(None, Some(RED)), Some(&green(112, 112)));
        assert_eq!(with, without);
    }

    #[test]
    fn a_surface_with_no_area_does_not_fail() {
        let size = Size {
            width: 0,
            height: 0,
        };
        let image = surface_image(size, &content(Some("Mail"), Some(RED)), Some(&green(8, 8)));
        assert_eq!(image.dimensions(), (0, 0));
    }
}
