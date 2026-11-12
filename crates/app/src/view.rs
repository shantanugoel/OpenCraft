//! The main window: the sidebar, the canvas, and the right panel of
//! decision 0029.
//!
//! Pixels and content come from engine events. Clicks send commands only.
//! Each part of the window has its own module:
//!
//! | Module | Part |
//! |---|---|
//! | `panes` | The order and the widths of the panes, the dividers, show and hide |
//! | `toolbar` | The toolbar above the canvas and the right panel |
//! | `sidebar` | The sidebar |
//! | `canvas` | The canvas: the page title, the device drawing |
//! | `status_bar` | The status bar and the messages below the canvas |
//! | `right_panel` | The right panel and its tabs |
//! | `inspector` | The "Inspector" tab: the editor of the selection |
//! | `library` | The "Library" tab |
//! | `actions` | The gesture cards of the selected control, and the settings form of the open card |
//! | `device`, `pages`, `hotkey` | The device settings, the changes of the pages and the profiles, the hotkey names |
//! | `form` | The parts of an inspector: the header, a section, the rows of a form |
//! | `gesture_card` | The gesture card |
//! | `controls` | Controls that more than one part uses |
//! | `theme` | The visual constants |

use crate::{
    app_menu::{ToggleInspector, ToggleSidebar, WORKSPACE},
    images,
    settings::Panes,
    shell::Shell,
    state::{ViewState, HIGHLIGHT_TIME},
};
use controls::{button, button_if, small_button};
use gpui_kit::base::{Scrollbar, ScrollbarMode};
use gpui_kit::{
    component::{
        input::{Input, InputEvent as TextEvent, InputState},
        tooltip::Tooltip,
    },
    div, img,
    prelude::*,
    px, AnyElement, ClickEvent, Context, Div, Entity, FocusHandle, PathPromptOptions, Render,
    RenderImage, ScrollHandle, SharedString, Stateful, Subscription, Task, Window,
};
use opencraft_engine::{
    Color, Command, ControlId, ControlKind, Event, Gesture, SurfaceId, SurfaceKind,
};
use std::{collections::BTreeMap, rc::Rc, sync::Arc, time::Instant};

mod actions;
mod canvas;
mod controls;
mod device;
mod form;
mod gesture_card;
mod hotkey;
mod inspector;
mod library;
mod pages;
mod panes;
mod right_panel;
mod sidebar;
mod status_bar;
pub(crate) mod theme;
mod toolbar;

/// The fill colours of a surface. These are data: the profile keeps them.
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
    /// The focus of the root element. The key commands of the window and the
    /// hotkey recorder need the focus in this element. After a click that
    /// takes the focus from a text field, the focus comes back here.
    pub(crate) focus: FocusHandle,
    input: Entity<InputState>,
    /// A label is initialized once per selection, or on its first surface event.
    label_loaded: bool,
    _input_subscription: Subscription,
    highlight_tasks: BTreeMap<ControlId, Task<()>>,
    /// The timer that changes "now" to the time of the last input.
    input_timer: Option<Task<()>>,
    /// True from a press on an empty part of the toolbar to the first move
    /// of the pointer, which then moves the window.
    window_move: bool,
    /// The panes of this window. The shell keeps them in `shell.toml`.
    pub(crate) panes: Panes,
    form: actions::ActionForm,
    inspector: inspector::InspectorState,
    lists: pages::Lists,
    device: device::DevicePanel,
    /// The scroll position of the inspector. It shows a scrollbar when its
    /// content is taller than the window.
    inspector_scroll: ScrollHandle,
}

impl DeviceView {
    pub fn new(
        shell: &Entity<Shell>,
        state: ViewState,
        panes: Panes,
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
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            state,
            shell: shell.clone(),
            _shell_subscription: shell_subscription,
            images,
            send,
            focus,
            input,
            label_loaded: false,
            _input_subscription: subscription,
            highlight_tasks: BTreeMap::new(),
            input_timer: None,
            window_move: false,
            panes,
            form: actions::ActionForm::default(),
            inspector: inspector::InspectorState::new(window, cx),
            lists: pages::Lists::new(window, cx),
            device: device::DevicePanel::new(),
            inspector_scroll: ScrollHandle::new(),
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
        if self.update_lists(&event) {
            self.focus_workspace(window, cx);
        }
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
        let input = matches!(event, Event::Input(_));
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
        if input {
            self.start_input_timer(window, cx);
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

    /// Give the focus back to the root element, for example after a click
    /// that ends an edit in a text field.
    fn focus_workspace(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
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
}

impl Render for DeviceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .key_context(WORKSPACE)
            .track_focus(&self.focus)
            .capture_key_down(
                cx.listener(|view, event, window, cx| view.record_key(event, window, cx)),
            )
            .on_action(cx.listener(|view, _: &ToggleSidebar, _, cx| view.toggle_sidebar(cx)))
            .on_action(cx.listener(|view, _: &ToggleInspector, window, cx| {
                view.toggle_inspector(window, cx)
            }))
            .map(|root| self.sidebar_actions(root, cx))
            .relative()
            .size_full()
            .bg(theme::BG_CANVAS)
            .text_color(theme::TEXT_PRIMARY)
            .text_size(theme::TEXT_BODY)
            .child(self.panes_layout(window, cx))
    }
}

#[cfg(test)]
mod tests;
