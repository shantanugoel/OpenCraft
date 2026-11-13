//! The appearance of each state (decision 0029, section 10.5): the section
//! "Appearance for each state" of a key face or a bar panel when the first
//! gesture of the control, "Press" or "Tap", has a kind with two states,
//! for example a toggle.
//!
//! The section shows the two faces "Off" and "On" next to each other. Both
//! pictures are images of the engine (rule 11): the engine sends the image
//! of each state with the surface (`SurfaceState::states`). The selected
//! face chooses the form below: "Off" has the rows of "Appearance", and
//! "On" has the label of the state On, a field of the action. The kind
//! description tells which kind has two states and which field is the
//! label (`KindDescription::states`), thus the window has no list of kinds
//! (rule 8).

use super::*;
use crate::state::gestures;
use gpui_kit::FontWeight;
use opencraft_engine::{BindingId, FeedbackState, KindDescription, Value};

/// The state of the section that is not in the profile.
pub(super) struct StateFaces {
    /// The control whose face "On" is selected. The face "Off" of each
    /// other control is selected.
    on: Option<ControlId>,
    /// The field "Title" of the face "On".
    pub(super) input: Entity<InputState>,
    _subscription: Subscription,
    /// The window images of the two states of each surface: Off, On.
    pub(super) images: BTreeMap<SurfaceId, (Arc<RenderImage>, Arc<RenderImage>)>,
}

impl StateFaces {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Label"));
        let subscription = cx.subscribe_in(&input, window, |view, input, event, _, cx| {
            view.text_edit_event(input.entity_id(), event, cx);
            if matches!(event, TextEvent::Change) {
                let text = input.read(cx).value().to_string();
                view.set_on_label(text);
            }
        });
        Self {
            on: None,
            input,
            _subscription: subscription,
            images: BTreeMap::new(),
        }
    }
}

impl DeviceView {
    /// The binding of the first gesture of the selected surface control,
    /// if its kind has two states, and the description of the kind.
    pub(super) fn faces_binding(&self) -> Option<(BindingId, &KindDescription)> {
        self.state.selected?;
        let control = self.state.selected_control?;
        let gesture = gestures(control.kind).first()?.0;
        let binding = BindingId {
            page: self.state.page,
            control,
            gesture,
        };
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
        let new = surface
            .states
            .as_ref()
            .map(|s| (images::from_rgb(&s.off), images::from_rgb(&s.on)));
        let old = match new {
            Some(new) => self.faces.images.insert(surface.id, new),
            None => self.faces.images.remove(&surface.id),
        };
        if let Some((off, on)) = old {
            let _ = window.drop_image(off);
            let _ = window.drop_image(on);
        }
    }

    /// Put the label of the state On of the action into the field "Title"
    /// of the face "On", unless the user types in it.
    pub(super) fn sync_state_faces(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((binding, kind)) = self.faces_binding() else {
            return;
        };
        let Some(key) = kind.states.as_ref().and_then(|s| s.on_label.clone()) else {
            return;
        };
        let input = self.faces.input.clone();
        if gpui_kit::Focusable::focus_handle(input.read(cx), cx).is_focused(window) {
            return;
        }
        let text = match self
            .edit_action(binding)
            .and_then(|a| a.settings.get(&key).cloned())
        {
            Some(Value::Text(text)) => text,
            _ => String::new(),
        };
        if input.read(cx).value().as_ref() != text {
            input.update(cx, |input, cx| input.set_value(text, window, cx));
        }
    }

    /// The user typed in the field "Title" of the face "On". An empty text
    /// removes the label.
    fn set_on_label(&mut self, text: String) {
        let Some((binding, kind)) = self.faces_binding() else {
            return;
        };
        let Some(key) = kind.states.as_ref().and_then(|s| s.on_label.clone()) else {
            return;
        };
        self.edit(binding, |action| {
            if text.is_empty() {
                action.settings.remove(&key);
            } else {
                action.settings.insert(key, Value::Text(text));
            }
        });
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
            .on_click(cx.listener(move |view, _, window, cx| {
                view.faces.on = if on { control } else { None };
                view.close_kinds();
                view.sync_state_faces(window, cx);
                view.focus_workspace(window, cx);
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
        let on = self.faces.on.is_some() && self.faces.on == self.state.selected_control;
        let form = if on {
            form::form(theme::FORM_GAP)
                .child(form::row(
                    "Title",
                    theme::FORM_LABEL,
                    theme::FORM_GAP,
                    controls::text_field(
                        &gpui_kit::component::input::AnyInputState::Input(self.faces.input.clone()),
                        theme::BG_FIELD,
                        window,
                        cx,
                    )
                    .id("on-label-field")
                    .debug_selector(|| "on-label-field".into()),
                ))
                .child(form::help(
                    "The On face uses the fill and the image of the Off face, with a frame.",
                ))
                .into_any_element()
        } else {
            form::form(theme::FORM_GAP)
                .children(self.appearance_rows(surface, window, cx))
                .into_any_element()
        };
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
            .child(form)
            .into_any_element()
    }
}
