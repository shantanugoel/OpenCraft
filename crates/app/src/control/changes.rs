//! Typed wire requests become only the commands allowed by decision 0040.
//!
//! A request that names a place becomes one `Command::AtPlace`. The engine
//! checks the place, goes there, and obeys the change as a step of the
//! history, in one command. A request about a profile only becomes the
//! command of the profile, or `AtPlace` when it changes the content of the
//! profile (a page, a folder).

use super::{protocol as w, state::permitted};
use opencraft_engine as e;
use std::path::PathBuf;

/// A command for the engine, and what the response of the request has.
pub(super) struct Change {
    pub command: e::Command,
    pub made: Made,
    /// The place that the request names. The response has it, and `made`
    /// adds the identity of a new thing.
    pub named: w::Changed,
}

/// The command and the new identity, before the place is added.
struct Built {
    command: e::Command,
    made: Made,
}

/// The identity that a request makes. The worker reads it from the cache,
/// on the engine thread, right after the engine obeyed the command: nothing
/// can change the active profile or the active page in between.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Made {
    Nothing,
    /// The new profile is the active profile, with its first page.
    Profile,
    /// The new page is the active page.
    Page,
    /// The active location after the change: the place of the step that an
    /// undo or a redo reversed.
    Place,
    /// The folder that the control module named.
    Folder(String),
}

fn parse<T: std::str::FromStr>(text: String, what: &str) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("The {what} is not correct."))
}

pub(super) fn path(text: String) -> Result<PathBuf, String> {
    let path = PathBuf::from(text);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err("A file path must be absolute.".into())
    }
}

impl From<w::SurfaceKind> for e::SurfaceKind {
    fn from(kind: w::SurfaceKind) -> Self {
        match kind {
            w::SurfaceKind::KeyFace => Self::KeyFace,
            w::SurfaceKind::BarPanel => Self::BarPanel,
            w::SurfaceKind::Background => Self::Background,
        }
    }
}

impl From<w::ControlKind> for e::ControlKind {
    fn from(kind: w::ControlKind) -> Self {
        match kind {
            w::ControlKind::Key => Self::Key,
            w::ControlKind::Knob => Self::Knob,
            w::ControlKind::BarPanel => Self::BarPanel,
            w::ControlKind::TouchBar => Self::TouchBar,
        }
    }
}

impl From<w::Gesture> for e::GestureKind {
    fn from(gesture: w::Gesture) -> Self {
        match gesture {
            w::Gesture::Press => Self::Press,
            w::Gesture::Release => Self::Release,
            w::Gesture::TurnLeft => Self::TurnLeft,
            w::Gesture::TurnRight => Self::TurnRight,
            w::Gesture::Push => Self::Push,
            w::Gesture::Tap => Self::Tap,
            w::Gesture::SwipeLeft => Self::SwipeLeft,
            w::Gesture::SwipeRight => Self::SwipeRight,
        }
    }
}

impl From<w::LabelPosition> for e::LabelPosition {
    fn from(position: w::LabelPosition) -> Self {
        match position {
            w::LabelPosition::Top => Self::Top,
            w::LabelPosition::Middle => Self::Middle,
            w::LabelPosition::Bottom => Self::Bottom,
            w::LabelPosition::Hidden => Self::Hidden,
        }
    }
}

impl From<w::BackgroundFit> for e::BackgroundFit {
    fn from(fit: w::BackgroundFit) -> Self {
        match fit {
            w::BackgroundFit::Fill => Self::Fill,
            w::BackgroundFit::Fit => Self::Fit,
        }
    }
}

impl From<w::KnobLightMode> for e::KnobLightMode {
    fn from(mode: w::KnobLightMode) -> Self {
        match mode {
            w::KnobLightMode::Effect => Self::Effect,
            w::KnobLightMode::Colors => Self::Colors,
            w::KnobLightMode::Off => Self::Off,
        }
    }
}

fn surface(id: w::SurfaceId, state: &w::State) -> Result<e::SurfaceId, String> {
    let layout = state
        .layout
        .as_ref()
        .ok_or("The layout is not available.")?;
    if !layout.surfaces.iter().any(|s| s.id == id) {
        return Err("The surface does not exist.".into());
    }
    Ok(e::SurfaceId::new(id.kind.into(), id.number))
}

fn control(id: w::ControlId, state: &w::State) -> Result<e::ControlId, String> {
    let layout = state
        .layout
        .as_ref()
        .ok_or("The layout is not available.")?;
    if !layout.controls.iter().any(|c| c.id == id) {
        return Err("The control does not exist.".into());
    }
    Ok(e::ControlId::new(id.kind.into(), id.number))
}

/// The profile of the cache with this identity.
fn profile_of<'a>(state: &'a w::State, id: &str) -> Result<&'a w::Profile, String> {
    state
        .profiles
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "The profile does not exist.".to_string())
}

/// The page of the cache: its number, and the reference for the engine,
/// which checks it again.
fn page_ref(state: &w::State, profile: &str, page: &str) -> Result<e::PageRef, String> {
    let number = profile_of(state, profile)?
        .pages
        .iter()
        .position(|p| p.id == page)
        .ok_or("The page does not exist.")?;
    Ok(e::PageRef {
        number,
        id: parse(page.to_string(), "page identity")?,
    })
}

fn folders(folder: Option<String>) -> Result<Vec<e::FolderId>, String> {
    Ok(folder
        .map(|folder| parse(folder, "folder identity"))
        .transpose()?
        .into_iter()
        .collect())
}

/// A location: the profile, the page, and the open folder.
fn location(state: &w::State, target: w::Target) -> Result<(e::Place, usize), String> {
    let page = page_ref(state, &target.profile, &target.page)?;
    let number = page.number;
    Ok((
        e::Place {
            profile: target.profile,
            page: Some(page),
            open: Some(folders(target.folder)?),
        },
        number,
    ))
}

/// A page, with the folders as they are.
fn page_place(state: &w::State, target: w::PageTarget) -> Result<(e::Place, usize), String> {
    let page = page_ref(state, &target.profile, &target.page)?;
    let number = page.number;
    Ok((
        e::Place {
            profile: target.profile,
            page: Some(page),
            open: None,
        },
        number,
    ))
}

/// A profile only: the page and the folders stay as they are.
fn profile_place(profile: String) -> e::Place {
    e::Place {
        profile,
        page: None,
        open: None,
    }
}

fn at(place: e::Place, command: e::Command) -> e::Command {
    e::Command::AtPlace {
        place,
        command: Box::new(command),
    }
}

pub(super) fn command(method: w::Method, state: &w::State) -> Result<Change, String> {
    let named = named(&method);
    let Built { command, made } = build(method, state)?;
    Ok(Change {
        command,
        made,
        named,
    })
}

/// The place that a request names, in the form of the response: the profile,
/// the page, and the folder, each null if the request has no such part. A
/// request that adds a thing leaves the field of the thing null: the worker
/// adds the new identity after the engine obeyed. `open_folder` names the
/// folder that it opens.
fn named(method: &w::Method) -> w::Changed {
    use w::Method as M;
    let place = |profile: &str, page: Option<&str>, folder: Option<&str>| w::Changed {
        profile: Some(profile.to_string()),
        page: page.map(str::to_string),
        folder: folder.map(str::to_string),
    };
    let target = |t: &w::Target| place(&t.profile, Some(&t.page), t.folder.as_deref());
    let page = |t: &w::PageTarget| place(&t.profile, Some(&t.page), None);
    let profile = |profile: &str| place(profile, None, None);
    match method {
        M::SetLabel(p) => target(&p.target),
        M::SetLabelStyle(p) => target(&p.target),
        M::SetColor(p) => target(&p.target),
        M::SetImage(p) => target(&p.target),
        M::SetIcon(p) => target(&p.target),
        M::SetBinding(p) => target(&p.target),
        M::CopyControl(p) | M::PasteControl(p) | M::ClearControl(p) | M::CopyToAllPages(p) => {
            target(&p.target)
        }
        M::CloseFolder(p) => target(&p.target),
        M::OpenFolder(p) => place(&p.target.profile, Some(&p.target.page), Some(&p.folder)),
        M::AddPage(p) => profile(&p.profile),
        M::DuplicatePage(p) | M::RemovePage(p) | M::SelectPage(p) | M::SetFirstPage(p) => {
            page(&p.target)
        }
        M::RenamePage(p) => page(&p.target),
        M::MovePage(p) => page(&p.target),
        M::SetBackground(p) => page(&p.target),
        M::SetBackgroundFit(p) => page(&p.target),
        M::AddFolder(p) => profile(&p.profile),
        M::RenameFolder(p) => place(&p.profile, None, Some(&p.folder)),
        M::MoveFolder(p) => place(&p.profile, None, Some(&p.folder)),
        M::RemoveFolder(p) => place(&p.profile, None, Some(&p.folder)),
        M::RemoveProfile(p) | M::SelectProfile(p) | M::SetDefaultProfile(p) => profile(&p.profile),
        M::Undo(p) | M::Redo(p) => profile(&p.profile),
        M::RenameProfile(p) => profile(&p.profile),
        M::SetProfileColor(p) => profile(&p.profile),
        M::LinkApplication(p) => profile(&p.profile),
        M::UnlinkApplication(p) => profile(&p.profile),
        M::ExportProfile(p) => profile(&p.profile),
        M::AddProfile(_)
        | M::ImportProfile(_)
        | M::SetFollowFrontApplication(_)
        | M::SetDeviceSettings(_)
        | M::Hello(_)
        | M::State(_)
        | M::Layout(_)
        | M::Kinds(_)
        | M::Choices(_)
        | M::Profiles(_)
        | M::Pages(_)
        | M::Folders(_)
        | M::Content(_)
        | M::DeviceSettings(_)
        | M::History(_)
        | M::Connection(_) => w::Changed::default(),
    }
}

fn build(method: w::Method, state: &w::State) -> Result<Built, String> {
    use e::Command as C;
    use w::Method as M;
    let nothing = |command| Built {
        command,
        made: Made::Nothing,
    };
    let change = match method {
        M::SetLabel(p) => {
            let surface = surface(p.surface, state)?;
            let (place, _) = location(state, p.target)?;
            nothing(at(
                place,
                C::SetLabel {
                    surface,
                    label: p.label,
                },
            ))
        }
        M::SetLabelStyle(p) => {
            let surface = surface(p.surface, state)?;
            // No style is the default style of the engine.
            let default = e::LabelStyle::default();
            let style = p.style.unwrap_or(w::LabelStyle {
                position: None,
                size: default.size,
                color: None,
                bold: default.bold,
            });
            let (place, _) = location(state, p.target)?;
            nothing(at(
                place,
                C::Group {
                    name: "Set Title Style".into(),
                    commands: vec![
                        C::SetLabelStyle {
                            surface,
                            change: e::LabelChange::Position(style.position.map(Into::into)),
                        },
                        C::SetLabelStyle {
                            surface,
                            change: e::LabelChange::Size(style.size),
                        },
                        C::SetLabelStyle {
                            surface,
                            change: e::LabelChange::Color(
                                style.color.map(|c| parse(c, "colour")).transpose()?,
                            ),
                        },
                        C::SetLabelStyle {
                            surface,
                            change: e::LabelChange::Bold(style.bold),
                        },
                    ],
                },
            ))
        }
        M::SetColor(p) => {
            let surface = surface(p.surface, state)?;
            let color = p.color.map(|c| parse(c, "colour")).transpose()?;
            let (place, _) = location(state, p.target)?;
            nothing(at(place, C::SetColor { surface, color }))
        }
        M::SetImage(p) => {
            let surface = surface(p.surface, state)?;
            let path = p.path.map(path).transpose()?;
            let (place, _) = location(state, p.target)?;
            nothing(at(place, C::SetImage { surface, path }))
        }
        M::SetIcon(p) => {
            let surface = surface(p.surface, state)?;
            let icon = p
                .icon
                .map(|i| {
                    Ok::<_, String>(e::SurfaceIcon {
                        name: i.name,
                        color: i.color.map(|c| parse(c, "colour")).transpose()?,
                    })
                })
                .transpose()?;
            let (place, _) = location(state, p.target)?;
            nothing(at(place, C::SetIcon { surface, icon }))
        }
        M::SetBinding(p) => {
            let control_id = p.control;
            let control = control(p.control, state)?;
            let gesture = p.gesture;
            // The layout lists the gestures of the control. A binding with a
            // gesture that the control cannot make would never run.
            let allowed = state.layout.as_ref().is_some_and(|layout| {
                layout
                    .controls
                    .iter()
                    .any(|c| c.id == control_id && c.gestures.contains(&gesture))
            });
            if !allowed {
                return Err("A control of this kind cannot make this gesture.".into());
            }
            // The conversion names the setting that it refuses.
            let action = p.action.map(e::Action::try_from).transpose()?;
            // The window cannot make a binding with an unknown kind, an
            // unknown field, a hidden field, or a value that the kind
            // refuses.
            if let Some(action) = &action {
                super::check::action(
                    action,
                    &state.kinds,
                    &super::check::At {
                        state,
                        profile: &p.target.profile,
                    },
                )?;
            }
            let (place, page) = location(state, p.target)?;
            // The engine rule for the holder of a binding: a key is in the
            // open folder, every other control is on the page.
            let binding = e::Location {
                page,
                open: place.open.clone().unwrap_or_default(),
            }
            .binding_id(control, gesture.into());
            nothing(at(place, C::SetBinding { binding, action }))
        }
        M::CopyControl(p) => {
            let control = control(p.control, state)?;
            let (place, _) = location(state, p.target)?;
            nothing(at(place, C::CopyControl { control }))
        }
        M::PasteControl(p) => {
            let control = control(p.control, state)?;
            let (place, _) = location(state, p.target)?;
            nothing(at(place, C::PasteControl { control }))
        }
        M::ClearControl(p) => {
            let control = control(p.control, state)?;
            let (place, _) = location(state, p.target)?;
            nothing(at(place, C::ClearControl { control }))
        }
        M::CopyToAllPages(p) => {
            let control = control(p.control, state)?;
            let (place, _) = location(state, p.target)?;
            nothing(at(place, C::CopyToAllPages { control }))
        }
        M::AddPage(p) => Built {
            command: at(profile_place(p.profile), C::AddPage { copy_of: None }),
            made: Made::Page,
        },
        M::DuplicatePage(p) => {
            let (place, number) = page_place(state, p.target)?;
            Built {
                command: at(
                    place,
                    C::AddPage {
                        copy_of: Some(number),
                    },
                ),
                made: Made::Page,
            }
        }
        M::RenamePage(p) => {
            let (place, page) = page_place(state, p.target)?;
            nothing(at(place, C::RenamePage { page, name: p.name }))
        }
        M::MovePage(p) => {
            let (place, page) = page_place(state, p.target)?;
            nothing(at(place, C::MovePage { page, to: p.to }))
        }
        M::RemovePage(p) => {
            let (place, page) = page_place(state, p.target)?;
            nothing(at(place, C::RemovePage { page }))
        }
        M::SelectPage(p) => {
            let (mut place, page) = page_place(state, p.target)?;
            // A selection closes the open folders.
            place.open = Some(Vec::new());
            nothing(at(place, C::SelectPage { page }))
        }
        M::SetBackground(p) => {
            let background = p
                .background
                .map(|b| {
                    Ok::<_, String>(match b {
                        w::NewBackground::Color { color } => {
                            e::NewBackground::Color(parse(color, "colour")?)
                        }
                        w::NewBackground::Image { path: file, fit } => e::NewBackground::Image {
                            path: path(file)?,
                            fit: fit.into(),
                        },
                    })
                })
                .transpose()?;
            let (place, page) = page_place(state, p.target)?;
            nothing(at(place, C::SetBackground { page, background }))
        }
        M::SetBackgroundFit(p) => {
            let (place, page) = page_place(state, p.target)?;
            nothing(at(
                place,
                C::SetBackgroundFit {
                    page,
                    fit: p.fit.into(),
                },
            ))
        }
        M::SetFirstPage(p) => {
            let profile = p.target.profile.clone();
            let (place, page) = page_place(state, p.target)?;
            nothing(at(place, C::SetFirstPage { profile, page }))
        }
        M::AddFolder(p) => {
            let id = match p.id {
                Some(id) => parse(id, "folder identity")?,
                None => e::FolderId::random(),
            };
            let copy_of = p
                .copy_of
                .map(|folder| parse(folder, "folder identity"))
                .transpose()?;
            Built {
                command: at(
                    profile_place(p.profile),
                    C::AddFolder {
                        id,
                        name: p.name,
                        copy_of,
                    },
                ),
                made: Made::Folder(id.to_string()),
            }
        }
        M::RenameFolder(p) => nothing(at(
            profile_place(p.profile),
            C::RenameFolder {
                folder: parse(p.folder, "folder identity")?,
                name: p.name,
            },
        )),
        M::MoveFolder(p) => nothing(at(
            profile_place(p.profile),
            C::MoveFolder {
                folder: parse(p.folder, "folder identity")?,
                to: p.to,
            },
        )),
        M::RemoveFolder(p) => nothing(at(
            profile_place(p.profile),
            C::RemoveFolder {
                folder: parse(p.folder, "folder identity")?,
            },
        )),
        M::OpenFolder(p) => {
            let folder = parse(p.folder, "folder identity")?;
            let (place, _) = location(state, p.target)?;
            nothing(at(place, C::OpenFolder { folder }))
        }
        M::CloseFolder(p) => {
            // The open folders are a part of the active location. A profile
            // that is not active has none, and a folder that is not open
            // cannot close.
            if state.profiles.active.as_deref() != Some(p.target.profile.as_str()) {
                return Err(
                    "The profile is not active, thus it has no open folder. Read `pages` \
                     to find the active profile."
                        .into(),
                );
            }
            let open = state
                .pages
                .location
                .as_ref()
                .filter(|place| place.page == p.target.page)
                .and_then(|place| place.open.last())
                .ok_or("The folder is not open.")?;
            if Some(open.as_str()) != p.target.folder.as_deref() {
                return Err("The folder is not open.".into());
            }
            let (place, _) = location(state, p.target)?;
            nothing(at(place, C::CloseFolder))
        }
        M::AddProfile(p) => Built {
            command: C::AddProfile {
                name: p.name,
                copy_of: p.copy_of,
            },
            made: Made::Profile,
        },
        M::RenameProfile(p) => nothing(C::RenameProfile {
            profile: p.profile,
            name: p.name,
        }),
        M::RemoveProfile(p) => nothing(C::RemoveProfile { profile: p.profile }),
        M::SelectProfile(p) => nothing(C::SelectProfile { profile: p.profile }),
        M::SetProfileColor(p) => nothing(C::SetProfileColor {
            profile: p.profile,
            color: p.color.map(|c| parse(c, "colour")).transpose()?,
        }),
        M::SetDefaultProfile(p) => nothing(C::SetDefaultProfile { profile: p.profile }),
        M::LinkApplication(p) => nothing(C::LinkApplication {
            profile: p.profile,
            link: e::ApplicationLink {
                bundle_id: p.link.bundle_id,
                name: p.link.name,
            },
        }),
        M::UnlinkApplication(p) => nothing(C::UnlinkApplication {
            profile: p.profile,
            bundle_id: p.bundle_id,
        }),
        M::SetFollowFrontApplication(p) => nothing(C::SetFollowFrontApplication(p.follow)),
        M::ExportProfile(p) => nothing(C::ExportProfile {
            profile: p.profile,
            path: path(p.path)?,
        }),
        M::ImportProfile(p) => Built {
            command: C::ImportProfile {
                path: path(p.path)?,
            },
            made: Made::Profile,
        },
        M::SetDeviceSettings(p) => nothing(C::SetDeviceSettings(e::DeviceSettings {
            brightness: p.brightness,
            screen_off_minutes: p.screen_off_minutes,
            haptic: p.haptic,
            knob_lights: e::KnobLights {
                mode: p.knob_lights.mode.into(),
                brightness: p.knob_lights.brightness,
                colors: p
                    .knob_lights
                    .colors
                    .into_iter()
                    .map(|c| parse(c, "colour"))
                    .collect::<Result<_, _>>()?,
            },
        })),
        M::Undo(p) => Built {
            command: C::InProfile {
                profile: p.profile,
                command: Box::new(C::Undo),
            },
            made: Made::Place,
        },
        M::Redo(p) => Built {
            command: C::InProfile {
                profile: p.profile,
                command: Box::new(C::Redo),
            },
            made: Made::Place,
        },
        M::Hello(_)
        | M::State(_)
        | M::Layout(_)
        | M::Kinds(_)
        | M::Choices(_)
        | M::Profiles(_)
        | M::Pages(_)
        | M::Folders(_)
        | M::Content(_)
        | M::DeviceSettings(_)
        | M::History(_)
        | M::Connection(_) => return Err("This is not a change request.".into()),
    };
    debug_assert!(permitted(&change.command));
    Ok(change)
}

/// The answer of a read that needed the file of a profile.
pub(super) fn read(
    reading: &super::server::ProfileReading,
    profile: &e::Profile,
    feed: &super::Feed,
) -> Result<serde_json::Value, String> {
    use super::server::ProfileReading as R;
    match reading {
        R::Content(target) => feed.read(|state| content(target, profile, state).map(to_json)),
        R::Pages { profile: id } => Ok(to_json(pages(id, profile))),
        R::Folders { .. } => Ok(to_json(folder_list(profile))),
    }
}

fn to_json(value: impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).expect("wire result is JSON data")
}

/// The pages of a profile that is not active. It has no active location.
fn pages(id: &str, profile: &e::Profile) -> w::Pages {
    let ids: Vec<e::PageId> = profile.pages().iter().map(e::Page::id).collect();
    let names: Vec<String> = profile.pages().iter().map(|p| p.name.clone()).collect();
    w::Pages {
        profile: Some(id.to_string()),
        pages: super::state::page_summaries(&ids, &names),
        location: None,
        first: super::state::page_id(&ids, profile.first_page()),
        folders: folder_list(profile),
    }
}

fn folder_list(profile: &e::Profile) -> Vec<w::Folder> {
    profile
        .folders()
        .iter()
        .map(|f| w::Folder {
            id: f.id.to_string(),
            name: f.page.name.clone(),
        })
        .collect()
}

/// Use the engine's Location rules, not another copy of the folder rules.
/// `profile` is a profile that the engine read, with the identity
/// `target.profile`.
pub(super) fn content(
    target: &w::Target,
    profile: &e::Profile,
    state: &w::State,
) -> Result<w::Content, String> {
    let page: e::PageId = parse(target.page.clone(), "page identity")?;
    let number = profile
        .page_number(page)
        .ok_or("The requested page or folder does not exist.")?;
    let location = e::Location {
        page: number,
        open: folders(target.folder.clone())?,
    };
    if e::Location::restored_open(profile, &location.open) != location.open {
        return Err("The requested page or folder does not exist.".into());
    }
    let layout = state
        .layout
        .as_ref()
        .ok_or("The layout is not available.")?;
    let ids: Vec<e::PageId> = profile.pages().iter().map(e::Page::id).collect();
    let surfaces = layout
        .surfaces
        .iter()
        .map(|s| {
            let id = surface(s.id, state)?;
            Ok(w::Surface::from_content(
                s.id,
                &location.content(profile, id),
            ))
        })
        .collect::<Result<_, String>>()?;
    let bindings = location
        .bindings(profile)
        .into_iter()
        .map(|(binding, action)| w::Binding {
            page: super::state::page_id(&ids, binding.page),
            folder: binding.folder.map(|f| f.to_string()),
            control: binding.control.into(),
            gesture: binding.gesture.into(),
            available: state.kinds.iter().any(|k| k.kind == action.kind),
            action: (&action).into(),
        })
        .collect();
    Ok(w::Content {
        profile: Some(target.profile.clone()),
        location: super::state::location(&location, &ids),
        background: profile
            .page(number)
            .and_then(|p| p.background.as_ref())
            .map(Into::into),
        surfaces,
        bindings,
    })
}
