//! The one list of the commands of `opencraft ctl` (decision 0040, section 4).
//!
//! The parser, the help text, and the tests read this list. A test of the
//! skill reads it too: each command has a name, a help text, and an example.
//! Each request of the control protocol (`METHODS`) has exactly one command,
//! except `hello`: the client sends `hello` by itself.
//!
//! An argument has a flag, a place in the `params` of the request (a path of
//! names, with a point between the names), and a kind of value. A value that
//! the user does not give is not in the request. For a request that sets a
//! value or removes it, no value means remove. The request types of
//! `control/protocol.rs` check the result: the client has no copy of them.
//!
//! Each command has a [`Class`]: the MCP server makes one tool for each class,
//! and this field is the only place that says which tool holds the command.
//! The macros `read!`, `change!`, and `remove!` are the only ways to make a
//! command, thus no command has a class by accident.

use serde_json::{Map, Value};

/// The kind of the value of an argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    /// A whole number.
    Int,
    /// `true` or `false`.
    Bool,
    /// JSON text.
    Json,
    /// A file path. A relative path is relative to the current directory.
    Path,
    /// A surface of the layout, as `kind:number`, for example `key_face:3`.
    Surface,
    /// A control of the layout, as `kind:number`, for example `knob:1`.
    Control,
    /// Text values with a comma between them.
    List,
    /// `KEY=VALUE`, can be given more than once. The value is JSON if it is
    /// JSON, and text if it is not.
    Setting,
}

pub struct Arg {
    pub flag: &'static str,
    pub path: &'static str,
    pub kind: Kind,
    pub required: bool,
    /// The value that the request has when its parent object is in the
    /// request and this argument is not.
    pub fill: Option<&'static str>,
    pub help: &'static str,
}

/// A change to the parameters, after the arguments are in them.
pub type Finish = fn(&mut Map<String, Value>) -> Result<(), String>;

/// What a command does to the data of OpenCraft. The MCP server makes one tool
/// for each class (decision 0040, section 6): `opencraft_read`,
/// `opencraft_change`, and `opencraft_remove`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// A command that only reads. It changes nothing.
    Read,
    /// A command that adds or changes data: an additive update.
    Change,
    /// A command whose purpose is to remove data, or to write a file in place
    /// of the file that is there.
    Remove,
}

pub struct Command {
    /// One or two words, for example `profile add`.
    pub name: &'static str,
    /// What the command does. It is the tool of the MCP server that holds it.
    pub class: Class,
    /// The wire name of the request.
    pub method: &'static str,
    pub help: &'static str,
    /// A whole command line that is correct.
    pub example: &'static str,
    /// Groups of arguments. The groups are shared between commands.
    pub args: &'static [&'static [Arg]],
    /// The name of a read request. Its result is the start of the parameters:
    /// the arguments change only the fields that the user gives.
    pub base: Option<&'static str>,
    pub finish: Option<Finish>,
}

impl Command {
    pub fn arguments(&self) -> impl Iterator<Item = &'static Arg> {
        self.args.iter().flat_map(|group| group.iter())
    }
}

const fn arg(flag: &'static str, path: &'static str, kind: Kind, help: &'static str) -> Arg {
    Arg {
        flag,
        path,
        kind,
        required: false,
        fill: None,
        help,
    }
}

const fn req(flag: &'static str, path: &'static str, kind: Kind, help: &'static str) -> Arg {
    Arg {
        required: true,
        ..arg(flag, path, kind, help)
    }
}

const fn fill(
    flag: &'static str,
    path: &'static str,
    kind: Kind,
    default: &'static str,
    help: &'static str,
) -> Arg {
    Arg {
        fill: Some(default),
        ..arg(flag, path, kind, help)
    }
}

const PROFILE: &[Arg] = &[req(
    "profile",
    "profile",
    Kind::Text,
    "The identity of the profile (see `profile list`).",
)];

/// A read about one profile. No profile means the active profile.
const OPTIONAL_PROFILE: &[Arg] = &[arg(
    "profile",
    "profile",
    Kind::Text,
    "The identity of the profile (see `profile list`). Default: the active profile.",
)];

const PAGE_TARGET: &[Arg] = &[
    req(
        "profile",
        "target.profile",
        Kind::Text,
        "The identity of the profile (see `profile list`).",
    ),
    req(
        "page",
        "target.page",
        Kind::Text,
        "The identity of the page: 16 hexadecimal digits (see `page list`).",
    ),
];

const TARGET: &[Arg] = &[
    req(
        "profile",
        "target.profile",
        Kind::Text,
        "The identity of the profile (see `profile list`).",
    ),
    req(
        "page",
        "target.page",
        Kind::Text,
        "The identity of the page: 16 hexadecimal digits (see `page list`).",
    ),
    arg(
        "folder",
        "target.folder",
        Kind::Text,
        "The identity of the folder, if the location is inside a folder.",
    ),
];

/// The target of a close: the open folder is a part of the location.
const CLOSE_TARGET: &[Arg] = &[
    req(
        "profile",
        "target.profile",
        Kind::Text,
        "The identity of the profile (see `profile list`).",
    ),
    req(
        "page",
        "target.page",
        Kind::Text,
        "The identity of the page: 16 hexadecimal digits (see `page list`).",
    ),
    req(
        "folder",
        "target.folder",
        Kind::Text,
        "The identity of the open folder (see `folder list`).",
    ),
];

/// `open folder` has its own flag for the folder to open.
const OPEN_TARGET: &[Arg] = &[
    req(
        "profile",
        "target.profile",
        Kind::Text,
        "The identity of the profile.",
    ),
    req(
        "page",
        "target.page",
        Kind::Text,
        "The identity of the page.",
    ),
    arg(
        "inside",
        "target.folder",
        Kind::Text,
        "The identity of the open folder in which the folder opens, if there is one.",
    ),
    req(
        "folder",
        "folder",
        Kind::Text,
        "The identity of the folder to open.",
    ),
];

const FOLDER: &[Arg] = &[
    req(
        "profile",
        "profile",
        Kind::Text,
        "The identity of the profile.",
    ),
    req(
        "folder",
        "folder",
        Kind::Text,
        "The identity of the folder (see `folder list`).",
    ),
];

const SURFACE: &[Arg] = &[
    req(
        "surface",
        "surface",
        Kind::Surface,
        "The surface, as `kind:number`: `key_face`, `bar_panel`, or `background`. See `layout`.",
    ),
    arg(
        "on",
        "on",
        Kind::Bool,
        "`true` to change the On face, or `false` for the base face.",
    ),
];

const CONTROL: &[Arg] = &[req(
    "control",
    "control",
    Kind::Control,
    "The control, as `kind:number`: `key`, `knob`, `bar_panel`, or `touch_bar`. See `layout`.",
)];

const GESTURE: &[Arg] = &[req(
    "gesture",
    "gesture",
    Kind::Text,
    "The gesture, for example `press`, `long_press`, `double_press`, or `turn_left`. `layout` lists the gestures of each control.",
)];

const ACTION: &[Arg] = &[
    arg(
        "action",
        "action",
        Kind::Json,
        "The action as JSON: {\"kind\": \"...\", \"settings\": {...}}. See `kinds`.",
    ),
    arg(
        "action-kind",
        "action.kind",
        Kind::Text,
        "The kind of the action, in full as `kinds` gives it, for example `opencraft.hotkey`. Use it in place of --action.",
    ),
    arg(
        "setting",
        "action.settings",
        Kind::Setting,
        "A setting of the action, as KEY=VALUE. Give a text that is a number, `true`, `false`, `null`, or starts with a quotation mark, a bracket, or a brace as a JSON string. Can be given more than once.",
    ),
];

const CHOICE_ACTION: &[Arg] = &[
    req(
        "action-kind",
        "action.kind",
        Kind::Text,
        "The kind of the action, in full as `kinds` gives it, for example `opencraft.hotkey`.",
    ),
    arg(
        "setting",
        "action.settings",
        Kind::Setting,
        "A setting of the action, as KEY=VALUE. Give a text that is a number, `true`, `false`, `null`, or starts with a quotation mark, a bracket, or a brace as a JSON string. Can be given more than once.",
    ),
    req(
        "field",
        "field",
        Kind::Text,
        "The key of the field whose choices to read. See `kinds`.",
    ),
];

const NAME: &[Arg] = &[req("name", "name", Kind::Text, "The new name.")];
const TO: &[Arg] = &[req(
    "to",
    "to",
    Kind::Int,
    "The number of the place in the list, from 1: 1 is the first.",
)];
const COLOR: &[Arg] = &[arg(
    "color",
    "color",
    Kind::Text,
    "A colour, for example `#ff8800`. Leave it out to remove the colour.",
)];

const FOLDER_NAME: &[Arg] = &[
    arg(
        "name",
        "name",
        Kind::Text,
        "The name. Leave it out for the default name.",
    ),
    arg(
        "copy-of",
        "copy_of",
        Kind::Text,
        "The identity of a folder to copy. Leave it out for an empty folder.",
    ),
];

const NO_ARGS: &[&[Arg]] = &[];

macro_rules! command {
    ($class:expr, $name:literal, $method:literal, $help:literal, $example:literal, $args:expr) => {
        Command {
            name: $name,
            class: $class,
            method: $method,
            help: $help,
            example: $example,
            args: $args,
            base: None,
            finish: None,
        }
    };
}

/// A command that only reads.
macro_rules! read {
    ($name:literal, $method:literal, $help:literal, $example:literal, $args:expr) => {
        command!(Class::Read, $name, $method, $help, $example, $args)
    };
}

/// A command that adds or changes data.
macro_rules! change {
    ($name:literal, $method:literal, $help:literal, $example:literal, $args:expr) => {
        command!(Class::Change, $name, $method, $help, $example, $args)
    };
}

/// A command that removes data, or that writes a file in place of a file.
macro_rules! remove {
    ($name:literal, $method:literal, $help:literal, $example:literal, $args:expr) => {
        command!(Class::Remove, $name, $method, $help, $example, $args)
    };
}

/// Each command of the client.
pub const COMMANDS: &[Command] = &[
    // The reads.
    read!(
        "state",
        "state",
        "Read all state in one result: the layout, the kinds, the profiles, the pages, the content of the active page, the device settings, the history, and the connection.",
        "opencraft ctl state",
        NO_ARGS
    ),
    read!(
        "layout",
        "layout",
        "Read the layout of the device model: the surfaces, the controls, and the gestures of each control.",
        "opencraft ctl layout",
        NO_ARGS
    ),
    read!(
        "kinds",
        "kinds",
        "Read the kinds of action, with their fields, their help, and their status.",
        "opencraft ctl kinds",
        NO_ARGS
    ),
    read!(
        "icons",
        "icons",
        "Read the names of the icons for a surface: the set `Lucide`, its version, and the name of each icon. A search gives the icons whose name or tag agrees with the text, the best first.",
        "opencraft ctl icons --search mic",
        &[&[arg(
            "search",
            "search",
            Kind::Text,
            "Text that the name or a tag of an icon has, for example `mic` or `arrow right`. Leave it out for each icon."
        )]]
    ),
    read!(
        "choices",
        "choices",
        "Read the choices of one field of an action, for example the sound devices.",
        "opencraft ctl choices --action-kind opencraft.play-sound --field device",
        &[CHOICE_ACTION]
    ),
    read!(
        "content",
        "content",
        "Read the surfaces and the bindings of a location. With no argument: the active location. A location that is not active is read, and not selected.",
        "opencraft ctl content --profile work --page 0123456789abcdef",
        &[&[
            arg("profile", "target.profile", Kind::Text, "The identity of the profile."),
            arg("page", "target.page", Kind::Text, "The identity of the page."),
            arg("folder", "target.folder", Kind::Text, "The identity of the folder, if there is one."),
        ]]
    ),
    read!(
        "device settings",
        "device_settings",
        "Read the settings of the device: the brightness, the screen-off time, the haptic feedback, and the knob lights.",
        "opencraft ctl device settings",
        NO_ARGS
    ),
    read!(
        "history",
        "history",
        "Read the names of the next undo and the next redo. With no `--profile`: the active profile. A profile that is not active keeps its history: it cannot change while the profile is not active.",
        "opencraft ctl history",
        &[OPTIONAL_PROFILE]
    ),
    read!(
        "connection",
        "connection",
        "Read the state of the device connection: connected or not, and the firmware.",
        "opencraft ctl connection",
        NO_ARGS
    ),
    // Profiles.
    read!(
        "profile list",
        "profiles",
        "Read the profiles with their pages, their application links, and which profile is active and default.",
        "opencraft ctl profile list",
        NO_ARGS
    ),
    change!(
        "profile add",
        "add_profile",
        "Add a profile, empty or as a copy of a profile. The result has the identity of the new profile and of its first page.",
        "opencraft ctl profile add --name Work",
        &[&[
            req("name", "name", Kind::Text, "The name of the new profile."),
            arg("copy-of", "copy_of", Kind::Text, "The identity of a profile to copy."),
        ]]
    ),
    change!(
        "profile rename",
        "rename_profile",
        "Rename a profile.",
        "opencraft ctl profile rename --profile work --name Office",
        &[PROFILE, NAME]
    ),
    remove!(
        "profile remove",
        "remove_profile",
        "Remove a profile with all its pages. There is no undo: OpenCraft moves the profile to the folder `removed` in the profiles folder.",
        "opencraft ctl profile remove --profile work",
        &[PROFILE]
    ),
    change!(
        "profile select",
        "select_profile",
        "Make a profile the active profile. The device then shows it.",
        "opencraft ctl profile select --profile work",
        &[PROFILE]
    ),
    change!(
        "profile set-color",
        "set_profile_color",
        "Set the colour of a profile, or remove it.",
        "opencraft ctl profile set-color --profile work --color '#ff8800'",
        &[PROFILE, COLOR]
    ),
    change!(
        "profile set-title-style",
        "set_profile_default_label_style",
        "Set the default title style of a profile. Leave out all values to make each part the engine default.",
        "opencraft ctl profile set-title-style --profile work --font 'Avenir Next' --size 24 --bold false",
        &[
            PROFILE,
            &[
                arg("font", "style.font", Kind::Text, "A family of an installed font, or `system`. Leave it out for the system font."),
                arg("position", "style.position", Kind::Text, "`auto`, `top`, `middle`, `bottom`, or `hidden`. Leave it out for the automatic position."),
                arg("size", "style.size", Kind::Int, "The size of the font, in pixels, from 10 to 80. Leave it out for 20."),
                arg("color", "style.color", Kind::Text, "A colour as `#RRGGBB`, or `auto`. Leave it out for the automatic colour."),
                arg("bold", "style.bold", Kind::Bool, "`true` or `false`. Leave it out for bold."),
                arg("italic", "style.italic", Kind::Bool, "`true` or `false`. Leave it out for upright."),
                arg("underline", "style.underline", Kind::Bool, "`true` or `false`. Leave it out for no underline."),
            ]
        ]
    ),
    change!(
        "profile set-default",
        "set_default_profile",
        "Make a profile the default profile.",
        "opencraft ctl profile set-default --profile work",
        &[PROFILE]
    ),
    change!(
        "profile link-app",
        "link_application",
        "Link an application to a profile. The profile then becomes active when the application is in front, if the automatic switch is on.",
        "opencraft ctl profile link-app --profile work --bundle-id com.apple.Safari --name Safari",
        &[
            PROFILE,
            &[
                req("bundle-id", "link.bundle_id", Kind::Text, "The bundle identifier of the application."),
                req("name", "link.name", Kind::Text, "The name of the application."),
            ]
        ]
    ),
    remove!(
        "profile unlink-app",
        "unlink_application",
        "Remove the link of an application from a profile.",
        "opencraft ctl profile unlink-app --profile work --bundle-id com.apple.Safari",
        &[
            PROFILE,
            &[req("bundle-id", "bundle_id", Kind::Text, "The bundle identifier of the application.")]
        ]
    ),
    change!(
        "profile follow",
        "set_follow_front_application",
        "Turn the automatic switch of the profile to the front application on or off.",
        "opencraft ctl profile follow --follow true",
        &[&[req("follow", "follow", Kind::Bool, "`true` or `false`.")]]
    ),
    remove!(
        "profile export",
        "export_profile",
        "Write a profile with its images to a file.",
        "opencraft ctl profile export --profile work --path /tmp/work.opencraft",
        &[
            PROFILE,
            &[req("path", "path", Kind::Path, "The file to write.")]
        ]
    ),
    change!(
        "profile import",
        "import_profile",
        "Read a profile from a file. The result has the identity of the new profile and of its first page.",
        "opencraft ctl profile import --path /tmp/work.opencraft",
        &[&[req("path", "path", Kind::Path, "The file to read.")]]
    ),
    // Pages.
    read!(
        "page list",
        "pages",
        "Read the pages of a profile with their identities, its folders, and the first page. With no `--profile`: the active profile, with its active location. A profile that is not active has no active location (`location` is null).",
        "opencraft ctl page list --profile work",
        &[OPTIONAL_PROFILE]
    ),
    change!(
        "page add",
        "add_page",
        "Add an empty page at the end of a profile. The result has the identity of the new page.",
        "opencraft ctl page add --profile work",
        &[PROFILE]
    ),
    change!(
        "page duplicate",
        "duplicate_page",
        "Add a copy of a page after it. The result has the identity of the new page.",
        "opencraft ctl page duplicate --profile work --page 0123456789abcdef",
        &[PAGE_TARGET]
    ),
    change!(
        "page rename",
        "rename_page",
        "Rename a page.",
        "opencraft ctl page rename --profile work --page 0123456789abcdef --name Music",
        &[PAGE_TARGET, NAME]
    ),
    change!(
        "page move",
        "move_page",
        "Move a page to a place in the list of the pages. Page 1 is the first page.",
        "opencraft ctl page move --profile work --page 0123456789abcdef --to 1",
        &[PAGE_TARGET, TO]
    ),
    remove!(
        "page remove",
        "remove_page",
        "Remove a page.",
        "opencraft ctl page remove --profile work --page 0123456789abcdef",
        &[PAGE_TARGET]
    ),
    change!(
        "page select",
        "select_page",
        "Make a page the active page. The device then shows it.",
        "opencraft ctl page select --profile work --page 0123456789abcdef",
        &[PAGE_TARGET]
    ),
    change!(
        "page set-first",
        "set_first_page",
        "Make a page the first page of its profile: the page that the device shows when the profile becomes active.",
        "opencraft ctl page set-first --profile work --page 0123456789abcdef",
        &[PAGE_TARGET]
    ),
    Command {
        finish: Some(finish_background),
        ..change!(
            "page set-background",
            "set_background",
            "Set the background of a page: a colour or an image. Leave out all values to remove the background.",
            "opencraft ctl page set-background --profile work --page 0123456789abcdef --color '#102030'",
            &[
                PAGE_TARGET,
                &[
                    arg("color", "background.color", Kind::Text, "A colour, for example `#102030`."),
                    arg("image", "background.path", Kind::Path, "An image file."),
                    arg("fit", "background.fit", Kind::Text, "`fill` or `fit`, for an image. The default is `fill`."),
                ]
            ]
        )
    },
    change!(
        "page set-background-fit",
        "set_background_fit",
        "Set how the background image of a page fits the display.",
        "opencraft ctl page set-background-fit --profile work --page 0123456789abcdef --fit fit",
        &[
            PAGE_TARGET,
            &[req("fit", "fit", Kind::Text, "`fill` or `fit`.")]
        ]
    ),
    // Folders.
    read!(
        "folder list",
        "folders",
        "Read the folders of a profile. With no `--profile`: the active profile.",
        "opencraft ctl folder list --profile work",
        &[OPTIONAL_PROFILE]
    ),
    change!(
        "folder add",
        "add_folder",
        "Add a folder to a profile, empty or as a copy of a folder. The result has the identity of the new folder.",
        "opencraft ctl folder add --profile work --name Tools",
        &[
            PROFILE,
            FOLDER_NAME,
            &[arg("id", "id", Kind::Text, "The identity of the new folder: 16 hexadecimal digits. Leave it out for a new identity.")]
        ]
    ),
    change!(
        "folder rename",
        "rename_folder",
        "Rename a folder.",
        "opencraft ctl folder rename --profile work --folder 0123456789abcdef --name Tools",
        &[FOLDER, NAME]
    ),
    change!(
        "folder move",
        "move_folder",
        "Move a folder to a place in the list of the folders. Folder 1 is the first folder.",
        "opencraft ctl folder move --profile work --folder 0123456789abcdef --to 1",
        &[FOLDER, TO]
    ),
    remove!(
        "folder remove",
        "remove_folder",
        "Remove a folder.",
        "opencraft ctl folder remove --profile work --folder 0123456789abcdef",
        &[FOLDER]
    ),
    change!(
        "folder open",
        "open_folder",
        "Open a folder on a page. The device then shows the folder.",
        "opencraft ctl folder open --profile work --page 0123456789abcdef --folder fedcba9876543210",
        &[OPEN_TARGET]
    ),
    change!(
        "folder close",
        "close_folder",
        "Close the folder that is open at the location. The folder must be the last open folder of the active location. The device then shows the page or the folder below it.",
        "opencraft ctl folder close --profile work --page 0123456789abcdef --folder fedcba9876543210",
        &[CLOSE_TARGET]
    ),
    // Surfaces.
    change!(
        "surface set-label",
        "set_label",
        "Set the text of a surface, or remove it.",
        "opencraft ctl surface set-label --profile work --page 0123456789abcdef --surface key_face:1 --label Mute",
        &[
            TARGET,
            SURFACE,
            &[arg("label", "label", Kind::Text, "The text. Leave it out to remove the text.")]
        ]
    ),
    change!(
        "surface set-label-style",
        "set_label_style",
        "Set the style of the text of a surface. Leave out all values for the default style.",
        "opencraft ctl surface set-label-style --profile work --page 0123456789abcdef --surface key_face:1 --position top --size 16",
        &[
            TARGET,
            SURFACE,
            &[
                arg("font", "style.font", Kind::Text, "A family of an installed font, or `system`. Leave it out to inherit the default title style."),
                arg("position", "style.position", Kind::Text, "`top`, `middle`, `bottom`, or `hidden`. Leave it out for the automatic position."),
                fill("size", "style.size", Kind::Int, "20", "The size of the font, in pixels. The default is 20."),
                arg("color", "style.color", Kind::Text, "A colour. Leave it out for black or white."),
                fill("bold", "style.bold", Kind::Bool, "true", "`true` or `false`. The default is `true`."),
                fill("italic", "style.italic", Kind::Bool, "false", "`true` or `false`. The default is `false`."),
                fill("underline", "style.underline", Kind::Bool, "false", "`true` or `false`. The default is `false`."),
            ]
        ]
    ),
    change!(
        "surface set-color",
        "set_color",
        "Set the fill colour of a surface, or remove it.",
        "opencraft ctl surface set-color --profile work --page 0123456789abcdef --surface key_face:1 --color '#336699'",
        &[TARGET, SURFACE, COLOR]
    ),
    change!(
        "surface set-image",
        "set_image",
        "Set the image of a surface from a file, or remove it.",
        "opencraft ctl surface set-image --profile work --page 0123456789abcdef --surface key_face:1 --path /tmp/key.png",
        &[
            TARGET,
            SURFACE,
            &[arg("path", "path", Kind::Path, "The image file. Leave it out to remove the image.")]
        ]
    ),
    change!(
        "surface set-icon",
        "set_icon",
        "Set the icon of a surface, or remove it. The name is a name of the icons that `icons` reads, for example `mic`.",
        "opencraft ctl surface set-icon --profile work --page 0123456789abcdef --surface key_face:1 --icon-name mic",
        &[
            TARGET,
            SURFACE,
            &[
                arg("icon-name", "icon.name", Kind::Text, "The name of an icon (see `icons`). Leave it out to remove the icon."),
                arg("icon-color", "icon.color", Kind::Text, "The colour of the icon, as six hexadecimal digits with `#` in front: `#ff8800`. Leave it out for the colour of the text."),
            ]
        ]
    ),
    change!(
        "surface set-image-crop",
        "set_image_crop",
        "Zoom and move the image of a surface in its area, or reset the crop. Leave out each value to reset the crop.",
        "opencraft ctl surface set-image-crop --profile work --page 0123456789abcdef --surface key_face:1 --zoom 200 --offset-x -500",
        &[
            TARGET,
            SURFACE,
            &[
                arg("zoom", "zoom", Kind::Int, "The zoom in percent, from 100 to 400. 100 is the fit. Leave it out for 100."),
                arg("offset-x", "offset_x", Kind::Int, "The pan on the x axis, in per mille of the overflow, from -1000 to 1000. A positive value shows more of the left of the image. Leave it out for 0."),
                arg("offset-y", "offset_y", Kind::Int, "The pan on the y axis, in per mille of the overflow, from -1000 to 1000. A positive value shows more of the top of the image. Leave it out for 0."),
            ]
        ]
    ),
    // Bindings and controls.
    change!(
        "binding set",
        "set_binding",
        "Set the action of a gesture of a control, or remove it. A person must push the key: a client cannot run an action.",
        "opencraft ctl binding set --profile work --page 0123456789abcdef --control key:1 --gesture press --action-kind opencraft.hotkey --setting hotkey=cmd+c",
        &[TARGET, CONTROL, GESTURE, ACTION]
    ),
    change!(
        "control copy",
        "copy_control",
        "Copy a control with its surface and its bindings.",
        "opencraft ctl control copy --profile work --page 0123456789abcdef --control key:1",
        &[TARGET, CONTROL]
    ),
    change!(
        "control paste",
        "paste_control",
        "Paste the copied control onto a control.",
        "opencraft ctl control paste --profile work --page 0123456789abcdef --control key:2",
        &[TARGET, CONTROL]
    ),
    remove!(
        "control clear",
        "clear_control",
        "Remove the surface and the bindings of a control.",
        "opencraft ctl control clear --profile work --page 0123456789abcdef --control key:2",
        &[TARGET, CONTROL]
    ),
    change!(
        "control copy-to-all-pages",
        "copy_to_all_pages",
        "Copy a control to the same place on all other pages of the profile.",
        "opencraft ctl control copy-to-all-pages --profile work --page 0123456789abcdef --control key:1",
        &[TARGET, CONTROL]
    ),
    change!(
        "control move",
        "move_control",
        "Move a control onto another control of the same kind: it moves to an empty control, or swaps with an occupied control. One undo restores both.",
        "opencraft ctl control move --profile work --page 0123456789abcdef --from key:1 --to key:2",
        &[
            TARGET,
            &[req(
                "from",
                "from",
                Kind::Control,
                "The control that moves, as `kind:number`, for example `key:1`."
            )],
            &[req(
                "to",
                "to",
                Kind::Control,
                "The destination control of the same kind, as `kind:number`."
            )],
        ]
    ),
    // The device.
    Command {
        base: Some("device_settings"),
        ..change!(
            "device set-settings",
            "set_device_settings",
            "Set the settings of the device. A value that you leave out keeps its value.",
            "opencraft ctl device set-settings --brightness 60 --knob-mode off",
            &[&[
                arg("brightness", "brightness", Kind::Int, "The brightness, in percent."),
                arg("screen-off-minutes", "screen_off_minutes", Kind::Int, "Minutes until the screen turns off. 0 means never."),
                arg("haptic", "haptic", Kind::Bool, "`true` or `false`."),
                arg("knob-mode", "knob_lights.mode", Kind::Text, "`effect`, `colors`, or `off`."),
                arg("knob-brightness", "knob_lights.brightness", Kind::Int, "The brightness of the knob lights."),
                arg("knob-colors", "knob_lights.colors", Kind::List, "The colours of the knob lights, with a comma between them."),
            ]]
        )
    },
    // The history.
    change!(
        "undo",
        "undo",
        "Undo the last change of a profile. The profile must be the active profile. The result gives the place of the reversed change. With no step, the error is `nothing_to_undo`.",
        "opencraft ctl undo --profile work",
        &[PROFILE]
    ),
    change!(
        "redo",
        "redo",
        "Redo the last undone change of a profile. The profile must be the active profile. The result gives the place of the change. With no undone change, the error is `nothing_to_redo`.",
        "opencraft ctl redo --profile work",
        &[PROFILE]
    ),
];

/// The type of a background is in `background.color` or in `background.path`.
fn finish_background(params: &mut Map<String, Value>) -> Result<(), String> {
    let Some(Value::Object(background)) = params.get_mut("background") else {
        return Ok(());
    };
    match (
        background.contains_key("color"),
        background.contains_key("path"),
    ) {
        (true, true) => Err("Give --color or --image, not both.".into()),
        (true, false) => {
            if background.contains_key("fit") {
                return Err("--fit is for an image.".into());
            }
            background.insert("type".into(), "color".into());
            Ok(())
        }
        (false, true) => {
            background.insert("type".into(), "image".into());
            background.entry("fit").or_insert_with(|| "fill".into());
            Ok(())
        }
        (false, false) => Err("Give --color or --image with --fit.".into()),
    }
}

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.name == name)
}
