//! The built-in icon set for surfaces: the line icons of Lucide (decision
//! 0033).
//!
//! The data is `icons/lucide.tsv` of this crate. `scripts/lucide-icons.sh`
//! makes it from one Lucide release. Each icon has a name, for example
//! `globe`, a category, tags for the search, and its SVG elements on a grid
//! of 24 × 24. The licence of the icons is `icons/LICENSE`.
//!
//! The engine draws an icon on a surface with [`draw`] (architecture rule
//! 11). A shell uses the same data: [`Icon::svg`] gives the SVG text of an
//! icon for a picture in the window, for example in the icon picker and on a
//! kind tile. A kind description names its icon with a name of this set.
//!
//! The data is parsed one time, at the first use.

use crate::profile::Color;
use image::RgbImage;
use resvg::{tiny_skia, usvg};
use std::sync::OnceLock;

/// The name of the icon set, for the user.
pub const ICON_SET_NAME: &str = "Lucide";
/// The version of the icon set.
pub const ICON_SET_VERSION: &str = "1.43.0";

/// The width and height of the grid of an icon, in its own units.
const GRID: f32 = 24.;

/// The data file: see `scripts/lucide-icons.sh` for the format.
const DATA: &str = include_str!("../icons/lucide.tsv");

/// One icon of the set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    /// The name, for example `arrow-right`. The profile file keeps it.
    pub name: &'static str,
    /// The index of the category in [`IconSet::categories`].
    pub category: usize,
    /// The tags, separated by commas, for example `home,living`.
    tags: &'static str,
    /// The SVG elements of the icon, with no element `<svg>`.
    body: &'static str,
}

impl Icon {
    /// The tags of the icon: words for the search.
    pub fn tags(&self) -> impl Iterator<Item = &'static str> {
        self.tags.split(',').filter(|t| !t.is_empty())
    }

    /// The SVG text of the icon, 24 × 24, with the stroke in the current
    /// colour (`currentColor`). A shell can show it as a picture of one
    /// colour.
    pub fn svg(&self) -> String {
        self.svg_with("currentColor")
    }

    fn svg_with(&self, stroke: &str) -> String {
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" \
             viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2\" \
             stroke-linecap=\"round\" stroke-linejoin=\"round\">{}</svg>",
            self.body.replace("currentColor", stroke)
        )
    }

    /// The name with spaces for the hyphens, for example "arrow right": the
    /// accessible label of the icon in a shell.
    pub fn title(&self) -> String {
        self.name.replace('-', " ")
    }
}

/// A category of icons, for example "Multimedia".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconCategory {
    /// The key of the category in the data, for example `multimedia`.
    pub key: &'static str,
    /// The title for the user, for example "Multimedia".
    pub title: &'static str,
}

/// The icon set.
#[derive(Debug)]
pub struct IconSet {
    /// The categories, in the order of their titles.
    categories: Vec<IconCategory>,
    /// The icons, in the order of their names.
    icons: Vec<Icon>,
}

/// The icon set of the engine.
pub fn icon_set() -> &'static IconSet {
    static SET: OnceLock<IconSet> = OnceLock::new();
    SET.get_or_init(|| IconSet::parse(DATA))
}

impl IconSet {
    /// Read the data. A line with an error is not in the set: the tests
    /// read all lines of the real data.
    fn parse(data: &'static str) -> Self {
        let mut categories = Vec::new();
        let mut icons = Vec::new();
        let mut keys = Vec::new();
        for line in data.lines().filter(|l| !l.starts_with('#')) {
            let fields: Vec<&'static str> = line.split('\t').collect();
            match fields.as_slice() {
                ["category", key, title] => categories.push(IconCategory { key, title }),
                ["icon", name, category, tags, body] => {
                    keys.push(*category);
                    icons.push(Icon {
                        name,
                        category: 0,
                        tags,
                        body,
                    });
                }
                _ => {}
            }
        }
        categories.sort_by(|a, b| a.title.cmp(b.title));
        for (icon, key) in icons.iter_mut().zip(keys) {
            icon.category = categories
                .iter()
                .position(|c| c.key == key)
                .unwrap_or_default();
        }
        icons.sort_by(|a, b| a.name.cmp(b.name));
        Self { categories, icons }
    }

    /// All icons, in the order of their names.
    pub fn icons(&self) -> &[Icon] {
        &self.icons
    }

    /// All categories, in the order of their titles.
    pub fn categories(&self) -> &[IconCategory] {
        &self.categories
    }

    /// The icon with the name `name`.
    pub fn get(&self, name: &str) -> Option<&Icon> {
        self.icons
            .binary_search_by(|icon| icon.name.cmp(name))
            .ok()
            .map(|i| &self.icons[i])
    }

    /// The icons of each category, in the order of the categories. A
    /// category with no icon is not in the list.
    pub fn groups(&self) -> Vec<(&IconCategory, Vec<&Icon>)> {
        let mut groups: Vec<(&IconCategory, Vec<&Icon>)> =
            self.categories.iter().map(|c| (c, Vec::new())).collect();
        for icon in &self.icons {
            if let Some((_, icons)) = groups.get_mut(icon.category) {
                icons.push(icon);
            }
        }
        groups.retain(|(_, icons)| !icons.is_empty());
        groups
    }

    /// The icons that agree with the text `query`, the best first. Each
    /// word of the query must be in the name or in a tag, with no
    /// difference between capital and small letters. First come the icons
    /// with the query as their name, then the icons whose name starts with
    /// the query, then the icons that have all words in their name, and
    /// last the icons that need a tag. An empty query gives all icons.
    pub fn search(&self, query: &str) -> Vec<&Icon> {
        let query = query.trim().to_lowercase();
        let words: Vec<&str> = query.split([' ', '-']).filter(|w| !w.is_empty()).collect();
        if words.is_empty() {
            return self.icons.iter().collect();
        }
        let joined = words.join("-");
        let mut found: Vec<(u8, &Icon)> = self
            .icons
            .iter()
            .filter_map(|icon| {
                let in_name = |w: &&str| icon.name.contains(*w);
                let rank = if icon.name == joined {
                    0
                } else if icon.name.starts_with(&joined) {
                    1
                } else if words.iter().all(in_name) {
                    2
                } else if words
                    .iter()
                    .all(|w| in_name(w) || icon.tags().any(|t| t.contains(*w)))
                {
                    3
                } else {
                    return None;
                };
                Some((rank, icon))
            })
            .collect();
        // A stable sort keeps the order of the names in each rank.
        found.sort_by_key(|(rank, _)| *rank);
        found.into_iter().map(|(_, icon)| icon).collect()
    }
}

/// Draw `icon` in `color` on `canvas`, in the square with the top-left
/// corner `x`, `y` and the side `side`, in pixels. The edges of the strokes
/// have anti-aliasing. A part of the square out of the canvas is not drawn.
pub fn draw(canvas: &mut RgbImage, icon: &Icon, color: Color, x: u32, y: u32, side: u32) {
    let Some(mut pixmap) = tiny_skia::Pixmap::new(side, side) else {
        return;
    };
    let svg = icon.svg_with(&color.to_string());
    let Ok(tree) = usvg::Tree::from_str(&svg, &usvg::Options::default()) else {
        return;
    };
    let scale = side as f32 / GRID;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // The pixels of the pixmap have their colour multiplied by their alpha.
    for (i, pixel) in pixmap.pixels().iter().enumerate() {
        let alpha = u16::from(pixel.alpha());
        if alpha == 0 {
            continue;
        }
        let (px, py) = (x + i as u32 % side, y + i as u32 / side);
        if px >= canvas.width() || py >= canvas.height() {
            continue;
        }
        let under = canvas.get_pixel_mut(px, py);
        let over = [pixel.red(), pixel.green(), pixel.blue()];
        for (channel, value) in under.0.iter_mut().zip(over) {
            let mixed = u16::from(value) + (u16::from(*channel) * (255 - alpha) + 127) / 255;
            *channel = mixed.min(255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn the_set_has_all_icons_and_categories_of_the_release() {
        let set = icon_set();
        assert_eq!(set.icons().len(), 1818);
        assert_eq!(set.categories().len(), 42);
        assert!(set.icons().windows(2).all(|w| w[0].name < w[1].name));
        // Each icon has a real category, and each line of the data is read.
        let lines = DATA.lines().filter(|l| l.starts_with("icon\t")).count();
        assert_eq!(lines, set.icons().len());
        for icon in set.icons() {
            let category = &set.categories()[icon.category];
            assert!(DATA.contains(&format!("icon\t{}\t{}\t", icon.name, category.key)));
        }
        assert_eq!(
            set.groups().iter().map(|(_, i)| i.len()).sum::<usize>(),
            1818
        );
    }

    #[test]
    fn the_kind_icons_of_decision_0029_are_in_the_set() {
        for name in [
            "app-window",
            "globe",
            "terminal",
            "keyboard",
            "type",
            "play",
            "volume-2",
            "arrow-right",
            "layers",
            "toggle-left",
            "list",
        ] {
            assert!(icon_set().get(name).is_some(), "{name}");
        }
        assert_eq!(icon_set().get("no-such-icon"), None);
    }

    #[test]
    fn each_icon_is_a_correct_svg() {
        for icon in icon_set().icons() {
            let svg = icon.svg();
            assert!(
                usvg::Tree::from_str(&svg, &usvg::Options::default()).is_ok(),
                "{}",
                icon.name
            );
        }
    }

    #[test]
    fn the_search_finds_names_and_tags_the_best_first() {
        let names = |query| -> Vec<&str> {
            icon_set()
                .search(query)
                .into_iter()
                .map(|i| i.name)
                .collect()
        };
        let globe = names("globe");
        assert_eq!(globe[0], "globe");
        assert!(globe.contains(&"globe-lock"));
        // Capital letters, spaces, and hyphens.
        assert_eq!(names("Arrow Right")[0], "arrow-right");
        assert_eq!(names("arrow-right")[0], "arrow-right");
        // A tag: "mute" is a tag of "volume-x".
        assert!(names("mute").contains(&"volume-x"));
        // An icon with the word in its name comes before an icon that has it
        // only in a tag.
        let house = names("house");
        let first_tag_only = house
            .iter()
            .position(|n| !n.contains("house"))
            .unwrap_or(house.len());
        assert!(house[..first_tag_only].iter().all(|n| n.contains("house")));
        assert!(house[first_tag_only..].iter().all(|n| !n.contains("house")));
        assert!(names("zzzz-no-icon").is_empty());
        assert_eq!(names("  ").len(), 1818);
    }

    #[test]
    fn an_icon_draws_in_its_colour_inside_its_square() {
        let red = Color::rgb(220, 20, 20);
        let mut canvas = RgbImage::from_pixel(112, 112, Rgb([0, 0, 0]));
        let icon = icon_set().get("square").unwrap();
        draw(&mut canvas, icon, red, 20, 20, 72);
        // The square has its stroke at 3..21 of 24: at 29 to 83 here, 6 wide.
        assert_eq!(canvas.get_pixel(56, 30), &Rgb(red.0));
        assert_eq!(canvas.get_pixel(30, 56), &Rgb(red.0));
        // The inside and the outside of the square stay black.
        assert_eq!(canvas.get_pixel(56, 56), &Rgb([0, 0, 0]));
        assert_eq!(canvas.get_pixel(10, 10), &Rgb([0, 0, 0]));
        assert_eq!(canvas.get_pixel(100, 100), &Rgb([0, 0, 0]));
        // No pixel out of the square changes.
        for (x, y, p) in canvas.enumerate_pixels() {
            if !(20..92).contains(&x) || !(20..92).contains(&y) {
                assert_eq!(p, &Rgb([0, 0, 0]), "{x} {y}");
            }
        }
    }

    #[test]
    fn an_icon_at_the_edge_of_the_canvas_does_not_fail() {
        let mut canvas = RgbImage::from_pixel(20, 20, Rgb([0, 0, 0]));
        let icon = icon_set().get("circle").unwrap();
        draw(&mut canvas, icon, Color::WHITE, 10, 10, 48);
        draw(&mut canvas, icon, Color::WHITE, 0, 0, 0);
        assert!(canvas.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
