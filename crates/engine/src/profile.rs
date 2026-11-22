//! The profile: a named set of pages for one device model. A page holds the
//! content of each surface and the bindings.
//!
//! These types are the profile in memory. The file format is in
//! [`crate::store`], and the profiles directory is in [`crate::profiles`].
//! The engine holds the location: see [`crate::location`].

use crate::action::{Action, GestureKind};
use opencraft_core::device::DESCRIPTORS;
use opencraft_core::{ControlId, ControlKind, SurfaceId, SurfaceKind};
use std::collections::hash_map::RandomState;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::hash::BuildHasher;
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

/// Where the label of a surface is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabelPosition {
    /// At the top. With an image or an icon, the label is in a band above it.
    Top,
    /// At the centre. With an image or an icon, the label is on it.
    Middle,
    /// At the bottom. With an image or an icon, the label is in a band below it.
    Bottom,
    /// The surface does not show the label. The text stays in the profile.
    Hidden,
}

impl LabelPosition {
    /// All positions, in the order of the window.
    pub const ALL: [LabelPosition; 4] = [Self::Top, Self::Middle, Self::Bottom, Self::Hidden];

    /// The text form in the profile file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Middle => "middle",
            Self::Bottom => "bottom",
            Self::Hidden => "none",
        }
    }

    /// Read the text form. See [`LabelPosition::key`].
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == key)
    }
}

/// The file word of the system font. A family with this name cannot be
/// selected, and the word is not a family name (decision 0043).
pub const SYSTEM_FONT_WORD: &str = "system";

/// The font size of a label that has no size of its own, in pixels of the
/// surface image. A label of the earlier versions has this size.
pub const DEFAULT_LABEL_SIZE: u8 = 20;
/// The weight of a label that has no weight of its own. A label of the
/// earlier versions is bold.
pub const DEFAULT_LABEL_BOLD: bool = true;
/// The smallest and the largest font size of a label, in pixels.
pub const MIN_LABEL_SIZE: u8 = 10;
pub const MAX_LABEL_SIZE: u8 = 40;
/// The font sizes that a shell offers, in pixels.
pub const LABEL_SIZES: [u8; 9] = [10, 12, 14, 16, 18, 20, 24, 28, 32];

/// The smallest zoom of an image, in percent. It is the fit: the largest
/// image with its proportion, at the centre (decision 0043).
pub const MIN_IMAGE_ZOOM: u16 = 100;
/// The largest zoom of an image, in percent.
pub const MAX_IMAGE_ZOOM: u16 = 400;
/// The largest pan of an image, in per mille of the overflow of each axis.
pub const MAX_IMAGE_PAN: i16 = 1000;
/// The zoom values that a shell offers, in percent. The first value is the
/// fit.
pub const IMAGE_ZOOMS: [u16; 7] = [100, 125, 150, 200, 250, 300, 400];

/// The crop of the image of a surface: the zoom, and the pan of the image
/// inside the area of the face (decision 0043).
///
/// The values are integers: [`SurfaceContent`] derives `Eq`, and a float
/// loses its exact value in the profile file. `zoom = 100` with no pan is
/// the fit, thus it is the same picture as no crop. The store writes no
/// keys for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageCrop {
    /// The zoom in percent, from [`MIN_IMAGE_ZOOM`] to [`MAX_IMAGE_ZOOM`].
    /// 100 is the fit.
    pub zoom: u16,
    /// The pan on the x axis, in per mille of the overflow, from
    /// -[`MAX_IMAGE_PAN`] to [`MAX_IMAGE_PAN`]. A positive value moves the
    /// image to the right, thus the face shows more of the left of the
    /// image.
    pub x: i16,
    /// The pan on the y axis, in per mille of the overflow, from
    /// -[`MAX_IMAGE_PAN`] to [`MAX_IMAGE_PAN`]. A positive value moves the
    /// image down, thus the face shows more of the top.
    pub y: i16,
}

impl Default for ImageCrop {
    /// The fit: zoom 100, and no pan.
    fn default() -> Self {
        Self {
            zoom: MIN_IMAGE_ZOOM,
            x: 0,
            y: 0,
        }
    }
}

impl ImageCrop {
    /// True if the crop is the fit: zoom 100 and no pan. It is the same
    /// picture as no crop.
    pub fn is_fit(&self) -> bool {
        *self == Self::default()
    }

    /// The crop with each value in its range. The engine applies a value of
    /// a shell through this function, thus a large number cannot leave the
    /// range of the profile file.
    pub fn clamped(self) -> Self {
        Self {
            zoom: self.zoom.clamp(MIN_IMAGE_ZOOM, MAX_IMAGE_ZOOM),
            x: self.x.clamp(-MAX_IMAGE_PAN, MAX_IMAGE_PAN),
            y: self.y.clamp(-MAX_IMAGE_PAN, MAX_IMAGE_PAN),
        }
    }
}

/// The value of one part of a label: the automatic value of the surface, or
/// a value of the user.
///
/// A part of a label has three forms (decision 0043). No key in the profile
/// file is `None` of an `Option<Auto<T>>`: the surface inherits the default
/// title style of the profile. The automatic word of the file (`"auto"` for
/// the position and the colour, `"system"` for the font) is [`Auto::Auto`]:
/// the surface uses the automatic value also when the profile default has a
/// value. A key with a value is [`Auto::Value`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Auto<T> {
    /// The automatic value of the surface, not the value of the default
    /// title style of the profile.
    Auto,
    /// A value of the user.
    Value(T),
}

impl<T> Auto<T> {
    /// The value of the user. `None` is the automatic value.
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Auto => None,
            Self::Value(value) => Some(value),
        }
    }
}

/// How the label of a surface shows. A field with `None` is not a value of
/// the surface: the surface inherits the default title style of the profile
/// (decision 0043). See [`LabelStyle::resolve`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct LabelStyle {
    /// The font family, for example `"Avenir Next"`. `None` is the system
    /// font, and [`Auto::Auto`] is the system font too.
    pub font: Option<Auto<String>>,
    /// `None` or [`Auto::Auto`]: the automatic position. It is
    /// [`LabelPosition::Middle`] on a surface with no image and no icon, and
    /// [`LabelPosition::Bottom`] on a surface with an image or an icon. See
    /// [`LabelStyle::position_for`].
    pub position: Option<Auto<LabelPosition>>,
    /// The font size, in pixels of the surface image. If the text does not
    /// fit at this size, the font becomes smaller. The engine default is
    /// [`DEFAULT_LABEL_SIZE`].
    pub size: Option<u8>,
    /// `None` or [`Auto::Auto`]: black or white, the colour that shows best
    /// on the fill. See [`crate::render::label_color`].
    pub color: Option<Auto<Color>>,
    /// The weight of the font. The engine default is [`DEFAULT_LABEL_BOLD`].
    pub bold: Option<bool>,
    /// Italic. The engine default is upright.
    pub italic: Option<bool>,
    /// Underline. The engine default is no underline.
    pub underline: Option<bool>,
}

/// A [`LabelStyle`] with a value for each field: the own value of the
/// surface, or the value of the default title style of the profile, or the
/// engine default. The renderer takes this type (decision 0043).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedLabelStyle {
    /// `None`: the system font.
    pub font: Option<String>,
    /// `None`: the automatic position. See
    /// [`ResolvedLabelStyle::position_for`].
    pub position: Option<LabelPosition>,
    /// The font size, in pixels of the surface image.
    pub size: u8,
    /// `None`: the automatic colour.
    pub color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for ResolvedLabelStyle {
    /// The engine default of each field: the system font, the automatic
    /// position, [`DEFAULT_LABEL_SIZE`] pixels, the automatic colour, bold,
    /// upright, and no underline.
    fn default() -> Self {
        Self {
            font: None,
            position: None,
            size: DEFAULT_LABEL_SIZE,
            color: None,
            bold: DEFAULT_LABEL_BOLD,
            italic: false,
            underline: false,
        }
    }
}

impl ResolvedLabelStyle {
    /// The position of the label on a surface with a picture (an image or
    /// an icon) or with no picture. See [`SurfaceContent::has_picture`].
    pub fn position_for(&self, has_picture: bool) -> LabelPosition {
        automatic_position(self.position, has_picture)
    }
}

/// The position that shows when `position` is `None`, the automatic
/// position.
fn automatic_position(position: Option<LabelPosition>, has_picture: bool) -> LabelPosition {
    position.unwrap_or(if has_picture {
        LabelPosition::Bottom
    } else {
        LabelPosition::Middle
    })
}

impl LabelStyle {
    /// The position of the label on a surface with a picture (an image or
    /// an icon) or with no picture. This uses no default title style of the
    /// profile: [`LabelStyle::resolve`] has it.
    pub fn position_for(&self, has_picture: bool) -> LabelPosition {
        self.resolve(&LabelStyle::default())
            .position_for(has_picture)
    }

    /// The style with a value for each field. An own value wins, then the
    /// value of `profile_default`, then the engine default. A field of
    /// `profile_default` that is `None` gives the engine default, and an own
    /// [`Auto::Auto`] gives the automatic value also when the profile default
    /// has a value.
    pub fn resolve(&self, profile_default: &LabelStyle) -> ResolvedLabelStyle {
        ResolvedLabelStyle {
            font: own_auto(self.font.as_ref(), profile_default.font.as_ref()),
            position: own_auto(self.position.as_ref(), profile_default.position.as_ref()),
            size: own_value(self.size.as_ref(), profile_default.size.as_ref())
                .unwrap_or(DEFAULT_LABEL_SIZE),
            color: own_auto(self.color.as_ref(), profile_default.color.as_ref()),
            bold: own_value(self.bold.as_ref(), profile_default.bold.as_ref())
                .unwrap_or(DEFAULT_LABEL_BOLD),
            italic: own_value(self.italic.as_ref(), profile_default.italic.as_ref())
                .unwrap_or(false),
            underline: own_value(self.underline.as_ref(), profile_default.underline.as_ref())
                .unwrap_or(false),
        }
    }

    /// Apply `change`. A size goes into the range from [`MIN_LABEL_SIZE`] to
    /// [`MAX_LABEL_SIZE`]. `None` makes the part inherit. An empty family
    /// name is no family, and the reserved word "system" is the automatic
    /// font: a change cannot make a profile file that does not load again
    /// (decision 0043, section 2).
    pub fn apply(&mut self, change: LabelChange) {
        match change {
            LabelChange::Font(font) => self.font = normalize_font(font),
            LabelChange::Position(position) => self.position = position,
            LabelChange::Size(size) => {
                self.size = size.map(|size| size.clamp(MIN_LABEL_SIZE, MAX_LABEL_SIZE))
            }
            LabelChange::Color(color) => self.color = color,
            LabelChange::Bold(bold) => self.bold = bold,
            LabelChange::Italic(italic) => self.italic = italic,
            LabelChange::Underline(underline) => self.underline = underline,
        }
    }
}

/// The font of a change, without an empty family name and without the
/// reserved word as a family name. The profile file refuses both, thus an
/// apply cannot make a file that does not load again (decision 0043).
fn normalize_font(font: Option<Auto<String>>) -> Option<Auto<String>> {
    match font {
        None => None,
        Some(Auto::Auto) => Some(Auto::Auto),
        Some(Auto::Value(name)) => {
            let name = name.trim();
            if name.is_empty() {
                None
            } else if name == SYSTEM_FONT_WORD {
                Some(Auto::Auto)
            } else {
                Some(Auto::Value(name.to_string()))
            }
        }
    }
}

/// The value of one field: the own value, or else the value of the profile
/// default.
fn own_value<T: Clone>(own: Option<&T>, default: Option<&T>) -> Option<T> {
    own.or(default).cloned()
}

/// The value of one field that has an automatic value. An own automatic value
/// wins over a value of the profile default.
fn own_auto<T: Clone>(own: Option<&Auto<T>>, default: Option<&Auto<T>>) -> Option<T> {
    match (own, default) {
        (Some(Auto::Auto), _) => None,
        (Some(Auto::Value(value)), _) => Some(value.clone()),
        (None, Some(Auto::Value(value))) => Some(value.clone()),
        (None, _) => None,
    }
}

/// A change of one part of a [`LabelStyle`]. `None` makes the part inherit
/// the default title style of the profile. [`Auto::Auto`] makes it the
/// automatic value of the surface.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LabelChange {
    /// [`Auto::Auto`] is the system font.
    Font(Option<Auto<String>>),
    /// [`Auto::Auto`] is the automatic position.
    Position(Option<Auto<LabelPosition>>),
    Size(Option<u8>),
    /// [`Auto::Auto`] is the automatic colour.
    Color(Option<Auto<Color>>),
    Bold(Option<bool>),
    Italic(Option<bool>),
    Underline(Option<bool>),
}

/// What the user put on one surface. Each part is optional.
///
/// A new part of the content needs a default that is "no change" to the
/// look, so that [`SurfaceContent::is_empty`] and the file format can find
/// it. The undo history copies the content, thus it covers a new part with
/// no more work (decision 0030).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceContent {
    /// The text on the surface.
    pub label: Option<String>,
    /// How the label shows.
    pub label_style: LabelStyle,
    /// The colour behind the image and the label.
    pub color: Option<Color>,
    /// The image, as a path relative to the profile directory. The store
    /// makes this path: see [`crate::store::ProfileStore::import_image`].
    pub image: Option<String>,
    /// An icon of the built-in set (see [`crate::icons`]). A surface has an
    /// image or an icon, not both: the commands remove the other one.
    pub icon: Option<SurfaceIcon>,
    /// The crop of the image: the zoom and the pan (decision 0043). `None`
    /// is the fit. The crop goes away with the image, and a face with no
    /// image has no crop. The undo history covers it with the content.
    pub image_crop: Option<ImageCrop>,
    /// The replacement face for On. Its own `on` is always `None`.
    pub on: Option<Box<SurfaceContent>>,
}

/// An icon on a surface: a name of the built-in icon set and a colour
/// (decision 0033).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SurfaceIcon {
    /// The name of the icon in [`crate::icons::icon_set`], for example
    /// `globe`. A name that the set does not have stays in the profile, and
    /// the surface shows no icon.
    pub name: String,
    /// `None`: black or white, the colour that shows best on the fill. See
    /// [`crate::render::icon_color`].
    pub color: Option<Color>,
}

impl SurfaceIcon {
    /// The icon `name` with the automatic colour.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: None,
        }
    }

    /// True if `name` can be an icon name in the profile file: small
    /// letters, digits, and hyphens.
    pub fn is_name(name: &str) -> bool {
        !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    }
}

impl SurfaceContent {
    /// True if the content has no part.
    pub fn is_empty(&self) -> bool {
        self.label.is_none()
            && self.label_style == LabelStyle::default()
            && self.color.is_none()
            && self.image.is_none()
            && self.icon.is_none()
            && self.image_crop.is_none()
            && self.on.is_none()
    }

    /// Get the requested face, using the base face if On has no replacement.
    pub fn face(&self, on: bool) -> &Self {
        if on {
            self.on.as_deref().unwrap_or(self)
        } else {
            self
        }
    }

    /// Get the requested face. The first On edit starts with a copy of Off.
    pub fn face_mut(&mut self, on: bool) -> &mut Self {
        if !on {
            return self;
        }
        if self.on.is_none() {
            self.on = Some(Box::new(self.clone()));
        }
        self.on.as_deref_mut().expect("On face exists")
    }

    pub(crate) fn image_paths(&self) -> impl Iterator<Item = &str> {
        self.image
            .as_deref()
            .into_iter()
            .chain(self.on.as_deref().and_then(|face| face.image.as_deref()))
    }

    fn normalize(&mut self) {
        if self.label.as_deref() == Some("") {
            self.label = None;
        }
        // The fit is the same picture as no crop, and a face with no image
        // has no crop (decision 0043).
        if self.image.is_none() || self.image_crop.is_some_and(|crop| crop.is_fit()) {
            self.image_crop = None;
        }
        if let Some(on) = self.on.as_deref_mut() {
            on.on = None;
            on.normalize();
        }
    }

    /// True if the content shows a picture: an image, or an icon of the
    /// built-in set. The label then has the bottom as its automatic
    /// position (see [`LabelStyle::position_for`]).
    pub fn has_picture(&self) -> bool {
        self.image.is_some() || self.known_icon().is_some()
    }

    /// The icon of the built-in set that the surface shows: none if the
    /// surface has an image, or if the set does not have the name.
    pub fn known_icon(&self) -> Option<&'static crate::icons::Icon> {
        if self.image.is_some() {
            return None;
        }
        crate::icons::icon_set().get(&self.icon.as_ref()?.name)
    }
}

/// How a background image that does not have the proportion of the display
/// goes on the display. The reasons are in
/// `docs/decisions/0032-page-background.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BackgroundFit {
    /// The image covers all of the display, with its proportion, at the
    /// centre. The parts of the image out of the display do not show. This
    /// is the default.
    #[default]
    Fill,
    /// All of the image is on the display, with its proportion, at the
    /// centre. Black fills the rest of the display.
    Fit,
}

impl BackgroundFit {
    /// All fits, in the order of the window.
    pub const ALL: [BackgroundFit; 2] = [Self::Fill, Self::Fit];

    /// The text form in the profile file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Fit => "fit",
        }
    }

    /// Read the text form. See [`BackgroundFit::key`].
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }
}

/// The background of a page: the image on the full display, behind the key
/// faces and the bar panels. A surface with no fill colour shows the part of
/// the background at its position (decision 0032).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Background {
    /// One colour on all of the display.
    Color(Color),
    /// An image, as a path relative to the profile directory. The store
    /// makes this path: see
    /// [`crate::store::ProfileStore::import_background`].
    Image { image: String, fit: BackgroundFit },
}

impl Background {
    /// The image path, if the background is an image.
    pub fn image(&self) -> Option<&str> {
        match self {
            Background::Image { image, .. } => Some(image),
            Background::Color(_) => None,
        }
    }
}

/// One full set of bindings and surface content for a device.
///
/// Two pages are equal if their content is equal. The identity is not part of
/// the content: compare [`Page::id`] where the identity matters.
#[derive(Debug, Clone)]
pub struct Page {
    /// The identity of the page. It stays with the page when the page moves,
    /// and the profile file keeps it. The pages of one profile have different
    /// identities (decision 0040).
    id: PageId,
    pub name: String,
    /// The background of the page. `None`: the page has no background, and
    /// the display is black behind the surfaces.
    pub background: Option<Background>,
    /// The content of each surface that has content. A surface with no
    /// content is not in the map: use [`Page::update`] to change the map.
    surfaces: BTreeMap<SurfaceId, SurfaceContent>,
    bindings: BTreeMap<(ControlId, GestureKind), Action>,
    /// True on the first page of the profile, if the user selected one: see
    /// [`Profile::first_page`]. The mark is on the page, so that it moves
    /// with the page, and the undo history covers it (decision 0030).
    first: bool,
}

impl PartialEq for Page {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.background == other.background
            && self.surfaces == other.surfaces
            && self.bindings == other.bindings
            && self.first == other.first
    }
}

impl Eq for Page {}

impl Page {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: PageId::random(),
            name: name.into(),
            background: None,
            surfaces: BTreeMap::new(),
            bindings: BTreeMap::new(),
            first: false,
        }
    }

    /// The identity of the page.
    pub fn id(&self) -> PageId {
        self.id
    }

    /// The same page with the identity `id`. A profile file uses this.
    pub(crate) fn with_id(mut self, id: PageId) -> Self {
        self.id = id;
        self
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

    /// Change the image path of each surface and of the background with
    /// `map`. The second argument of `map` tells the use of the image.
    /// `None` from `map` removes the image, or the background. An export
    /// and an import of a profile file use this: see [`crate::share`].
    pub(crate) fn map_images(&mut self, mut map: impl FnMut(&str, ImageUse) -> Option<String>) {
        let ids: Vec<SurfaceId> = self.surfaces.keys().copied().collect();
        for id in ids {
            self.update(id, |content| {
                content.image = content
                    .image
                    .as_deref()
                    .and_then(|p| map(p, ImageUse::Surface));
                if let Some(on) = content.on.as_deref_mut() {
                    on.image = on.image.as_deref().and_then(|p| map(p, ImageUse::Surface));
                }
            });
        }
        if let Some(Background::Image { image, fit }) = &self.background {
            let fit = *fit;
            self.background =
                map(image, ImageUse::Background).map(|image| Background::Image { image, fit });
        }
    }

    /// Change the content of `surface` with `change`. An empty label becomes
    /// no label. Return true if the content is different after the change.
    pub fn update(&mut self, surface: SurfaceId, change: impl FnOnce(&mut SurfaceContent)) -> bool {
        let before = self.content(surface);
        let mut after = before.clone();
        change(&mut after);
        after.normalize();
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

    /// Keep only key faces and key bindings. A folder is a page of keys
    /// (decision 0036). The background and the first-page mark go away.
    pub(crate) fn keep_keys_only(&mut self) {
        self.background = None;
        self.first = false;
        self.surfaces
            .retain(|id, _| id.kind == SurfaceKind::KeyFace);
        self.bindings
            .retain(|(control, _), _| control.kind == ControlKind::Key);
    }
}

/// The use of an image file of the profile. A surface image and a
/// background have different sizes: see [`crate::store`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ImageUse {
    Surface,
    Background,
}

/// One application that a profile is for (decision 0035).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationLink {
    /// The bundle identifier, for example `us.zoom.xos`. The engine keeps
    /// the text as the shell gave it.
    pub bundle_id: String,
    /// The name of the application for the user at the time of the link.
    pub name: String,
}

/// A random identity with the text form of 16 small hexadecimal digits.
/// Zero is not an identity. The macro makes the identity of a folder (decision
/// 0036) and the identity of a page (decision 0040).
macro_rules! hex_identity {
    ($(#[$meta:meta])* $name:ident, $error:ident, $what:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            /// A new identity: a random number that is not 0. It uses
            /// [`RandomState`] and needs no new crate.
            pub fn random() -> Self {
                loop {
                    let value = RandomState::new().hash_one(0u8);
                    if let Some(id) = Self::from_raw(value) {
                        return id;
                    }
                }
            }

            /// The identity of `value`. Zero is not an identity.
            pub const fn from_raw(value: u64) -> Option<Self> {
                if value == 0 {
                    None
                } else {
                    Some(Self(value))
                }
            }
        }

        #[doc = concat!("The text is not the text form of ", $what, " identity.")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $error;

        impl fmt::Display for $error {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{} identity is 16 hexadecimal digits", $what)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:016x}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = $error;

            /// Read 16 hexadecimal digits with small letters. Zero is not an
            /// identity.
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                if text.len() != 16
                    || !text
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err($error);
                }
                let value = u64::from_str_radix(text, 16).map_err(|_| $error)?;
                Self::from_raw(value).ok_or($error)
            }
        }
    };
}

hex_identity!(
    /// The identity of a folder. The text form is 16 hexadecimal digits with
    /// small letters. Zero is not an identity (decision 0036).
    FolderId,
    ParseFolderIdError,
    "a folder"
);

hex_identity!(
    /// The identity of a page. It stays with the page when the page moves, and
    /// the profile file keeps it (decision 0040). The text form is 16
    /// hexadecimal digits with small letters. Zero is not an identity.
    PageId,
    ParsePageIdError,
    "a page"
);

impl PageId {
    /// The identity of the page at `index` in a profile file that has no
    /// identities (schema version 5 or lower). The identity depends on the
    /// place only, thus a profile that no one saved has the same identities
    /// each time that it loads.
    pub(crate) fn of_old_page(index: usize) -> Self {
        Self::from_raw(index as u64 + 1).expect("an index plus one is not 0")
    }
}

/// A folder: a page of keys outside the page cycle (decision 0036). The name
/// of the page is the name of the folder. The page has no background, is
/// never the first page, and has only key faces and keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub id: FolderId,
    pub page: Page,
}

/// One folder in [`crate::Event::Pages`]: its identity and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderEntry {
    pub id: FolderId,
    pub name: String,
}

/// A named set of pages for one device model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// The name for the user. The identity of a profile is the name of its
    /// directory: see [`crate::profiles`].
    pub name: String,
    /// The model identifier of the descriptor, for example `n4-pro`.
    pub model: String,
    /// The colour of the profile for the user, for example the colour of
    /// its icon in the sidebar. `None`: the profile has no colour.
    pub color: Option<Color>,
    /// The applications that this profile is for (decision 0035).
    pub applications: Vec<ApplicationLink>,
    /// The pages. A profile has one page or more: see [`Profile::new`].
    pages: Vec<Page>,
    /// The folders, in the order of the sidebar. A folder is not a page of
    /// the page cycle (decision 0036).
    folders: Vec<Folder>,
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
        // The pages of a profile have different identities, also if the caller
        // made them as copies of one page.
        let mut seen = HashSet::new();
        for page in &mut pages {
            while !seen.insert(page.id) {
                page.id = PageId::random();
            }
        }
        Self {
            name: name.into(),
            model: model.into(),
            color: None,
            applications: Vec::new(),
            pages,
            folders: Vec::new(),
        }
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// One page. Page numbers start at 0.
    pub fn page(&self, page: usize) -> Option<&Page> {
        self.pages.get(page)
    }

    /// The number of the page with the identity `id`.
    pub fn page_number(&self, id: PageId) -> Option<usize> {
        self.pages.iter().position(|page| page.id == id)
    }

    /// Get one page for a change. Page numbers start at 0.
    pub fn page_mut(&mut self, page: usize) -> Option<&mut Page> {
        self.pages.get_mut(page)
    }

    /// Put `page` after the last page. Return its number. The new page is
    /// not the first page of the profile, also if it is a copy of the first
    /// page.
    pub fn add_page(&mut self, mut page: Page) -> usize {
        page.first = false;
        // A copy of a page has a new identity.
        while self.pages.iter().any(|p| p.id == page.id) {
            page.id = PageId::random();
        }
        self.pages.push(page);
        self.pages.len() - 1
    }

    /// The number of the page that the device shows when the profile
    /// becomes the active profile. Page numbers start at 0. If the user did
    /// not select a page, or if the user removed it, it is the page at the
    /// start of the list.
    pub fn first_page(&self) -> usize {
        self.pages.iter().position(|p| p.first).unwrap_or(0)
    }

    /// Make `page` the first page: see [`Self::first_page`]. Return false if
    /// it is not a page, or if it is the first page now.
    pub fn set_first_page(&mut self, page: usize) -> bool {
        if page >= self.pages.len() || page == self.first_page() {
            return false;
        }
        for (number, each) in self.pages.iter_mut().enumerate() {
            each.first = number == page;
        }
        true
    }

    /// True if the user selected a first page, also the page at the start
    /// of the list. The profile file writes the first page only then.
    pub(crate) fn has_first_page(&self) -> bool {
        self.pages.iter().any(|p| p.first)
    }

    /// Mark `page` as the first page, also if it is the page at the start of
    /// the list. The load of a profile file uses this, so that a load and a
    /// save give the same file.
    pub(crate) fn mark_first_page(&mut self, page: usize) {
        for (number, each) in self.pages.iter_mut().enumerate() {
            each.first = number == page;
        }
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

    /// The folders, in the order of the sidebar.
    pub fn folders(&self) -> &[Folder] {
        &self.folders
    }

    /// Call `visit` for each action on a page and for each action in a
    /// folder. The export uses this to replace a home path in a path
    /// setting. It does not change the profile that the engine keeps.
    pub(crate) fn for_each_action(&mut self, mut visit: impl FnMut(&mut Action)) {
        for page in &mut self.pages {
            for action in page.bindings.values_mut() {
                visit(action);
            }
        }
        for folder in &mut self.folders {
            for action in folder.page.bindings.values_mut() {
                visit(action);
            }
        }
    }

    /// The folder with the identity `id`.
    pub fn folder(&self, id: FolderId) -> Option<&Folder> {
        self.folders.iter().find(|folder| folder.id == id)
    }

    /// The folder with the identity `id`, for a change.
    pub fn folder_mut(&mut self, id: FolderId) -> Option<&mut Folder> {
        self.folders.iter_mut().find(|folder| folder.id == id)
    }

    /// Put `folder` after the last folder. Return false if the profile
    /// already has that identity.
    pub fn add_folder(&mut self, mut folder: Folder) -> bool {
        if self.folder(folder.id).is_some() {
            return false;
        }
        folder.page.keep_keys_only();
        self.folders.push(folder);
        true
    }

    /// Move the folder `id` to the position `to`. Return false if the
    /// profile has no such folder, or if `to` is not a position.
    pub fn move_folder(&mut self, id: FolderId, to: usize) -> bool {
        let Some(from) = self.folders.iter().position(|folder| folder.id == id) else {
            return false;
        };
        if to >= self.folders.len() {
            return false;
        }
        let folder = self.folders.remove(from);
        self.folders.insert(to, folder);
        true
    }

    /// Remove the folder `id`. Return false if the profile has no such folder.
    pub fn remove_folder(&mut self, id: FolderId) -> bool {
        let Some(index) = self.folders.iter().position(|folder| folder.id == id) else {
            return false;
        };
        self.folders.remove(index);
        true
    }

    /// A name for a new folder: `Folder N`, where N is the number of folders
    /// after the add. If a folder has that name, N increases.
    pub fn new_folder_name(&self) -> String {
        (self.folders.len() + 1..)
            .map(|n| format!("Folder {n}"))
            .find(|name| self.folders.iter().all(|folder| &folder.page.name != name))
            .expect("a name that no folder has")
    }

    /// Put `folders` in place of all folders, and return the folders that
    /// were there. The undo and the redo of the engine use this.
    pub fn replace_folders(&mut self, folders: Vec<Folder>) -> Vec<Folder> {
        std::mem::replace(&mut self.folders, folders)
    }

    /// Change the image path of each surface and of each background of
    /// each page, and of each key face of each folder, with `map`. See
    /// [`Page::map_images`].
    pub(crate) fn map_images(&mut self, mut map: impl FnMut(&str, ImageUse) -> Option<String>) {
        for page in &mut self.pages {
            page.map_images(&mut map);
        }
        for folder in &mut self.folders {
            folder.page.map_images(&mut map);
        }
    }

    /// Each image path that a surface or a background of the profile uses.
    pub fn image_paths(&self) -> impl Iterator<Item = &str> {
        self.image_uses().map(|(path, _)| path)
    }

    /// Each image path that the profile uses, with its use: the images of
    /// the surfaces, then the backgrounds of the pages, then the images of
    /// the folder key faces.
    pub fn image_uses(&self) -> impl Iterator<Item = (&str, ImageUse)> {
        image_uses_of(&self.pages, &self.folders)
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

/// Each image path that `pages` and `folders` use, with its use: the images
/// of the surfaces, then the backgrounds of the pages, then the images of
/// the folder key faces. See [`Profile::image_uses`]. The engine also uses
/// it for the copies of the undo history.
pub(crate) fn image_uses_of<'a>(
    pages: &'a [Page],
    folders: &'a [Folder],
) -> impl Iterator<Item = (&'a str, ImageUse)> {
    let surfaces = pages
        .iter()
        .flat_map(|p| p.surfaces.values())
        .flat_map(SurfaceContent::image_paths)
        .map(|p| (p, ImageUse::Surface));
    let backgrounds = pages
        .iter()
        .filter_map(|p| p.background.as_ref().and_then(Background::image))
        .map(|p| (p, ImageUse::Background));
    let folders = folders
        .iter()
        .flat_map(|folder| folder.page.surfaces())
        .flat_map(|(_, content)| content.image_paths())
        .map(|p| (p, ImageUse::Surface));
    surfaces.chain(backgrounds).chain(folders)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_3: SurfaceId = SurfaceId::new(SurfaceKind::KeyFace, 3);

    #[test]
    fn an_empty_on_face_is_a_replacement_not_an_absent_face() {
        let mut page = Page::new("Main");
        page.update(KEY_3, |c| {
            c.label = Some("Off".into());
            *c.face_mut(true) = SurfaceContent {
                label: Some(String::new()),
                on: Some(Box::new(SurfaceContent::default())),
                ..SurfaceContent::default()
            };
        });
        let content = page.content(KEY_3);
        assert_eq!(content.face(true), &SurfaceContent::default());
        assert_eq!(content.face(false).label.as_deref(), Some("Off"));
        assert!(!SurfaceContent {
            on: content.on,
            ..SurfaceContent::default()
        }
        .is_empty());
    }

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
    fn a_label_style_is_content_and_its_default_is_the_old_look() {
        let mut page = Page::new("Page 1");
        assert!(page.update(KEY_3, |c| c
            .label_style
            .apply(LabelChange::Position(Some(Auto::Value(LabelPosition::Top))))));
        assert_eq!(page.surfaces().count(), 1, "a style alone is content");
        assert!(page.update(KEY_3, |c| c.label_style.apply(LabelChange::Position(None))));
        assert_eq!(page.surfaces().count(), 0);

        let style = LabelStyle::default();
        assert_eq!(style.position_for(false), LabelPosition::Middle);
        assert_eq!(style.position_for(true), LabelPosition::Bottom);
        let resolved = style.resolve(&LabelStyle::default());
        assert_eq!(
            (resolved.size, resolved.color, resolved.bold),
            (DEFAULT_LABEL_SIZE, None, true)
        );
        assert!(!resolved.italic);
        assert!(!resolved.underline);
        assert_eq!(resolved.font, None);
        let mut style = LabelStyle::default();
        style.apply(LabelChange::Size(Some(4)));
        assert_eq!(style.size, Some(MIN_LABEL_SIZE));
        style.apply(LabelChange::Size(Some(200)));
        assert_eq!(style.size, Some(MAX_LABEL_SIZE));
        style.apply(LabelChange::Size(None));
        assert_eq!(style.size, None);
        style.apply(LabelChange::Position(Some(Auto::Value(
            LabelPosition::Hidden,
        ))));
        assert_eq!(style.position_for(true), LabelPosition::Hidden);
        for position in LabelPosition::ALL {
            assert_eq!(LabelPosition::from_key(position.key()), Some(position));
        }
        assert_eq!(LabelPosition::from_key("centre"), None);
        assert!(LABEL_SIZES
            .iter()
            .all(|s| (MIN_LABEL_SIZE..=MAX_LABEL_SIZE).contains(s)));
        assert!(LABEL_SIZES.contains(&DEFAULT_LABEL_SIZE));
    }

    #[test]
    fn a_font_change_cannot_make_a_family_that_the_file_refuses() {
        let mut style = LabelStyle::default();
        for (font, expected) in [
            (Some(Auto::Value(String::new())), None),
            (Some(Auto::Value("   ".into())), None),
            (Some(Auto::Value(" system ".into())), Some(Auto::Auto)),
            (
                Some(Auto::Value("Avenir Next".into())),
                Some(Auto::Value("Avenir Next".into())),
            ),
            (None, None),
        ] {
            style.apply(LabelChange::Font(font));
            assert_eq!(style.font, expected);
            assert!(style.font.as_ref().is_none_or(|font| match font {
                Auto::Auto => true,
                Auto::Value(name) => !name.trim().is_empty() && name != SYSTEM_FONT_WORD,
            }));
        }
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
    fn the_first_page_moves_with_its_page_and_a_copy_is_not_the_first_page() {
        let mut profile = Profile::new("P", vec![Page::new("A"), Page::new("B"), Page::new("C")]);
        assert_eq!(
            profile.first_page(),
            0,
            "with no choice, the page at the start"
        );
        assert!(!profile.set_first_page(0), "no change");
        assert!(!profile.set_first_page(3), "not a page");
        assert!(profile.set_first_page(2));
        assert!(!profile.set_first_page(2));
        assert!(profile.move_page(2, 0));
        assert_eq!(profile.first_page(), 0);
        assert_eq!(profile.pages()[0].name, "C");
        let copy = profile.pages()[0].clone();
        assert_eq!(profile.add_page(copy), 3);
        assert_eq!(profile.first_page(), 0, "the copy is not the first page");
        assert!(profile.set_first_page(1));
        assert!(profile.remove_page(1));
        assert_eq!(profile.first_page(), 0, "the first page is gone");
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
