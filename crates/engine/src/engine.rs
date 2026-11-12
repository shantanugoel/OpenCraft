//! The engine logic: own the device, obey commands, and send events.
//!
//! [`Engine`] owns the [`Link`] to the device, the profile, and the image of
//! each surface. A shell sends a [`Command`] and gets an [`Event`]
//! (architecture rules 1 and 2). The messages contain data only.
//!
//! The engine holds the image of each surface, so it can draw all surfaces
//! again at any time (rule 10). It does this after each connection and after
//! each wake of the computer from sleep. The device and the shell get the
//! same image (rule 11).
//!
//! The engine holds the number of the active page in memory. The device and
//! the shell show the active page, and the commands for a surface or a
//! binding change it. At the start, the active page is the first page. The
//! steps of a page change are in [`Engine::select_page`]. The reasons are in
//! `docs/decisions/0016-pages-and-engine-requests-from-a-handler.md`.
//!
//! The engine gets the profiles directory, and holds only the active profile
//! in memory. It offers only the profiles of the model of the device. The
//! steps of a change of the active profile are in [`Engine::select_profile`].
//! The reasons are in `docs/decisions/0018-many-profiles.md`.
//!
//! The engine holds the device settings of the model of the device, and
//! sends them to the device after each start sequence and at each change.
//! The steps are in [`Engine::set_device_settings`]. The reasons are in
//! `docs/decisions/0022-device-settings-and-stop-state.md`.
//!
//! The logic has no thread of its own. A caller calls [`Engine::command`]
//! and [`Engine::poll`] in a loop: see [`crate::runtime`]. A test calls them
//! directly, with a fake transport and a fake clock (rule 7). The reasons
//! are in `docs/decisions/0011-engine-thread-model.md`.

use crate::action::{
    Action, BindingId, BindingState, EngineRequest, Feedback, GestureKind, HandlerPort,
    KindDescription, Message, Request, Status,
};
use crate::device_settings::{self, DeviceSettings, SettingCommands};
use crate::layout::DeviceLayout;
use crate::profile::{Color, Page, Profile, SurfaceContent, DEFAULT_PROFILE_NAME};
use crate::profiles::{ProfileEntry, ProfilesDir};
use crate::render::{self, Size};
use crate::store::ProfileStore;
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

/// The time from the last change of the profile, or of the device settings,
/// to the write of the file. Many changes in a short time, for example the
/// letters of a label, give one write.
pub const SAVE_DELAY: Duration = Duration::from_millis(500);

/// Write one line about the device to standard error, with the time of the
/// system clock in seconds since 1970. The engine writes a line only for an
/// event that is not frequent: a connection, a disconnection, a wake of the
/// computer, a full draw, the stop, each problem for the shell, and the
/// detail of an error of the device.
/// A person can read the result of a test on the device from a log file.
fn log(text: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    eprintln!("opencraft-engine {:.3} {text}", now.as_secs_f64());
}

/// The text for the user about an error of the device: what occurred, and
/// what to do. The text of the device layer names internal things, for
/// example a return code of macOS, and it goes only to the log.
fn device_problem(error: &DeviceError) -> &'static str {
    match error {
        DeviceError::PermissionDenied => {
            "OpenCraft cannot open the device. A different application can hold it, for \
             example VSD Craft. Quit VSD Craft. Then disconnect the device, and connect it \
             again."
        }
        DeviceError::NoVendorInterface | DeviceError::DeviceNotFound => {
            "OpenCraft found the device, but cannot open it. Disconnect the device, and \
             connect it again."
        }
        DeviceError::Disconnected => "The device is disconnected. Connect the device again.",
        _ => {
            "OpenCraft has a problem with the device. Disconnect the device, and connect it \
             again. If the problem stays, the log file has the details."
        }
    }
}

/// A command from a shell to the engine.
///
/// The commands for a surface and for a binding change the active page.
/// Page numbers start at 0. A command with a number that is not a page gives
/// [`Event::Problem`] and changes nothing.
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
    /// Set or remove a binding. The page of `binding` must be the active
    /// page. If it is not, the engine changes nothing and gives a problem: the
    /// shell made the command for a page that the device does not show now.
    SetBinding {
        binding: BindingId,
        action: Option<Action>,
    },
    /// Make `page` the active page. The device shows it.
    SelectPage { page: usize },
    /// Add a page after the last page, and make it the active page. With
    /// `copy_of`, the new page is a copy of that page: its content and its
    /// bindings. With no `copy_of`, the new page is empty. The engine gives
    /// the name: see [`Profile::new_page_name`].
    AddPage { copy_of: Option<usize> },
    /// Give `page` the name `name`. A name with only spaces gives a problem.
    RenamePage { page: usize, name: String },
    /// Move `page` to the position `to` in the list of pages. The active page
    /// stays the same page, possibly with a new number.
    MovePage { page: usize, to: usize },
    /// Remove `page`. A profile keeps one page at the minimum. If `page` is
    /// the active page, the page after it becomes the active page. If it was
    /// the last page, the page before it becomes the active page.
    RemovePage { page: usize },
    /// Make the profile with the identity `profile` the active profile. The
    /// engine writes the old profile first if it changed. See
    /// [`Engine::select_profile`].
    SelectProfile { profile: String },
    /// Add a profile with the name `name` for the user, and make it the
    /// active profile. With `copy_of`, the new profile is a copy of the
    /// profile with that identity: its pages and its image files. With no
    /// `copy_of`, the new profile has one empty page. The engine makes the
    /// identity from the name: see [`ProfilesDir::new_id`]. A name with only
    /// spaces gives a problem.
    AddProfile {
        name: String,
        copy_of: Option<String>,
    },
    /// Give the profile `profile` the name `name` for the user. The identity
    /// does not change. A name with only spaces gives a problem.
    RenameProfile { profile: String, name: String },
    /// Move the directory of the profile `profile` into the directory
    /// `removed` of the profiles directory. The engine does not delete it.
    /// One profile for the model stays at the minimum. If `profile` is the
    /// active profile, the profile after it in the list becomes the active
    /// profile. If it was the last profile, the profile before it does.
    RemoveProfile { profile: String },
    /// Set the device settings of the model of the device. The command has
    /// all settings. The engine puts each value into its range, and then
    /// does the steps of [`Engine::set_device_settings`].
    SetDeviceSettings(DeviceSettings),
    /// Ask the handler to correct the status of this kind.
    Resolve { kind: String },
    /// Send the current counters in an event.
    ReadCounters,
    /// Stop the engine: write the profile and the device settings if they
    /// changed, make the device dark with the stop sequence, and close the
    /// device. See [`Engine::finish`].
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

/// Counts since this engine starts. A test of the device compares them, to
/// find lost input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counters {
    /// Each report from the device, also acknowledgements and unknown
    /// reports.
    pub reports_received: u64,
    /// Each input event that the engine decoded from a report.
    pub input_events: u64,
    /// Each Invoke for a gesture of the user that the engine sent to a
    /// handler. An Invoke that a handler asks for with
    /// [`EngineRequest::Invoke`] is not in this count: it is not input, and
    /// the count must agree with the input events.
    pub invokes_sent: u64,
}

/// An event from the engine to a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The layout of the device model. It is the first event. It comes again
    /// if a device of a different descriptor connects. Before the first
    /// connection, it is the layout of the default model.
    ///
    /// If the new descriptor has a different model identifier, the engine
    /// selects the profiles of that model. Then [`Event::Kinds`], each
    /// [`Event::Surface`], [`Event::Profiles`], [`Event::Pages`], and each
    /// [`Event::Binding`] follow, as at the start, and then
    /// [`Event::Connected`].
    Layout(Arc<DeviceLayout>),
    /// All kind descriptions, after the layout.
    Kinds(Vec<KindDescription>),
    /// The profiles of the model of the device, in the order of their
    /// identities, and the identity of the active profile.
    ///
    /// The engine sends this event at the start and after each change of
    /// the list or of the active profile. A rename, and the removal of a
    /// profile that is not active, send only this event.
    ///
    /// If the active profile changed, the engine sends [`Event::Surface`]
    /// for each surface before this event. Then [`Event::Pages`] comes, with
    /// the first page of the new profile as the active page, and then
    /// [`Event::Binding`] for each binding of that page. Thus a shell drops
    /// all that it holds about the old profile when it gets this event with
    /// a new `active`: the pages and the bindings come next.
    Profiles {
        profiles: Vec<ProfileEntry>,
        active: String,
    },
    /// The names of the pages in their order, and the number of the active
    /// page. Page numbers start at 0.
    ///
    /// The engine sends this event at the start and after each change of
    /// the pages or of the active page. When a shell gets it, the shell
    /// removes all binding states that it holds. Then the engine sends
    /// [`Event::Binding`] for each binding of the active page. Thus the shell
    /// holds the bindings of the active page only, with their current page
    /// number.
    ///
    /// If the active page changed, the engine sends [`Event::Surface`] for
    /// each surface before this event. After a change of the active profile,
    /// [`Event::Profiles`] comes between the surfaces and this event.
    Pages { names: Vec<String>, active: usize },
    /// A binding of the active page: after each [`Event::Pages`], and after
    /// a change of the binding. An unavailable kind stays. A state with no
    /// action tells that the binding is gone.
    Binding(BindingState),
    /// The latest status that a handler sends for a kind.
    Status { kind: String, status: Status },
    /// The response to `Command::ReadCounters`.
    Counters(Counters),
    /// All device settings of the model of the device: at the start, after
    /// each change, and after a change of the model. At the start and after
    /// a change of the model, it comes after the bindings. The layout tells
    /// which settings the model has: see [`DeviceLayout::settings`].
    DeviceSettings(DeviceSettings),
    /// The state of a surface of the active page: at the start for each
    /// surface, after each change, and for each surface after a change of
    /// the active page.
    Surface(SurfaceState),
    /// A device is connected, and the engine sent the image of each surface
    /// to it. Before this event, no device is connected.
    ///
    /// After a sleep of the computer, the engine sends [`Event::Disconnected`]
    /// and then this event again, with a full draw between them. The device
    /// possibly lost its images during the sleep, and an input event during
    /// the sleep, for example a release, is lost.
    Connected,
    /// The device is gone, or the computer slept. The engine looks for the
    /// device again, or draws it again. See [`Event::Connected`].
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
    profiles: ProfilesDir,
    /// The profiles of the model of the device, in the order of their
    /// identities. It always contains the active profile.
    entries: Vec<ProfileEntry>,
    /// The identity of the active profile.
    profile_id: String,
    /// The directory of the active profile.
    store: ProfileStore,
    /// The active profile.
    profile: Profile,
    /// The number of the active page. It is always a page of `profile`.
    active: usize,
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
    /// The device settings of the model of the device.
    settings: DeviceSettings,
    /// False if the settings file has an error or a newer schema version.
    /// Then the engine does not write the file until the user changes a
    /// setting.
    settings_can_save: bool,
    /// The time of the last change of the settings that is not in the file.
    settings_changed_at: Option<Instant>,
    /// True after a failed write of the settings, until a write is
    /// successful.
    settings_save_failed: bool,
    ports: Vec<HandlerPort>,
    feedback: BTreeMap<BindingId, (u64, Feedback)>,
    feedback_order: u64,
    counters: Counters,
    sink: S,
}

impl<C: Connector, S: FnMut(Event), K: Clock> Engine<C, S, K> {
    /// Make the engine: load the active profile of `default_model` from
    /// `profiles`, and make the image of each surface. `default_model` is
    /// the model for the layout until a device connects.
    ///
    /// The engine sends the layout, the state of each surface, the profiles,
    /// the pages, and the bindings to `sink`. It does not connect to the
    /// device here: the first [`Self::poll`] does.
    pub fn new(
        link: Link<C, K>,
        profiles: ProfilesDir,
        default_model: &'static Descriptor,
        sink: S,
    ) -> Self {
        Self::with_handlers(link, profiles, default_model, Vec::new(), sink)
    }

    /// Make the logic with non-blocking handler ports. No handler code runs here.
    pub fn with_handlers(
        link: Link<C, K>,
        profiles: ProfilesDir,
        default_model: &'static Descriptor,
        ports: Vec<HandlerPort>,
        sink: S,
    ) -> Self {
        let mut engine = Self {
            link,
            // `open_profiles` sets the active profile below.
            store: profiles.store(""),
            profiles,
            entries: Vec::new(),
            profile_id: String::new(),
            profile: Profile::default(),
            active: 0,
            can_save: true,
            descriptor: default_model,
            images: BTreeMap::new(),
            needs_draw: false,
            changed_at: None,
            save_failed: false,
            link_problem: None,
            settings: DeviceSettings::defaults(default_model.settings),
            settings_can_save: true,
            settings_changed_at: None,
            settings_save_failed: false,
            ports,
            feedback: BTreeMap::new(),
            feedback_order: 0,
            counters: Counters::default(),
            sink,
        };
        engine.open_profiles();
        engine.load_settings();
        engine.show_model();
        engine.send_profiles();
        engine.send_pages();
        engine.appear();
        engine.send_settings();
        engine
    }

    /// The device settings of the model of the device.
    pub fn device_settings(&self) -> &DeviceSettings {
        &self.settings
    }

    /// Read the settings of the model of the device from the settings file.
    /// A file with an error gives the defaults and a problem for the shell.
    fn load_settings(&mut self) {
        let support = self.descriptor.settings;
        self.settings_changed_at = None;
        self.settings_save_failed = false;
        match device_settings::load(self.profiles.dir(), self.descriptor.model, support) {
            Ok(settings) => {
                self.settings = settings;
                self.settings_can_save = true;
            }
            Err(problem) => {
                self.settings = DeviceSettings::defaults(support);
                self.settings_can_save = false;
                self.problem(format!(
                    "OpenCraft could not use the device settings file {}, because {problem}. \
                     It uses the default device settings. It does not change the file until \
                     you change a device setting.",
                    device_settings::settings_path(self.profiles.dir()).display()
                ));
            }
        }
    }

    fn send_settings(&mut self) {
        (self.sink)(Event::DeviceSettings(self.settings.clone()));
    }

    /// Set the device settings. This is [`Command::SetDeviceSettings`]. The
    /// engine puts each value into its range for the model. If the settings
    /// are then different:
    ///
    /// 1. It keeps the new settings, and sends [`Event::DeviceSettings`].
    /// 2. If a device is connected and shows all images, it sends only the
    ///    commands of the settings that changed: `LIG`, or `SETLB` and
    ///    `LBLIG`, or `QUCMD`. A setting that the model does not have sends
    ///    nothing. With no device, the next start sequence sends the new
    ///    settings.
    /// 3. It writes the settings file after [`SAVE_DELAY`], also if the file
    ///    had an error before.
    pub fn set_device_settings(&mut self, settings: DeviceSettings) {
        let support = self.descriptor.settings;
        let settings = settings.fit(support);
        if settings == self.settings {
            return;
        }
        let before = SettingCommands::new(&self.settings, support);
        let after = SettingCommands::new(&settings, support);
        self.settings = settings;
        self.settings_can_save = true;
        self.settings_changed_at = Some(self.now());
        self.send_settings();
        let changes: Vec<Vec<u8>> = before.changes(&after).into_iter().cloned().collect();
        if changes.is_empty() || !self.link.is_connected() || self.needs_draw {
            return;
        }
        let result = self.link.with_device(|device| {
            for command in changes {
                device.send(command)?;
            }
            Ok(())
        });
        match result {
            Ok(()) | Err(DeviceError::Disconnected) => {}
            Err(e) => {
                // The state of the device is not known. Draw all again, with
                // all settings.
                self.link_error(&e);
                self.needs_draw = true;
            }
        }
    }

    /// Write the settings file now, if the settings changed.
    fn save_settings(&mut self, now: Instant) {
        if self.settings_changed_at.is_none() {
            return;
        }
        if !self.settings_can_save {
            self.settings_changed_at = None;
            return;
        }
        match device_settings::save(self.profiles.dir(), self.descriptor.model, &self.settings) {
            Ok(()) => {
                self.settings_changed_at = None;
                self.settings_save_failed = false;
            }
            Err(e) => {
                // Try again after the delay. Tell the shell one time.
                self.settings_changed_at = Some(now);
                if !self.settings_save_failed {
                    self.settings_save_failed = true;
                    self.problem(format!("OpenCraft could not save the device settings: {e}"));
                }
            }
        }
    }

    /// The active profile in memory.
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// The identity of the active profile.
    pub fn active_profile(&self) -> &str {
        &self.profile_id
    }

    /// The number of the active page. Page numbers start at 0.
    pub fn active_page(&self) -> usize {
        self.active
    }

    /// The link to the device.
    pub fn link(&self) -> &Link<C, K> {
        &self.link
    }

    /// The input counts since this engine starts.
    pub fn counters(&self) -> Counters {
        self.counters
    }

    /// The page that the device shows.
    fn page(&self) -> &Page {
        &self.profile.pages()[self.active]
    }

    /// Each binding of the active page, with its action.
    fn page_bindings(&self) -> Vec<(BindingId, Action)> {
        let page = self.active;
        self.page()
            .bindings()
            .map(|((control, gesture), action)| {
                let binding = BindingId {
                    page,
                    control,
                    gesture,
                };
                (binding, action.clone())
            })
            .collect()
    }

    /// Send the list of pages, then the state of each binding of the active
    /// page. See [`Event::Pages`].
    fn send_pages(&mut self) {
        let names = self
            .profile
            .pages()
            .iter()
            .map(|p| p.name.clone())
            .collect();
        (self.sink)(Event::Pages {
            names,
            active: self.active,
        });
        for (binding, action) in self.page_bindings() {
            self.binding_event(binding, Some(action));
        }
    }

    /// Send Appear for each binding of the active page.
    fn appear(&self) {
        for (binding, action) in self.page_bindings() {
            let kind = action.kind.clone();
            self.request(&kind, Request::Appear { binding, action });
        }
    }

    /// Send Disappear for each binding of the active page, and remove all
    /// feedback. This is step 1 of a page change.
    fn disappear(&mut self) {
        for (binding, action) in self.page_bindings() {
            self.request(&action.kind, Request::Disappear { binding });
        }
        self.feedback.clear();
    }

    /// Make `page` the active page with the steps of a page change: see
    /// [`Self::select_page`]. `change` changes the pages, for example it adds
    /// a page, after step 1. Thus a handler never holds a binding with an old
    /// page number.
    fn change_page<R>(&mut self, change: impl FnOnce(&mut Profile) -> R, page: usize) {
        self.disappear();
        change(&mut self.profile);
        self.active = page;
        self.redraw();
        self.send_pages();
        self.appear();
    }

    /// Make the image of each surface for the active page, and write to the
    /// device only the images that are different from the images before.
    /// These are steps 2 and 3 of a page change.
    fn redraw(&mut self) {
        let before = std::mem::take(&mut self.images);
        let surfaces: Vec<&'static Surface> = self.descriptor.slot_surfaces().collect();
        for surface in &surfaces {
            self.render(surface);
        }
        let changed: Vec<&Surface> = surfaces
            .into_iter()
            .filter(|s| before.get(&s.id) != self.images.get(&s.id))
            .collect();
        if self.link.is_connected() && !self.needs_draw {
            self.draw(&changed);
        }
    }

    /// Make `page` the active page. This is [`Command::SelectPage`]. The
    /// engine does the steps of a page change in the order of decision 0016:
    ///
    /// 1. It sends Disappear for each binding of the old page, and removes
    ///    all feedback.
    /// 2. It makes the image of each surface for the new page.
    /// 3. It writes to the device only the images that are different from
    ///    the images before, and then one commit. It does not send the start
    ///    sequence. If no image is different, it writes nothing.
    /// 4. It sends Appear for each binding of the new page.
    ///
    /// The shell gets the state of each surface, then [`Event::Pages`], then
    /// the state of each binding of the new page. A move or a removal of a
    /// page that changes the number of the active page does the same steps.
    pub fn select_page(&mut self, page: usize) {
        if !self.is_page(page) || page == self.active {
            return;
        }
        self.change_page(|_| {}, page);
    }

    /// True if `page` is a page of the profile. If it is not, tell the shell.
    fn is_page(&mut self, page: usize) -> bool {
        let found = page < self.profile.pages().len();
        if !found {
            self.problem(format!("The profile has no page {page}."));
        }
        found
    }

    fn add_page(&mut self, copy_of: Option<usize>) {
        let mut page = match copy_of {
            None => Page::new(""),
            Some(number) if self.is_page(number) => self.profile.pages()[number].clone(),
            Some(_) => return,
        };
        page.name = self.profile.new_page_name();
        let number = self.profile.pages().len();
        self.changed_at = Some(self.now());
        self.change_page(|profile| profile.add_page(page), number);
    }

    fn rename_page(&mut self, page: usize, name: String) {
        if !self.is_page(page) {
            return;
        }
        if name.trim().is_empty() {
            self.problem("A page must have a name.".to_string());
            return;
        }
        let Some(target) = self.profile.page_mut(page).filter(|p| p.name != name) else {
            return;
        };
        target.name = name;
        self.changed_at = Some(self.now());
        self.send_pages();
    }

    fn move_page(&mut self, page: usize, to: usize) {
        if !self.is_page(page) || !self.is_page(to) || page == to {
            return;
        }
        let active = self.active;
        let after = if active == page {
            to
        } else if page < active && to >= active {
            active - 1
        } else if page > active && to <= active {
            active + 1
        } else {
            active
        };
        self.changed_at = Some(self.now());
        self.change_pages(|profile| profile.move_page(page, to), after);
    }

    fn remove_page(&mut self, page: usize) {
        if !self.is_page(page) {
            return;
        }
        let count = self.profile.pages().len();
        if count == 1 {
            self.problem("A profile keeps one page at the minimum.".to_string());
            return;
        }
        let after = match page.cmp(&self.active) {
            std::cmp::Ordering::Less => self.active - 1,
            std::cmp::Ordering::Equal => page.min(count - 2),
            std::cmp::Ordering::Greater => self.active,
        };
        self.changed_at = Some(self.now());
        if page == self.active {
            // The same number can be a different page now.
            self.change_page(|profile| profile.remove_page(page), after);
        } else {
            self.change_pages(|profile| profile.remove_page(page), after);
        }
    }

    /// Change the list of pages with no change of the page that the device
    /// shows. If the number of the active page changes, do the steps of a
    /// page change, so that no handler holds an old page number. Else send
    /// only the new list of pages.
    fn change_pages<R>(&mut self, change: impl FnOnce(&mut Profile) -> R, active: usize) {
        if active == self.active {
            change(&mut self.profile);
            self.send_pages();
        } else {
            self.change_page(change, active);
        }
    }

    fn available(&self, kind: &str) -> bool {
        self.ports
            .iter()
            .any(|p| p.kinds.iter().any(|k| k.kind == kind))
    }

    fn request(&self, kind: &str, request: Request) -> bool {
        self.ports
            .iter()
            .find(|p| p.kinds.iter().any(|k| k.kind == kind))
            .is_some_and(|p| p.sender.send(request).is_ok())
    }

    fn binding_event(&mut self, binding: BindingId, action: Option<Action>) {
        let available = action.as_ref().is_some_and(|a| self.available(&a.kind));
        (self.sink)(Event::Binding(BindingState {
            binding,
            action,
            available,
        }));
    }

    fn set_binding(&mut self, binding: BindingId, action: Option<Action>) {
        if binding.page != self.active {
            self.problem(
                "OpenCraft did not change the binding, because its page is not the page \
                 that the device shows."
                    .to_string(),
            );
            return;
        }
        let Some(page) = self.profile.page_mut(binding.page) else {
            return;
        };
        let before = page.binding(binding.control, binding.gesture).cloned();
        if !page.set_binding(binding.control, binding.gesture, action.clone()) {
            return;
        }
        if let Some(before) = before {
            self.request(&before.kind, Request::Disappear { binding });
        }
        let had_feedback = self.feedback.remove(&binding).is_some();
        self.changed_at = Some(self.now());
        self.binding_event(binding, action.clone());
        if let Some(action) = action {
            let kind = action.kind.clone();
            self.request(&kind, Request::Appear { binding, action });
        }
        if had_feedback {
            self.refresh_feedback_surface(binding.control);
        }
    }

    fn invoke(&mut self, event: InputEvent) {
        let Some(gesture) = GestureKind::from_gesture(event.gesture) else {
            return;
        };
        let Some(action) = self.page().binding(event.control, gesture).cloned() else {
            return;
        };
        let binding = BindingId {
            page: self.active,
            control: event.control,
            gesture,
        };
        let kind = action.kind.clone();
        if self.request(&kind, Request::Invoke { binding, action }) {
            self.counters.invokes_sent += 1;
        }
    }

    fn feedback_surface(&self, control: opencraft_core::ControlId) -> Option<SurfaceId> {
        self.descriptor.control(control)?.feedback_surface()
    }

    fn refresh_feedback_surface(&mut self, control: opencraft_core::ControlId) {
        let Some(surface) = self
            .feedback_surface(control)
            .and_then(|id| self.descriptor.surface(id))
            .filter(|s| s.slot.is_some())
        else {
            return;
        };
        self.render(surface);
        if self.link.is_connected() && !self.needs_draw {
            self.draw(&[surface]);
        }
    }

    /// Accept one message from the shared handler queue.
    pub fn handler_message(&mut self, message: Message) {
        match message {
            Message::Feedback { binding, feedback } => {
                if binding.page != self.active
                    || self
                        .page()
                        .binding(binding.control, binding.gesture)
                        .is_none()
                    || self.feedback_surface(binding.control).is_none()
                {
                    return;
                }
                if let Some(mut feedback) = feedback {
                    feedback.level = feedback.level.map(|v| v.min(100));
                    self.feedback_order += 1;
                    self.feedback
                        .insert(binding, (self.feedback_order, feedback));
                } else {
                    self.feedback.remove(&binding);
                }
                self.refresh_feedback_surface(binding.control);
            }
            Message::Status { kind, status } => (self.sink)(Event::Status { kind, status }),
            Message::Problem(text) => self.problem(text),
            Message::Engine(EngineRequest::SelectPage(target)) => {
                let count = self.profile.pages().len();
                self.select_page(target.resolve(self.active, count));
            }
            Message::Engine(EngineRequest::Invoke { binding, action }) => {
                // The engine does not check the binding: a sequence can ask
                // after a page change, and the action must still occur.
                // Feedback for a binding of a different page does nothing.
                let kind = action.kind.clone();
                if !self.request(&kind, Request::Invoke { binding, action }) {
                    self.problem(format!(
                        "A toggle or a sequence contains an action that this version of \
                         OpenCraft does not have ({kind}). Select a different action in it."
                    ));
                }
            }
            Message::Engine(EngineRequest::SelectProfile { name }) => {
                // The list is in the order of the identities, so the first
                // profile with the name wins.
                match self.entries.iter().find(|e| e.name == name) {
                    Some(entry) => {
                        let id = entry.id.clone();
                        self.select_profile(&id);
                    }
                    None => self.problem(format!(
                        "OpenCraft did not change the profile, because no profile has the name \
                         '{name}'."
                    )),
                }
            }
        }
    }

    /// Find the profiles of the model of the device, and load the active
    /// profile. The active profile is the profile in `state.toml` for the
    /// model. If the file names no profile of the model, it is the first
    /// profile in the list. If the model has no profile, the engine makes one
    /// in memory. The first change, or a change of the active profile, writes
    /// it.
    fn open_profiles(&mut self) {
        let model = self.descriptor.model;
        self.entries = match self.profiles.list(model) {
            Ok(entries) => entries,
            Err(e) => {
                self.problem(format!("OpenCraft could not read the profiles: {e}"));
                Vec::new()
            }
        };
        let recorded = self
            .profiles
            .active(model)
            .filter(|id| self.entries.iter().any(|e| &e.id == id));
        match recorded.or_else(|| self.entries.first().map(|e| e.id.clone())) {
            Some(id) => self.load_profile(id),
            None => self.make_profile(),
        }
    }

    /// Make a new profile with no content for the model of the device, in
    /// memory, and make it the active profile.
    fn make_profile(&mut self) {
        let id = match self.profiles.new_id(DEFAULT_PROFILE_NAME, &[]) {
            Ok(id) => id,
            Err(e) => {
                self.problem(format!("OpenCraft could not read the profiles: {e}"));
                DEFAULT_PROFILE_NAME.to_lowercase()
            }
        };
        self.use_profile(
            id,
            Profile::for_model(self.descriptor.model, DEFAULT_PROFILE_NAME, Vec::new()),
            true,
        );
    }

    /// Load the profile `id` from its directory, and make it the active
    /// profile. A file that the engine cannot use gets the steps of decision
    /// 0009: the engine renames it to a backup name, tells the shell, and
    /// starts with an empty profile.
    fn load_profile(&mut self, id: String) {
        let store = self.profiles.store(&id);
        let model = self.descriptor.model;
        let empty = || Profile::for_model(model, DEFAULT_PROFILE_NAME, Vec::new());
        let (profile, can_save) = match store.load() {
            Ok(loaded) => match loaded.backup {
                Some(backup) => {
                    self.problem(format!(
                        "OpenCraft could not use the profile file, because {}. \
                         The file is now {}. OpenCraft started with an empty profile.",
                        backup.reason,
                        backup.path.display()
                    ));
                    (empty(), true)
                }
                None => {
                    if let Err(e) = store.remove_unused_images(&loaded.profile) {
                        self.problem(format!("OpenCraft could not remove unused images: {e}"));
                    }
                    (loaded.profile, true)
                }
            },
            Err(e) => {
                self.problem(format!(
                    "OpenCraft could not read the profile: {e}. \
                     It started with an empty profile, and it does not save changes."
                ));
                (empty(), false)
            }
        };
        self.use_profile(id, profile, can_save);
    }

    /// Make `profile` with the identity `id` the active profile, with its
    /// first page as the active page. Put it in the list of profiles.
    fn use_profile(&mut self, id: String, profile: Profile, can_save: bool) {
        self.store = self.profiles.store(&id);
        self.profile = profile;
        self.profile_id = id;
        self.active = 0;
        self.can_save = can_save;
        self.changed_at = None;
        self.save_failed = false;
        self.update_entry();
    }

    /// Put the name of the active profile in the list of profiles. Add the
    /// entry if the list does not have it.
    fn update_entry(&mut self) {
        let entry = ProfileEntry {
            id: self.profile_id.clone(),
            name: self.profile.name.clone(),
        };
        match self.entries.binary_search_by(|e| e.id.cmp(&entry.id)) {
            Ok(at) => self.entries[at] = entry,
            Err(at) => self.entries.insert(at, entry),
        }
    }

    /// Send the list of profiles and the identity of the active profile. See
    /// [`Event::Profiles`].
    fn send_profiles(&mut self) {
        (self.sink)(Event::Profiles {
            profiles: self.entries.clone(),
            active: self.profile_id.clone(),
        });
    }

    /// Make the profile `id` the active profile. This is
    /// [`Command::SelectProfile`]. The engine does these steps in this order
    /// (decision 0018):
    ///
    /// 1. It writes the old profile, if it changed or if it is not in a file
    ///    yet. If the write fails, the engine tells the shell and stays on
    ///    the old profile.
    /// 2. It does the steps of a page change of [`Self::select_page`], with
    ///    the first page of the new profile as the new page: Disappear for
    ///    each binding of the old page, the images of the new page, a write
    ///    of only the images that are different, and Appear for each binding
    ///    of the new page.
    /// 3. It writes the identity of the new profile in `state.toml`.
    ///
    /// The shell gets the state of each surface, then [`Event::Profiles`],
    /// then [`Event::Pages`], then the state of each binding of the new
    /// page.
    pub fn select_profile(&mut self, id: &str) {
        if id == self.profile_id {
            return;
        }
        if !self.is_profile(id) || !self.save_before_leaving() {
            return;
        }
        self.change_profile(id.to_string(), None);
    }

    /// True if `id` is a profile of the model. If it is not, tell the shell.
    fn is_profile(&mut self, id: &str) -> bool {
        let found = self.entries.iter().any(|e| e.id == id);
        if !found {
            self.problem(format!("The device model has no profile '{id}'."));
        }
        found
    }

    /// Write the active profile before the engine leaves it: if it changed,
    /// or if it is not in a file yet, for example a profile that the engine
    /// made. Return false if the write failed: then the engine tells the
    /// shell and stays on the profile.
    fn save_before_leaving(&mut self) -> bool {
        match self.write_now() {
            Ok(()) => true,
            Err(e) => {
                self.problem(format!(
                    "OpenCraft did not change the profile, because it could not save the \
                     active profile: {e}"
                ));
                false
            }
        }
    }

    /// The write of [`Self::save_before_leaving`], with no problem for the
    /// shell.
    fn write_now(&mut self) -> crate::Result<()> {
        if !self.can_save || (self.changed_at.is_none() && self.store.profile_path().exists()) {
            return Ok(());
        }
        self.store.save(&self.profile)?;
        self.changed_at = None;
        self.save_failed = false;
        Ok(())
    }

    /// Steps 2 and 3 of [`Self::select_profile`]. The caller wrote the old
    /// profile. With `profile`, the engine uses that profile in memory, and
    /// does not read the file.
    fn change_profile(&mut self, id: String, profile: Option<Profile>) {
        self.disappear();
        match profile {
            Some(profile) => self.use_profile(id, profile, true),
            None => self.load_profile(id),
        }
        self.redraw();
        self.send_profiles();
        self.send_pages();
        self.appear();
        if let Err(e) = self
            .profiles
            .set_active(self.descriptor.model, &self.profile_id)
        {
            self.problem(format!(
                "OpenCraft could not record the active profile: {e}"
            ));
        }
    }

    /// A name for the user that is not empty, or a problem for the shell.
    fn profile_name(&mut self, name: String) -> Option<String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            self.problem("A profile must have a name.".to_string());
            return None;
        }
        Some(name)
    }

    fn add_profile(&mut self, name: String, copy_of: Option<String>) {
        let Some(name) = self.profile_name(name) else {
            return;
        };
        let (mut profile, source) = match copy_of {
            None => (
                Profile::for_model(self.descriptor.model, "", Vec::new()),
                None,
            ),
            Some(id) if id == self.profile_id => (self.profile.clone(), Some(self.store.clone())),
            Some(id) if self.is_profile(&id) => {
                let source = self.profiles.store(&id);
                match source.read() {
                    Ok(profile) => (profile, Some(source)),
                    Err(e) => {
                        self.problem(format!("OpenCraft could not copy the profile: {e}"));
                        return;
                    }
                }
            }
            Some(_) => return,
        };
        profile.name = name;
        if !self.save_before_leaving() {
            return;
        }
        let id = match self.profiles.new_id(&profile.name, &[&self.profile_id]) {
            Ok(id) => id,
            Err(e) => {
                self.problem(format!("OpenCraft could not add the profile: {e}"));
                return;
            }
        };
        let store = self.profiles.store(&id);
        let written = source
            .map_or(Ok(()), |source| store.copy_images_from(&source, &profile))
            .and_then(|()| store.save(&profile));
        if let Err(e) = written {
            // The directory is new and holds no profile of the user.
            let _ = std::fs::remove_dir_all(store.dir());
            self.problem(format!("OpenCraft could not add the profile: {e}"));
            return;
        }
        self.change_profile(id, Some(profile));
    }

    fn rename_profile(&mut self, id: String, name: String) {
        if !self.is_profile(&id) {
            return;
        }
        let Some(name) = self.profile_name(name) else {
            return;
        };
        if id == self.profile_id {
            if self.profile.name == name {
                return;
            }
            self.profile.name = name;
            self.changed_at = Some(self.now());
            self.update_entry();
        } else {
            let store = self.profiles.store(&id);
            let renamed = store.read().and_then(|mut profile| {
                if profile.name != name {
                    profile.name = name.clone();
                    store.save(&profile)?;
                }
                Ok(())
            });
            if let Err(e) = renamed {
                self.problem(format!("OpenCraft could not rename the profile: {e}"));
                return;
            }
            let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) else {
                return;
            };
            if entry.name == name {
                return;
            }
            entry.name = name;
        }
        self.send_profiles();
    }

    fn remove_profile(&mut self, id: String) {
        if !self.is_profile(&id) {
            return;
        }
        if self.entries.len() == 1 {
            self.problem("One profile for the device model stays at the minimum.".to_string());
            return;
        }
        let at = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .expect("the profile is in the list");
        let active = id == self.profile_id;
        // The removed directory keeps the last state of the profile.
        if active && !self.save_before_leaving() {
            return;
        }
        if let Err(e) = self.profiles.remove(&id) {
            self.problem(format!("OpenCraft could not remove the profile: {e}"));
            return;
        }
        self.entries.remove(at);
        if active {
            let next = self.entries[at.min(self.entries.len() - 1)].id.clone();
            self.change_profile(next, None);
        } else {
            self.send_profiles();
        }
    }

    /// Send the layout of the model, make the image of each surface, and
    /// send the state of each surface.
    fn show_model(&mut self) {
        (self.sink)(Event::Layout(Arc::new(DeviceLayout::from(self.descriptor))));
        (self.sink)(Event::Kinds(
            self.ports.iter().flat_map(|p| p.kinds.clone()).collect(),
        ));
        self.images.clear();
        for surface in self.descriptor.slot_surfaces() {
            self.render(surface);
        }
    }

    /// Make the image of `surface` from the profile, keep it, and send the
    /// state of the surface.
    fn render(&mut self, surface: &Surface) {
        let content = self.page().content(surface.id);
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
        let feedback = self
            .feedback
            .iter()
            .filter(|(binding, _)| self.feedback_surface(binding.control) == Some(surface.id))
            .max_by_key(|(_, (order, _))| *order)
            .map(|(_, (_, feedback))| feedback);
        let image = Arc::new(render::surface_image_with_feedback(
            size,
            &content,
            source.as_ref(),
            feedback,
        ));
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
            Command::SetBinding { binding, action } => self.set_binding(binding, action),
            Command::SelectPage { page } => self.select_page(page),
            Command::AddPage { copy_of } => self.add_page(copy_of),
            Command::RenamePage { page, name } => self.rename_page(page, name),
            Command::MovePage { page, to } => self.move_page(page, to),
            Command::RemovePage { page } => self.remove_page(page),
            Command::SelectProfile { profile } => self.select_profile(&profile),
            Command::AddProfile { name, copy_of } => self.add_profile(name, copy_of),
            Command::RenameProfile { profile, name } => self.rename_profile(profile, name),
            Command::RemoveProfile { profile } => self.remove_profile(profile),
            Command::SetDeviceSettings(settings) => self.set_device_settings(settings),
            Command::Resolve { kind } => {
                self.request(&kind, Request::Resolve { kind: kind.clone() });
            }
            Command::ReadCounters => (self.sink)(Event::Counters(self.counters)),
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
        let active = self.active;
        if !self
            .profile
            .page_mut(active)
            .is_some_and(|p| p.update(id, change))
        {
            return;
        }
        self.changed_at = Some(self.now());
        self.render(surface);
        if self.link.is_connected() && !self.needs_draw {
            self.draw(&[surface]);
        }
    }

    /// Wait up to `timeout` for the next event of the link, and do the work
    /// for it. Then write the profile and the device settings if a write is
    /// due.
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
                    self.change_model(model);
                }
                log("connected");
                self.draw_all();
                self.announce_connected();
            }
            Ok(LinkEvent::Woke { slept }) => {
                log(&format!(
                    "wake detected: the computer slept {:.1} s",
                    slept.as_secs_f64()
                ));
                if self.link.is_connected() {
                    // The engine does not know what the device kept during
                    // the sleep. It does the same as after a new connection
                    // (rule 10, decision 0023). The shell gets Disconnected
                    // first, so that it drops its input marks: a release
                    // during the sleep is lost.
                    (self.sink)(Event::Disconnected);
                    self.draw_all();
                    self.announce_connected();
                }
            }
            Ok(LinkEvent::Disconnected) => {
                log("disconnected");
                self.needs_draw = false;
                (self.sink)(Event::Disconnected);
            }
            Ok(LinkEvent::Input(report)) => {
                self.counters.reports_received += 1;
                if let Decoded::Input(event) = decode(self.descriptor, &report) {
                    self.counters.input_events += 1;
                    (self.sink)(Event::Input(event));
                    self.invoke(event);
                }
            }
            Ok(LinkEvent::Timeout) => {}
            Err(e) => self.link_error(&e),
        }
        self.save_if_due();
    }

    /// Tell the shell that the device is ready, after the full draw. If the
    /// draw lost the device, the link gives `Disconnected` next, and the
    /// shell gets no `Connected`.
    fn announce_connected(&mut self) {
        if self.link.is_connected() {
            (self.sink)(Event::Connected);
        }
    }

    /// Use the descriptor of a device that connected. If its model
    /// identifier is different, select the profiles and the device settings
    /// of that model, as at the start: see [`Event::Layout`]. The old
    /// profile and the old settings are written first if they changed. The
    /// caller draws all surfaces next.
    fn change_model(&mut self, descriptor: &'static Descriptor) {
        let new_model = descriptor.model != self.descriptor.model;
        if new_model {
            // The device of the new model must show its own profile, so the
            // change continues after a failed write.
            if let Err(e) = self.write_now() {
                self.problem(format!("OpenCraft could not save the profile: {e}"));
            }
            let now = self.now();
            self.save_settings(now);
            self.disappear();
        }
        self.descriptor = descriptor;
        if new_model {
            self.open_profiles();
            self.load_settings();
        }
        self.show_model();
        if new_model {
            self.send_profiles();
            self.send_pages();
            self.appear();
            self.send_settings();
        }
    }

    /// Send the start sequence, the device settings, and the image of each
    /// surface to the device (rule 10). The order is that of decision 0022:
    /// `DIS`, `LIG` with the brightness setting, `CLE 0xFF`, `STP`, then the
    /// knob light commands (`SETLB` and `LBLIG`, or `LBLIG 0` only), then
    /// `QUCMD`, then the images, then `STP`. The device keeps no setting
    /// after a USB restart, and not the knob light colours after `DC`
    /// (measured), so each full draw sends all settings.
    fn draw_all(&mut self) {
        let mut encoded = Vec::with_capacity(self.images.len());
        for surface in self.descriptor.slot_surfaces() {
            if let Some(data) = self.encode(surface) {
                encoded.push(data);
            }
        }
        let brightness = self.settings.brightness;
        let commands = SettingCommands::new(&self.settings, self.descriptor.settings);
        let result = self.link.with_device(|device| {
            device.start(brightness)?;
            for command in commands.after_start() {
                device.send(command.clone())?;
            }
            for (slot, data) in &encoded {
                device.send_image(*slot, data)?;
            }
            device.commit()
        });
        self.needs_draw = match result {
            Ok(()) => {
                log(&format!("full draw done: {} surfaces", encoded.len()));
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

    /// Send the images of `surfaces` to the device, and then one commit.
    /// With no image, send nothing.
    fn draw(&mut self, surfaces: &[&Surface]) {
        let encoded: Vec<_> = surfaces.iter().filter_map(|s| self.encode(s)).collect();
        if encoded.is_empty() {
            return;
        }
        let result = self.link.with_device(|device| {
            for (slot, data) in &encoded {
                device.send_image(*slot, data)?;
            }
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

    /// Write the profile and the device settings, each if it changed and no
    /// change came for [`SAVE_DELAY`].
    fn save_if_due(&mut self) {
        let now = self.now();
        let due = |at: Option<Instant>| {
            at.is_some_and(|at| now.saturating_duration_since(at) >= SAVE_DELAY)
        };
        if due(self.changed_at) {
            self.save(now);
        }
        if due(self.settings_changed_at) {
            self.save_settings(now);
        }
    }

    /// The stop of the engine, in the order of decision 0022:
    ///
    /// 1. Write the profile and the device settings now, each if it has a
    ///    change that is not in the file.
    /// 2. If a device is connected, send the stop sequence (`DC`, then
    ///    `HAN`): the displays become dark, and the knob lights go off.
    /// 3. Close the device.
    ///
    /// After this call, the engine sends nothing to the device: the link does
    /// not send the keep-alive, and it does not connect again. Call this one
    /// time, when the engine stops. [`crate::runtime`] calls it at
    /// [`Command::Stop`] and when each sender of commands is gone.
    pub fn finish(&mut self) {
        let now = self.now();
        if self.changed_at.is_some() {
            self.save(now);
        }
        self.save_settings(now);
        match self.link.stop() {
            Ok(true) => log("stop: sent DC and HAN, the device is dark"),
            Ok(false) => log("stop: no device is connected, sent nothing"),
            Err(e) => log(&format!("stop: the stop sequence failed: {e}")),
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

    /// Tell the shell about a problem, and write it to the log.
    fn problem(&mut self, text: String) {
        log(&format!("problem: {text}"));
        (self.sink)(Event::Problem(text));
    }

    /// Tell the shell about an error of the link. An error that stays gives
    /// one event, not one event for each try.
    ///
    /// The shell gets a text that tells the user what to do. The log gets the
    /// text of the device layer, which is for a developer, as the probe shows
    /// it.
    fn link_error(&mut self, error: &DeviceError) {
        let detail = error.to_string();
        if self.link_problem.as_deref() != Some(&detail) {
            log(&format!("device error: {}", detail.replace('\n', " ")));
            self.link_problem = Some(detail);
            self.problem(device_problem(error).to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_problem_tells_the_user_what_to_do_with_no_internal_names() {
        let errors = [
            DeviceError::PermissionDenied,
            DeviceError::NoVendorInterface,
            DeviceError::DeviceNotFound,
            DeviceError::Disconnected,
            DeviceError::Transport("write failed".into()),
            DeviceError::ShortReport(3),
            DeviceError::NoSuchSlot(99),
        ];
        for error in &errors {
            let text = device_problem(error);
            for internal in ["opencraft-probe", "kIOReturn", "vendor interface", "HID"] {
                assert!(!text.contains(internal), "{text}");
            }
            assert!(text.contains("connect"), "{text}");
        }
        assert!(device_problem(&DeviceError::PermissionDenied).contains("Quit VSD Craft"));
    }
}
