//! Wire data shared by the socket server and the `ctl` client.
//!
//! No engine command or event is a wire type. Page numbers start at zero;
//! control and surface numbers are those of the layout. Optional values
//! are JSON null. Each request, including a read with no arguments, has
//! an object in `params`.
//!
//! Read methods are `state` (all read state), `layout`, `kinds`, `profiles`,
//! `pages`, `folders`, `content`, `device_settings`, `history`, and
//! `connection`. They take `{}`. `content` also takes
//! `{ "target": <target> }`: then it reads that location, also of a profile
//! that is not active, and selects nothing. `choices` takes an action and a
//! field key. Its result is a list of choices; runtime choices arrive from
//! the handler, not a stale cache. `hello` takes `{ "version": 1 }` and must
//! precede other requests.
//!
//! Change methods use the names in [`METHODS`] and named parameters. A
//! target is `{ "profile", "page", "folder" }`: the profile identity, the
//! zero-based page, and a folder identity or null. A request with a target
//! makes that location the active location, and fails with no change if the
//! profile, the page, or the folder is not there. A request is one step of
//! the history. Null removes an optional part. A change returns `{}` after
//! the engine obeyed it. A problem of the engine is the error
//! `engine_problem` with its text. File inputs and export destinations must
//! be absolute paths.
//!
//! Invalid requests have an error, with a null id when no valid id was
//! readable. Oversized lines and version mismatches close the connection
//! after the error. Other request errors leave it open. Each connection
//! processes requests in order. A pending runtime choice has a five-second
//! limit. Requests are at most 1 MiB without the newline; responses are at
//! most 8 MiB without the newline. No stability is promised before 1.0.
//!
//! No result contains rendered image bytes, device identity, engine logs,
//! or absolute image destinations. The server replaces the current home
//! path with `~` in returned text, also within nested action settings.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub profile: String,
    pub page: usize,
    pub folder: Option<String>,
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
pub struct SetLabelParams {
    pub target: Target,
    pub surface: Id,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetLabelStyleParams {
    pub target: Target,
    pub surface: Id,
    pub style: Option<LabelStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetColorParams {
    pub target: Target,
    pub surface: Id,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetImageParams {
    pub target: Target,
    pub surface: Id,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetIconParams {
    pub target: Target,
    pub surface: Id,
    pub icon: Option<Icon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetBindingParams {
    pub target: Target,
    pub control: Id,
    pub gesture: String,
    pub action: Option<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlParams {
    pub target: Target,
    pub control: Id,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddPageParams {
    pub target: Target,
    pub copy_of: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenamePageParams {
    pub target: Target,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovePageParams {
    pub target: Target,
    pub to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetBackgroundParams {
    pub target: Target,
    pub background: Option<NewBackground>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetBackgroundFitParams {
    pub target: Target,
    pub fit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddFolderParams {
    pub target: Target,
    /// The identity of the new folder: 16 small hexadecimal digits. Without
    /// it, OpenCraft makes a new random identity.
    #[serde(default)]
    pub id: Option<String>,
    pub name: Option<String>,
    pub copy_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenameFolderParams {
    pub target: Target,
    pub folder: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveFolderParams {
    pub target: Target,
    pub folder: String,
    pub to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderParams {
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
pub struct SetFirstPageParams {
    pub profile: String,
    pub page: usize,
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
    Image { path: String, fit: String },
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
    Pages(Empty),
    Folders(Empty),
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
    AddPage(AddPageParams),
    RenamePage(RenamePageParams),
    MovePage(MovePageParams),
    RemovePage(TargetParams),
    SelectPage(TargetParams),
    SetBackground(SetBackgroundParams),
    SetBackgroundFit(SetBackgroundFitParams),
    AddFolder(AddFolderParams),
    RenameFolder(RenameFolderParams),
    MoveFolder(MoveFolderParams),
    RemoveFolder(FolderParams),
    OpenFolder(FolderParams),
    CloseFolder(TargetParams),
    AddProfile(AddProfileParams),
    RenameProfile(RenameProfileParams),
    RemoveProfile(ProfileParams),
    SelectProfile(ProfileParams),
    SetProfileColor(SetProfileColorParams),
    SetFirstPage(SetFirstPageParams),
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
    History(Empty),
    Connection(Empty),
}

impl Method {
    /// The wire name of the method. A new variant must have a name here.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseBody<T> {
    Success { result: T },
    Failure { error: ProtocolError },
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id {
    pub kind: String,
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
    pub id: Id,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlLayout {
    pub id: Id,
    pub row: u8,
    pub column: u8,
    pub surface: Option<Id>,
    pub feedback_surface: Option<Id>,
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
    pub pages: Vec<String>,
    pub first_page: usize,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Location {
    pub page: usize,
    pub open: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pages {
    pub names: Vec<String>,
    pub location: Location,
    pub first: usize,
    pub folders: Vec<Folder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Surface {
    pub id: Id,
    pub label: Option<String>,
    pub label_style: LabelStyle,
    pub color: Option<String>,
    /// A profile-relative image name, not an absolute file path.
    pub image: Option<String>,
    pub icon: Option<Icon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelStyle {
    pub position: Option<String>,
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
    Image { image: String, fit: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub page: usize,
    pub folder: Option<String>,
    pub control: Id,
    pub gesture: String,
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
    pub mode: String,
    pub brightness: u8,
    pub colors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
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
}
