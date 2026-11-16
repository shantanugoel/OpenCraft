//! Wire data shared by the socket server and the `ctl` client.
//!
//! No engine command or event is a wire type. Optional values are JSON null.
//! Each request, including a read with no arguments, has an object in
//! `params`; a field of type `Option` can also be absent.
//!
//! # Names
//!
//! A profile, a page, and a folder have an identity: a text. A profile
//! identity is the name of its directory. A page identity and a folder
//! identity are 16 small hexadecimal digits. The identity of a page stays with
//! the page when the page moves, and when the profile is closed and read
//! again. A page number is not on the wire, except in `move_page`, where `to`
//! is a position in the list of the pages. Control and surface numbers are
//! those of the layout. The kinds and the gestures are enums with snake case
//! names (`key_face`, `turn_left`); the layout lists the gestures of each
//! control.
//!
//! # Reads
//!
//! `state` (all read state), `layout`, `kinds`, `profiles`,
//! `device_settings`, and `connection` take `{}`. `pages`, `folders`, and
//! `history` take `{}` or `{ "profile": <identity> }`: no profile is the
//! active profile. The history of a profile that is not active is the
//! history that OpenCraft kept for it; it cannot change while the profile is
//! not active. `content` takes `{}` or `{ "target": <target> }`. A read of
//! a profile that is not active reads its file and selects nothing; its
//! `pages` result has no `location`.
//! `choices` takes an action and a field key. Its result is a list of
//! choices; runtime choices arrive from the handler, not a stale cache. A
//! read of a runtime choice has a five-second limit; the error is `timeout`.
//! `hello` takes `{ "version": 1 }` and must precede other requests.
//!
//! # Changes
//!
//! The change methods are in [`Method`], with named parameters. A request
//! names the place of its change, in one of three forms:
//!
//! - [`Target`]: `{ "profile", "page", "folder" }`, the profile, the page,
//!   and a folder or null. For the content of a location: surfaces, bindings,
//!   and controls. `close_folder` needs the folder: it is the open folder of
//!   the location, and the request fails if it is not the last open folder of
//!   the active location.
//! - [`PageTarget`]: `{ "profile", "page" }`. For a request about a page:
//!   `rename_page`, `move_page`, `remove_page`, `select_page`,
//!   `duplicate_page`, `set_first_page`, `set_background`, and
//!   `set_background_fit`.
//! - A profile only: `add_page`, the requests for folders (a folder is a part
//!   of the profile), and the requests for profiles.
//!
//! One request is one command of the engine. The engine checks the place
//! first. If the profile, the page, or the folder is not there, or if the
//! page with this identity has a different place than the client read, the
//! request fails and changes nothing, also not the active profile. Else the
//! engine makes the profile the active profile and the page the active page,
//! and the device and the window show it. If the change itself is refused, the
//! engine goes back. A request is one step of the history; its name is the
//! name that the window uses for the same change. `undo` and `redo` only work
//! for the active profile.
//!
//! A change returns [`Changed`]: the place that the request names, with the
//! identity of the thing that it made, and null for each part that the
//! request has not. A problem of the engine is the error
//! `engine_problem` with its text. A change has no time limit: the response
//! comes after the engine obeyed the request, or the error comes when the
//! engine stops. File inputs and export destinations must be absolute paths.
//!
//! # Lines and connections
//!
//! Invalid requests have an error, with a null id when no valid id was
//! readable. Oversized lines and version mismatches close the connection
//! after the error. Other request errors leave it open. Each connection
//! processes requests in order. Requests are at most 1 MiB without the
//! newline; responses are at most 8 MiB without the newline. No stability is
//! promised before 1.0.
//!
//! The server closes a connection that sends no `hello` in 10 seconds, and a
//! connection that does not read its response in 10 seconds. It allows 16
//! connections; the next connection gets the error `busy` and is closed.
//! `hello` reads only `version`; the server version is in the text of the
//! error `version_mismatch`.
//!
//! # What the results hold
//!
//! No result contains rendered image bytes, device identity, engine logs,
//! or absolute image destinations. In a returned text (and in a name of an
//! object) that begins with the home directory of the user, the server
//! replaces the home directory with `~`. The rule is that of the export of a
//! profile: it ignores the case and acts only at a path boundary. A text that
//! has the home directory in the middle, for example a script, is not
//! changed. A path setting with `~/` is correct in a request: the handler
//! expands it. A file input of a request (an image, an export or import
//! file) must be an absolute path.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A location: the content of a page, or of a folder on a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub profile: String,
    pub page: String,
    pub folder: Option<String>,
}

/// A page of a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageTarget {
    pub profile: String,
    pub page: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentParams {
    pub target: Option<Target>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetParams {
    pub target: Target,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageTargetParams {
    pub target: PageTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetLabelParams {
    pub target: Target,
    pub surface: SurfaceId,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetLabelStyleParams {
    pub target: Target,
    pub surface: SurfaceId,
    pub style: Option<LabelStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetColorParams {
    pub target: Target,
    pub surface: SurfaceId,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetImageParams {
    pub target: Target,
    pub surface: SurfaceId,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetIconParams {
    pub target: Target,
    pub surface: SurfaceId,
    pub icon: Option<Icon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetBindingParams {
    pub target: Target,
    pub control: ControlId,
    pub gesture: Gesture,
    pub action: Option<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlParams {
    pub target: Target,
    pub control: ControlId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenamePageParams {
    pub target: PageTarget,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovePageParams {
    pub target: PageTarget,
    /// The position in the list of the pages, from 0.
    pub to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetBackgroundParams {
    pub target: PageTarget,
    pub background: Option<NewBackground>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetBackgroundFitParams {
    pub target: PageTarget,
    pub fit: BackgroundFit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddFolderParams {
    pub profile: String,
    /// The identity of the new folder: 16 small hexadecimal digits. Without
    /// it, OpenCraft makes a new random identity. The result has it.
    #[serde(default)]
    pub id: Option<String>,
    pub name: Option<String>,
    pub copy_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenameFolderParams {
    pub profile: String,
    pub folder: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveFolderParams {
    pub profile: String,
    pub folder: String,
    pub to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderParams {
    pub profile: String,
    pub folder: String,
}

/// `open_folder`: the folder opens in the location of the target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenFolderParams {
    pub target: Target,
    pub folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileParams {
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddProfileParams {
    pub name: String,
    pub copy_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenameProfileParams {
    pub profile: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetProfileColorParams {
    pub profile: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkApplicationParams {
    pub profile: String,
    pub link: ApplicationLink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnlinkApplicationParams {
    pub profile: String,
    pub bundle_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowParams {
    pub follow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportProfileParams {
    pub profile: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportProfileParams {
    pub path: String,
}

/// A file input, unlike Background, whose image is a stored relative name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum NewBackground {
    Color { color: String },
    Image { path: String, fit: BackgroundFit },
}

/// The result of each change request: the place that the request names (the
/// profile, the page, and the folder, each null if the request has no such
/// part), and for an add, the identity of the new thing in its field. A
/// client can use the identity at once: it is the identity that the engine
/// made, and no other request changed it in between. A request that removes a
/// thing names it, and the thing is gone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changed {
    /// The profile of the request. For `add_profile` and `import_profile`:
    /// the new profile.
    pub profile: Option<String>,
    /// The page of the request. For `add_page` and `duplicate_page`: the new
    /// page. For `add_profile` and `import_profile`: the first page of the
    /// new profile. For `undo` and `redo`: the page of the reversed step.
    pub page: Option<String>,
    /// The folder of the request. For `add_folder`: the new folder. For
    /// `open_folder`: the folder that opens. For `undo` and `redo`: the
    /// innermost open folder of the reversed step.
    pub folder: Option<String>,
}

pub const VERSION: u32 = 1;
pub const FILE_NAME: &str = "control.sock";
pub const MAX_LINE: usize = 1024 * 1024;
pub const MAX_RESPONSE: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub id: u64,
    #[serde(flatten)]
    pub method: Method,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Method {
    Hello(HelloParams),
    State(Empty),
    Layout(Empty),
    Kinds(Empty),
    Choices(ChoicesParams),
    Profiles(Empty),
    Pages(ProfileRead),
    Folders(ProfileRead),
    Content(ContentParams),
    SetLabel(SetLabelParams),
    SetLabelStyle(SetLabelStyleParams),
    SetColor(SetColorParams),
    SetImage(SetImageParams),
    SetIcon(SetIconParams),
    SetBinding(SetBindingParams),
    CopyControl(ControlParams),
    PasteControl(ControlParams),
    ClearControl(ControlParams),
    CopyToAllPages(ControlParams),
    AddPage(ProfileParams),
    DuplicatePage(PageTargetParams),
    RenamePage(RenamePageParams),
    MovePage(MovePageParams),
    RemovePage(PageTargetParams),
    SelectPage(PageTargetParams),
    SetBackground(SetBackgroundParams),
    SetBackgroundFit(SetBackgroundFitParams),
    AddFolder(AddFolderParams),
    RenameFolder(RenameFolderParams),
    MoveFolder(MoveFolderParams),
    RemoveFolder(FolderParams),
    OpenFolder(OpenFolderParams),
    CloseFolder(TargetParams),
    AddProfile(AddProfileParams),
    RenameProfile(RenameProfileParams),
    RemoveProfile(ProfileParams),
    SelectProfile(ProfileParams),
    SetProfileColor(SetProfileColorParams),
    SetFirstPage(PageTargetParams),
    SetDefaultProfile(ProfileParams),
    LinkApplication(LinkApplicationParams),
    UnlinkApplication(UnlinkApplicationParams),
    SetFollowFrontApplication(FollowParams),
    ExportProfile(ExportProfileParams),
    ImportProfile(ImportProfileParams),
    SetDeviceSettings(DeviceSettings),
    Undo(ProfileParams),
    Redo(ProfileParams),
    DeviceSettings(Empty),
    History(ProfileRead),
    Connection(Empty),
}

impl Method {
    /// The wire name of the method. A new variant must have a name here. The
    /// client of the command line uses it.
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Hello(_) => "hello",
            Self::State(_) => "state",
            Self::Layout(_) => "layout",
            Self::Kinds(_) => "kinds",
            Self::Choices(_) => "choices",
            Self::Profiles(_) => "profiles",
            Self::Pages(_) => "pages",
            Self::Folders(_) => "folders",
            Self::Content(_) => "content",
            Self::SetLabel(_) => "set_label",
            Self::SetLabelStyle(_) => "set_label_style",
            Self::SetColor(_) => "set_color",
            Self::SetImage(_) => "set_image",
            Self::SetIcon(_) => "set_icon",
            Self::SetBinding(_) => "set_binding",
            Self::CopyControl(_) => "copy_control",
            Self::PasteControl(_) => "paste_control",
            Self::ClearControl(_) => "clear_control",
            Self::CopyToAllPages(_) => "copy_to_all_pages",
            Self::AddPage(_) => "add_page",
            Self::DuplicatePage(_) => "duplicate_page",
            Self::RenamePage(_) => "rename_page",
            Self::MovePage(_) => "move_page",
            Self::RemovePage(_) => "remove_page",
            Self::SelectPage(_) => "select_page",
            Self::SetBackground(_) => "set_background",
            Self::SetBackgroundFit(_) => "set_background_fit",
            Self::AddFolder(_) => "add_folder",
            Self::RenameFolder(_) => "rename_folder",
            Self::MoveFolder(_) => "move_folder",
            Self::RemoveFolder(_) => "remove_folder",
            Self::OpenFolder(_) => "open_folder",
            Self::CloseFolder(_) => "close_folder",
            Self::AddProfile(_) => "add_profile",
            Self::RenameProfile(_) => "rename_profile",
            Self::RemoveProfile(_) => "remove_profile",
            Self::SelectProfile(_) => "select_profile",
            Self::SetProfileColor(_) => "set_profile_color",
            Self::SetFirstPage(_) => "set_first_page",
            Self::SetDefaultProfile(_) => "set_default_profile",
            Self::LinkApplication(_) => "link_application",
            Self::UnlinkApplication(_) => "unlink_application",
            Self::SetFollowFrontApplication(_) => "set_follow_front_application",
            Self::ExportProfile(_) => "export_profile",
            Self::ImportProfile(_) => "import_profile",
            Self::SetDeviceSettings(_) => "set_device_settings",
            Self::Undo(_) => "undo",
            Self::Redo(_) => "redo",
            Self::DeviceSettings(_) => "device_settings",
            Self::History(_) => "history",
            Self::Connection(_) => "connection",
        }
    }
}

/// The wire name of each method. A request with one of these names and
/// parameters that are not correct gets `invalid_request`, and a request with
/// any other name gets `unknown_method`.
pub const METHODS: &[&str] = &[
    "hello",
    "state",
    "layout",
    "kinds",
    "choices",
    "profiles",
    "pages",
    "folders",
    "content",
    "set_label",
    "set_label_style",
    "set_color",
    "set_image",
    "set_icon",
    "set_binding",
    "copy_control",
    "paste_control",
    "clear_control",
    "copy_to_all_pages",
    "add_page",
    "duplicate_page",
    "rename_page",
    "move_page",
    "remove_page",
    "select_page",
    "set_background",
    "set_background_fit",
    "add_folder",
    "rename_folder",
    "move_folder",
    "remove_folder",
    "open_folder",
    "close_folder",
    "add_profile",
    "rename_profile",
    "remove_profile",
    "select_profile",
    "set_profile_color",
    "set_first_page",
    "set_default_profile",
    "link_application",
    "unlink_application",
    "set_follow_front_application",
    "export_profile",
    "import_profile",
    "set_device_settings",
    "undo",
    "redo",
    "device_settings",
    "history",
    "connection",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

/// A read about a profile. No profile means the active profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRead {
    pub profile: Option<String>,
}

/// A later version can add a field. Another field is not an error here, so
/// that the version check comes first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloParams {
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoicesParams {
    pub action: Action,
    pub field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T = Value> {
    pub id: Option<u64>,
    #[serde(flatten)]
    pub body: ResponseBody<T>,
}

/// An error line has `error`, and a success line has `result`. `Failure` is
/// first: a result type that is an `Option` reads a missing `result` as
/// `None`, and an error line must not read as a success with no value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseBody<T> {
    Failure { error: ProtocolError },
    Success { result: T },
}

impl<T> Response<T> {
    /// The result, or the error of the server. The client of the command line
    /// uses it.
    pub fn into_result(self) -> Result<T, ProtocolError> {
        match self.body {
            ResponseBody::Success { result } => Ok(result),
            ResponseBody::Failure { error } => Err(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolError {
    pub code: ErrorCode,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    UnknownMethod,
    LineTooLong,
    HelloRequired,
    VersionMismatch,
    InvalidParams,
    Unavailable,
    Timeout,
    ResponseTooLarge,
    EngineProblem,
    /// All places for clients are in use.
    Busy,
    /// An undo with no step of the history.
    NothingToUndo,
    /// A redo with no undone step.
    NothingToRedo,
}

impl Response {
    pub fn success(id: u64, result: impl Serialize) -> Self {
        Self {
            id: Some(id),
            body: ResponseBody::Success {
                result: serde_json::to_value(result).expect("wire result is JSON data"),
            },
        }
    }

    pub fn error(id: Option<u64>, code: ErrorCode, text: &str) -> Self {
        Self {
            id,
            body: ResponseBody::Failure {
                error: ProtocolError {
                    code,
                    text: text.into(),
                },
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResult {
    pub version: u32,
    pub app_version: String,
}

/// The kind of a surface. The wire names are these names in snake case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    KeyFace,
    BarPanel,
    Background,
}

/// The kind of a control. The wire names are these names in snake case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    Key,
    Knob,
    BarPanel,
    TouchBar,
}

/// A gesture of a binding. The wire names are these names in snake case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gesture {
    Press,
    Release,
    TurnLeft,
    TurnRight,
    Push,
    Tap,
    SwipeLeft,
    SwipeRight,
}

/// Where the label of a surface is. `Hidden`: the text stays, but the surface
/// does not show it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelPosition {
    Top,
    Middle,
    Bottom,
    Hidden,
}

/// How a background image is on the display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundFit {
    Fill,
    Fit,
}

/// What the knob lights show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnobLightMode {
    Effect,
    Colors,
    Off,
}

/// A surface of the layout: a kind and the number of the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SurfaceId {
    pub kind: SurfaceKind,
    pub number: u8,
}

/// A control of the layout: a kind and the number of the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ControlId {
    pub kind: ControlKind,
    pub number: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub display_width: u16,
    pub display_height: u16,
    pub surfaces: Vec<SurfaceLayout>,
    pub controls: Vec<ControlLayout>,
    pub settings: SettingsSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceLayout {
    pub id: SurfaceId,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlLayout {
    pub id: ControlId,
    pub row: u8,
    pub column: u8,
    pub surface: Option<SurfaceId>,
    pub feedback_surface: Option<SurfaceId>,
    /// The gestures that a binding of this control can have.
    pub gestures: Vec<Gesture>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsSupport {
    pub lowest_brightness: u8,
    pub knob_lights: u8,
    pub haptic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub kind: String,
    #[serde(default)]
    pub settings: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kind {
    pub kind: String,
    pub name: String,
    pub group: String,
    pub help: Option<String>,
    pub icon: Option<String>,
    pub needs_accessibility: bool,
    pub fields: Vec<Field>,
    pub direction: Option<Direction>,
    pub states: Option<TwoStates>,
    pub folder_back: bool,
    pub shown_when: Vec<ShownWhen>,
    pub permission_when: Option<ShownWhen>,
    pub status: Option<Status>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub input: Input,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    Text,
    LongText,
    Integer { min: i64, max: i64 },
    Choice { options: Vec<Choice> },
    RuntimeChoice,
    RuntimeChoiceList,
    Profile,
    Colour,
    Toggle,
    Hotkey,
    Path { kind: PathKind },
    Action,
    ActionList,
    TimedActionList,
    Folder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PathKind {
    File,
    Application { bundle_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Direction {
    pub key: String,
    pub right: String,
    pub left: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoStates {
    pub on_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShownWhen {
    pub field: String,
    pub key: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Status {
    Ready,
    NeedsUser { text: String, button: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profiles {
    pub profiles: Vec<Profile>,
    pub active: Option<String>,
    pub default: Option<String>,
    pub cause: Cause,
    pub follow_front_application: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub pages: Vec<PageSummary>,
    /// The identity of the first page.
    pub first_page: String,
    pub applications: Vec<ApplicationLink>,
    pub applied: Vec<ApplicationLink>,
    pub link_owners: Vec<LinkOwner>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationLink {
    pub bundle_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkOwner {
    pub bundle_id: String,
    pub profile: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Cause {
    #[default]
    Selected,
    Linked {
        bundle_id: String,
    },
    Default,
}

/// The identity and the name of a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Location {
    /// The identity of the page under the open folders.
    pub page: String,
    pub open: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pages {
    /// The identity of the profile.
    pub profile: Option<String>,
    pub pages: Vec<PageSummary>,
    /// The active location. Null for a profile that is not active: it has
    /// no active location.
    pub location: Option<Location>,
    /// The identity of the first page.
    pub first: String,
    pub folders: Vec<Folder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Surface {
    pub id: SurfaceId,
    pub label: Option<String>,
    pub label_style: LabelStyle,
    pub color: Option<String>,
    /// A profile-relative image name, not an absolute file path.
    pub image: Option<String>,
    pub icon: Option<Icon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelStyle {
    pub position: Option<LabelPosition>,
    pub size: u8,
    pub color: Option<String>,
    pub bold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Background {
    Color { color: String },
    Image { image: String, fit: BackgroundFit },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    /// The identity of the page.
    pub page: String,
    pub folder: Option<String>,
    pub control: ControlId,
    pub gesture: Gesture,
    pub action: Action,
    pub available: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Content {
    pub profile: Option<String>,
    pub location: Location,
    pub background: Option<Background>,
    pub surfaces: Vec<Surface>,
    pub bindings: Vec<Binding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSettings {
    pub brightness: u8,
    pub screen_off_minutes: u16,
    pub haptic: bool,
    pub knob_lights: KnobLights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnobLights {
    pub mode: KnobLightMode,
    pub brightness: u8,
    pub colors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
    /// The identity of the profile of the history. Null only before the
    /// engine sends the first history.
    pub profile: Option<String>,
    pub undo: Option<String>,
    pub redo: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Connection {
    pub connected: bool,
    pub firmware: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub layout: Option<Layout>,
    pub kinds: Vec<Kind>,
    pub profiles: Profiles,
    pub pages: Pages,
    pub content: Content,
    pub device_settings: Option<DeviceSettings>,
    pub history: History,
    pub connection: Connection,
    /// The history of each profile that was active since the start. The
    /// history of a profile that is not active cannot change, thus the
    /// client can read it. It is not a part of `state`.
    #[serde(skip)]
    pub histories: Vec<History>,
}
