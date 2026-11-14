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
//! The engine holds the location in memory: the active page and the open
//! folders. [`crate::location::Location`] answers which page or folder holds
//! a control. The device and the shell show that location, and the commands
//! for a surface or a binding change it. At the start, the active page is
//! the first page and no folder is open. The steps of a page change are in
//! [`Engine::select_page`]. The reasons are in
//! `docs/decisions/0016-pages-and-engine-requests-from-a-handler.md` and
//! `docs/decisions/0036-folders.md`.
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
//! The engine keeps the undo and redo history of the active profile. Each
//! command that changes the pages is one step: see `step_of` and
//! [`Engine::undo`]. The reasons are in
//! `docs/decisions/0030-undo-and-redo-in-the-engine.md`.
//!
//! The engine keeps the clipboard of the window: the content of one
//! control. See [`Engine::paste_control`] and [`crate::clipboard`].
//!
//! The logic has no thread of its own. A caller calls [`Engine::command`]
//! and [`Engine::poll`] in a loop: see [`crate::runtime`]. A test calls them
//! directly, with a fake transport and a fake clock (rule 7). The reasons
//! are in `docs/decisions/0011-engine-thread-model.md`.

use crate::action::{
    Action, BindingId, BindingState, EngineRequest, Feedback, FeedbackState, GestureKind,
    HandlerPort, KindDescription, Message, ProfileTarget, Request, Status,
};
use crate::clipboard::{ControlContent, Parts};
mod front;
mod power;

use crate::device_settings::{self, DeviceSettings, SettingCommands};
use crate::front::{ActiveCause, FrontApplication};
use crate::history::{History, SavedProfile, Target};
use crate::icons;
use crate::layout::DeviceLayout;
use crate::location::Location;
use crate::power::{DisplayState, Power, PowerNotice};
use crate::profile::{
    image_uses_of, ApplicationLink, Background, BackgroundFit, Color, Folder, FolderEntry,
    FolderId, LabelChange, Page, Profile, SurfaceContent, SurfaceIcon, DEFAULT_PROFILE_NAME,
};
use crate::profiles::{ProfileEntry, ProfilesDir};
use crate::render::{self, Size};
use crate::store::ProfileStore;
use image::RgbImage;
use opencraft_core::image::{encode_for, jpeg_quality_for};
use opencraft_core::{
    decode, Clock, Connector, ControlId, ControlKind, Decoded, Descriptor, Error as DeviceError,
    InputEvent, Link, LinkEvent, Surface, SurfaceId, SurfaceKind, SystemClock,
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
/// The environment variable that turns on the log lines of a measurement of
/// the automatic switch (`docs/kb/footprint.md`). The shell reads the same
/// variable for its line at the notice.
pub const SWITCH_TRACE: &str = "OPENCRAFT_SWITCH_TRACE";

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
    /// Change one part of the label style of a surface: the position, the
    /// size, the colour, or the weight. The engine puts a size into its
    /// range. The label text does not change.
    SetLabelStyle {
        surface: SurfaceId,
        change: LabelChange,
    },
    /// Set the colour of a surface. `None` removes it.
    SetColor {
        surface: SurfaceId,
        color: Option<Color>,
    },
    /// Set the image of a surface from an image file of the user. The engine
    /// makes a copy of the image in the profile, and removes the icon of the
    /// surface. `None` removes the image.
    SetImage {
        surface: SurfaceId,
        path: Option<PathBuf>,
    },
    /// Set the icon of a surface: a name of the built-in icon set and a
    /// colour (decision 0033). The engine removes the image of the surface.
    /// `None` removes the icon and the image ("No icon" in the window). A
    /// name that the set does not have gives a problem and changes nothing.
    SetIcon {
        surface: SurfaceId,
        icon: Option<SurfaceIcon>,
    },
    /// Set or remove a binding. The page of `binding` must be the active
    /// page. If it is not, the engine changes nothing and gives a problem: the
    /// shell made the command for a page that the device does not show now.
    SetBinding {
        binding: BindingId,
        action: Option<Action>,
    },
    /// Set the background of `page`, or remove it with `None`. For an image,
    /// the engine makes a copy of the image file in the profile: see
    /// [`ProfileStore::import_background`]. A model with no background
    /// gives a problem. The reasons are in
    /// `docs/decisions/0032-page-background.md`.
    SetBackground {
        page: usize,
        background: Option<NewBackground>,
    },
    /// Change the fit of the background image of `page`. A page with no
    /// background image does not change.
    SetBackgroundFit { page: usize, fit: BackgroundFit },
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
    /// the last page, the page before it becomes the active page. If `page`
    /// is the page under the folders, all folders close (decision 0036).
    RemovePage { page: usize },
    /// Add a folder at the end of the list, and open it. The shell makes
    /// `id` with [`FolderId::random`]. With `name` equal to `None`, the name
    /// is "Folder *N*". With `copy_of`, the new folder has the key faces and
    /// the bindings of that folder. An `id` that exists, or a `copy_of` that
    /// does not exist, gives a problem and changes nothing (decision 0036).
    AddFolder {
        id: FolderId,
        name: Option<String>,
        copy_of: Option<FolderId>,
    },
    /// Give the folder `folder` the name `name`. Only spaces give a problem.
    RenameFolder { folder: FolderId, name: String },
    /// Move the folder `folder` to the position `to` in the list of folders.
    MoveFolder { folder: FolderId, to: usize },
    /// Remove the folder `folder`. If it is open, the engine closes it and
    /// the folders after it.
    RemoveFolder { folder: FolderId },
    /// Open the folder `folder`. See decision 0036, section 3. This is not a
    /// step of the history.
    OpenFolder { folder: FolderId },
    /// Close the last open folder. With no open folder, nothing changes.
    /// This is not a step of the history.
    CloseFolder,
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
    /// Give the profile `profile` the colour `color` for the user. `None`
    /// removes the colour. The undo history does not cover it, as for the
    /// name of a profile (decision 0030).
    SetProfileColor {
        profile: String,
        color: Option<Color>,
    },
    /// Make `page` the first page of the profile `profile`: the page that
    /// the device shows when the profile becomes the active profile. For
    /// the active profile, this is a step of the history, because the mark
    /// is on the page.
    SetFirstPage { profile: String, page: usize },
    /// Write the profile `profile` and its images to the file `path`, in the
    /// format of [`crate::share`]. The engine sends
    /// [`Event::ProfileExported`], or a problem.
    ExportProfile { profile: String, path: PathBuf },
    /// Read a profile from the file `path` in the format of
    /// [`crate::share`], and add it as a new profile. If a profile of the
    /// model has its name, the new profile gets a number after the name.
    /// The new profile becomes the active profile, as after
    /// [`Command::AddProfile`], and the engine then sends
    /// [`Event::ProfileImported`]. A file with an error, or with a profile
    /// for a different model, gives a problem and changes nothing.
    ImportProfile { path: PathBuf },
    /// Add `profile`, a starter profile of the first start (decision 0029,
    /// section 18), and make it the active profile, as after
    /// [`Command::AddProfile`]. If a profile of the model has its name, the
    /// new profile gets a number after the name.
    ///
    /// If the active profile is not in a file and has no change, for
    /// example the profile that the engine made because the model had no
    /// profile, the new profile takes its place: the engine does not write
    /// the old profile. A profile for a different model, or a profile with
    /// an image file, gives a problem and changes nothing: a starter
    /// profile has no image files.
    AddStarterProfile { profile: Profile },
    /// Undo the last step of the history: put the pages back as they were
    /// before it, and make the page that was active before it the active
    /// page. See [`Engine::undo`]. With an empty undo list, nothing occurs.
    Undo,
    /// Do the last undone step again. With an empty redo list, nothing
    /// occurs.
    Redo,
    /// A text field of the shell got the focus. Until
    /// [`Command::EndTextEdit`], the changes of one target, for example the
    /// label of one surface, are one step of the history.
    StartTextEdit,
    /// The text field lost the focus, or the user pressed Return.
    EndTextEdit,
    /// Obey `commands` in their order as one step of the history, with the
    /// name `name`, for example "Drop Action". If no command changes the
    /// pages, the history does not change.
    Group {
        name: String,
        commands: Vec<Command>,
    },
    /// Copy the content of `control` on the active page to the clipboard of
    /// the engine: its appearance and its bindings (see
    /// [`crate::clipboard`]). The engine sends [`Event::Clipboard`]. The
    /// pages do not change.
    CopyControl { control: ControlId },
    /// Put the content of the clipboard on `control` on the active page.
    /// The kind of `control` must be the kind of the copy. Else the engine
    /// changes nothing and gives a problem.
    PasteControl { control: ControlId },
    /// Remove the appearance and the bindings of `control` on the active
    /// page.
    ClearControl { control: ControlId },
    /// Put the content of `control` on the active page on the same control
    /// on each other page of the profile. This is one step of the history.
    CopyToAllPages { control: ControlId },
    /// Set the device settings of the model of the device. The command has
    /// all settings. The engine puts each value into its range, and then
    /// does the steps of [`Engine::set_device_settings`].
    SetDeviceSettings(DeviceSettings),
    /// A system power notice observed by the shell.
    PowerNotice(PowerNotice),
    /// The front application changed, or the shell reports the front
    /// application at the start (decision 0035, section 8).
    FrontApplication(FrontApplication),
    /// Add `link` to the profile with the identity `profile`. A link that
    /// another profile of the model has moves to this profile.
    LinkApplication {
        profile: String,
        link: ApplicationLink,
    },
    /// Remove the link with this bundle identifier from the profile.
    UnlinkApplication { profile: String, bundle_id: String },
    /// Make the profile the default profile of the model (decision 0035,
    /// section 6).
    SetDefaultProfile { profile: String },
    /// Turn the automatic switch on or off (decision 0035, section 7).
    /// The active profile does not change.
    SetFollowFrontApplication(bool),
    /// Wake from the computer side, unless the system is locked or asleep.
    WakeDevice,
    /// Ask the handler to correct the status of this kind.
    Resolve { kind: String },
    /// Ask the handler for the current choices of a field. The opaque id
    /// lets a shell reject a late reply after the form changes.
    Choices {
        id: u64,
        action: Action,
        field: String,
    },
    /// Run the action of `binding` as if the gesture occurred, with no push
    /// on the device: the engine sends [`Request::Invoke`] to the handler of
    /// the kind. The result comes from the handler, as for a gesture of the
    /// device: for example the problem of a missing permission. The binding
    /// must be on the active page. If it is not, or if it has no action, or
    /// if no handler has the kind, the engine gives a problem.
    ///
    /// The command does not change the profile and is not a step of the
    /// history. It is not input: the counters do not change.
    TestAction { binding: BindingId },
    /// Send the current counters in an event.
    ReadCounters,
    /// Obey `command` only if `profile` is the identity of the active
    /// profile. Else give a problem and change nothing. A shell sends a
    /// command of a page row in this form, with the profile that its
    /// window showed when the user clicked (issue #321): a window that
    /// shows an old picture must not change a different profile.
    InProfile {
        profile: String,
        command: Box<Command>,
    },
    /// Stop the engine: write the profile and the device settings if they
    /// changed, make the device dark with the stop sequence, and close the
    /// device. See [`Engine::finish`].
    Stop,
}

/// The name of the step of the history that `command` makes, and the
/// target that a later change in the same text edit can join (decision
/// 0030). `location` is the page under the folders and the open folders.
/// `None`: the command changes no page, and makes no step.
///
/// Each command has one line here. A new command that changes the pages
/// gets its name here, and then the history covers it with no more work.
/// The match has no wildcard, thus the compiler asks for the line.
fn step_of(command: &Command, location: &Location) -> Option<(String, Option<Target>)> {
    let step = |name: &str, target| Some((name.to_string(), target));
    match command {
        Command::SetLabel { surface, .. } => step(
            "Set Title",
            Some(Target::Label {
                page: location.page,
                folder: location.folder_of_surface(*surface),
                surface: *surface,
            }),
        ),
        Command::SetLabelStyle { change, .. } => step(
            match change {
                LabelChange::Position(_) => "Set Title Position",
                LabelChange::Size(_) => "Set Title Size",
                LabelChange::Color(_) => "Set Title Colour",
                LabelChange::Bold(_) => "Set Title Weight",
            },
            None,
        ),
        Command::SetColor { .. } => step("Set Fill", None),
        Command::SetBackground {
            background: Some(_),
            ..
        } => step("Set Background", None),
        Command::SetBackground {
            background: None, ..
        } => step("Remove the Background", None),
        Command::SetBackgroundFit { .. } => step("Set Background Fit", None),
        Command::SetImage { path: Some(_), .. } => step("Set Image", None),
        Command::SetImage { path: None, .. } => step("Remove the Image", None),
        Command::SetIcon { icon: Some(_), .. } => step("Set Icon", None),
        Command::SetIcon { icon: None, .. } => step("Remove the Icon", None),
        Command::SetBinding {
            binding,
            action: Some(_),
        } => step("Set Action", Some(Target::Binding(*binding))),
        Command::SetBinding { action: None, .. } => step("Remove the Action", None),
        Command::AddPage { copy_of: None } => step("Add Page", None),
        Command::AddPage { copy_of: Some(_) } => step("Duplicate Page", None),
        Command::RenamePage { page, .. } => {
            step("Rename Page", Some(Target::PageName { page: *page }))
        }
        Command::MovePage { .. } => step("Move Page", None),
        Command::RemovePage { .. } => step("Remove Page", None),
        Command::AddFolder { copy_of: None, .. } => step("Add Folder", None),
        Command::AddFolder { copy_of: Some(_), .. } => step("Duplicate Folder", None),
        Command::RenameFolder { folder, .. } => {
            step("Rename Folder", Some(Target::FolderName { folder: *folder }))
        }
        Command::MoveFolder { .. } => step("Move Folder", None),
        Command::RemoveFolder { .. } => step("Remove Folder", None),
        // The mark of the first page is on a page. For a profile that is not
        // active, the pages do not change, and no step occurs.
        Command::SetFirstPage { .. } => step("Set First Page", None),
        Command::Group { name, commands } => step(name, group_target(commands, location)),
        Command::InProfile { command, .. } => step_of(command, location),
        Command::PasteControl { .. } => step("Paste", None),
        Command::ClearControl { .. } => step("Clear", None),
        Command::CopyToAllPages { .. } => step("Copy to All Pages", None),
        // These commands change no page. A change of the active profile
        // empties the history. A test of an action runs a binding and
        // changes nothing in the profile (#188).
        Command::SelectPage { .. }
        | Command::OpenFolder { .. }
        | Command::CloseFolder
        | Command::SelectProfile { .. }
        | Command::AddProfile { .. }
        | Command::RenameProfile { .. }
        | Command::RemoveProfile { .. }
        // The colour is a part of the profile that is not in the pages, as
        // the name. An export changes nothing. An import adds a profile and
        // makes it the active profile.
        | Command::SetProfileColor { .. }
        | Command::ExportProfile { .. }
        | Command::ImportProfile { .. }
        | Command::AddStarterProfile { .. }
        | Command::SetDeviceSettings(_)
        | Command::PowerNotice(_)
        | Command::FrontApplication(_)
        | Command::LinkApplication { .. }
        | Command::UnlinkApplication { .. }
        | Command::SetDefaultProfile { .. }
        | Command::SetFollowFrontApplication(_)
        | Command::WakeDevice
        | Command::CopyControl { .. }
        | Command::Undo
        | Command::Redo
        | Command::StartTextEdit
        | Command::EndTextEdit
        | Command::Resolve { .. }
        | Command::Choices { .. }
        | Command::TestAction { .. }
        | Command::ReadCounters
        | Command::Stop => None,
    }
}

/// The target of a group in a text edit: the target of its first command,
/// if each command of the group has a target. Thus one edit of a text field
/// that writes two bindings, for example a setting of the "Turn" card of a
/// knob, is one step (decision 0030).
fn group_target(commands: &[Command], location: &Location) -> Option<Target> {
    let targets = commands
        .iter()
        .map(|c| step_of(c, location).and_then(|(_, target)| target))
        .collect::<Option<Vec<_>>>()?;
    targets.first().copied()
}

/// `name` if no profile in `entries` has it, else `name 2`, `name 3`, and so
/// on, with the lowest free number.
fn free_profile_name(name: &str, entries: &[ProfileEntry]) -> String {
    let free = |candidate: &str| !entries.iter().any(|e| e.name == candidate);
    if free(name) {
        return name.to_string();
    }
    (2..)
        .map(|n| format!("{name} {n}"))
        .find(|candidate| free(candidate))
        .expect("a free name exists")
}

/// The state of one surface: its content in the profile, and the image that
/// the engine made from the content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceState {
    pub id: SurfaceId,
    pub content: SurfaceContent,
    /// The upright image. The device shows this same image.
    pub image: Arc<RgbImage>,
    /// The image of each state, when the feedback on the surface has a
    /// state (decision 0017). One of the two is `image`. The shell shows
    /// them in the appearance of each state (decision 0029, section 10.5).
    pub states: Option<StateImages>,
}

/// A new background for a page, in [`Command::SetBackground`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewBackground {
    /// One colour on all of the display.
    Color(Color),
    /// An image file of the user, with its fit.
    Image { path: PathBuf, fit: BackgroundFit },
}

/// The background of the active page: its setting in the profile, and the
/// image that the engine made from it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackgroundState {
    /// `None`: the page has no background.
    pub background: Option<Background>,
    /// The upright image of the full display, with the size of the
    /// background of the model. The device shows this same image (rule 11).
    /// `None` when the page has no background, or when the model has no
    /// background.
    pub image: Option<Arc<RgbImage>>,
}

/// The images of a surface with a feedback of two states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateImages {
    /// The state that the surface shows now.
    pub now: FeedbackState,
    pub off: Arc<RgbImage>,
    pub on: Arc<RgbImage>,
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

/// A write of the background to the device.
enum BackgroundWrite {
    /// The background image, encoded for the background surface.
    Image(Vec<u8>),
    /// The page has no background: remove the background behind the keys
    /// and behind the touch bar.
    Clear,
}

impl BackgroundWrite {
    fn send<T: opencraft_core::Transport>(
        &self,
        device: &mut opencraft_core::Device<T>,
    ) -> opencraft_core::Result<()> {
        match self {
            BackgroundWrite::Image(data) => device.send_background(data),
            BackgroundWrite::Clear => device.clear_background(),
        }
    }
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
    /// The reply to a Choices command. This is form data, not profile data.
    Choices {
        id: u64,
        options: Vec<crate::ChoiceOption>,
    },
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
        /// The identity of the default profile of the model (decision 0035,
        /// section 6).
        default: String,
        /// Why `active` is the active profile.
        cause: ActiveCause,
    },
    /// The setting "Change the profile with the front application". The
    /// engine sends it after the first [`Event::Profiles`] at the start,
    /// and after each change of the setting.
    FollowFrontApplication(bool),
    /// The names of the pages in their order, the number of the active
    /// page, and the number of the first page of the profile (see
    /// [`Profile::first_page`]). Page numbers start at 0. `folders` is the
    /// folders of the profile, and `open` is the open folders (decision
    /// 0036).
    ///
    /// The engine sends this event at the start and after each change of
    /// the pages, the folders, or the location. When a shell gets it, the
    /// shell removes all binding states that it holds. Then the engine sends
    /// [`Event::Binding`] for each binding of the location: the keys of the
    /// last open folder, and the other controls of the active page. Thus the
    /// shell holds the bindings of the location only.
    ///
    /// If the active page changed, the engine sends [`Event::Surface`] for
    /// each surface before this event. A change of the open folders sends
    /// the key faces only. After a change of the active profile,
    /// [`Event::Profiles`] comes between the surfaces and this event.
    Pages {
        names: Vec<String>,
        active: usize,
        first: usize,
        folders: Vec<FolderEntry>,
        open: Vec<FolderId>,
    },
    /// A binding of the active page: after each [`Event::Pages`], and after
    /// a change of the binding. An unavailable kind stays. A state with no
    /// action tells that the binding is gone.
    Binding(BindingState),
    /// The names of the steps of the history: the step that an undo
    /// undoes, and the step that a redo does again, for example "Set Title".
    /// `None` is an empty list. The engine sends this event after each
    /// change of the history. At the start, and after a change of the
    /// active profile, which empties the history, it comes after the
    /// bindings. See `docs/decisions/0030-undo-and-redo-in-the-engine.md`.
    History {
        undo: Option<String>,
        redo: Option<String>,
    },
    /// The clipboard of the engine has the content of a control of this
    /// kind, after [`Command::CopyControl`]. A paste is possible only on a
    /// control of this kind. Before the first copy, the clipboard is empty
    /// and the engine sends no event.
    Clipboard(ControlKind),
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
    /// The background of the active page, when it changes: a change of the
    /// background, of the active page, of the active profile, or of the
    /// model. Before the first event, the active page has no background,
    /// thus a page with no background at the start sends no event. It
    /// comes before the surfaces, because a surface with no fill colour
    /// shows a part of it.
    Background(BackgroundState),
    /// A device is connected, and the engine sent the image of each surface
    /// to it. Before this event, no device is connected.
    ///
    /// After a sleep of the computer, the engine sends [`Event::Disconnected`]
    /// and then this event again, with a full draw between them. The device
    /// possibly lost its images during the sleep, and an input event during
    /// the sleep, for example a release, is lost.
    Connected,
    /// The firmware of the connected device, or `None` if its read failed.
    /// Sent before `Connected`. This contains no device serial number.
    Firmware(Option<String>),
    /// The device is gone, or the computer slept. The engine looks for the
    /// device again, or draws it again. See [`Event::Connected`].
    Disconnected,
    /// The user did a gesture on a control.
    Input(InputEvent),
    /// The engine wrote the profile `profile` to the file `path`: the
    /// response to [`Command::ExportProfile`].
    ProfileExported { profile: String, path: PathBuf },
    /// The engine added the profile `profile` from a file: the response to
    /// [`Command::ImportProfile`]. It comes after the events of the change
    /// of the active profile.
    ProfileImported { profile: String },
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
    /// The active page and the open folders. Location answers which page
    /// or folder holds a control (decision 0036).
    location: Location,
    /// False if the profile file could not be read. Then the engine does not
    /// write the file, because the write would replace it.
    can_save: bool,
    /// The model of the device: of the connected device, or of the last one,
    /// or the default model.
    descriptor: &'static Descriptor,
    /// The image of each surface that has a slot (rule 10).
    images: BTreeMap<SurfaceId, Arc<RgbImage>>,
    /// The background image of the active page, if the page has a
    /// background and the model has one (rule 10).
    background: Option<Arc<RgbImage>>,
    /// The last background state that the shell got. Before the first
    /// event, the shell knows no background.
    background_sent: BackgroundState,
    /// True if the connected device does not show all images.
    needs_draw: bool,
    /// The number of images in the last successful `draw`. The log of a
    /// front-application switch reads it. A draw that writes nothing sets 0.
    last_written: usize,
    /// True if the environment variable [`SWITCH_TRACE`] is set. Then the
    /// engine logs the time of each automatic switch. Else it writes no
    /// line for a switch, so that the log does not grow in a long session.
    switch_trace: bool,
    /// The time of the last change of the profile that is not in the file.
    changed_at: Option<Instant>,
    /// True after a failed write of the profile, until a write is successful.
    save_failed: bool,
    /// The text of the last problem of the link, to tell the shell one time
    /// only about a problem that stays.
    link_problem: Option<String>,
    /// The device settings of the model of the device.
    settings: DeviceSettings,
    power: Power,
    /// True if the last front application was this process (decision 0035).
    shell_in_front: bool,
    /// The profile that the switch rule gave for the last front application
    /// that is not OpenCraft. Empty at the start, and after the setting
    /// goes on.
    last_target: Option<String>,
    /// True after a manual selection while OpenCraft is in front, until the
    /// next front application that is not OpenCraft.
    chosen_in_window: bool,
    /// Why the active profile is active.
    cause: ActiveCause,
    /// The setting of section 7. A missing key in `state.toml` is true.
    follow_front: bool,
    /// The default mark of the model of the device, if `state.toml` has one.
    default_mark: Option<String>,
    /// False if the settings file has an error or a newer schema version.
    /// Then the engine does not write the file until the user changes a
    /// setting.
    settings_can_save: bool,
    /// The time of the last change of the settings that is not in the file.
    settings_changed_at: Option<Instant>,
    /// True after a failed write of the settings, until a write is
    /// successful.
    settings_save_failed: bool,
    /// The undo and redo history of the active profile (decision 0030).
    history: History,
    /// The content of the last copied control, and the directory of the
    /// profile of the copy: its image paths are relative to it.
    clipboard: Option<(ControlContent, PathBuf)>,
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
            location: Location::default(),
            can_save: true,
            descriptor: default_model,
            images: BTreeMap::new(),
            background: None,
            background_sent: BackgroundState::default(),
            needs_draw: false,
            last_written: 0,
            switch_trace: std::env::var_os(SWITCH_TRACE).is_some(),
            changed_at: None,
            save_failed: false,
            link_problem: None,
            settings: DeviceSettings::defaults(default_model.settings),
            power: Power::default(),
            shell_in_front: false,
            last_target: None,
            chosen_in_window: false,
            cause: ActiveCause::Selected,
            follow_front: true,
            default_mark: None,
            settings_can_save: true,
            settings_changed_at: None,
            settings_save_failed: false,
            history: History::default(),
            clipboard: None,
            ports,
            feedback: BTreeMap::new(),
            feedback_order: 0,
            counters: Counters::default(),
            sink,
        };
        engine.open_profiles();
        engine.load_settings();
        engine.restart_idle();
        engine.show_model();
        engine.send_profiles();
        engine.send_follow();
        engine.send_pages();
        engine.send_history();
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
        let timer_changed = self.settings.screen_off_minutes != settings.screen_off_minutes;
        self.settings = settings;
        if timer_changed {
            self.restart_idle();
        }
        self.settings_can_save = true;
        self.settings_changed_at = Some(self.now());
        self.send_settings();
        let changes: Vec<Vec<u8>> = before.changes(&after).into_iter().cloned().collect();
        if changes.is_empty()
            || !self.link.is_connected()
            || self.needs_draw
            || self.power.state != DisplayState::Awake
        {
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

    /// The number of the active page. Page numbers start at 0. While folders
    /// are open, this is the page under the folders.
    pub fn active_page(&self) -> usize {
        self.location.page
    }

    /// The open folders, from the first to the last.
    pub fn open_folders(&self) -> &[FolderId] {
        &self.location.open
    }

    /// The link to the device.
    pub fn link(&self) -> &Link<C, K> {
        &self.link
    }

    /// The input counts since this engine starts.
    pub fn counters(&self) -> Counters {
        self.counters
    }

    /// The page under the folders. The knobs, the bar panels, the touch bar,
    /// and the background come from this page.
    fn page(&self) -> &Page {
        &self.profile.pages()[self.location.page]
    }

    /// The key bindings of the location. A change of the open folders
    /// sends Disappear and Appear for these bindings only (decision 0036).
    fn key_bindings(&self) -> Vec<(BindingId, Action)> {
        self.location
            .bindings(&self.profile)
            .into_iter()
            .filter(|(binding, _)| binding.control.kind == ControlKind::Key)
            .collect()
    }

    fn disappear_bindings(&self, bindings: &[(BindingId, Action)]) {
        for (binding, action) in bindings {
            self.request(&action.kind, Request::Disappear { binding: *binding });
        }
    }

    fn appear_bindings(&self, bindings: &[(BindingId, Action)]) {
        for (binding, action) in bindings {
            let kind = action.kind.clone();
            self.request(
                &kind,
                Request::Appear {
                    binding: *binding,
                    action: action.clone(),
                },
            );
        }
    }

    /// Send the list of pages, then the state of each binding of the
    /// location. See [`Event::Pages`].
    fn send_pages(&mut self) {
        let names = self
            .profile
            .pages()
            .iter()
            .map(|p| p.name.clone())
            .collect();
        let folders = self
            .profile
            .folders()
            .iter()
            .map(|folder| FolderEntry {
                id: folder.id,
                name: folder.page.name.clone(),
            })
            .collect();
        let open = self.location.open.clone();
        // The entry of the active profile follows the pages.
        self.update_entry();
        (self.sink)(Event::Pages {
            names,
            active: self.location.page,
            first: self.profile.first_page(),
            folders,
            open,
        });
        for (binding, action) in self.location.bindings(&self.profile) {
            self.binding_event(binding, Some(action));
        }
    }

    /// Send Appear for each binding of the location.
    fn appear(&self) {
        self.appear_bindings(&self.location.bindings(&self.profile));
    }

    /// Send Disappear for each binding of the location, and remove all
    /// feedback. This is step 1 of a page change.
    fn disappear(&mut self) {
        self.disappear_bindings(&self.location.bindings(&self.profile));
        self.feedback.clear();
    }

    /// Make `page` the active page with the steps of a page change: see
    /// [`Self::select_page`]. `change` changes the pages, for example it adds
    /// a page, after step 1. Thus a handler never holds a binding with an old
    /// page number. With `close_folders`, the open folders close.
    fn change_page<R>(
        &mut self,
        change: impl FnOnce(&mut Profile) -> R,
        page: usize,
        close_folders: bool,
    ) -> R {
        self.disappear();
        let result = change(&mut self.profile);
        let count = self.profile.pages().len();
        self.location.page = page.min(count.saturating_sub(1));
        if close_folders {
            self.location.open.clear();
        } else {
            let open = std::mem::take(&mut self.location.open);
            self.location.open = Location::restored_open(&self.profile, &open);
        }
        self.redraw();
        self.send_pages();
        self.appear();
        result
    }

    /// Change the open folders. Only the keys change: the knobs, the bar
    /// panels, the touch bar, and the background stay (decision 0036).
    fn set_open(&mut self, open: Vec<FolderId>) {
        let open = Location::restored_open(&self.profile, &open);
        if open == self.location.open {
            return;
        }
        let old = self.key_bindings();
        self.disappear_bindings(&old);
        for (binding, _) in &old {
            self.feedback.remove(binding);
        }
        self.location.open = open;
        self.redraw_keys();
        self.send_pages();
        let now = self.key_bindings();
        self.appear_bindings(&now);
    }

    /// Make the image of each surface for the active page, and write to the
    /// device only the images that are different from the images before.
    /// These are steps 2 and 3 of a page change.
    fn redraw(&mut self) {
        let before = std::mem::take(&mut self.images);
        let background_before = self.background.take();
        self.render_background();
        let surfaces: Vec<&'static Surface> = self.descriptor.slot_surfaces().collect();
        for surface in &surfaces {
            self.render(surface);
        }
        let changed: Vec<&Surface> = surfaces
            .into_iter()
            .filter(|s| before.get(&s.id) != self.images.get(&s.id))
            .collect();
        let background = background_before != self.background;
        if self.link.is_connected() && !self.needs_draw {
            self.draw(background, &changed);
        }
    }

    /// Make the image of each key face for the location, and write only the
    /// images that changed. The knobs, the bar panels, and the background
    /// stay.
    fn redraw_keys(&mut self) {
        let surfaces: Vec<&'static Surface> = self
            .descriptor
            .slot_surfaces()
            .filter(|surface| surface.id.kind == SurfaceKind::KeyFace)
            .collect();
        let before: Vec<(SurfaceId, Arc<RgbImage>)> = surfaces
            .iter()
            .filter_map(|surface| {
                self.images
                    .get(&surface.id)
                    .cloned()
                    .map(|image| (surface.id, image))
            })
            .collect();
        for surface in &surfaces {
            self.render(surface);
        }
        let changed: Vec<&Surface> = surfaces
            .into_iter()
            .filter(|surface| {
                let previous = before
                    .iter()
                    .find(|(id, _)| *id == surface.id)
                    .map(|(_, image)| image);
                previous != self.images.get(&surface.id)
            })
            .collect();
        if self.link.is_connected() && !self.needs_draw {
            self.draw(false, &changed);
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
    ///
    /// A selection also closes every open folder. If `page` is the page
    /// under the folders, only the folders close, and only the keys change
    /// (decision 0036).
    pub fn select_page(&mut self, page: usize) {
        if !self.is_page(page) {
            return;
        }
        if page == self.location.page {
            if !self.location.open.is_empty() {
                self.set_open(Vec::new());
            }
            return;
        }
        self.change_page(|_| {}, page, true);
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
        self.change_page(|profile| profile.add_page(page), number, true);
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
        let active = self.location.page;
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
        self.change_pages(|profile| profile.move_page(page, to), after, false);
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
        let after = match page.cmp(&self.location.page) {
            std::cmp::Ordering::Less => self.location.page - 1,
            std::cmp::Ordering::Equal => page.min(count - 2),
            std::cmp::Ordering::Greater => self.location.page,
        };
        self.changed_at = Some(self.now());
        if page == self.location.page {
            // The same number can be a different page now. The folders close.
            self.change_page(|profile| profile.remove_page(page), after, true);
        } else {
            self.change_pages(|profile| profile.remove_page(page), after, false);
        }
    }

    /// Change the list of pages. If the number of the active page changes,
    /// do the steps of a page change, so that no handler holds an old page
    /// number. Else send only the new list. With `close_folders`, the open
    /// folders close.
    fn change_pages<R>(
        &mut self,
        change: impl FnOnce(&mut Profile) -> R,
        active: usize,
        close_folders: bool,
    ) {
        if active == self.location.page && !(close_folders && !self.location.open.is_empty()) {
            change(&mut self.profile);
            self.send_pages();
        } else if active == self.location.page {
            change(&mut self.profile);
            self.set_open(Vec::new());
        } else {
            self.change_page(change, active, close_folders);
        }
    }

    fn open_folder(&mut self, id: FolderId) {
        if self.profile.folder(id).is_none() {
            self.problem(
                "The folder of this key does not exist. Select a folder for the key.".to_string(),
            );
            return;
        }
        if self.location.open.last() == Some(&id) {
            return;
        }
        let mut open = self.location.open.clone();
        if let Some(pos) = open.iter().position(|folder| *folder == id) {
            open.truncate(pos + 1);
        } else {
            open.push(id);
        }
        self.set_open(open);
    }

    fn close_folder(&mut self) {
        if self.location.open.is_empty() {
            return;
        }
        let mut open = self.location.open.clone();
        open.pop();
        self.set_open(open);
    }

    fn add_folder(&mut self, id: FolderId, name: Option<String>, copy_of: Option<FolderId>) {
        if self.profile.folder(id).is_some() {
            self.problem("The profile already has this folder.".to_string());
            return;
        }
        let mut page = if let Some(source) = copy_of {
            match self.profile.folder(source) {
                Some(folder) => folder.page.clone(),
                None => {
                    self.problem(
                        "OpenCraft cannot duplicate a folder that does not exist.".to_string(),
                    );
                    return;
                }
            }
        } else {
            Page::new(String::new())
        };
        match name {
            Some(name) if name.trim().is_empty() => {
                self.problem("A folder must have a name.".to_string());
                return;
            }
            Some(name) => page.name = name,
            None => page.name = self.profile.new_folder_name(),
        }
        if !self.profile.add_folder(Folder { id, page }) {
            self.problem("The profile already has this folder.".to_string());
            return;
        }
        self.changed_at = Some(self.now());
        self.open_folder(id);
    }

    fn rename_folder(&mut self, id: FolderId, name: String) {
        if name.trim().is_empty() {
            self.problem("A folder must have a name.".to_string());
            return;
        }
        let Some(folder) = self.profile.folder_mut(id) else {
            self.problem("The profile has no folder with this identity.".to_string());
            return;
        };
        if folder.page.name == name {
            return;
        }
        folder.page.name = name;
        self.changed_at = Some(self.now());
        self.send_pages();
    }

    fn move_folder(&mut self, id: FolderId, to: usize) {
        let Some(from) = self
            .profile
            .folders()
            .iter()
            .position(|folder| folder.id == id)
        else {
            self.problem("The profile has no folder with this identity.".to_string());
            return;
        };
        if to >= self.profile.folders().len() {
            self.problem("OpenCraft could not move the folder.".to_string());
            return;
        }
        if from == to {
            return;
        }
        self.profile.move_folder(id, to);
        self.changed_at = Some(self.now());
        self.send_pages();
    }

    fn remove_folder(&mut self, id: FolderId) {
        if self.profile.folder(id).is_none() {
            self.problem("The profile has no folder with this identity.".to_string());
            return;
        }
        let open = match self.location.open.iter().position(|folder| *folder == id) {
            Some(pos) => self.location.open[..pos].to_vec(),
            None => self.location.open.clone(),
        };
        let location_changes = open != self.location.open;
        self.profile.remove_folder(id);
        self.changed_at = Some(self.now());
        if location_changes {
            self.set_open(open);
        } else {
            self.send_pages();
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
        if !self.location.shows(binding) {
            self.problem(
                "OpenCraft did not change the binding, because its page is not the page \
                 that the device shows."
                    .to_string(),
            );
            return;
        }
        let Some(page) = self
            .location
            .page_for_change(&mut self.profile, binding.control)
        else {
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
        let Some((binding, action)) = self
            .location
            .binding(&self.profile, event.control, gesture)
            .map(|(binding, action)| (binding, action.clone()))
        else {
            return;
        };
        let kind = action.kind.clone();
        if self.request(&kind, Request::Invoke { binding, action }) {
            self.counters.invokes_sent += 1;
        }
    }

    /// Run the action of `binding` as a gesture does. See
    /// [`Command::TestAction`].
    fn test_action(&mut self, binding: BindingId) {
        if !self.location.shows(binding) {
            self.problem(
                "OpenCraft did not test the action, because its page is not the page \
                 that the device shows."
                    .to_string(),
            );
            return;
        }
        let Some(action) = self
            .location
            .binding(&self.profile, binding.control, binding.gesture)
            .map(|(_, action)| action.clone())
        else {
            self.problem("OpenCraft did not test the action, because the gesture has none.".into());
            return;
        };
        let kind = action.kind.clone();
        if !self.request(&kind, Request::Invoke { binding, action }) {
            self.problem(format!(
                "OpenCraft cannot test this action, because this version of OpenCraft does not \
                 have it ({kind})."
            ));
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
            self.draw(false, &[surface]);
        }
    }

    /// Accept one message from the shared handler queue.
    pub fn handler_message(&mut self, message: Message) {
        match message {
            Message::Feedback { binding, feedback } => {
                if !self.location.shows(binding)
                    || self
                        .location
                        .binding(&self.profile, binding.control, binding.gesture)
                        .is_none()
                    || self.feedback_surface(binding.control).is_none()
                {
                    return;
                }
                if let Some(mut feedback) = feedback {
                    feedback.level = feedback.level.map(|v| v.min(100));
                    if let Some(other) = &mut feedback.other_state {
                        other.level = other.level.map(|v| v.min(100));
                    }
                    self.feedback_order += 1;
                    self.feedback
                        .insert(binding, (self.feedback_order, feedback));
                } else {
                    self.feedback.remove(&binding);
                }
                self.refresh_feedback_surface(binding.control);
            }
            Message::Choices { id, options } => (self.sink)(Event::Choices { id, options }),
            Message::Status { kind, status } => (self.sink)(Event::Status { kind, status }),
            Message::Problem(text) => self.problem(text),
            Message::Engine(EngineRequest::DeviceBrightness(change)) => {
                let mut settings = self.settings.clone();
                settings.brightness = match change {
                    crate::BrightnessChange::Set(value) => value,
                    crate::BrightnessChange::Step(step) => {
                        (i32::from(settings.brightness) + i32::from(step)).clamp(0, 100) as u8
                    }
                };
                self.set_device_settings(settings);
            }
            Message::Engine(EngineRequest::DeviceDisplay(state)) => {
                if !self.power.locked && !self.power.sleeping {
                    self.set_display_state(state);
                }
            }
            Message::Engine(EngineRequest::SelectPage(target)) => {
                let count = self.profile.pages().len();
                self.select_page(target.resolve(self.location.page, count));
            }
            Message::Engine(EngineRequest::OpenFolder(id)) => self.open_folder(id),
            Message::Engine(EngineRequest::CloseFolder) => self.close_folder(),
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
            Message::Engine(EngineRequest::SelectProfile { target, page }) => {
                self.select_profile_request(target, page);
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
        self.follow_front = self.profiles.follow_front();
        self.default_mark = self.profiles.default_of(model);
        self.last_target = None;
        self.chosen_in_window = false;
        self.cause = ActiveCause::Selected;
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
                    // The kept steps are for the pages of the old file.
                    self.history.forget(&id);
                    (empty(), true)
                }
                None => {
                    // A kept step of the history can use an image that the
                    // profile does not use now. An undo must find it.
                    let kept = self
                        .history
                        .kept_copies(&id)
                        .flat_map(|(pages, folders)| image_uses_of(pages, folders))
                        .map(|(path, _)| path);
                    if let Err(e) = store.remove_unused_images_but(&loaded.profile, kept) {
                        self.problem(format!("OpenCraft could not remove unused images: {e}"));
                    }
                    (loaded.profile, true)
                }
            },
            Err(e) => {
                self.history.forget(&id);
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
    /// first page (see [`Profile::first_page`]) as the active page. Put it
    /// in the list of profiles.
    fn use_profile(&mut self, id: String, profile: Profile, can_save: bool) {
        self.store = self.profiles.store(&id);
        self.location.page = profile.first_page();
        self.location.open.clear();
        self.profile = profile;
        self.can_save = can_save;
        self.changed_at = None;
        self.save_failed = false;
        // The steps of the old profile wait until it is active again, and
        // the steps of the new profile come back (decision 0030).
        self.history.change_profile(&self.profile_id, &id);
        self.profile_id = id;
        self.update_entry();
    }

    /// Put the name of the active profile in the list of profiles. Add the
    /// entry if the list does not have it.
    fn update_entry(&mut self) {
        let entry = ProfileEntry::of(self.profile_id.clone(), &self.profile);
        match self.entries.binary_search_by(|e| e.id.cmp(&entry.id)) {
            Ok(at) => self.entries[at] = entry,
            Err(at) => self.entries.insert(at, entry),
        }
    }

    /// Send the list of profiles and the identity of the active profile. See
    /// [`Event::Profiles`].
    fn send_profiles(&mut self) {
        self.assign_clashes();
        let default = self.default_profile_id();
        (self.sink)(Event::Profiles {
            profiles: self.entries.clone(),
            active: self.profile_id.clone(),
            default,
            cause: self.cause.clone(),
        });
    }

    /// Tell the shell the setting of the automatic switch.
    fn send_follow(&mut self) {
        (self.sink)(Event::FollowFrontApplication(self.follow_front));
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
        self.note_manual_selection();
        self.change_profile(id.to_string(), None);
    }

    /// [`EngineRequest::SelectProfile`]. A key uses this. The window uses
    /// [`Command::SelectProfile`], which names an identity and shows the
    /// first page.
    fn select_profile_request(&mut self, target: ProfileTarget, page: Option<usize>) {
        /// The page of a profile request.
        enum AskedPage {
            /// No page. A different profile opens at its first page. The
            /// active profile stays on its page.
            Unspecified,
            /// This page, from 0.
            Number(usize),
            /// The request named a page that the profile does not have.
            /// Open the first page.
            First,
        }

        let Some(id) = self.resolve_profile_target(target) else {
            return;
        };
        let (name, count, first) = self.profile_page_facts(&id);
        let page = match page {
            Some(page) if page < count => AskedPage::Number(page),
            Some(page) => {
                self.problem(format!("The profile “{name}” has no page {}.", page + 1));
                AskedPage::First
            }
            None => AskedPage::Unspecified,
        };
        if id != self.profile_id {
            if !self.save_before_leaving() {
                return;
            }
            let page = match page {
                AskedPage::Number(page) => Some(page),
                AskedPage::First | AskedPage::Unspecified => None,
            };
            self.note_manual_selection();
            self.change_profile_at(id, None, page);
            return;
        }
        match page {
            AskedPage::Unspecified => {}
            AskedPage::First => self.select_page(first),
            AskedPage::Number(page) => self.select_page(page),
        }
    }

    /// The profile that `target` names. `None` after a problem, or when the
    /// model has no profile.
    fn resolve_profile_target(&mut self, target: ProfileTarget) -> Option<String> {
        match target {
            ProfileTarget::Profile(text) => {
                if let Some(entry) = self.entries.iter().find(|entry| entry.id == text) {
                    return Some(entry.id.clone());
                }
                if let Some(entry) = self.entries.iter().find(|entry| entry.name == text) {
                    return Some(entry.id.clone());
                }
                self.problem(format!(
                    "OpenCraft did not change the profile, because no profile has the \
                     identity or the name “{text}”."
                ));
                None
            }
            other => {
                let count = self.entries.len();
                if count == 0 {
                    return None;
                }
                let index = self
                    .entries
                    .iter()
                    .position(|entry| entry.id == self.profile_id)
                    .unwrap_or(0);
                let step = if other == ProfileTarget::Next {
                    1
                } else {
                    count - 1
                };
                Some(self.entries[(index + step) % count].id.clone())
            }
        }
    }

    /// The name, the page count, and the first page of the profile `id`.
    fn profile_page_facts(&self, id: &str) -> (String, usize, usize) {
        if id == self.profile_id {
            return (
                self.profile.name.clone(),
                self.profile.pages().len(),
                self.profile.first_page(),
            );
        }
        match self.entries.iter().find(|entry| entry.id == id) {
            Some(entry) => (entry.name.clone(), entry.pages.len(), entry.first_page),
            None => (id.to_string(), 0, 0),
        }
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
        self.change_profile_at(id, profile, None);
    }

    /// [`Self::change_profile`]. `page` becomes the active page when it is a
    /// page of the profile. `None` keeps the first page.
    fn change_profile_at(&mut self, id: String, profile: Option<Profile>, page: Option<usize>) {
        self.disappear();
        match profile {
            Some(profile) => {
                // A profile in memory is a new profile: an add, an import,
                // or a starter profile. It has no history.
                self.history.forget(&id);
                self.use_profile(id, profile, true)
            }
            None => self.load_profile(id),
        }
        if let Some(page) = page.filter(|page| *page < self.profile.pages().len()) {
            self.location.page = page;
        }
        self.redraw();
        self.send_profiles();
        self.send_pages();
        self.send_history();
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
        // A duplicate copies the pages, not the links (decision 0035).
        profile.applications.clear();
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
        self.note_manual_selection();
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

    /// Change a part of the profile `id` that is not in the pages with
    /// `change`, and send the list of profiles if the part changed. For a
    /// profile that is not active, the engine reads its file, changes it,
    /// and writes it. `what` names the change in a problem.
    fn change_profile_settings(
        &mut self,
        id: String,
        what: &str,
        change: impl Fn(&mut Profile) -> bool,
    ) {
        if !self.is_profile(&id) {
            return;
        }
        if id == self.profile_id {
            if !change(&mut self.profile) {
                return;
            }
            self.changed_at = Some(self.now());
            self.update_entry();
        } else {
            let store = self.profiles.store(&id);
            let changed = store.read().and_then(|mut profile| {
                if !change(&mut profile) {
                    return Ok(None);
                }
                store.save(&profile)?;
                Ok(Some(profile))
            });
            match changed {
                Ok(Some(profile)) => {
                    if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
                        *entry = ProfileEntry::of(id, &profile);
                    }
                }
                Ok(None) => return,
                Err(e) => {
                    self.problem(format!("OpenCraft could not change the {what}: {e}"));
                    return;
                }
            }
        }
        self.send_profiles();
    }

    fn set_first_page(&mut self, id: String, page: usize) {
        if id == self.profile_id {
            if !self.is_page(page) {
                return;
            }
            if self.profile.set_first_page(page) {
                self.changed_at = Some(self.now());
                self.send_pages();
            }
            return;
        }
        self.change_profile_settings(id, "first page", |profile| profile.set_first_page(page));
    }

    /// Write the profile `id` to the file `path`. This is
    /// [`Command::ExportProfile`]. For the active profile, the engine uses
    /// the profile in memory, thus a change that is not in its file yet is
    /// in the export.
    fn export_profile(&mut self, id: String, path: PathBuf) {
        if !self.is_profile(&id) {
            return;
        }
        let store = self.profiles.store(&id);
        let profile = if id == self.profile_id {
            Ok(self.profile.clone())
        } else {
            store.read()
        };
        match profile.and_then(|profile| crate::share::export(&profile, &store, &path)) {
            Ok(()) => (self.sink)(Event::ProfileExported { profile: id, path }),
            Err(e) => self.problem(format!("OpenCraft could not export the profile: {e}")),
        }
    }

    /// Add a profile from the file `path`. This is
    /// [`Command::ImportProfile`].
    fn import_profile(&mut self, path: PathBuf) {
        let failed = |e: &dyn std::fmt::Display| {
            format!("OpenCraft could not import the profile, because {e}.")
        };
        let package = match crate::share::import(&path) {
            Ok(package) => package,
            Err(e) => return self.problem(failed(&e)),
        };
        let model = self.descriptor.model;
        if package.profile.model != model {
            return self.problem(failed(&format!(
                "the profile in the file is for a different device model ({})",
                package.profile.model
            )));
        }
        let name = package.profile.name.trim().to_string();
        let name = free_profile_name(
            if name.is_empty() {
                DEFAULT_PROFILE_NAME
            } else {
                &name
            },
            &self.entries,
        );
        if !self.save_before_leaving() {
            return;
        }
        let id = match self.profiles.new_id(&name, &[&self.profile_id]) {
            Ok(id) => id,
            Err(e) => return self.problem(failed(&e)),
        };
        let store = self.profiles.store(&id);
        let size = self.background_size();
        let written = match package.write_images(&store, (size.width, size.height)) {
            Ok(mut profile) => {
                profile.name = name;
                self.drop_clashing_links(&mut profile);
                store.save(&profile).map(|()| profile)
            }
            Err(error) => Err(error),
        };
        match written {
            Ok(profile) => {
                self.note_manual_selection();
                self.change_profile(id.clone(), Some(profile));
                (self.sink)(Event::ProfileImported { profile: id });
            }
            Err(e) => {
                // The directory is new and holds no profile of the user.
                let _ = std::fs::remove_dir_all(store.dir());
                self.problem(failed(&e));
            }
        }
    }

    /// Add the starter profile `profile`. This is
    /// [`Command::AddStarterProfile`].
    fn add_starter_profile(&mut self, mut profile: Profile) {
        let failed = |e: &dyn std::fmt::Display| {
            format!("OpenCraft could not add the profile, because {e}.")
        };
        if profile.model != self.descriptor.model {
            return self.problem(failed(&format!(
                "the profile is for a different device model ({})",
                profile.model
            )));
        }
        if profile.image_paths().next().is_some() {
            return self.problem(failed(&"the profile refers to image files"));
        }
        // The profile that the engine made at the start is not in a file,
        // and the user did not change it.
        let replace =
            self.can_save && self.changed_at.is_none() && !self.store.profile_path().exists();
        let others: Vec<ProfileEntry> = self
            .entries
            .iter()
            .filter(|e| !replace || e.id != self.profile_id)
            .cloned()
            .collect();
        let name = profile.name.trim().to_string();
        profile.name = free_profile_name(
            if name.is_empty() {
                DEFAULT_PROFILE_NAME
            } else {
                &name
            },
            &others,
        );
        if !replace && !self.save_before_leaving() {
            return;
        }
        let taken: Vec<&str> = if replace {
            Vec::new()
        } else {
            vec![self.profile_id.as_str()]
        };
        let id = match self.profiles.new_id(&profile.name, &taken) {
            Ok(id) => id,
            Err(e) => return self.problem(failed(&e)),
        };
        let store = self.profiles.store(&id);
        self.drop_clashing_links(&mut profile);
        if let Err(e) = store.save(&profile) {
            // The directory is new and holds no profile of the user.
            let _ = std::fs::remove_dir_all(store.dir());
            return self.problem(failed(&e));
        }
        if replace {
            self.entries = others;
        }
        self.note_manual_selection();
        self.change_profile(id, Some(profile));
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
        if self.default_mark.as_deref() == Some(id.as_str()) {
            self.default_mark = None;
            if let Err(e) = self.profiles.clear_default(self.descriptor.model) {
                self.problem(format!(
                    "OpenCraft could not record the default profile: {e}"
                ));
            }
        }
        self.entries.remove(at);
        if active {
            let next = self.entries[at.min(self.entries.len() - 1)].id.clone();
            self.note_manual_selection();
            self.change_profile(next, None);
        } else {
            self.send_profiles();
        }
        // The history of the removed profile goes away with it. A new
        // profile can get the same identity later.
        self.history.forget(&id);
    }

    /// Send the layout of the model, make the image of each surface, and
    /// send the state of each surface.
    fn show_model(&mut self) {
        (self.sink)(Event::Layout(Arc::new(DeviceLayout::from(self.descriptor))));
        (self.sink)(Event::Kinds(
            self.ports.iter().flat_map(|p| p.kinds.clone()).collect(),
        ));
        self.images.clear();
        self.render_background();
        for surface in self.descriptor.slot_surfaces() {
            self.render(surface);
        }
    }

    /// The size of the background of the model, in pixels. A model with no
    /// background gives the size of its display.
    fn background_size(&self) -> Size {
        let (width, height) = self
            .descriptor
            .background()
            .map_or(self.descriptor.display_size(), |s| (s.width, s.height));
        Size {
            width: u32::from(width),
            height: u32::from(height),
        }
    }

    /// Make the background image of the active page, keep it, and send the
    /// state of the background. Make the images of the surfaces after
    /// this: a surface with no fill colour shows a part of it.
    fn render_background(&mut self) {
        let background = self.page().background.clone();
        let image = match (&background, self.descriptor.background()) {
            (Some(background), Some(_)) => {
                let source =
                    background
                        .image()
                        .and_then(|path| match self.store.load_image(path) {
                            Ok(image) => Some(image),
                            Err(e) => {
                                self.problem(format!(
                                "OpenCraft could not show the background image of the page: {e}"
                            ));
                                None
                            }
                        });
                Some(Arc::new(render::background_image(
                    self.background_size(),
                    background,
                    source.as_ref(),
                )))
            }
            _ => None,
        };
        self.background = image.clone();
        let state = BackgroundState { background, image };
        if state != self.background_sent {
            self.background_sent = state.clone();
            (self.sink)(Event::Background(state));
        }
    }

    /// Change the background of `page` with `change`. If it changes on the
    /// active page, make the new images, send them to the shell, and draw
    /// on the device only the parts that changed.
    fn change_background(&mut self, page: usize, change: impl FnOnce(&mut Option<Background>)) {
        if !self.is_page(page) {
            return;
        }
        let Some(target) = self.profile.page_mut(page) else {
            return;
        };
        let before = target.background.clone();
        change(&mut target.background);
        if target.background == before {
            return;
        }
        self.changed_at = Some(self.now());
        if page == self.location.page {
            self.redraw();
        }
    }

    /// Set the background of `page`. This is [`Command::SetBackground`].
    fn set_background(&mut self, page: usize, background: Option<NewBackground>) {
        if !self.is_page(page) {
            return;
        }
        if background.is_some() && self.descriptor.background().is_none() {
            self.problem("This device has no background.".to_string());
            return;
        }
        let new = match background {
            None => None,
            Some(NewBackground::Color(color)) => Some(Background::Color(color)),
            Some(NewBackground::Image { path, fit }) => {
                let size = self.background_size();
                match self
                    .store
                    .import_background(&path, (size.width, size.height))
                {
                    Ok(image) => Some(Background::Image { image, fit }),
                    Err(e) => {
                        self.problem(format!("OpenCraft could not use the image: {e}"));
                        return;
                    }
                }
            }
        };
        self.change_background(page, |background| *background = new);
    }

    /// Make the image of `surface` from the profile, keep it, and send the
    /// state of the surface.
    fn render(&mut self, surface: &Surface) {
        let content = self.location.content(&self.profile, surface.id);
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
        // A surface with no fill colour shows the background (decision 0032).
        let backdrop = self.background.as_ref().map(|background| {
            render::backdrop(background, u32::from(surface.x), u32::from(surface.y), size)
        });
        let image = Arc::new(render::surface_image_on_background(
            size,
            &content,
            source.as_ref(),
            feedback,
            backdrop.as_ref(),
        ));
        let states = feedback.and_then(|feedback| {
            let now = feedback.state?;
            let other = render::other_state(feedback);
            let other = Arc::new(render::surface_image_on_background(
                size,
                &content,
                source.as_ref(),
                Some(&other),
                backdrop.as_ref(),
            ));
            let (off, on) = match now {
                FeedbackState::Off => (Arc::clone(&image), other),
                FeedbackState::On => (other, Arc::clone(&image)),
            };
            Some(StateImages { now, off, on })
        });
        self.images.insert(surface.id, Arc::clone(&image));
        (self.sink)(Event::Surface(SurfaceState {
            id: surface.id,
            content,
            image,
            states,
        }));
    }

    /// Obey one command. Return false if the engine must stop. A command
    /// that changes the pages is one step of the history: see `step_of`.
    pub fn command(&mut self, command: Command) -> bool {
        let Some(command) = self.for_active_profile(command) else {
            return true;
        };
        match step_of(&command, &self.location) {
            Some((name, target)) => self.step(name, target, command),
            None => self.obey(command),
        }
    }

    /// The command in `command`, with no [`Command::InProfile`] around it.
    /// `None` if it is for a profile that is not the active profile: then
    /// the engine gives a problem.
    fn for_active_profile(&mut self, command: Command) -> Option<Command> {
        match command {
            Command::InProfile { profile, command } => {
                if profile == self.profile_id {
                    self.for_active_profile(*command)
                } else {
                    self.problem(
                        "OpenCraft did not do the change: the window showed a different \
                         profile than the active profile. Look at the window, and do it again."
                            .to_string(),
                    );
                    None
                }
            }
            command => Some(command),
        }
    }

    /// Obey `command` as one step of the history with the name `name`
    /// (decision 0030). In a text edit, a change of the same `target` as
    /// the last step joins that step. Else the engine keeps a copy of the
    /// pages before the command, and makes a step if the pages changed.
    fn step(&mut self, name: String, target: Option<Target>, command: Command) -> bool {
        if self.history.joins(target) {
            let names = self.history.names();
            let go_on = self.obey(command);
            self.history
                .joined(self.profile.pages(), self.profile.folders(), &self.location);
            if self.history.names() != names {
                self.send_history();
            }
            return go_on;
        }
        let before_pages = self.profile.pages().to_vec();
        let before_folders = self.profile.folders().to_vec();
        let undo_place = self.location.clone();
        let go_on = self.obey(command);
        if self.profile.pages() != before_pages.as_slice()
            || self.profile.folders() != before_folders.as_slice()
        {
            self.history.push(
                name,
                target,
                SavedProfile {
                    pages: before_pages,
                    folders: before_folders,
                },
                undo_place,
                self.location.clone(),
            );
            self.send_history();
        }
        go_on
    }

    /// Send the names of the steps of the history. See [`Event::History`].
    fn send_history(&mut self) {
        let (undo, redo) = self.history.names();
        (self.sink)(Event::History { undo, redo });
    }

    /// Undo the last step of the history. This is [`Command::Undo`]. The
    /// engine puts back the pages from before the step, and makes the page
    /// that was active before the step the active page. It does this with
    /// the steps of a page change of [`Self::select_page`], thus the device
    /// and the shell show the result. The file is written after
    /// [`SAVE_DELAY`], as after each change.
    pub fn undo(&mut self) {
        let mut history = std::mem::take(&mut self.history);
        let done = history.undo(|saved, place| {
            let (pages, folders) =
                self.swap_location(saved.pages, saved.folders, place.page, place.open);
            SavedProfile { pages, folders }
        });
        self.history = history;
        if done {
            self.send_history();
        }
    }

    /// Do the last undone step again. This is [`Command::Redo`]. The steps
    /// are those of [`Self::undo`], with the page that was active after
    /// the step.
    pub fn redo(&mut self) {
        let mut history = std::mem::take(&mut self.history);
        let done = history.redo(|saved, place| {
            let (pages, folders) =
                self.swap_location(saved.pages, saved.folders, place.page, place.open);
            SavedProfile { pages, folders }
        });
        self.history = history;
        if done {
            self.send_history();
        }
    }

    /// Put `pages` and `folders` in place, with `page` and `open` as the
    /// location, and return the pages and the folders that were there.
    fn swap_location(
        &mut self,
        pages: Vec<Page>,
        folders: Vec<Folder>,
        page: usize,
        open: Vec<FolderId>,
    ) -> (Vec<Page>, Vec<Folder>) {
        self.changed_at = Some(self.now());
        self.disappear();
        let old_pages = self.profile.replace_pages(pages);
        let old_folders = self.profile.replace_folders(folders);
        let count = self.profile.pages().len();
        self.location.page = page.min(count.saturating_sub(1));
        self.location.open = Location::restored_open(&self.profile, &open);
        self.redraw();
        self.send_pages();
        self.appear();
        (old_pages, old_folders)
    }

    /// Obey one command with no step of the history. Return false if the
    /// engine must stop.
    fn obey(&mut self, command: Command) -> bool {
        match command {
            Command::SetLabel { surface, label } => self.change(surface, |c| c.label = label),
            Command::SetLabelStyle { surface, change } => {
                self.change(surface, |c| c.label_style.apply(change))
            }
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
                self.change(surface, |c| {
                    if image.is_some() {
                        c.icon = None;
                    }
                    c.image = image;
                });
            }
            Command::SetIcon { surface, icon } => {
                if let Some(icon) = &icon {
                    if icons::icon_set().get(&icon.name).is_none() {
                        self.problem(format!(
                            "The icon set does not have the icon '{}'.",
                            icon.name
                        ));
                        return true;
                    }
                }
                self.change(surface, |c| {
                    c.image = None;
                    c.icon = icon;
                });
            }
            Command::SetBinding { binding, action } => self.set_binding(binding, action),
            Command::SetBackground { page, background } => self.set_background(page, background),
            Command::SetBackgroundFit { page, fit } => self.change_background(page, |background| {
                if let Some(Background::Image { fit: old, .. }) = background {
                    *old = fit;
                }
            }),
            Command::SelectPage { page } => self.select_page(page),
            Command::AddPage { copy_of } => self.add_page(copy_of),
            Command::RenamePage { page, name } => self.rename_page(page, name),
            Command::MovePage { page, to } => self.move_page(page, to),
            Command::RemovePage { page } => self.remove_page(page),
            Command::AddFolder { id, name, copy_of } => self.add_folder(id, name, copy_of),
            Command::RenameFolder { folder, name } => self.rename_folder(folder, name),
            Command::MoveFolder { folder, to } => self.move_folder(folder, to),
            Command::RemoveFolder { folder } => self.remove_folder(folder),
            Command::OpenFolder { folder } => self.open_folder(folder),
            Command::CloseFolder => self.close_folder(),
            Command::SelectProfile { profile } => self.select_profile(&profile),
            Command::AddProfile { name, copy_of } => self.add_profile(name, copy_of),
            Command::RenameProfile { profile, name } => self.rename_profile(profile, name),
            Command::RemoveProfile { profile } => self.remove_profile(profile),
            Command::SetProfileColor { profile, color } => {
                self.change_profile_settings(profile, "colour of the profile", |p| {
                    std::mem::replace(&mut p.color, color) != color
                })
            }
            Command::SetFirstPage { profile, page } => self.set_first_page(profile, page),
            Command::ExportProfile { profile, path } => self.export_profile(profile, path),
            Command::ImportProfile { path } => self.import_profile(path),
            Command::AddStarterProfile { profile } => self.add_starter_profile(profile),
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
            Command::StartTextEdit => self.history.start_text_edit(),
            Command::EndTextEdit => self.history.end_text_edit(),
            Command::InProfile { .. } => {
                return match self.for_active_profile(command) {
                    Some(command) => self.obey(command),
                    None => true,
                };
            }
            Command::Group { commands, .. } => {
                let mut go_on = true;
                // A command for the history has no meaning in a step.
                for command in commands.into_iter().filter(|c| {
                    !matches!(
                        c,
                        Command::Undo
                            | Command::Redo
                            | Command::StartTextEdit
                            | Command::EndTextEdit
                    )
                }) {
                    go_on &= self.obey(command);
                }
                return go_on;
            }
            Command::CopyControl { control } => self.copy_control(control),
            Command::PasteControl { control } => self.paste_control(control),
            Command::ClearControl { control } => {
                if self.parts(control).is_some() {
                    self.put_control(control, &ControlContent::empty(control.kind));
                }
            }
            Command::CopyToAllPages { control } => self.copy_to_all_pages(control),
            Command::SetDeviceSettings(settings) => self.set_device_settings(settings),
            Command::Choices { id, action, field } => {
                let kind = action.kind.clone();
                if !self.request(&kind, Request::Choices { id, action, field }) {
                    (self.sink)(Event::Choices {
                        id,
                        options: Vec::new(),
                    });
                }
            }
            Command::PowerNotice(notice) => self.power_notice(notice),
            Command::FrontApplication(front) => self.front_application(front),
            Command::LinkApplication { profile, link } => self.link_application(profile, link),
            Command::UnlinkApplication { profile, bundle_id } => {
                self.unlink_application(profile, bundle_id)
            }
            Command::SetDefaultProfile { profile } => self.set_default_profile(profile),
            Command::SetFollowFrontApplication(follow) => self.set_follow_front(follow),
            Command::WakeDevice => self.wake_device(),
            Command::Resolve { kind } => {
                self.request(&kind, Request::Resolve { kind: kind.clone() });
            }
            Command::TestAction { binding } => self.test_action(binding),
            Command::ReadCounters => (self.sink)(Event::Counters(self.counters)),
            Command::Stop => return false,
        }
        true
    }

    /// The parts of `control` in the descriptor. If the device has no such
    /// control, tell the shell.
    fn parts(&mut self, control: ControlId) -> Option<Parts> {
        let parts = Parts::of(self.descriptor, control);
        if parts.is_none() {
            self.problem(format!("This device has no {control}."));
        }
        parts
    }

    /// Copy the content of `control` on the active page to the clipboard.
    /// This is [`Command::CopyControl`].
    fn copy_control(&mut self, control: ControlId) {
        let Some(parts) = self.parts(control) else {
            return;
        };
        let content = ControlContent::read(
            self.location.page_of(&self.profile, control),
            control,
            parts,
        );
        self.clipboard = Some((content, self.store.dir().to_path_buf()));
        (self.sink)(Event::Clipboard(control.kind));
    }

    /// Put the content of the clipboard on `control` on the active page.
    /// This is [`Command::PasteControl`]. A copy from a different profile
    /// has its image in the directory of that profile: the engine copies
    /// the image into the active profile first.
    fn paste_control(&mut self, control: ControlId) {
        let Some((mut content, source)) = self.clipboard.clone() else {
            return;
        };
        if content.kind != control.kind {
            self.problem(format!(
                "OpenCraft did not paste: the copy is of a {}, and a paste is possible only \
                 on a {}.",
                content.kind.name(),
                content.kind.name()
            ));
            return;
        }
        if self.parts(control).is_none() {
            return;
        }
        if source != self.store.dir() {
            if let Some(image) = content.image() {
                match self.store.import_image(&source.join(image)) {
                    Ok(copy) => {
                        if let Some(surface) = content.surface.as_mut() {
                            surface.image = Some(copy);
                        }
                    }
                    Err(e) => {
                        self.problem(format!("OpenCraft could not use the image: {e}"));
                        return;
                    }
                }
            }
        }
        self.put_control(control, &content);
    }

    /// Put `content` on `control` on the active page, with the steps of a
    /// change of a surface and of a binding: thus the device, the shell,
    /// and the handlers get each change.
    fn put_control(&mut self, control: ControlId, content: &ControlContent) {
        let Some(parts) = Parts::of(self.descriptor, control) else {
            return;
        };
        let (surface, bindings) = content.changes(
            self.location.page_of(&self.profile, control),
            control,
            parts,
        );
        // The surface first: a shell then gets the new content before the
        // bindings, for example for the text field of the label.
        if let Some((id, new)) = surface {
            self.change(id, |c| *c = new);
        }
        for (control, gesture, action) in bindings {
            let binding = self.location.binding_id(control, gesture);
            self.set_binding(binding, action);
        }
    }

    /// Put the content of `control` on the active page on the same control
    /// on each other page. This is [`Command::CopyToAllPages`]. The device
    /// shows the active page, which does not change, thus no handler and no
    /// image changes.
    fn copy_to_all_pages(&mut self, control: ControlId) {
        if !self.location.open.is_empty() {
            self.problem("Copy to All Pages is not possible in a folder.".to_string());
            return;
        }
        let Some(parts) = self.parts(control) else {
            return;
        };
        let content = ControlContent::read(self.page(), control, parts);
        let mut changed = false;
        for number in (0..self.profile.pages().len()).filter(|n| *n != self.location.page) {
            if let Some(page) = self.profile.page_mut(number) {
                changed |= content.write(page, control, parts);
            }
        }
        if changed {
            self.changed_at = Some(self.now());
        }
    }

    /// Change the content of the surface `id`. If the content is different
    /// after the change, make the new image, send it to the shell, and draw
    /// it on the device.
    fn change(&mut self, id: SurfaceId, change: impl FnOnce(&mut SurfaceContent)) {
        let Some(surface) = self.descriptor.surface(id).filter(|s| s.slot.is_some()) else {
            self.problem(format!("This device has no {id} with content."));
            return;
        };
        let updated = self
            .location
            .surface_for_change(&mut self.profile, id)
            .is_some_and(|page| page.update(id, change));
        if !updated {
            return;
        }
        self.changed_at = Some(self.now());
        self.render(surface);
        if self.link.is_connected() && !self.needs_draw {
            self.draw(false, &[surface]);
        }
    }

    /// Wait up to `timeout` for the next event of the link, and do the work
    /// for it. Then write the profile and the device settings if a write is
    /// due.
    ///
    /// Call this function in a loop. It sends the keep-alive (through the
    /// link), so the time between two calls must be short.
    pub fn poll(&mut self, timeout: Duration) {
        self.check_idle();
        if self.needs_draw && self.link.is_connected() {
            self.draw_all();
        }
        let timeout = self.power.idle_deadline.map_or(timeout, |at| {
            timeout.min(at.saturating_duration_since(self.now()))
        });
        match self.link.next_event(timeout) {
            Ok(LinkEvent::Connected) => {
                self.link_problem = None;
                let model = self.link.device().map(|d| d.descriptor());
                if let Some(model) = model.filter(|m| !std::ptr::eq(*m, self.descriptor)) {
                    self.change_model(model);
                }
                log("connected");
                // The idle time is the time with no input on a connected
                // device. A device that connects late starts it again.
                self.restart_idle();
                self.draw_all();
                self.announce_connected();
            }
            Ok(LinkEvent::Woke { slept }) => {
                log(&format!(
                    "wake detected: the computer slept {:.1} s",
                    slept.as_secs_f64()
                ));
                self.power_notice(PowerNotice::DidWake);
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
                    if !self.consume_dark_input(event) {
                        (self.sink)(Event::Input(event));
                        self.invoke(event);
                    }
                }
            }
            Ok(LinkEvent::Timeout) => {}
            Err(e) => self.link_error(&e),
        }
        self.check_idle();
        self.save_if_due();
    }

    /// Tell the shell that the device is ready, after the full draw. If the
    /// draw lost the device, the link gives `Disconnected` next, and the
    /// shell gets no `Connected`.
    fn announce_connected(&mut self) {
        let firmware = self
            .link
            .with_device(|device| device.firmware_version())
            .ok()
            .map(|version| {
                version
                    .chars()
                    .filter(|c| !c.is_control())
                    .collect::<String>()
            })
            .filter(|version| !version.trim().is_empty());
        if self.link.is_connected() {
            (self.sink)(Event::Firmware(firmware));
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
            self.restart_idle();
        }
        self.show_model();
        if new_model {
            self.send_profiles();
            self.send_pages();
            self.send_history();
            self.appear();
            self.send_settings();
        }
    }

    /// Send the start sequence, the device settings, and the image of each
    /// surface to the device (rule 10). The order is that of decision 0022:
    /// `DIS`, `LIG` with the brightness setting, `CLE 0xFF`, `STP`, then the
    /// knob light commands (`SETLB` and `LBLIG`, or `LBLIG 0` only), then
    /// `QUCMD`, then the background (`BGPIC` with the image, or `BGCLE` for a
    /// page with no background; decision 0032), then the images, then
    /// `STP`. The device keeps no setting after a USB restart, and not the
    /// knob light colours after `DC` (measured), so each full draw sends all
    /// settings.
    fn draw_all(&mut self) {
        if self.power.state != DisplayState::Awake {
            self.apply_dark();
            return;
        }
        if self.power.ready_after.is_some_and(|at| self.now() < at) {
            self.needs_draw = true;
            return;
        }
        self.power.ready_after = None;
        let mut encoded = Vec::with_capacity(self.images.len());
        for surface in self.descriptor.slot_surfaces() {
            if let Some(data) = self.encode(surface) {
                encoded.push(data);
            }
        }
        let background = self.encode_background();
        let brightness = self.settings.brightness;
        let commands = SettingCommands::new(&self.settings, self.descriptor.settings);
        let result = self.link.with_device(|device| {
            device.start(brightness)?;
            for command in commands.after_start() {
                device.send(command.clone())?;
            }
            // The device possibly keeps a background of an earlier run,
            // thus a page with no background removes it.
            if let Some(background) = &background {
                background.send(device)?;
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
    /// With `background`, send the background of the active page first: its
    /// image, or the command that removes it. With nothing to send, send
    /// nothing.
    fn draw(&mut self, background: bool, surfaces: &[&Surface]) {
        if self.power.state != DisplayState::Awake {
            return;
        }
        let background = if background {
            self.encode_background()
        } else {
            None
        };
        let encoded: Vec<_> = surfaces.iter().filter_map(|s| self.encode(s)).collect();
        let written = encoded.len() + usize::from(background.is_some());
        if written == 0 {
            self.last_written = 0;
            return;
        }
        let result = self.link.with_device(|device| {
            if let Some(background) = &background {
                background.send(device)?;
            }
            for (slot, data) in &encoded {
                device.send_image(*slot, data)?;
            }
            device.commit()
        });
        match result {
            Ok(()) => self.last_written = written,
            Err(DeviceError::Disconnected) => self.last_written = 0,
            Err(e) => {
                // The state of the device is not known. Draw all again.
                self.last_written = 0;
                self.link_error(&e);
                self.needs_draw = true;
            }
        }
    }

    /// The write of the background of the active page, in the form that the
    /// device needs: its image, or the command that removes it. `None` for
    /// a model with no background, and after an error, which goes to the
    /// shell.
    fn encode_background(&mut self) -> Option<BackgroundWrite> {
        let surface = self.descriptor.background()?;
        let Some(image) = &self.background else {
            return Some(BackgroundWrite::Clear);
        };
        match encode_for(surface, image, jpeg_quality_for(surface)) {
            Ok(data) => Some(BackgroundWrite::Image(data)),
            Err(e) => {
                self.problem(format!(
                    "OpenCraft could not make the background image: {e}"
                ));
                None
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
