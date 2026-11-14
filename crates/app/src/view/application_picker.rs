//! The list of installed applications (issue 336).
//!
//! A path field of kind Application shows a text field and "Choose…".
//! "Choose…" opens this popover. The popover lists the applications that
//! the shell found, with a search field. The last row, "Choose a different
//! application…", opens the file dialog of a path field.
//!
//! The action stores the path of the application that the user picks. A
//! typed name stays as the user typed it. A later profile link (issue 338)
//! can use the bundle identifier from the same list.

use super::actions::{contained, FieldPath};
use super::*;
use crate::applications::{self, InstalledApplication};
use gpui_kit::{
    anchored, assets::IconName, canvas, component::Icon, deferred, list, point, relative, Bounds,
    KeyDownEvent, ListAlignment, ListState, MouseButton, Pixels,
};
use opencraft_engine::{ApplicationLink, Command, Value};
use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// How many icons to draw before the window paints again.
const ICON_BATCH: usize = 8;

/// Where the list comes from.
enum CatalogSource {
    /// `None` is the usual application folders.
    Directories(Option<Vec<PathBuf>>),
    /// A list that a test supplies. The picker does not scan the disk.
    #[cfg(test)]
    Fixed,
}

/// The control that the picker writes.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PickerTarget {
    /// A path field. The stored text is the path.
    Field(FieldPath),
    /// A profile link. The stored value is the bundle identifier.
    Link { profile: String },
}

/// A drawn icon, or the fact that this path has no icon.
enum CachedIcon {
    Ready(Arc<RenderImage>),
    Missing,
}

/// The popover and the cache of the application list.
pub(super) struct ApplicationPicker {
    target: Option<PickerTarget>,
    search: Entity<InputState>,
    _search_subscription: Subscription,
    source: CatalogSource,
    /// The last scan. A new scan keeps this list until it finishes.
    applications: Vec<InstalledApplication>,
    /// The applications that the search field shows.
    shown: Vec<InstalledApplication>,
    /// The highlighted row. The last index is "Choose a different
    /// application…".
    highlight: usize,
    list: ListState,
    scanning: bool,
    /// A scan that started earlier than this number is ignored.
    generation: u64,
    icons: HashMap<PathBuf, CachedIcon>,
    icon_task: Option<Task<()>>,
    /// The bounds of the button "Choose…" in the last frame.
    button: Rc<Cell<Option<Bounds<Pixels>>>>,
    focus: FocusHandle,
}

impl ApplicationPicker {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search applications"));
        let subscription = cx.subscribe_in(&search, window, |view, _, event, window, cx| {
            if matches!(event, TextEvent::Change) {
                view.refresh_application_list(cx);
                view.application_picker.highlight = 0;
                if !view.application_picker.shown.is_empty() {
                    view.application_picker.list.scroll_to_reveal_item(0);
                }
                view.pump_application_icons(window, cx);
                cx.notify();
            }
        });
        Self {
            target: None,
            search,
            _search_subscription: subscription,
            source: CatalogSource::Directories(None),
            applications: Vec::new(),
            shown: Vec::new(),
            highlight: 0,
            list: ListState::new(0, ListAlignment::Top, theme::HEIGHT_MENU_ROW * 4.),
            scanning: false,
            generation: 0,
            icons: HashMap::new(),
            icon_task: None,
            button: Rc::new(Cell::new(None)),
            focus: cx.focus_handle(),
        }
    }
}

/// The place of the popover in the window. The trailing edge lines up with
/// the button. The popover is below the button when it fits there.
struct Place {
    origin: gpui_kit::Point<Pixels>,
    height: Pixels,
}

fn place(button: Bounds<Pixels>, viewport: gpui_kit::Size<Pixels>) -> Place {
    let margin = theme::ICON_PICKER_WINDOW_MARGIN;
    let width = theme::APPLICATION_PICKER_WIDTH;
    let gap = theme::SPACE_4;
    let below_space = viewport.height - margin - (button.bottom() + gap);
    let above_space = button.top() - gap - margin;
    let full = theme::APPLICATION_PICKER_HEIGHT;
    let below = below_space >= full || (above_space < full && below_space >= above_space);
    let height = full
        .min(if below { below_space } else { above_space })
        .max(theme::APPLICATION_PICKER_MIN_HEIGHT);
    let left = (button.right() - width)
        .max(margin)
        .min((viewport.width - margin - width).max(margin));
    let top = if below {
        button.bottom() + gap
    } else {
        button.top() - gap - height
    };
    Place {
        origin: point(left, top),
        height,
    }
}

impl DeviceView {
    /// Use `applications` and do not scan. A window test calls this before
    /// it opens the picker.
    #[cfg(test)]
    pub(super) fn use_fixed_applications(
        &mut self,
        applications: Vec<InstalledApplication>,
        cx: &mut Context<Self>,
    ) {
        self.application_picker.generation = self.application_picker.generation.wrapping_add(1);
        self.application_picker.scanning = false;
        self.application_picker.source = CatalogSource::Fixed;
        self.application_picker.applications = applications;
        if self.application_picker.target.is_some() {
            self.refresh_application_list(cx);
            self.highlight_current_application();
        }
        cx.notify();
    }

    /// Scan `directories` the next time the picker opens.
    #[cfg(test)]
    pub(super) fn use_application_directories(&mut self, directories: Vec<PathBuf>) {
        self.application_picker.generation = self.application_picker.generation.wrapping_add(1);
        self.application_picker.scanning = false;
        self.application_picker.source = CatalogSource::Directories(Some(directories));
        self.application_picker.applications.clear();
        self.application_picker.shown.clear();
    }

    pub(super) fn application_picker_is_open(&self) -> bool {
        self.application_picker.target.is_some()
    }

    /// True when the list is open to add a link to `profile`.
    pub(super) fn link_picker_open(&self, profile: &str) -> bool {
        matches!(
            &self.application_picker.target,
            Some(PickerTarget::Link { profile: id }) if id == profile
        )
    }

    /// "Add an application…" and the list, when the list is open for `profile`.
    pub(super) fn add_application_control(
        &self,
        profile: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = profile.to_string();
        let adding = self.link_picker_open(profile);
        let bounds = self.application_picker.button.clone();
        div()
            .relative()
            .flex_shrink_0()
            .child(
                controls::button("profile-add-application", "Add an application…")
                    .when(adding, |row| row.border_color(theme::ACCENT_LINE))
                    .aria_expanded(adding)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        if view.link_picker_open(&id) {
                            view.dismiss_application_picker(window, cx);
                        } else {
                            view.open_link_picker(id.clone(), window, cx);
                        }
                    })),
            )
            .child(
                canvas(move |b, _, _| bounds.set(Some(b)), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .when(adding, |row| {
                row.child(self.application_popover(window, cx))
            })
            .into_any_element()
    }

    /// The names that the open picker shows, in order.
    #[cfg(test)]
    pub(super) fn shown_application_names(&self) -> Vec<String> {
        self.application_picker
            .shown
            .iter()
            .map(|application| application.name.clone())
            .collect()
    }

    /// "Choose…" for a path field of kind Application, and the popover when
    /// it is open for this field.
    pub(super) fn application_choose(
        &self,
        id: &str,
        index: usize,
        field: &FieldPath,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open =
            self.application_picker.target.as_ref() == Some(&PickerTarget::Field(field.clone()));
        let field_for_click = field.clone();
        let bounds = self.application_picker.button.clone();
        div()
            .relative()
            .flex_shrink_0()
            .child(
                button(format!("{id}path-{index}"), "Choose…")
                    .px(theme::SPACE_10)
                    .when(open, |row| row.border_color(theme::ACCENT_LINE))
                    .aria_expanded(open)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        if view.application_picker.target.as_ref()
                            == Some(&PickerTarget::Field(field_for_click.clone()))
                        {
                            view.close_application_picker(window, cx);
                        } else {
                            view.open_application_picker(field_for_click.clone(), window, cx);
                        }
                    })),
            )
            .child(
                canvas(move |b, _, _| bounds.set(Some(b)), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .when(open, |row| row.child(self.application_popover(window, cx)))
            .into_any_element()
    }

    /// The icon of the application that `stored` names, when the list has
    /// it. The text field shows this icon.
    pub(super) fn application_field_icon(&self, stored: &str) -> Option<AnyElement> {
        let application = self
            .application_picker
            .applications
            .iter()
            .find(|application| application.matches_stored(stored))?;
        Some(self.application_icon_tile(application).into_any_element())
    }

    /// Open the list for a new link of the profile `profile`.
    pub(super) fn open_link_picker(
        &mut self,
        profile: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sheet.pages_open = false;
        self.begin_application_picker(PickerTarget::Link { profile }, window, cx);
    }

    /// Escape or Return while the list is open. `true` when the list took
    /// the key, so a sheet does not close.
    pub(super) fn application_picker_key(
        &mut self,
        key: super::modal::ModalKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.application_picker_is_open() {
            return false;
        }
        match key {
            super::modal::ModalKey::Escape => self.close_application_picker(window, cx),
            super::modal::ModalKey::Return => self.choose_highlighted_application(window, cx),
        }
        true
    }

    /// Close the list if it is open.
    pub(super) fn dismiss_application_picker(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_application_picker(window, cx);
    }

    /// Start a scan so the profile sheet can show icons. A test that uses
    /// a fixed list calls [`Self::use_fixed_applications`] instead.
    pub(super) fn prepare_applications(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.start_application_scan(window, cx);
    }

    /// The icon of a link: the application icon when the list has it, or
    /// the first letter of `name`.
    pub(super) fn link_icon(&self, bundle_id: &str, name: &str) -> AnyElement {
        let found = self
            .application_picker
            .applications
            .iter()
            .find(|application| {
                !application.bundle_identifier.is_empty()
                    && opencraft_engine::front::same_bundle(
                        &application.bundle_identifier,
                        bundle_id,
                    )
            });
        if let Some(application) = found {
            return self.application_icon_tile(application).into_any_element();
        }
        self.application_icon_tile(&InstalledApplication {
            name: name.to_string(),
            bundle_identifier: bundle_id.to_string(),
            path: PathBuf::from(name),
        })
        .into_any_element()
    }

    fn open_application_picker(
        &mut self,
        field: FieldPath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.begin_application_picker(PickerTarget::Field(field), window, cx);
    }

    fn begin_application_picker(
        &mut self,
        target: PickerTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_kinds();
        self.application_picker.target = Some(target);
        let search = self.application_picker.search.clone();
        search.update(cx, |search, cx| {
            search.set_value("", window, cx);
            search.focus(window, cx);
        });
        self.refresh_application_list(cx);
        self.highlight_current_application();
        self.start_application_scan(window, cx);
        self.pump_application_icons(window, cx);
        cx.notify();
    }

    fn close_application_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.application_picker.target.is_none() {
            return;
        }
        self.application_picker.target = None;
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// Drop the picker when another menu opens or the form changes. The
    /// caller gives the focus back when a key closed the picker.
    pub(super) fn clear_application_picker(&mut self) {
        self.application_picker.target = None;
    }

    fn start_application_scan(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.application_picker.scanning {
            return;
        }
        #[cfg(test)]
        if matches!(self.application_picker.source, CatalogSource::Fixed) {
            return;
        }
        let directories = match &self.application_picker.source {
            CatalogSource::Directories(Some(directories)) => directories.clone(),
            CatalogSource::Directories(None) => applications::standard_directories(),
            #[cfg(test)]
            CatalogSource::Fixed => return,
        };
        self.application_picker.scanning = true;
        let generation = self.application_picker.generation;
        cx.spawn_in(window, async move |view, cx| {
            let found = cx
                .background_executor()
                .spawn(async move { applications::scan(&directories) })
                .await;
            let _ = view.update_in(cx, |view, window, cx| {
                if view.application_picker.generation != generation {
                    return;
                }
                view.application_picker.scanning = false;
                view.application_picker.applications = found;
                view.refresh_application_list(cx);
                if view
                    .application_picker
                    .search
                    .read(cx)
                    .value()
                    .trim()
                    .is_empty()
                {
                    view.highlight_current_application();
                }
                view.pump_application_icons(window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn refresh_application_list(&mut self, cx: &App) {
        let query = self.application_picker.search.read(cx).value().to_string();
        self.application_picker.shown =
            applications::filter(&self.application_picker.applications, &query);
        let last = self.application_picker.shown.len();
        if self.application_picker.highlight > last {
            self.application_picker.highlight = 0;
        }
        self.application_picker
            .list
            .reset(self.application_picker.shown.len());
    }

    fn highlight_current_application(&mut self) {
        let Some(stored) = self.stored_application_text() else {
            return;
        };
        if let Some(index) = self
            .application_picker
            .shown
            .iter()
            .position(|application| application.matches_stored(&stored))
        {
            self.application_picker.highlight = index;
            self.application_picker.list.scroll_to_reveal_item(index);
        }
    }

    fn stored_application_text(&self) -> Option<String> {
        let PickerTarget::Field(field) = self.application_picker.target.as_ref()? else {
            return None;
        };
        let binding = self.form_binding()?;
        let action = self.edit_action(binding)?;
        let action = contained(&action, &field.action)?;
        match action.settings.get(&field.key) {
            Some(Value::Text(text)) if !text.trim().is_empty() => Some(text.clone()),
            _ => None,
        }
    }

    fn move_application_highlight(&mut self, delta: isize) {
        let count = self.application_picker.shown.len() + 1;
        let current = self.application_picker.highlight as isize;
        let next = (current + delta).rem_euclid(count as isize) as usize;
        self.application_picker.highlight = next;
        if next < self.application_picker.shown.len() {
            self.application_picker.list.scroll_to_reveal_item(next);
        }
    }

    fn choose_highlighted_application(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.application_picker.highlight;
        if index >= self.application_picker.shown.len() {
            self.choose_different_application(window, cx);
            return;
        }
        let Some(application) = self.application_picker.shown.get(index).cloned() else {
            return;
        };
        self.accept_installed_application(application, window, cx);
    }

    fn accept_installed_application(
        &mut self,
        application: InstalledApplication,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = self.application_picker.target.clone() else {
            return;
        };
        match target {
            PickerTarget::Field(field) => {
                let Some(binding) = self.form_binding() else {
                    return;
                };
                self.cache_application_icon(&application.path);
                self.set_field(
                    binding,
                    &field,
                    Value::Text(application.path.to_string_lossy().into_owned()),
                );
                self.sync_form(window, cx);
                self.close_application_picker(window, cx);
            }
            PickerTarget::Link { profile } => {
                self.add_profile_link(profile, &application);
                self.close_application_picker(window, cx);
            }
        }
    }

    /// Send the link. An application with no bundle identifier cannot be a
    /// link: the engine compares bundle identifiers.
    fn add_profile_link(&mut self, profile: String, application: &InstalledApplication) {
        if application.bundle_identifier.is_empty() {
            self.state.problems.push(
                "OpenCraft did not add the link, because the application has no bundle \
                 identifier."
                    .into(),
            );
            return;
        }
        (self.send)(Command::LinkApplication {
            profile,
            link: ApplicationLink {
                bundle_id: application.bundle_identifier.clone(),
                name: application.name.clone(),
            },
        });
    }

    fn choose_different_application(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.application_picker.target.clone() else {
            return;
        };
        self.close_application_picker(window, cx);
        match target {
            PickerTarget::Field(field) => self.path_field(field, window, cx),
            PickerTarget::Link { profile } => self.choose_application_file(profile, window, cx),
        }
    }

    /// The file dialog of "Choose a different application…" for a link.
    fn choose_application_file(
        &mut self,
        profile: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let paths = cx.prompt_for_paths(gpui_kit::PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = paths.await;
            let _ = view.update_in(cx, |view, window, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.first() {
                            let application = applications::application_at(path.clone());
                            view.add_profile_link(profile, &application);
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => view
                        .state
                        .problems
                        .push("OpenCraft could not open the file dialog.".into()),
                }
                if view.sheet.profile.is_some() {
                    window.focus(&view.sheet.focus, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn application_popover(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let viewport = window.viewport_size();
        let Some(button) = self.application_picker.button.get() else {
            window.request_animation_frame();
            return div().into_any_element();
        };
        let place = place(button, viewport);
        let close = cx.listener(|view, _: &gpui_kit::MouseDownEvent, window, cx| {
            view.close_application_picker(window, cx);
        });
        div()
            .child(
                deferred(
                    anchored().position(point(px(0.), px(0.))).child(
                        div()
                            .id("application-picker-backdrop")
                            .w(viewport.width)
                            .h(viewport.height)
                            .occlude()
                            .on_mouse_down(MouseButton::Left, close),
                    ),
                )
                .with_priority(1),
            )
            .child(
                deferred(
                    anchored().position(place.origin).child(
                        div()
                            .id("application-picker")
                            .debug_selector(|| "application-picker".into())
                            .accessible(Role::Dialog, "Choose an application")
                            .occlude()
                            .relative()
                            .w(theme::APPLICATION_PICKER_WIDTH)
                            .h(place.height)
                            .capture_key_down(cx.listener(
                                |view, event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.modifiers.modified() {
                                        return;
                                    }
                                    match event.keystroke.key.as_str() {
                                        "escape" => {
                                            cx.stop_propagation();
                                            view.close_application_picker(window, cx);
                                        }
                                        "up" => {
                                            cx.stop_propagation();
                                            view.move_application_highlight(-1);
                                            cx.notify();
                                        }
                                        "down" => {
                                            cx.stop_propagation();
                                            view.move_application_highlight(1);
                                            cx.notify();
                                        }
                                        "enter" => {
                                            cx.stop_propagation();
                                            view.choose_highlighted_application(window, cx);
                                        }
                                        _ => {}
                                    }
                                },
                            ))
                            .child(
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .rounded(theme::RADIUS_POPOVER)
                                    .bg(theme::BG_FIELD)
                                    .border_1()
                                    .border_color(theme::BORDER_OVERLAY)
                                    .shadow(theme::popover_shadow())
                                    .text_size(theme::TEXT_CONTROL)
                                    .font_weight(gpui_kit::FontWeight::NORMAL)
                                    .text_color(theme::TEXT_PRIMARY)
                                    .child(self.application_picker_search(window, cx))
                                    .child(self.application_list(cx))
                                    .child(self.application_picker_footer(cx)),
                            )
                            .map(|popover| {
                                controls::popover_focus(
                                    popover,
                                    "application-picker-trap".into(),
                                    &self.application_picker.focus,
                                )
                            }),
                    ),
                )
                .with_priority(2),
            )
            .into_any_element()
    }

    fn application_picker_search(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        div()
            .flex_shrink_0()
            .p(theme::SPACE_12)
            .border_b_1()
            .border_color(theme::BORDER_FIELD)
            .child(
                controls::search_field(
                    "application-search",
                    "Search applications",
                    &self.application_picker.search,
                    theme::HEIGHT_SEARCH_FIELD,
                    window,
                    cx,
                )
                .bg(theme::BG_BAR),
            )
    }

    fn application_list(&self, cx: &mut Context<Self>) -> Div {
        let frame = div().flex_1().min_h_0().relative();
        if self.application_picker.shown.is_empty() {
            let text = self.application_empty_text(cx);
            return frame.child(
                div()
                    .id("application-empty")
                    .debug_selector(|| "application-empty".into())
                    .pt(theme::EMPTY_STATE_TOP)
                    .px(theme::SPACE_16)
                    .text_center()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_TERTIARY)
                    .readable_text(text.clone())
                    .child(text),
            );
        }
        let shown = self.application_picker.shown.clone();
        let view = cx.entity();
        frame
            .child(
                list(self.application_picker.list.clone(), move |ix, _, cx| {
                    let application = shown[ix].clone();
                    view.update(cx, |view, cx| view.application_row(ix, &application, cx))
                })
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(Scrollbar::vertical(&self.application_picker.list)),
            )
    }

    /// The caption "Linked to “Work”" when the picker adds a link, and a
    /// different profile has the link of this application that applies. A
    /// pick moves that link (decision 0035, section 5). The profile of the
    /// sheet shows no caption for its own link.
    fn linked_profile_note(&self, application: &InstalledApplication) -> Option<String> {
        let Some(PickerTarget::Link { profile }) = &self.application_picker.target else {
            return None;
        };
        if application.bundle_identifier.is_empty() {
            return None;
        }
        let owner = link_owner(&self.state.profiles, &application.bundle_identifier)?;
        (owner.id != *profile).then(|| format!("Linked to “{}”", owner.name))
    }

    /// The caption of each row that the open picker shows, in order.
    #[cfg(test)]
    pub(super) fn shown_application_links(&self) -> Vec<Option<String>> {
        self.application_picker
            .shown
            .iter()
            .map(|application| self.linked_profile_note(application))
            .collect()
    }

    fn application_empty_text(&self, cx: &App) -> String {
        let query = self.application_picker.search.read(cx).value();
        let query = query.trim();
        if !query.is_empty() {
            return format!("No application matches “{query}”.");
        }
        if self.application_picker.scanning && self.application_picker.applications.is_empty() {
            return "Looking for applications…".into();
        }
        "No application is in the list.".into()
    }

    fn application_row(
        &self,
        index: usize,
        application: &InstalledApplication,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let highlighted = self.application_picker.highlight == index;
        let stored = self.stored_application_text().unwrap_or_default();
        let current = application.matches_stored(&stored);
        let name = application.name.clone();
        // Before a link moves, the row names the profile that has it
        // (decision 0035, section 5). A path field does not move a link.
        let note = self.linked_profile_note(application);
        let label = match &note {
            Some(note) => format!("{name}, {note}"),
            None => name.clone(),
        };
        let row_id = format!("app-row-{index}");
        let selector = row_id.clone();
        div()
            .id(SharedString::from(row_id))
            .debug_selector(move || selector)
            .w_full()
            .h(theme::HEIGHT_MENU_ROW)
            .mx(theme::SPACE_6)
            .px(theme::SPACE_8)
            .flex()
            .items_center()
            .gap(theme::SPACE_8)
            .rounded(theme::RADIUS_CONTROL)
            .cursor_pointer()
            .when(highlighted, |row| row.bg(theme::MENU_HIGHLIGHT))
            .accessible(Role::MenuItem, label)
            .child(self.application_icon_tile(application))
            .child(div().flex_1().min_w_0().truncate().child(name))
            .when_some(note, |row, note| {
                let selector = format!("app-row-linked-{index}");
                row.child(
                    div()
                        .debug_selector(move || selector)
                        .flex_shrink_0()
                        .max_w(relative(0.55))
                        .truncate()
                        .text_size(theme::TEXT_CAPTION)
                        .text_color(theme::TEXT_TERTIARY)
                        .child(note),
                )
            })
            .child(self.application_check(current))
            .on_mouse_move(cx.listener(move |view, _, _, cx| {
                if view.application_picker.highlight != index {
                    view.application_picker.highlight = index;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |view, _, window, cx| {
                let Some(application) = view.application_picker.shown.get(index).cloned() else {
                    return;
                };
                view.accept_installed_application(application, window, cx);
            }))
            .into_any_element()
    }

    fn application_picker_footer(&self, cx: &mut Context<Self>) -> Div {
        let index = self.application_picker.shown.len();
        let highlighted = self.application_picker.highlight == index;
        div()
            .flex_shrink_0()
            .pt(theme::SPACE_4)
            .pb(theme::SPACE_6)
            .border_t_1()
            .border_color(theme::BORDER_FIELD)
            .child(
                div()
                    .id("choose-different-application")
                    .debug_selector(|| "choose-different-application".into())
                    .w_full()
                    .h(theme::HEIGHT_MENU_ROW)
                    .mx(theme::SPACE_6)
                    .px(theme::SPACE_8)
                    .flex()
                    .items_center()
                    .rounded(theme::RADIUS_CONTROL)
                    .cursor_pointer()
                    .when(highlighted, |row| row.bg(theme::MENU_HIGHLIGHT))
                    .accessible(Role::MenuItem, "Choose a different application…")
                    .child("Choose a different application…")
                    .on_mouse_move(cx.listener(move |view, _, _, cx| {
                        if view.application_picker.highlight != index {
                            view.application_picker.highlight = index;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.choose_different_application(window, cx);
                    })),
            )
    }

    fn application_check(&self, current: bool) -> Div {
        div()
            .flex_shrink_0()
            .w(theme::ICON_SMALL)
            .when(current, |mark| {
                mark.child(
                    Icon::new(IconName::Check)
                        .size(theme::ICON_SMALL)
                        .text_color(theme::TEXT_SECONDARY),
                )
            })
    }

    fn application_icon_tile(&self, application: &InstalledApplication) -> Div {
        let tile = div()
            .flex_shrink_0()
            .size(theme::ICON_APPLICATION)
            .rounded(theme::RADIUS_BADGE)
            .overflow_hidden();
        match self.application_picker.icons.get(&application.path) {
            Some(CachedIcon::Ready(image)) => tile.child(
                img(image.clone())
                    .size(theme::ICON_APPLICATION)
                    .rounded(theme::RADIUS_BADGE),
            ),
            _ => {
                let letter = application
                    .name
                    .chars()
                    .next()
                    .map(|character| character.to_uppercase().to_string())
                    .unwrap_or_else(|| "A".into());
                tile.flex()
                    .items_center()
                    .justify_center()
                    .bg(theme::BG_CONTROL)
                    .text_size(theme::TEXT_CAPTION)
                    .text_color(theme::TEXT_SECONDARY)
                    .child(letter)
            }
        }
    }

    /// Draw the next icons on the user interface thread. AppKit draws an
    /// icon there. A short pause between batches lets the window paint.
    fn pump_application_icons(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.application_picker.icon_task.is_some() {
            return;
        }
        let paths: Vec<PathBuf> = {
            let picker = &self.application_picker;
            picker
                .applications
                .iter()
                .filter(|application| !picker.icons.contains_key(&application.path))
                .take(ICON_BATCH)
                .map(|application| application.path.clone())
                .collect()
        };
        if paths.is_empty() {
            return;
        }
        for path in &paths {
            self.cache_application_icon(path);
        }
        cx.notify();
        let pending = self
            .application_picker
            .applications
            .iter()
            .any(|application| {
                !self
                    .application_picker
                    .icons
                    .contains_key(&application.path)
            });
        if !pending {
            return;
        }
        let task = cx.spawn_in(window, async move |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1))
                .await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.application_picker.icon_task = None;
                view.pump_application_icons(window, cx);
            });
        });
        self.application_picker.icon_task = Some(task);
    }

    fn cache_application_icon(&mut self, path: &Path) {
        if self.application_picker.icons.contains_key(path) {
            return;
        }
        // A missing path has no icon. A test uses those paths, and AppKit
        // is not called for them.
        let icon = if path.exists() {
            match applications::application_icon(path) {
                Some(image) => CachedIcon::Ready(images::from_rgba(&image)),
                None => CachedIcon::Missing,
            }
        } else {
            CachedIcon::Missing
        };
        self.application_picker
            .icons
            .insert(path.to_path_buf(), icon);
    }
}

/// The profile whose link of `bundle_id` applies. A link in `clashes` does
/// not apply, so the first profile that links the application and has no
/// clash for it is the owner (decision 0035, section 5).
pub(super) fn link_owner<'a>(
    profiles: &'a [opencraft_engine::ProfileEntry],
    bundle_id: &str,
) -> Option<&'a opencraft_engine::ProfileEntry> {
    use opencraft_engine::front::same_bundle;
    profiles.iter().find(|entry| {
        entry.applications.iter().any(|link| {
            same_bundle(&link.bundle_id, bundle_id)
                && !entry
                    .clashes
                    .iter()
                    .any(|clash| same_bundle(clash, bundle_id))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_popover_goes_below_the_button_or_above_it() {
        let viewport = gpui_kit::size(px(1440.), px(900.));
        let button = Bounds::new(
            point(px(1200.), px(200.)),
            gpui_kit::size(px(120.), theme::HEIGHT_CONTROL),
        );
        let below = place(button, viewport);
        assert_eq!(below.height, theme::APPLICATION_PICKER_HEIGHT);
        assert_eq!(below.origin.y, button.bottom() + theme::SPACE_4);
        assert_eq!(
            below.origin.x + theme::APPLICATION_PICKER_WIDTH,
            button.right()
        );
        let button = Bounds::new(
            point(px(1100.), px(760.)),
            gpui_kit::size(px(120.), theme::HEIGHT_CONTROL),
        );
        let above = place(button, viewport);
        assert_eq!(above.origin.y + above.height, button.top() - theme::SPACE_4);
        let viewport = gpui_kit::size(px(1080.), px(700.));
        let button = Bounds::new(
            point(px(800.), px(340.)),
            gpui_kit::size(px(120.), theme::HEIGHT_CONTROL),
        );
        let small = place(button, viewport);
        assert!(small.height < theme::APPLICATION_PICKER_HEIGHT);
        assert!(small.origin.y >= theme::ICON_PICKER_WINDOW_MARGIN);
        assert!(
            small.origin.y + small.height <= viewport.height - theme::ICON_PICKER_WINDOW_MARGIN
        );
    }
}
