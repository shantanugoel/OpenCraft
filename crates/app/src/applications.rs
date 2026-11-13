//! The installed applications on this Mac (issue 336).
//!
//! The shell owns this list (rule 12). The scan reads `Info.plist` in the
//! usual application folders. It does not load icons. A caller runs
//! [`scan`] off the user interface thread. [`application_icon`] draws one
//! icon when a row needs it, and the window caches the bitmap.
//!
//! A picker gives an [`InstalledApplication`]: the name, the bundle
//! identifier, and the path. The action "Open an application" stores the
//! path. That text is the form the action already accepts. A later profile
//! link can store the bundle identifier.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One installed application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledApplication {
    /// The name that the list shows.
    pub name: String,
    /// The bundle identifier. Empty when the bundle has none.
    pub bundle_identifier: String,
    /// The path of the application bundle.
    pub path: PathBuf,
}

impl InstalledApplication {
    /// The file name without `.app`.
    pub fn file_stem(&self) -> String {
        file_stem(&self.path)
    }

    /// True when `query` is empty or is part of the name, the bundle
    /// identifier, or the file name. The match ignores the case of letters.
    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.trim();
        if query.is_empty() {
            return true;
        }
        let query = query.to_lowercase();
        self.name.to_lowercase().contains(&query)
            || self.bundle_identifier.to_lowercase().contains(&query)
            || self.file_stem().to_lowercase().contains(&query)
    }

    /// True when `stored` is this application. A value with `/` or a
    /// leading `~/` is a path. Any other value is a name, a bundle
    /// identifier, or the file name. The action stores both forms.
    pub fn matches_stored(&self, stored: &str) -> bool {
        let stored = stored.trim().trim_end_matches('/');
        if stored.is_empty() {
            return false;
        }
        if let Some(path) = stored_as_path(stored) {
            return path == self.path;
        }
        let stored = strip_app_suffix(stored).to_lowercase();
        self.name.to_lowercase() == stored
            || (!self.bundle_identifier.is_empty()
                && self.bundle_identifier.to_lowercase() == stored)
            || self.file_stem().to_lowercase() == stored
    }
}

/// The applications in `directories`, one level deep. A directory that
/// does not exist adds nothing. The result is sorted by name.
pub fn scan(directories: &[PathBuf]) -> Vec<InstalledApplication> {
    let mut applications = Vec::new();
    for directory in directories {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if !is_bundle_name(&name) {
                continue;
            }
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            applications.push(application_at(path));
        }
    }
    applications.sort_by(|a, b| a.path.cmp(&b.path));
    applications.dedup_by(|a, b| a.path == b.path);
    applications.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    applications
}

/// The folders that macOS uses for applications: `/Applications`,
/// `/System/Applications`, the `Utilities` folder of each, and
/// `Applications` in the home folder when `HOME` is set.
pub fn standard_directories() -> Vec<PathBuf> {
    let mut directories = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/Applications/Utilities"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home) = std::env::var_os("HOME").filter(|home| !home.is_empty()) {
        directories.push(PathBuf::from(home).join("Applications"));
    }
    directories
}

/// The applications in `applications` whose name, bundle identifier, or
/// file name contains `query`.
pub fn filter(applications: &[InstalledApplication], query: &str) -> Vec<InstalledApplication> {
    applications
        .iter()
        .filter(|application| application.matches_query(query))
        .cloned()
        .collect()
}

/// The icon of the application at `path`, or `None` when this system
/// cannot draw one. Call this on the user interface thread: AppKit draws
/// the icon there. The scan does not call this.
pub fn application_icon(path: &Path) -> Option<image::RgbaImage> {
    #[cfg(target_os = "macos")]
    {
        macos::icon(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        None
    }
}

fn application_at(path: PathBuf) -> InstalledApplication {
    let (display_name, bundle_name, bundle_identifier) = read_bundle(&path);
    let name = non_empty(display_name)
        .or_else(|| non_empty(bundle_name))
        .unwrap_or_else(|| file_stem(&path));
    InstalledApplication {
        name,
        bundle_identifier: non_empty(bundle_identifier).unwrap_or_default(),
        path,
    }
}

fn read_bundle(bundle: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let values = plist_strings(&bundle.join("Contents/Info.plist"));
    (
        values.get("CFBundleDisplayName").cloned(),
        values.get("CFBundleName").cloned(),
        values.get("CFBundleIdentifier").cloned(),
    )
}

fn plist_strings(path: &Path) -> HashMap<String, String> {
    let Ok(bytes) = std::fs::read(path) else {
        return HashMap::new();
    };
    if bytes.starts_with(b"bplist") {
        #[cfg(target_os = "macos")]
        {
            return macos::binary_plist(path);
        }
        #[cfg(not(target_os = "macos"))]
        {
            return HashMap::new();
        }
    }
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF])
        .unwrap_or(bytes.as_slice());
    let Ok(text) = std::str::from_utf8(bytes) else {
        return HashMap::new();
    };
    xml_plist_strings(text)
}

/// The string values of the keys that the scan uses. A value that is not
/// a string is skipped. The first string for a key wins.
fn xml_plist_strings(xml: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<key>") {
        rest = &rest[start + "<key>".len()..];
        let Some(end) = rest.find("</key>") else {
            break;
        };
        let key = rest[..end].trim();
        rest = &rest[end + "</key>".len()..];
        let Some((value, consumed)) = next_string(rest) else {
            continue;
        };
        if matches!(
            key,
            "CFBundleDisplayName" | "CFBundleName" | "CFBundleIdentifier"
        ) {
            values.entry(key.to_string()).or_insert(value);
        }
        rest = &rest[consumed..];
    }
    values
}

/// The next `<string>` element, and how many bytes of `text` it used,
/// including the space before it. `None` when the next element is not a
/// string.
fn next_string(text: &str) -> Option<(String, usize)> {
    let skipped = text.len() - text.trim_start().len();
    let text = text.trim_start();
    let rest = text.strip_prefix("<string")?;
    if rest.starts_with("/>") {
        return Some((String::new(), skipped + "<string/>".len()));
    }
    if let Some(rest) = rest.strip_prefix(" />") {
        let _ = rest;
        return Some((String::new(), skipped + "<string />".len()));
    }
    let end_of_tag = rest.find('>')?;
    if rest[..end_of_tag].ends_with('/') {
        return Some((String::new(), skipped + "<string".len() + end_of_tag + 1));
    }
    let content = &rest[end_of_tag + 1..];
    let end = content.find("</string>")?;
    let consumed = skipped + "<string".len() + end_of_tag + 1 + end + "</string>".len();
    Some((decode_xml(&content[..end]), consumed))
}

fn decode_xml(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at + 1..];
        let Some(end) = rest.find(';') else {
            out.push('&');
            continue;
        };
        let entity = &rest[..end];
        rest = &rest[end + 1..];
        if let Some(character) = xml_entity(entity) {
            out.push(character);
        } else {
            out.push('&');
            out.push_str(entity);
            out.push(';');
        }
    }
    out.push_str(rest);
    out
}

fn xml_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            let code = entity.strip_prefix('#')?;
            let code = if let Some(hex) = code.strip_prefix('x').or_else(|| code.strip_prefix('X'))
            {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                code.parse().ok()?
            };
            char::from_u32(code)
        }
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    })
}

fn is_bundle_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".app")
}

fn strip_app_suffix(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    match lower.strip_suffix(".app") {
        Some(rest) => &name[..rest.len()],
        None => name,
    }
}

fn file_stem(path: &Path) -> String {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return "Application".into();
    };
    let stem = strip_app_suffix(name);
    if stem.is_empty() {
        "Application".into()
    } else {
        stem.to_string()
    }
}

fn stored_as_path(stored: &str) -> Option<PathBuf> {
    if let Some(rest) = stored.strip_prefix("~/") {
        let home = std::env::var_os("HOME")?;
        return Some(PathBuf::from(home).join(rest));
    }
    if stored.contains('/') {
        Some(PathBuf::from(stored))
    } else {
        None
    }
}

/// The channel order of one pixel in a focused icon bitmap.
///
/// macOS builds every variant. A Linux test builds only `RgbaF16`, so the
/// other variants are allowed to stay unused there.
#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy)]
#[allow(dead_code)]
enum IconLayout {
    /// 8-bit blue, green, red, alpha.
    Bgra8,
    /// 8-bit red, green, blue, alpha.
    Rgba8,
    /// 8-bit alpha, red, green, blue.
    Argb8,
    /// 8-bit alpha, blue, green, red.
    Abgr8,
    /// 16-bit float red, green, blue, alpha.
    RgbaF16,
    /// 16-bit float alpha, red, green, blue.
    ArgbF16,
}

/// Copy `data` into an 8-bit image. A 16-bit float sample is a little-endian
/// half. A premultiplied colour is divided by its alpha.
#[cfg(any(target_os = "macos", test))]
fn decode_icon_bitmap(
    width: usize,
    height: usize,
    row_bytes: usize,
    data: &[u8],
    layout: IconLayout,
    premultiplied: bool,
) -> Option<image::RgbaImage> {
    let bytes_per_pixel = match layout {
        IconLayout::Bgra8 | IconLayout::Rgba8 | IconLayout::Argb8 | IconLayout::Abgr8 => 4,
        IconLayout::RgbaF16 | IconLayout::ArgbF16 => 8,
    };
    if width == 0
        || height == 0
        || row_bytes < width * bytes_per_pixel
        || data.len() < height * row_bytes
    {
        return None;
    }
    let mut image = image::RgbaImage::new(width as u32, height as u32);
    for y in 0..height {
        let row = &data[y * row_bytes..y * row_bytes + width * bytes_per_pixel];
        for x in 0..width {
            let pixel = &row[x * bytes_per_pixel..(x + 1) * bytes_per_pixel];
            let (red, green, blue, alpha) = match layout {
                IconLayout::Bgra8 => {
                    u8_channels(pixel[2], pixel[1], pixel[0], pixel[3], premultiplied)
                }
                IconLayout::Rgba8 => {
                    u8_channels(pixel[0], pixel[1], pixel[2], pixel[3], premultiplied)
                }
                IconLayout::Argb8 => {
                    u8_channels(pixel[1], pixel[2], pixel[3], pixel[0], premultiplied)
                }
                IconLayout::Abgr8 => {
                    u8_channels(pixel[3], pixel[2], pixel[1], pixel[0], premultiplied)
                }
                IconLayout::RgbaF16 => f16_channels(pixel, 0, premultiplied),
                IconLayout::ArgbF16 => f16_channels(pixel, 1, premultiplied),
            };
            image.put_pixel(x as u32, y as u32, image::Rgba([red, green, blue, alpha]));
        }
    }
    Some(image)
}

#[cfg(any(target_os = "macos", test))]
fn u8_channels(
    mut red: u8,
    mut green: u8,
    mut blue: u8,
    alpha: u8,
    premultiplied: bool,
) -> (u8, u8, u8, u8) {
    if premultiplied {
        red = unpremultiply_u8(red, alpha);
        green = unpremultiply_u8(green, alpha);
        blue = unpremultiply_u8(blue, alpha);
    }
    (red, green, blue, alpha)
}

#[cfg(any(target_os = "macos", test))]
fn unpremultiply_u8(channel: u8, alpha: u8) -> u8 {
    if alpha == 0 {
        return 0;
    }
    if alpha == 255 {
        return channel;
    }
    ((u16::from(channel) * 255) / u16::from(alpha)).min(255) as u8
}

/// `colour_at` is the index of red. Alpha is the sample before it when the
/// alpha is first, and the sample after blue when the alpha is last.
#[cfg(any(target_os = "macos", test))]
fn f16_channels(pixel: &[u8], colour_at: usize, premultiplied: bool) -> (u8, u8, u8, u8) {
    let red = half_to_f32(u16::from_le_bytes([
        pixel[colour_at * 2],
        pixel[colour_at * 2 + 1],
    ]));
    let green = half_to_f32(u16::from_le_bytes([
        pixel[(colour_at + 1) * 2],
        pixel[(colour_at + 1) * 2 + 1],
    ]));
    let blue = half_to_f32(u16::from_le_bytes([
        pixel[(colour_at + 2) * 2],
        pixel[(colour_at + 2) * 2 + 1],
    ]));
    let alpha_at = if colour_at == 0 { 3 } else { 0 };
    let alpha = half_to_f32(u16::from_le_bytes([
        pixel[alpha_at * 2],
        pixel[alpha_at * 2 + 1],
    ]));
    let (red, green, blue) = if premultiplied && alpha > 0.0 {
        (red / alpha, green / alpha, blue / alpha)
    } else if premultiplied {
        (0.0, 0.0, 0.0)
    } else {
        (red, green, blue)
    };
    (
        unit_to_u8(red),
        unit_to_u8(green),
        unit_to_u8(blue),
        unit_to_u8(alpha),
    )
}

#[cfg(any(target_os = "macos", test))]
fn half_to_f32(half: u16) -> f32 {
    let sign = (half >> 15) & 1;
    let exponent = (half >> 10) & 0x1f;
    let fraction = half & 0x3ff;
    let bits = if exponent == 0 {
        if fraction == 0 {
            (sign as u32) << 31
        } else {
            let mut fraction = fraction;
            let mut exponent = -14i32;
            while fraction & 0x400 == 0 {
                fraction <<= 1;
                exponent -= 1;
            }
            fraction &= 0x3ff;
            ((sign as u32) << 31) | (((exponent + 127) as u32) << 23) | ((fraction as u32) << 13)
        }
    } else if exponent == 0x1f {
        ((sign as u32) << 31) | 0x7f80_0000 | ((fraction as u32) << 13)
    } else {
        ((sign as u32) << 31) | (((exponent as u32) + 112) << 23) | ((fraction as u32) << 13)
    };
    f32::from_bits(bits)
}

#[cfg(any(target_os = "macos", test))]
fn unit_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// AppKit reads a binary property list and draws an icon. Both stay on
/// the side that calls them. The scan calls the property-list reader from
/// a background task. The window calls the icon reader on the user
/// interface thread.
#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use objc2::AnyThread;
    use objc2_app_kit::{NSBitmapFormat, NSBitmapImageRep, NSWorkspace};
    use objc2_foundation::{NSMutableDictionary, NSObject, NSPoint, NSRect, NSSize, NSString};
    use std::ffi::CStr;

    /// The side of an application icon, in pixels. The window shows it at
    /// `theme::ICON_APPLICATION` points. A Retina screen can draw a larger
    /// bitmap; the cache keeps the bitmap that AppKit returns.
    const ICON_PIXELS: u32 = 32;

    pub fn binary_plist(path: &Path) -> HashMap<String, String> {
        let Some(text) = path.to_str() else {
            return HashMap::new();
        };
        let path = NSString::from_str(text);
        // `NSString` keeps the path alive for this call.
        let Some(dictionary) = (unsafe {
            NSMutableDictionary::<NSString, NSObject>::dictionaryWithContentsOfFile(&path)
        }) else {
            return HashMap::new();
        };
        let mut values = HashMap::new();
        for key in ["CFBundleDisplayName", "CFBundleName", "CFBundleIdentifier"] {
            let name = NSString::from_str(key);
            let Some(value) = dictionary.objectForKey(&name) else {
                continue;
            };
            let Ok(string) = value.downcast::<NSString>() else {
                continue;
            };
            values.insert(key.to_string(), ns_string(&string));
        }
        values
    }

    fn ns_string(value: &NSString) -> String {
        let pointer = value.UTF8String();
        if pointer.is_null() {
            return String::new();
        }
        // `UTF8String` stays valid while `value` is alive.
        unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned()
    }

    /// Draw the workspace icon into a bitmap. `initWithFocusedViewRect` is
    /// deprecated. Its replacement snapshots a view, and an icon is not a
    /// view.
    #[allow(deprecated)]
    pub fn icon(path: &Path) -> Option<image::RgbaImage> {
        let text = path.to_str()?;
        let image = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(text));
        let side = f64::from(ICON_PIXELS);
        image.setSize(NSSize::new(side, side));
        image.lockFocus();
        let rep = NSBitmapImageRep::initWithFocusedViewRect(
            NSBitmapImageRep::alloc(),
            NSRect::new(NSPoint::new(0., 0.), NSSize::new(side, side)),
        );
        image.unlockFocus();
        let rep = rep?;
        rgba_image(&rep)
    }

    fn rgba_image(rep: &NSBitmapImageRep) -> Option<image::RgbaImage> {
        if rep.isPlanar() || rep.samplesPerPixel() != 4 {
            return None;
        }
        let format = rep.bitmapFormat();
        let width = usize::try_from(rep.pixelsWide()).ok()?;
        let height = usize::try_from(rep.pixelsHigh()).ok()?;
        let row_bytes = usize::try_from(rep.bytesPerRow()).ok()?;
        let bytes_per_pixel = match rep.bitsPerPixel() {
            32 if !format.contains(NSBitmapFormat::FloatingPointSamples) => 4,
            64 if format.contains(NSBitmapFormat::FloatingPointSamples) => 8,
            _ => return None,
        };
        if width == 0 || height == 0 || row_bytes < width * bytes_per_pixel {
            return None;
        }
        let pointer = rep.bitmapData();
        if pointer.is_null() {
            return None;
        }
        let byte_count = height * row_bytes;
        // The pointer stays valid while `rep` is alive.
        let data = unsafe { std::slice::from_raw_parts(pointer, byte_count) };
        let layout = if bytes_per_pixel == 8 {
            if format.contains(NSBitmapFormat::AlphaFirst) {
                super::IconLayout::ArgbF16
            } else {
                super::IconLayout::RgbaF16
            }
        } else {
            match channel_order(format) {
                Order::Bgra => super::IconLayout::Bgra8,
                Order::Rgba => super::IconLayout::Rgba8,
                Order::Argb => super::IconLayout::Argb8,
                Order::Abgr => super::IconLayout::Abgr8,
            }
        };
        let premultiplied = !format.contains(NSBitmapFormat::AlphaNonpremultiplied);
        super::decode_icon_bitmap(width, height, row_bytes, data, layout, premultiplied)
    }

    /// The byte order of a 32-bit pixel. AppKit's focused bitmap is
    /// little-endian with alpha first, which is BGRA in memory. An empty
    /// format flag uses that order too.
    fn channel_order(format: NSBitmapFormat) -> Order {
        let little = format.contains(NSBitmapFormat::ThirtyTwoBitLittleEndian)
            || !format.contains(NSBitmapFormat::ThirtyTwoBitBigEndian);
        let alpha_first = format.contains(NSBitmapFormat::AlphaFirst) || format.is_empty();
        match (little, alpha_first) {
            (true, true) => Order::Bgra,
            (true, false) => Order::Abgr,
            (false, true) => Order::Argb,
            (false, false) => Order::Rgba,
        }
    }

    #[derive(Clone, Copy)]
    enum Order {
        Bgra,
        Rgba,
        Argb,
        Abgr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_bundle(directory: &Path, folder: &str, plist: &str) -> PathBuf {
        let contents = directory.join(folder).join("Contents");
        std::fs::create_dir_all(&contents).unwrap();
        std::fs::write(contents.join("Info.plist"), plist).unwrap();
        directory.join(folder)
    }

    fn plist(display: &str, bundle_name: &str, identifier: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>LSRequiresNativeExecution</key>
    <true/>
    <key>CFBundleDisplayName</key>
    <string>{display}</string>
    <key>CFBundleName</key>
    <string>{bundle_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{identifier}</string>
</dict>
</plist>
"#
        )
    }

    #[test]
    fn a_scratch_directory_gives_the_name_and_the_bundle_identifier() {
        let root = tempfile::tempdir().unwrap();
        let applications = root.path().join("Applications");
        let utilities = applications.join("Utilities");
        write_bundle(
            &applications,
            "Mail.app",
            &plist("Mail &amp; News", "MailBundle", "com.example.mail"),
        );
        write_bundle(
            &utilities,
            "Terminal.app",
            &plist("", "Terminal", "com.example.terminal"),
        );
        write_bundle(
            &applications,
            "Notes.app",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Notes</string>
<key>CFBundleIdentifier</key><string>com.example.notes</string>
</dict></plist>"#,
        );
        std::fs::create_dir_all(applications.join("Helper.app")).unwrap();
        std::fs::write(applications.join("Not.app"), "not a bundle").unwrap();
        std::fs::create_dir_all(applications.join("Nested/Inner.app/Contents")).unwrap();
        std::fs::write(applications.join("readme.txt"), "text").unwrap();

        let found = scan(&[applications.clone(), utilities.clone()]);
        let names: Vec<_> = found.iter().map(|app| app.name.as_str()).collect();
        assert_eq!(names, ["Helper", "Mail & News", "Notes", "Terminal"]);
        let mail = found.iter().find(|app| app.name == "Mail & News").unwrap();
        assert_eq!(mail.bundle_identifier, "com.example.mail");
        assert_eq!(mail.path, applications.join("Mail.app"));
        let terminal = found.iter().find(|app| app.name == "Terminal").unwrap();
        assert_eq!(terminal.bundle_identifier, "com.example.terminal");
        assert_eq!(terminal.path, utilities.join("Terminal.app"));
        let helper = found.iter().find(|app| app.name == "Helper").unwrap();
        assert_eq!(helper.bundle_identifier, "");
        assert!(found.iter().all(|app| !app.path.ends_with("Inner.app")));
        assert!(found.iter().all(|app| app.file_stem() != "Not"));

        let again = scan(&[applications.clone(), applications]);
        assert_eq!(
            again.iter().filter(|app| app.name == "Mail & News").count(),
            1
        );
    }

    #[test]
    fn a_query_and_a_stored_value_find_the_application() {
        let app = InstalledApplication {
            name: "Mail & News".into(),
            bundle_identifier: "com.example.mail".into(),
            path: PathBuf::from("/Applications/Mail.app"),
        };
        assert!(app.matches_query(""));
        assert!(app.matches_query(" news"));
        assert!(app.matches_query("EXAMPLE.Mail"));
        assert!(app.matches_query("mail"));
        assert!(!app.matches_query("calendar"));
        assert!(app.matches_stored("Mail & News"));
        assert!(app.matches_stored("mail.app"));
        assert!(app.matches_stored("com.example.mail"));
        assert!(app.matches_stored("/Applications/Mail.app"));
        assert!(app.matches_stored("/Applications/Mail.app/"));
        assert!(!app.matches_stored("/Applications/Other.app"));
        assert!(!app.matches_stored(""));
        if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(&home).join("Applications/Sample.app");
            let home_app = InstalledApplication {
                name: "Sample".into(),
                bundle_identifier: "com.example.sample".into(),
                path: path.clone(),
            };
            let stored = format!("~/{}", path.strip_prefix(&home).unwrap().display());
            assert!(home_app.matches_stored(&stored));
        }
    }

    #[test]
    fn standard_directories_are_the_usual_application_folders() {
        let directories = standard_directories();
        for path in [
            "/Applications",
            "/Applications/Utilities",
            "/System/Applications",
            "/System/Applications/Utilities",
        ] {
            assert!(directories
                .iter()
                .any(|directory| directory == Path::new(path)));
        }
        if let Some(home) = std::env::var_os("HOME").filter(|home| !home.is_empty()) {
            assert!(directories
                .iter()
                .any(|directory| directory == &PathBuf::from(&home).join("Applications")));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_binary_plist_in_a_scratch_directory_gives_the_same_names() {
        let root = tempfile::tempdir().unwrap();
        let xml = plist("Binary Mail", "Binary", "com.example.binary");
        let bundle = write_bundle(root.path(), "Binary.app", &xml);
        let plist_path = bundle.join("Contents/Info.plist");
        let status = std::process::Command::new("/usr/bin/plutil")
            .args([
                "-convert",
                "binary1",
                plist_path.to_str().expect("the scratch path is text"),
            ])
            .status()
            .expect("plutil");
        assert!(status.success());
        let found = scan(&[root.path().to_path_buf()]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Binary Mail");
        assert_eq!(found[0].bundle_identifier, "com.example.binary");
    }

    #[test]
    fn a_16_bit_float_bitmap_becomes_8_bit_colour() {
        assert_eq!(super::half_to_f32(0x3c00), 1.0);
        assert_eq!(super::half_to_f32(0x3800), 0.5);
        assert_eq!(super::half_to_f32(0), 0.0);
        // Premultiplied red at half alpha, then opaque blue.
        let mut bytes = Vec::new();
        for bits in [0x3800u16, 0, 0, 0x3800, 0, 0, 0x3c00, 0x3c00] {
            bytes.extend(bits.to_le_bytes());
        }
        let image = super::decode_icon_bitmap(2, 1, 16, &bytes, super::IconLayout::RgbaF16, true)
            .expect("the bitmap has two pixels");
        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 128]);
        assert_eq!(image.get_pixel(1, 0).0, [0, 0, 255, 255]);
    }
}
