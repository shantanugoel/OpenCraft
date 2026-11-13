//! The content of one control, for Copy, Paste, Clear, and Copy to All
//! Pages (decision 0029, section 13).
//!
//! The content of a control is its appearance and its bindings:
//!
//! - A key and a bar panel: the content of the surface of the control, and
//!   the bindings of the control.
//! - A knob: the bindings of the knob, and its bar panel. The bar panel is
//!   the feedback surface of the knob: its content, and the bindings of the
//!   bar panel control with that surface. The knob inspector shows the bar
//!   panel as a part of the knob (decision 0029, section 10.4), thus the
//!   copy has the same parts that the inspector shows.
//!
//! The parts of a control come from the descriptor, not from a check for a
//! model (rule 4). The engine keeps the clipboard: the shell sends only the
//! identity of the control (rule 12).

use crate::action::{Action, GestureKind};
use crate::profile::{Page, SurfaceContent};
use opencraft_core::{ControlId, ControlKind, Descriptor, SurfaceId};

/// The parts of the profile that belong to one control on a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Parts {
    /// The surface of the control. For a control with no display, for
    /// example a knob, the feedback surface.
    pub surface: Option<SurfaceId>,
    /// For a control with no display: the control that has the feedback
    /// surface as its own surface, for example the bar panel of a knob.
    pub panel: Option<ControlId>,
}

impl Parts {
    /// The parts of `control` in `descriptor`. `None` if the descriptor has
    /// no such control.
    pub fn of(descriptor: &Descriptor, control: ControlId) -> Option<Self> {
        let found = descriptor.control(control)?;
        if found.surface.is_some() {
            return Some(Self {
                surface: found.surface,
                panel: None,
            });
        }
        let surface = found.feedback_surface.filter(|id| {
            descriptor
                .surface(*id)
                .is_some_and(|surface| surface.slot.is_some())
        });
        let panel = surface.and_then(|id| {
            descriptor
                .controls
                .iter()
                .find(|c| c.surface == Some(id))
                .map(|c| c.id())
        });
        Some(Self { surface, panel })
    }
}

/// The content of one control: a copy that is independent of the page and
/// of the number of the control. An image path is relative to the
/// directory of the profile of the copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlContent {
    /// Paste is possible only on a control of this kind.
    pub kind: ControlKind,
    /// The content of the surface of [`Parts`]. `None` if the control has
    /// no such surface.
    pub surface: Option<SurfaceContent>,
    /// The bindings of the control.
    pub bindings: Vec<(GestureKind, Action)>,
    /// The bindings of the panel of [`Parts`]. `None` if the control has no
    /// panel.
    pub panel: Option<Vec<(GestureKind, Action)>>,
}

/// One change of a binding: set or remove the binding of the control and
/// the gesture.
pub(crate) type BindingChange = (ControlId, GestureKind, Option<Action>);

fn bindings_of(page: &Page, control: ControlId) -> Vec<(GestureKind, Action)> {
    page.bindings()
        .filter(|((c, _), _)| *c == control)
        .map(|((_, gesture), action)| (gesture, action.clone()))
        .collect()
}

/// The changes that make the bindings of `control` on `page` equal to
/// `bindings`: a removal of each binding that is not in the list, and the
/// list.
fn binding_changes(
    page: &Page,
    control: ControlId,
    bindings: &[(GestureKind, Action)],
) -> Vec<BindingChange> {
    let mut changes: Vec<BindingChange> = bindings_of(page, control)
        .into_iter()
        .filter(|(gesture, _)| !bindings.iter().any(|(g, _)| g == gesture))
        .map(|(gesture, _)| (control, gesture, None))
        .collect();
    changes.extend(
        bindings
            .iter()
            .map(|(gesture, action)| (control, *gesture, Some(action.clone()))),
    );
    changes
}

impl ControlContent {
    /// The content of `control` with the parts `parts` on `page`.
    pub(crate) fn read(page: &Page, control: ControlId, parts: Parts) -> Self {
        Self {
            kind: control.kind,
            surface: parts.surface.map(|id| page.content(id)),
            bindings: bindings_of(page, control),
            panel: parts.panel.map(|panel| bindings_of(page, panel)),
        }
    }

    /// The content of a control of `kind` with no appearance and no
    /// bindings. Clear puts it on a control.
    pub(crate) fn empty(kind: ControlKind) -> Self {
        Self {
            kind,
            surface: Some(SurfaceContent::default()),
            bindings: Vec::new(),
            panel: Some(Vec::new()),
        }
    }

    /// The changes that put this content on `control` with the parts
    /// `parts` on `page`: the new content of the surface, and the changes
    /// of the bindings. A part that the content or the control does not
    /// have does not change.
    pub(crate) fn changes(
        &self,
        page: &Page,
        control: ControlId,
        parts: Parts,
    ) -> (Option<(SurfaceId, SurfaceContent)>, Vec<BindingChange>) {
        let surface = parts.surface.zip(self.surface.clone());
        let mut bindings = binding_changes(page, control, &self.bindings);
        if let (Some(panel), Some(list)) = (parts.panel, &self.panel) {
            bindings.extend(binding_changes(page, panel, list));
        }
        (surface, bindings)
    }

    /// Put this content on `control` on `page` directly, with no event and
    /// no request to a handler: for a page that the device does not show.
    /// Return true if the page changed.
    pub(crate) fn write(&self, page: &mut Page, control: ControlId, parts: Parts) -> bool {
        let (surface, bindings) = self.changes(page, control, parts);
        let mut changed = false;
        if let Some((id, content)) = surface {
            changed |= page.update(id, |c| *c = content);
        }
        for (control, gesture, action) in bindings {
            changed |= page.set_binding(control, gesture, action);
        }
        changed
    }

    /// The image path of the surface part, if it has one.
    pub fn image(&self) -> Option<&str> {
        self.surface.as_ref()?.image.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::Color;
    use opencraft_core::device::DESCRIPTORS;

    static N4_PRO: &Descriptor = &DESCRIPTORS[0];

    fn action(kind: &str) -> Action {
        Action {
            kind: kind.into(),
            settings: Default::default(),
        }
    }

    #[test]
    fn a_knob_has_its_feedback_bar_panel_and_a_key_has_its_face() {
        let knob = N4_PRO
            .controls_of(ControlKind::Knob)
            .next()
            .expect("a knob");
        let parts = Parts::of(N4_PRO, knob.id()).unwrap();
        assert_eq!(parts.surface, knob.feedback_surface);
        let panel = N4_PRO.control(parts.panel.unwrap()).unwrap();
        assert_eq!(panel.kind(), ControlKind::BarPanel);
        assert_eq!(panel.surface, knob.feedback_surface);

        let key = N4_PRO.controls_of(ControlKind::Key).next().unwrap();
        let parts = Parts::of(N4_PRO, key.id()).unwrap();
        assert_eq!((parts.surface, parts.panel), (key.surface, None));
    }

    #[test]
    fn a_write_replaces_the_content_and_each_binding_of_the_control() {
        let mut keys = N4_PRO.controls_of(ControlKind::Key);
        let (one, two) = (keys.next().unwrap(), keys.next().unwrap());
        let (p1, p2) = (
            Parts::of(N4_PRO, one.id()).unwrap(),
            Parts::of(N4_PRO, two.id()).unwrap(),
        );
        let mut page = Page::new("Page 1");
        page.update(one.surface.unwrap(), |c| {
            c.label = Some("Mail".into());
            c.color = Some(Color::rgb(1, 2, 3));
        });
        page.set_binding(one.id(), GestureKind::Press, Some(action("a")));
        page.update(two.surface.unwrap(), |c| c.label = Some("Old".into()));
        page.set_binding(two.id(), GestureKind::Release, Some(action("b")));

        let copy = ControlContent::read(&page, one.id(), p1);
        assert!(copy.write(&mut page, two.id(), p2));
        assert_eq!(
            page.content(two.surface.unwrap()),
            page.content(one.surface.unwrap())
        );
        assert_eq!(
            page.binding(two.id(), GestureKind::Press),
            Some(&action("a"))
        );
        assert_eq!(page.binding(two.id(), GestureKind::Release), None);
        assert!(!copy.write(&mut page, two.id(), p2), "no second change");

        assert!(ControlContent::empty(ControlKind::Key).write(&mut page, two.id(), p2));
        assert!(page.content(two.surface.unwrap()).is_empty());
        assert_eq!(
            page.bindings().filter(|((c, _), _)| *c == two.id()).count(),
            0
        );
    }
}
