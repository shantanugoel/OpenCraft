//! The editors of the actions that contain actions (decision 0029, section
//! 10.5): the nested card of a field of the input type Action, for example
//! "From Off to On" of a toggle, and the step list of a field of the input
//! type Action list, for example the steps of a sequence.
//!
//! A nested card and an open step use the gesture card
//! ([`GestureCard::nested`]). The fields of a contained action come from
//! the form of `actions`, to each depth that the settings have (decision
//! 0017).
//!
//! Each change is one step of the history (decision 0030): the window ends
//! an open text edit first, so that the change does not join the step of
//! the text. The steps of a list have their own names: "Add Step", "Move
//! Step", and "Remove Step".
//!
//! The element identifiers start with the text of the path of the action
//! (`actions::prefix`), for example `items.0/` for the first step of the
//! list `items`:
//!
//! | Identifier | Element |
//! |---|---|
//! | `{list}/add` | The button "Add a step" of the list `{list}`, for example `items/add` |
//! | `{step}step` | The row of a step |
//! | `{step}handle` | The drag handle of a step |
//! | `{step}more` | The "⋯" button of an open step |
//! | `{step}action` | The nested card of an open step, or of a field of the input type Action |

use super::actions::{child, list, prefix, unreadable, ActionPath, Menu, Slot};
use super::gesture_card::{menu_button_height, GestureCard, KindLabel, TileSize};
use super::*;
use gpui_kit::{
    actions,
    assets::IconName,
    component::{native_menu::NativeMenu, tooltip::Tooltip, Icon},
    FontWeight, MouseButton, MouseDownEvent, Pixels, Point,
};
use opencraft_engine::{Action, BindingId, Field, InputType, Value};
use std::collections::BTreeSet;

/// Make inherited timing explicit before a change of list positions.
fn keep_step_delays(items: &mut [Action], timing: Option<&Field>) {
    if let Some(field) = timing {
        for (index, item) in items.iter_mut().enumerate() {
            let default = if index == 0 {
                Some(Value::Integer(0))
            } else {
                field.default.clone()
            };
            if let Some(value) = default {
                item.settings.entry(field.key.clone()).or_insert(value);
            }
        }
    }
}

// The items of the menu of a step. A native menu item sends its action to
// the focused element, thus the root element of the window handles them
// for the step of the menu (`NestedState::step_menu`).
actions!(nested, [MoveStepUp, MoveStepDown, RemoveStep]);

/// The dashed button below the steps.
const ADD_A_STEP: &str = "Add a step";

/// The tooltip and the accessible label of the drag handle of a step.
const DRAG_TO_REORDER: &str = "Drag to reorder";

/// The tooltip of the "⋯" button of an open step.
const MORE_FOR_THIS_STEP: &str = "More for this step";

/// A field of the input type Action list: the path of the action that has
/// the field, and the key of the field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StepList {
    pub(super) path: ActionPath,
    pub(super) key: String,
}

impl StepList {
    /// The path of the step at `item`.
    pub(super) fn step(&self, item: usize) -> ActionPath {
        child(&self.path, &self.key, Some(item))
    }

    /// The position of the step of this list that `path` is in, or that
    /// `path` is.
    fn item_of(&self, path: &[Slot]) -> Option<usize> {
        let slot = path.get(self.path.len())?;
        (path.starts_with(&self.path) && slot.key == self.key)
            .then_some(slot.item)
            .flatten()
    }
}

/// The state of the nested cards of the form. It belongs to the binding of
/// the form, and a different binding clears it.
#[derive(Default)]
pub(super) struct NestedState {
    /// The open nested cards and steps, by the path of their action. A
    /// nested card is closed until the user opens it or gives it an action.
    pub(super) open: BTreeSet<ActionPath>,
    /// The step of the open step menu.
    step_menu: Option<(StepList, usize)>,
}

/// A step in a drag: its list, its position, its binding, and its kind for
/// the drag picture.
#[derive(Clone)]
struct StepDrag {
    binding: BindingId,
    list: StepList,
    from: usize,
    label: KindLabel,
}

/// The drag picture of a step (decision 0029, section 12): a chip at the
/// pointer, with the kind tile and the kind name.
struct StepDragPicture {
    label: KindLabel,
    /// The point of the press in the drag handle. The padding keeps the
    /// chip at the pointer (`docs/kb/gpui.md`, drag picture offset).
    offset: Point<Pixels>,
}

impl Render for StepDragPicture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let inset = |value: Pixels| (value - theme::SPACE_8).max(px(0.));
        div()
            .pl(inset(self.offset.x))
            .pt(inset(self.offset.y))
            .child(self.label.drag_chip())
    }
}

/// The new position of the item at `index` after the item at `from` moves
/// to `to`.
fn moved(index: usize, from: usize, to: usize) -> usize {
    if index == from {
        to
    } else if from < to && (from + 1..=to).contains(&index) {
        index - 1
    } else if to < from && (to..from).contains(&index) {
        index + 1
    } else {
        index
    }
}

/// "1 step", "3 steps", or "No steps".
fn steps_text(count: usize) -> String {
    match count {
        0 => "No steps".into(),
        1 => "1 step".into(),
        n => format!("{n} steps"),
    }
}

impl DeviceView {
    /// The root element handles the items of the step menu.
    pub(super) fn step_actions(
        &self,
        root: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        root.on_action(cx.listener(|view, _: &MoveStepUp, window, cx| {
            if let Some((list, item)) = view.form.nested.step_menu.take() {
                if item > 0 {
                    view.move_step(list, item, item - 1, window, cx);
                }
            }
        }))
        .on_action(cx.listener(|view, _: &MoveStepDown, window, cx| {
            if let Some((list, item)) = view.form.nested.step_menu.take() {
                view.move_step(list, item, item + 1, window, cx);
            }
        }))
        .on_action(cx.listener(|view, _: &RemoveStep, window, cx| {
            if let Some((list, item)) = view.form.nested.step_menu.take() {
                view.remove_step(list, item, window, cx);
            }
        }))
    }

    /// The short value of an action: the value of its first field, as text
    /// (section 10.5). A hotkey has the symbols, a choice the option label,
    /// a list its count of steps, and a contained action the kind name.
    pub(super) fn short_value(&self, action: &Action) -> String {
        let Some(kind) = self.kind(&action.kind) else {
            return String::new();
        };
        let Some(field) = kind.fields.first() else {
            return String::new();
        };
        let value = action.settings.get(&field.key);
        let text = match value {
            Some(Value::Text(text)) => text.clone(),
            Some(Value::Integer(number)) => number.to_string(),
            _ => String::new(),
        };
        match &field.input {
            InputType::Hotkey if !text.is_empty() => super::hotkey::symbols(&text),
            InputType::Choice { options } => options
                .iter()
                .find(|o| o.value == text)
                .map(|o| o.label.clone())
                .unwrap_or_default(),
            InputType::Toggle => match value {
                Some(Value::Boolean(true)) => "On".into(),
                Some(Value::Boolean(false)) => "Off".into(),
                _ => String::new(),
            },
            InputType::Action => kind.name.clone(),
            InputType::ActionList | InputType::TimedActionList => {
                steps_text(list(action, &field.key).len())
            }
            InputType::Path { .. } => std::path::Path::new(&text)
                .file_name()
                .map_or(text.clone(), |name| name.to_string_lossy().into_owned()),
            _ => text,
        }
    }

    /// The summary in the head of an open nested card: the count of steps
    /// of a kind with a step list, for example "2 steps". Else nothing: the
    /// card shows the fields.
    fn summary(&self, action: &Action) -> String {
        self.kind(&action.kind)
            .and_then(|kind| kind.fields.iter().find(|f| f.input.is_action_list()))
            .map(|field| steps_text(list(action, &field.key).len()))
            .unwrap_or_default()
    }

    /// Open or close the nested card at `path`. A step that opens closes
    /// the other open step of its list.
    fn toggle_nested(&mut self, path: ActionPath, window: &mut Window, cx: &mut Context<Self>) {
        if !self.form.nested.open.remove(&path) {
            self.open_nested(&path);
        }
        self.close_kinds();
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// Open the nested card at `path`: for example after the user gives it
    /// an action. A step closes the other open steps of its list.
    pub(super) fn open_nested(&mut self, path: &[Slot]) {
        if let Some((last, parent)) = path.split_last() {
            if last.item.is_some() {
                let list = StepList {
                    path: parent.to_vec(),
                    key: last.key.clone(),
                };
                self.form
                    .nested
                    .open
                    .retain(|open| open.len() != path.len() || list.item_of(open).is_none());
            }
        }
        self.form.nested.open.insert(path.to_vec());
    }

    /// Change the positions of the open cards in `list` with `f`. A card
    /// with no new position closes, with all cards in it.
    fn remap_open(&mut self, list: &StepList, f: impl Fn(usize) -> Option<usize>) {
        let depth = list.path.len();
        self.form.nested.open = std::mem::take(&mut self.form.nested.open)
            .into_iter()
            .filter_map(|mut path| match list.item_of(&path) {
                Some(item) => {
                    let new = f(item)?;
                    path[depth].item = Some(new);
                    Some(path)
                }
                None => Some(path),
            })
            .collect();
    }

    /// Move the step at `from` of `list` to the position `to`.
    fn move_step(
        &mut self,
        list: StepList,
        from: usize,
        to: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_workspace(window, cx);
        let Some(binding) = self.form_binding() else {
            return;
        };
        let key = list.key.clone();
        let timing = self.step_delay_field(&list.step(1));
        let mut done = false;
        self.edit_as(binding, Some("Move Step"), |root| {
            super::actions::change(root, &list.path, |action| {
                let mut items = super::actions::list(action, &key);
                if from == to || from >= items.len() || to >= items.len() {
                    return;
                }
                keep_step_delays(&mut items, timing.as_ref());
                let item = items.remove(from);
                items.insert(to, item);
                action.settings.insert(key, Action::list_to_value(&items));
                done = true;
            })
        });
        if done {
            self.remap_open(&list, |index| Some(moved(index, from, to)));
        }
        self.sync_form(window, cx);
        cx.notify();
    }

    /// Remove the step at `item` of `list`.
    fn remove_step(
        &mut self,
        list: StepList,
        item: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_workspace(window, cx);
        let Some(binding) = self.form_binding() else {
            return;
        };
        let key = list.key.clone();
        let mut done = false;
        let timing = self.step_delay_field(&list.step(1));
        self.edit_as(binding, Some("Remove Step"), |root| {
            super::actions::change(root, &list.path, |action| {
                let mut items = super::actions::list(action, &key);
                if item >= items.len() {
                    return;
                }
                keep_step_delays(&mut items, timing.as_ref());
                items.remove(item);
                action.settings.insert(key, Action::list_to_value(&items));
                done = true;
            })
        });
        if done {
            self.remap_open(&list, |index| match index.cmp(&item) {
                std::cmp::Ordering::Less => Some(index),
                std::cmp::Ordering::Equal => None,
                std::cmp::Ordering::Greater => Some(index - 1),
            });
        }
        self.sync_form(window, cx);
        cx.notify();
    }

    /// Show the menu of a step at the pointer: "Move Up", "Move Down", and
    /// "Remove".
    fn open_step_menu(
        &mut self,
        list: StepList,
        item: usize,
        count: usize,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The menu sends its action to the focus: the root element.
        self.close_kinds();
        self.focus_workspace(window, cx);
        self.form.nested.step_menu = Some((list, item));
        NativeMenu::new()
            .menu_with_disabled("Move Up", item == 0, Box::new(MoveStepUp))
            .menu_with_disabled("Move Down", item + 1 >= count, Box::new(MoveStepDown))
            .separator()
            .menu("Remove", Box::new(RemoveStep))
            .show(position, window, cx);
        cx.notify();
    }

    /// A nested card for the contained action at `path` (section 10.5). With
    /// no action, it is a dashed card "Add an action". `remove` adds
    /// "Remove the action" to its kind menu. `accessory` is at the end of
    /// the head of the open card.
    pub(super) fn nested_card(
        &self,
        path: ActionPath,
        action: Option<&Action>,
        remove: bool,
        accessory: Option<AnyElement>,
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
        let open = self.form.nested.open.contains(&path);
        let body_path = path.clone();
        let card =
            card.action(self.kind_label(action, available))
                .open(open)
                .summary(if open {
                    self.summary(action)
                } else {
                    self.short_value(action)
                })
                .on_toggle(cx.listener(move |view, _, window, cx| {
                    view.toggle_nested(path.clone(), window, cx)
                }));
        let card = match accessory {
            Some(accessory) => card.accessory(accessory),
            None => card,
        };
        if !open {
            return card.into_any_element();
        }
        card.lead(self.step_lists(&body_path, action, available, window, cx))
            .children(self.action_rows(&body_path, action, available, window, cx))
            .into_any_element()
    }

    /// The step lists of the fields of the input type Action list of
    /// `action` at `path`, one below the other. `None` if the kind has no
    /// such field.
    pub(super) fn step_lists(
        &self,
        path: &[Slot],
        action: &Action,
        available: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !available {
            return None;
        }
        let kind = self.kind(&action.kind)?;
        let lists: Vec<AnyElement> = kind
            .fields
            .iter()
            .filter(|f| f.input.is_action_list())
            .map(|field| {
                self.step_list(
                    StepList {
                        path: path.to_vec(),
                        key: field.key.clone(),
                    },
                    action.settings.get(&field.key),
                    window,
                    cx,
                )
            })
            .collect();
        (!lists.is_empty()).then(|| div().flex().flex_col().children(lists).into_any_element())
    }

    /// The step list of `list` (section 10.5): one row for each step, and
    /// the button "Add a step".
    fn step_list(
        &self,
        list: StepList,
        value: Option<&Value>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let items = value.and_then(Action::list_from_value);
        let id = format!("{}{}", prefix(&list.path), list.key);
        let count = items.as_ref().map_or(0, Vec::len);
        let mut rows = div().flex().flex_col();
        if value.is_some() && items.is_none() {
            rows = rows.child(
                div()
                    .px(theme::SPACE_8)
                    .py(theme::SPACE_6)
                    .border_b_1()
                    .border_color(theme::LINE_SECTION)
                    .child(unreadable()),
            );
        }
        for (item, inner) in items.iter().flatten().enumerate() {
            rows = rows.child(self.step_row(&list, item, inner, count, window, cx));
        }
        let menu = Menu::Add(list.path.clone(), list.key.clone());
        let toggle = menu.clone();
        let menu_element = self.kind_menu(
            &format!("{id}/add-kind"),
            &menu,
            false,
            theme::HEIGHT_ADD_STEP,
            window,
            cx,
        );
        let add_id = format!("{id}/add");
        let menu_open = menu_element.is_some();
        rows.child(
            div().px(theme::SPACE_8).py(theme::SPACE_6).flex().child(
                div()
                    .relative()
                    .flex()
                    .child(
                        div()
                            .id(SharedString::from(add_id.clone()))
                            .debug_selector(move || add_id.clone())
                            .h(theme::HEIGHT_ADD_STEP)
                            .px(theme::SPACE_8)
                            .flex()
                            .items_center()
                            .gap(theme::SPACE_6)
                            .rounded(theme::RADIUS_CONTROL)
                            .border_1()
                            .border_dashed()
                            .border_color(if menu_open {
                                theme::ACCENT_LINE
                            } else {
                                theme::BORDER_OVERLAY
                            })
                            .text_size(theme::TEXT_LABEL)
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme::TEXT_SECONDARY)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme::HOVER_ROW))
                            .active(|d| d.bg(theme::PRESSED))
                            .accessible(Role::Button, ADD_A_STEP)
                            .aria_expanded(menu_open)
                            .child(Icon::new(IconName::Plus).size(theme::ICON_KIND_NESTED))
                            .child(ADD_A_STEP)
                            .map(|d| controls::keyboard_focus(d, controls::Ring::Inside))
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.toggle_menu(toggle.clone(), window, cx)
                            })),
                    )
                    .children(menu_element),
            ),
        )
        .into_any_element()
    }

    /// The drag handle and the number of a step.
    fn step_handle(&self, list: &StepList, item: usize, label: &KindLabel) -> impl IntoElement {
        let item_id = prefix(&list.step(item));
        let handle_id = format!("{item_id}handle");
        let drag = self.form_binding().map(|binding| StepDrag {
            binding,
            list: list.clone(),
            from: item,
            label: label.clone(),
        });
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(theme::SPACE_8)
            .child(
                div()
                    .id(SharedString::from(handle_id.clone()))
                    .debug_selector(move || handle_id.clone())
                    .flex_shrink_0()
                    .w(theme::WIDTH_DRAG_HANDLE)
                    .h(theme::HEIGHT_DRAG_HANDLE)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme::TEXT_DISABLED)
                    .cursor_grab()
                    // The label of decision 0029, section 10.4. The handle
                    // has no click: it is a picture for VoiceOver.
                    .accessible(Role::Image, DRAG_TO_REORDER)
                    .tooltip(|window, cx| Tooltip::new(DRAG_TO_REORDER).build(window, cx))
                    .child(Icon::new(IconName::GripVertical).size(theme::HEIGHT_DRAG_HANDLE))
                    .when_some(drag, |d, drag| {
                        d.on_drag(drag, |drag, offset, _, cx| {
                            cx.new(|_| StepDragPicture {
                                label: drag.label.clone(),
                                offset,
                            })
                        })
                    }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .w(theme::WIDTH_STEP_NUMBER)
                    .text_size(theme::TEXT_CAPTION)
                    .text_color(theme::TEXT_TERTIARY)
                    .child((item + 1).to_string()),
            )
    }

    /// The row of one step: closed, the kind and its short value; open, a
    /// nested card in its place. A drop of a step of the same list moves
    /// that step to this position.
    fn step_row(
        &self,
        list: &StepList,
        item: usize,
        inner: &Action,
        count: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let path = list.step(item);
        let item_id = prefix(&path);
        let row_id = format!("{item_id}step");
        let available = self.kind(&inner.kind).is_some();
        let label = self.kind_label(inner, available);
        let open = self.form.nested.open.contains(&path);
        let target = list.clone();
        let accepts = list.clone();
        let dropped = list.clone();
        let binding = self.form_binding();
        let row = div()
            .id(SharedString::from(row_id.clone()))
            .debug_selector(move || row_id.clone())
            .flex_shrink_0()
            .px(theme::SPACE_8)
            .flex()
            .gap(theme::SPACE_8)
            .border_b_1()
            .border_color(theme::LINE_SECTION)
            .drag_over::<StepDrag>(move |style, drag, _, _| {
                if drag.list == target && Some(drag.binding) == binding {
                    style.bg(theme::ACCENT_TINT_DRAG)
                } else {
                    style
                }
            })
            .can_drop(move |value, _, _| {
                value
                    .downcast_ref::<StepDrag>()
                    .is_some_and(|drag| drag.list == accepts && Some(drag.binding) == binding)
            })
            .on_drop(cx.listener(move |view, drag: &StepDrag, window, cx| {
                cx.stop_propagation();
                view.move_step(dropped.clone(), drag.from, item, window, cx);
            }));
        let menu_list = list.clone();
        let step = format!("Step {}", item + 1);
        if !open {
            let toggle = path.clone();
            let value = self.short_value(inner);
            return row
                .accessible(
                    Role::Button,
                    if value.is_empty() {
                        format!("{step}: {}", label.name())
                    } else {
                        format!("{step}: {}, {value}", label.name())
                    },
                )
                .aria_expanded(false)
                .h(theme::HEIGHT_STEP_ROW)
                .items_center()
                .cursor_pointer()
                .hover(|d| d.bg(theme::HOVER_ROW))
                .child(self.step_handle(list, item, &label))
                .child(label.tile(TileSize::Nested))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(theme::TEXT_CONTROL)
                        .when(!available, |d| d.text_color(theme::TEXT_TERTIARY))
                        .child(label.name().clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .max_w(gpui_kit::relative(0.45))
                        .truncate()
                        .text_size(theme::TEXT_LABEL)
                        .text_color(theme::TEXT_SECONDARY)
                        .child(value),
                )
                .map(|d| controls::keyboard_focus(d, controls::Ring::Inside))
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.toggle_nested(toggle.clone(), window, cx)
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        view.open_step_menu(
                            menu_list.clone(),
                            item,
                            count,
                            event.position,
                            window,
                            cx,
                        )
                    }),
                )
                .into_any_element();
        }
        let more_id = format!("{item_id}more");
        let more_list = list.clone();
        let more = div()
            .id(SharedString::from(more_id.clone()))
            .debug_selector(move || more_id.clone())
            .flex_shrink_0()
            .size(theme::HEIGHT_NESTED_KIND)
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme::RADIUS_CONTROL)
            .text_color(theme::TEXT_SECONDARY)
            .cursor_pointer()
            .hover(|d| d.bg(theme::HOVER_ICON))
            .active(|d| d.bg(theme::PRESSED))
            .accessible(Role::Button, MORE_FOR_THIS_STEP)
            .tooltip(|window, cx| Tooltip::new(MORE_FOR_THIS_STEP).build(window, cx))
            .child(Icon::new(IconName::Ellipsis).size(theme::ICON_BUTTON))
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    view.open_step_menu(more_list.clone(), item, count, event.position, window, cx)
                }),
            );
        // The handle and the number are at the height of the head of the
        // card: the kind button, its padding, and the two borders.
        let head = theme::HEIGHT_NESTED_KIND + theme::SPACE_6 * 2. + theme::SPACE_2;
        row.accessible(Role::Group, step)
            .aria_expanded(true)
            .py(theme::SPACE_6)
            .items_start()
            .child(
                div()
                    .h(head)
                    .flex()
                    .items_center()
                    .child(self.step_handle(list, item, &label)),
            )
            .child(div().flex_1().min_w_0().child(self.nested_card(
                path,
                Some(inner),
                false,
                Some(more.into_any_element()),
                window,
                cx,
            )))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_move_shifts_the_items_between_the_two_positions() {
        let order = |from, to| (0..4).map(|i| moved(i, from, to)).collect::<Vec<_>>();
        assert_eq!(order(0, 2), [2, 0, 1, 3]);
        assert_eq!(order(3, 1), [0, 2, 3, 1]);
        assert_eq!(order(1, 2), [0, 2, 1, 3]);
        assert_eq!(order(2, 2), [0, 1, 2, 3]);
        assert_eq!(steps_text(0), "No steps");
        assert_eq!(steps_text(1), "1 step");
        assert_eq!(steps_text(2), "2 steps");
    }
}
