//! GPUI elements for the device and the surface editor.
//!
//! Pixels and content come from engine events. Clicks send commands only.

use crate::{
    images,
    shell::Shell,
    state::{ViewState, HIGHLIGHT_TIME},
};
use gpui_kit::base::{Scrollbar, ScrollbarMode};
use gpui_kit::{
    component::{
        input::{Input, InputEvent as TextEvent, InputState},
        tooltip::Tooltip,
    },
    div, img,
    prelude::*,
    px, rgb, AnyElement, ClickEvent, Context, Div, Entity, PathPromptOptions, Render, RenderImage,
    ScrollHandle, SharedString, Stateful, Subscription, Task, Window,
};
use opencraft_engine::{
    Color, Command, ControlId, ControlKind, Event, Gesture, SurfaceId, SurfaceKind,
};
use std::{collections::BTreeMap, rc::Rc, sync::Arc, time::Instant};

mod actions;
mod device;
mod hotkey;
mod pages;

const BACKGROUND: u32 = 0x191c22;
const PANEL: u32 = 0x232730;
const BORDER: u32 = 0x454d5b;
const TEXT: u32 = 0xeff2f7;
const MUTED: u32 = 0xaeb8c7;
const SELECTED: u32 = 0x89b9ff;
const ACTIVE: u32 = 0xffce73;
const DISABLED: u32 = 0x6b7482;
const DANGER: u32 = 0xb83f4c;
const COLOURS: [u32; 12] = [
    0x20242b, 0x596273, 0xd7dee8, 0xb83f4c, 0xcc7135, 0xc4a53e, 0x3d875a, 0x298886, 0x356ab6,
    0x6454ac, 0x9357a3, 0xb65c86,
];

fn colour(value: u32) -> Color {
    Color::rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

/// The name of a control for the user.
fn control_name(id: ControlId) -> String {
    match id.kind {
        ControlKind::Key => format!("Key {}", id.number),
        ControlKind::Knob => format!("Knob {}", id.number),
        ControlKind::BarPanel => format!("Bar panel {}", id.number),
        ControlKind::TouchBar => "Touch bar".into(),
    }
}

fn surface_name(id: SurfaceId) -> String {
    format!(
        "{} {}",
        if id.kind == SurfaceKind::KeyFace {
            "Key face"
        } else {
            "Bar panel"
        },
        id.number
    )
}

/// A small command interface lets window tests inspect commands without a device.
///
/// The view starts from the state of the [`Shell`], and then gets each
/// engine event from it (decision 0020). The view keeps its own copy because
/// it also does work for each event in the window: the images, the text
/// fields, and the changes that wait for a reply.
pub struct DeviceView {
    pub(crate) state: ViewState,
    shell: Entity<Shell>,
    _shell_subscription: Subscription,
    images: BTreeMap<SurfaceId, Arc<RenderImage>>,
    send: Rc<dyn Fn(Command)>,
    input: Entity<InputState>,
    /// A label is initialized once per selection, or on its first surface event.
    label_loaded: bool,
    _input_subscription: Subscription,
    highlight_tasks: BTreeMap<ControlId, Task<()>>,
    form: actions::ActionForm,
    lists: pages::Lists,
    device: device::DevicePanel,
    /// The scroll position of the editor at the right. The editor shows a
    /// scrollbar when its content is taller than the window.
    editor_scroll: ScrollHandle,
}

impl DeviceView {
    pub fn new(
        shell: &Entity<Shell>,
        state: ViewState,
        send: Rc<dyn Fn(Command)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Label"));
        let subscription = cx.subscribe_in(&input, window, |view, input, event, _, cx| {
            if matches!(event, TextEvent::Change) {
                if let Some(surface) = view.state.selected {
                    let label = input.read(cx).value().to_string();
                    (view.send)(Command::SetLabel {
                        surface,
                        label: if label.is_empty() { None } else { Some(label) },
                    });
                    // A later engine reply must not reset the cursor or the text.
                    view.label_loaded = true;
                }
            }
        });
        let shell_subscription = cx.subscribe_in(shell, window, |view, _, event, window, cx| {
            view.receive(event.clone(), window, cx)
        });
        let images = state
            .surfaces
            .iter()
            .map(|(id, surface)| (*id, images::from_rgb(&surface.image)))
            .collect();
        Self {
            state,
            shell: shell.clone(),
            _shell_subscription: shell_subscription,
            images,
            send,
            input,
            label_loaded: false,
            _input_subscription: subscription,
            highlight_tasks: BTreeMap::new(),
            form: actions::ActionForm::default(),
            lists: pages::Lists::new(window, cx),
            device: device::DevicePanel::new(),
            editor_scroll: ScrollHandle::new(),
        }
    }

    fn receive(&mut self, event: Event, window: &mut Window, cx: &mut Context<Self>) {
        if let Event::Binding(binding) = &event {
            self.acknowledge(binding);
        }
        if matches!(event, Event::Pages { .. } | Event::Profiles { .. }) {
            self.forget_pending();
        }
        if matches!(event, Event::Pages { .. }) {
            // The surfaces of the new page came before this event.
            self.label_loaded = false;
        }
        self.update_lists(&event);
        if let Event::Surface(surface) = &event {
            if let Some(old) = self
                .images
                .insert(surface.id, images::from_rgb(&surface.image))
            {
                let _ = window.drop_image(old);
            }
        }
        if let Event::Layout(layout) = &event {
            let removed: Vec<_> = self
                .images
                .keys()
                .copied()
                .filter(|id| layout.surface(*id).is_none())
                .collect();
            for id in removed {
                if let Some(old) = self.images.remove(&id) {
                    let _ = window.drop_image(old);
                }
            }
            self.highlight_tasks.clear();
        }
        if matches!(event, Event::Disconnected) {
            self.highlight_tasks.clear();
        }
        let settings = matches!(event, Event::Layout(_) | Event::DeviceSettings(_));
        if let Some((control, deadline)) = self.state.apply(event, Instant::now()) {
            let task = cx.spawn_in(window, async move |view, cx| {
                cx.background_executor().timer(HIGHLIGHT_TIME).await;
                let _ = view.update_in(cx, |view, _, cx| {
                    view.state.expire(control, deadline);
                    cx.notify();
                });
            });
            self.highlight_tasks.insert(control, task);
        }
        if !self.label_loaded {
            self.load_label(window, cx);
        }
        if settings {
            self.sync_device_panel(window, cx);
        }
        self.sync_form(window, cx);
        cx.notify();
    }

    /// Close a problem in this window and in the state of the shell.
    fn dismiss_problem(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.state.problems.len() {
            let text = self.state.problems.remove(index);
            self.shell
                .update(cx, |shell, _| shell.dismiss_problem(&text));
        }
        cx.notify();
    }

    fn load_label(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let content = self
            .state
            .selected
            .and_then(|id| self.state.surfaces.get(&id));
        let label = content
            .and_then(|s| s.content.label.clone())
            .unwrap_or_default();
        self.label_loaded = content.is_some();
        self.input
            .update(cx, |input, cx| input.set_value(label, window, cx));
    }

    fn select(&mut self, id: SurfaceId, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.device.open) {
            cx.notify();
        }
        if self.state.select(id) {
            self.close_kinds();
            self.load_label(window, cx);
            self.sync_form(window, cx);
            window.blur(cx);
            cx.notify();
        }
    }

    fn set_colour(&mut self, color: Option<Color>) {
        if let Some(surface) = self.state.selected {
            (self.send)(Command::SetColor { surface, color });
        }
    }

    fn dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(surface) = self.state.selected else {
            return;
        };
        let epoch = self.state.epoch;
        // Keep the surface captured here, even if selection changes during the dialog.
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select an image".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = paths.await;
            let _ = view.update_in(cx, |view, _, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        // A page change during the dialog drops the image: the
                        // command would change a different page.
                        if let Some(path) = paths
                            .into_iter()
                            .next()
                            .filter(|_| view.state.epoch == epoch)
                        {
                            // The engine reads, checks, and copies the file on its thread.
                            (view.send)(Command::SetImage {
                                surface,
                                path: Some(path),
                            });
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => view
                        .state
                        .problems
                        .push("OpenCraft could not open the file dialog.".into()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn surface(
        &self,
        surface: opencraft_engine::SurfaceLayout,
        scale: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = surface.id;
        let active = self.state.surface_active(id);
        div()
            .id(format!("surface-{id}"))
            .debug_selector(move || format!("surface-{id}"))
            .absolute()
            .left(px(f32::from(surface.x) * scale))
            .top(px(f32::from(surface.y) * scale))
            .w(px(f32::from(surface.width) * scale))
            .h(px(f32::from(surface.height) * scale))
            .bg(rgb(0x080a0d))
            .cursor_pointer()
            // An inset overlay keeps the engine image at the exact layout size.
            .children(
                self.images
                    .get(&id)
                    .cloned()
                    .map(|image| img(image).size_full()),
            )
            .child(
                div()
                    .absolute()
                    .size_full()
                    .top_0()
                    .left_0()
                    .border_3()
                    .border_color(rgb(if active {
                        ACTIVE
                    } else if self.state.selected == Some(id) {
                        SELECTED
                    } else {
                        BORDER
                    })),
            )
            .on_click(cx.listener(move |view, _, window, cx| view.select(id, window, cx)))
            .into_any_element()
    }

    fn device(&self, width: f32, height: f32, cx: &mut Context<Self>) -> AnyElement {
        let Some(layout) = &self.state.layout else {
            return div().child("Load the device layout…").into_any_element();
        };
        let scale = ((width - 32.) / f32::from(layout.display_width.max(1)))
            .min((height - 150.) / f32::from(layout.display_height.max(1)))
            .max(0.1);
        let display_width = f32::from(layout.display_width) * scale;
        let display_height = f32::from(layout.display_height) * scale;
        let mut display = div()
            .relative()
            .w(px(display_width))
            .h(px(display_height))
            .bg(rgb(0x101216));
        for surface in layout
            .surfaces
            .iter()
            .filter(|s| matches!(s.id.kind, SurfaceKind::KeyFace | SurfaceKind::BarPanel))
        {
            display = display.child(self.surface(*surface, scale, cx));
        }
        if let Some((x, y)) = self.state.touch_position() {
            display = display.child(
                div()
                    .absolute()
                    .left(px(x * scale - 6.))
                    .top(px(y * scale - 6.))
                    .size(px(12.))
                    .rounded_full()
                    .bg(rgb(ACTIVE)),
            );
        }
        // Columns, not model-specific knob coordinates, determine the row.
        let columns = layout
            .controls
            .iter()
            .filter(|c| c.id.kind == ControlKind::Knob)
            .map(|c| c.column)
            .max()
            .unwrap_or(1)
            .max(1);
        let knob_row = div().relative().w(px(display_width)).h(px(76.)).children(
            layout
                .controls
                .iter()
                .filter(|c| c.id.kind == ControlKind::Knob)
                .map(|control| {
                    let diameter = 48.;
                    let center =
                        display_width * (f32::from(control.column) - 0.5) / f32::from(columns);
                    let angle = *self.state.knob_steps.get(&control.id).unwrap_or(&0) as f32
                        * std::f32::consts::TAU
                        / 12.;
                    let highlight = self.state.highlights.get(&control.id);
                    let id = control.id;
                    div()
                        .id(format!("control-{id}"))
                        .debug_selector(move || format!("control-{id}"))
                        .cursor_pointer()
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.select_control(id, window, cx)
                        }))
                        .absolute()
                        .left(px(center - diameter / 2.))
                        .top(px(12.))
                        .size(px(diameter))
                        .rounded_full()
                        .bg(rgb(PANEL))
                        .border_2()
                        .border_color(rgb(if highlight.is_some() {
                            ACTIVE
                        } else if self.state.selected_control == Some(id) {
                            SELECTED
                        } else {
                            BORDER
                        }))
                        .child(
                            div()
                                .absolute()
                                .left(px(20. + angle.sin() * 14.))
                                .top(px(20. - angle.cos() * 14.))
                                .size(px(5.))
                                .rounded_full()
                                .bg(rgb(
                                    if highlight.is_some_and(|h| h.gesture == Gesture::Push) {
                                        ACTIVE
                                    } else {
                                        TEXT
                                    },
                                )),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(52.))
                                .w_full()
                                .text_center()
                                .text_xs()
                                .text_color(rgb(MUTED))
                                .child(format!("Knob {}", control.id.number)),
                        )
                }),
        );
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .p_4()
            .bg(rgb(0x101216))
            .rounded_lg()
            .child(display)
            .children(self.touch_bar(layout, scale, display_width, cx))
            .child(knob_row)
            .into_any_element()
    }

    /// A strip below the bar panels for the touch bar control. The touch bar
    /// control has no surface, so the strip is the element that selects it.
    fn touch_bar(
        &self,
        layout: &opencraft_engine::DeviceLayout,
        scale: f32,
        display_width: f32,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let control = layout
            .controls
            .iter()
            .find(|c| c.id.kind == ControlKind::TouchBar)?;
        let panels = layout
            .surfaces
            .iter()
            .filter(|s| s.id.kind == SurfaceKind::BarPanel);
        let left = panels.clone().map(|s| s.x).min().unwrap_or(0);
        let right = panels
            .map(|s| s.x + s.width)
            .max()
            .unwrap_or(layout.display_width);
        let id = control.id;
        let highlight = self.state.highlights.get(&id).map(|h| h.gesture);
        let arrow = |gesture: Gesture, text: &'static str| {
            div()
                .text_color(rgb(if highlight == Some(gesture) {
                    ACTIVE
                } else {
                    MUTED
                }))
                .child(text)
        };
        Some(
            div()
                .relative()
                .w(px(display_width))
                .h(px(24.))
                .child(
                    div()
                        .id(format!("control-{id}"))
                        .debug_selector(move || format!("control-{id}"))
                        .absolute()
                        .left(px(f32::from(left) * scale))
                        .w(px(f32::from(right.saturating_sub(left)) * scale))
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .rounded_md()
                        .bg(rgb(PANEL))
                        .border_2()
                        .border_color(rgb(if highlight.is_some() {
                            ACTIVE
                        } else if self.state.selected_control == Some(id) {
                            SELECTED
                        } else {
                            BORDER
                        }))
                        .text_xs()
                        .cursor_pointer()
                        .child(arrow(Gesture::SwipeLeft, "◀"))
                        .child(div().text_color(rgb(MUTED)).child("Touch bar"))
                        .child(arrow(Gesture::SwipeRight, "▶"))
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.select_control(id, window, cx)
                        })),
                )
                .into_any_element(),
        )
    }

    fn select_control(&mut self, id: ControlId, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.device.open) {
            cx.notify();
        }
        if self.state.select_control(id) {
            self.close_kinds();
            self.sync_form(window, cx);
            window.blur(cx);
            cx.notify();
        }
    }

    /// The frame of the editor at the right, around `content`. The content
    /// scrolls, and a scrollbar shows when it is taller than the frame. Thus
    /// the user sees that more settings are below.
    fn editor_frame(&self, content: Stateful<Div>) -> AnyElement {
        div()
            .relative()
            .w(px(272.))
            .flex_shrink_0()
            .h_full()
            .rounded_lg()
            .bg(rgb(PANEL))
            .child(
                content
                    .size_full()
                    .track_scroll(&self.editor_scroll)
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4(),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(Scrollbar::vertical(&self.editor_scroll).mode(ScrollbarMode::Always)),
            )
            .into_any_element()
    }

    fn editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let content = if self.device.open {
            self.device_editor(cx)
        } else {
            self.surface_editor(cx)
        };
        self.editor_frame(content)
    }

    fn surface_editor(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut editor = div().id("editor").debug_selector(|| "editor".into());
        if let Some(control) = self.state.selected_control {
            // A control with no surface has an action only.
            if self.state.selected.is_none() {
                return editor
                    .child(div().text_lg().child(control_name(control)))
                    .when(control.kind == ControlKind::TouchBar, |d| {
                        d.child(
                            div().text_sm().text_color(rgb(MUTED)).child(
                                "A swipe has no position. It is the same on all bar panels.",
                            ),
                        )
                    })
                    .child(self.action_editor(cx));
            }
        }
        let Some(id) = self.state.selected else {
            return editor.child(div().text_lg().child("Editor")).child(
                div().text_color(rgb(MUTED)).child(
                    "Click a key, a knob, a bar panel, or the touch bar in the device picture.",
                ),
            );
        };
        let content = self.state.surfaces.get(&id).map(|s| &s.content);
        let current_colour = content.and_then(|c| c.color);
        editor = editor
            .child(div().text_lg().child(surface_name(id)))
            .child(self.action_editor(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child("Label")
                    .child(div().text_color(rgb(TEXT)).child(Input::new(&self.input))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child("Colour")
                            .child(
                                small_button(
                                    "no-colour",
                                    if current_colour.is_none() {
                                        "No colour ✓"
                                    } else {
                                        "No colour"
                                    },
                                    true,
                                )
                                .on_click(cx.listener(|view, _, _, _| view.set_colour(None))),
                            ),
                    )
                    .child(div().flex().flex_wrap().gap_2().children(
                        COLOURS.into_iter().enumerate().map(|(index, value)| {
                            div()
                                .id(format!("colour-{index}"))
                                .debug_selector(move || format!("colour-{index}"))
                                .size(px(32.))
                                .rounded_md()
                                .bg(rgb(value))
                                .border_2()
                                .border_color(rgb(if current_colour == Some(colour(value)) {
                                    SELECTED
                                } else {
                                    BORDER
                                }))
                                .cursor_pointer()
                                .on_click(cx.listener(move |view, _, _, _| {
                                    view.set_colour(Some(colour(value)))
                                }))
                        }),
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child("Image")
                    .child(div().text_sm().text_color(rgb(MUTED)).child(format!(
                        "This {} has {} image.",
                        if id.kind == SurfaceKind::KeyFace {
                            "key face"
                        } else {
                            "bar panel"
                        },
                        if content.is_some_and(|c| c.image.is_some()) {
                            "an"
                        } else {
                            "no"
                        },
                    )))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(button("choose-image", "Select image…").on_click(
                                cx.listener(|view, _, window, cx| view.dialog(window, cx)),
                            ))
                            .when(content.is_some_and(|c| c.image.is_some()), |d| {
                                d.child(button("remove-image", "Remove image").on_click(
                                    cx.listener(move |view, _, _, _| {
                                        (view.send)(Command::SetImage {
                                            surface: id,
                                            path: None,
                                        })
                                    }),
                                ))
                            }),
                    ),
            );
        editor
    }
}

/// A button with a small padding. A button that is not enabled shows dim
/// text; the caller adds no click handler to it.
fn small_button(
    id: impl Into<SharedString>,
    text: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let text: SharedString = text.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .text_sm()
        .when(enabled, |d| {
            d.cursor_pointer().hover(|d| d.bg(rgb(0x343c49)))
        })
        .when(!enabled, |d| d.text_color(rgb(DISABLED)))
        .child(text)
}

/// A button for a change that removes something.
fn danger_button(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Stateful<Div> {
    // GPUI panics if an element gets a second hover style.
    plain_button(id, text)
        .bg(rgb(0x5a2a31))
        .border_color(rgb(DANGER))
        .hover(|d| d.bg(rgb(0x6d3039)))
}

fn button(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Stateful<Div> {
    plain_button(id, text).hover(|d| d.bg(rgb(0x343c49)))
}

/// A button with no hover style.
fn plain_button(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Stateful<Div> {
    let id: SharedString = id.into();
    let text: SharedString = text.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .text_sm()
        .cursor_pointer()
        .child(text)
}

impl Render for DeviceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = window.viewport_size();
        let width = (f32::from(size.width) - 336.).max(400.);
        let problem_height = if self.state.problems.is_empty() {
            0.
        } else {
            136.
        };
        let height = (f32::from(size.height) - 216. - problem_height).max(200.);
        div()
            .when_some(self.form.focus.as_ref(), |d, focus| d.track_focus(focus))
            .capture_key_down(
                cx.listener(|view, event, window, cx| view.record_key(event, window, cx)),
            )
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xl().child("OpenCraft"))
                            .child(
                                div().text_sm().text_color(rgb(MUTED)).child(
                                    self.state
                                        .layout
                                        .as_ref()
                                        .map_or("Device".to_string(), |l| l.name.clone()),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_5()
                            .child(
                                small_button("open-device-settings", "Device settings", true)
                                    .when(self.device.open, |d| {
                                        d.border_color(rgb(SELECTED)).text_color(rgb(SELECTED))
                                    })
                                    .on_click(cx.listener(|view, _, window, cx| {
                                        view.toggle_device_panel(window, cx)
                                    })),
                            )
                            .child(self.profile_control(cx))
                            .child(
                                div()
                                    .id("connection")
                                    .text_sm()
                                    .text_color(rgb(if self.state.connected {
                                        0x8bcea6
                                    } else {
                                        MUTED
                                    }))
                                    .child(if self.state.connected {
                                        "● Connected"
                                    } else {
                                        "○ Not connected"
                                    })
                                    .when(!self.state.connected, |d| {
                                        d.tooltip(|window, cx| {
                                            Tooltip::new(
                                                "Connect the device with a USB cable. If VSD \
                                                 Craft runs, quit it.",
                                            )
                                            .build(window, cx)
                                        })
                                    }),
                            ),
                    ),
            )
            .child(self.page_row(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .justify_center()
                            .items_center()
                            .child(self.device(width, height, cx)),
                    )
                    .child(self.editor(cx)),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_4()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child(
                        self.state
                            .last_input
                            .clone()
                            .unwrap_or_else(|| "No input yet".into()),
                    )
                    .child(
                        "OpenCraft saves each change. You can make changes also when no device is \
                         connected.",
                    ),
            )
            .when(!self.state.problems.is_empty(), |d| {
                d.child(
                    div()
                        .id("problems")
                        .max_h(px(120.))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(self.state.problems.iter().enumerate().map(
                            |(index, problem)| {
                                div()
                                    .flex()
                                    .gap_3()
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(0x49372b))
                                    .text_sm()
                                    .child(div().flex_1().child(problem.clone()))
                                    .child(
                                        div()
                                            .id(("dismiss", index))
                                            .cursor_pointer()
                                            .child("Close")
                                            .on_click(cx.listener(move |view, _, _, cx| {
                                                view.dismiss_problem(index, cx)
                                            })),
                                    )
                            },
                        )),
                )
            })
            .children(self.profile_backdrop(cx))
    }
}

#[cfg(test)]
mod tests;
