//! Engine-to-wire conversion. The exhaustive matches are the protocol audit
//! required by decision 0040. Rendered pixels, input, problem logs and file
//! destinations are deliberately not part of the read state.

use super::protocol as w;
use opencraft_engine as e;

impl From<e::SurfaceId> for w::Id {
    fn from(id: e::SurfaceId) -> Self {
        Self {
            kind: id.kind.name().into(),
            number: id.number,
        }
    }
}

impl From<e::ControlId> for w::Id {
    fn from(id: e::ControlId) -> Self {
        Self {
            kind: id.kind.name().into(),
            number: id.number,
        }
    }
}

impl From<&e::Action> for w::Action {
    fn from(action: &e::Action) -> Self {
        Self {
            kind: action.kind.clone(),
            settings: action
                .settings
                .iter()
                .map(|(k, v)| (k.clone(), value(v)))
                .collect(),
        }
    }
}

fn value(value: &e::Value) -> serde_json::Value {
    match value {
        e::Value::Text(v) => v.clone().into(),
        e::Value::Integer(v) => (*v).into(),
        e::Value::Boolean(v) => (*v).into(),
        e::Value::List(v) => v.iter().map(self::value).collect(),
        e::Value::Table(v) => {
            serde_json::Value::Object(v.iter().map(|(k, v)| (k.clone(), self::value(v))).collect())
        }
    }
}

impl TryFrom<w::Action> for e::Action {
    type Error = serde_json::Error;
    fn try_from(action: w::Action) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: action.kind,
            settings: serde_json::from_value(serde_json::to_value(action.settings)?)?,
        })
    }
}

impl From<&e::ChoiceOption> for w::Choice {
    fn from(choice: &e::ChoiceOption) -> Self {
        Self {
            value: choice.value.clone(),
            label: choice.label.clone(),
        }
    }
}

impl From<&e::ShownWhen> for w::ShownWhen {
    fn from(rule: &e::ShownWhen) -> Self {
        Self {
            field: rule.field.clone(),
            key: rule.key.clone(),
            values: rule.values.clone(),
        }
    }
}

impl From<&e::InputType> for w::Input {
    fn from(input: &e::InputType) -> Self {
        match input {
            e::InputType::Text => Self::Text,
            e::InputType::LongText => Self::LongText,
            e::InputType::Integer { min, max } => Self::Integer {
                min: *min,
                max: *max,
            },
            e::InputType::Choice { options } => Self::Choice {
                options: options.iter().map(Into::into).collect(),
            },
            e::InputType::RuntimeChoice => Self::RuntimeChoice,
            e::InputType::RuntimeChoiceList => Self::RuntimeChoiceList,
            e::InputType::Profile => Self::Profile,
            e::InputType::Colour => Self::Colour,
            e::InputType::Toggle => Self::Toggle,
            e::InputType::Hotkey => Self::Hotkey,
            e::InputType::Path { kind } => Self::Path {
                kind: match kind {
                    e::PathKind::File => w::PathKind::File,
                    e::PathKind::Application { bundle_id } => w::PathKind::Application {
                        bundle_id: bundle_id.clone(),
                    },
                },
            },
            e::InputType::Action => Self::Action,
            e::InputType::ActionList => Self::ActionList,
            e::InputType::TimedActionList => Self::TimedActionList,
            e::InputType::Folder => Self::Folder,
        }
    }
}

impl From<&e::KindDescription> for w::Kind {
    fn from(kind: &e::KindDescription) -> Self {
        Self {
            kind: kind.kind.clone(),
            name: kind.name.clone(),
            group: kind.group.clone(),
            help: kind.help.clone(),
            icon: kind.icon.clone(),
            needs_accessibility: kind.needs_accessibility,
            fields: kind
                .fields
                .iter()
                .map(|f| w::Field {
                    key: f.key.clone(),
                    label: f.label.clone(),
                    input: (&f.input).into(),
                    default: f.default.as_ref().map(value),
                })
                .collect(),
            direction: kind.direction.as_ref().map(|d| w::Direction {
                key: d.key.clone(),
                right: d.right.clone(),
                left: d.left.clone(),
            }),
            states: kind.states.as_ref().map(|s| w::TwoStates {
                on_label: s.on_label.clone(),
            }),
            folder_back: kind.folder_back,
            shown_when: kind.shown_when.iter().map(Into::into).collect(),
            permission_when: kind.permission_when.as_ref().map(Into::into),
            status: None,
        }
    }
}

impl From<&e::Status> for w::Status {
    fn from(status: &e::Status) -> Self {
        match status {
            e::Status::Ready => Self::Ready,
            e::Status::NeedsUser { text, button } => Self::NeedsUser {
                text: text.clone(),
                button: button.clone(),
            },
        }
    }
}

impl From<&e::DeviceLayout> for w::Layout {
    fn from(layout: &e::DeviceLayout) -> Self {
        Self {
            name: layout.name.clone(),
            display_width: layout.display_width,
            display_height: layout.display_height,
            surfaces: layout
                .surfaces
                .iter()
                .map(|s| w::SurfaceLayout {
                    id: s.id.into(),
                    x: s.x,
                    y: s.y,
                    width: s.width,
                    height: s.height,
                })
                .collect(),
            controls: layout
                .controls
                .iter()
                .map(|c| w::ControlLayout {
                    id: c.id.into(),
                    row: c.row,
                    column: c.column,
                    surface: c.surface.map(Into::into),
                    feedback_surface: c.feedback_surface.map(Into::into),
                })
                .collect(),
            settings: w::SettingsSupport {
                lowest_brightness: layout.settings.lowest_brightness,
                knob_lights: layout.settings.knob_lights,
                haptic: layout.settings.haptic,
            },
        }
    }
}

impl From<&e::ApplicationLink> for w::ApplicationLink {
    fn from(link: &e::ApplicationLink) -> Self {
        Self {
            bundle_id: link.bundle_id.clone(),
            name: link.name.clone(),
        }
    }
}

impl From<&e::ProfileEntry> for w::Profile {
    fn from(profile: &e::ProfileEntry) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            color: profile.color.map(|c| c.to_string()),
            pages: profile.pages.clone(),
            first_page: profile.first_page,
            applications: profile.applications.iter().map(Into::into).collect(),
            applied: profile.applied.iter().map(Into::into).collect(),
            link_owners: profile
                .link_owners
                .iter()
                .map(|l| w::LinkOwner {
                    bundle_id: l.bundle_id.clone(),
                    profile: l.profile.clone(),
                })
                .collect(),
        }
    }
}

impl From<&e::Location> for w::Location {
    fn from(location: &e::Location) -> Self {
        Self {
            page: location.page,
            open: location.open.iter().map(ToString::to_string).collect(),
        }
    }
}

impl From<&e::SurfaceState> for w::Surface {
    fn from(surface: &e::SurfaceState) -> Self {
        let c = &surface.content;
        Self {
            id: surface.id.into(),
            label: c.label.clone(),
            label_style: w::LabelStyle {
                position: c.label_style.position.map(|p| p.key().into()),
                size: c.label_style.size,
                color: c.label_style.color.map(|c| c.to_string()),
                bold: c.label_style.bold,
            },
            color: c.color.map(|c| c.to_string()),
            image: c.image.clone(),
            icon: c.icon.as_ref().map(|i| w::Icon {
                name: i.name.clone(),
                color: i.color.map(|c| c.to_string()),
            }),
        }
    }
}

impl From<&e::Background> for w::Background {
    fn from(background: &e::Background) -> Self {
        match background {
            e::Background::Color(color) => Self::Color {
                color: color.to_string(),
            },
            e::Background::Image { image, fit } => Self::Image {
                image: image.clone(),
                fit: fit.key().into(),
            },
        }
    }
}

impl From<&e::DeviceSettings> for w::DeviceSettings {
    fn from(settings: &e::DeviceSettings) -> Self {
        Self {
            brightness: settings.brightness,
            screen_off_minutes: settings.screen_off_minutes,
            haptic: settings.haptic,
            knob_lights: w::KnobLights {
                mode: settings.knob_lights.mode.name().into(),
                brightness: settings.knob_lights.brightness,
                colors: settings
                    .knob_lights
                    .colors
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            },
        }
    }
}

impl w::State {
    /// Each event has an explicit policy. This runs on the shell thread and
    /// on the socket thread; neither copy contains rendered pixels.
    pub(super) fn apply(&mut self, event: &e::Event) {
        match event {
            e::Event::Layout(layout) => {
                // A new model invalidates all model-specific state.
                *self = Self {
                    layout: Some(layout.as_ref().into()),
                    ..Default::default()
                };
            }
            e::Event::Kinds(kinds) => self.kinds = kinds.iter().map(Into::into).collect(),
            // Only the client with the pending opaque id gets these choices.
            e::Event::Choices { .. } => {}
            e::Event::Profiles {
                profiles,
                active,
                default,
                cause,
            } => {
                if self.profiles.active.as_ref() != Some(active) {
                    self.pages = w::Pages::default();
                    self.content.bindings.clear();
                    self.history = w::History::default();
                }
                self.profiles.profiles = profiles.iter().map(Into::into).collect();
                self.profiles.active = Some(active.clone());
                self.profiles.default = Some(default.clone());
                self.profiles.cause = match cause {
                    e::ActiveCause::Selected => w::Cause::Selected,
                    e::ActiveCause::Linked { bundle_id } => w::Cause::Linked {
                        bundle_id: bundle_id.clone(),
                    },
                    e::ActiveCause::Default => w::Cause::Default,
                };
                self.content.profile = Some(active.clone());
            }
            e::Event::FollowFrontApplication(follow) => {
                self.profiles.follow_front_application = *follow
            }
            e::Event::Pages {
                names,
                location,
                first,
                folders,
            } => {
                self.pages = w::Pages {
                    names: names.clone(),
                    location: location.into(),
                    first: *first,
                    folders: folders
                        .iter()
                        .map(|f| w::Folder {
                            id: f.id.to_string(),
                            name: f.name.clone(),
                        })
                        .collect(),
                };
                self.content.location = location.into();
                self.content.bindings.clear();
            }
            e::Event::Binding(binding) => {
                let id = binding.binding;
                let control: w::Id = id.control.into();
                let gesture = serde_json::to_value(id.gesture)
                    .expect("gesture is text")
                    .as_str()
                    .unwrap()
                    .to_string();
                let folder = id.folder.map(|f| f.to_string());
                self.content.bindings.retain(|b| {
                    !(b.page == id.page
                        && b.folder == folder
                        && b.control == control
                        && b.gesture == gesture)
                });
                if let Some(action) = &binding.action {
                    self.content.bindings.push(w::Binding {
                        page: id.page,
                        folder,
                        control,
                        gesture,
                        action: action.into(),
                        available: binding.available,
                    });
                }
            }
            e::Event::History { undo, redo } => {
                self.history = w::History {
                    undo: undo.clone(),
                    redo: redo.clone(),
                }
            }
            // The clipboard is internal; no read request exposes its contents.
            e::Event::Clipboard(_) => {}
            e::Event::Status { kind, status } => {
                if let Some(k) = self.kinds.iter_mut().find(|k| k.kind == *kind) {
                    k.status = Some(status.into());
                }
            }
            // Counters are a diagnostic, not a control read.
            e::Event::Counters(_) => {}
            e::Event::DeviceSettings(settings) => self.device_settings = Some(settings.into()),
            e::Event::Surface(surface) => {
                let wire = w::Surface::from(surface);
                self.content.surfaces.retain(|s| s.id != wire.id);
                self.content.surfaces.push(wire);
            }
            e::Event::Background(background) => {
                self.content.background = background.background.as_ref().map(Into::into)
            }
            e::Event::Connected => self.connection.connected = true,
            e::Event::Firmware(firmware) => self.connection.firmware = firmware.clone(),
            e::Event::Disconnected => self.connection = w::Connection::default(),
            // Never expose input, file destinations, or arbitrary engine logs.
            e::Event::Input(_) => {}
            e::Event::ProfileExported { .. } => {}
            e::Event::ProfileImported { .. } => {}
            e::Event::Problem(_) => {}
        }
    }
}

/// Explicit command audit. `true` means permitted by decision 0040 (change
/// requests are added in #403); `false` means never accepted from a client.
/// This is not a deserializer: the wire enum is the actual allow list.
pub(super) fn permitted(command: &e::Command) -> bool {
    match command {
        e::Command::SetLabel { .. } => true,
        e::Command::SetLabelStyle { .. } => true,
        e::Command::SetColor { .. } => true,
        e::Command::SetImage { .. } => true,
        e::Command::SetIcon { .. } => true,
        e::Command::SetBinding { .. } => true,
        e::Command::SetBackground { .. } => true,
        e::Command::SetBackgroundFit { .. } => true,
        e::Command::SelectPage { .. } => true,
        e::Command::AddPage { .. } => true,
        e::Command::RenamePage { .. } => true,
        e::Command::MovePage { .. } => true,
        e::Command::RemovePage { .. } => true,
        e::Command::AddFolder { .. } => true,
        e::Command::RenameFolder { .. } => true,
        e::Command::MoveFolder { .. } => true,
        e::Command::RemoveFolder { .. } => true,
        e::Command::OpenFolder { .. } => true,
        e::Command::CloseFolder => true,
        e::Command::SelectProfile { .. } => true,
        e::Command::AddProfile { .. } => true,
        e::Command::RenameProfile { .. } => true,
        e::Command::RemoveProfile { .. } => true,
        e::Command::SetProfileColor { .. } => true,
        e::Command::SetFirstPage { .. } => true,
        e::Command::ExportProfile { .. } => true,
        e::Command::ImportProfile { .. } => true,
        e::Command::AddStarterProfile { .. } => false,
        e::Command::Undo => true,
        e::Command::Redo => true,
        e::Command::StartTextEdit => false,
        e::Command::EndTextEdit => false,
        e::Command::Group { commands, .. } => commands.iter().all(permitted),
        e::Command::CopyControl { .. } => true,
        e::Command::PasteControl { .. } => true,
        e::Command::ClearControl { .. } => true,
        e::Command::CopyToAllPages { .. } => true,
        e::Command::SetDeviceSettings(_) => true,
        e::Command::PowerNotice(_) => false,
        e::Command::FrontApplication(_) => false,
        e::Command::LinkApplication { .. } => true,
        e::Command::UnlinkApplication { .. } => true,
        e::Command::SetDefaultProfile { .. } => true,
        e::Command::SetFollowFrontApplication(_) => true,
        e::Command::WakeDevice => false,
        e::Command::Resolve { .. } => false,
        e::Command::Choices { .. } => true,
        e::Command::TestAction { .. } => false,
        e::Command::ReadCounters => false,
        e::Command::InProfile { command, .. } => permitted(command),
        e::Command::Stop => false,
    }
}
