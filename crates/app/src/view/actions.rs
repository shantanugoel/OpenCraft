//! The gesture cards of the selected control, and the settings form of the
//! open card, made only from engine descriptions (decision 0029, section
//! 10.3).
//!
//! A field of the input type Action or Action list contains actions. The form
//! shows each contained action with the same code as the action of the
//! binding, to each depth that the settings have (decision 0017). The form
//! reads and writes the value of a contained action only with the functions
//! of the engine: see [`Action::from_value`].

use super::gesture_card::{menu_button_height, GestureCard, KindLabel};
use super::*;
use crate::state::gestures;
use gpui_kit::{
    assets::IconName,
    base::TestTextExt,
    component::{
        input::{AnyInputState, TextareaState},
        Icon, WindowExt,
    },
    KeyDownEvent, Pixels,
};
use opencraft_engine::{
    Action, BindingId, ChoiceOption, Field, GestureKind, InputType, KindDescription, Status, Value,
};

/// The function that makes the body rows of an open gesture card from its
/// action, and true if the action has a handler.
pub(super) type CardRows<'a> =
    &'a dyn Fn(&DeviceView, &Action, bool, &Window, &mut Context<DeviceView>) -> Vec<AnyElement>;

/// One step from an action to an action that it contains: the key of a
/// field, and for an action list, the position in the list.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Slot {
    pub(super) key: String,
    pub(super) item: Option<usize>,
}

/// The steps from the action of the binding to a contained action. It is
/// empty for the action of the binding.
pub(super) type ActionPath = Vec<Slot>;

/// A field of the form: the path of its action, and the key of the field.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FieldPath {
    pub(super) action: ActionPath,
    pub(super) key: String,
}

/// A text input of the form. It belongs to a field of an action of one kind.
pub(super) struct FieldInput {
    kind: String,
    long: bool,
    pub(super) state: AnyInputState,
    _subscription: Subscription,
}

/// The menu that is open in the gesture cards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Menu {
    /// The kind menu of the gesture card of this gesture.
    Card(GestureKind),
    /// The kind menu of the contained action at the path. The path is not
    /// empty.
    Replace(ActionPath),
    /// The kind menu that adds an action at the end of the list in the field
    /// `key` of the action at the path.
    Add(ActionPath, String),
    /// The options of the choice field at the path.
    Choice(FieldPath),
}

#[derive(Default)]
pub(super) struct ActionForm {
    /// The edit epoch and the binding of the inputs. The inputs are only for
    /// an available binding.
    owner: Option<(u64, BindingId)>,
    pub(super) inputs: BTreeMap<FieldPath, FieldInput>,
    // These are edits in flight, not confirmed bindings. Only events change ViewState.
    pending: BTreeMap<BindingId, Vec<Action>>,
    pub(super) recording: Option<FieldPath>,
    modifier_recorder: super::hotkey::ModifierRecorder,
    menu: Option<Menu>,
    /// The open nested cards and the step menu (`nested`).
    pub(super) nested: super::nested::NestedState,
    choices: BTreeMap<FieldPath, (Action, u64, Vec<ChoiceOption>)>,
}

impl ActionForm {
    pub(super) fn accept_choices(&mut self, id: u64, options: Vec<ChoiceOption>) {
        if let Some((_, _, current)) = self
            .choices
            .values_mut()
            .find(|(_, request, _)| *request == id)
        {
            *current = options;
        }
    }
}

/// The text form of a path for the identifiers of the elements. It is empty
/// for the action of the binding, so the identifiers of its form stay short.
pub(super) fn prefix(path: &[Slot]) -> String {
    path.iter()
        .map(|slot| match slot.item {
            Some(item) => format!("{}.{item}/", slot.key),
            None => format!("{}/", slot.key),
        })
        .collect()
}

/// The status bar text after a click on "Test the action", for example
/// "Testing “Open a URL” of Key 8 for Press". The result of the action comes
/// from its handler: a failure shows as a message of the engine.
fn test_text(control: ControlId, kind: &str, gesture: &str) -> String {
    format!(
        "Testing “{kind}” of {} for {gesture}",
        control_name(control)
    )
}

pub(super) fn child(path: &[Slot], key: &str, item: Option<usize>) -> ActionPath {
    let mut path = path.to_vec();
    path.push(Slot {
        key: key.into(),
        item,
    });
    path
}

/// The action at `path` in `action`.
pub(super) fn contained(action: &Action, path: &[Slot]) -> Option<Action> {
    let Some((slot, rest)) = path.split_first() else {
        return Some(action.clone());
    };
    let value = action.settings.get(&slot.key)?;
    let inner = match slot.item {
        None => Action::from_value(value)?,
        Some(item) => Action::list_from_value(value)?.into_iter().nth(item)?,
    };
    contained(&inner, rest)
}

/// Change the action at `path` in `action`, and write each action on the
/// path again. Nothing changes if no action is at `path`.
pub(super) fn change(action: &mut Action, path: &[Slot], f: impl FnOnce(&mut Action)) {
    let Some((slot, rest)) = path.split_first() else {
        f(action);
        return;
    };
    let Some(value) = action.settings.get(&slot.key) else {
        return;
    };
    let value = match slot.item {
        None => {
            let Some(mut inner) = Action::from_value(value) else {
                return;
            };
            change(&mut inner, rest, f);
            inner.to_value()
        }
        Some(item) => {
            let Some(mut list) = Action::list_from_value(value) else {
                return;
            };
            let Some(inner) = list.get_mut(item) else {
                return;
            };
            change(inner, rest, f);
            Action::list_to_value(&list)
        }
    };
    action.settings.insert(slot.key.clone(), value);
}

/// The actions in the field `key`. A missing value or a value of a
/// different form gives an empty list.
pub(super) fn list(action: &Action, key: &str) -> Vec<Action> {
    action
        .settings
        .get(key)
        .and_then(Action::list_from_value)
        .unwrap_or_default()
}

impl DeviceView {
    pub(super) fn kind(&self, kind: &str) -> Option<&KindDescription> {
        self.state.kinds.iter().find(|k| k.kind == kind)
    }

    /// A new action of `kind` with the default value of each field.
    pub(super) fn default_action(&self, kind: &str) -> Option<Action> {
        let kind = self.kind(kind)?;
        Some(Action {
            kind: kind.kind.clone(),
            settings: kind
                .fields
                .iter()
                .filter_map(|f| f.default.clone().map(|v| (f.key.clone(), v)))
                .collect(),
        })
    }

    /// The binding of the inputs, if they are for the page that the device
    /// shows now.
    pub(super) fn form_binding(&self) -> Option<BindingId> {
        self.form
            .owner
            .filter(|(epoch, binding)| {
                *epoch == self.state.epoch && Some(*binding) == self.state.binding_id()
            })
            .map(|(_, binding)| binding)
    }

    pub(super) fn sync_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let owner = self
            .state
            .binding_id()
            .filter(|id| self.state.bindings.get(id).is_some_and(|b| b.available))
            .map(|id| (self.state.epoch, id));
        if self.form.owner != owner {
            self.form.inputs.clear();
            self.form.choices.clear();
            self.form.recording = None;
            self.form.menu = None;
            self.form.nested = Default::default();
            self.form.owner = owner;
        }
        let Some((_, binding)) = owner else {
            return;
        };
        let Some(action) = self.edit_action(binding) else {
            return;
        };
        let mut choices = Vec::new();
        self.choice_fields(&action, &[], &mut choices);
        self.form
            .choices
            .retain(|path, (action, _, _)| choices.iter().any(|(p, a)| p == path && a == action));
        for (path, action) in choices {
            if !self.form.choices.contains_key(&path) {
                // Shared by all forms: a reply cannot enter a different window.
                static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                (self.send)(Command::Choices {
                    id,
                    action: action.clone(),
                    field: path.key.clone(),
                });
                self.form.choices.insert(path, (action, id, Vec::new()));
            }
        }
        let mut wanted = Vec::new();
        self.text_fields(&action, &[], &mut wanted);
        self.form.inputs.retain(|path, input| {
            wanted
                .iter()
                .any(|(p, kind, long, _)| p == path && *kind == input.kind && *long == input.long)
        });
        for (path, kind, long, value) in wanted {
            if !self.form.inputs.contains_key(&path) {
                let input = self.text_input(owner.unwrap(), path.clone(), long, window, cx);
                self.form.inputs.insert(
                    path.clone(),
                    FieldInput {
                        kind,
                        long,
                        state: input.0,
                        _subscription: input.1,
                    },
                );
            }
            let input = &self.form.inputs[&path].state;
            if input.value(cx).as_ref() == value {
                continue;
            }
            match input {
                AnyInputState::Input(input) => {
                    input.update(cx, |input, cx| input.set_value(value, window, cx))
                }
                AnyInputState::Textarea(input) => {
                    input.update(cx, |input, cx| input.set_value(value, window, cx))
                }
                _ => {}
            }
        }
        if self
            .form
            .recording
            .as_ref()
            .is_some_and(|path| contained(&action, &path.action).is_none())
        {
            self.form.recording = None;
        }
        self.sync_state_faces(window, cx);
    }

    /// Find dynamic fields at each depth, without names of built-in kinds.
    fn choice_fields(&self, action: &Action, path: &[Slot], out: &mut Vec<(FieldPath, Action)>) {
        let Some(kind) = self.kind(&action.kind) else {
            return;
        };
        for field in &kind.fields {
            match field.input {
                InputType::RuntimeChoice | InputType::RuntimeChoiceList => out.push((
                    FieldPath {
                        action: path.to_vec(),
                        key: field.key.clone(),
                    },
                    action.clone(),
                )),
                InputType::Action => {
                    if let Some(inner) =
                        action.settings.get(&field.key).and_then(Action::from_value)
                    {
                        self.choice_fields(&inner, &child(path, &field.key, None), out);
                    }
                }
                InputType::ActionList | InputType::TimedActionList => {
                    for (item, inner) in list(action, &field.key).iter().enumerate() {
                        self.choice_fields(inner, &child(path, &field.key, Some(item)), out);
                    }
                }
                _ => {}
            }
        }
    }

    /// Each field of `action` and of its contained actions that has a text
    /// input: its path, the kind of its action, true for a long text, and
    /// its value as text.
    fn text_fields(
        &self,
        action: &Action,
        path: &[Slot],
        out: &mut Vec<(FieldPath, String, bool, String)>,
    ) {
        let Some(kind) = self.kind(&action.kind) else {
            return;
        };
        if let Some(field) = self.step_delay_field(path) {
            out.push((
                FieldPath {
                    action: path.to_vec(),
                    key: field.key.clone(),
                },
                action.kind.clone(),
                false,
                value_text(action.settings.get(&field.key).or(field.default.as_ref())),
            ));
        }
        for field in &kind.fields {
            let value = action.settings.get(&field.key);
            match field.input {
                InputType::Choice { .. }
                | InputType::RuntimeChoice
                | InputType::RuntimeChoiceList
                | InputType::Toggle
                | InputType::Hotkey => {}
                InputType::Action => {
                    if let Some(inner) = value.and_then(Action::from_value) {
                        self.text_fields(&inner, &child(path, &field.key, None), out);
                    }
                }
                InputType::ActionList | InputType::TimedActionList => {
                    for (item, inner) in list(action, &field.key).iter().enumerate() {
                        self.text_fields(inner, &child(path, &field.key, Some(item)), out);
                    }
                }
                _ => out.push((
                    FieldPath {
                        action: path.to_vec(),
                        key: field.key.clone(),
                    },
                    action.kind.clone(),
                    matches!(field.input, InputType::LongText),
                    value_text(value),
                )),
            }
        }
    }

    /// Timing is a property of the parent list, not of the child's kind.
    pub(super) fn step_delay_field(&self, path: &[Slot]) -> Option<Field> {
        let (slot, parent_path) = path.split_last()?;
        let item = slot.item?;
        let root = self.edit_action(self.form_binding()?)?;
        let parent = contained(&root, parent_path)?;
        let field = self
            .kind(&parent.kind)?
            .fields
            .iter()
            .find(|f| f.key == slot.key)?;
        if field.input != InputType::TimedActionList {
            return None;
        }
        let default = if item == 0 {
            0
        } else {
            match parent.settings.get("delay_ms") {
                Some(Value::Integer(ms)) => *ms,
                _ => 100,
            }
        };
        Some(Field::step_delay(default))
    }

    fn text_input(
        &mut self,
        owner: (u64, BindingId),
        path: FieldPath,
        long: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (AnyInputState, Subscription) {
        if long {
            let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 7));
            let subscription = cx.subscribe_in(&input, window, move |view, input, event, _, cx| {
                view.text_edit_event(input.entity_id(), event, cx);
                if matches!(event, TextEvent::Change) {
                    view.text_field(owner, &path, input.read(cx).value().to_string());
                }
            });
            return (input.into(), subscription);
        }
        let input = cx.new(|cx| InputState::new(window, cx));
        let subscription =
            cx.subscribe_in(&input, window, move |view, input, event, window, cx| {
                view.text_edit_event(input.entity_id(), event, cx);
                if matches!(event, TextEvent::Change) {
                    let text = input.read(cx).value().to_string();
                    if let Some(Value::Integer(value)) = view.text_field(owner, &path, text.clone())
                    {
                        // Show the clamped value, but allow a temporary empty or minus input.
                        if value.to_string() != text {
                            input.update(cx, |input, cx| {
                                input.set_value(value.to_string(), window, cx)
                            });
                        }
                    }
                }
            });
        (input.into(), subscription)
    }

    pub(super) fn edit_action(&self, id: BindingId) -> Option<Action> {
        self.form
            .pending
            .get(&id)
            .and_then(|p| p.last())
            .cloned()
            .or_else(|| self.state.bindings.get(&id)?.action.clone())
    }

    pub(super) fn acknowledge(&mut self, binding: &opencraft_engine::BindingState) {
        if let Some(pending) = self.form.pending.get_mut(&binding.binding) {
            if let Some(index) = pending
                .iter()
                .position(|a| Some(a) == binding.action.as_ref())
            {
                pending.drain(..=index);
            } else {
                pending.clear();
            }
        }
    }

    /// An edit in flight is for a binding that a page list event removes.
    pub(super) fn forget_pending(&mut self) {
        self.form.pending.clear();
    }

    /// The value of a text input changed. `owner` is the binding of the
    /// input: if the form is for a different binding or page now, the text
    /// goes nowhere.
    fn text_field(
        &mut self,
        owner: (u64, BindingId),
        path: &FieldPath,
        text: String,
    ) -> Option<Value> {
        if self.form.owner != Some(owner) || self.form_binding() != Some(owner.1) {
            return None;
        }
        let action = contained(&self.edit_action(owner.1)?, &path.action)?;
        let timing = self.step_delay_field(&path.action);
        let field = timing.as_ref().filter(|f| f.key == path.key).or_else(|| {
            self.kind(&action.kind)?
                .fields
                .iter()
                .find(|f| f.key == path.key)
        })?;
        let value = match field.input {
            InputType::Colour => {
                let colour = super::inspector::parse_colour(&text)?;
                Value::Text(super::inspector::colour_text(colour))
            }
            InputType::Integer { min, max } => {
                let digits = text.strip_prefix(['-', '+']).unwrap_or(&text);
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                let number = text.parse::<i64>().unwrap_or(if text.starts_with('-') {
                    i64::MIN
                } else {
                    i64::MAX
                });
                Value::Integer(number.clamp(min.min(max), min.max(max)))
            }
            _ => Value::Text(text),
        };
        self.set_field(owner.1, path, value.clone());
        Some(value)
    }

    /// Change the edit action of `binding`, then send the full action to the
    /// engine. A change that changes nothing sends nothing.
    pub(super) fn edit(&mut self, binding: BindingId, f: impl FnOnce(&mut Action)) {
        self.edit_as(binding, None, f);
    }

    /// As [`Self::edit`]. With a `name`, the change is one step of the
    /// history with that name, for example "Add Step" (decision 0030).
    pub(super) fn edit_as(
        &mut self,
        binding: BindingId,
        name: Option<&str>,
        f: impl FnOnce(&mut Action),
    ) {
        let Some(before) = self.edit_action(binding) else {
            return;
        };
        let mut action = before.clone();
        f(&mut action);
        if action == before {
            return;
        }
        // The "Turn" card of a knob writes the two turn bindings.
        let changes = self.turn_changes(binding, action);
        match name {
            Some(name) => self.send_bindings_as(name, changes, true),
            None => self.send_bindings("Set Action", changes),
        }
    }

    /// Send the new actions of bindings, and keep each as an edit in
    /// flight. One binding is one `SetBinding`. More bindings are one
    /// `Command::Group` with the name `name`, thus one step of the history
    /// (decision 0030).
    pub(super) fn send_bindings(&mut self, name: &str, changes: Vec<(BindingId, Option<Action>)>) {
        self.send_bindings_as(name, changes, false);
    }

    /// As [`Self::send_bindings`]. With `group_one`, one binding is also a
    /// `Command::Group`, so that its step of the history has the name
    /// `name`, for example "Add Step".
    fn send_bindings_as(
        &mut self,
        name: &str,
        changes: Vec<(BindingId, Option<Action>)>,
        group_one: bool,
    ) {
        let mut commands: Vec<Command> = changes
            .into_iter()
            .map(|(binding, action)| {
                match &action {
                    Some(action) => self
                        .form
                        .pending
                        .entry(binding)
                        .or_default()
                        .push(action.clone()),
                    None => {
                        self.form.pending.remove(&binding);
                    }
                }
                Command::SetBinding { binding, action }
            })
            .collect();
        match commands.len() {
            0 => {}
            1 if !group_one => (self.send)(commands.remove(0)),
            _ => (self.send)(Command::Group {
                name: name.into(),
                commands,
            }),
        }
    }

    /// Forget the edits in flight of `bindings`: a new kind replaces them.
    pub(super) fn drop_edits(&mut self, bindings: &[BindingId]) {
        for binding in bindings {
            self.form.pending.remove(binding);
        }
    }

    fn set_field(&mut self, binding: BindingId, path: &FieldPath, value: Value) {
        let key = path.key.clone();
        self.edit(binding, |root| {
            change(root, &path.action, |action| {
                action.settings.insert(key, value);
            })
        });
    }

    /// Open or close a menu of the cards. The focus goes back to the root
    /// element, so that Escape closes the menu.
    pub(super) fn toggle_menu(&mut self, menu: Menu, window: &mut Window, cx: &mut Context<Self>) {
        let open = self.form.menu.as_ref() != Some(&menu);
        self.close_kinds();
        if open {
            self.form.menu = Some(menu);
        }
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// Open or close the gesture card of `gesture`. The card that was open
    /// closes.
    fn toggle_card(&mut self, gesture: GestureKind, window: &mut Window, cx: &mut Context<Self>) {
        let open = self.state.gesture != Some(gesture);
        self.state.open_card(open.then_some(gesture));
        self.close_kinds();
        self.sync_form(window, cx);
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// Select a kind in the open kind menu. `None` is "Remove the action".
    fn choose(
        &mut self,
        menu: Menu,
        kind: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.form.menu = None;
        // The click was on the menu, thus no text field keeps the focus. The
        // text edit ends first, so that the change is a step of its own.
        self.focus_workspace(window, cx);
        self.choose_in(menu, kind);
        // A contained action can have new text fields now.
        self.sync_form(window, cx);
        cx.notify();
    }

    fn choose_in(&mut self, menu: Menu, kind: Option<String>) {
        match menu {
            Menu::Card(gesture) => self.choose_card(gesture, kind),
            Menu::Choice(_) => {}
            Menu::Replace(path) => {
                let Some(binding) = self.form_binding() else {
                    return;
                };
                let current = self.edit_action(binding).and_then(|a| contained(&a, &path));
                // Selecting the same kind must not reset its settings to defaults.
                if current
                    .as_ref()
                    .is_some_and(|a| Some(&a.kind) == kind.as_ref())
                {
                    return;
                }
                let mut new = kind.and_then(|kind| self.default_action(&kind));
                if let (Some(field), Some(new)) = (self.step_delay_field(&path), new.as_mut()) {
                    if let Some(value) = current
                        .as_ref()
                        .and_then(|a| a.settings.get(&field.key))
                        .cloned()
                        .or(field.default)
                    {
                        new.settings.insert(field.key, value);
                    }
                }
                let Some((slot, parent)) = path.split_last() else {
                    return;
                };
                // A new action opens its card (section 10.3).
                if new.is_some() {
                    self.open_nested(&path);
                }
                let name = new.is_none().then_some("Remove the Action");
                self.edit_as(binding, name, |root| {
                    change(root, parent, |action| match slot.item {
                        None => {
                            match new {
                                Some(new) => {
                                    action.settings.insert(slot.key.clone(), new.to_value())
                                }
                                None => action.settings.remove(&slot.key),
                            };
                        }
                        Some(item) => {
                            let mut items = list(action, &slot.key);
                            if let (Some(new), Some(old)) = (new, items.get_mut(item)) {
                                *old = new;
                                action
                                    .settings
                                    .insert(slot.key.clone(), Action::list_to_value(&items));
                            }
                        }
                    })
                });
            }
            Menu::Add(path, key) => {
                let Some(binding) = self.form_binding() else {
                    return;
                };
                let Some(mut new) = kind.and_then(|kind| self.default_action(&kind)) else {
                    return;
                };
                let Some(count) = self
                    .edit_action(binding)
                    .and_then(|a| contained(&a, &path))
                    .map(|a| list(&a, &key).len())
                else {
                    return;
                };
                if let Some(field) = self.step_delay_field(&child(&path, &key, Some(count))) {
                    if let Some(value) = field.default {
                        new.settings.insert(field.key, value);
                    }
                }
                // The new step opens (section 10.3).
                self.open_nested(&child(&path, &key, Some(count)));
                self.edit_as(binding, Some("Add Step"), |root| {
                    change(root, &path, |action| {
                        let mut items = list(action, &key);
                        items.push(new);
                        action.settings.insert(key, Action::list_to_value(&items));
                    })
                });
            }
        }
    }

    /// Set the action of the gesture card of `gesture` to a new action of
    /// `kind`, or remove it with `None`. A new action opens the card; the
    /// removal closes it. The engine makes the new binding, so no edit is in
    /// flight after this.
    pub(super) fn choose_card(&mut self, gesture: GestureKind, kind: Option<String>) {
        let Some(binding) = self.state.binding_of(gesture) else {
            return;
        };
        if super::knob::is_turn(gesture) && self.joined_turn(binding.control).is_some() {
            self.set_turn(binding.control, kind);
            return;
        }
        self.state.open_card(kind.is_some().then_some(gesture));
        let current = self
            .state
            .bindings
            .get(&binding)
            .and_then(|b| b.action.as_ref());
        // The same kind keeps its settings.
        let same = match (&kind, current) {
            (Some(kind), Some(current)) => current.kind == *kind,
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        let action = kind.and_then(|kind| self.default_action(&kind));
        self.form.pending.remove(&binding);
        (self.send)(Command::SetBinding { binding, action });
    }

    fn path_field(&mut self, path: FieldPath, window: &mut Window, cx: &mut Context<Self>) {
        let Some(binding) = self.form_binding() else {
            return;
        };
        let owner = (self.state.epoch, binding);
        let Some(kind) = self
            .edit_action(binding)
            .and_then(|a| contained(&a, &path.action))
            .map(|a| a.kind)
        else {
            return;
        };
        // This toolkit has no initial-directory option. Application bundles are files.
        // A folder is also a valid target of a path field ("Open an application").
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = paths.await;
            let _ = view.update_in(cx, |view, window, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        // Do not replace a different action, or an action of a
                        // different page, after a delayed dialog.
                        let same = owner.0 == view.state.epoch
                            && view
                                .edit_action(binding)
                                .and_then(|a| contained(&a, &path.action))
                                .is_some_and(|a| a.kind == kind);
                        if let Some(path_name) = paths.first().filter(|_| same) {
                            view.set_field(
                                binding,
                                &path,
                                Value::Text(path_name.to_string_lossy().into_owned()),
                            );
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => view
                        .state
                        .problems
                        .push("OpenCraft could not open the file dialog.".into()),
                }
                view.sync_form(window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn record_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The recorder and the open menus get the keys of the root element
        // and of a control with the keyboard focus. A key in a text field is
        // text.
        if window.focused_input(cx).is_some() {
            return;
        }
        let escape = event.keystroke.key == "escape"
            && event.keystroke.modifiers == gpui_kit::Modifiers::default();
        if escape
            && (self.form.menu.is_some() || self.inspector.is_open() || self.device.timer_open)
        {
            cx.stop_propagation();
            self.close_kinds();
            cx.notify();
            return;
        }
        let Some(path) = self.form.recording.clone() else {
            return;
        };
        cx.stop_propagation();
        if let Some(value) = super::hotkey::hotkey(&event.keystroke) {
            if let Some(binding) = self.form_binding() {
                self.set_field(binding, &path, Value::Text(value));
            }
            self.form.recording = None;
        } else if escape {
            self.form.recording = None;
        }
        cx.notify();
    }

    pub(super) fn record_modifiers(
        &mut self,
        event: &gpui_kit::ModifiersChangedEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self.form.recording.clone() else {
            return;
        };
        cx.stop_propagation();
        if let Some(value) = self.form.modifier_recorder.changed(event.modifiers) {
            if let Some(binding) = self.form_binding() {
                self.set_field(binding, &path, Value::Text(value));
            }
            self.form.recording = None;
            cx.notify();
        }
    }

    /// Close the open menu of the cards, the menus and popovers of the
    /// "Appearance" section, and the popover of a knob light colour.
    pub(super) fn close_kinds(&mut self) {
        self.form.menu = None;
        self.inspector.close();
        self.device.colour_open = None;
        self.device.timer_open = false;
    }

    /// The label of the popup of `action`.
    pub(super) fn kind_label(&self, action: &Action, available: bool) -> KindLabel {
        match self.kind(&action.kind) {
            Some(kind) if available => KindLabel::new(kind),
            _ => KindLabel::unavailable(),
        }
    }

    /// One gesture card for each gesture of `control`, in the order of
    /// `state::gestures`. The open card shows the form of its action.
    pub(super) fn gesture_cards(
        &self,
        control: ControlId,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        gestures(control.kind)
            .iter()
            .map(|&(gesture, name)| {
                self.gesture_card(
                    control,
                    gesture,
                    &format!("{gesture:?}"),
                    name,
                    &Self::card_rows,
                    window,
                    cx,
                )
            })
            .collect()
    }

    /// The body rows of an open gesture card: the form of its action.
    pub(super) fn card_rows(
        &self,
        action: &Action,
        available: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        self.action_rows(&[], action, available, window, cx)
    }

    /// The gesture card of the binding of `gesture` of `control` on the
    /// active page. The identifiers start with `gesture-{id}`, and `name`
    /// is the gesture name. `rows` makes the body rows of the open card.
    /// The "Turn" card of a knob (`knob`) uses this card with the binding
    /// of one turn and its own rows.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn gesture_card(
        &self,
        control: ControlId,
        gesture: GestureKind,
        id: &str,
        name: &str,
        rows: CardRows,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let binding = BindingId {
            page: self.state.page,
            control,
            gesture,
        };
        let menu = Menu::Card(gesture);
        let current = self.state.bindings.get(&binding);
        let toggle = menu.clone();
        // A swipe has the wider name column of section 10.6.
        let name_width = if control.kind == ControlKind::TouchBar {
            theme::GESTURE_NAME_WIDE
        } else {
            theme::FORM_LABEL
        };
        let card =
            GestureCard::new(format!("gesture-{id}"), name.to_string())
                .name_width(name_width)
                .menu(self.kind_menu(
                    "kind",
                    &menu,
                    current.is_some(),
                    menu_button_height(current.is_some(), false),
                    window,
                    cx,
                ))
                .on_menu(cx.listener(move |view, _, window, cx| {
                    view.toggle_menu(toggle.clone(), window, cx)
                }));
        let Some(current) = current else {
            return card.into_any_element();
        };
        let open = self.state.gesture == Some(gesture);
        // The open card shows the edits in flight.
        let action = if open {
            self.edit_action(binding)
        } else {
            current.action.clone()
        };
        let Some(action) = action else {
            return card.into_any_element();
        };
        let (lead, rows) = if open {
            (
                self.step_lists(&[], &action, current.available, window, cx),
                rows(self, &action, current.available, window, cx),
            )
        } else {
            (None, Vec::new())
        };
        let label = self.kind_label(&action, current.available);
        let mut card = card.action(label.clone()).open(open).lead(lead).on_toggle(
            cx.listener(move |view, _, window, cx| view.toggle_card(gesture, window, cx)),
        );
        // An action with no handler cannot run (rule 8).
        if current.available {
            let name = name.to_string();
            card = card.on_test(cx.listener(move |view, _, window, cx| {
                let text = test_text(control, label.name(), &name);
                (view.send)(Command::TestAction { binding });
                view.show_message(text, window, cx);
            }));
        }
        card.children(rows).into_any_element()
    }

    /// The kind menu `menu`, if it is open: one row for each kind, and with
    /// `remove`, "Remove the action". `id` is the start of the identifier of
    /// each row.
    pub(super) fn kind_menu(
        &self,
        id: &str,
        menu: &Menu,
        remove: bool,
        button_height: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if self.form.menu.as_ref() != Some(menu) {
            return None;
        }
        // One section for each group of the library, with the group name
        // as its title (section 10.3).
        let mut rows: Vec<AnyElement> = Vec::new();
        let groups = super::library::groups(&self.state.kinds, |_| true);
        for (number, (group, members)) in groups.into_iter().enumerate() {
            let selector = format!("{id}-group-{number}");
            rows.push(
                controls::group_title(group)
                    .debug_selector(move || selector.clone())
                    .when(number > 0, |d| d.mt(theme::SPACE_6))
                    .into_any_element(),
            );
            for index in members {
                let k = &self.state.kinds[index];
                let kind = k.kind.clone();
                let menu = menu.clone();
                rows.push(
                    controls::menu_row(format!("{id}-{index}"))
                        .child(KindLabel::new(k).content(false))
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.choose(menu.clone(), Some(kind.clone()), window, cx)
                        }))
                        .into_any_element(),
                );
            }
        }
        if remove {
            let menu = menu.clone();
            rows.push(controls::menu_separator().into_any_element());
            rows.push(
                controls::menu_row(format!("{id}-remove"))
                    .child("Remove the action")
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.choose(menu.clone(), None, window, cx)
                    }))
                    .into_any_element(),
            );
        }
        Some(controls::popup_menu(
            format!("{id}-menu"),
            controls::MenuPlace::below(button_height),
            rows,
            window,
            cx.listener(|view, _, _, cx| {
                view.close_kinds();
                cx.notify();
            }),
        ))
    }

    /// The body rows of a card with `action` at `path`: the form of the
    /// fields, or the text of an action with no handler, and the request of
    /// the handler for the user. A field of the input type Action list is
    /// not a row: it is a step list above the form (`nested`). The label
    /// of the state On is not a row when the appearance of each state
    /// shows it (`states`).
    pub(super) fn action_rows(
        &self,
        path: &[Slot],
        action: &Action,
        available: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        self.action_rows_with(path, action, available, None, window, cx)
    }

    /// The rows of [`Self::action_rows`]. With `replace`, the field with
    /// that key shows the given row in place of its own control: the
    /// "Direction" row of the "Turn" card of a knob.
    pub(super) fn action_rows_with(
        &self,
        path: &[Slot],
        action: &Action,
        available: bool,
        mut replace: Option<(&str, AnyElement)>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let id = prefix(path);
        let mut rows = Vec::new();
        if let Some(field) = self.step_delay_field(path) {
            rows.push(self.field_row(
                path,
                usize::MAX,
                &field,
                action.settings.get(&field.key).or(field.default.as_ref()),
                window,
                cx,
            ));
        }
        if !available {
            rows.push(
                div()
                    .id(format!("{id}unavailable"))
                    .debug_selector(move || format!("{id}unavailable"))
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_6)
                    .text_size(theme::TEXT_LABEL)
                    .child(
                        div()
                            .text_color(theme::TEXT_TERTIARY)
                            .child(action.kind.clone()),
                    )
                    .child(div().text_color(theme::WARNING).child(
                        "This version of OpenCraft does not have this action. The binding stays \
                         in the profile.",
                    ))
                    .into_any_element(),
            );
        } else if let Some(kind) = self.kind(&action.kind) {
            let on_label = self.on_label_in_faces(path);
            for (index, field) in kind.fields.iter().enumerate() {
                if field.input.is_action_list() || on_label.as_ref() == Some(&field.key) {
                    continue;
                }
                if replace.as_ref().is_some_and(|(key, _)| *key == field.key) {
                    if let Some((_, row)) = replace.take() {
                        rows.push(row);
                    }
                    continue;
                }
                rows.push(self.field_row(
                    path,
                    index,
                    field,
                    action.settings.get(&field.key),
                    window,
                    cx,
                ));
            }
        }
        if let Some(Status::NeedsUser {
            text,
            button: label,
        }) = self.state.statuses.get(&action.kind)
        {
            let kind = action.kind.clone();
            rows.push(
                div()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::WARNING)
                    .child(text.clone())
                    .into_any_element(),
            );
            rows.push(
                form::end_row(
                    theme::FORM_LABEL,
                    theme::CARD_FORM_GAP,
                    small_button(format!("{}resolve-kind", prefix(path)), label.clone(), true)
                        .on_click(cx.listener(move |view, _, _, _| {
                            (view.send)(Command::Resolve { kind: kind.clone() });
                        })),
                )
                .into_any_element(),
            );
        }
        rows
    }

    /// The row of one field: the label, and the control of the input type
    /// (section 10.3).
    fn field_row(
        &self,
        path: &[Slot],
        index: usize,
        field: &Field,
        value: Option<&Value>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = prefix(path);
        let field_path = FieldPath {
            action: path.to_vec(),
            key: field.key.clone(),
        };
        let row_id = format!("{id}field-{index}");
        let selector = row_id.clone();
        let row = |value: AnyElement| {
            form::fitted_row(
                field.label.clone(),
                theme::FORM_LABEL,
                theme::CARD_FORM_GAP,
                value,
                window,
            )
        };
        let content = match &field.input {
            InputType::Choice { .. } | InputType::RuntimeChoice | InputType::RuntimeChoiceList => {
                let dynamic;
                let options = match &field.input {
                    InputType::Choice { options } => options,
                    _ => {
                        dynamic = self
                            .form
                            .choices
                            .get(&field_path)
                            .map(|(_, _, options)| options.clone())
                            .unwrap_or_default();
                        &dynamic
                    }
                };
                let multiple = field.input == InputType::RuntimeChoiceList;
                let selected = choice_values(value, multiple);
                let menu = Menu::Choice(field_path.clone());
                let open = self.form.menu.as_ref() == Some(&menu);
                let current = choice_text(&selected, options);
                let toggle = menu.clone();
                let menu_element = open.then(|| {
                    // Include unavailable saved values so they can be removed
                    // deliberately, never as a side effect of a device loss.
                    let mut options = options.clone();
                    for value in &selected {
                        if !options.iter().any(|o| o.value == *value) {
                            options.push(ChoiceOption {
                                value: value.clone(),
                                label: format!("{value} (not available)"),
                            });
                        }
                    }
                    let rows =
                        options
                            .iter()
                            .enumerate()
                            .map(|(option, item)| {
                                let field_path = field_path.clone();
                                let chosen = selected.contains(&item.value);
                                let v = item.value.clone();
                                let mut next = selected.clone();
                                if chosen {
                                    next.retain(|value| value != &v);
                                } else {
                                    next.push(v.clone());
                                }
                                let next = if multiple {
                                    Value::List(next.into_iter().map(Value::Text).collect())
                                } else {
                                    Value::Text(v.clone())
                                };
                                controls::menu_row(format!("{id}choice-{index}-{option}"))
                                    .child(div().flex_shrink_0().w(theme::ICON_SMALL).when(
                                        chosen,
                                        |d| {
                                            d.child(
                                                Icon::new(IconName::Check).size(theme::ICON_SMALL),
                                            )
                                        },
                                    ))
                                    .child(item.label.clone())
                                    .on_click(cx.listener(move |view, _, window, cx| {
                                        view.form.menu = None;
                                        if let Some(binding) = view.form_binding() {
                                            view.set_field(binding, &field_path, next.clone())
                                        }
                                        view.focus_workspace(window, cx);
                                        cx.notify();
                                    }))
                                    .into_any_element()
                            })
                            .collect();
                    controls::popup_menu(
                        format!("{id}choice-{index}-menu"),
                        controls::MenuPlace::below(theme::HEIGHT_CONTROL),
                        rows,
                        window,
                        cx.listener(|view, _, _, cx| {
                            view.close_kinds();
                            cx.notify();
                        }),
                    )
                });
                row(div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .child(
                        controls::popup_button(
                            format!("{id}choice-{index}"),
                            field.label.clone(),
                            current.to_string(),
                            theme::HEIGHT_CONTROL,
                            open,
                            div().truncate().child(current.to_string()),
                        )
                        .text_size(theme::TEXT_LABEL)
                        .on_click(cx.listener(
                            move |view, _, window, cx| view.toggle_menu(toggle.clone(), window, cx),
                        )),
                    )
                    .children(menu_element)
                    .into_any_element())
            }
            InputType::Colour => {
                let current = value.and_then(|v| match v {
                    Value::Text(v) => super::inspector::parse_colour(v),
                    _ => None,
                });
                let samples = form::colour_palette(
                    &format!("{id}field-colour-{index}"),
                    current,
                    &field.label,
                    |colour| {
                        let field_path = field_path.clone();
                        cx.listener(move |view, _, _, _| {
                            if let Some(binding) = view.form_binding() {
                                view.set_field(
                                    binding,
                                    &field_path,
                                    Value::Text(super::inspector::colour_text(colour)),
                                );
                            }
                        })
                    },
                );
                let text = self.form.inputs.get(&field_path).map(|input| {
                    controls::text_field(
                        &input.state,
                        field.label.clone(),
                        theme::BG_BAR,
                        window,
                        cx,
                    )
                });
                row(div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_6)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(theme::SPACE_6)
                            .children(samples),
                    )
                    .children(text)
                    .child(form::help(controls::HEX_HELP))
                    .into_any_element())
            }
            InputType::Toggle => row(controls::switch(
                format!("{id}toggle-{index}"),
                field.label.clone(),
                value == Some(&Value::Boolean(true)),
                theme::SWITCH_INSPECTOR,
            )
            .on_click(cx.listener(move |view, _, _, cx| {
                let Some(binding) = view.form_binding() else {
                    return;
                };
                let on = view
                    .edit_action(binding)
                    .and_then(|a| contained(&a, &field_path.action))
                    .and_then(|a| a.settings.get(&field_path.key).cloned())
                    == Some(Value::Boolean(true));
                view.set_field(binding, &field_path, Value::Boolean(!on));
                cx.notify();
            }))
            .into_any_element()),
            InputType::Hotkey => {
                let recording = self.form.recording.as_ref() == Some(&field_path);
                let record = field_path.clone();
                let text = value_text(value);
                // The recorder shows the symbols of the hotkey, as the short
                // value of a nested card and of a step does. The profile
                // keeps the text of decision 0014.
                let (text, colour) = if recording {
                    ("Push a key…".to_string(), theme::TEXT_SECONDARY)
                } else if text.is_empty() {
                    ("Record hotkey…".to_string(), theme::TEXT_TERTIARY)
                } else {
                    (super::hotkey::symbols(&text), theme::TEXT_PRIMARY)
                };
                row(div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(theme::GAP_ITEMS)
                    .child(
                        div()
                            .id(format!("{id}hotkey-{index}"))
                            .debug_selector({
                                let id = format!("{id}hotkey-{index}");
                                move || id.clone()
                            })
                            // The label of the row, and the text of the
                            // recorder as the value.
                            .accessible(Role::Button, field.label.clone())
                            .aria_value(text.clone())
                            .flex_1()
                            .min_w_0()
                            .h(theme::HEIGHT_CONTROL)
                            .px(theme::SPACE_8)
                            .flex()
                            .items_center()
                            .rounded(theme::RADIUS_CONTROL)
                            .bg(theme::BG_BAR)
                            .border_1()
                            .border_color(if recording {
                                theme::ACCENT_LINE
                            } else {
                                theme::BORDER_FIELD
                            })
                            .text_size(theme::TEXT_BODY)
                            .text_color(colour)
                            .cursor_pointer()
                            .child(div().truncate().child(text.clone()).test_text(text))
                            .map(|d| controls::keyboard_focus(d, controls::Ring::Outside))
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.form.modifier_recorder = Default::default();
                                view.form.recording = Some(record.clone());
                                view.focus_workspace(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        small_button(format!("{id}clear-hotkey-{index}"), "Clear", true).on_click(
                            cx.listener(move |view, _, _, cx| {
                                view.form.recording = None;
                                if let Some(binding) = view.form_binding() {
                                    view.set_field(
                                        binding,
                                        &field_path,
                                        Value::Text(String::new()),
                                    );
                                }
                                cx.notify();
                            }),
                        ),
                    )
                    .into_any_element())
            }
            InputType::Action => {
                let inner = value.and_then(Action::from_value);
                form::stacked_row(
                    field.label.clone(),
                    div()
                        .flex()
                        .flex_col()
                        .gap(theme::SPACE_6)
                        .when(value.is_some() && inner.is_none(), |d| {
                            d.child(unreadable())
                        })
                        .child(self.nested_card(
                            child(path, &field.key, None),
                            inner.as_ref(),
                            true,
                            None,
                            window,
                            cx,
                        )),
                )
            }
            // A step list is above the form: see `action_rows`.
            InputType::ActionList | InputType::TimedActionList => div(),
            input => {
                let integer = matches!(input, InputType::Integer { .. });
                let text = self.form.inputs.get(&field_path).map(|input| {
                    controls::text_field(
                        &input.state,
                        field.label.clone(),
                        theme::BG_BAR,
                        window,
                        cx,
                    )
                    .when(integer, |d| d.flex_none().w(theme::WIDTH_INTEGER_FIELD))
                });
                let path_button = matches!(input, InputType::Path { .. }).then(|| {
                    button(format!("{id}path-{index}"), "Choose…")
                        .px(theme::SPACE_10)
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.path_field(field_path.clone(), window, cx)
                        }))
                });
                let value = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(theme::GAP_ITEMS)
                    .children(text)
                    .children(path_button);
                if matches!(input, InputType::LongText) {
                    form::top_row(
                        field.label.clone(),
                        theme::FORM_LABEL,
                        theme::CARD_FORM_GAP,
                        value,
                    )
                } else {
                    row(value.into_any_element())
                }
            }
        };
        content
            .id(row_id)
            .debug_selector(move || selector.clone())
            .into_any_element()
    }
}

pub(super) fn unreadable() -> Div {
    div()
        .text_size(theme::TEXT_LABEL)
        .text_color(theme::TEXT_TERTIARY)
        .child("OpenCraft cannot read the saved value. A change replaces it.")
}

fn choice_values(value: Option<&Value>, multiple: bool) -> Vec<String> {
    match value {
        Some(Value::Text(value)) if !multiple => vec![value.clone()],
        Some(Value::List(values)) if multiple => values
            .iter()
            .filter_map(|value| match value {
                Value::Text(value) => Some(value.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn choice_text(selected: &[String], options: &[ChoiceOption]) -> String {
    if selected.is_empty() {
        return "Select…".into();
    }
    selected
        .iter()
        .map(|value| {
            options
                .iter()
                .find(|o| &o.value == value)
                .map(|o| o.label.clone())
                .unwrap_or_else(|| format!("{value} (not available)"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn value_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Text(text)) => text.clone(),
        Some(Value::Integer(number)) => number.to_string(),
        Some(Value::Boolean(value)) => value.to_string(),
        _ => String::new(),
    }
}
