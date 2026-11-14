//! The location: the active page and the open folders.
//!
//! The keys come from the last open folder. All other controls come from
//! the page under the folders. This is the folder rule of decision 0036,
//! section 3. The engine asks this module for each question about the
//! location.
//!
//! A test builds a profile and a location. It needs no engine and no device.

use crate::action::{Action, BindingId, GestureKind};
use crate::profile::{FolderId, Page, Profile, SurfaceContent};
use opencraft_core::{ControlId, ControlKind, SurfaceId, SurfaceKind};

/// The active page and the open folders (decision 0036).
///
/// `page` is the page under the folders. Page numbers start at 0. `open` is
/// the open folders, from the first to the last. The device shows the last
/// folder. The list is empty when no folder is open.
///
/// `page` is a page of the profile. The engine keeps this true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub page: usize,
    pub open: Vec<FolderId>,
}

impl Default for Location {
    /// The first page, with no open folder.
    fn default() -> Self {
        Self {
            page: 0,
            open: Vec::new(),
        }
    }
}

/// The page or the folder that holds a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holder {
    /// A page of the page cycle. The number starts at 0.
    Page(usize),
    /// A folder.
    Folder(FolderId),
}

impl Location {
    /// Which page or folder holds `control`.
    ///
    /// A key is in the last open folder when that folder exists. Every
    /// other control is on the page under the folders. A key is also on
    /// that page when no folder is open, or when the last open folder does
    /// not exist.
    pub fn holder(&self, profile: &Profile, control: ControlId) -> Holder {
        self.holder_for(profile, control.kind == ControlKind::Key)
    }

    /// The binding of `gesture` on `control` at this location.
    ///
    /// `None` when the gesture has no binding. The page number of the
    /// identity is the page under the folders. The folder of the identity
    /// is the last open folder for a key, also when that folder does not
    /// exist, and `None` for every other control.
    pub fn binding<'a>(
        &self,
        profile: &'a Profile,
        control: ControlId,
        gesture: GestureKind,
    ) -> Option<(BindingId, &'a Action)> {
        let action = self.page_of(profile, control).binding(control, gesture)?;
        Some((self.binding_id(control, gesture), action))
    }

    /// The bindings at this location.
    ///
    /// The keys come from the last open folder. The other controls come
    /// from the page under the folders. When no folder is open, every
    /// binding comes from that page. When the last open folder does not
    /// exist, the location has no key bindings: the keys do not come from
    /// the page under the folders.
    pub fn bindings(&self, profile: &Profile) -> Vec<(BindingId, Action)> {
        let page = self.page;
        let source = self.cycle_page(profile);
        let Some(id) = self.last_folder() else {
            return listed(page, source, None, |_| true);
        };
        let mut out = profile
            .folder(id)
            .map(|folder| {
                listed(page, &folder.page, Some(id), |kind| {
                    kind == ControlKind::Key
                })
            })
            .unwrap_or_default();
        out.extend(listed(page, source, None, |kind| kind != ControlKind::Key));
        out
    }

    /// The content of `surface` at this location.
    ///
    /// A key face comes from the last open folder when that folder exists.
    /// It comes from the page under the folders when no folder is open, or
    /// when that folder does not exist. Every other surface comes from the
    /// page under the folders.
    pub fn content(&self, profile: &Profile, surface: SurfaceId) -> SurfaceContent {
        let holder = self.holder_for(profile, surface.kind == SurfaceKind::KeyFace);
        self.page_of_holder(profile, holder).content(surface)
    }

    /// The open folders that the engine can restore from `open`.
    ///
    /// The list stops at the first folder that `profile` does not have,
    /// and at a folder that is already in the list. The page number is a
    /// separate value: this function does not change it.
    pub fn restored_open(profile: &Profile, open: &[FolderId]) -> Vec<FolderId> {
        let mut out = Vec::new();
        for id in open {
            if profile.folder(*id).is_none() || out.contains(id) {
                break;
            }
            out.push(*id);
        }
        out
    }

    /// The page that holds `control`, for a read.
    pub(crate) fn page_of<'a>(&self, profile: &'a Profile, control: ControlId) -> &'a Page {
        self.page_of_holder(profile, self.holder(profile, control))
    }

    /// The page that a change of `control` writes.
    ///
    /// A key of a missing folder gives `None`: the change does not go to
    /// the page under the folders. A read of that key does: see [`Self::holder`].
    pub(crate) fn page_for_change<'a>(
        &self,
        profile: &'a mut Profile,
        control: ControlId,
    ) -> Option<&'a mut Page> {
        self.page_for(profile, control.kind == ControlKind::Key)
    }

    /// The page that a change of `surface` writes. See [`Self::page_for_change`].
    pub(crate) fn surface_for_change<'a>(
        &self,
        profile: &'a mut Profile,
        surface: SurfaceId,
    ) -> Option<&'a mut Page> {
        self.page_for(profile, surface.kind == SurfaceKind::KeyFace)
    }

    /// The identity of the binding of `gesture` on `control` at this location.
    /// See [`Self::binding`].
    pub(crate) fn binding_id(&self, control: ControlId, gesture: GestureKind) -> BindingId {
        BindingId {
            page: self.page,
            control,
            gesture,
            folder: self.folder_of_control(control),
        }
    }

    /// True when `binding` names a control of this location.
    ///
    /// The page number is the page under the folders. The folder is the
    /// last open folder for a key, and `None` for every other control.
    /// This does not check that the gesture has an action.
    pub(crate) fn shows(&self, binding: BindingId) -> bool {
        binding.page == self.page && binding.folder == self.folder_of_control(binding.control)
    }

    /// The folder of a key face, for a label step. `None` for each other
    /// surface, and when no folder is open.
    ///
    /// This does not check that the folder exists.
    pub(crate) fn folder_of_surface(&self, surface: SurfaceId) -> Option<FolderId> {
        if surface.kind == SurfaceKind::KeyFace {
            self.last_folder()
        } else {
            None
        }
    }

    fn holder_for(&self, profile: &Profile, from_folder: bool) -> Holder {
        match self.existing_folder(profile) {
            Some(id) if from_folder => Holder::Folder(id),
            _ => Holder::Page(self.page),
        }
    }

    fn page_of_holder<'a>(&self, profile: &'a Profile, holder: Holder) -> &'a Page {
        match holder {
            Holder::Page(number) => &profile.pages()[number],
            Holder::Folder(id) => {
                &profile
                    .folder(id)
                    .expect("the holder names a folder of the profile")
                    .page
            }
        }
    }

    fn page_for<'a>(&self, profile: &'a mut Profile, from_folder: bool) -> Option<&'a mut Page> {
        if let Some(id) = self.last_folder().filter(|_| from_folder) {
            return profile.folder_mut(id).map(|folder| &mut folder.page);
        }
        profile.page_mut(self.page)
    }

    fn cycle_page<'a>(&self, profile: &'a Profile) -> &'a Page {
        &profile.pages()[self.page]
    }

    fn last_folder(&self) -> Option<FolderId> {
        self.open.last().copied()
    }

    fn existing_folder(&self, profile: &Profile) -> Option<FolderId> {
        self.last_folder()
            .filter(|id| profile.folder(*id).is_some())
    }

    fn folder_of_control(&self, control: ControlId) -> Option<FolderId> {
        if control.kind == ControlKind::Key {
            self.last_folder()
        } else {
            None
        }
    }
}

fn listed(
    page: usize,
    source: &Page,
    folder: Option<FolderId>,
    keep: impl Fn(ControlKind) -> bool,
) -> Vec<(BindingId, Action)> {
    source
        .bindings()
        .filter(|((control, _), _)| keep(control.kind))
        .map(|((control, gesture), action)| {
            (
                BindingId {
                    page,
                    control,
                    gesture,
                    folder,
                },
                action.clone(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::Folder;

    fn action(kind: &str) -> Action {
        Action {
            kind: kind.to_string(),
            settings: Default::default(),
        }
    }

    fn key(number: u8) -> ControlId {
        ControlId::new(ControlKind::Key, number)
    }

    fn knob(number: u8) -> ControlId {
        ControlId::new(ControlKind::Knob, number)
    }

    fn bar_control() -> ControlId {
        ControlId::new(ControlKind::BarPanel, 1)
    }

    fn touch() -> ControlId {
        ControlId::new(ControlKind::TouchBar, 1)
    }

    fn face(number: u8) -> SurfaceId {
        SurfaceId::new(SurfaceKind::KeyFace, number)
    }

    fn bar_surface() -> SurfaceId {
        SurfaceId::new(SurfaceKind::BarPanel, 1)
    }

    fn background() -> SurfaceId {
        SurfaceId::new(SurfaceKind::Background, 0)
    }

    fn label(page: &mut Page, surface: SurfaceId, text: &str) {
        page.update(surface, |content| content.label = Some(text.to_string()));
    }

    /// A profile with two pages and two folders. The folder pages also get
    /// a knob binding after the add, so a test can show that the location
    /// ignores it.
    fn sample() -> (Profile, FolderId, FolderId) {
        let mut first = Page::new("Page 1");
        first.set_binding(key(1), GestureKind::Press, Some(action("page-key")));
        first.set_binding(knob(1), GestureKind::TurnLeft, Some(action("page-knob")));
        first.set_binding(bar_control(), GestureKind::Tap, Some(action("page-bar")));
        first.set_binding(touch(), GestureKind::SwipeLeft, Some(action("page-swipe")));
        label(&mut first, face(1), "Page face");
        label(&mut first, bar_surface(), "Page bar");
        label(&mut first, background(), "Page ground");

        let mut second = Page::new("Page 2");
        second.set_binding(knob(1), GestureKind::TurnLeft, Some(action("page-2-knob")));
        label(&mut second, face(1), "Page 2 face");

        let mut folder_a = Page::new("Folder A");
        folder_a.set_binding(key(1), GestureKind::Press, Some(action("folder-a")));
        label(&mut folder_a, face(1), "Face A");

        let mut folder_b = Page::new("Folder B");
        folder_b.set_binding(key(1), GestureKind::Press, Some(action("folder-b")));
        folder_b.set_binding(key(2), GestureKind::Press, Some(action("folder-b-2")));
        label(&mut folder_b, face(1), "Face B");

        let a = FolderId::from_raw(1).expect("1 is a folder identity");
        let b = FolderId::from_raw(2).expect("2 is a folder identity");
        let mut profile = Profile::new("Streaming", vec![first, second]);
        assert!(profile.add_folder(Folder {
            id: a,
            page: folder_a,
        }));
        assert!(profile.add_folder(Folder {
            id: b,
            page: folder_b,
        }));
        profile.folder_mut(b).expect("folder B").page.set_binding(
            knob(1),
            GestureKind::TurnLeft,
            Some(action("folder-knob")),
        );
        (profile, a, b)
    }

    fn kind_of(
        profile: &Profile,
        location: &Location,
        control: ControlId,
        gesture: GestureKind,
    ) -> Option<String> {
        location
            .binding(profile, control, gesture)
            .map(|(_, action)| action.kind.clone())
    }

    fn binding_rows(
        profile: &Profile,
        location: &Location,
    ) -> Vec<(ControlId, Option<FolderId>, String)> {
        location
            .bindings(profile)
            .into_iter()
            .map(|(id, action)| (id.control, id.folder, action.kind))
            .collect()
    }

    #[test]
    fn no_folder_uses_the_page_for_every_control() {
        let (profile, _, _) = sample();
        let location = Location::default();
        assert_eq!(location.page, 0);
        assert!(location.open.is_empty());

        assert_eq!(location.holder(&profile, key(1)), Holder::Page(0));
        assert_eq!(location.holder(&profile, knob(1)), Holder::Page(0));
        assert_eq!(location.holder(&profile, bar_control()), Holder::Page(0));
        assert_eq!(location.holder(&profile, touch()), Holder::Page(0));

        assert_eq!(
            kind_of(&profile, &location, key(1), GestureKind::Press).as_deref(),
            Some("page-key")
        );
        assert_eq!(
            kind_of(&profile, &location, knob(1), GestureKind::TurnLeft).as_deref(),
            Some("page-knob")
        );
        assert_eq!(location.binding_id(key(1), GestureKind::Press).folder, None);
        assert!(location.shows(location.binding_id(knob(1), GestureKind::TurnLeft)));
        assert!(!location.shows(BindingId {
            page: 1,
            control: key(1),
            gesture: GestureKind::Press,
            folder: None,
        }));

        assert_eq!(
            binding_rows(&profile, &location),
            vec![
                (key(1), None, "page-key".into()),
                (knob(1), None, "page-knob".into()),
                (bar_control(), None, "page-bar".into()),
                (touch(), None, "page-swipe".into()),
            ]
        );
        assert_eq!(
            location.content(&profile, face(1)).label.as_deref(),
            Some("Page face")
        );
        assert_eq!(
            location.content(&profile, bar_surface()).label.as_deref(),
            Some("Page bar")
        );
        assert_eq!(
            location.content(&profile, background()).label.as_deref(),
            Some("Page ground")
        );
        assert_eq!(location.folder_of_surface(face(1)), None);
        assert_eq!(location.folder_of_surface(background()), None);
    }

    #[test]
    fn the_last_open_folder_gives_the_keys_and_the_page_gives_the_other_controls() {
        let (profile, a, b) = sample();
        let location = Location {
            page: 1,
            open: vec![a, b],
        };

        assert_eq!(location.holder(&profile, key(1)), Holder::Folder(b));
        assert_eq!(location.holder(&profile, key(2)), Holder::Folder(b));
        assert_eq!(location.holder(&profile, knob(1)), Holder::Page(1));
        assert_eq!(location.holder(&profile, bar_control()), Holder::Page(1));
        assert_eq!(location.holder(&profile, touch()), Holder::Page(1));

        let (id, action) = location
            .binding(&profile, key(1), GestureKind::Press)
            .expect("folder B has the key");
        assert_eq!(action.kind, "folder-b");
        assert_eq!(id.page, 1);
        assert_eq!(id.folder, Some(b));
        assert_eq!(
            kind_of(&profile, &location, key(2), GestureKind::Press).as_deref(),
            Some("folder-b-2")
        );
        // The knob binding on the folder is ignored. The knob is page 2.
        assert_eq!(
            kind_of(&profile, &location, knob(1), GestureKind::TurnLeft).as_deref(),
            Some("page-2-knob")
        );
        assert_eq!(
            location.binding_id(knob(1), GestureKind::TurnLeft).folder,
            None
        );
        assert!(kind_of(&profile, &location, key(3), GestureKind::Press).is_none());

        assert_eq!(
            binding_rows(&profile, &location),
            vec![
                (key(1), Some(b), "folder-b".into()),
                (key(2), Some(b), "folder-b-2".into()),
                (knob(1), None, "page-2-knob".into()),
            ]
        );
        assert_eq!(
            location.content(&profile, face(1)).label.as_deref(),
            Some("Face B")
        );
        assert_eq!(
            location.content(&profile, background()).label.as_deref(),
            None
        );
        assert_eq!(location.folder_of_surface(face(1)), Some(b));
        assert_eq!(location.folder_of_surface(bar_surface()), None);
        assert_eq!(location.folder_of_surface(background()), None);

        assert!(location.shows(id));
        assert!(!location.shows(BindingId {
            folder: Some(a),
            ..id
        }));
        assert!(!location.shows(BindingId { folder: None, ..id }));
        assert!(!location.shows(BindingId {
            folder: Some(b),
            ..location.binding_id(knob(1), GestureKind::TurnLeft)
        }));

        let earlier = Location {
            page: 0,
            open: vec![a],
        };
        assert_eq!(earlier.holder(&profile, key(1)), Holder::Folder(a));
        assert_eq!(
            kind_of(&profile, &earlier, key(1), GestureKind::Press).as_deref(),
            Some("folder-a")
        );
        assert_eq!(
            earlier.content(&profile, face(1)).label.as_deref(),
            Some("Face A")
        );
        assert_eq!(
            kind_of(&profile, &earlier, knob(1), GestureKind::TurnLeft).as_deref(),
            Some("page-knob")
        );
    }

    #[test]
    fn a_change_writes_the_folder_for_a_key_and_the_page_for_a_knob() {
        let (mut profile, a, b) = sample();
        let location = Location {
            page: 0,
            open: vec![a, b],
        };
        location
            .page_for_change(&mut profile, key(1))
            .expect("the key is in folder B")
            .name = "Changed B".into();
        location
            .page_for_change(&mut profile, knob(1))
            .expect("the knob is on the page")
            .name = "Changed page".into();
        location
            .surface_for_change(&mut profile, face(1))
            .expect("the key face is in folder B")
            .update(face(1), |content| content.label = Some("New face".into()));
        location
            .surface_for_change(&mut profile, background())
            .expect("the background is on the page")
            .update(background(), |content| {
                content.label = Some("New ground".into())
            });

        assert_eq!(profile.folder(b).expect("folder B").page.name, "Changed B");
        assert_eq!(
            location.content(&profile, face(1)).label.as_deref(),
            Some("New face")
        );
        assert_eq!(profile.pages()[0].name, "Changed page");
        assert_eq!(
            location.content(&profile, background()).label.as_deref(),
            Some("New ground")
        );
        assert_eq!(profile.folder(a).expect("folder A").page.name, "Folder A");
        assert_eq!(profile.pages()[1].name, "Page 2");
    }

    #[test]
    fn a_missing_folder_keeps_the_read_of_a_key_on_the_page_and_drops_the_key_bindings() {
        let (mut profile, _, _) = sample();
        let missing = FolderId::from_raw(99).expect("99 is a folder identity");
        let location = Location {
            page: 0,
            open: vec![missing],
        };

        assert_eq!(location.holder(&profile, key(1)), Holder::Page(0));
        let (id, action) = location
            .binding(&profile, key(1), GestureKind::Press)
            .expect("the page has the key");
        assert_eq!(action.kind, "page-key");
        assert_eq!(id.folder, Some(missing));
        assert!(location.shows(id));
        assert!(location
            .bindings(&profile)
            .iter()
            .all(|(binding, _)| binding.control.kind != ControlKind::Key));
        assert_eq!(
            location.content(&profile, face(1)).label.as_deref(),
            Some("Page face")
        );
        assert!(location.page_for_change(&mut profile, key(1)).is_none());
        assert!(location.surface_for_change(&mut profile, face(1)).is_none());
        assert_eq!(profile.pages()[0].name, "Page 1");
        assert!(location
            .surface_for_change(&mut profile, background())
            .is_some());
    }

    #[test]
    fn a_restored_open_list_stops_at_a_missing_folder_and_at_a_repeat() {
        let (profile, a, b) = sample();
        let missing = FolderId::from_raw(99).expect("99 is a folder identity");
        assert_eq!(Location::restored_open(&profile, &[a, b]), vec![a, b]);
        assert_eq!(Location::restored_open(&profile, &[a, missing, b]), vec![a]);
        assert_eq!(Location::restored_open(&profile, &[missing, a]), vec![]);
        assert_eq!(Location::restored_open(&profile, &[a, b, a]), vec![a, b]);
        assert_eq!(Location::restored_open(&profile, &[a, a]), vec![a]);
        assert_eq!(Location::restored_open(&profile, &[b, a, b]), vec![b, a]);
        assert_eq!(Location::restored_open(&profile, &[]), vec![]);
    }
}
