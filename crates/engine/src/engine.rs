//! The engine logic: own the device, obey commands, and send events.
//!
//! [`Engine`] owns the [`Link`] to the device, the profile, and the image of
//! each surface. A shell sends a [`Command`] and gets an [`Event`]
//! (architecture rules 1 and 2). The messages contain data only.
//!
//! The engine holds the image of each surface, so it can draw all surfaces
//! again at any time (rule 10). It does this after each connection. The
//! device and the shell get the same image (rule 11).
//!
//! The logic has no thread of its own. A caller calls [`Engine::command`]
//! and [`Engine::poll`] in a loop: see [`crate::runtime`]. A test calls them
//! directly, with a fake transport and a fake clock (rule 7). The reasons
//! are in `docs/decisions/0011-engine-thread-model.md`.

use crate::layout::DeviceLayout;
use crate::profile::{Color, Profile, SurfaceContent};
use crate::render::{self, Size};
use crate::store::{BackupReason, ProfileStore};
use image::RgbImage;
use opencraft_core::image::{encode_for, jpeg_quality_for};
use opencraft_core::{
    decode, Clock, Connector, Decoded, Descriptor, Error as DeviceError, InputEvent, Link,
    LinkEvent, Surface, SurfaceId, SystemClock,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The time from the last change of the profile to the write of the file.
/// Many changes in a short time, for example the letters of a label, give
/// one write.
pub const SAVE_DELAY: Duration = Duration::from_millis(500);

/// The brightness of the displays, in percent. Phase 5 adds the device
/// settings.
pub const DEFAULT_BRIGHTNESS: u8 = 80;

/// A command from a shell to the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Set the label of a surface. `None` or an empty text removes it.
    SetLabel {
        surface: SurfaceId,
        label: Option<String>,
    },
    /// Set the colour of a surface. `None` removes it.
    SetColor {
        surface: SurfaceId,
        color: Option<Color>,
    },
    /// Set the image of a surface from an image file of the user. The engine
    /// makes a copy of the image in the profile. `None` removes the image.
    SetImage {
        surface: SurfaceId,
        path: Option<PathBuf>,
    },
    /// Write the profile if it changed, and stop the engine.
    Stop,
}

/// The state of one surface: its content in the profile, and the image that
/// the engine made from the content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceState {
    pub id: SurfaceId,
    pub content: SurfaceContent,
    /// The upright image. The device shows this same image.
    pub image: Arc<RgbImage>,
}

/// An event from the engine to a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The layout of the device model. It is the first event. It comes again
    /// if a device of a different model connects. Before the first
    /// connection, it is the layout of the default model.
    Layout(Arc<DeviceLayout>),
    /// The state of a surface: at the start for each surface, and after each
    /// change.
    Surface(SurfaceState),
    /// A device is connected, and the engine draws all surfaces. Before this
    /// event, no device is connected.
    Connected,
    /// The device is gone. The engine looks for it again.
    Disconnected,
    /// The user did a gesture on a control.
    Input(InputEvent),
    /// Something went wrong that the user must know. The text is for the
    /// user.
    Problem(String),
}

/// The engine logic. See the module documentation.
///
/// `S` is the function that gets each event. It must return quickly.
pub struct Engine<C: Connector, S: FnMut(Event), K: Clock = SystemClock> {
    link: Link<C, K>,
    store: ProfileStore,
    profile: Profile,
    /// False if the profile file could not be read. Then the engine does not
    /// write the file, because the write would replace it.
    can_save: bool,
    /// The model of the device: of the connected device, or of the last one,
    /// or the default model.
    descriptor: &'static Descriptor,
    /// The image of each surface that has a slot (rule 10).
    images: BTreeMap<SurfaceId, Arc<RgbImage>>,
    /// True if the connected device does not show all images.
    needs_draw: bool,
    /// The time of the last change of the profile that is not in the file.
    changed_at: Option<Instant>,
    /// True after a failed write of the profile, until a write is successful.
    save_failed: bool,
    /// The text of the last problem of the link, to tell the shell one time
    /// only about a problem that stays.
    link_problem: Option<String>,
    brightness: u8,
    sink: S,
}

impl<C: Connector, S: FnMut(Event), K: Clock> Engine<C, S, K> {
    /// Make the engine: load the profile from `store`, and make the image of
    /// each surface. `default_model` is the model for the layout until a
    /// device connects.
    ///
    /// The engine sends the layout and the state of each surface to `sink`.
    /// It does not connect to the device here: the first [`Self::poll`] does.
    pub fn new(
        link: Link<C, K>,
        store: ProfileStore,
        default_model: &'static Descriptor,
        sink: S,
    ) -> Self {
        let mut engine = Self {
            link,
            store,
            profile: Profile::default(),
            can_save: true,
            descriptor: default_model,
            images: BTreeMap::new(),
            needs_draw: false,
            changed_at: None,
            save_failed: false,
            link_problem: None,
            brightness: DEFAULT_BRIGHTNESS,
            sink,
        };
        engine.load_profile();
        engine.show_model();
        engine
    }

    /// The profile in memory.
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// The link to the device.
    pub fn link(&self) -> &Link<C, K> {
        &self.link
    }

    fn load_profile(&mut self) {
        match self.store.load() {
            Ok(loaded) => {
                self.profile = loaded.profile;
                if let Some(backup) = loaded.backup {
                    let reason = match backup.reason {
                        BackupReason::NewerSchema { found } => {
                            format!("it has the schema version {found}, from a newer OpenCraft")
                        }
                        BackupReason::Invalid(error) => format!("it has an error ({error})"),
                    };
                    self.problem(format!(
                        "OpenCraft could not use the profile file, because {reason}. \
                         The file is now {}. OpenCraft started with an empty profile.",
                        backup.path.display()
                    ));
                } else if let Err(e) = self.store.remove_unused_images(&self.profile) {
                    self.problem(format!("OpenCraft could not remove unused images: {e}"));
                }
            }
            Err(e) => {
                self.can_save = false;
                self.problem(format!(
                    "OpenCraft could not read the profile: {e}. \
                     It started with an empty profile, and it does not save changes."
                ));
            }
        }
    }

    /// Send the layout of the model, make the image of each surface, and
    /// send the state of each surface.
    fn show_model(&mut self) {
        (self.sink)(Event::Layout(Arc::new(DeviceLayout::from(self.descriptor))));
        self.images.clear();
        for surface in self.descriptor.slot_surfaces() {
            self.render(surface);
        }
    }

    /// Make the image of `surface` from the profile, keep it, and send the
    /// state of the surface.
    fn render(&mut self, surface: &Surface) {
        let content = self.profile.active_page().content(surface.id);
        let source = content
            .image
            .as_deref()
            .and_then(|path| match self.store.load_image(path) {
                Ok(image) => Some(image),
                Err(e) => {
                    self.problem(format!(
                        "OpenCraft could not show the image of {}: {e}",
                        surface.id
                    ));
                    None
                }
            });
        let size = Size {
            width: u32::from(surface.width),
            height: u32::from(surface.height),
        };
        let image = Arc::new(render::surface_image(size, &content, source.as_ref()));
        self.images.insert(surface.id, Arc::clone(&image));
        (self.sink)(Event::Surface(SurfaceState {
            id: surface.id,
            content,
            image,
        }));
    }

    /// Obey one command. Return false if the engine must stop.
    pub fn command(&mut self, command: Command) -> bool {
        match command {
            Command::SetLabel { surface, label } => self.change(surface, |c| c.label = label),
            Command::SetColor { surface, color } => self.change(surface, |c| c.color = color),
            Command::SetImage { surface, path } => {
                let image = match path {
                    None => None,
                    Some(path) => match self.store.import_image(&path) {
                        Ok(copy) => Some(copy),
                        Err(e) => {
                            self.problem(format!("OpenCraft could not use the image: {e}"));
                            return true;
                        }
                    },
                };
                self.change(surface, |c| c.image = image);
            }
            Command::Stop => return false,
        }
        true
    }

    /// Change the content of the surface `id`. If the content is different
    /// after the change, make the new image, send it to the shell, and draw
    /// it on the device.
    fn change(&mut self, id: SurfaceId, change: impl FnOnce(&mut SurfaceContent)) {
        let Some(surface) = self.descriptor.surface(id).filter(|s| s.slot.is_some()) else {
            self.problem(format!("This device has no {id} with content."));
            return;
        };
        if !self.profile.active_page_mut().update(id, change) {
            return;
        }
        self.changed_at = Some(self.now());
        self.render(surface);
        if self.link.is_connected() && !self.needs_draw {
            self.draw_one(surface);
        }
    }

    /// Wait up to `timeout` for the next event of the link, and do the work
    /// for it. Then write the profile if a write is due.
    ///
    /// Call this function in a loop. It sends the keep-alive (through the
    /// link), so the time between two calls must be short.
    pub fn poll(&mut self, timeout: Duration) {
        if self.needs_draw && self.link.is_connected() {
            self.draw_all();
        }
        match self.link.next_event(timeout) {
            Ok(LinkEvent::Connected) => {
                self.link_problem = None;
                let model = self.link.device().map(|d| d.descriptor());
                if let Some(model) = model.filter(|m| !std::ptr::eq(*m, self.descriptor)) {
                    self.descriptor = model;
                    self.show_model();
                }
                (self.sink)(Event::Connected);
                self.draw_all();
            }
            Ok(LinkEvent::Disconnected) => {
                self.needs_draw = false;
                (self.sink)(Event::Disconnected);
            }
            Ok(LinkEvent::Input(report)) => {
                if let Decoded::Input(event) = decode(self.descriptor, &report) {
                    (self.sink)(Event::Input(event));
                }
            }
            Ok(LinkEvent::Timeout) => {}
            Err(e) => self.link_error(&e),
        }
        self.save_if_due();
    }

    /// Send the start sequence and the image of each surface to the device
    /// (rule 10).
    fn draw_all(&mut self) {
        let mut encoded = Vec::with_capacity(self.images.len());
        for surface in self.descriptor.slot_surfaces() {
            if let Some(data) = self.encode(surface) {
                encoded.push(data);
            }
        }
        let brightness = self.brightness;
        let result = self.link.with_device(|device| {
            device.start(brightness)?;
            for (slot, data) in &encoded {
                device.send_image(*slot, data)?;
            }
            device.commit()
        });
        self.needs_draw = match result {
            Ok(()) => {
                self.link_problem = None;
                false
            }
            // The link drops the device and gives `Disconnected`.
            Err(DeviceError::Disconnected) => false,
            // Try again at the next poll. The link counts the errors, and it
            // drops the device if they do not stop.
            Err(e) => {
                self.link_error(&e);
                true
            }
        };
    }

    /// Send the image of one surface to the device.
    fn draw_one(&mut self, surface: &Surface) {
        let Some((slot, data)) = self.encode(surface) else {
            return;
        };
        let result = self.link.with_device(|device| {
            device.send_image(slot, &data)?;
            device.commit()
        });
        match result {
            Ok(()) | Err(DeviceError::Disconnected) => {}
            Err(e) => {
                // The state of the device is not known. Draw all again.
                self.link_error(&e);
                self.needs_draw = true;
            }
        }
    }

    /// The slot of `surface`, and its image in the form that the device
    /// needs.
    fn encode(&mut self, surface: &Surface) -> Option<(u8, Vec<u8>)> {
        let slot = surface.slot?;
        let image = self.images.get(&surface.id)?;
        match encode_for(surface, image, jpeg_quality_for(surface)) {
            Ok(data) => Some((slot, data)),
            Err(e) => {
                self.problem(format!(
                    "OpenCraft could not make the image of {}: {e}",
                    surface.id
                ));
                None
            }
        }
    }

    /// Write the profile if it changed and no change came for
    /// [`SAVE_DELAY`].
    fn save_if_due(&mut self) {
        let Some(changed_at) = self.changed_at else {
            return;
        };
        let now = self.now();
        if now.saturating_duration_since(changed_at) >= SAVE_DELAY {
            self.save(now);
        }
    }

    /// Write the profile now, if it has a change that is not in the file.
    /// Call this before the engine stops.
    pub fn finish(&mut self) {
        if self.changed_at.is_some() {
            let now = self.now();
            self.save(now);
        }
    }

    fn save(&mut self, now: Instant) {
        if !self.can_save {
            self.changed_at = None;
            return;
        }
        match self.store.save(&self.profile) {
            Ok(()) => {
                self.changed_at = None;
                self.save_failed = false;
            }
            Err(e) => {
                // Try again after the delay. Tell the shell one time.
                self.changed_at = Some(now);
                if !self.save_failed {
                    self.save_failed = true;
                    self.problem(format!("OpenCraft could not save the profile: {e}"));
                }
            }
        }
    }

    fn now(&self) -> Instant {
        self.link.clock().now()
    }

    fn problem(&mut self, text: String) {
        (self.sink)(Event::Problem(text));
    }

    /// Tell the shell about an error of the link. An error that stays gives
    /// one event, not one event for each try.
    fn link_error(&mut self, error: &DeviceError) {
        let text = format!("OpenCraft has a problem with the device: {error}");
        if self.link_problem.as_deref() != Some(&text) {
            self.link_problem = Some(text.clone());
            self.problem(text);
        }
    }
}
