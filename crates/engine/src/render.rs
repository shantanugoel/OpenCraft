//! Make the image of a surface from its content.
//!
//! The engine makes the image of a surface one time. The device and the shell
//! show that same image (architecture rule 11). The image is upright and has
//! the size of the surface. The device layer turns and encodes it for the
//! device.
//!
//! The layout of a surface:
//!
//! - The colour fills the surface. With no colour, the surface shows the
//!   part of the page background at its position, or black on a page with
//!   no background (decision 0032).
//! - The picture is the image or the icon of the content (a surface has one
//!   or none of them).
//! - The image is as large as possible in its area, with its proportion, at
//!   the centre. The colour shows through the transparent parts.
//! - The icon is a square at the centre of its area, in its colour (see
//!   [`icon_color`]). Its side is a multiple of 24 pixels, the grid of the
//!   icons, so that each line of the grid is on the edge of a pixel: see
//!   [`icon_side`] and decision 0033.
//! - The label has the position, the size, the colour, and the weight of its
//!   [`LabelStyle`]. The default position is the middle with no picture, and
//!   the bottom with a picture.
//! - With a label and no picture, the label is at the top, at the centre, or
//!   at the bottom of the surface.
//! - With a label at the top or at the bottom and a picture, the label is in
//!   a band at that edge, and the picture has the rest of the surface. A
//!   label at the middle is on the picture, at the centre.
//! - A label with the position "none" does not show.
//!
//! Feedback of a handler changes this layout: see
//! [`surface_image_with_feedback`].

use crate::action::{Feedback, FeedbackState};
use crate::icons;
use crate::label::{self, Align, Area, Font};
use crate::profile::{Background, BackgroundFit, Color, LabelPosition, LabelStyle, SurfaceContent};
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
    surface_image_on(size, content, image, &Base::of(content, None))
}

/// What is behind the image and the label of a surface: the fill colour,
/// or the part of the page background at the position of the surface.
#[derive(Debug, Clone)]
enum Base {
    Color(Color),
    /// The part of the background, with the size of the area.
    Backdrop(RgbImage),
}

impl Base {
    /// The base of `content`: its colour, or else `backdrop`, or else
    /// [`DEFAULT_COLOR`].
    fn of(content: &SurfaceContent, backdrop: Option<&RgbImage>) -> Self {
        match (content.color, backdrop) {
            (Some(color), _) => Base::Color(color),
            (None, Some(backdrop)) => Base::Backdrop(backdrop.clone()),
            (None, None) => Base::Color(DEFAULT_COLOR),
        }
    }

    /// An image of `size` with the base. A backdrop of a different size
    /// goes at the top left, on black.
    fn canvas(&self, size: Size) -> RgbImage {
        match self {
            Base::Color(color) => RgbImage::from_pixel(size.width, size.height, Rgb(color.0)),
            Base::Backdrop(backdrop) => {
                if backdrop.dimensions() == (size.width, size.height) {
                    return backdrop.clone();
                }
                let mut canvas =
                    RgbImage::from_pixel(size.width, size.height, Rgb(DEFAULT_COLOR.0));
                imageops::replace(&mut canvas, backdrop, 0, 0);
                canvas
            }
        }
    }

    /// The base of the part of the area at `x`, `y` with the size `size`.
    fn part(&self, x: u32, y: u32, size: Size) -> Self {
        match self {
            Base::Color(color) => Base::Color(*color),
            Base::Backdrop(backdrop) => Base::Backdrop(
                imageops::crop_imm(backdrop, x, y, size.width, size.height).to_image(),
            ),
        }
    }

    /// The colour that a label must show on: the fill colour, or the mean
    /// colour of the backdrop.
    fn tone(&self) -> Color {
        match self {
            Base::Color(color) => *color,
            Base::Backdrop(backdrop) => mean_color(backdrop),
        }
    }
}

/// The mean colour of the pixels of `image`. An image with no pixels gives
/// [`DEFAULT_COLOR`].
fn mean_color(image: &RgbImage) -> Color {
    let count = u64::from(image.width()) * u64::from(image.height());
    if count == 0 {
        return DEFAULT_COLOR;
    }
    let mut sum = [0u64; 3];
    for pixel in image.pixels() {
        for (total, value) in sum.iter_mut().zip(pixel.0) {
            *total += u64::from(value);
        }
    }
    Color(sum.map(|total| (total / count) as u8))
}

/// Make the upright image of a surface on `base`. See [`surface_image`].
fn surface_image_on(
    size: Size,
    content: &SurfaceContent,
    image: Option<&DynamicImage>,
    base: &Base,
) -> RgbImage {
    let mut canvas = base.canvas(size);
    if size.width == 0 || size.height == 0 {
        return canvas;
    }
    let style = content.label_style;
    let icon = content.known_icon().filter(|_| image.is_none());
    let has_picture = image.is_some() || icon.is_some();
    let position = style.position_for(has_picture);
    let text = content
        .label
        .as_deref()
        .filter(|t| !t.trim().is_empty() && position != LabelPosition::Hidden);
    let font = Font {
        size: style.size,
        bold: style.bold,
    };
    let text_color = content
        .label_style
        .color
        .unwrap_or_else(|| text_color(base.tone()));
    let width = size.width.saturating_sub(2 * MARGIN);
    // The label on all of the surface, inside the margin.
    let full = Area {
        x: MARGIN,
        y: MARGIN,
        width,
        height: size.height.saturating_sub(2 * MARGIN),
    };

    // The area of the picture, from its top: all of the surface, or the
    // part that the band of the label does not use.
    let (mut image_top, mut image_height) = (0, size.height);
    match (text, has_picture, position) {
        (Some(text), true, LabelPosition::Top | LabelPosition::Bottom) => {
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
        (Some(text), false, _) => {
            label::draw(&mut canvas, full, text, font, text_color, align(position));
        }
        _ => {}
    }

    if let Some(image) = image {
        draw_image(&mut canvas, image, image_top, size.width, image_height);
    } else if let Some(icon) = icon {
        let side = icon_side(size.width, image_height);
        let x = (size.width - side) / 2;
        let y = image_top + (image_height - side) / 2;
        // The automatic colour is black or white for what is behind the
        // icon, as for the label: the fill, or the page background.
        let colour = content
            .icon
            .as_ref()
            .and_then(|i| i.color)
            .unwrap_or_else(|| self::text_color(base.tone()));
        icons::draw(&mut canvas, icon, colour, x, y, side);
    }
    // A label at the middle is on the picture.
    if let (Some(text), true, LabelPosition::Middle) = (text, has_picture, position) {
        label::draw(&mut canvas, full, text, font, text_color, Align::Middle);
    }
    canvas
}

/// The side of an icon in an area of `width` × `height` pixels: the largest
/// multiple of 24 that is not more than three quarters of the shorter side.
/// Thus the icon has a margin, and it keeps the same size on a key face of
/// 112 and inside the frame of the state "on". On a small area, the side
/// is three quarters of the shorter side.
pub fn icon_side(width: u32, height: u32) -> u32 {
    let most = width.min(height) * 3 / 4;
    match most / 24 * 24 {
        0 => most,
        side => side,
    }
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

/// The colour of the icon of `content`: the colour of its [`SurfaceIcon`],
/// or else black or white for the fill (see [`text_color`]). A shell can
/// use it to show the colour of the icon. On a surface with no fill and a
/// page background, the image uses the colour for the background.
///
/// [`SurfaceIcon`]: crate::profile::SurfaceIcon
pub fn icon_color(content: &SurfaceContent) -> Color {
    content
        .icon
        .as_ref()
        .and_then(|icon| icon.color)
        .unwrap_or_else(|| text_color(content.color.unwrap_or(DEFAULT_COLOR)))
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
    surface_image_on_background(size, content, image, feedback, None)
}

/// Make the surface image as [`surface_image_with_feedback`] does, on a page
/// with a background. `backdrop` is the part of the background image at
/// the position of the surface, with the size of the surface: see
/// [`backdrop`]. A surface with no fill colour shows it behind its image
/// and its label, and the automatic colour of the label is black or white
/// for the mean colour of the backdrop. A surface with a fill colour does
/// not show it (decision 0032).
pub fn surface_image_on_background(
    size: Size,
    content: &SurfaceContent,
    image: Option<&DynamicImage>,
    feedback: Option<&Feedback>,
    backdrop: Option<&RgbImage>,
) -> RgbImage {
    let base = Base::of(content, backdrop);
    let Some(feedback) = feedback else {
        return surface_image_on(size, content, image, &base);
    };
    let mut content = content.clone();
    if let Some(label) = &feedback.label {
        content.label = Some(label.clone());
    }
    if feedback.state != Some(FeedbackState::On) {
        return with_level(size, &content, image, feedback.level, &base);
    }
    let frame = FRAME.min(size.width / 4).min(size.height / 4);
    let inner = Size {
        width: size.width - 2 * frame,
        height: size.height - 2 * frame,
    };
    let mut canvas = base.canvas(size);
    let inside = with_level(
        inner,
        &content,
        image,
        feedback.level,
        &base.part(frame, frame, inner),
    );
    imageops::replace(&mut canvas, &inside, i64::from(frame), i64::from(frame));
    draw_frame(&mut canvas, frame, text_color(base.tone()));
    canvas
}

/// Make the upright image of the page background `background` of the size
/// `size`: one colour, or the image with its fit, at the centre. `image` is
/// the decoded background image, if the background is an image. If it is
/// `None` for an image, the display is black. The caller loads it: see
/// [`crate::store::ProfileStore::load_image`].
pub fn background_image(
    size: Size,
    background: &Background,
    image: Option<&DynamicImage>,
) -> RgbImage {
    let (width, height) = (size.width, size.height);
    let fit = match background {
        Background::Color(color) => return RgbImage::from_pixel(width, height, Rgb(color.0)),
        Background::Image { fit, .. } => *fit,
    };
    let mut canvas = RgbImage::from_pixel(width, height, Rgb(DEFAULT_COLOR.0));
    let Some(image) = image.filter(|i| i.width() > 0 && i.height() > 0) else {
        return canvas;
    };
    if width == 0 || height == 0 {
        return canvas;
    }
    let scaled = if (image.width(), image.height()) == (width, height) {
        image.to_rgba8()
    } else {
        match fit {
            BackgroundFit::Fill => image.resize_to_fill(width, height, FilterType::CatmullRom),
            BackgroundFit::Fit => image.resize(width, height, FilterType::CatmullRom),
        }
        .into_rgba8()
    };
    let x = (width - scaled.width().min(width)) / 2;
    let y = (height - scaled.height().min(height)) / 2;
    let mut under = DynamicImage::ImageRgb8(canvas).into_rgba8();
    imageops::overlay(&mut under, &scaled, i64::from(x), i64::from(y));
    canvas = DynamicImage::ImageRgba8(under).into_rgb8();
    canvas
}

/// The part of the background image `background` at `x`, `y` with the size
/// `size`: the backdrop of a surface at that position. A part out of the
/// background is black.
pub fn backdrop(background: &RgbImage, x: u32, y: u32, size: Size) -> RgbImage {
    let mut part = RgbImage::from_pixel(size.width, size.height, Rgb(DEFAULT_COLOR.0));
    let crop = imageops::crop_imm(background, x, y, size.width, size.height).to_image();
    imageops::replace(&mut part, &crop, 0, 0);
    part
}

/// Make the surface image with a level bar at the bottom, if `level` is
/// `Some` and the surface is large enough for the bar.
fn with_level(
    size: Size,
    content: &SurfaceContent,
    image: Option<&DynamicImage>,
    level: Option<u8>,
    base: &Base,
) -> RgbImage {
    let Some(level) = level.filter(|_| size.width >= 16 && size.height >= 20) else {
        return surface_image_on(size, content, image, base);
    };
    let area = Size {
        width: size.width,
        height: size.height - 16,
    };
    let top = surface_image_on(area, content, image, &base.part(0, 0, area));
    let mut canvas = base.canvas(size);
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

    /// `content` with the icon `name` in `color`.
    fn with_icon(
        label: Option<&str>,
        fill: Option<Color>,
        name: &str,
        color: Option<Color>,
    ) -> SurfaceContent {
        SurfaceContent {
            icon: Some(crate::profile::SurfaceIcon {
                name: name.into(),
                color,
            }),
            ..content(label, fill)
        }
    }

    /// The first and the last row and column with a pixel of the colour
    /// `want`.
    fn box_of(image: &RgbImage, want: Rgb<u8>) -> Option<(u32, u32, u32, u32)> {
        let points: Vec<(u32, u32)> = image
            .enumerate_pixels()
            .filter(|(_, _, p)| **p == want)
            .map(|(x, y, _)| (x, y))
            .collect();
        Some((
            points.iter().map(|p| p.0).min()?,
            points.iter().map(|p| p.1).min()?,
            points.iter().map(|p| p.0).max()?,
            points.iter().map(|p| p.1).max()?,
        ))
    }

    const AMBER: Color = Color::rgb(245, 158, 11);

    #[test]
    fn an_icon_is_a_square_of_a_multiple_of_24_at_the_centre_of_a_key_face_and_a_bar_panel() {
        // The icon "square" has its stroke from 2 to 22 of 24: the box of
        // the ink is the square of the icon less 1/12 at each side.
        for size in [KEY, BAR] {
            let image = surface_image(
                size,
                &with_icon(None, Some(RED), "square", Some(AMBER)),
                None,
            );
            assert_eq!(image.dimensions(), (size.width, size.height));
            let side = icon_side(size.width, size.height);
            assert_eq!(side, 72, "{size:?}");
            let (left, top, right, bottom) = box_of(&image, Rgb(AMBER.0)).expect("the icon");
            let x = (size.width - side) / 2;
            let y = (size.height - side) / 2;
            assert_eq!((left, top), (x + 6, y + 6), "{size:?}");
            assert_eq!((right, bottom), (x + side - 7, y + side - 7), "{size:?}");
            // The fill is around and inside the icon.
            assert_eq!(image.get_pixel(1, 1), &Rgb(RED.0));
            assert_eq!(
                image.get_pixel(size.width / 2, size.height / 2),
                &Rgb(RED.0)
            );
        }
    }

    #[test]
    fn the_side_of_an_icon_is_a_multiple_of_24_up_to_three_quarters_of_the_area() {
        assert_eq!(icon_side(112, 112), 72);
        assert_eq!(icon_side(176, 112), 72);
        assert_eq!(icon_side(100, 100), 72);
        assert_eq!(icon_side(112, 84), 48);
        assert_eq!(icon_side(72, 72), 48);
        assert_eq!(icon_side(30, 30), 22);
        assert_eq!(icon_side(0, 50), 0);
    }

    #[test]
    fn an_icon_with_no_colour_is_black_or_white_for_the_fill() {
        let image = surface_image(KEY, &with_icon(None, None, "square", None), None);
        assert!(image.pixels().any(|p| *p == Rgb([255, 255, 255])));
        let image = surface_image(
            KEY,
            &with_icon(None, Some(Color::WHITE), "square", None),
            None,
        );
        assert!(image.pixels().any(|p| *p == Rgb([0, 0, 0])));
        assert_eq!(
            icon_color(&with_icon(None, Some(Color::WHITE), "x", None)),
            Color::BLACK
        );
        assert_eq!(icon_color(&with_icon(None, None, "x", Some(AMBER))), AMBER);
    }

    #[test]
    fn an_icon_that_the_set_does_not_have_draws_nothing() {
        assert_eq!(
            surface_image(KEY, &with_icon(None, Some(RED), "no-such-icon", None), None),
            surface_image(KEY, &content(None, Some(RED)), None)
        );
    }

    #[test]
    fn an_image_hides_the_icon_of_the_same_content() {
        let both = with_icon(None, Some(RED), "square", Some(AMBER));
        assert_eq!(
            surface_image(KEY, &both, Some(&green(112, 112))),
            surface_image(KEY, &content(None, Some(RED)), Some(&green(112, 112)))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn each_label_position_puts_the_label_there_on_a_surface_with_an_icon() {
        for size in [KEY, BAR] {
            let draw = |position| {
                let mut content = with_icon(Some("Mail"), Some(RED), "square", Some(AMBER));
                content.label_style = at(position);
                surface_image(size, &content, None)
            };
            let h = size.height;
            // Top: a band at the top, and the icon below it.
            let image = draw(LabelPosition::Top);
            let (_, label_bottom) = rows_of(&image, WHITE).unwrap();
            let (_, icon_top, _, _) = box_of(&image, Rgb(AMBER.0)).unwrap();
            assert!(
                label_bottom < icon_top,
                "{size:?} {label_bottom} {icon_top}"
            );
            // Bottom: the icon above, and a band at the bottom.
            let image = draw(LabelPosition::Bottom);
            let (label_top, _) = rows_of(&image, WHITE).unwrap();
            let (_, _, _, icon_bottom) = box_of(&image, Rgb(AMBER.0)).unwrap();
            assert!(icon_bottom < label_top, "{size:?}");
            // The automatic position is the bottom.
            assert_eq!(
                image,
                surface_image(
                    size,
                    &with_icon(Some("Mail"), Some(RED), "square", Some(AMBER)),
                    None
                )
            );
            // Middle: the icon has all of the surface, and the label is on it.
            let image = draw(LabelPosition::Middle);
            let (label_top, label_bottom) = rows_of(&image, WHITE).unwrap();
            assert!(label_top > h / 3 && label_bottom < h * 2 / 3, "{size:?}");
            let (_, icon_top, _, _) = box_of(&image, Rgb(AMBER.0)).unwrap();
            assert_eq!(icon_top, (h - 72) / 2 + 6, "{size:?}");
            // None: only the icon, as with no label.
            assert_eq!(
                draw(LabelPosition::Hidden),
                surface_image(
                    size,
                    &with_icon(None, Some(RED), "square", Some(AMBER)),
                    None
                )
            );
        }
    }

    #[test]
    fn the_state_on_keeps_the_size_of_the_icon_inside_the_frame() {
        let content = with_icon(None, Some(RED), "square", Some(AMBER));
        let off = surface_image(KEY, &content, None);
        let on = surface_image_with_feedback(KEY, &content, None, Some(&on(None, None)));
        let size = |image: &RgbImage| {
            let (left, top, right, bottom) = box_of(image, Rgb(AMBER.0)).unwrap();
            (right - left, bottom - top)
        };
        assert_eq!(size(&off), size(&on));
        assert!(has_frame(&on, FRAME, Color::WHITE));
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

    const DISPLAY: Size = Size {
        width: 800,
        height: 480,
    };

    fn image_background(fit: BackgroundFit) -> Background {
        Background::Image {
            image: "images/0123456789abcdef.png".into(),
            fit,
        }
    }

    #[test]
    fn a_colour_background_fills_the_display() {
        let blue = Color::rgb(10, 20, 200);
        let image = background_image(DISPLAY, &Background::Color(blue), None);
        assert_eq!(image.dimensions(), (800, 480));
        assert!(image.pixels().all(|p| *p == Rgb(blue.0)));
        // An image background with no image is black.
        let image = background_image(DISPLAY, &image_background(BackgroundFit::Fill), None);
        assert!(image.pixels().all(|p| *p == Rgb([0, 0, 0])));
    }

    #[test]
    fn fill_covers_the_display_and_fit_shows_all_of_the_image() {
        // A square image: "Fill" covers all of the display, "Fit" has black
        // at the left and at the right.
        let square = green(300, 300);
        let fill = background_image(
            DISPLAY,
            &image_background(BackgroundFit::Fill),
            Some(&square),
        );
        assert!(fill.pixels().all(|p| *p == GREEN));
        let fit = background_image(
            DISPLAY,
            &image_background(BackgroundFit::Fit),
            Some(&square),
        );
        assert_eq!(fit.get_pixel(400, 0), &GREEN);
        assert_eq!(fit.get_pixel(400, 479), &GREEN);
        assert_eq!(fit.get_pixel(160, 240), &GREEN);
        assert_eq!(fit.get_pixel(639, 240), &GREEN);
        assert_eq!(fit.get_pixel(150, 240), &Rgb([0, 0, 0]));
        assert_eq!(fit.get_pixel(650, 240), &Rgb([0, 0, 0]));

        // "Fill" keeps the centre: an image with a red left half and a green
        // right half, wider than the display, loses the same part at each
        // side.
        let wide = DynamicImage::ImageRgb8(RgbImage::from_fn(1600, 480, |x, _| {
            if x < 800 {
                Rgb(RED.0)
            } else {
                GREEN
            }
        }));
        let fill = background_image(DISPLAY, &image_background(BackgroundFit::Fill), Some(&wide));
        assert_eq!(fill.get_pixel(398, 240), &Rgb(RED.0));
        assert_eq!(fill.get_pixel(402, 240), &GREEN);
    }

    #[test]
    fn a_surface_with_no_fill_shows_the_background_behind_it() {
        // A background with a red left half and a green right half.
        let background =
            RgbImage::from_fn(800, 480, |x, _| if x < 400 { Rgb(RED.0) } else { GREEN });
        let part = backdrop(&background, 346, 8, KEY);
        assert_eq!(part.dimensions(), (112, 112));
        assert_eq!(part.get_pixel(53, 0), &Rgb(RED.0));
        assert_eq!(part.get_pixel(54, 0), &GREEN);
        // A part out of the background is black.
        let edge = backdrop(&background, 750, 400, KEY);
        assert_eq!(edge.get_pixel(60, 90), &Rgb([0, 0, 0]));
        assert_eq!(edge.get_pixel(10, 10), &GREEN);

        let empty = SurfaceContent::default();
        let image = surface_image_on_background(KEY, &empty, None, None, Some(&part));
        assert_eq!(
            image, part,
            "an empty surface is the part of the background"
        );
        // A fill colour covers the background.
        let red = content(None, Some(Color::rgb(1, 2, 3)));
        let image = surface_image_on_background(KEY, &red, None, None, Some(&part));
        assert!(image.pixels().all(|p| *p == Rgb([1, 2, 3])));
        // An image goes on the background, which shows through its
        // transparent parts.
        let clear = DynamicImage::ImageRgba8(RgbaImage::from_pixel(112, 112, Rgba([0, 0, 0, 0])));
        let image = surface_image_on_background(KEY, &empty, Some(&clear), None, Some(&part));
        assert_eq!(image, part);
        // The feedback level and the frame of the state "on" keep the
        // background inside.
        let feedback = Feedback {
            state: Some(FeedbackState::On),
            level: Some(50),
            ..Feedback::default()
        };
        let image = surface_image_on_background(KEY, &empty, None, Some(&feedback), Some(&part));
        assert_eq!(image.get_pixel(FRAME + 2, FRAME + 2), &Rgb(RED.0));
        assert_eq!(image.get_pixel(111 - FRAME - 2, FRAME + 2), &GREEN);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_automatic_label_colour_shows_on_the_background() {
        // On a light background, the automatic label is black.
        let light = RgbImage::from_pixel(112, 112, Rgb([240, 240, 240]));
        let image = surface_image_on_background(
            KEY,
            &content(Some("Mail"), None),
            None,
            None,
            Some(&light),
        );
        assert!(image.pixels().any(|p| *p == Rgb([0, 0, 0])));
        assert!(!image.pixels().any(|p| *p == WHITE));
        let dark = RgbImage::from_pixel(112, 112, Rgb([20, 20, 30]));
        let image =
            surface_image_on_background(KEY, &content(Some("Mail"), None), None, None, Some(&dark));
        assert!(image.pixels().any(|p| *p == WHITE));
    }
}
