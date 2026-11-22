//! Copy, Paste, Clear, and Copy to All Pages of the selected control
//! (decision 0029, sections 8.3, 10.2, and 13).
//!
//! The engine keeps the clipboard and does the work (rule 12): the window
//! sends only the identity of the control. The commands come from the Edit
//! menu, from ⌘C, ⌘V, and Delete, from the "⋯" menu of the inspector header,
//! and from the right-click menu of a control on the canvas.
//!
//! The focus rule of section 13: a focused text field keeps ⌘C, ⌘V, and
//! Delete, because its key context is deeper than the workspace. A click on
//! an Edit menu item sends the workspace action also when a field has the
//! focus, thus the handler gives the action to the field then
//! (`docs/kb/gpui.md`, "Key commands in the window").

use super::*;
use crate::app_menu::{Clear, Copy, CopyToAllPages, Paste, PasteNotPossible};
use gpui_kit::{
    component::{input, native_menu::NativeMenu, WindowExt},
    Action, Pixels, Point,
};

/// The commands of the menus of a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClipboardItem {
    Copy,
    Paste,
    Clear,
    CopyToAllPages,
}

impl ClipboardItem {
    fn label(self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::Clear => "Clear",
            Self::CopyToAllPages => "Copy to All Pages",
        }
    }

    fn action(self) -> Box<dyn Action> {
        match self {
            Self::Copy => Box::new(Copy),
            Self::Paste => Box::new(Paste),
            Self::Clear => Box::new(Clear),
            Self::CopyToAllPages => Box::new(CopyToAllPages),
        }
    }
}

impl DeviceView {
    /// The control that the clipboard commands are for: the selected key,
    /// bar panel, or knob. The page, the device, and the touch bar have no
    /// clipboard commands. While the hotkey recorder waits for a key, the
    /// keys go to it.
    pub(super) fn clipboard_control(&self) -> Option<ControlId> {
        if self.device.open || self.state.page_selected || self.form.recording.is_some() {
            return None;
        }
        self.state
            .selected_control
            .filter(|c| c.kind != ControlKind::TouchBar)
    }

    /// True if the clipboard has a copy of a control of the kind of
    /// `control`.
    fn can_paste(&self, control: ControlId) -> bool {
        self.state.clipboard == Some(control.kind)
    }

    /// The surface that a paste of a system image changes: the selected key
    /// face or bar panel. A knob has no surface of its own.
    fn paste_surface(&self) -> Option<SurfaceId> {
        let control = self.clipboard_control()?;
        self.state
            .layout
            .as_ref()?
            .controls
            .iter()
            .find(|c| c.id == control)?
            .surface
    }

    /// True if the system pasteboard holds an item that is newer than the
    /// copy of a control. A copy of a control keeps the change count of its
    /// time, and a copy in another application changes it (decision 0043,
    /// section 5).
    fn system_image_is_newer(&self) -> bool {
        self.state.system_clipboard.has_image()
            && Some(self.state.system_clipboard.change_count)
                != self.state.control_copy_change_count
    }

    /// True if a paste on the selected surface changes its image to the
    /// item of the system pasteboard.
    fn can_paste_system_image(&self) -> bool {
        self.paste_surface().is_some() && self.system_image_is_newer()
    }

    /// True if `control` has an appearance or a binding on the active page.
    /// The parts are those of the engine: the surface of the control, or the
    /// feedback bar panel of a knob and the bar panel control of it.
    fn has_content(&self, control: ControlId) -> bool {
        let Some(layout) = &self.state.layout else {
            return false;
        };
        let Some(found) = layout.controls.iter().find(|c| c.id == control) else {
            return false;
        };
        let surface = found.surface.or(found.feedback_surface);
        let panel = found
            .surface
            .is_none()
            .then(|| {
                surface.and_then(|s| {
                    layout
                        .controls
                        .iter()
                        .find(|c| c.surface == Some(s))
                        .map(|c| c.id)
                })
            })
            .flatten();
        surface
            .and_then(|s| self.state.surfaces.get(&s))
            .is_some_and(|s| !s.content.is_empty())
            || self
                .state
                .bindings
                .keys()
                .any(|b| b.control == control || Some(b.control) == panel)
    }

    /// The items of the menu of `control`, with the enabled state of each.
    fn clipboard_items(&self, control: ControlId, all_pages: bool) -> Vec<(ClipboardItem, bool)> {
        let mut items = vec![
            (ClipboardItem::Copy, true),
            (
                ClipboardItem::Paste,
                self.can_paste(control) || self.can_paste_system_image(),
            ),
            (ClipboardItem::Clear, self.has_content(control)),
        ];
        // A folder is not a page of the cycle, so the copy cannot go to
        // every page (decision 0036).
        if all_pages {
            items.push((
                ClipboardItem::CopyToAllPages,
                self.state.location.folder().is_none(),
            ));
        }
        items
    }

    /// Add the handlers of the clipboard commands to the root element of
    /// the window. macOS enables an Edit menu item only if the element has a
    /// handler for it: thus Paste has a handler only when a paste is
    /// possible, and Clear only when the control has content.
    pub(super) fn clipboard_actions(
        &self,
        root: Stateful<Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        if window.focused_input(cx).is_some() {
            // From the Edit menu: the command is for the focused field.
            return root
                .on_action(cx.listener(|_, _: &Copy, window, cx| {
                    window.dispatch_action(Box::new(input::Copy), cx)
                }))
                .on_action(cx.listener(|_, _: &Paste, window, cx| {
                    window.dispatch_action(Box::new(input::Paste), cx)
                }))
                .on_action(cx.listener(|_, _: &Clear, window, cx| {
                    window.dispatch_action(Box::new(input::Delete), cx)
                }));
        }
        if self.form.recording.is_some() {
            return root;
        }
        let control = self.clipboard_control();
        root.on_action(
            cx.listener(|view, _: &PasteNotPossible, window, cx| {
                view.paste_not_possible(window, cx)
            }),
        )
        .when(control.is_some(), |d| {
            d.on_action(cx.listener(|view, _: &Copy, window, cx| {
                view.clipboard(ClipboardItem::Copy, window, cx)
            }))
        })
        .when(
            control.is_some() && self.state.location.folder().is_none(),
            |d| {
                d.on_action(cx.listener(|view, _: &CopyToAllPages, window, cx| {
                    view.clipboard(ClipboardItem::CopyToAllPages, window, cx)
                }))
            },
        )
        .when(
            control.is_some_and(|c| self.can_paste(c)) || self.can_paste_system_image(),
            |d| {
                d.on_action(cx.listener(|view, _: &Paste, window, cx| {
                    view.clipboard(ClipboardItem::Paste, window, cx)
                }))
            },
        )
        .when(control.is_some_and(|c| self.has_content(c)), |d| {
            d.on_action(cx.listener(|view, _: &Clear, window, cx| {
                view.clipboard(ClipboardItem::Clear, window, cx)
            }))
        })
    }

    /// Do `item` for the selected control.
    fn clipboard(&mut self, item: ClipboardItem, window: &mut Window, cx: &mut Context<Self>) {
        let Some(control) = self.clipboard_control() else {
            return;
        };
        let name = control_name(control);
        match item {
            ClipboardItem::Copy => {
                (self.send)(Command::CopyControl { control });
                // The paste tells which item is newer: the copy of the
                // control or the item of the system pasteboard.
                self.state.control_copy_change_count = Some(self.system_change_count());
                self.show_message(format!("{name} is copied"), window, cx);
            }
            ClipboardItem::Paste => {
                // The system item is newer than a copy of a control, or no
                // control is copied: it sets the image (decision 0043,
                // section 5).
                if self.paste_system_image(window, cx) {
                    return;
                }
                if !self.can_paste(control) {
                    self.paste_not_possible(window, cx);
                    return;
                }
                // The label field shows the new label when it comes.
                self.label_loaded = false;
                (self.send)(Command::PasteControl { control });
            }
            ClipboardItem::Clear => {
                self.label_loaded = false;
                (self.send)(Command::ClearControl { control });
            }
            ClipboardItem::CopyToAllPages => {
                (self.send)(Command::CopyToAllPages { control });
                self.show_message(format!("{name} is the same on each page now"), window, cx);
            }
        }
    }

    /// Set the image of `surface` from the system pasteboard. The bytes go
    /// to a temporary file, because the engine makes the copy (rule 9).
    /// The shell removes the file when the engine has read it, also when the
    /// image does not change and the engine sends no surface (decision 0043,
    /// section 5).
    fn paste_image(&mut self, surface: SurfaceId, image: ClipboardImage, cx: &mut Context<Self>) {
        let (path, temporary) = match image {
            // A path of the user is not a temporary file: the engine reads
            // it and the window keeps it.
            ClipboardImage::File(path) => (path, false),
            ClipboardImage::Bytes { bytes, extension } => {
                match pasteboard::temporary_image(&bytes, extension) {
                    Ok(path) => (path, true),
                    Err(error) => {
                        self.state
                            .problems
                            .push(format!("OpenCraft could not write the image: {error}"));
                        cx.notify();
                        return;
                    }
                }
            }
        };
        if temporary {
            let command = Command::SetImage {
                surface,
                path: Some(path.clone()),
            }
            .for_face(self.editing_on_face(surface));
            self.send_temporary_image(command, path, cx);
        } else {
            self.set_image_path(surface, path);
        }
    }

    /// Paste the item of the system pasteboard when it is possible. Return
    /// true if the item went to the surface. A path of an application bundle
    /// sets the action that opens it, as a drop of the bundle does
    /// (KEY-13). Any other image sets the image.
    fn paste_system_image(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.refresh_system_clipboard();
        let Some(surface) = self.paste_surface() else {
            return false;
        };
        if !self.system_image_is_newer() {
            return false;
        }
        // The window reads the bytes only now, when the user pastes.
        let Some(image) = self.state.system_clipboard.image() else {
            return false;
        };
        if let ClipboardImage::File(path) = &image {
            if crate::applications::is_application_bundle(path) {
                let path = path.clone();
                self.drop_application(surface, path, window, cx);
                return true;
            }
        }
        self.paste_image(surface, image, cx);
        true
    }

    /// ⌘V when a paste is not possible: tell the user why in the status bar.
    /// The pasteboard can have an image that the last frame did not see, thus
    /// this handler reads it again before it gives the message.
    fn paste_not_possible(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.paste_system_image(window, cx) {
            return;
        }
        let text = match (self.clipboard_control(), self.state.clipboard) {
            (None, _) => super::library::SELECT_FIRST.to_string(),
            (Some(_), None) => "Copy a control first".to_string(),
            (Some(_), Some(kind)) => {
                format!("You can paste a {0} only on a {0}", kind.name())
            }
        };
        self.show_message(text, window, cx);
    }

    /// Open the menu of the selected control at `position`: "Copy",
    /// "Paste", "Clear", and with `all_pages` also "Copy to All Pages".
    pub(super) fn open_control_menu(
        &mut self,
        all_pages: bool,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(control) = self.clipboard_control() else {
            return;
        };
        // A screenshot or a copy in another application does not draw a
        // frame. Read the pasteboard before the menu shows Paste.
        self.refresh_system_clipboard();
        // The menu sends its action to the focus: the root element.
        self.focus_workspace(window, cx);
        let mut menu = NativeMenu::new();
        for (item, enabled) in self.clipboard_items(control, all_pages) {
            if item == ClipboardItem::CopyToAllPages {
                menu = menu.separator();
            }
            menu = menu.menu_with_disabled(item.label(), !enabled, item.action());
        }
        menu.show(position, window, cx);
    }

    /// The "⋯" button of the inspector header of `control` (section 10.2).
    /// A click opens the menu of the control.
    pub(super) fn control_more_button(
        &self,
        control: ControlId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let enabled = self.clipboard_control() == Some(control);
        form::more_button(format!("More for {}", control_name(control)), enabled)
            .when(enabled, |d| {
                d.on_click(cx.listener(|view, event: &ClickEvent, window, cx| {
                    view.open_control_menu(true, event.position(), window, cx)
                }))
            })
            .into_any_element()
    }

    /// A right click on the control `control` of the canvas: select it, then
    /// open its menu at the pointer (section 8.3).
    pub(super) fn right_click_control(
        &mut self,
        control: ControlId,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if std::mem::take(&mut self.device.open) {
            cx.notify();
        }
        let surface = self
            .state
            .layout
            .as_ref()
            .and_then(|l| l.controls.iter().find(|c| c.id == control))
            .and_then(|c| c.surface);
        let changed = match surface {
            Some(id) => self.state.select(id),
            None => self.state.select_control(control),
        };
        if changed {
            self.close_kinds();
            self.sync_inspector_fields(window, cx);
            cx.notify();
        }
        self.open_control_menu(false, position, window, cx);
    }
}
