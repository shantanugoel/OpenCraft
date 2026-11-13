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
//! - The label has the position, the size, the colour, and the weight of its
//!   [`LabelStyle`]. The default position is the middle with no image, and
//!   the bottom with an image.
//! - With a label and no image, the label is at the top, at the centre, or
//!   at the bottom of the surface.
//! - With a label at the top or at the bottom and an image, the label is in
//!   a band at that edge, and the image has the rest of the surface. A label
//!   at the middle is on the image, at the centre.
//! - A label with the position "none" does not show.
//!
//! Feedback of a handler changes this layout: see
//! [`surface_image_with_feedback`].

use crate::action::{Feedback, FeedbackState};
use crate::label::{self, Align, Area, Font};
use crate::profile::{Color, LabelPosition, LabelStyle, SurfaceContent};
use image::imageops::{self, FilterType};
use image::{DynamicImage, Rgb, RgbImage};

/// The colour of a surface that has no colour.
pub const DEFAULT_COLOR: Color = Color::BLACK;

/// The space between the edge of the surface and the label, in pixels.
const MARGIN: u32 = 4;

/// The width of the frame for the state "on", in pixels. On a key face of
/// 112 × 112, a frame of this width stays clear below the key cap, also
/// where the cap hides a part of one edge from the eye (see
/// `docs/kb/n4-pro-hardware.md`).
pub const FRAME: u32 = 6;

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
    let style = content.label_style;
    let position = style.position_for(image.is_some());
    let text = content
        .label
        .as_deref()
        .filter(|t| !t.trim().is_empty() && position != LabelPosition::Hidden);
    let font = Font {
        size: style.size,
        bold: style.bold,
    };
    let text_color = label_color(content);
    let width = size.width.saturating_sub(2 * MARGIN);
    // The label on all of the surface, inside the margin.
    let full = Area {
        x: MARGIN,
        y: MARGIN,
        width,
        height: size.height.saturating_sub(2 * MARGIN),
    };

    // The area of the image, from its top: all of the surface, or the part
    // that the band of the label does not use.
    let (mut image_top, mut image_height) = (0, size.height);
    match (text, image, position) {
        (Some(text), Some(_), LabelPosition::Top | LabelPosition::Bottom) => {
            let band = label::band_height(text, font, width, size.height);
            image_height = size.height - band;
            let y = if position == LabelPosition::Top {
                image_top = band;
                MARGIN / 2
            } else {
                image_height
            };
            let area = Area {
                x: MARGIN,
                y,
                width,
                height: band.saturating_sub(MARGIN / 2),
            };
            label::draw(&mut canvas, area, text, font, text_color, Align::Middle);
        }
        (Some(text), None, _) => {
            label::draw(&mut canvas, full, text, font, text_color, align(position));
        }
        _ => {}
    }

    if let Some(image) = image {
        draw_image(&mut canvas, image, image_top, size.width, image_height);
        // A label at the middle is on the image.
        if let (Some(text), LabelPosition::Middle) = (text, position) {
            label::draw(&mut canvas, full, text, font, text_color, Align::Middle);
        }
    }
    canvas
}

/// The vertical place of a label in its area for `position`.
fn align(position: LabelPosition) -> Align {
    match position {
        LabelPosition::Top => Align::Top,
        LabelPosition::Bottom => Align::Bottom,
        LabelPosition::Middle | LabelPosition::Hidden => Align::Middle,
    }
}

/// The colour of the label of `content`: the colour of its [`LabelStyle`],
/// or else black or white for the fill (see [`text_color`]). A shell can use
/// it to show the colour of the label.
pub fn label_color(content: &SurfaceContent) -> Color {
    let LabelStyle { color, .. } = content.label_style;
    color.unwrap_or_else(|| text_color(content.color.unwrap_or(DEFAULT_COLOR)))
}

/// The feedback of the state that a feedback with a state does not show:
/// [`Feedback::other_state`], or else the same parts with the other state.
/// The image of the other state comes from it (decision 0017).
pub fn other_state(feedback: &Feedback) -> Feedback {
    let flipped = match feedback.state {
        Some(FeedbackState::On) => Some(FeedbackState::Off),
        Some(FeedbackState::Off) => Some(FeedbackState::On),
        None => None,
    };
    match &feedback.other_state {
        Some(other) => Feedback {
            state: other.state.or(flipped),
            other_state: None,
            ..(**other).clone()
        },
        None => Feedback {
            state: flipped,
            other_state: None,
            ..feedback.clone()
        },
    }
}

/// Make the surface image with the newest feedback of its bindings.
///
/// - A label of the feedback replaces the label of the content.
/// - A level is a bar at the bottom, with a dark track and a white fill. Its
///   area is separate from the label and the image.
/// - The state "on" is a frame around the surface, in black or white for the
///   fill (see [`text_color`]), also when the label has a colour of its
///   own. The rest of the image, with the level bar,
///   is inside the frame, so the frame covers no part of it. The state
///   "off" draws no frame.
pub fn surface_image_with_feedback(
    size: Size,
    content: &SurfaceContent,
    image: Option<&DynamicImage>,
    feedback: Option<&Feedback>,
) -> RgbImage {
    let Some(feedback) = feedback else {
        return surface_image(size, content, image);
    };
    let mut content = content.clone();
    if let Some(label) = &feedback.label {
        content.label = Some(label.clone());
    }
    if feedback.state != Some(FeedbackState::On) {
        return with_level(size, &content, image, feedback.level);
    }
    let color = content.color.unwrap_or(DEFAULT_COLOR);
    let frame = FRAME.min(size.width / 4).min(size.height / 4);
    let inner = Size {
        width: size.width - 2 * frame,
        height: size.height - 2 * frame,
    };
    let mut canvas = RgbImage::from_pixel(size.width, size.height, Rgb(color.0));
    let inside = with_level(inner, &content, image, feedback.level);
    imageops::replace(&mut canvas, &inside, i64::from(frame), i64::from(frame));
    draw_frame(&mut canvas, frame, text_color(color));
    canvas
}

/// Make the surface image with a level bar at the bottom, if `level` is
/// `Some` and the surface is large enough for the bar.
fn with_level(
    size: Size,
    content: &SurfaceContent,
    image: Option<&DynamicImage>,
    level: Option<u8>,
) -> RgbImage {
    let Some(level) = level.filter(|_| size.width >= 16 && size.height >= 20) else {
        return surface_image(size, content, image);
    };
    let area = Size {
        width: size.width,
        height: size.height - 16,
    };
    let top = surface_image(area, content, image);
    let mut canvas = RgbImage::from_pixel(
        size.width,
        size.height,
        Rgb(content.color.unwrap_or(DEFAULT_COLOR).0),
    );
    imageops::replace(&mut canvas, &top, 0, 0);
    let width = size.width - 12;
    let fill = width * u32::from(level.min(100)) / 100;
    for y in size.height - 12..size.height - 4 {
        for x in 4..size.width - 4 {
            let border =
                x < 6 || x >= size.width - 6 || y == size.height - 12 || y == size.height - 5;
            let color = if border {
                [0, 0, 0]
            } else if x - 6 < fill {
                [255, 255, 255]
            } else {
                [64, 64, 64]
            };
            canvas.put_pixel(x, y, Rgb(color));
        }
    }
    canvas
}

/// Draw a frame of `width` pixels in `color` along the four edges of
/// `canvas`.
fn draw_frame(canvas: &mut RgbImage, width: u32, color: Color) {
    let (w, h) = canvas.dimensions();
    for (x, y, pixel) in canvas.enumerate_pixels_mut() {
        if x < width || y < width || x + width >= w || y + width >= h {
            *pixel = Rgb(color.0);
        }
    }
}

/// Draw `image` on `canvas`, as large as possible in the area from 0, `top`
/// with the size `width` × `height`, with its proportion, at the centre.
fn draw_image(canvas: &mut RgbImage, image: &DynamicImage, top: u32, width: u32, height: u32) {
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
    let y = top + (height - scaled.height().min(height)) / 2;
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
            ..SurfaceContent::default()
        }
    }

    /// `content` with the label style `style`.
    #[cfg(target_os = "macos")]
    fn styled(label: &str, color: Option<Color>, style: LabelStyle) -> SurfaceContent {
        SurfaceContent {
            label_style: style,
            ..content(Some(label), color)
        }
    }

    #[cfg(target_os = "macos")]
    fn at(position: LabelPosition) -> LabelStyle {
        LabelStyle {
            position: Some(position),
            ..LabelStyle::default()
        }
    }

    /// The first and the last row with a pixel of the colour `want`.
    #[cfg(target_os = "macos")]
    fn rows_of(image: &RgbImage, want: Rgb<u8>) -> Option<(u32, u32)> {
        let rows: Vec<u32> = image
            .enumerate_pixels()
            .filter(|(_, _, p)| **p == want)
            .map(|(_, y, _)| y)
            .collect();
        Some((*rows.iter().min()?, *rows.iter().max()?))
    }

    #[cfg(target_os = "macos")]
    const WHITE: Rgb<u8> = Rgb([255, 255, 255]);

    #[cfg(target_os = "macos")]
    #[test]
    fn each_position_puts_the_label_there_on_a_surface_with_no_image() {
        for size in [KEY, BAR] {
            let rows = |position| {
                let image = surface_image(size, &styled("Mail", Some(RED), at(position)), None);
                rows_of(&image, WHITE)
            };
            let (top, _) = rows(LabelPosition::Top).expect("the label shows at the top");
            let (middle_top, middle_bottom) = rows(LabelPosition::Middle).unwrap();
            let (_, bottom) = rows(LabelPosition::Bottom).unwrap();
            let h = size.height;
            assert!(top < 14, "{size:?} top {top}");
            assert!(bottom > h - 14, "{size:?} bottom {bottom}");
            assert!(
                middle_top > h / 3 && middle_bottom < h * 2 / 3,
                "{size:?} middle {middle_top} {middle_bottom}"
            );
            assert_eq!(rows(LabelPosition::Hidden), None, "{size:?} none");
            // The automatic position is the middle.
            assert_eq!(
                surface_image(size, &content(Some("Mail"), Some(RED)), None),
                surface_image(
                    size,
                    &styled("Mail", Some(RED), at(LabelPosition::Middle)),
                    None
                )
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn each_position_puts_the_label_there_on_a_surface_with_an_image() {
        for size in [KEY, BAR] {
            let draw = |position| {
                surface_image(
                    size,
                    &styled("Mail", Some(RED), at(position)),
                    Some(&green(size.width, size.height)),
                )
            };
            let h = size.height;
            // Top: a band at the top, and the image below it.
            let image = draw(LabelPosition::Top);
            let (label_top, label_bottom) = rows_of(&image, WHITE).unwrap();
            let (green_top, green_bottom) = rows_of(&image, GREEN).unwrap();
            assert!(
                label_bottom < green_top,
                "{size:?} {label_bottom} {green_top}"
            );
            assert!(label_top < 10 && green_bottom == h - 1, "{size:?}");
            // Bottom: the image above, and a band at the bottom.
            let image = draw(LabelPosition::Bottom);
            let (label_top, _) = rows_of(&image, WHITE).unwrap();
            let (green_top, green_bottom) = rows_of(&image, GREEN).unwrap();
            assert!(green_bottom < label_top && green_top == 0, "{size:?}");
            // The automatic position is the bottom.
            assert_eq!(
                image,
                surface_image(
                    size,
                    &content(Some("Mail"), Some(RED)),
                    Some(&green(size.width, size.height))
                )
            );
            // Middle: the image fills the surface, and the label is on it.
            let image = draw(LabelPosition::Middle);
            let (label_top, label_bottom) = rows_of(&image, WHITE).unwrap();
            assert_eq!(rows_of(&image, GREEN), Some((0, h - 1)), "{size:?}");
            assert!(label_top > h / 3 && label_bottom < h * 2 / 3, "{size:?}");
            // None: only the image.
            let image = draw(LabelPosition::Hidden);
            assert_eq!(rows_of(&image, WHITE), None);
            assert_eq!(
                image,
                surface_image(
                    size,
                    &content(None, Some(RED)),
                    Some(&green(size.width, size.height))
                )
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_label_has_its_colour_size_and_weight() {
        let blue = Color::rgb(0, 0, 255);
        let image = surface_image(
            KEY,
            &styled(
                "Mail",
                Some(Color::WHITE),
                LabelStyle {
                    color: Some(blue),
                    ..LabelStyle::default()
                },
            ),
            None,
        );
        assert!(
            image.pixels().any(|p| *p == Rgb(blue.0)),
            "the label is blue"
        );
        assert_eq!(
            label_color(&styled("x", Some(Color::WHITE), at(LabelPosition::Top))),
            Color::BLACK
        );
        assert_eq!(
            label_color(&styled(
                "x",
                None,
                LabelStyle {
                    color: Some(blue),
                    ..LabelStyle::default()
                }
            )),
            blue
        );

        let ink = |style: LabelStyle| {
            let image = surface_image(KEY, &styled("Mail", None, style), None);
            let columns: Vec<u32> = image
                .enumerate_pixels()
                .filter(|(_, _, p)| p.0 != [0, 0, 0])
                .map(|(x, _, _)| x)
                .collect();
            let width = columns.iter().max().unwrap() - columns.iter().min().unwrap();
            (width, columns.len())
        };
        let size = |size| LabelStyle {
            size,
            ..LabelStyle::default()
        };
        let (small, _) = ink(size(12));
        let (normal, bold_pixels) = ink(size(20));
        let (large, _) = ink(size(32));
        assert!(small < normal && normal < large, "{small} {normal} {large}");
        let (_, regular_pixels) = ink(LabelStyle {
            bold: false,
            ..LabelStyle::default()
        });
        assert!(
            bold_pixels > regular_pixels,
            "{bold_pixels} {regular_pixels}"
        );
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
    fn feedback_keeps_the_label_above_a_clear_level_track() {
        for size in [KEY, BAR] {
            let content = content(Some("Original"), Some(RED));
            let feedback = Feedback {
                label: Some("Feedback".into()),
                level: Some(50),
                state: None,
                other_state: None,
            };
            let image = surface_image_with_feedback(size, &content, None, Some(&feedback));
            assert_eq!(image.dimensions(), (size.width, size.height));
            assert_eq!(image.get_pixel(8, size.height - 8), &Rgb([255, 255, 255]));
            assert_eq!(
                image.get_pixel(size.width - 8, size.height - 8),
                &Rgb([64, 64, 64])
            );
            let top = imageops::crop_imm(&image, 0, 0, size.width, size.height - 16).to_image();
            let expected = surface_image(
                Size {
                    width: size.width,
                    height: size.height - 16,
                },
                &SurfaceContent {
                    label: Some("Feedback".into()),
                    ..content.clone()
                },
                None,
            );
            assert_eq!(top, expected);
            for level in [0, 100, 255] {
                let feedback = Feedback {
                    label: None,
                    level: Some(level),
                    state: None,
                    other_state: None,
                };
                let image = surface_image_with_feedback(size, &content, None, Some(&feedback));
                let want = if level == 0 {
                    [64, 64, 64]
                } else {
                    [255, 255, 255]
                };
                assert_eq!(image.get_pixel(size.width - 8, size.height - 8), &Rgb(want));
            }
            let feedback = Feedback {
                label: Some("Replacement".into()),
                level: None,
                state: Some(FeedbackState::Off),
                other_state: None,
            };
            assert_eq!(
                surface_image_with_feedback(size, &content, None, Some(&feedback)),
                surface_image(
                    size,
                    &SurfaceContent {
                        label: Some("Replacement".into()),
                        ..content
                    },
                    None
                )
            );
        }
    }

    /// True if each pixel of the frame of `width` has the colour `want`.
    fn has_frame(image: &RgbImage, width: u32, want: Color) -> bool {
        let (w, h) = image.dimensions();
        image
            .enumerate_pixels()
            .filter(|(x, y, _)| *x < width || *y < width || x + width >= w || y + width >= h)
            .all(|(_, _, p)| *p == Rgb(want.0))
    }

    fn on(label: Option<&str>, level: Option<u8>) -> Feedback {
        Feedback {
            label: label.map(str::to_string),
            level,
            state: Some(FeedbackState::On),
            other_state: None,
        }
    }

    #[test]
    fn the_state_on_draws_a_frame_in_the_colour_of_the_label() {
        for size in [KEY, BAR] {
            // White on a dark colour, and black on a light colour.
            for (background, frame) in [
                (None, Color::WHITE),
                (Some(RED), Color::WHITE),
                (Some(Color::rgb(255, 210, 30)), Color::BLACK),
            ] {
                let content = content(None, background);
                let image =
                    surface_image_with_feedback(size, &content, None, Some(&on(None, None)));
                assert_eq!(image.dimensions(), (size.width, size.height));
                assert!(has_frame(&image, FRAME, frame), "{size:?} {background:?}");
                // Inside the frame, the surface has its colour.
                let inside = background.unwrap_or(DEFAULT_COLOR);
                assert_eq!(image.get_pixel(FRAME, FRAME), &Rgb(inside.0));
                assert_eq!(
                    image.get_pixel(size.width / 2, size.height / 2),
                    &Rgb(inside.0)
                );
            }
        }
    }

    #[test]
    fn the_state_off_draws_no_frame() {
        let content = content(Some("Mail"), Some(RED));
        let off = Feedback {
            state: Some(FeedbackState::Off),
            ..Feedback::default()
        };
        assert_eq!(
            surface_image_with_feedback(KEY, &content, None, Some(&off)),
            surface_image(KEY, &content, None)
        );
    }

    #[test]
    fn a_frame_keeps_the_image_the_label_and_the_level_bar_inside_it() {
        for size in [KEY, BAR] {
            let content = content(Some("Original"), Some(RED));
            let feedback = on(Some("On"), Some(50));
            let image =
                surface_image_with_feedback(size, &content, Some(&green(64, 64)), Some(&feedback));
            assert!(has_frame(&image, FRAME, Color::WHITE));

            // The inside is the image with the level bar, at the size of the
            // inside.
            let inner = Size {
                width: size.width - 2 * FRAME,
                height: size.height - 2 * FRAME,
            };
            let level = Feedback {
                state: None,
                ..feedback.clone()
            };
            let want =
                surface_image_with_feedback(inner, &content, Some(&green(64, 64)), Some(&level));
            let inside =
                imageops::crop_imm(&image, FRAME, FRAME, inner.width, inner.height).to_image();
            assert_eq!(inside, want);
            // The fill and the track of the bar are inside the frame.
            let bar_y = FRAME + inner.height - 8;
            assert_eq!(image.get_pixel(FRAME + 8, bar_y), &Rgb([255, 255, 255]));
            assert_eq!(
                image.get_pixel(FRAME + inner.width - 8, bar_y),
                &Rgb([64, 64, 64])
            );
        }
    }

    #[test]
    fn a_frame_on_a_small_surface_does_not_fail() {
        for (width, height) in [(0, 0), (1, 1), (8, 8), (30, 12)] {
            let size = Size { width, height };
            let image = surface_image_with_feedback(
                size,
                &content(Some("Mail"), None),
                None,
                Some(&on(Some("On"), Some(100))),
            );
            assert_eq!(image.dimensions(), (width, height));
        }
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
