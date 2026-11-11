//! A settings form made only from engine descriptions.

use super::*;
use gpui_kit::{
    component::input::{AnyInputState, Textarea, TextareaState},
    FocusHandle, KeyDownEvent,
};
use opencraft_engine::{
    Action, BindingId, Field, GestureKind, InputType, KindDescription, Status, Value,
};

#[derive(Default)]
pub(super) struct ActionForm {
    identity: Option<(BindingId, KindDescription)>,
    pub(super) inputs: BTreeMap<String, AnyInputState>,
    subscriptions: Vec<Subscription>,
    // These are edits in flight, not confirmed bindings. Only events change ViewState.
    pending: BTreeMap<BindingId, Vec<Action>>,
    recording: Option<String>,
    pub(super) focus: Option<FocusHandle>,
    kinds_open: bool,
}

impl DeviceView {
    pub(super) fn sync_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let identity = self.state.binding_id().and_then(|id| {
            let binding = self.state.bindings.get(&id)?;
            if !binding.available {
                return None;
            }
            let action = binding.action.as_ref()?;
            let kind = self.state.kinds.iter().find(|k| k.kind == action.kind)?;
            Some((id, kind.clone()))
        });
        if self.form.identity != identity {
            self.form.inputs.clear();
            self.form.subscriptions.clear();
            self.form.recording = None;
            self.form.identity = identity.clone();
            if let Some((_, kind)) = &identity {
                for field in &kind.fields {
                    if matches!(
                        field.input,
                        InputType::Choice { .. } | InputType::Toggle | InputType::Hotkey
                    ) {
                        continue;
                    }
                    let key = field.key.clone();
                    let input: AnyInputState = if matches!(field.input, InputType::LongText) {
                        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 7));
                        let subscription =
                            cx.subscribe_in(&input, window, move |view, input, event, _, cx| {
                                if matches!(event, TextEvent::Change) {
                                    view.text_field(&key, input.read(cx).value().to_string());
                                }
                            });
                        self.form.subscriptions.push(subscription);
                        input.into()
                    } else {
                        let input = cx.new(|cx| InputState::new(window, cx));
                        let subscription = cx.subscribe_in(
                            &input,
                            window,
                            move |view, input, event, window, cx| {
                                if matches!(event, TextEvent::Change) {
                                    let text = input.read(cx).value().to_string();
                                    if let Some(Value::Integer(value)) =
                                        view.text_field(&key, text.clone())
                                    {
                                        // Show the clamped value, but allow a temporary empty or minus input.
                                        if value.to_string() != text {
                                            input.update(cx, |input, cx| {
                                                input.set_value(value.to_string(), window, cx)
                                            });
                                        }
                                    }
                                }
                            },
                        );
                        self.form.subscriptions.push(subscription);
                        input.into()
                    };
                    self.form.inputs.insert(field.key.clone(), input);
                }
            }
        }
        let Some((binding, kind)) = identity else {
            return;
        };
        let Some(action) = self.edit_action(binding) else {
            return;
        };
        for field in kind.fields {
            let Some(input) = self.form.inputs.get(&field.key) else {
                continue;
            };
            let value = value_text(action.settings.get(&field.key));
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

    fn text_field(&mut self, key: &str, text: String) -> Option<Value> {
        let field = self
            .form
            .identity
            .as_ref()?
            .1
            .fields
            .iter()
            .find(|f| f.key == key)?;
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
        self.set_field(key, value.clone());
        Some(value)
    }

    fn set_field(&mut self, key: &str, value: Value) {
        let Some(binding) = self.state.binding_id() else {
            return;
        };
        self.set_binding_field(binding, key, value);
    }

    fn set_binding_field(&mut self, binding: BindingId, key: &str, value: Value) {
        let Some(mut action) = self.edit_action(binding) else {
            return;
        };
        action.settings.insert(key.into(), value);
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

    fn choose_kind(&mut self, kind: Option<String>, cx: &mut Context<Self>) {
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
            self.form.kinds_open = false;
            cx.notify();
            return;
        }
        let action = kind
            .and_then(|kind| self.state.kinds.iter().find(|k| k.kind == kind))
            .map(|kind| Action {
                kind: kind.kind.clone(),
                settings: kind
                    .fields
                    .iter()
                    .filter_map(|f| f.default.clone().map(|v| (f.key.clone(), v)))
                    .collect(),
            });
        self.form.pending.remove(&binding);
        self.form.kinds_open = false;
        (self.send)(Command::SetBinding { binding, action });
        cx.notify();
    }

    fn path_field(&mut self, key: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(binding) = self.state.binding_id() else {
            return;
        };
        let Some(action) = self
            .state
            .bindings
            .get(&binding)
            .and_then(|b| b.action.as_ref())
        else {
            return;
        };
        let kind = action.kind.clone();
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
                        // Do not replace a different action after a delayed dialog.
                        if view
                            .state
                            .bindings
                            .get(&binding)
                            .and_then(|b| b.action.as_ref())
                            .is_some_and(|a| a.kind == kind)
                        {
                            if let Some(path) = paths.first() {
                                view.set_binding_field(
                                    binding,
                                    &key,
                                    Value::Text(path.to_string_lossy().into_owned()),
                                );
                            }
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
        let Some(key) = self.form.recording.clone() else {
            return;
        };
        cx.stop_propagation();
        if let Some(value) = super::hotkey::hotkey(&event.keystroke) {
            self.set_field(&key, Value::Text(value));
            self.form.recording = None;
        } else if event.keystroke.key == "escape"
            && event.keystroke.modifiers == gpui_kit::Modifiers::default()
        {
            self.form.recording = None;
        }
        cx.notify();
    }

    pub(super) fn action_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(binding) = self.state.binding_id() else {
            return div().into_any_element();
        };
        let gestures: &[(GestureKind, &str)] = match binding.control.kind {
            ControlKind::Key => &[
                (GestureKind::Press, "Press"),
                (GestureKind::Release, "Release"),
            ],
            ControlKind::Knob => &[
                (GestureKind::TurnLeft, "Turn left"),
                (GestureKind::TurnRight, "Turn right"),
                (GestureKind::Push, "Push"),
            ],
            _ => &[],
        };
        let mut section = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_lg().child("Action"))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .children(gestures.iter().map(|&(gesture, name)| {
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
                                view.form.kinds_open = false;
                                view.sync_form(window, cx);
                                window.blur(cx);
                                cx.notify();
                            },
                        ))
                    })),
            );
        let current = self.state.bindings.get(&binding);
        let action = current.and_then(|b| b.action.as_ref());
        let kind = action.and_then(|a| self.state.kinds.iter().find(|k| k.kind == a.kind));
        let name = action.map_or_else(
            || "No action".to_string(),
            |a| kind.map_or_else(|| a.kind.clone(), |k| k.name.clone()),
        );
        section = section.child(
            button("action-kind", format!("{name} ▾")).on_click(cx.listener(|view, _, _, cx| {
                view.form.kinds_open = !view.form.kinds_open;
                cx.notify();
            })),
        );
        if self.form.kinds_open {
            section = section
                .child(
                    button("kind-none", "No action")
                        .on_click(cx.listener(|view, _, _, cx| view.choose_kind(None, cx))),
                )
                .children(self.state.kinds.iter().enumerate().map(|(index, k)| {
                    let id = k.kind.clone();
                    button(format!("kind-{index}"), k.name.clone()).on_click(
                        cx.listener(move |view, _, _, cx| view.choose_kind(Some(id.clone()), cx)),
                    )
                }));
        }
        if let Some(action) = action {
            if current.is_some_and(|b| !b.available) {
                section = section.child(
                    div()
                        .id("unavailable")
                        .debug_selector(|| "unavailable".into())
                        .text_sm()
                        .text_color(rgb(MUTED))
                        .child(format!("{}: This action is not available.", action.kind)),
                );
            } else if let Some(kind) = kind {
                for (index, field) in kind.fields.iter().enumerate() {
                    section = section.child(self.field_editor(
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
                section = section.child(div().text_sm().child(text.clone())).child(
                    button("resolve-kind", label.clone()).on_click(cx.listener(
                        move |view, _, _, _| {
                            (view.send)(Command::Resolve { kind: kind.clone() });
                        },
                    )),
                );
            }
        }
        section.into_any_element()
    }

    fn field_editor(
        &self,
        index: usize,
        field: &Field,
        value: Option<&Value>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = field.key.clone();
        let mut row = div()
            .id(format!("field-{index}"))
            .debug_selector(move || format!("field-{index}"))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .child(field.label.clone());
        match &field.input {
            InputType::Choice { options } => {
                row = row.children(options.iter().enumerate().map(|(option, item)| {
                    let key = key.clone();
                    let v = item.value.clone();
                    button(
                        format!("choice-{index}-{option}"),
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
                        view.set_field(&key, Value::Text(v.clone()))
                    }))
                }));
            }
            InputType::Toggle => {
                row = row.child(
                    button(
                        format!("toggle-{index}"),
                        if value == Some(&Value::Boolean(true)) {
                            "On ✓"
                        } else {
                            "Off"
                        },
                    )
                    .on_click(cx.listener(move |view, _, _, _| {
                        let current = view.state.binding_id().and_then(|id| view.edit_action(id));
                        let on = current.as_ref().and_then(|a| a.settings.get(&key))
                            == Some(&Value::Boolean(true));
                        view.set_field(&key, Value::Boolean(!on));
                    })),
                );
            }
            InputType::Hotkey => {
                let recording = self.form.recording.as_ref() == Some(&key);
                row = row.child(
                    button(
                        format!("hotkey-{index}"),
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
                        view.form.recording = Some(key.clone());
                        window.focus(&focus, cx);
                        cx.notify();
                    })),
                );
                let key = field.key.clone();
                row = row.child(button(format!("clear-hotkey-{index}"), "Clear").on_click(
                    cx.listener(move |view, _, _, cx| {
                        view.form.recording = None;
                        view.set_field(&key, Value::Text(String::new()));
                        cx.notify();
                    }),
                ));
            }
            _ => {
                if let Some(input) = self.form.inputs.get(&field.key) {
                    row = row.child(match input {
                        AnyInputState::Textarea(input) => Textarea::new(input).into_any_element(),
                        AnyInputState::Input(input) => Input::new(input).into_any_element(),
                        _ => div().into_any_element(),
                    });
                }
                if matches!(field.input, InputType::Path { .. }) {
                    row = row.child(button(format!("path-{index}"), "Choose…").on_click(
                        cx.listener(move |view, _, window, cx| {
                            view.path_field(key.clone(), window, cx)
                        }),
                    ));
                }
            }
        }
        row.into_any_element()
    }
}

fn value_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Text(text)) => text.clone(),
        Some(Value::Integer(number)) => number.to_string(),
        Some(Value::Boolean(value)) => value.to_string(),
        _ => String::new(),
    }
}
