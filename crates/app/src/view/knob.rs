//! The inspector of a knob (decision 0029, section 10.4): the "Turn" card,
//! the cards "Turn left" and "Turn right" apart, the "Push" card, and the
//! section of the bar panel of the feedback of the knob.
//!
//! The "Turn" card shows the two bindings "Turn left" and "Turn right" as
//! one action. The shell writes the two bindings for each change. If the
//! kind has a direction description (decision 0014), the two bindings get
//! the two values of its direction field. These values are data from the
//! handler (rule 12): the shell has no code for a specific kind.

use super::*;
use crate::state::gestures;
use opencraft_engine::{
    Action, BindingId, DeviceLayout, Direction, GestureKind, InputType, KindDescription, Value,
};

/// The two turn gestures of a knob.
const TURNS: [GestureKind; 2] = [GestureKind::TurnLeft, GestureKind::TurnRight];

/// True for "Turn left" and "Turn right".
pub(super) fn is_turn(gesture: GestureKind) -> bool {
    TURNS.contains(&gesture)
}

/// The direction description of `kind`, if it has one.
fn direction<'a>(kinds: &'a [KindDescription], kind: &str) -> Option<&'a Direction> {
    kinds.iter().find(|k| k.kind == kind)?.direction.as_ref()
}

/// True if `action` has the text `value` in the field `key`.
fn has(action: &Action, key: &str, value: &str) -> bool {
    action.settings.get(key) == Some(&Value::Text(value.into()))
}

/// The turn actions of a knob as one pair (section 10.4). `Some(false)`:
/// the "Turn" card can show them, and a turn to the right has the value of
/// the right (for "Sound volume": right is up). `Some(true)`: the values
/// are the other way. `None`: they are not one pair, and the inspector
/// shows two cards. Two empty bindings are one pair.
pub(super) fn turn_pair(
    left: Option<&Action>,
    right: Option<&Action>,
    kinds: &[KindDescription],
) -> Option<bool> {
    let (left, right) = match (left, right) {
        (None, None) => return Some(false),
        (Some(left), Some(right)) if left.kind == right.kind => (left, right),
        _ => return None,
    };
    let Some(d) = direction(kinds, &right.kind) else {
        return (left.settings == right.settings).then_some(false);
    };
    let reversed = if has(right, &d.key, &d.right) && has(left, &d.key, &d.left) {
        false
    } else if has(right, &d.key, &d.left) && has(left, &d.key, &d.right) {
        true
    } else {
        return None;
    };
    let others = |action: &Action| {
        let mut settings = action.settings.clone();
        settings.remove(&d.key);
        settings
    };
    (others(left) == others(right)).then_some(reversed)
}

/// The action of the other turn of a pair: `action` with the other value
/// of the direction field. With no direction description, the same action.
pub(super) fn mirror(action: &Action, kinds: &[KindDescription]) -> Action {
    let mut other = action.clone();
    if let Some(d) = direction(kinds, &action.kind) {
        let value = if has(action, &d.key, &d.right) {
            &d.left
        } else if has(action, &d.key, &d.left) {
            &d.right
        } else {
            return other;
        };
        other
            .settings
            .insert(d.key.clone(), Value::Text(value.clone()));
    }
    other
}

/// The actions for "Turn right" and "Turn left" from `action`, with the
/// direction `reversed` (see [`turn_pair`]).
pub(super) fn turn_actions(
    action: &Action,
    kinds: &[KindDescription],
    reversed: bool,
) -> (Action, Action) {
    let mut right = action.clone();
    if let Some(d) = direction(kinds, &action.kind) {
        let value = if reversed { &d.left } else { &d.right };
        right
            .settings
            .insert(d.key.clone(), Value::Text(value.clone()));
    }
    let left = mirror(&right, kinds);
    (right, left)
}

impl DeviceView {
    fn turn_binding(&self, control: ControlId, gesture: GestureKind) -> BindingId {
        BindingId {
            page: self.state.page,
            control,
            gesture,
            folder: self.state.folder_of(control),
        }
    }

    /// The pair of the turn bindings of `control`, with the edits in
    /// flight. See [`turn_pair`].
    fn turn_pair_of(&self, control: ControlId) -> Option<bool> {
        let [left, right] = TURNS.map(|g| self.edit_action(self.turn_binding(control, g)));
        turn_pair(left.as_ref(), right.as_ref(), &self.state.kinds)
    }

    /// `Some(reversed)` if the inspector shows the one "Turn" card for the
    /// knob `control`: the turn bindings are one pair, and the user did not
    /// set them apart.
    pub(super) fn joined_turn(&self, control: ControlId) -> Option<bool> {
        if control.kind != ControlKind::Knob || self.state.turns_apart.contains(&control) {
            return None;
        }
        self.turn_pair_of(control)
    }

    /// The bindings that a new `action` of `binding` changes: the binding,
    /// and for the "Turn" card also the other turn, with the other value of
    /// the direction field.
    pub(super) fn turn_changes(
        &self,
        binding: BindingId,
        action: Action,
    ) -> Vec<(BindingId, Option<Action>)> {
        if !is_turn(binding.gesture) || self.joined_turn(binding.control).is_none() {
            return vec![(binding, Some(action))];
        }
        let other = TURNS
            .into_iter()
            .find(|g| *g != binding.gesture)
            .map(|g| self.turn_binding(binding.control, g));
        let mirrored = mirror(&action, &self.state.kinds);
        let mut changes = vec![(binding, Some(action))];
        changes.extend(other.map(|other| (other, Some(mirrored))));
        // "Turn right" first: then the target of the step is the same for
        // each change of one text edit (decision 0030).
        changes.sort_by_key(|(b, _)| b.gesture != GestureKind::TurnRight);
        changes
    }

    /// Set the "Turn" card of the knob `control` to a new action of `kind`,
    /// or remove the two actions with `None`. The two bindings change in
    /// one step of the history. The knob then shows the "Turn" card, open
    /// for a new action. The same kind keeps its settings.
    pub(super) fn set_turn(&mut self, control: ControlId, kind: Option<String>) {
        let [left, right] = TURNS.map(|g| self.turn_binding(control, g));
        let pair = self.turn_pair_of(control);
        let current = [left, right].map(|b| self.edit_action(b));
        self.state.turns_apart.remove(&control);
        self.state
            .open_card(kind.is_some().then_some(GestureKind::TurnRight));
        let (name, changes) = match kind {
            None => {
                if current.iter().all(Option::is_none) {
                    return;
                }
                ("Remove the Action", vec![(right, None), (left, None)])
            }
            Some(kind) => {
                let same = current
                    .iter()
                    .all(|a| a.as_ref().is_some_and(|a| a.kind == kind));
                if pair.is_some() && same {
                    return;
                }
                let Some(action) = self.default_action(&kind) else {
                    return;
                };
                let (r, l) = turn_actions(&action, &self.state.kinds, false);
                ("Set Action", vec![(right, Some(r)), (left, Some(l))])
            }
        };
        self.drop_edits(&[left, right]);
        self.send_bindings(name, changes);
    }

    /// Write the direction `reversed` to the two turn bindings of `control`.
    fn set_direction(&mut self, control: ControlId, reversed: bool) {
        let [left, right] = TURNS.map(|g| self.turn_binding(control, g));
        let Some(action) = self.edit_action(right) else {
            return;
        };
        let (r, l) = turn_actions(&action, &self.state.kinds, reversed);
        if Some(&r) == Some(&action) && self.edit_action(left).as_ref() == Some(&l) {
            return;
        }
        self.send_bindings("Set Action", vec![(right, Some(r)), (left, Some(l))]);
    }

    /// The name of the card that shows the binding of `gesture` in the
    /// inspector of `control`, for the status bar: "Turn" for a turn of
    /// the "Turn" card.
    pub(super) fn card_name(
        &self,
        control: ControlId,
        gesture: GestureKind,
    ) -> Option<&'static str> {
        if is_turn(gesture) && self.joined_turn(control).is_some() {
            return Some("Turn");
        }
        let binding = self.state.binding_of(gesture)?;
        gestures(binding.control.kind)
            .iter()
            .find(|(g, _)| *g == gesture)
            .map(|(_, name)| *name)
    }

    /// The gesture cards of a knob: "Turn" (or "Turn left" and "Turn
    /// right"), and "Push".
    pub(super) fn knob_cards(
        &self,
        control: ControlId,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let name = |gesture: GestureKind| {
            gestures(ControlKind::Knob)
                .iter()
                .find(|(g, _)| *g == gesture)
                .map_or("", |(_, name)| *name)
        };
        let mut cards = Vec::new();
        if let Some(reversed) = self.joined_turn(control) {
            // The card edits the binding of the open turn, or of "Turn
            // right" when it is closed.
            let owner = self
                .state
                .gesture
                .filter(|g| is_turn(*g))
                .unwrap_or(GestureKind::TurnRight);
            cards.push(self.gesture_card(
                control,
                owner,
                "Turn",
                "Turn",
                &move |view, action, available, window, cx| {
                    view.turn_rows(control, action, available, reversed, window, cx)
                },
                window,
                cx,
            ));
        } else {
            let joinable = self.turn_pair_of(control).is_some();
            for gesture in TURNS {
                cards.push(self.gesture_card(
                    control,
                    gesture,
                    &format!("{gesture:?}"),
                    name(gesture),
                    &move |view, action, available, window, cx| {
                        let mut rows = view.card_rows(action, available, window, cx);
                        if joinable {
                            rows.push(view.turn_link(control, false, cx));
                        }
                        rows
                    },
                    window,
                    cx,
                ));
            }
        }
        cards.push(self.gesture_card(
            control,
            GestureKind::Push,
            "Push",
            name(GestureKind::Push),
            &Self::card_rows,
            window,
            cx,
        ));
        cards
    }

    /// The body of the open "Turn" card: the "Direction" row in place of
    /// the direction field, the other fields, and the link "Set left and
    /// right apart".
    fn turn_rows(
        &self,
        control: ControlId,
        action: &Action,
        available: bool,
        reversed: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let replace = direction(&self.state.kinds, &action.kind)
            .filter(|_| available)
            .map(|d| {
                (
                    d.key.as_str(),
                    self.direction_row(control, d, &action.kind, reversed, cx),
                )
            });
        let mut rows = self.action_rows_with(&[], action, available, replace, window, cx);
        rows.push(self.turn_link(control, true, cx));
        rows
    }

    /// The row "Direction": "Right is *value*" and "Left is *value*", with
    /// the label of the option of a turn to the right in small letters.
    fn direction_row(
        &self,
        control: ControlId,
        direction: &Direction,
        kind: &str,
        reversed: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let value = self
            .state
            .kinds
            .iter()
            .find(|k| k.kind == kind)
            .and_then(|k| k.fields.iter().find(|f| f.key == direction.key))
            .and_then(|f| match &f.input {
                InputType::Choice { options } => options
                    .iter()
                    .find(|o| o.value == direction.right)
                    .map(|o| o.label.to_lowercase()),
                _ => None,
            })
            .unwrap_or_else(|| direction.right.clone());
        let segments = [
            ("turn-direction-right", format!("Right is {value}"), false),
            ("turn-direction-left", format!("Left is {value}"), true),
        ]
        .into_iter()
        .map(|(id, text, option)| {
            let selected = option == reversed;
            controls::segment(id, text, selected, theme::HEIGHT_SEGMENT_INSPECTOR)
                .min_w_0()
                .when(!selected, |d| {
                    d.on_click(cx.listener(move |view, _, window, cx| {
                        view.set_direction(control, option);
                        view.close_kinds();
                        view.focus_workspace(window, cx);
                        cx.notify();
                    }))
                })
        })
        .collect();
        form::row(
            "Direction",
            theme::FORM_LABEL,
            theme::CARD_FORM_GAP,
            // As the "Position" row: a track of full width with the border
            // `border-field`.
            controls::segmented("turn-direction", segments)
                .flex_1()
                .min_w_0()
                .border_1()
                .border_color(theme::BORDER_FIELD),
        )
        .into_any_element()
    }

    /// The last row of a turn card: "Set left and right apart" in the
    /// "Turn" card (`joined`), or "Join left and right" in the two cards.
    /// The bindings do not change.
    fn turn_link(&self, control: ControlId, joined: bool, cx: &mut Context<Self>) -> AnyElement {
        let (id, text) = if joined {
            ("turn-apart", "Set left and right apart")
        } else {
            ("turn-join", "Join left and right")
        };
        form::row(
            "",
            theme::FORM_LABEL,
            theme::CARD_FORM_GAP,
            controls::link(id, text).on_click(cx.listener(move |view, _, window, cx| {
                if joined {
                    view.state.turns_apart.insert(control);
                } else {
                    view.state.turns_apart.remove(&control);
                }
                view.close_kinds();
                view.sync_form(window, cx);
                view.focus_workspace(window, cx);
                cx.notify();
            })),
        )
        .into_any_element()
    }

    /// The section "Bar panel *N*" of the knob `control`: the image of the
    /// bar panel of its feedback, the rows of "Appearance", and the "Tap"
    /// card of the bar panel. A knob with no bar panel has no section.
    pub(super) fn knob_panel(
        &self,
        control: ControlId,
        layout: &DeviceLayout,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let panel = self.state.feedback_panel(control)?;
        let surface = layout
            .controls
            .iter()
            .find(|c| c.id == panel)
            .and_then(|c| c.surface)
            .and_then(|id| layout.surface(id))?;
        let width = theme::KNOB_PANEL_FACE;
        let height = width * (f32::from(surface.height) / f32::from(surface.width.max(1)));
        let face = div()
            .id("knob-panel-face")
            .debug_selector(|| "knob-panel-face".into())
            .flex_shrink_0()
            .w(width)
            .h(height)
            .rounded(theme::RADIUS_CARD)
            .overflow_hidden()
            .bg(theme::DEVICE_DISPLAY)
            .border_1()
            .border_color(theme::LINE_PANE)
            .children(
                self.images
                    .get(&surface.id)
                    .cloned()
                    .map(|image| img(image).size_full().rounded(theme::RADIUS_CARD)),
            );
        let tap = gestures(panel.kind).first().map(|&(gesture, name)| {
            self.gesture_card(
                panel,
                gesture,
                &format!("{gesture:?}"),
                name,
                &Self::card_rows,
                window,
                cx,
            )
        });
        let title = control_name(panel);
        Some(
            form::section("knob-panel", &title, None)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(theme::CARD_GAP)
                        .child(
                            form::form(theme::FORM_GAP)
                                .child(form::row("Face", theme::FORM_LABEL, theme::FORM_GAP, face))
                                .children(self.appearance_rows(surface.id, window, cx)),
                        )
                        .children(tap),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencraft_engine::{ChoiceOption, Field, Settings};

    fn volume() -> KindDescription {
        KindDescription {
            kind: "test.volume".into(),
            name: "Volume".into(),
            fields: vec![Field {
                key: "operation".into(),
                label: "Operation".into(),
                input: InputType::Choice {
                    options: ["up", "down", "mute"]
                        .map(|v| ChoiceOption {
                            value: v.into(),
                            label: v.into(),
                        })
                        .into(),
                },
                default: None,
            }],
            direction: Some(Direction {
                key: "operation".into(),
                right: "up".into(),
                left: "down".into(),
            }),
            ..Default::default()
        }
    }

    fn action(kind: &str, operation: &str, step: i64) -> Action {
        Action {
            kind: kind.into(),
            settings: Settings::from([
                ("operation".into(), Value::Text(operation.into())),
                ("step".into(), Value::Integer(step)),
            ]),
        }
    }

    #[test]
    fn two_turns_are_one_pair_with_the_direction_values_or_the_same_settings() {
        let kinds = [volume()];
        let pair = |left: Option<Action>, right: Option<Action>| {
            turn_pair(left.as_ref(), right.as_ref(), &kinds)
        };
        let v = |op, step| Some(action("test.volume", op, step));
        assert_eq!(pair(None, None), Some(false));
        assert_eq!(pair(v("down", 5), v("up", 5)), Some(false));
        assert_eq!(pair(v("up", 5), v("down", 5)), Some(true));
        // A different setting, the same value, a value that is not a
        // direction, or one empty binding: two cards.
        assert_eq!(pair(v("down", 5), v("up", 6)), None);
        assert_eq!(pair(v("up", 5), v("up", 5)), None);
        assert_eq!(pair(v("mute", 5), v("up", 5)), None);
        assert_eq!(pair(None, v("up", 5)), None);
        // A kind with no direction: the same settings.
        let o = |op, step| Some(action("test.other", op, step));
        assert_eq!(pair(o("a", 1), o("a", 1)), Some(false));
        assert_eq!(pair(o("a", 1), o("b", 1)), None);
        assert_eq!(pair(o("a", 1), v("up", 1)), None);
    }

    #[test]
    fn the_turn_actions_have_the_two_direction_values() {
        let kinds = [volume()];
        let base = action("test.volume", "mute", 3);
        let (right, left) = turn_actions(&base, &kinds, false);
        assert_eq!(right, action("test.volume", "up", 3));
        assert_eq!(left, action("test.volume", "down", 3));
        let (right, left) = turn_actions(&base, &kinds, true);
        assert_eq!(right, action("test.volume", "down", 3));
        assert_eq!(left, action("test.volume", "up", 3));
        assert_eq!(mirror(&left, &kinds), right);
        // No direction: the same action for the two turns.
        let other = action("test.other", "x", 1);
        assert_eq!(turn_actions(&other, &kinds, true), (other.clone(), other));
    }
}
