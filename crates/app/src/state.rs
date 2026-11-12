//! Window-independent state of the shell. No device or file work occurs here.

use opencraft_engine::{
    BindingId, BindingState, ControlId, ControlKind, DeviceLayout, DeviceSettings, Event, Gesture,
    GestureKind, InputEvent, KindDescription, ProfileEntry, Status, SurfaceId, SurfaceKind,
    SurfaceState,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::{Duration, Instant},
};

pub const HIGHLIGHT_TIME: Duration = Duration::from_millis(280);

/// A short input mark. Its deadline also identifies it to a timer task.
#[derive(Clone, Copy, Debug)]
pub struct Highlight {
    pub gesture: Gesture,
    pub until: Instant,
}

/// The latest engine state and the temporary marks shown in the window.
#[derive(Default, Clone)]
pub struct ViewState {
    pub layout: Option<Arc<DeviceLayout>>,
    pub surfaces: BTreeMap<SurfaceId, SurfaceState>,
    pub connected: bool,
    pub selected: Option<SurfaceId>,
    pub selected_control: Option<ControlId>,
    pub gesture: Option<GestureKind>,
    pub kinds: Vec<KindDescription>,
    /// The page that the device shows. The commands for a binding use it.
    pub page: usize,
    /// The names of the pages of the active profile, in their order.
    pub pages: Vec<String>,
    /// The profiles of the device model, and the identity of the active profile.
    pub profiles: Vec<ProfileEntry>,
    pub profile: Option<String>,
    /// The count of the events that can change the active page. An edit that
    /// started at a different count is for a page that the device does not
    /// show now, so the shell drops it.
    pub epoch: u64,
    /// The bindings of the active page only.
    pub bindings: BTreeMap<BindingId, BindingState>,
    pub statuses: BTreeMap<String, Status>,
    /// The device settings of the model of the device. The engine sends them
    /// at the start and after each change. The layout tells which settings
    /// the model has.
    pub device_settings: Option<DeviceSettings>,
    pub pressed: BTreeSet<ControlId>,
    /// One step is one twelfth of a turn. This is a display mark, not device state.
    pub knob_steps: BTreeMap<ControlId, i32>,
    pub touch: Option<(u16, u16)>,
    pub highlights: BTreeMap<ControlId, Highlight>,
    pub last_input: Option<String>,
    pub problems: Vec<String>,
}

impl ViewState {
    /// Apply an engine event. Return the deadline of a new short highlight.
    pub fn apply(&mut self, event: Event, now: Instant) -> Option<(ControlId, Instant)> {
        match event {
            Event::Layout(layout) => {
                self.surfaces.retain(|id, _| layout.surface(*id).is_some());
                if self.selected.is_some_and(|id| layout.surface(id).is_none()) {
                    self.selected = None;
                }
                if self
                    .selected_control
                    .is_some_and(|id| !layout.controls.iter().any(|c| c.id == id))
                {
                    self.selected_control = None;
                    self.gesture = None;
                }
                self.clear_input();
                self.knob_steps.clear();
                self.layout = Some(layout);
            }
            Event::Surface(surface) => {
                self.surfaces.insert(surface.id, surface);
            }
            Event::Connected => self.connected = true,
            Event::Disconnected => {
                self.connected = false;
                self.clear_input();
            }
            // The same text shows one time. For example, each press of a key
            // can send the same problem.
            Event::Problem(text) => {
                if !self.problems.contains(&text) {
                    self.problems.push(text);
                }
            }
            Event::Input(input) => return self.input(input, now),
            Event::Kinds(kinds) => self.kinds = kinds,
            Event::Profiles { profiles, active } => {
                // The engine sends the pages and the bindings of a new active
                // profile next. Drop all that is of the old profile.
                if self.profile.as_ref().is_some_and(|p| *p != active) {
                    self.pages.clear();
                    self.bindings.clear();
                    self.epoch += 1;
                }
                self.profiles = profiles;
                self.profile = Some(active);
            }
            Event::Pages { names, active } => {
                // The engine sends each binding of the active page next.
                self.page = active;
                self.pages = names;
                self.bindings.clear();
                self.epoch += 1;
            }
            Event::Binding(binding) => {
                if binding.action.is_some() {
                    self.bindings.insert(binding.binding, binding);
                } else {
                    self.bindings.remove(&binding.binding);
                }
            }
            Event::Status { kind, status } => {
                self.statuses.insert(kind, status);
            }
            Event::DeviceSettings(settings) => self.device_settings = Some(settings),
            Event::Counters(_) => {}
        }
        None
    }

    fn clear_input(&mut self) {
        self.pressed.clear();
        self.touch = None;
        self.highlights.clear();
    }

    fn input(&mut self, input: InputEvent, now: Instant) -> Option<(ControlId, Instant)> {
        let InputEvent { control, gesture } = input;
        self.last_input = Some(format!("{control}: {}", gesture.name()));
        match gesture {
            Gesture::Press => {
                self.pressed.insert(control);
            }
            Gesture::Release => {
                self.pressed.remove(&control);
            }
            Gesture::TurnLeft | Gesture::TurnRight => {
                let step = self.knob_steps.entry(control).or_default();
                *step = (*step + if gesture == Gesture::TurnRight { 1 } else { -1 }).rem_euclid(12);
            }
            Gesture::TouchStart { x } => self.touch = Some((x, 0)),
            Gesture::TouchPoint { x, y } => self.touch = Some((x, y)),
            Gesture::Tap { .. } | Gesture::SwipeLeft | Gesture::SwipeRight | Gesture::TouchEnd => {
                self.touch = None;
            }
            Gesture::Push => {}
        }
        if matches!(
            gesture,
            Gesture::TurnLeft
                | Gesture::TurnRight
                | Gesture::Push
                | Gesture::Tap { .. }
                | Gesture::SwipeLeft
                | Gesture::SwipeRight
        ) {
            let until = now + HIGHLIGHT_TIME;
            self.highlights
                .insert(control, Highlight { gesture, until });
            Some((control, until))
        } else {
            None
        }
    }

    /// An old timer cannot remove a newer mark on the same control.
    pub fn expire(&mut self, control: ControlId, deadline: Instant) {
        if self
            .highlights
            .get(&control)
            .is_some_and(|h| h.until == deadline)
        {
            self.highlights.remove(&control);
        }
    }

    pub fn select(&mut self, id: SurfaceId) -> bool {
        if !matches!(id.kind, SurfaceKind::KeyFace | SurfaceKind::BarPanel)
            || !self
                .layout
                .as_ref()
                .is_some_and(|l| l.surface(id).is_some())
            || self.selected == Some(id)
        {
            return false;
        }
        self.selected = Some(id);
        self.selected_control = self.layout.as_ref().and_then(|l| {
            l.controls
                .iter()
                .find(|c| c.surface == Some(id))
                .map(|c| c.id)
        });
        self.gesture = self.selected_control.and_then(|c| default_gesture(c.kind));
        true
    }

    pub fn select_control(&mut self, id: ControlId) -> bool {
        if self.selected_control == Some(id) {
            return false;
        }
        let Some(control) = self
            .layout
            .as_ref()
            .and_then(|l| l.controls.iter().find(|c| c.id == id))
        else {
            return false;
        };
        self.selected_control = Some(id);
        self.selected = control.surface;
        self.gesture = default_gesture(id.kind);
        true
    }

    pub fn binding_id(&self) -> Option<BindingId> {
        Some(BindingId {
            page: self.page,
            control: self.selected_control?,
            gesture: self.gesture?,
        })
    }

    /// The control associated with a surface comes from the layout only.
    pub fn surface_active(&self, id: SurfaceId) -> bool {
        self.layout.as_ref().is_some_and(|layout| {
            layout.controls.iter().any(|c| {
                (c.surface == Some(id)
                    && (self.pressed.contains(&c.id) || self.highlights.contains_key(&c.id)))
                    || (id.kind == SurfaceKind::BarPanel
                        && c.id.kind == ControlKind::TouchBar
                        && self.highlights.contains_key(&c.id))
            })
        })
    }

    /// The top of the bar panels is the origin of touch y coordinates.
    pub fn touch_position(&self) -> Option<(f32, f32)> {
        let layout = self.layout.as_ref()?;
        let panel = layout
            .surfaces
            .iter()
            .filter(|s| s.id.kind == SurfaceKind::BarPanel)
            .min_by_key(|s| s.y)?;
        let (x, y) = self.touch?;
        Some((
            f32::from(x.min(layout.display_width.saturating_sub(1))),
            f32::from(panel.y) + f32::from(y.min(panel.height.saturating_sub(1))),
        ))
    }
}

/// The gestures that a binding of a control of this kind can have, with
/// their names for the user. The touch gestures are in decision 0015.
pub fn gestures(kind: ControlKind) -> &'static [(GestureKind, &'static str)] {
    match kind {
        ControlKind::Key => &[
            (GestureKind::Press, "Press"),
            (GestureKind::Release, "Release"),
        ],
        ControlKind::Knob => &[
            (GestureKind::TurnLeft, "Turn left"),
            (GestureKind::TurnRight, "Turn right"),
            (GestureKind::Push, "Push"),
        ],
        ControlKind::BarPanel => &[(GestureKind::Tap, "Tap")],
        ControlKind::TouchBar => &[
            (GestureKind::SwipeLeft, "Swipe left"),
            (GestureKind::SwipeRight, "Swipe right"),
        ],
    }
}

fn default_gesture(kind: ControlKind) -> Option<GestureKind> {
    match kind {
        ControlKind::Knob => Some(GestureKind::TurnRight),
        _ => gestures(kind).first().map(|&(gesture, _)| gesture),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use opencraft_engine::{ControlLayout, RgbImage, SurfaceContent, SurfaceLayout};

    pub fn layout() -> Event {
        Event::Layout(Arc::new(DeviceLayout {
            name: "Test device".into(),
            display_width: 400,
            display_height: 300,
            surfaces: vec![
                SurfaceLayout {
                    id: key(),
                    x: 20,
                    y: 10,
                    width: 90,
                    height: 90,
                },
                SurfaceLayout {
                    id: bar(),
                    x: 30,
                    y: 180,
                    width: 180,
                    height: 60,
                },
            ],
            controls: vec![
                ControlLayout {
                    id: control(ControlKind::Key),
                    row: 1,
                    column: 1,
                    surface: Some(key()),
                    feedback_surface: Some(key()),
                },
                ControlLayout {
                    id: control(ControlKind::Knob),
                    row: 1,
                    column: 1,
                    surface: None,
                    feedback_surface: Some(bar()),
                },
                ControlLayout {
                    id: control(ControlKind::BarPanel),
                    row: 1,
                    column: 1,
                    surface: Some(bar()),
                    feedback_surface: Some(bar()),
                },
                ControlLayout {
                    id: control(ControlKind::TouchBar),
                    row: 1,
                    column: 1,
                    surface: None,
                    feedback_surface: None,
                },
            ],
            settings: opencraft_engine::SettingsSupport {
                lowest_brightness: 15,
                knob_lights: 2,
                haptic: true,
            },
        }))
    }
    pub fn key() -> SurfaceId {
        SurfaceId::new(SurfaceKind::KeyFace, 2)
    }
    pub fn bar() -> SurfaceId {
        SurfaceId::new(SurfaceKind::BarPanel, 3)
    }
    fn control(kind: ControlKind) -> ControlId {
        ControlId::new(kind, 2)
    }
    fn input(state: &mut ViewState, kind: ControlKind, gesture: Gesture, now: Instant) {
        state.apply(
            Event::Input(InputEvent {
                control: control(kind),
                gesture,
            }),
            now,
        );
    }

    #[test]
    fn engine_state_and_early_problems_are_kept() {
        let now = Instant::now();
        let mut s = ViewState::default();
        assert!(!s.connected);
        s.apply(Event::Problem("Cannot save.".into()), now);
        s.apply(layout(), now);
        assert_eq!(s.problems, ["Cannot save."]);
        // The same problem again does not show a second time. After the
        // user closed it, it shows again.
        s.apply(Event::Problem("Cannot save.".into()), now);
        assert_eq!(s.problems, ["Cannot save."]);
        s.problems.clear();
        s.apply(Event::Problem("Cannot save.".into()), now);
        assert_eq!(s.problems, ["Cannot save."]);
        s.apply(Event::Connected, now);
        assert!(s.connected);
        let surface = SurfaceState {
            id: key(),
            content: SurfaceContent {
                label: Some("Mail".into()),
                ..Default::default()
            },
            image: Arc::new(RgbImage::new(90, 90)),
        };
        s.apply(Event::Surface(surface.clone()), now);
        assert_eq!(s.surfaces[&key()], surface);
        assert!(s.select(key()));
        assert!(!s.select(key()));
        assert!(s.select(bar()));
    }

    #[test]
    fn action_events_are_the_source_of_truth() {
        use opencraft_engine::{Action, Settings};
        let now = Instant::now();
        let mut s = ViewState::default();
        let kind = KindDescription {
            kind: "test.kind".into(),
            name: "Test action".into(),
            fields: vec![],
        };
        s.apply(Event::Kinds(vec![kind.clone()]), now);
        assert_eq!(s.kinds, vec![kind]);
        s.apply(layout(), now);
        assert!(s.select(key()));
        let id = s.binding_id().unwrap();
        assert_eq!(id.gesture, GestureKind::Press);
        let binding = BindingState {
            binding: id,
            action: Some(Action {
                kind: "test.kind".into(),
                settings: Settings::new(),
            }),
            available: false,
        };
        s.apply(Event::Binding(binding.clone()), now);
        assert_eq!(s.bindings.get(&id), Some(&binding));
        let status = Status::NeedsUser {
            text: "Do a step.".into(),
            button: "Continue".into(),
        };
        s.apply(
            Event::Status {
                kind: "test.kind".into(),
                status: status.clone(),
            },
            now,
        );
        assert_eq!(s.statuses.get("test.kind"), Some(&status));
        s.apply(
            Event::Status {
                kind: "test.kind".into(),
                status: Status::Ready,
            },
            now,
        );
        assert_eq!(s.statuses.get("test.kind"), Some(&Status::Ready));
        s.apply(Event::Disconnected, now);
        assert!(s.bindings.contains_key(&id));
        s.apply(
            Event::Binding(BindingState {
                binding: id,
                action: None,
                available: false,
            }),
            now,
        );
        assert!(!s.bindings.contains_key(&id));
        s.apply(Event::Kinds(vec![]), now);
        assert!(s.kinds.is_empty());
    }

    #[test]
    fn a_pages_event_removes_the_bindings_and_sets_the_page_of_an_edit() {
        use opencraft_engine::{Action, Settings};
        let now = Instant::now();
        let mut s = ViewState::default();
        s.apply(layout(), now);
        assert!(s.select(key()));
        let id = s.binding_id().unwrap();
        assert_eq!(id.page, 0);
        s.apply(
            Event::Binding(BindingState {
                binding: id,
                action: Some(Action {
                    kind: "test.kind".into(),
                    settings: Settings::new(),
                }),
                available: true,
            }),
            now,
        );
        s.apply(
            Event::Pages {
                names: vec!["Page 1".into(), "Page 2".into()],
                active: 1,
            },
            now,
        );
        assert!(s.bindings.is_empty());
        assert_eq!(s.binding_id().unwrap().page, 1);
    }

    #[test]
    fn control_selection_uses_layout_and_defaults() {
        let now = Instant::now();
        let mut s = ViewState::default();
        s.apply(layout(), now);
        assert!(s.select_control(control(ControlKind::Knob)));
        assert_eq!(s.selected, None);
        assert_eq!(s.binding_id().unwrap().gesture, GestureKind::TurnRight);
        assert!(s.select(key()));
        assert_eq!(s.selected_control, Some(control(ControlKind::Key)));
        assert_eq!(s.binding_id().unwrap().gesture, GestureKind::Press);
        assert!(s.select(bar()));
        assert_eq!(s.selected_control, Some(control(ControlKind::BarPanel)));
        assert_eq!(s.binding_id().unwrap().gesture, GestureKind::Tap);
        assert!(s.select_control(control(ControlKind::TouchBar)));
        assert_eq!(s.selected, None);
        assert_eq!(s.binding_id().unwrap().gesture, GestureKind::SwipeLeft);
        assert!(!s.select_control(ControlId::new(ControlKind::Key, 99)));
        assert!(s.select_control(control(ControlKind::Knob)));
        let Event::Layout(mut empty) = layout() else {
            panic!()
        };
        Arc::make_mut(&mut empty).controls.clear();
        s.apply(Event::Layout(empty), now);
        assert_eq!(s.selected_control, None);
        assert_eq!(s.binding_id(), None);
    }

    #[test]
    fn key_stays_pressed_until_release_or_disconnect() {
        let now = Instant::now();
        let mut s = ViewState::default();
        s.apply(layout(), now);
        input(&mut s, ControlKind::Key, Gesture::Press, now);
        assert!(s.surface_active(key()));
        input(&mut s, ControlKind::Key, Gesture::Release, now);
        assert!(!s.surface_active(key()));
        input(&mut s, ControlKind::Key, Gesture::Press, now);
        s.apply(Event::Disconnected, now);
        assert!(!s.surface_active(key()));
    }

    #[test]
    fn turns_pushes_and_taps_have_independent_expiry() {
        let now = Instant::now();
        let mut s = ViewState::default();
        s.apply(layout(), now);
        let knob = control(ControlKind::Knob);
        input(&mut s, ControlKind::Knob, Gesture::TurnLeft, now);
        assert_eq!(s.knob_steps[&knob], 11);
        input(&mut s, ControlKind::Knob, Gesture::TurnRight, now);
        assert_eq!(s.knob_steps[&knob], 0);
        input(
            &mut s,
            ControlKind::Knob,
            Gesture::Push,
            now + Duration::from_millis(10),
        );
        s.expire(knob, now + HIGHLIGHT_TIME);
        assert!(s.highlights.contains_key(&knob));
        s.expire(knob, now + HIGHLIGHT_TIME + Duration::from_millis(10));
        assert!(!s.highlights.contains_key(&knob));
        input(&mut s, ControlKind::BarPanel, Gesture::Tap { x: 80 }, now);
        assert!(s.surface_active(bar()));
        s.expire(control(ControlKind::BarPanel), now + HIGHLIGHT_TIME);
        assert!(!s.surface_active(bar()));
    }

    #[test]
    fn touch_uses_display_x_and_panel_relative_y_and_ends_on_each_lift() {
        let now = Instant::now();
        for lift in [
            Gesture::Tap { x: 80 },
            Gesture::SwipeLeft,
            Gesture::SwipeRight,
            Gesture::TouchEnd,
        ] {
            let mut s = ViewState::default();
            s.apply(layout(), now);
            input(
                &mut s,
                ControlKind::TouchBar,
                Gesture::TouchStart { x: 80 },
                now,
            );
            assert_eq!(s.touch_position(), Some((80., 180.)));
            input(
                &mut s,
                ControlKind::TouchBar,
                Gesture::TouchPoint { x: 90, y: 30 },
                now,
            );
            assert_eq!(s.touch_position(), Some((90., 210.)));
            input(&mut s, ControlKind::TouchBar, lift, now);
            assert_eq!(s.touch_position(), None);
            if matches!(lift, Gesture::SwipeLeft | Gesture::SwipeRight) {
                assert!(s.surface_active(bar()));
            }
        }
    }
}
