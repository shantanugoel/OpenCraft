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
    component::{
        input::{AnyInputState, TextareaState},
        Icon,
    },
    KeyDownEvent, Pixels,
};
use opencraft_engine::{
    Action, BindingId, Field, GestureKind, InputType, KindDescription, Status, Value,
};

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
enum Menu {
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

/// A change to the list of actions in a field.
#[derive(Clone, Copy)]
enum ListChange {
    Remove(usize),
    Move(usize, usize),
}

#[derive(Default)]
pub(super) struct ActionForm {
    /// The edit epoch and the binding of the inputs. The inputs are only for
    /// an available binding.
    owner: Option<(u64, BindingId)>,
    pub(super) inputs: BTreeMap<FieldPath, FieldInput>,
    // These are edits in flight, not confirmed bindings. Only events change ViewState.
    pending: BTreeMap<BindingId, Vec<Action>>,
    recording: Option<FieldPath>,
    menu: Option<Menu>,
}

/// The text form of a path for the identifiers of the elements. It is empty
/// for the action of the binding, so the identifiers of its form stay short.
fn prefix(path: &[Slot]) -> String {
    path.iter()
        .map(|slot| match slot.item {
            Some(item) => format!("{}.{item}/", slot.key),
            None => format!("{}/", slot.key),
        })
        .collect()
}

fn child(path: &[Slot], key: &str, item: Option<usize>) -> ActionPath {
    let mut path = path.to_vec();
    path.push(Slot {
        key: key.into(),
        item,
    });
    path
}

/// The action at `path` in `action`.
fn contained(action: &Action, path: &[Slot]) -> Option<Action> {
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
fn change(action: &mut Action, path: &[Slot], f: impl FnOnce(&mut Action)) {
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
fn list(action: &Action, key: &str) -> Vec<Action> {
    action
        .settings
        .get(key)
        .and_then(Action::list_from_value)
        .unwrap_or_default()
}

impl DeviceView {
    fn kind(&self, kind: &str) -> Option<&KindDescription> {
        self.state.kinds.iter().find(|k| k.kind == kind)
    }

    /// A new action of `kind` with the default value of each field.
    fn default_action(&self, kind: &str) -> Option<Action> {
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
    fn form_binding(&self) -> Option<BindingId> {
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
            self.form.recording = None;
            self.form.menu = None;
            self.form.owner = owner;
        }
        let Some((_, binding)) = owner else {
            return;
        };
        let Some(action) = self.edit_action(binding) else {
            return;
        };
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
        for field in &kind.fields {
            let value = action.settings.get(&field.key);
            match field.input {
                InputType::Choice { .. } | InputType::Toggle | InputType::Hotkey => {}
                InputType::Action => {
                    if let Some(inner) = value.and_then(Action::from_value) {
                        self.text_fields(&inner, &child(path, &field.key, None), out);
                    }
                }
                InputType::ActionList => {
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

    fn edit_action(&self, id: BindingId) -> Option<Action> {
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
        let field = self
            .kind(&action.kind)?
            .fields
            .iter()
            .find(|f| f.key == path.key)?;
        let value = match field.input {
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
    fn edit(&mut self, binding: BindingId, f: impl FnOnce(&mut Action)) {
        let Some(before) = self.edit_action(binding) else {
            return;
        };
        let mut action = before.clone();
        f(&mut action);
        if action == before {
            return;
        }
        self.form
            .pending
            .entry(binding)
            .or_default()
            .push(action.clone());
        (self.send)(Command::SetBinding {
            binding,
            action: Some(action),
        });
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
    fn toggle_menu(&mut self, menu: Menu, window: &mut Window, cx: &mut Context<Self>) {
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
        self.choose_in(menu, kind);
        // A contained action can have new text fields now.
        self.sync_form(window, cx);
        // The click was on the menu, thus no text field keeps the focus.
        self.focus_workspace(window, cx);
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
                if current.is_some_and(|a| Some(&a.kind) == kind.as_ref()) {
                    return;
                }
                let new = kind.and_then(|kind| self.default_action(&kind));
                let Some((slot, parent)) = path.split_last() else {
                    return;
                };
                self.edit(binding, |root| {
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
                let Some(new) = kind.and_then(|kind| self.default_action(&kind)) else {
                    return;
                };
                self.edit(binding, |root| {
                    change(root, &path, |action| {
                        let mut items = list(action, &key);
                        items.push(new);
                        action.settings.insert(key, Action::list_to_value(&items));
                    })
                });
            }
        }
    }

    fn change_list(
        &mut self,
        path: ActionPath,
        key: String,
        list_change: ListChange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.change_list_in(path, key, list_change);
        self.sync_form(window, cx);
        cx.notify();
    }

    fn change_list_in(&mut self, path: ActionPath, key: String, list_change: ListChange) {
        let Some(binding) = self.form_binding() else {
            return;
        };
        self.edit(binding, |root| {
            change(root, &path, |action| {
                let mut items = list(action, &key);
                match list_change {
                    ListChange::Remove(item) if item < items.len() => {
                        items.remove(item);
                    }
                    ListChange::Move(from, to) if from < items.len() && to < items.len() => {
                        items.swap(from, to);
                    }
                    _ => return,
                }
                action.settings.insert(key, Action::list_to_value(&items));
            })
        });
    }

    /// Set the action of the gesture card of `gesture` to a new action of
    /// `kind`, or remove it with `None`. A new action opens the card; the
    /// removal closes it. The engine makes the new binding, so no edit is in
    /// flight after this.
    fn choose_card(&mut self, gesture: GestureKind, kind: Option<String>) {
        let Some(control) = self.state.selected_control else {
            return;
        };
        let binding = BindingId {
            page: self.state.page,
            control,
            gesture,
        };
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
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a path".into()),
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
        // The recorder has the focus of the root element. A key in a text
        // field is text.
        if !self.focus.is_focused(window) {
            return;
        }
        let escape = event.keystroke.key == "escape"
            && event.keystroke.modifiers == gpui_kit::Modifiers::default();
        if escape && (self.form.menu.is_some() || self.inspector.colour_open) {
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

    /// Close the open menu of the cards, and the popover of "Other colour".
    pub(super) fn close_kinds(&mut self) {
        self.form.menu = None;
        self.inspector.colour_open = false;
    }

    /// The label of the popup of `action`.
    fn kind_label(&self, action: &Action, available: bool) -> KindLabel {
        match self.kind(&action.kind) {
            Some(kind) if available => KindLabel::new(kind.name.clone()),
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
                let card = GestureCard::new(format!("gesture-{gesture:?}"), name)
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
                let rows = if open {
                    self.action_rows(&[], &action, current.available, window, cx)
                } else {
                    Vec::new()
                };
                card.action(self.kind_label(&action, current.available))
                    .open(open)
                    .on_toggle(
                        cx.listener(move |view, _, window, cx| {
                            view.toggle_card(gesture, window, cx)
                        }),
                    )
                    .children(rows)
                    .into_any_element()
            })
            .collect()
    }

    /// The kind menu `menu`, if it is open: one row for each kind, and with
    /// `remove`, "Remove the action". `id` is the start of the identifier of
    /// each row.
    fn kind_menu(
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
        // #184 adds a title for each group of the library.
        let mut rows: Vec<AnyElement> = self
            .state
            .kinds
            .iter()
            .enumerate()
            .map(|(index, k)| {
                let kind = k.kind.clone();
                let menu = menu.clone();
                controls::menu_row(format!("{id}-{index}"))
                    .child(KindLabel::new(k.name.clone()).content(false))
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.choose(menu.clone(), Some(kind.clone()), window, cx)
                    }))
                    .into_any_element()
            })
            .collect();
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
    /// the handler for the user.
    fn action_rows(
        &self,
        path: &[Slot],
        action: &Action,
        available: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let id = prefix(path);
        let mut rows = Vec::new();
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
            for (index, field) in kind.fields.iter().enumerate() {
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

    /// A nested card for the contained action at `path` (section 10.5). With
    /// no action, it is a dashed card "Add an action". `remove` adds
    /// "Remove the action" to its kind menu.
    fn nested_card(
        &self,
        path: ActionPath,
        action: Option<&Action>,
        remove: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = prefix(&path);
        let menu = Menu::Replace(path.clone());
        let toggle = menu.clone();
        let card =
            GestureCard::new(format!("{id}action"), "")
                .nested()
                .menu(self.kind_menu(
                    &format!("{id}kind"),
                    &menu,
                    remove && action.is_some(),
                    menu_button_height(action.is_some(), true),
                    window,
                    cx,
                ))
                .on_menu(cx.listener(move |view, _, window, cx| {
                    view.toggle_menu(toggle.clone(), window, cx)
                }));
        let Some(action) = action else {
            return card.into_any_element();
        };
        let available = self.kind(&action.kind).is_some();
        card.action(self.kind_label(action, available))
            .open(true)
            .children(self.action_rows(&path, action, available, window, cx))
            .into_any_element()
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
            InputType::Choice { options } => {
                let menu = Menu::Choice(field_path.clone());
                let open = self.form.menu.as_ref() == Some(&menu);
                let current = options
                    .iter()
                    .find(|o| value == Some(&Value::Text(o.value.clone())))
                    .map_or("", |o| o.label.as_str());
                let toggle = menu.clone();
                let menu_element = open.then(|| {
                    let rows = options
                        .iter()
                        .enumerate()
                        .map(|(option, item)| {
                            let field_path = field_path.clone();
                            let chosen = value == Some(&Value::Text(item.value.clone()));
                            let v = item.value.clone();
                            controls::menu_row(format!("{id}choice-{index}-{option}"))
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .w(theme::ICON_SMALL)
                                        .when(chosen, |d| {
                                            d.child(
                                                Icon::new(IconName::Check).size(theme::ICON_SMALL),
                                            )
                                        }),
                                )
                                .child(item.label.clone())
                                .on_click(cx.listener(move |view, _, window, cx| {
                                    view.form.menu = None;
                                    if let Some(binding) = view.form_binding() {
                                        view.set_field(binding, &field_path, Value::Text(v.clone()))
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
            InputType::Toggle => row(controls::switch(
                format!("{id}toggle-{index}"),
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
                let (text, colour) = if recording {
                    ("Press a key…".to_string(), theme::TEXT_SECONDARY)
                } else if text.is_empty() {
                    ("Record hotkey…".to_string(), theme::TEXT_TERTIARY)
                } else {
                    (text, theme::TEXT_PRIMARY)
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
                            .child(div().truncate().child(text))
                            .on_click(cx.listener(move |view, _, window, cx| {
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
                            window,
                            cx,
                        )),
                )
            }
            InputType::ActionList => form::stacked_row(
                field.label.clone(),
                self.action_list(path, &field.key, value, window, cx),
            ),
            input => {
                let integer = matches!(input, InputType::Integer { .. });
                let text = self.form.inputs.get(&field_path).map(|input| {
                    controls::text_field(&input.state, theme::BG_BAR, window, cx)
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

    /// The editor of the field `key` of the input type Action list: each
    /// action in its order, with buttons to move and remove it, and a card
    /// to add an action at the end. #183 changes it to the step list of
    /// section 10.5.
    fn action_list(
        &self,
        path: &[Slot],
        key: &str,
        value: Option<&Value>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let items = value.and_then(Action::list_from_value);
        let id = format!("{}{key}", prefix(path));
        let count = items.as_ref().map_or(0, Vec::len);
        let mut list = div().flex().flex_col().gap(theme::SPACE_6);
        if value.is_some() && items.is_none() {
            list = list.child(unreadable());
        }
        for (item, inner) in items.iter().flatten().enumerate() {
            let item_path = child(path, key, Some(item));
            let item_id = prefix(&item_path);
            let tool = |name: &str, label: &str, enabled: bool, list_change: ListChange| {
                let (path, key) = (path.to_vec(), key.to_string());
                small_button(format!("{item_id}{name}"), label.to_string(), enabled).when(
                    enabled,
                    |d| {
                        d.on_click(cx.listener(move |view, _, window, cx| {
                            view.change_list(path.clone(), key.clone(), list_change, window, cx)
                        }))
                    },
                )
            };
            list = list
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(theme::SPACE_4)
                        .child(
                            div()
                                .flex_1()
                                .text_size(theme::TEXT_CAPTION)
                                .text_color(theme::TEXT_TERTIARY)
                                .child(format!("Action {}", item + 1)),
                        )
                        .child(tool(
                            "up",
                            "↑",
                            item > 0,
                            ListChange::Move(item, item.wrapping_sub(1)),
                        ))
                        .child(tool(
                            "down",
                            "↓",
                            item + 1 < count,
                            ListChange::Move(item, item + 1),
                        ))
                        .child(tool("remove", "Remove", true, ListChange::Remove(item))),
                )
                .child(self.nested_card(item_path, Some(inner), false, window, cx));
        }
        let menu = Menu::Add(path.to_vec(), key.to_string());
        let toggle = menu.clone();
        list.child(
            GestureCard::new(format!("{id}/add"), "")
                .nested()
                .menu(self.kind_menu(
                    &format!("{id}/add-kind"),
                    &menu,
                    false,
                    menu_button_height(false, true),
                    window,
                    cx,
                ))
                .on_menu(cx.listener(move |view, _, window, cx| {
                    view.toggle_menu(toggle.clone(), window, cx)
                })),
        )
    }
}

fn unreadable() -> Div {
    div()
        .text_size(theme::TEXT_LABEL)
        .text_color(theme::TEXT_TERTIARY)
        .child("OpenCraft cannot read the saved value. A change replaces it.")
}

fn value_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Text(text)) => text.clone(),
        Some(Value::Integer(number)) => number.to_string(),
        Some(Value::Boolean(value)) => value.to_string(),
        _ => String::new(),
    }
}
