//! Engine-to-wire conversion. The exhaustive matches are the protocol audit
//! required by decision 0040. Rendered pixels, input, problem logs and file
//! destinations are deliberately not part of the read state.

use super::protocol as w;
use opencraft_engine as e;

impl From<e::SurfaceKind> for w::SurfaceKind {
    fn from(kind: e::SurfaceKind) -> Self {
        match kind {
            e::SurfaceKind::KeyFace => Self::KeyFace,
            e::SurfaceKind::BarPanel => Self::BarPanel,
            e::SurfaceKind::Background => Self::Background,
        }
    }
}

impl From<e::ControlKind> for w::ControlKind {
    fn from(kind: e::ControlKind) -> Self {
        match kind {
            e::ControlKind::Key => Self::Key,
            e::ControlKind::Knob => Self::Knob,
            e::ControlKind::BarPanel => Self::BarPanel,
            e::ControlKind::TouchBar => Self::TouchBar,
        }
    }
}

impl From<e::SurfaceId> for w::SurfaceId {
    fn from(id: e::SurfaceId) -> Self {
        Self {
            kind: id.kind.into(),
            number: id.number,
        }
    }
}

impl From<e::ControlId> for w::ControlId {
    fn from(id: e::ControlId) -> Self {
        Self {
            kind: id.kind.into(),
            number: id.number,
        }
    }
}

impl From<e::GestureKind> for w::Gesture {
    fn from(gesture: e::GestureKind) -> Self {
        match gesture {
            e::GestureKind::Press => Self::Press,
            e::GestureKind::Release => Self::Release,
            e::GestureKind::TurnLeft => Self::TurnLeft,
            e::GestureKind::TurnRight => Self::TurnRight,
            e::GestureKind::Push => Self::Push,
            e::GestureKind::Tap => Self::Tap,
            e::GestureKind::SwipeLeft => Self::SwipeLeft,
            e::GestureKind::SwipeRight => Self::SwipeRight,
            e::GestureKind::LongPress => Self::LongPress,
            e::GestureKind::DoublePress => Self::DoublePress,
        }
    }
}

impl From<e::LabelPosition> for w::LabelPosition {
    fn from(position: e::LabelPosition) -> Self {
        match position {
            e::LabelPosition::Top => Self::Top,
            e::LabelPosition::Middle => Self::Middle,
            e::LabelPosition::Bottom => Self::Bottom,
            e::LabelPosition::Hidden => Self::Hidden,
        }
    }
}

impl From<e::BackgroundFit> for w::BackgroundFit {
    fn from(fit: e::BackgroundFit) -> Self {
        match fit {
            e::BackgroundFit::Fill => Self::Fill,
            e::BackgroundFit::Fit => Self::Fit,
        }
    }
}

impl From<e::KnobLightMode> for w::KnobLightMode {
    fn from(mode: e::KnobLightMode) -> Self {
        match mode {
            e::KnobLightMode::Effect => Self::Effect,
            e::KnobLightMode::Colors => Self::Colors,
            e::KnobLightMode::Off => Self::Off,
        }
    }
}

/// The text of the identity of the page with this number. Empty if the list
/// has no such page: the cache can be ahead of an event, never behind.
pub(super) fn page_id(ids: &[e::PageId], page: usize) -> String {
    ids.get(page).map(ToString::to_string).unwrap_or_default()
}

/// The pages of a profile: the identity and the name of each.
pub(super) fn page_summaries(ids: &[e::PageId], names: &[String]) -> Vec<w::PageSummary> {
    ids.iter()
        .zip(names)
        .map(|(id, name)| w::PageSummary {
            id: id.to_string(),
            name: name.clone(),
        })
        .collect()
}

/// The location in wire form, for the pages `ids`.
pub(super) fn location(location: &e::Location, ids: &[e::PageId]) -> w::Location {
    w::Location {
        page: page_id(ids, location.page),
        open: location.open.iter().map(ToString::to_string).collect(),
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

/// The setting of an action for a wire value. Null and a number with a
/// fraction are not a setting. The reason is the end of the text of the
/// problem: [`TryFrom<w::Action>`] puts the key in front of it.
fn setting(value: serde_json::Value) -> Result<e::Value, String> {
    Ok(match value {
        serde_json::Value::String(v) => e::Value::Text(v),
        serde_json::Value::Bool(v) => e::Value::Boolean(v),
        serde_json::Value::Number(v) => {
            e::Value::Integer(v.as_i64().ok_or("it is not a whole number")?)
        }
        serde_json::Value::Array(v) => {
            e::Value::List(v.into_iter().map(setting).collect::<Result<_, _>>()?)
        }
        serde_json::Value::Object(v) => e::Value::Table(
            v.into_iter()
                .map(|(k, v)| Ok((k, setting(v)?)))
                .collect::<Result<_, String>>()?,
        ),
        serde_json::Value::Null => return Err("it is null".into()),
    })
}

impl TryFrom<w::Action> for e::Action {
    type Error = String;
    fn try_from(action: w::Action) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: action.kind,
            settings: action
                .settings
                .into_iter()
                .map(|(key, value)| {
                    setting(value)
                        .map(|value| (key.clone(), value))
                        .map_err(|why| format!("The setting \"{key}\" is not correct: {why}."))
                })
                .collect::<Result<_, String>>()?,
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

/// The icon names of the build, for `set_icon`. The engine draws an icon on a
/// surface and the window shows it in the icon picker: the client reads the
/// same set (decision 0033). The search is the search of the icon picker.
pub(super) fn icons(search: Option<&str>) -> w::Icons {
    let set = e::icon_set();
    w::Icons {
        set: e::icons::ICON_SET_NAME.into(),
        version: e::icons::ICON_SET_VERSION.into(),
        icons: set
            .search(search.unwrap_or_default())
            .iter()
            .map(|icon| icon.name.to_string())
            .collect(),
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
            receives_release: kind.receives_release,
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
                    gestures: e::GestureKind::of_control(c.id.kind)
                        .iter()
                        .map(|gesture| (*gesture).into())
                        .collect(),
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
            pages: page_summaries(&profile.page_ids, &profile.pages),
            first_page: page_id(&profile.page_ids, profile.first_page),
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

impl From<&e::SurfaceState> for w::Surface {
    fn from(surface: &e::SurfaceState) -> Self {
        Self::from_content(surface.id.into(), &surface.content)
    }
}

impl w::Surface {
    pub(super) fn from_content(id: w::SurfaceId, c: &e::SurfaceContent) -> Self {
        let w::SurfaceFace {
            label,
            label_style,
            color,
            image,
            icon,
        } = w::SurfaceFace::from_content(c);
        Self {
            id,
            label,
            label_style,
            color,
            image,
            icon,
            on: c.on.as_deref().map(w::SurfaceFace::from_content),
        }
    }
}

impl w::SurfaceFace {
    fn from_content(c: &e::SurfaceContent) -> Self {
        Self {
            label: c.label.clone(),
            label_style: w::LabelStyle {
                position: c.label_style.position.map(Into::into),
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
                fit: (*fit).into(),
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
                mode: settings.knob_lights.mode.into(),
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
    /// Each event has an explicit policy. This runs on the engine thread, in
    /// the sink, and it cannot fail; the copy contains no rendered pixels.
    pub(super) fn apply(&mut self, event: &e::Event) {
        match event {
            e::Event::Layout(layout) => {
                // The engine does not send all state again for each layout
                // (the same model with a different descriptor, for example).
                // Keep what it does not send again, as the window does, and
                // drop only the surfaces that the new layout does not have.
                let layout: w::Layout = layout.as_ref().into();
                self.content
                    .surfaces
                    .retain(|s| layout.surfaces.iter().any(|l| l.id == s.id));
                self.layout = Some(layout);
            }
            e::Event::Kinds(kinds) => {
                // The status comes in its own event: keep it for a kind that
                // is still there.
                let old = std::mem::take(&mut self.kinds);
                self.kinds = kinds
                    .iter()
                    .map(|kind| {
                        let mut kind = w::Kind::from(kind);
                        kind.status = old
                            .iter()
                            .find(|k| k.kind == kind.kind)
                            .and_then(|k| k.status.clone());
                        kind
                    })
                    .collect();
            }
            // Only the client with the pending opaque id gets these choices.
            e::Event::Choices { .. } | e::Event::ProfileRead { .. } => {}
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
                ids,
                location: place,
                first,
                folders,
            } => {
                self.pages = w::Pages {
                    profile: self.profiles.active.clone(),
                    pages: page_summaries(ids, names),
                    location: Some(location(place, ids)),
                    first: page_id(ids, *first),
                    folders: folders
                        .iter()
                        .map(|f| w::Folder {
                            id: f.id.to_string(),
                            name: f.name.clone(),
                        })
                        .collect(),
                };
                // The entry of the active profile follows the pages, as the
                // entry of the engine does. The engine sends the list of the
                // profiles at other times only.
                let active = self.profiles.active.clone();
                if let Some(entry) = self
                    .profiles
                    .profiles
                    .iter_mut()
                    .find(|p| Some(&p.id) == active.as_ref())
                {
                    entry.pages = self.pages.pages.clone();
                    entry.first_page = self.pages.first.clone();
                }
                self.content.location = self.pages.location.clone().unwrap_or_default();
                self.content.bindings.clear();
            }
            e::Event::Binding(binding) => {
                let id = binding.binding;
                let page = self
                    .pages
                    .pages
                    .get(id.page)
                    .map(|p| p.id.clone())
                    .unwrap_or_default();
                let control: w::ControlId = id.control.into();
                let gesture: w::Gesture = id.gesture.into();
                let folder = id.folder.map(|f| f.to_string());
                self.content.bindings.retain(|b| {
                    !(b.page == page
                        && b.folder == folder
                        && b.control == control
                        && b.gesture == gesture)
                });
                if let Some(action) = &binding.action {
                    self.content.bindings.push(w::Binding {
                        page,
                        folder,
                        control,
                        gesture,
                        action: action.into(),
                        available: binding.available,
                        inert_by: binding.inert_by.map(Into::into),
                    });
                }
            }
            e::Event::History { undo, redo } => {
                let history = w::History {
                    profile: self.profiles.active.clone(),
                    undo: undo.clone(),
                    redo: redo.clone(),
                };
                // Keep the names of each profile that was active: the history
                // of a profile that is not active cannot change, thus a read
                // of it needs no engine command.
                match self
                    .histories
                    .iter_mut()
                    .find(|h| h.profile == history.profile)
                {
                    Some(old) => *old = history.clone(),
                    None => self.histories.push(history.clone()),
                }
                self.history = history;
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
                // The place of a surface in the list does not change.
                let wire = w::Surface::from(surface);
                match self.content.surfaces.iter_mut().find(|s| s.id == wire.id) {
                    Some(old) => *old = wire,
                    None => {
                        self.content.surfaces.push(wire);
                        self.content.surfaces.sort_by_key(|s| s.id);
                    }
                }
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

/// True for an event that [`w::State::apply`] never uses. The feed returns
/// before it takes the lock, for example for each input of the device.
pub(super) fn ignored(event: &e::Event) -> bool {
    matches!(
        event,
        e::Event::Input(_)
            | e::Event::Counters(_)
            | e::Event::Clipboard(_)
            | e::Event::Problem(_)
            | e::Event::ProfileExported { .. }
            | e::Event::ProfileImported { .. }
    )
}

/// Explicit command audit. `true` means permitted by decision 0040;
/// `false` means never accepted from a client.
/// This is not a deserializer: the wire enum is the actual allow list.
pub(super) fn permitted(command: &e::Command) -> bool {
    match command {
        e::Command::OnFace(command) => matches!(
            command.as_ref(),
            e::Command::SetLabel { .. }
                | e::Command::SetLabelStyle { .. }
                | e::Command::SetColor { .. }
                | e::Command::SetImage { .. }
                | e::Command::SetIcon { .. }
        ),
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
        e::Command::InProfile { command, .. } | e::Command::AtPlace { command, .. } => {
            permitted(command)
        }
        e::Command::ReadProfile { .. } => true,
        e::Command::Stop => false,
    }
}
