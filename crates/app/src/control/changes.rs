//! Typed wire requests become only the commands allowed by decision 0040.

use super::{protocol as w, state::permitted};
use opencraft_engine as e;
use std::path::PathBuf;

pub(super) struct Change {
    pub select: Option<String>,
    pub command: e::Command,
}

/// The name of a step of the history: `set_label` is `Set Label`.
fn title(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut letters = word.chars();
            letters
                .next()
                .map(|first| first.to_uppercase().chain(letters).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse<T: std::str::FromStr>(text: String) -> Result<T, String> {
    text.parse()
        .map_err(|_| "The parameter value is not correct.".into())
}

pub(super) fn path(text: String) -> Result<PathBuf, String> {
    let path = PathBuf::from(text);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err("A file path must be absolute.".into())
    }
}

pub(super) fn location(target: &w::Target) -> Result<e::Location, String> {
    Ok(e::Location {
        page: target.page,
        open: target
            .folder
            .clone()
            .map(parse)
            .transpose()?
            .into_iter()
            .collect(),
    })
}

fn surface(id: w::Id, state: &w::State) -> Result<e::SurfaceId, String> {
    let layout = state
        .layout
        .as_ref()
        .ok_or("The layout is not available.")?;
    if !layout.surfaces.iter().any(|s| s.id == id) {
        return Err("The surface does not exist.".into());
    }
    Ok(e::SurfaceId::new(
        match id.kind.as_str() {
            "key face" => e::SurfaceKind::KeyFace,
            "bar panel" => e::SurfaceKind::BarPanel,
            "background" => e::SurfaceKind::Background,
            _ => return Err("The surface kind does not exist.".into()),
        },
        id.number,
    ))
}

fn control(id: w::Id, state: &w::State) -> Result<e::ControlId, String> {
    let layout = state
        .layout
        .as_ref()
        .ok_or("The layout is not available.")?;
    if !layout.controls.iter().any(|c| c.id == id) {
        return Err("The control does not exist.".into());
    }
    Ok(e::ControlId::new(
        match id.kind.as_str() {
            "key" => e::ControlKind::Key,
            "knob" => e::ControlKind::Knob,
            "bar panel" => e::ControlKind::BarPanel,
            "touch bar" => e::ControlKind::TouchBar,
            _ => return Err("The control kind does not exist.".into()),
        },
        id.number,
    ))
}

fn fit(text: String) -> Result<e::BackgroundFit, String> {
    e::BackgroundFit::from_key(&text).ok_or_else(|| "The image fit is not correct.".into())
}

pub(super) fn command(method: w::Method, state: &w::State) -> Result<Change, String> {
    use e::Command as C;
    use w::Method as M;
    let step = title(method.name());
    let mut target = None;
    let mut at = |place: w::Target| {
        target = Some(place);
    };
    let command = match method {
        M::SetLabel(p) => {
            at(p.target);
            C::SetLabel {
                surface: surface(p.surface, state)?,
                label: p.label,
            }
        }
        M::SetLabelStyle(p) => {
            at(p.target);
            let surface = surface(p.surface, state)?;
            // No style is the default style of the engine.
            let default = e::LabelStyle::default();
            let style = p.style.unwrap_or(w::LabelStyle {
                position: None,
                size: default.size,
                color: None,
                bold: default.bold,
            });
            let position = style
                .position
                .map(|p| e::LabelPosition::from_key(&p).ok_or("The label position is not correct."))
                .transpose()?;
            C::Group {
                name: "Set Title Style".into(),
                commands: vec![
                    C::SetLabelStyle {
                        surface,
                        change: e::LabelChange::Position(position),
                    },
                    C::SetLabelStyle {
                        surface,
                        change: e::LabelChange::Size(style.size),
                    },
                    C::SetLabelStyle {
                        surface,
                        change: e::LabelChange::Color(style.color.map(parse).transpose()?),
                    },
                    C::SetLabelStyle {
                        surface,
                        change: e::LabelChange::Bold(style.bold),
                    },
                ],
            }
        }
        M::SetColor(p) => {
            at(p.target);
            C::SetColor {
                surface: surface(p.surface, state)?,
                color: p.color.map(parse).transpose()?,
            }
        }
        M::SetImage(p) => {
            at(p.target);
            C::SetImage {
                surface: surface(p.surface, state)?,
                path: p.path.map(path).transpose()?,
            }
        }
        M::SetIcon(p) => {
            at(p.target);
            C::SetIcon {
                surface: surface(p.surface, state)?,
                icon: p
                    .icon
                    .map(|i| {
                        Ok::<_, String>(e::SurfaceIcon {
                            name: i.name,
                            color: i.color.map(parse).transpose()?,
                        })
                    })
                    .transpose()?,
            }
        }
        M::SetBinding(p) => {
            let control = control(p.control, state)?;
            let gesture =
                super::state::gesture_from_name(&p.gesture).ok_or("The gesture is not correct.")?;
            let binding = location(&p.target)?.binding_id(control, gesture);
            at(p.target);
            C::SetBinding {
                binding,
                action: p
                    .action
                    .map(TryInto::try_into)
                    .transpose()
                    .map_err(|_| "The action settings are not correct.")?,
            }
        }
        M::CopyControl(p) => {
            at(p.target);
            C::CopyControl {
                control: control(p.control, state)?,
            }
        }
        M::PasteControl(p) => {
            at(p.target);
            C::PasteControl {
                control: control(p.control, state)?,
            }
        }
        M::ClearControl(p) => {
            at(p.target);
            C::ClearControl {
                control: control(p.control, state)?,
            }
        }
        M::CopyToAllPages(p) => {
            at(p.target);
            C::CopyToAllPages {
                control: control(p.control, state)?,
            }
        }
        M::AddPage(p) => {
            at(p.target);
            C::AddPage { copy_of: p.copy_of }
        }
        M::RenamePage(p) => {
            let page = p.target.page;
            at(p.target);
            C::RenamePage { page, name: p.name }
        }
        M::MovePage(p) => {
            let page = p.target.page;
            at(p.target);
            C::MovePage { page, to: p.to }
        }
        M::RemovePage(p) => {
            let page = p.target.page;
            at(p.target);
            C::RemovePage { page }
        }
        M::SelectPage(p) => {
            let page = p.target.page;
            at(p.target);
            C::SelectPage { page }
        }
        M::SetBackground(p) => {
            let page = p.target.page;
            at(p.target);
            C::SetBackground {
                page,
                background: p
                    .background
                    .map(|b| {
                        Ok::<_, String>(match b {
                            w::NewBackground::Color { color } => {
                                e::NewBackground::Color(parse(color)?)
                            }
                            w::NewBackground::Image { path: file, fit: f } => {
                                e::NewBackground::Image {
                                    path: path(file)?,
                                    fit: fit(f)?,
                                }
                            }
                        })
                    })
                    .transpose()?,
            }
        }
        M::SetBackgroundFit(p) => {
            let page = p.target.page;
            at(p.target);
            C::SetBackgroundFit {
                page,
                fit: fit(p.fit)?,
            }
        }
        M::AddFolder(p) => {
            at(p.target);
            C::AddFolder {
                id: p
                    .id
                    .map(parse)
                    .transpose()?
                    .unwrap_or_else(e::FolderId::random),
                name: p.name,
                copy_of: p.copy_of.map(parse).transpose()?,
            }
        }
        M::RenameFolder(p) => {
            at(p.target);
            C::RenameFolder {
                folder: parse(p.folder)?,
                name: p.name,
            }
        }
        M::MoveFolder(p) => {
            at(p.target);
            C::MoveFolder {
                folder: parse(p.folder)?,
                to: p.to,
            }
        }
        M::RemoveFolder(p) => {
            at(p.target);
            C::RemoveFolder {
                folder: parse(p.folder)?,
            }
        }
        M::OpenFolder(p) => {
            at(p.target);
            C::OpenFolder {
                folder: parse(p.folder)?,
            }
        }
        M::CloseFolder(p) => {
            at(p.target);
            C::CloseFolder
        }
        M::AddProfile(p) => C::AddProfile {
            name: p.name,
            copy_of: p.copy_of,
        },
        M::RenameProfile(p) => C::RenameProfile {
            profile: p.profile,
            name: p.name,
        },
        M::RemoveProfile(p) => C::RemoveProfile { profile: p.profile },
        M::SelectProfile(p) => C::SelectProfile { profile: p.profile },
        M::SetProfileColor(p) => C::SetProfileColor {
            profile: p.profile,
            color: p.color.map(parse).transpose()?,
        },
        // The mark is on a page of a profile that can be inactive. The
        // request names both: it does not select anything.
        M::SetFirstPage(p) => C::SetFirstPage {
            profile: p.profile,
            page: p.page,
        },
        M::SetDefaultProfile(p) => C::SetDefaultProfile { profile: p.profile },
        M::LinkApplication(p) => C::LinkApplication {
            profile: p.profile,
            link: e::ApplicationLink {
                bundle_id: p.link.bundle_id,
                name: p.link.name,
            },
        },
        M::UnlinkApplication(p) => C::UnlinkApplication {
            profile: p.profile,
            bundle_id: p.bundle_id,
        },
        M::SetFollowFrontApplication(p) => C::SetFollowFrontApplication(p.follow),
        M::ExportProfile(p) => C::ExportProfile {
            profile: p.profile,
            path: path(p.path)?,
        },
        M::ImportProfile(p) => C::ImportProfile {
            path: path(p.path)?,
        },
        M::SetDeviceSettings(p) => C::SetDeviceSettings(e::DeviceSettings {
            brightness: p.brightness,
            screen_off_minutes: p.screen_off_minutes,
            haptic: p.haptic,
            knob_lights: e::KnobLights {
                mode: match p.knob_lights.mode.as_str() {
                    "effect" => e::KnobLightMode::Effect,
                    "colors" => e::KnobLightMode::Colors,
                    "off" => e::KnobLightMode::Off,
                    _ => return Err("The knob light mode is not correct.".into()),
                },
                brightness: p.knob_lights.brightness,
                colors: p
                    .knob_lights
                    .colors
                    .into_iter()
                    .map(parse)
                    .collect::<Result<_, _>>()?,
            },
        }),
        M::Undo(p) => C::InProfile {
            profile: p.profile,
            command: Box::new(C::Undo),
        },
        M::Redo(p) => C::InProfile {
            profile: p.profile,
            command: Box::new(C::Redo),
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
    let change = if let Some(target) = target {
        Change {
            select: Some(target.profile.clone()),
            command: C::InProfile {
                profile: target.profile.clone(),
                command: Box::new(C::Group {
                    name: step,
                    commands: vec![C::AtLocation {
                        location: location(&target)?,
                        command: Box::new(command),
                    }],
                }),
            },
        }
    } else {
        Change {
            select: None,
            command,
        }
    };
    debug_assert!(permitted(&change.command));
    Ok(change)
}

/// Use the engine's Location rules, not another copy of the folder rules.
pub(super) fn content(
    target: &w::Target,
    profile: &e::Profile,
    state: &w::State,
) -> Result<w::Content, String> {
    let location = location(target)?;
    if profile.page(location.page).is_none()
        || e::Location::restored_open(profile, &location.open) != location.open
    {
        return Err("The requested page or folder does not exist.".into());
    }
    let layout = state
        .layout
        .as_ref()
        .ok_or("The layout is not available.")?;
    let surfaces = layout
        .surfaces
        .iter()
        .map(|s| {
            let id = surface(s.id.clone(), state)?;
            Ok(w::Surface::from_content(
                s.id.clone(),
                &location.content(profile, id),
            ))
        })
        .collect::<Result<_, String>>()?;
    let bindings = location
        .bindings(profile)
        .into_iter()
        .map(|(binding, action)| w::Binding {
            page: binding.page,
            folder: binding.folder.map(|f| f.to_string()),
            control: binding.control.into(),
            gesture: super::state::gesture_name(binding.gesture).into(),
            available: state.kinds.iter().any(|k| k.kind == action.kind),
            action: (&action).into(),
        })
        .collect();
    Ok(w::Content {
        profile: Some(target.profile.clone()),
        location: (&location).into(),
        background: profile
            .page(target.page)
            .and_then(|p| p.background.as_ref())
            .map(Into::into),
        surfaces,
        bindings,
    })
}
