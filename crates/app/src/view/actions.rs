//! A settings form made only from engine descriptions.
//!
//! A field of the input type Action or Action list contains actions. The form
//! shows each contained action with the same code as the action of the
//! binding, to each depth that the settings have (decision 0017). The form
//! reads and writes the value of a contained action only with the functions
//! of the engine: see [`Action::from_value`].

use super::*;
use crate::state::gestures;
use gpui_kit::{
    component::input::{AnyInputState, Textarea, TextareaState},
    FocusHandle, KeyDownEvent,
};
use opencraft_engine::{Action, BindingId, Field, InputType, KindDescription, Status, Value};

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

/// The kind list that is open.
#[derive(Clone, Debug, PartialEq, Eq)]
enum KindMenu {
    /// Replace the action at the path.
    Replace(ActionPath),
    /// Add an action at the end of the list in the field `key` of the action
    /// at the path.
    Add(ActionPath, String),
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
    pub(super) focus: Option<FocusHandle>,
    menu: Option<KindMenu>,
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
                if matches!(event, TextEvent::Change) {
                    view.text_field(owner, &path, input.read(cx).value().to_string());
                }
            });
            return (input.into(), subscription);
        }
        let input = cx.new(|cx| InputState::new(window, cx));
        let subscription =
            cx.subscribe_in(&input, window, move |view, input, event, window, cx| {
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

    /// Select a kind in the open kind list. `None` is "No action".
    fn choose(
        &mut self,
        menu: KindMenu,
        kind: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.form.menu = None;
        self.choose_in(menu, kind);
        // A contained action can have new text fields now.
        self.sync_form(window, cx);
        // The click was on the list, thus no text field keeps the focus.
        window.blur(cx);
        cx.notify();
    }

    fn choose_in(&mut self, menu: KindMenu, kind: Option<String>) {
        match menu {
            KindMenu::Replace(path) if path.is_empty() => self.choose_kind(kind),
            KindMenu::Replace(path) => {
                let Some(binding) = self.form_binding() else {
                    return;
                };
                let current = self.edit_action(binding).and_then(|a| contained(&a, &path));
                // Selecting the same kind must not reset its settings to defaults.
                if current.is_some_and(|a| Some(&a.kind) == kind.as_ref()) {
                    return;
                }
                let new = kind.and_then(|kind| self.default_action(&kind));
                let (slot, parent) = path.split_last().expect("a contained action");
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
            KindMenu::Add(path, key) => {
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

    /// Replace the action of the binding. The engine makes the new binding,
    /// so no edit is in flight after this.
    fn choose_kind(&mut self, kind: Option<String>) {
        let Some(binding) = self.state.binding_id() else {
            return;
        };
        if kind.as_deref().is_some_and(|kind| {
            self.state
                .bindings
                .get(&binding)
                .and_then(|b| b.action.as_ref())
                .is_some_and(|a| a.kind == kind)
        }) {
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
        if !self
            .form
            .focus
            .as_ref()
            .is_some_and(|f| f.is_focused(window))
        {
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
        } else if event.keystroke.key == "escape"
            && event.keystroke.modifiers == gpui_kit::Modifiers::default()
        {
            self.form.recording = None;
        }
        cx.notify();
    }

    /// Close the open kind list.
    pub(super) fn close_kinds(&mut self) {
        self.form.menu = None;
    }

    pub(super) fn action_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(binding) = self.state.binding_id() else {
            return div().into_any_element();
        };
        let section = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_lg().child("Action"))
            .child(
                div().flex().flex_wrap().gap_1().children(
                    gestures(binding.control.kind)
                        .iter()
                        .map(|&(gesture, name)| {
                            let bound = self
                                .state
                                .bindings
                                .contains_key(&BindingId { gesture, ..binding });
                            button(
                                format!("gesture-{gesture:?}"),
                                format!("{name}{}", if bound { " •" } else { "" }),
                            )
                            .when(binding.gesture == gesture, |d| {
                                d.border_color(rgb(SELECTED))
                            })
                            .on_click(cx.listener(
                                move |view, _, window, cx| {
                                    view.state.gesture = Some(gesture);
                                    view.form.menu = None;
                                    view.sync_form(window, cx);
                                    window.blur(cx);
                                    cx.notify();
                                },
                            ))
                        }),
                ),
            );
        let current = self.state.bindings.get(&binding);
        let action = current.and_then(|_| self.edit_action(binding));
        let available = current.is_some_and(|b| b.available);
        section
            .child(self.action_body(&[], action.as_ref(), available, true, cx))
            .into_any_element()
    }

    /// The kind button, the kind list, and the fields of the action at
    /// `path`. With `none`, the kind list has "No action".
    fn action_body(
        &self,
        path: &[Slot],
        action: Option<&Action>,
        available: bool,
        none: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let id = prefix(path);
        let kind = action.and_then(|a| self.kind(&a.kind));
        // A kind with no description, for example from a newer version of
        // OpenCraft, has no name. Its identifier shows below the button.
        let name = action.map_or_else(
            || "No action".to_string(),
            |_| kind.map_or_else(|| "Unknown action".to_string(), |k| k.name.clone()),
        );
        let menu = KindMenu::Replace(path.to_vec());
        let open = self.form.menu.as_ref() == Some(&menu);
        let toggle = menu.clone();
        let mut body = div().flex().flex_col().gap_2().child(
            button(format!("{id}action-kind"), format!("{name} ▾")).on_click(cx.listener(
                move |view, _, _, cx| {
                    view.form.menu = if view.form.menu.as_ref() == Some(&toggle) {
                        None
                    } else {
                        Some(toggle.clone())
                    };
                    cx.notify();
                },
            )),
        );
        if open {
            body = body.child(self.kind_list(format!("{id}kind"), menu, none, cx));
        }
        let Some(action) = action else {
            return body;
        };
        if !available {
            body = body.child(
                div()
                    .id(format!("{id}unavailable"))
                    .debug_selector(move || format!("{id}unavailable"))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child(
                        "This version of OpenCraft does not have this action. It keeps the \
                         action in the profile. Select a different action, or use a newer \
                         version of OpenCraft.",
                    )
                    .child(
                        div()
                            .text_xs()
                            .child(format!("Action identifier: {}", action.kind)),
                    ),
            );
        } else if let Some(kind) = kind {
            for (index, field) in kind.fields.iter().enumerate() {
                body = body.child(self.field_editor(
                    path,
                    index,
                    field,
                    action.settings.get(&field.key),
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
            body = body.child(div().text_sm().child(text.clone())).child(
                button(format!("{}resolve-kind", prefix(path)), label.clone()).on_click(
                    cx.listener(move |view, _, _, _| {
                        (view.send)(Command::Resolve { kind: kind.clone() });
                    }),
                ),
            );
        }
        body
    }

    /// The list of all kinds. `id` is the start of the identifier of each
    /// item.
    fn kind_list(&self, id: String, menu: KindMenu, none: bool, cx: &mut Context<Self>) -> Div {
        let empty = menu.clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .when(none, |d| {
                d.child(
                    button(format!("{id}-none"), "No action").on_click(cx.listener(
                        move |view, _, window, cx| view.choose(empty.clone(), None, window, cx),
                    )),
                )
            })
            .children(self.state.kinds.iter().enumerate().map(|(index, k)| {
                let kind = k.kind.clone();
                let menu = menu.clone();
                button(format!("{id}-{index}"), k.name.clone()).on_click(cx.listener(
                    move |view, _, window, cx| {
                        view.choose(menu.clone(), Some(kind.clone()), window, cx)
                    },
                ))
            }))
    }

    fn field_editor(
        &self,
        path: &[Slot],
        index: usize,
        field: &Field,
        value: Option<&Value>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = prefix(path);
        let field_path = FieldPath {
            action: path.to_vec(),
            key: field.key.clone(),
        };
        let row_id = format!("{id}field-{index}");
        let mut row = div()
            .id(row_id.clone())
            .debug_selector(move || row_id.clone())
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .child(field.label.clone());
        match &field.input {
            InputType::Choice { options } => {
                row = row.children(options.iter().enumerate().map(|(option, item)| {
                    let field_path = field_path.clone();
                    let v = item.value.clone();
                    button(
                        format!("{id}choice-{index}-{option}"),
                        format!(
                            "{}{}",
                            item.label,
                            if value == Some(&Value::Text(v.clone())) {
                                " ✓"
                            } else {
                                ""
                            }
                        ),
                    )
                    .on_click(cx.listener(move |view, _, _, _| {
                        if let Some(binding) = view.form_binding() {
                            view.set_field(binding, &field_path, Value::Text(v.clone()))
                        }
                    }))
                }));
            }
            InputType::Toggle => {
                row = row.child(
                    button(
                        format!("{id}toggle-{index}"),
                        if value == Some(&Value::Boolean(true)) {
                            "On ✓"
                        } else {
                            "Off"
                        },
                    )
                    .on_click(cx.listener(move |view, _, _, _| {
                        let Some(binding) = view.form_binding() else {
                            return;
                        };
                        let on = view
                            .edit_action(binding)
                            .and_then(|a| contained(&a, &field_path.action))
                            .and_then(|a| a.settings.get(&field_path.key).cloned())
                            == Some(Value::Boolean(true));
                        view.set_field(binding, &field_path, Value::Boolean(!on));
                    })),
                );
            }
            InputType::Hotkey => {
                let recording = self.form.recording.as_ref() == Some(&field_path);
                let record = field_path.clone();
                row = row.child(
                    button(
                        format!("{id}hotkey-{index}"),
                        if recording {
                            "Press a key… Esc cancels".into()
                        } else {
                            let text = value_text(value);
                            if text.is_empty() {
                                "Record hotkey…".into()
                            } else {
                                text
                            }
                        },
                    )
                    .on_click(cx.listener(move |view, _, window, cx| {
                        let focus = view
                            .form
                            .focus
                            .get_or_insert_with(|| cx.focus_handle())
                            .clone();
                        view.form.recording = Some(record.clone());
                        window.focus(&focus, cx);
                        cx.notify();
                    })),
                );
                row = row.child(
                    button(format!("{id}clear-hotkey-{index}"), "Clear").on_click(cx.listener(
                        move |view, _, _, cx| {
                            view.form.recording = None;
                            if let Some(binding) = view.form_binding() {
                                view.set_field(binding, &field_path, Value::Text(String::new()));
                            }
                            cx.notify();
                        },
                    )),
                );
            }
            InputType::Action => {
                let inner = value.and_then(Action::from_value);
                let path = child(path, &field.key, None);
                let available = inner.as_ref().is_some_and(|a| self.kind(&a.kind).is_some());
                row = row.child(
                    nested()
                        .when(value.is_some() && inner.is_none(), |d| {
                            d.child(unreadable())
                        })
                        .child(self.action_body(&path, inner.as_ref(), available, true, cx)),
                );
            }
            InputType::ActionList => {
                row = row.child(self.action_list(path, &field.key, value, cx));
            }
            _ => {
                if let Some(input) = self.form.inputs.get(&field_path) {
                    row = row.child(match &input.state {
                        AnyInputState::Textarea(input) => Textarea::new(input).into_any_element(),
                        AnyInputState::Input(input) => Input::new(input).into_any_element(),
                        _ => div().into_any_element(),
                    });
                }
                if matches!(field.input, InputType::Path { .. }) {
                    row = row.child(button(format!("{id}path-{index}"), "Choose…").on_click(
                        cx.listener(move |view, _, window, cx| {
                            view.path_field(field_path.clone(), window, cx)
                        }),
                    ));
                }
            }
        }
        row.into_any_element()
    }

    /// The editor of the field `key` of the input type Action list: each
    /// action in its order, with buttons to move and remove it, and a button
    /// to add an action at the end.
    fn action_list(
        &self,
        path: &[Slot],
        key: &str,
        value: Option<&Value>,
        cx: &mut Context<Self>,
    ) -> Div {
        let items = value.and_then(Action::list_from_value);
        let id = format!("{}{key}", prefix(path));
        let count = items.as_ref().map_or(0, Vec::len);
        let mut list = nested();
        if value.is_some() && items.is_none() {
            list = list.child(unreadable());
        }
        if count == 0 {
            list = list.child(
                div()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child("No actions. Add an action."),
            );
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
            let available = self.kind(&inner.kind).is_some();
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .rounded_md()
                    .bg(rgb(BACKGROUND))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
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
                    .child(self.action_body(&item_path, Some(inner), available, false, cx)),
            );
        }
        let menu = KindMenu::Add(path.to_vec(), key.to_string());
        let open = self.form.menu.as_ref() == Some(&menu);
        let toggle = menu.clone();
        list = list.child(
            button(format!("{id}/add"), "Add action ▾").on_click(cx.listener(
                move |view, _, _, cx| {
                    view.form.menu = if view.form.menu.as_ref() == Some(&toggle) {
                        None
                    } else {
                        Some(toggle.clone())
                    };
                    cx.notify();
                },
            )),
        );
        if open {
            list = list.child(self.kind_list(format!("{id}/add-kind"), menu, false, cx));
        }
        list
    }
}

/// The frame of a contained action: a line at the left shows the depth.
fn nested() -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .pl_3()
        .border_l_2()
        .border_color(rgb(BORDER))
}

fn unreadable() -> Div {
    div()
        .text_sm()
        .text_color(rgb(MUTED))
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
