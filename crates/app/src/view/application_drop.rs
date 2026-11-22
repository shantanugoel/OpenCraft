//! A drop of an application bundle from Finder onto a key face
//! (KEY-13, decision 0043, section 5).
//!
//! The drop sets the action kind of the engine that opens an application
//! (rule 12: the kind comes from `Event::Kinds`, never from a name in this
//! crate), with the path and the bundle identifier of the bundle (decision
//! 0037). It also sets the title of the key face to the name of the
//! application, and its image to the application icon.
//!
//! The engine decodes and copies the image (rule 9). Thus the window
//! writes the icon bytes to a temporary file and sends the existing
//! `Command::SetImage`, as a paste does: the file goes away when the engine
//! sends the surface, when a problem names it, or at the stop. The
//! commands are one step of the history, "Drop Application".

use super::library::{assigned_text, first_gesture};
use super::*;
use crate::applications;
use crate::pasteboard;
use opencraft_engine::{Action, Field, InputType, KindDescription, PathKind, Settings, Value};

/// The name of the step of the history that a drop of an application makes
/// (decision 0029, section 13).
pub(super) const DROP_APPLICATION_STEP: &str = "Drop Application";

impl DeviceView {
    /// The action kind that opens an application, with the keys of its path
    /// field and of its bundle identifier setting, from the descriptions of
    /// the engine (rule 8). The kind is the one with a path field of kind
    /// Application, and the description gives the setting of the bundle
    /// identifier.
    fn application_kind(&self) -> Option<(&KindDescription, &Field, String)> {
        self.state.kinds.iter().find_map(|kind| {
            kind.fields.iter().find_map(|field| match &field.input {
                InputType::Path {
                    kind: PathKind::Application { bundle_id },
                } if !bundle_id.is_empty() => Some((kind, field, bundle_id.clone())),
                _ => None,
            })
        })
    }

    /// The control of `surface` in the layout.
    fn control_of_surface(&self, surface: SurfaceId) -> Option<ControlId> {
        self.state
            .layout
            .as_ref()?
            .controls
            .iter()
            .find(|control| control.surface == Some(surface))
            .map(|control| control.id)
    }

    /// Set the action "Open an application" of the bundle at `path` on the
    /// Press gesture of the key face `surface`, with the icon and the name
    /// of the application. The canvas selected the key face first. That is
    /// one step of the history.
    pub(super) fn drop_application(
        &mut self,
        surface: SurfaceId,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(control) = self.control_of_surface(surface) else {
            return;
        };
        let Some((kind, field, bundle_setting)) = self.application_kind() else {
            self.show_message(
                "An application goes on a key only when the library has its action",
                window,
                cx,
            );
            return;
        };
        // Owned values: the step closure changes the view.
        let kind_name = kind.name.clone();
        let kind_id = kind.kind.clone();
        let field_key = field.key.clone();

        let application = applications::application_at(path);
        let mut action = self.default_action(&kind_id).unwrap_or_else(|| Action {
            kind: kind_id.clone(),
            settings: Settings::new(),
        });
        action.settings.insert(
            field_key,
            Value::Text(application.path.to_string_lossy().into_owned()),
        );
        match applications::bundle_identifier_at(&application.path) {
            Some(id) => {
                action.settings.insert(bundle_setting, Value::Text(id));
            }
            None => {
                action.settings.remove(&bundle_setting);
            }
        }

        // The first gesture of the control: Press for a key, Tap for a bar
        // panel (decision 0029, section 12).
        let Some((gesture, gesture_name)) = first_gesture(control.kind) else {
            return;
        };
        let binding = self.state.location.binding_id(control, gesture);
        let title = application.name.clone();
        // The icon goes through a temporary file, as the image of a paste
        // does (decision 0043, section 5). An icon that the system cannot
        // draw does not stop the action.
        let icon = applications::application_icon_png(&application.path).and_then(|bytes| {
            match pasteboard::temporary_image(&bytes, "png") {
                Ok(path) => Some(path),
                Err(error) => {
                    self.state
                        .problems
                        .push(format!("OpenCraft could not write the icon: {error}"));
                    None
                }
            }
        });
        self.drop_edits(&[binding]);
        self.one_step(DROP_APPLICATION_STEP, |view| {
            (view.send)(Command::SetBinding {
                binding,
                action: Some(action),
            });
            view.set_title(surface, Some(title));
            if let Some(path) = icon {
                view.pending_images.push(PendingImage {
                    path: path.clone(),
                    surface,
                });
                view.set_image_path(surface, path);
            }
        });
        self.sync_form(window, cx);
        self.show_message(assigned_text(control, &kind_name, gesture_name), window, cx);
        cx.notify();
    }
}
