//! Wire data shared by the socket server and the `ctl` client.
//!
//! No engine command or event is a wire type. Page numbers start at zero;
//! control and surface numbers are those of the layout. Optional values
//! are JSON null. Each request, including a read with no arguments, has
//! an object in `params`.
//!
//! Read methods are `state` (all read state), `layout`, `kinds`, `profiles`,
//! `pages`, `folders`, `content` (appearance and bindings at the active
//! location), `device_settings`, `history`, and `connection`. They take
//! `{}`. `choices` takes an action and a field key. Its result is a list
//! of choices; runtime choices arrive from the handler, not a stale cache.
//! `hello` takes `{ "version": 1 }` and must precede other requests.
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
    Content(Empty),
    DeviceSettings(Empty),
    History(Empty),
    Connection(Empty),
}

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
