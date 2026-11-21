//! The appearance of each state (decision 0029, section 10.5): the section
//! "Appearance for each state" of a key face or a bar panel when the first
//! gesture of the control, "Press" or "Tap", has a kind with two states,
//! for example a toggle.
//!
//! The section shows the two faces "Off" and "On" next to each other. Both
//! pictures are images of the engine (rule 11): the engine sends the image
//! of each state with the surface (`SurfaceState::states`). The selected
//! face chooses which appearance to edit. The kind description tells which
//! kind has two states and which setting holds a legacy On label (rule 8).

use super::*;
use crate::state::gestures;
use gpui_kit::FontWeight;
use opencraft_engine::{BindingId, FeedbackState, KindDescription, SurfaceContent, Value};

/// The state of the section that is not in the profile.
#[derive(Default)]
pub(super) struct StateFaces {
    /// The control whose face "On" is selected. The face "Off" of each
    /// other control is selected.
    on: Option<ControlId>,
    /// The window images of the two states of each surface: Off, On.
    pub(super) images: BTreeMap<SurfaceId, (Arc<RenderImage>, Arc<RenderImage>)>,
}

impl StateFaces {
    pub(super) fn new(state: &ViewState) -> Self {
        Self {
            images: state
                .surfaces
                .values()
                .filter_map(|surface| Some((surface.id, face_images(surface)?)))
                .collect(),
            ..Self::default()
        }
    }
}

fn face_images(
    surface: &opencraft_engine::SurfaceState,
) -> Option<(Arc<RenderImage>, Arc<RenderImage>)> {
    let states = surface.states.as_ref()?;
    Some((images::from_rgb(&states.off), images::from_rgb(&states.on)))
}

impl DeviceView {
    /// The binding of the first gesture of the selected surface control,
    /// if its kind has two states, and the description of the kind.
    pub(super) fn faces_binding(&self) -> Option<(BindingId, &KindDescription)> {
        self.state.selected?;
        let control = self.state.selected_control?;
        let gesture = gestures(control.kind).first()?.0;
        let binding = self.state.location.binding_id(control, gesture);
        let state = self.state.bindings.get(&binding).filter(|b| b.available)?;
        let kind = self.kind(&state.action.as_ref()?.kind)?;
        kind.states.as_ref()?;
        Some((binding, kind))
    }

    /// The key of the field of the label of the state On, if the form of
    /// `path` is the action of the binding of the appearance of each state.
    /// The section shows that field, thus the card does not.
    pub(super) fn on_label_in_faces(&self, path: &[actions::Slot]) -> Option<String> {
        if !path.is_empty() {
            return None;
        }
        let (binding, kind) = self.faces_binding()?;
        (self.form_binding() == Some(binding))
            .then(|| kind.states.as_ref()?.on_label.clone())
            .flatten()
    }

    /// The second line of the inspector header for a control with two
    /// states, for example "Toggle · the state now is Off".
    pub(super) fn state_line(&self) -> Option<String> {
        let (_, kind) = self.faces_binding()?;
        let now = self
            .state
            .selected
            .and_then(|id| self.state.surfaces.get(&id))
            .and_then(|s| s.states.as_ref())
            .map(|s| s.now);
        Some(match now {
            Some(FeedbackState::Off) => format!("{} · the state now is Off", kind.name),
            Some(FeedbackState::On) => format!("{} · the state now is On", kind.name),
            None => kind.name.clone(),
        })
    }

    /// Keep the window images of the two states of each surface.
    pub(super) fn state_images_event(&mut self, event: &Event, window: &mut Window) {
        let Event::Surface(surface) = event else {
            return;
        };
        let new = face_images(surface);
        let old = match new {
            Some(new) => self.faces.images.insert(surface.id, new),
            None => self.faces.images.remove(&surface.id),
        };
        if let Some((off, on)) = old {
            let _ = window.drop_image(off);
            let _ = window.drop_image(on);
        }
    }

    pub(super) fn editing_on_face(&self, surface: SurfaceId) -> bool {
        self.state.selected == Some(surface)
            && self.faces.on.is_some()
            && self.faces.on == self.state.selected_control
            && self.faces_binding().is_some()
    }

    pub(super) fn edited_face(&self, surface: SurfaceId) -> SurfaceContent {
        self.state
            .surfaces
            .get(&surface)
            .map(|s| s.content.face(self.editing_on_face(surface)).clone())
            .unwrap_or_default()
    }

    pub(super) fn legacy_on_label(&self) -> Option<String> {
        let (binding, kind) = self.faces_binding()?;
        let key = kind.states.as_ref()?.on_label.as_ref()?;
        match self.edit_action(binding)?.settings.get(key)? {
            Value::Text(text) if !text.trim().is_empty() => Some(text.clone()),
            _ => None,
        }
    }

    pub(super) fn clear_legacy_on_label(&mut self) -> Option<Command> {
        let (binding, kind) = self.faces_binding()?;
        let key = kind.states.as_ref()?.on_label.clone()?;
        let mut action = self.edit_action(binding)?;
        action.settings.remove(&key)?;
        Some(self.pending_binding(binding, Some(action)))
    }

    /// The face button of one state: the image of the engine and the name.
    fn face_button(
        &self,
        surface: SurfaceId,
        on: bool,
        now: Option<FeedbackState>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let control = self.state.selected_control;
        let selected = (self.faces.on.is_some() && self.faces.on == control) == on;
        let state = if on {
            FeedbackState::On
        } else {
            FeedbackState::Off
        };
        let name = if on { "On" } else { "Off" };
        let text = if now == Some(state) {
            format!("{name} · now")
        } else {
            name.to_string()
        };
        let image = self
            .faces
            .images
            .get(&surface)
            .map(|(off, image_on)| if on { image_on.clone() } else { off.clone() })
            .or_else(|| self.images.get(&surface).cloned());
        let id = if on { "state-on" } else { "state-off" };
        div()
            .id(id)
            .debug_selector(move || id.into())
            .accessible(Role::RadioButton, text.clone())
            .checked(selected)
            .flex_1()
            .min_w_0()
            .py(theme::SPACE_10)
            .flex()
            .flex_col()
            .items_center()
            .gap(theme::SPACE_8)
            .rounded(theme::RADIUS_STATE_BUTTON)
            .border_1()
            .text_size(theme::TEXT_LABEL)
            .font_weight(FontWeight::MEDIUM)
            .map(|d| {
                if selected {
                    d.bg(theme::ACCENT_TINT)
                        .border_color(theme::ACCENT_LINE)
                        .text_color(theme::TEXT_PRIMARY)
                } else {
                    d.border_color(theme::BORDER_FIELD)
                        .text_color(theme::TEXT_SECONDARY)
                        .cursor_pointer()
                        .hover(|d| d.bg(theme::HOVER_ROW))
                }
            })
            .child(
                div()
                    .flex_shrink_0()
                    .size(theme::STATE_FACE)
                    .rounded(theme::RADIUS_STATE_FACE)
                    .overflow_hidden()
                    .bg(theme::DEVICE_DISPLAY)
                    .border_1()
                    .border_color(theme::DEVICE_SURFACE_BORDER)
                    .children(
                        image.map(|image| img(image).size_full().rounded(theme::RADIUS_STATE_FACE)),
                    ),
            )
            .child(text)
            .map(|d| controls::keyboard_focus(d, controls::Ring::Inside))
            .on_click(cx.listener(move |view, _, window, cx| {
                view.faces.on = if on { control } else { None };
                view.close_kinds();
                view.focus_workspace(window, cx);
                view.load_label(window, cx);
                cx.notify();
            }))
            .into_any_element()
    }

    /// The section "Appearance for each state" of `surface`.
    pub(super) fn state_appearance(
        &self,
        surface: SurfaceId,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let now = self
            .state
            .surfaces
            .get(&surface)
            .and_then(|s| s.states.as_ref())
            .map(|s| s.now);
        form::section("appearance", "Appearance for each state", None)
            .child(
                div()
                    .id("state-faces")
                    .debug_selector(|| "state-faces".into())
                    .flex()
                    .gap(theme::STATE_FACE_GAP)
                    // The form is 14 below the faces: the gap of the section and 2.
                    .mb(theme::SPACE_2)
                    .child(self.face_button(surface, false, now, cx))
                    .child(self.face_button(surface, true, now, cx)),
            )
            .child(form::form(theme::FORM_GAP).children(self.appearance_rows(surface, window, cx)))
            .into_any_element()
    }
}
