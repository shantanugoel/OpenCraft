//! Window-independent state of the shell. No device or file work occurs here.

use opencraft_engine::{
    BindingId, BindingState, ControlId, ControlKind, DeviceLayout, DeviceSettings, Event, Gesture,
    GestureKind, InputEvent, KindDescription, ProfileEntry, Status, SurfaceId, SurfaceKind,
    SurfaceState,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

pub const HIGHLIGHT_TIME: Duration = Duration::from_millis(280);

/// A short input mark. Its deadline also identifies it to a timer task.
#[derive(Clone, Copy, Debug)]
pub struct Highlight {
    pub gesture: Gesture,
    pub until: Instant,
}

/// The time in which the same gesture on the same control counts as one
/// more of the last input (decision 0029, section 6.1).
pub const REPEAT_TIME: Duration = Duration::from_secs(1);

/// The last input from the device, for the status bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LastInput {
    pub control: ControlId,
    /// The name of the gesture, as `Gesture::name` gives it.
    pub gesture: &'static str,
    /// The number of the same gestures, each less than [`REPEAT_TIME`] after
    /// the one before.
    pub count: u32,
    /// The time of the last of these gestures.
    pub at: Instant,
    /// The same time, on the clock of the system.
    pub time: SystemTime,
}

/// The latest engine state and the temporary marks shown in the window.
#[derive(Default, Clone)]
pub struct ViewState {
    pub layout: Option<Arc<DeviceLayout>>,
    pub surfaces: BTreeMap<SurfaceId, SurfaceState>,
    pub connected: bool,
    pub selected: Option<SurfaceId>,
    pub selected_control: Option<ControlId>,
    /// True when the page is the selection (decision 0029, section 8.3).
    /// Then no surface is selected, and `selected_control` is the touch bar
    /// of the layout: the swipe gestures belong to the page (section 10.6).
    pub page_selected: bool,
    /// The gesture of the open gesture card of the selected control. The
    /// form of the inspector edits the binding of this gesture. With no open
    /// card, it is `None`.
    pub gesture: Option<GestureKind>,
    /// The open card of each control that the user opened or closed while
    /// the application runs (decision 0029, section 10.3). A control that is
    /// not in this map opens its first card with an action.
    pub open_cards: BTreeMap<ControlId, Option<GestureKind>>,
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
    pub last_input: Option<LastInput>,
    pub problems: Vec<String>,
    /// The name of the step that an undo undoes, and of the step that a redo
    /// does again. `None` is an empty list (decision 0030).
    pub undo: Option<String>,
    pub redo: Option<String>,
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
                if self.page_selected {
                    self.selected_control = self.touch_bar();
                    self.gesture = self.selected_control.and_then(|c| self.open_card_of(c));
                }
                // At the start, the selection is the first key of the layout
                // (decision 0029, section 8.3).
                if self.selected_control.is_none() && !self.page_selected {
                    if let Some(first) = self.first_key() {
                        self.select_control(first);
                    }
                }
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
                self.update_open_card();
            }
            Event::Binding(binding) => {
                if binding.action.is_some() {
                    self.bindings.insert(binding.binding, binding);
                } else {
                    self.bindings.remove(&binding.binding);
                }
                self.update_open_card();
            }
            Event::Status { kind, status } => {
                self.statuses.insert(kind, status);
            }
            Event::DeviceSettings(settings) => self.device_settings = Some(settings),
            Event::History { undo, redo } => {
                self.undo = undo;
                self.redo = redo;
            }
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
        let count = match self.last_input {
            Some(last)
                if last.control == control
                    && last.gesture == gesture.name()
                    && now.saturating_duration_since(last.at) <= REPEAT_TIME =>
            {
                last.count + 1
            }
            _ => 1,
        };
        self.last_input = Some(LastInput {
            control,
            gesture: gesture.name(),
            count,
            at: now,
            time: SystemTime::now(),
        });
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
        self.page_selected = false;
        self.selected = Some(id);
        self.selected_control = self.layout.as_ref().and_then(|l| {
            l.controls
                .iter()
                .find(|c| c.surface == Some(id))
                .map(|c| c.id)
        });
        self.gesture = self.selected_control.and_then(|c| self.open_card_of(c));
        true
    }

    /// Select a control. The touch bar is part of the page: its selection
    /// is the selection of the page.
    pub fn select_control(&mut self, id: ControlId) -> bool {
        if id.kind == ControlKind::TouchBar {
            return self.select_page();
        }
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
        self.page_selected = false;
        self.selected_control = Some(id);
        self.selected = control.surface;
        self.gesture = self.open_card_of(id);
        true
    }

    /// Open the card of `gesture` of the selected control, or close the open
    /// card with `None`. The inspector remembers it for this control.
    pub fn open_card(&mut self, gesture: Option<GestureKind>) {
        if let Some(control) = self.selected_control {
            self.open_cards.insert(control, gesture);
            self.gesture = gesture;
        }
    }

    /// The open card of `control`: the card that the user opened last, or
    /// the first card with an action on the active page.
    fn open_card_of(&self, control: ControlId) -> Option<GestureKind> {
        if let Some(gesture) = self.open_cards.get(&control) {
            return *gesture;
        }
        gestures(control.kind)
            .iter()
            .map(|&(gesture, _)| gesture)
            .find(|&gesture| {
                self.bindings.contains_key(&BindingId {
                    page: self.page,
                    control,
                    gesture,
                })
            })
    }

    /// The bindings changed. A control that the user did not open or close
    /// opens its first card with an action.
    fn update_open_card(&mut self) {
        if let Some(control) = self.selected_control {
            self.gesture = self.open_card_of(control);
        }
    }

    /// Make the page the selection. Return false if it is the selection
    /// already.
    pub fn select_page(&mut self) -> bool {
        if self.page_selected {
            return false;
        }
        self.page_selected = true;
        self.selected = None;
        self.selected_control = self.touch_bar();
        self.gesture = self.selected_control.and_then(|c| self.open_card_of(c));
        true
    }

    /// The touch bar of the layout. Its gestures are the swipes of the page.
    pub fn touch_bar(&self) -> Option<ControlId> {
        self.layout
            .as_ref()?
            .controls
            .iter()
            .find(|c| c.id.kind == ControlKind::TouchBar)
            .map(|c| c.id)
    }

    /// The key of the top row and the first column of the layout.
    fn first_key(&self) -> Option<ControlId> {
        self.layout
            .as_ref()?
            .controls
            .iter()
            .filter(|c| c.id.kind == ControlKind::Key)
            .min_by_key(|c| (c.row, c.column, c.id.number))
            .map(|c| c.id)
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
    fn the_same_gesture_within_one_second_counts_as_one_more() {
        let now = Instant::now();
        let mut s = ViewState::default();
        input(&mut s, ControlKind::Knob, Gesture::TurnRight, now);
        input(
            &mut s,
            ControlKind::Knob,
            Gesture::TurnRight,
            now + REPEAT_TIME,
        );
        input(
            &mut s,
            ControlKind::Knob,
            Gesture::TurnRight,
            now + 2 * REPEAT_TIME,
        );
        let last = s.last_input.unwrap();
        assert_eq!(
            (last.control, last.gesture, last.count),
            (control(ControlKind::Knob), "turn right", 3)
        );
        assert_eq!(last.at, now + 2 * REPEAT_TIME);
        // A different gesture, a different control, or a later time starts
        // again at 1.
        input(
            &mut s,
            ControlKind::Knob,
            Gesture::TurnLeft,
            now + 2 * REPEAT_TIME,
        );
        assert_eq!(s.last_input.unwrap().count, 1);
        input(
            &mut s,
            ControlKind::Key,
            Gesture::Press,
            now + 2 * REPEAT_TIME,
        );
        input(
            &mut s,
            ControlKind::Key,
            Gesture::Press,
            now + 4 * REPEAT_TIME,
        );
        assert_eq!(s.last_input.unwrap().count, 1);
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
        // The first key of the layout is the selection at the start.
        assert_eq!(s.selected, Some(key()));
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
            ..Default::default()
        };
        s.apply(Event::Kinds(vec![kind.clone()]), now);
        assert_eq!(s.kinds, vec![kind]);
        s.apply(layout(), now);
        assert_eq!(s.selected, Some(key()));
        s.open_card(Some(GestureKind::Press));
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
        assert_eq!(s.selected, Some(key()));
        s.open_card(Some(GestureKind::Press));
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
    fn control_selection_uses_layout_and_opens_the_first_card_with_an_action() {
        use opencraft_engine::{Action, Settings};
        let now = Instant::now();
        let mut s = ViewState::default();
        s.apply(layout(), now);
        let bound = |control: ControlId, gesture| {
            Event::Binding(BindingState {
                binding: BindingId {
                    page: 0,
                    control,
                    gesture,
                },
                action: Some(Action {
                    kind: "test.kind".into(),
                    settings: Settings::new(),
                }),
                available: true,
            })
        };
        // With no action, no card is open.
        assert!(s.select_control(control(ControlKind::Knob)));
        assert_eq!(s.selected, None);
        assert_eq!(s.binding_id(), None);
        // A binding that comes later opens its card, in the order of the
        // gestures: "Turn left" before "Push".
        s.apply(bound(control(ControlKind::Knob), GestureKind::Push), now);
        assert_eq!(s.gesture, Some(GestureKind::Push));
        s.apply(
            bound(control(ControlKind::Knob), GestureKind::TurnLeft),
            now,
        );
        assert_eq!(s.gesture, Some(GestureKind::TurnLeft));
        assert!(s.select(key()));
        assert_eq!(s.selected_control, Some(control(ControlKind::Key)));
        assert_eq!(s.gesture, None);
        // The inspector remembers a card that the user opens or closes.
        s.open_card(Some(GestureKind::Release));
        assert!(s.select(bar()));
        assert_eq!(s.selected_control, Some(control(ControlKind::BarPanel)));
        assert!(s.select(key()));
        assert_eq!(s.binding_id().unwrap().gesture, GestureKind::Release);
        s.apply(bound(control(ControlKind::Key), GestureKind::Press), now);
        assert_eq!(s.gesture, Some(GestureKind::Release));
        assert!(s.select_control(control(ControlKind::Knob)));
        s.open_card(None);
        s.apply(
            bound(control(ControlKind::Knob), GestureKind::TurnRight),
            now,
        );
        assert_eq!(s.binding_id(), None);
        // The touch bar is part of the page.
        assert!(s.select_control(control(ControlKind::TouchBar)));
        assert!(s.page_selected);
        assert_eq!(s.selected, None);
        s.open_card(Some(GestureKind::SwipeLeft));
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
    fn the_first_key_is_the_selection_at_the_start_and_the_page_can_be_the_selection() {
        let now = Instant::now();
        let mut s = ViewState::default();
        s.apply(layout(), now);
        assert_eq!(s.selected_control, Some(control(ControlKind::Key)));
        // The key has no action, thus no gesture card is open (section 10.3).
        assert_eq!(s.gesture, None);
        assert!(s.select_page());
        assert!(!s.select_page());
        assert!(s.page_selected);
        // The swipe cards of the page edit the bindings of the touch bar.
        let touch_bar = Some(control(ControlKind::TouchBar));
        assert_eq!(
            (s.selected, s.selected_control, s.binding_id()),
            (None, touch_bar, None)
        );
        // A new layout keeps the page as the selection.
        s.apply(layout(), now);
        assert!(s.page_selected && s.selected_control == touch_bar);
        // A layout with no touch bar: the page has no swipe cards.
        let Event::Layout(mut no_bar) = layout() else {
            panic!()
        };
        Arc::make_mut(&mut no_bar)
            .controls
            .retain(|c| c.id.kind != ControlKind::TouchBar);
        s.apply(Event::Layout(no_bar), now);
        assert!(s.page_selected && s.selected_control.is_none());
        s.apply(layout(), now);
        assert_eq!(s.selected_control, touch_bar);
        // A control is then the selection, and the page is not.
        assert!(s.select_control(control(ControlKind::Knob)));
        assert!(!s.page_selected);
        assert!(s.select_page());
        assert!(s.select(bar()));
        assert!(!s.page_selected);
        // The first key is the key of the top row and of the first column,
        // not the first in the list.
        let Event::Layout(mut two) = layout() else {
            panic!()
        };
        let layout = Arc::make_mut(&mut two);
        let mut first = layout.controls[0];
        first.id = ControlId::new(ControlKind::Key, 7);
        first.surface = None;
        layout.controls[0].row = 2;
        layout.controls.push(first);
        let mut s = ViewState::default();
        s.apply(Event::Layout(two), now);
        assert_eq!(
            s.selected_control,
            Some(ControlId::new(ControlKind::Key, 7))
        );
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
