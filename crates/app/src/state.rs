//! Window-independent state of the shell. No device or file work occurs here.

use opencraft_engine::{
    ControlId, ControlKind, DeviceLayout, Event, Gesture, InputEvent, SurfaceId, SurfaceKind,
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
#[derive(Default)]
pub struct ViewState {
    pub layout: Option<Arc<DeviceLayout>>,
    pub surfaces: BTreeMap<SurfaceId, SurfaceState>,
    pub connected: bool,
    pub selected: Option<SurfaceId>,
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
            Event::Problem(text) => self.problems.push(text),
            Event::Input(input) => return self.input(input, now),
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
        true
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
                },
                ControlLayout {
                    id: control(ControlKind::Knob),
                    row: 1,
                    column: 1,
                    surface: None,
                },
                ControlLayout {
                    id: control(ControlKind::BarPanel),
                    row: 1,
                    column: 1,
                    surface: Some(bar()),
                },
                ControlLayout {
                    id: control(ControlKind::TouchBar),
                    row: 1,
                    column: 1,
                    surface: None,
                },
            ],
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
