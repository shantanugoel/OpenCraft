//! Upload the upright engine image without encoding or reading a file.

use gpui_kit::RenderImage;
use opencraft_engine::RgbImage;
use std::sync::Arc;

/// GPUI's image frames use BGRA bytes, despite the buffer type name.
/// An icon keeps its alpha channel: see [`from_rgba`].
pub fn from_rgb(rgb: &RgbImage) -> Arc<RenderImage> {
    let (width, height) = rgb.dimensions();
    let bgra = image::RgbaImage::from_fn(width, height, |x, y| {
        let p = rgb.get_pixel(x, y);
        image::Rgba([p[2], p[1], p[0], 255])
    });
    Arc::new(RenderImage::new([image::Frame::new(bgra)]))
}

/// A bitmap with an alpha channel, as BGRA bytes for GPUI. Application
/// icons use this. The bytes of `rgba` are red, green, blue, alpha.
pub fn from_rgba(rgba: &image::RgbaImage) -> Arc<RenderImage> {
    let (width, height) = rgba.dimensions();
    let bgra = image::RgbaImage::from_fn(width, height, |x, y| {
        let p = rgba.get_pixel(x, y);
        image::Rgba([p[2], p[1], p[0], p[3]])
    });
    Arc::new(RenderImage::new([image::Frame::new(bgra)]))
}

#[cfg(test)]
mod tests {
    #[test]
    fn upload_preserves_size() {
        let image = super::from_rgb(&opencraft_engine::RgbImage::new(3, 2));
        assert_eq!(
            image.size(0),
            gpui_kit::size(gpui_kit::DevicePixels(3), gpui_kit::DevicePixels(2))
        );
    }
}
