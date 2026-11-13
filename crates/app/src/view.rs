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
//! | `canvas` | The canvas: the page header, the device drawing, the help line |
//! | `status_bar` | The status bar and the messages below the canvas |
//! | `right_panel` | The right panel and its tabs |
//! | `inspector` | The "Inspector" tab: the editor of the selection |
//! | `icon_picker` | The row "Icon" of "Appearance", and the icon picker popover |
//! | `page_inspector` | The inspector of the page, with the swipe gestures of the touch bar |
//! | `knob` | The inspector of a knob: the "Turn" card, and the section of its bar panel |
//! | `library` | The "Library" tab: the action kinds in their groups |
//! | `actions` | The gesture cards of the selected control, and the settings form of the open card |
//! | `nested` | The nested card of a contained action, and the step list of a sequence |
//! | `states` | The appearance of each state of a toggle |
//! | `device`, `pages`, `hotkey` | The state of the device settings, the changes of the pages and the profiles, the hotkey names |
//! | `device_inspector` | The inspector of the device: the header and the sections of the device settings |
//! | `form` | The parts of an inspector: the header, a section, the rows of a form |
//! | `gesture_card` | The gesture card |
//! | `history` | Undo and redo, and the text edits for the history |
//! | `clipboard` | Copy, Paste, Clear, and Copy to All Pages of the selected control |
//! | `profile_sheet` | The profile settings sheet, with the export and the import of a profile |
//! | `modal` | The modal layer of a sheet or the command palette, and the parts of a sheet |
//! | `palette` | The command palette, the search field of the toolbar, and the commands that only the palette shows |
//! | `permission_sheet` | The Accessibility request of the first action that needs it |
//! | `controls` | Controls that more than one part uses |
//! | `focus_return` | The focus goes back to the control that opened a popover, a sheet, or the palette |
//! | `theme` | The visual constants |

use crate::{
    app_menu::{Redo, ToggleInspector, ToggleSidebar, Undo, WORKSPACE},
    images,
    settings::Panes,
    shell::Shell,
    state::{ViewState, HIGHLIGHT_TIME},
};
use controls::{button, button_if, small_button, Accessible};
use gpui_kit::base::{Scrollbar, ScrollbarMode};
use gpui_kit::{
    component::{
        input::{Input, InputEvent as TextEvent, InputState},
        tooltip::Tooltip,
    },
    div, img,
    prelude::*,
    px, AnyElement, App, ClickEvent, Context, Div, Entity, FocusHandle, PathPromptOptions, Render,
    RenderImage, Role, ScrollHandle, SharedString, Stateful, Subscription, Task, Window,
};
use opencraft_engine::{
    Color, Command, ControlId, ControlKind, Event, Gesture, SurfaceId, SurfaceKind,
};
use std::{collections::BTreeMap, rc::Rc, sync::Arc, time::Instant};

mod actions;
pub(crate) mod canvas;
mod clipboard;
pub(crate) mod controls;
mod device;
mod device_inspector;
mod focus_return;
mod form;
mod gesture_card;
mod history;
mod hotkey;
mod icon_picker;
mod inspector;
mod knob;
mod library;
mod menu;
pub(crate) mod modal;
mod nested;
mod page_inspector;
mod pages;
mod palette;
mod panes;
mod permission_sheet;
mod profile_sheet;
mod right_panel;
mod sidebar;
mod states;
mod status_bar;
pub(crate) mod theme;
mod toolbar;

/// The fill colours of a surface. These are data: the profile keeps them.
use theme::FILL_COLOURS as COLOURS;

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
    /// The background image of the active page from the engine (rule 11).
    background_image: Option<Arc<RenderImage>>,
    send: Rc<dyn Fn(Command)>,
    /// The focus of the root element. The key commands of the window and the
    /// hotkey recorder need the focus in this element. After a click that
    /// takes the focus from a text field, the focus comes back here.
    pub(crate) focus: FocusHandle,
    input: Entity<InputState>,
    /// A label is initialized once per selection, or on its first surface event.
    label_loaded: bool,
    /// The text field of the open text edit (decision 0030). `None` when no
    /// text field that edits the profile has the focus.
    text_edit: Option<gpui_kit::EntityId>,
    _input_subscription: Subscription,
    highlight_tasks: BTreeMap<ControlId, Task<()>>,
    /// The timer that changes "now" to the time of the last input.
    input_timer: Option<Task<()>>,
    /// The message of the status bar that shows in place of the last input
    /// (decision 0029, section 6.1), and the timer that removes it.
    status_message: Option<SharedString>,
    message_timer: Option<Task<()>>,
    /// The panes of this window. The shell keeps them in `shell.toml`.
    pub(crate) panes: Panes,
    form: actions::ActionForm,
    inspector: inspector::InspectorState,
    lists: pages::Lists,
    page_inspector: page_inspector::PageInspector,
    device: device::DevicePanel,
    library: library::LibraryState,
    /// The profile settings sheet (decision 0029, section 16).
    sheet: profile_sheet::ProfileSheet,
    /// The command palette (decision 0029, section 15).
    palette: palette::Palette,
    /// The Accessibility request (decision 0029, section 18).
    permission: permission_sheet::PermissionSheet,
    faces: states::StateFaces,
    /// The scroll position of the inspector. It shows a scrollbar when its
    /// content is taller than the window.
    inspector_scroll: ScrollHandle,
    /// Gives the focus back when the element with the focus is not drawn
    /// any more, for example a button that a Space removed.
    _focus_lost: Subscription,
    /// Notes the control with the focus at each key, so that a closed
    /// overlay gives the focus back to its control (`focus_return`).
    _focus_return: Subscription,
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
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Title"));
        let subscription = cx.subscribe_in(&input, window, |view, input, event, _, cx| {
            view.text_edit_event(input.entity_id(), event, cx);
            if matches!(event, TextEvent::Change) {
                if let Some(surface) = view.state.edit_surface() {
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
        let background_image = state.background.image.as_deref().map(images::from_rgb);
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let focus_lost = cx.on_focus_lost(window, |view, window, cx| {
            // An overlay that went away (a popover with a field, the icon
            // picker, a sheet) gives the focus to the control that opened
            // it, if a key opened it.
            if let Some(opener) = focus_return::closed_opener(window) {
                return window.focus(&opener, cx);
            }
            match window.focus_lost_restore_target(cx) {
                Some(target) => window.focus(&target, cx),
                None => view.focus_workspace(window, cx),
            }
        });
        let focus_return = focus_return::track_keys(&focus, window, cx);
        let library = library::LibraryState::new(state.kinds.len(), window, cx);
        let permission = permission_sheet::PermissionSheet::new(cx);
        // The window notes each binding that it sends, for the Accessibility
        // request (`permission_sheet`).
        let send = permission_sheet::noting_send(send, permission.sent.clone());
        if state.permission_request.is_some() {
            window.focus(&permission.focus, cx);
        }
        Self {
            state,
            shell: shell.clone(),
            _shell_subscription: shell_subscription,
            images,
            background_image,
            send,
            focus,
            input,
            label_loaded: false,
            text_edit: None,
            _input_subscription: subscription,
            highlight_tasks: BTreeMap::new(),
            input_timer: None,
            status_message: None,
            message_timer: None,
            panes,
            form: actions::ActionForm::default(),
            inspector: inspector::InspectorState::new(window, cx),
            lists: pages::Lists::new(window, cx),
            page_inspector: page_inspector::PageInspector::new(window, cx),
            device: device::DevicePanel::new(window, cx),
            library,
            sheet: profile_sheet::ProfileSheet::new(window, cx),
            palette: palette::Palette::new(window, cx),
            permission,
            faces: states::StateFaces::new(window, cx),
            inspector_scroll: ScrollHandle::new(),
            _focus_lost: focus_lost,
            _focus_return: focus_return,
        }
    }

    fn receive(&mut self, event: Event, window: &mut Window, cx: &mut Context<Self>) {
        if let Event::Binding(binding) = &event {
            self.acknowledge(binding);
            self.permission_event(binding, window, cx);
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
        self.page_event(&event, window, cx);
        self.profile_sheet_event(&event, window, cx);
        self.state_images_event(&event, window);
        if let Event::Surface(surface) = &event {
            if let Some(old) = self
                .images
                .insert(surface.id, images::from_rgb(&surface.image))
            {
                let _ = window.drop_image(old);
            }
        }
        if let Event::Background(background) = &event {
            let new = background.image.as_deref().map(images::from_rgb);
            if let Some(old) = std::mem::replace(&mut self.background_image, new) {
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
        if let Event::Choices { id, options } = &event {
            self.form.accept_choices(*id, options.clone());
        }
        let settings = matches!(event, Event::Layout(_) | Event::DeviceSettings(_));
        let kinds = matches!(event, Event::Kinds(_));
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
        if kinds {
            self.sync_library(window, cx);
        }
        self.sync_form(window, cx);
        self.sync_page_name(window, cx);
        cx.notify();
    }

    /// Give the focus back to the root element, for example after a click
    /// that ends an edit in a text field. A control in the root element
    /// that has the keyboard focus keeps it: the key commands operate there
    /// too, and the next Tab starts from that control (`controls`,
    /// `keyboard_focus`).
    fn focus_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.end_text_edit(cx);
        // An overlay that a key opened gives the focus back to its control
        // (`focus_return`).
        if let Some(opener) = focus_return::opener_of_focus(window, cx) {
            window.focus(&opener, cx);
            return;
        }
        let control = self.focus.contains_focused(window, cx)
            && !self.focus.is_focused(window)
            && gpui_kit::component::WindowExt::focused_input(window, cx).is_none();
        if !control {
            window.focus(&self.focus, cx);
        }
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
            .edit_surface()
            .and_then(|id| self.state.surfaces.get(&id));
        let label = content
            .and_then(|s| s.content.label.clone())
            .unwrap_or_default();
        self.label_loaded = content.is_some();
        self.input
            .update(cx, |input, cx| input.set_value(label, window, cx));
    }

    fn set_colour(&mut self, color: Option<Color>) {
        if let Some(surface) = self.state.edit_surface() {
            (self.send)(Command::SetColor { surface, color });
        }
    }

    fn dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(surface) = self.state.edit_surface() else {
            return;
        };
        let epoch = self.state.epoch;
        // Keep the surface captured here, even if selection changes during the dialog.
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose".into()),
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
        focus_return::begin_frame();
        // The modal layer is not in the element of the key context of the
        // window, thus the key commands of the window do not operate while
        // it is open (`modal`).
        let sheet = self.profile_sheet(window, cx);
        let palette = self.command_palette(window, cx);
        let permission = self.permission_sheet(window, cx);
        div()
            .relative()
            .size_full()
            .child(self.workspace(window, cx))
            .children(sheet)
            .children(palette)
            .children(permission)
    }
}

impl DeviceView {
    /// The root element of the window content, with the key context
    /// `"Workspace"`: the panes, the key commands, and the menu actions.
    fn workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("workspace")
            .key_context(WORKSPACE)
            .track_focus(&self.focus)
            .capture_key_down(
                cx.listener(|view, event, window, cx| view.record_key(event, window, cx)),
            )
            // A click on a control gives the focus to this element, also
            // from a control with the keyboard focus: the ring goes away.
            .on_action(cx.listener(|view, _: &controls::FocusOwner, window, cx| {
                view.end_text_edit(cx);
                window.focus(&view.focus, cx);
            }))
            .on_action(cx.listener(|view, _: &ToggleSidebar, _, cx| view.toggle_sidebar(cx)))
            .on_action(cx.listener(|view, _: &ToggleInspector, window, cx| {
                view.toggle_inspector(window, cx)
            }))
            .map(|root| self.sidebar_actions(root, cx))
            .map(|root| self.clipboard_actions(root, window, cx))
            .map(|root| self.step_actions(root, cx))
            .map(|root| self.window_commands(root, cx))
            // With no handler, macOS disables the Edit menu item.
            .when(self.can_history(true, window, cx), |d| {
                d.on_action(
                    cx.listener(|view, _: &Undo, window, cx| view.history(true, window, cx)),
                )
            })
            .when(self.can_history(false, window, cx), |d| {
                d.on_action(
                    cx.listener(|view, _: &Redo, window, cx| view.history(false, window, cx)),
                )
            })
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
