//! The profile: a named set of pages for one device model. A page holds the
//! content of each surface and the bindings.
//!
//! These types are the profile in memory. The file format is in
//! [`crate::store`], and the profiles directory is in [`crate::profiles`].
//! The engine holds the number of the active page: see [`crate::engine`].

use crate::action::{Action, GestureKind};
use opencraft_core::device::DESCRIPTORS;
use opencraft_core::{ControlId, ControlKind, SurfaceId, SurfaceKind};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// The name of a new profile.
pub const DEFAULT_PROFILE_NAME: &str = "Default";
/// The name of the first page of a new profile.
pub const DEFAULT_PAGE_NAME: &str = "Page 1";

/// The model identifier of the first descriptor. A profile file of schema
/// version 1 or 2 has no model, and gets this one: the earlier phases
/// supported only this model (decision 0018).
pub fn default_model() -> &'static str {
    DESCRIPTORS[0].model
}

/// A colour: red, green, and blue. The text form is `#RRGGBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color(pub [u8; 3]);

impl Color {
    pub const BLACK: Color = Color([0, 0, 0]);
    pub const WHITE: Color = Color([255, 255, 255]);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color([r, g, b])
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [r, g, b] = self.0;
        write!(f, "#{r:02X}{g:02X}{b:02X}")
    }
}

/// The text is not a colour of the form `#RRGGBB`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseColorError;

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a colour has the form #RRGGBB")
    }
}

impl FromStr for Color {
    type Err = ParseColorError;

    /// Read `#RRGGBB`. The `#` is optional, and the letters can be small.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ParseColorError);
        }
        let part = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ParseColorError);
        Ok(Color([part(0)?, part(2)?, part(4)?]))
    }
}

/// What the user put on one surface. Each part is optional.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceContent {
    /// The text on the surface.
    pub label: Option<String>,
    /// The colour behind the image and the label.
    pub color: Option<Color>,
    /// The image, as a path relative to the profile directory. The store
    /// makes this path: see [`crate::store::ProfileStore::import_image`].
    pub image: Option<String>,
}

impl SurfaceContent {
    /// True if the content has no part.
    pub fn is_empty(&self) -> bool {
        self.label.is_none() && self.color.is_none() && self.image.is_none()
    }
}

/// One full set of bindings and surface content for a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub name: String,
    /// The content of each surface that has content. A surface with no
    /// content is not in the map: use [`Page::update`] to change the map.
    surfaces: BTreeMap<SurfaceId, SurfaceContent>,
    bindings: BTreeMap<(ControlId, GestureKind), Action>,
}

impl Page {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            surfaces: BTreeMap::new(),
            bindings: BTreeMap::new(),
        }
    }

    pub fn binding(&self, control: ControlId, gesture: GestureKind) -> Option<&Action> {
        self.bindings.get(&(control, gesture))
    }

    pub fn bindings(&self) -> impl Iterator<Item = ((ControlId, GestureKind), &Action)> {
        self.bindings.iter().map(|(key, action)| (*key, action))
    }

    /// Set or remove a binding. Return true if it changes.
    pub fn set_binding(
        &mut self,
        control: ControlId,
        gesture: GestureKind,
        action: Option<Action>,
    ) -> bool {
        if self.binding(control, gesture) == action.as_ref() {
            return false;
        }
        if let Some(action) = action {
            self.bindings.insert((control, gesture), action);
        } else {
            self.bindings.remove(&(control, gesture));
        }
        true
    }

    /// The content of `surface`. A surface with no content gives the empty
    /// content.
    pub fn content(&self, surface: SurfaceId) -> SurfaceContent {
        self.surfaces.get(&surface).cloned().unwrap_or_default()
    }

    /// The surfaces that have content, with the content of each.
    pub fn surfaces(&self) -> impl Iterator<Item = (SurfaceId, &SurfaceContent)> {
        self.surfaces.iter().map(|(id, content)| (*id, content))
    }

    /// Change the content of `surface` with `change`. An empty label becomes
    /// no label. Return true if the content is different after the change.
    pub fn update(&mut self, surface: SurfaceId, change: impl FnOnce(&mut SurfaceContent)) -> bool {
        let before = self.content(surface);
        let mut after = before.clone();
        change(&mut after);
        if after.label.as_deref() == Some("") {
            after.label = None;
        }
        if after == before {
            return false;
        }
        if after.is_empty() {
            self.surfaces.remove(&surface);
        } else {
            self.surfaces.insert(surface, after);
        }
        true
    }
}

/// A named set of pages for one device model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// The name for the user. The identity of a profile is the name of its
    /// directory: see [`crate::profiles`].
    pub name: String,
    /// The model identifier of the descriptor, for example `n4-pro`.
    pub model: String,
    /// The pages. A profile has one page or more: see [`Profile::new`].
    pages: Vec<Page>,
}

impl Default for Profile {
    /// A new profile with one page and no content.
    fn default() -> Self {
        Self::new(DEFAULT_PROFILE_NAME, Vec::new())
    }
}

impl Profile {
    /// Make a profile for the model of the first descriptor: see
    /// [`default_model`]. If `pages` is empty, the profile gets one page with
    /// no content.
    pub fn new(name: impl Into<String>, pages: Vec<Page>) -> Self {
        Self::for_model(default_model(), name, pages)
    }

    /// Make a profile for the model `model`. If `pages` is empty, the profile
    /// gets one page with no content.
    pub fn for_model(
        model: impl Into<String>,
        name: impl Into<String>,
        mut pages: Vec<Page>,
    ) -> Self {
        if pages.is_empty() {
            pages.push(Page::new(DEFAULT_PAGE_NAME));
        }
        Self {
            name: name.into(),
            model: model.into(),
            pages,
        }
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// One page. Page numbers start at 0.
    pub fn page(&self, page: usize) -> Option<&Page> {
        self.pages.get(page)
    }

    /// Get one page for a change. Page numbers start at 0.
    pub fn page_mut(&mut self, page: usize) -> Option<&mut Page> {
        self.pages.get_mut(page)
    }

    /// Put `page` after the last page. Return its number.
    pub fn add_page(&mut self, page: Page) -> usize {
        self.pages.push(page);
        self.pages.len() - 1
    }

    /// Move the page `page` to the position `to`. The pages between the two
    /// positions get new numbers. Return false if a number is not a page.
    pub fn move_page(&mut self, page: usize, to: usize) -> bool {
        if page >= self.pages.len() || to >= self.pages.len() {
            return false;
        }
        let moved = self.pages.remove(page);
        self.pages.insert(to, moved);
        true
    }

    /// Remove the page `page`. Return false if it is not a page, or if it is
    /// the only page: a profile keeps one page at the minimum.
    pub fn remove_page(&mut self, page: usize) -> bool {
        if page >= self.pages.len() || self.pages.len() == 1 {
            return false;
        }
        self.pages.remove(page);
        true
    }

    /// Put `pages` in place of all pages, and return the pages that were
    /// there. If `pages` is empty, the profile gets one page with no
    /// content. The undo and the redo of the engine use this.
    pub fn replace_pages(&mut self, mut pages: Vec<Page>) -> Vec<Page> {
        if pages.is_empty() {
            pages.push(Page::new(DEFAULT_PAGE_NAME));
        }
        std::mem::replace(&mut self.pages, pages)
    }

    /// A name for a new page: `Page N`, where N is the number of pages after
    /// the add. If a page has that name, N increases.
    pub fn new_page_name(&self) -> String {
        (self.pages.len() + 1..)
            .map(|n| format!("Page {n}"))
            .find(|name| self.pages.iter().all(|p| &p.name != name))
            .expect("a name that no page has")
    }

    /// Each image path that a surface of the profile uses.
    pub fn image_paths(&self) -> impl Iterator<Item = &str> {
        self.pages
            .iter()
            .flat_map(|p| p.surfaces.values())
            .filter_map(|c| c.image.as_deref())
    }
}

/// The text form of a surface identity in the profile file, for example
/// `key-face-3`.
pub fn surface_key(id: SurfaceId) -> String {
    format!("{}-{}", kind_key(id.kind), id.number)
}

/// Read the text form of a surface identity. See [`surface_key`].
pub fn parse_surface_key(key: &str) -> Option<SurfaceId> {
    let (kind, number) = key.rsplit_once('-')?;
    let kind = [
        SurfaceKind::KeyFace,
        SurfaceKind::BarPanel,
        SurfaceKind::Background,
    ]
    .into_iter()
    .find(|k| kind_key(*k) == kind)?;
    // Only the form that `surface_key` writes: no sign, no zero in front.
    if number.starts_with('0') || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(SurfaceId::new(kind, number.parse().ok()?))
}

/// The text form of a control identity in the profile file.
pub fn control_key(id: ControlId) -> String {
    let kind = match id.kind {
        ControlKind::Key => "key",
        ControlKind::Knob => "knob",
        ControlKind::BarPanel => "bar-panel",
        ControlKind::TouchBar => "touch-bar",
    };
    format!("{kind}-{}", id.number)
}

/// Read the text form of a control identity. See [`control_key`].
pub fn parse_control_key(key: &str) -> Option<ControlId> {
    let (kind, number) = key.rsplit_once('-')?;
    let kind = match kind {
        "key" => ControlKind::Key,
        "knob" => ControlKind::Knob,
        "bar-panel" => ControlKind::BarPanel,
        "touch-bar" => ControlKind::TouchBar,
        _ => return None,
    };
    let id = ControlId::new(kind, number.parse().ok()?);
    (id.number > 0 && control_key(id) == key).then_some(id)
}

fn kind_key(kind: SurfaceKind) -> &'static str {
    match kind {
        SurfaceKind::KeyFace => "key-face",
        SurfaceKind::BarPanel => "bar-panel",
        SurfaceKind::Background => "background",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_3: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 3);

    #[test]
    fn a_colour_has_the_text_form_of_six_hex_digits() {
        assert_eq!(Color::rgb(0x1E, 0x6F, 0xD9).to_string(), "#1E6FD9");
        assert_eq!("#1E6FD9".parse(), Ok(Color::rgb(0x1E, 0x6F, 0xD9)));
        assert_eq!("1e6fd9".parse(), Ok(Color::rgb(0x1E, 0x6F, 0xD9)));
        for bad in ["", "#", "#12345", "#1234567", "#12345G", "#+12345", "red"] {
            assert_eq!(bad.parse::<Color>(), Err(ParseColorError), "{bad}");
        }
    }

    #[test]
    fn a_surface_key_goes_to_text_and_back() {
        for (id, key) in [
            (KEY_3, "key-face-3"),
            (SurfaceId::new(SurfaceKind::BarPanel, 1), "bar-panel-1"),
            (SurfaceId::new(SurfaceKind::Background, 1), "background-1"),
            (SurfaceId::new(SurfaceKind::KeyFace, 255), "key-face-255"),
        ] {
            assert_eq!(surface_key(id), key);
            assert_eq!(parse_surface_key(key), Some(id));
        }
        for bad in [
            "",
            "key-face",
            "key-face-",
            "key-face-256",
            "key-face-03",
            "key-face-+3",
            "key-face--3",
            "knob-1",
            "3",
        ] {
            assert_eq!(parse_surface_key(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_page_keeps_only_surfaces_that_have_content() {
        let mut page = Page::new("Page 1");
        assert!(page.content(KEY_3).is_empty());

        assert!(page.update(KEY_3, |c| c.label = Some("Mail".into())));
        assert_eq!(page.content(KEY_3).label.as_deref(), Some("Mail"));
        assert_eq!(page.surfaces().count(), 1);

        // The same value is not a change.
        assert!(!page.update(KEY_3, |c| c.label = Some("Mail".into())));

        // An empty label is no label, and a surface with no content goes away.
        assert!(page.update(KEY_3, |c| c.label = Some(String::new())));
        assert_eq!(page.surfaces().count(), 0);
        assert!(!page.update(KEY_3, |c| c.label = Some(String::new())));
    }

    #[test]
    fn a_profile_has_one_page_or_more() {
        assert_eq!(Profile::default().pages().len(), 1);
        assert_eq!(Profile::default().model, default_model());
        assert_eq!(
            Profile::for_model("other", "P", Vec::new()).pages().len(),
            1
        );
        assert_eq!(Profile::new("P", Vec::new()).pages().len(), 1);
        assert_eq!(Profile::default().pages()[0].name, DEFAULT_PAGE_NAME);
    }

    fn names(profile: &Profile) -> Vec<&str> {
        profile.pages().iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn pages_can_be_added_moved_and_removed_and_one_page_stays() {
        let mut profile = Profile::default();
        assert_eq!(profile.new_page_name(), "Page 2");
        assert_eq!(profile.add_page(Page::new("B")), 1);
        assert_eq!(profile.add_page(Page::new("C")), 2);
        assert_eq!(names(&profile), ["Page 1", "B", "C"]);

        assert!(profile.move_page(2, 0));
        assert_eq!(names(&profile), ["C", "Page 1", "B"]);
        assert!(profile.move_page(0, 2));
        assert_eq!(names(&profile), ["Page 1", "B", "C"]);
        assert!(!profile.move_page(3, 0));
        assert!(!profile.move_page(0, 3));

        assert!(!profile.remove_page(3));
        assert!(profile.remove_page(1));
        assert!(profile.remove_page(0));
        assert_eq!(names(&profile), ["C"]);
        assert!(!profile.remove_page(0), "the only page stays");
    }

    #[test]
    fn a_new_page_name_is_not_in_use() {
        let mut profile = Profile::new("P", vec![Page::new("Page 2"), Page::new("Page 3")]);
        assert_eq!(profile.new_page_name(), "Page 4");
        profile.page_mut(1).unwrap().name = "Other".into();
        assert_eq!(profile.new_page_name(), "Page 3");
    }

    #[test]
    fn a_profile_lists_the_image_paths_of_its_surfaces() {
        let mut profile = Profile::default();
        let page = profile.page_mut(0).unwrap();
        page.update(KEY_3, |c| c.image = Some("images/a.png".into()));
        page.update(SurfaceId::new(SurfaceKind::BarPanel, 1), |c| {
            c.label = Some("x".into())
        });
        assert_eq!(profile.image_paths().collect::<Vec<_>>(), ["images/a.png"]);
    }
}
