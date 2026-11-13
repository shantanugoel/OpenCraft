//! The sidebar on the leading side (decision 0029, section 7).
//!
//! From the top: the button that hides the sidebar, the device card, the
//! profiles with the pages of the active profile, and the bottom group with
//! "Settings". The rows are plain rows (`docs/kb/gpui.md`, "Sidebar with a
//! tree of two levels"): a click selects, the disclosure triangle alone
//! hides or shows the pages, a double click edits the name in the row, and
//! a right click opens a native menu. The work of each row is in `pages`.

use super::{
    controls::{dot, icon_button, keyboard_focus, Ring},
    pages::{in_profile, page_title, MenuItem, NameEdit, Row},
    *,
};
use crate::{app_menu::OpenSettings, shell::ShellCommand};
use gpui_kit::{
    actions,
    assets::IconName,
    component::{native_menu::NativeMenu, tooltip::Tooltip, Icon, Sizable},
    percentage, Action, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent, Pixels,
};

// The items of the right-click menus. A native menu item sends its action
// to the focused element, thus the root element of the window handles them
// for the row of the menu (`Lists::menu_row`).
actions!(
    sidebar,
    [
        RenameRow,
        DuplicateRow,
        MoveRowUp,
        MoveRowDown,
        RemoveRow,
        ProfileSettingsRow,
        SelectDevice
    ]
);

impl MenuItem {
    fn action(self) -> Box<dyn Action> {
        match self {
            Self::Rename => Box::new(RenameRow),
            Self::Duplicate => Box::new(DuplicateRow),
            Self::MoveUp => Box::new(MoveRowUp),
            Self::MoveDown => Box::new(MoveRowDown),
            Self::ProfileSettings => Box::new(ProfileSettingsRow),
            Self::Separator | Self::Remove => Box::new(RemoveRow),
        }
    }
}

/// The section title above the profiles, and the accessible label of the
/// tree of the rows.
const PROFILES: &str = "Profiles";

/// The badge of the active profile.
const ON_DEVICE: &str = "ON DEVICE";

/// The tooltip of "ON DEVICE" (decision 0035, section 11).
fn cause_text(
    cause: &opencraft_engine::ActiveCause,
    profiles: &[opencraft_engine::ProfileEntry],
) -> String {
    match cause {
        opencraft_engine::ActiveCause::Selected => "Selected".into(),
        opencraft_engine::ActiveCause::Linked { bundle_id } => {
            let name = profiles
                .iter()
                .find_map(|entry| {
                    entry.applications.iter().find_map(|link| {
                        opencraft_engine::front::same_bundle(&link.bundle_id, bundle_id)
                            .then(|| link.name.clone())
                    })
                })
                .unwrap_or_else(|| bundle_id.clone());
            format!("Because “{name}” is in front")
        }
        opencraft_engine::ActiveCause::Default => {
            "The default profile: the front application has no linked profile".into()
        }
    }
}

/// The mark of a profile that is not active: the first link that applies,
/// with "+N" for more links, and "DEFAULT" for the default profile.
fn profile_note(
    profile: &opencraft_engine::ProfileEntry,
    default_id: Option<&str>,
) -> Option<String> {
    let links: Vec<_> = profile
        .applications
        .iter()
        .filter(|link| {
            !profile
                .clashes
                .iter()
                .any(|clash| opencraft_engine::front::same_bundle(clash, &link.bundle_id))
        })
        .collect();
    let links = links.first().map(|link| {
        if links.len() == 1 {
            link.name.clone()
        } else {
            format!("{} +{}", link.name, links.len() - 1)
        }
    });
    let default = (default_id == Some(profile.id.as_str())).then(|| "DEFAULT".to_string());
    match (links, default) {
        (Some(links), Some(mark)) => Some(format!("{links} {mark}")),
        (Some(links), None) => Some(links),
        (None, Some(mark)) => Some(mark),
        (None, None) => None,
    }
}

/// The tooltip of the "+" button of the profiles.
const ADD_A_PROFILE: &str = "Add a profile";

/// The row below the pages.
const ADD_A_PAGE: &str = "Add a page";

/// The row below "Add a page".
const ADD_A_FOLDER: &str = "Add a folder";

/// The row of the bottom group.
const SETTINGS: &str = "Settings";

/// The tooltip of the chevrons of the device card.
const DEVICE_SWITCHER: &str = "Choose a device";

/// The parts of a row of the sidebar that all rows have: height 28, radius
/// 6, and the background under the pointer.
fn row(id: impl Into<SharedString>, selected: bool) -> Stateful<Div> {
    let id: SharedString = id.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .h(theme::HEIGHT_CONTROL)
        .flex()
        .items_center()
        .rounded(theme::RADIUS_CONTROL)
        .when(!selected, |d| d.hover(|d| d.bg(theme::HOVER_ROW)))
}

/// The text of a row. It takes the free width, and a long text ends with
/// "…".
fn row_text(text: impl Into<SharedString>) -> Div {
    div().flex_1().min_w_0().truncate().child(text.into())
}

impl DeviceView {
    pub(super) fn sidebar(&self, width: Pixels, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("sidebar")
            .debug_selector(|| "sidebar".into())
            .accessible(Role::Navigation, "Devices, profiles and pages")
            .flex_shrink_0()
            .w(width)
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::BG_SIDEBAR)
            .border_r_1()
            .border_color(theme::LINE_PANE)
            .child(
                // The window buttons are at the leading side of this area.
                div()
                    .flex_shrink_0()
                    .h(theme::TOOLBAR_HEIGHT)
                    .pr(theme::SPACE_12)
                    .flex()
                    .items_center()
                    .child(self.moves_window(div().id("sidebar-top").flex_1().h_full(), cx))
                    .child(
                        icon_button("hide-sidebar", IconName::PanelLeft, "Hide the sidebar")
                            .on_click(cx.listener(|view, _, _, cx| view.toggle_sidebar(cx))),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .pt(theme::SPACE_2)
                    .px(theme::SPACE_10)
                    .pb(theme::SPACE_8)
                    .child(self.device_card(cx)),
            )
            .child(self.profiles_header(cx))
            .child(
                div()
                    .id("sidebar-rows")
                    // The rows are a tree of two levels: the profiles, and
                    // the pages of the active profile.
                    .accessible(Role::Tree, PROFILES)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(self.profile_rows(cx)),
            )
            .child(self.bottom_group(cx))
            .into_any_element()
    }

    /// Add the handlers of the menu items to the root element of the window.
    pub(super) fn sidebar_actions(
        &self,
        root: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        root.on_action(cx.listener(|view, _: &RenameRow, window, cx| {
            view.menu_choice(MenuItem::Rename, window, cx)
        }))
        .on_action(cx.listener(|view, _: &DuplicateRow, window, cx| {
            view.menu_choice(MenuItem::Duplicate, window, cx)
        }))
        .on_action(cx.listener(|view, _: &MoveRowUp, window, cx| {
            view.menu_choice(MenuItem::MoveUp, window, cx)
        }))
        .on_action(cx.listener(|view, _: &MoveRowDown, window, cx| {
            view.menu_choice(MenuItem::MoveDown, window, cx)
        }))
        .on_action(cx.listener(|view, _: &RemoveRow, window, cx| {
            view.menu_choice(MenuItem::Remove, window, cx)
        }))
        .on_action(cx.listener(|view, _: &ProfileSettingsRow, window, cx| {
            view.menu_choice(MenuItem::ProfileSettings, window, cx)
        }))
        .on_action(cx.listener(|view, _: &SelectDevice, window, cx| view.select_device(window, cx)))
    }

    /// Open the right-click menu of `row` at the pointer.
    fn open_menu(
        &mut self,
        row: Row,
        shown: Option<String>,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The menu sends its action to the focus: the root element.
        self.focus_workspace(window, cx);
        let mut menu = NativeMenu::new();
        for (item, enabled) in self.menu_items(&row) {
            menu = if item == MenuItem::Separator {
                menu.separator()
            } else {
                menu.menu_with_disabled(item.label(), !enabled, item.action())
            };
        }
        self.lists.menu_row = Some(row);
        self.lists.menu_profile = shown;
        menu.show(event.position, window, cx);
    }

    /// The device: an icon, the model name, the connection state, and the
    /// device switcher. A click selects the device.
    fn device_card(&self, cx: &mut Context<Self>) -> AnyElement {
        let connected = self.state.connected;
        let selected = self.device.open;
        let name = self.device_name();
        let state = if connected {
            "Connected"
        } else {
            "Not connected"
        };
        div()
            .id("device-card")
            .debug_selector(|| "device-card".into())
            // The label of decision 0029, section 7.2.
            .accessible(
                Role::Button,
                format!(
                    "Device {name}, {}. Select to show the device settings.",
                    state.to_lowercase()
                ),
            )
            .aria_selected(selected)
            .p(theme::SPACE_8)
            .flex()
            .items_center()
            .gap(theme::SPACE_10)
            .rounded(theme::RADIUS_DEVICE_CARD)
            .border_1()
            .cursor_pointer()
            .map(|d| {
                if selected {
                    d.bg(theme::ACCENT)
                        .border_color(theme::ACCENT_LINE)
                        .text_color(theme::TEXT_ON_ACCENT)
                } else {
                    d.bg(theme::BG_FIELD)
                        .border_color(theme::BORDER_DEVICE_CARD)
                        .hover(|d| d.bg(theme::BG_CONTROL))
                }
            })
            .map(|d| keyboard_focus(d, Ring::Outside))
            .on_click(cx.listener(|view, _, window, cx| view.select_device(window, cx)))
            // A narrow sidebar shows only the start of a long model name.
            .tooltip({
                let name = name.clone();
                move |window, cx| Tooltip::new(name.clone()).build(window, cx)
            })
            .child(
                div()
                    .flex_shrink_0()
                    .size(theme::DEVICE_TILE)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme::RADIUS_CARD)
                    .bg(theme::BG_TILE)
                    .text_color(theme::TEXT_PRIMARY)
                    .child(Icon::new(IconName::Grid3x2).size(theme::ICON_DEVICE)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_2)
                    .child(
                        div()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(name),
                    )
                    .child(
                        div()
                            .id("device-state")
                            .debug_selector(|| "device-state".into())
                            .flex()
                            .items_center()
                            .gap(theme::SPACE_6)
                            .text_size(theme::TEXT_CAPTION)
                            .text_color(if selected {
                                theme::TEXT_ON_ACCENT
                            } else {
                                theme::TEXT_SECONDARY
                            })
                            .child(dot(if connected {
                                theme::SUCCESS
                            } else {
                                theme::TEXT_DISABLED
                            }))
                            .child(state),
                    ),
            )
            .child(
                div()
                    .id("device-switcher")
                    .debug_selector(|| "device-switcher".into())
                    .accessible(Role::Button, DEVICE_SWITCHER)
                    .tooltip(|window, cx| Tooltip::new(DEVICE_SWITCHER).build(window, cx))
                    .cursor_pointer()
                    .flex_shrink_0()
                    .text_color(if selected {
                        theme::TEXT_ON_ACCENT
                    } else {
                        theme::TEXT_SECONDARY
                    })
                    .child(Icon::new(IconName::ChevronsUpDown).size(theme::ICON_BUTTON))
                    // The engine knows one device now. More devices are a
                    // LATER place (section 21).
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|view, event: &MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            view.focus_workspace(window, cx);
                            NativeMenu::new()
                                .menu_with_check(view.device_name(), true, Box::new(SelectDevice))
                                .show(event.position, window, cx);
                        }),
                    ),
            )
            .into_any_element()
    }

    /// The model name from the layout of the engine (rule 4).
    pub(super) fn device_name(&self) -> String {
        self.state
            .layout
            .as_ref()
            .map_or_else(|| "Device".to_string(), |l| l.name.clone())
    }

    /// "PROFILES", and the menu that adds or imports a profile.
    fn profiles_header(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex_shrink_0()
            .pt(theme::SPACE_12)
            .pr(theme::SPACE_12)
            .pb(theme::SPACE_6)
            .pl(theme::SPACE_18)
            .flex()
            .items_center()
            .child(controls::section_title(PROFILES).flex_1())
            .child(
                div()
                    .id("profile-add")
                    .debug_selector(|| "profile-add".into())
                    .flex_shrink_0()
                    .w(theme::SIDEBAR_ADD_WIDTH)
                    .h(theme::SIDEBAR_ADD_HEIGHT)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme::RADIUS_SEGMENT)
                    .text_color(theme::TEXT_SECONDARY)
                    .cursor_pointer()
                    .hover(|d| d.bg(theme::HOVER_ICON))
                    .active(|d| d.bg(theme::PRESSED))
                    .accessible(Role::Button, ADD_A_PROFILE)
                    .tooltip(|window, cx| Tooltip::new(ADD_A_PROFILE).build(window, cx))
                    .child(Icon::new(IconName::Plus).size(theme::ICON_BUTTON))
                    .map(|d| keyboard_focus(d, Ring::Inside))
                    .on_click(cx.listener(|view, event: &ClickEvent, window, cx| {
                        view.focus_workspace(window, cx);
                        let mut menu = NativeMenu::new();
                        for command in crate::app_menu::commands(
                            &view.panes,
                            &crate::app_menu::EditMenu::default(),
                        ) {
                            if let Some(title) = command.profile_add_menu {
                                menu = menu.menu(title, command.action);
                            }
                        }
                        menu.show(event.position(), window, cx);
                    })),
            )
            .into_any_element()
    }

    /// The profile rows, and the page rows below the active profile.
    fn profile_rows(&self, cx: &mut Context<Self>) -> Div {
        let mut rows = div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_1)
            .px(theme::SPACE_10)
            .pb(theme::SPACE_8);
        let mut after_pages = false;
        for profile in &self.state.profiles {
            let active = self.state.profile.as_ref() == Some(&profile.id);
            rows = rows.child(self.profile_row(profile, active, after_pages, cx));
            after_pages = false;
            if active && !self.lists.pages_hidden && !self.state.pages.is_empty() {
                for page in 0..self.state.pages.len() {
                    rows = rows.child(self.page_row(page, cx));
                }
                for folder in &self.state.folders {
                    rows = rows.child(self.folder_row(folder.id, cx));
                }
                rows = rows.child(self.add_page_row(cx));
                rows = rows.child(self.add_folder_row(cx));
                after_pages = true;
            }
        }
        if self.lists.edit == Some(NameEdit::NewProfile) {
            rows = rows.child(
                Self::profile_shape(
                    row("profile-new", false),
                    div().id("profile-new-triangle"),
                    false,
                    after_pages,
                    None,
                )
                .child(self.name_field(cx)),
            );
        }
        rows
    }

    /// The parts of a profile row before its name: the disclosure triangle
    /// and the profile icon. The first profile row after the page rows has
    /// more space above it.
    fn profile_shape(
        row: Stateful<Div>,
        triangle: Stateful<Div>,
        open: bool,
        after_pages: bool,
        colour: Option<Color>,
    ) -> Stateful<Div> {
        row.px(theme::SPACE_8)
            .gap(theme::SPACE_6)
            .font_weight(FontWeight::MEDIUM)
            .when(after_pages, |d| d.mt(theme::SPACE_4))
            .child(
                triangle
                    .flex_shrink_0()
                    .text_color(theme::TEXT_TERTIARY)
                    .child(
                        Icon::new(IconName::ChevronRight)
                            .size(theme::ICON_SMALL)
                            .when(open, |i| i.rotate(percentage(0.25))),
                    ),
            )
            .child(
                // The icon has the colour of the profile, if it has one
                // (section 7.3).
                Icon::new(IconName::Layers)
                    .size(theme::ICON_ROW)
                    .text_color(colour.map_or(theme::TEXT_SECONDARY, theme::colour)),
            )
    }

    fn profile_row(
        &self,
        profile: &opencraft_engine::ProfileEntry,
        active: bool,
        after_pages: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = profile.id.clone();
        let editing = self.lists.edit == Some(NameEdit::Rename(Row::Profile(id.clone())));
        let open = active && !self.lists.pages_hidden;
        let triangle = div()
            .id(SharedString::from(format!("disclosure-{id}")))
            .debug_selector({
                let id = id.clone();
                move || format!("disclosure-{id}")
            });
        // The triangle of the active profile alone hides or shows its pages.
        // On a different profile, a click selects that profile, as a click
        // on the row does.
        let triangle = if active {
            keyboard_focus(triangle, Ring::Inside)
                .rounded(theme::RADIUS_BADGE)
                .accessible(
                    Role::Button,
                    if open {
                        "Hide the pages"
                    } else {
                        "Show the pages"
                    },
                )
                .on_click(cx.listener(|view, _, _, cx| {
                    cx.stop_propagation();
                    view.lists.pages_hidden = !view.lists.pages_hidden;
                    cx.notify();
                }))
        } else {
            triangle
        };
        let mut element = Self::profile_shape(
            row(format!("profile-{id}"), false),
            triangle,
            open,
            after_pages,
            profile.color,
        )
        .cursor_pointer()
        .accessible(
            Role::TreeItem,
            if active {
                format!("{}, {}", profile.name, ON_DEVICE.to_lowercase())
            } else {
                profile.name.clone()
            },
        )
        .aria_level(1)
        .aria_selected(false)
        .when(active, |d| d.aria_expanded(open));
        let cause = active.then(|| cause_text(&self.state.cause, &self.state.profiles));
        let note = if active {
            None
        } else {
            profile_note(profile, self.state.default_profile.as_deref())
        };
        element = if editing {
            element.child(self.name_field(cx))
        } else {
            element
                .child(row_text(profile.name.clone()))
                .when_some(note, |row, note| {
                    row.child(
                        div()
                            .id(SharedString::from(format!("profile-note-{}", profile.id)))
                            .readable_text(note.clone())
                            .flex_shrink_0()
                            .text_size(theme::TEXT_BADGE)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::TEXT_TERTIARY)
                            .child(note),
                    )
                })
                .when(active, |row| {
                    let cause = cause.clone().unwrap_or_else(|| "Selected".into());
                    row.child(
                        div()
                            .id(SharedString::from(format!("on-device-{}", profile.id)))
                            .readable_text(cause.clone())
                            .flex_shrink_0()
                            .text_size(theme::TEXT_BADGE)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::SUCCESS)
                            .tooltip(move |window, cx| {
                                Tooltip::new(cause.clone()).build(window, cx)
                            })
                            .child(ON_DEVICE),
                    )
                })
        };
        element
            .when(!editing, |d| keyboard_focus(d, Ring::Inside))
            .when(!editing, |d| {
                let id = id.clone();
                d.on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                    if event.click_count() < 2 {
                        if view.state.profile.as_ref() != Some(&id) {
                            (view.send)(Command::SelectProfile {
                                profile: id.clone(),
                            });
                        }
                    } else {
                        view.start_rename(Row::Profile(id.clone()), window, cx);
                    }
                }))
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    view.open_menu(Row::Profile(id.clone()), None, event, window, cx)
                }),
            )
            .into_any_element()
    }

    fn page_row(&self, page: usize, cx: &mut Context<Self>) -> AnyElement {
        // An open folder is the selection. The page rows show no selection
        // then (decision 0036).
        let selected = page == self.state.page && self.state.open.is_empty();
        // When the device is the selection, the active page has the look of
        // a selected row in a pane that does not have the selection.
        let focused = selected && !self.device.open;
        let editing = self.lists.edit == Some(NameEdit::Rename(Row::Page(page)));
        let title = page_title(&self.state.pages, page);
        // The profile of this frame: a click acts on what the user saw.
        let shown = self.state.profile.clone();
        let menu_shown = shown.clone();
        row(format!("page-{page}"), selected)
            .accessible(Role::TreeItem, title.clone())
            .aria_level(2)
            .aria_selected(selected)
            .pl(theme::SIDEBAR_PAGE_INDENT)
            .pr(theme::SPACE_8)
            .gap(theme::SPACE_8)
            .cursor_pointer()
            .when(focused, |d| {
                d.bg(theme::ACCENT)
                    .text_color(theme::TEXT_ON_ACCENT)
                    .font_weight(FontWeight::MEDIUM)
            })
            .when(selected && !focused, |d| d.bg(theme::SELECTED_UNFOCUSED))
            .child(if editing {
                self.name_field(cx)
            } else {
                row_text(title).into_any_element()
            })
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(theme::TEXT_CAPTION)
                    .font_weight(FontWeight::NORMAL)
                    .text_color(if focused {
                        theme::TEXT_ON_ACCENT
                    } else {
                        theme::TEXT_TERTIARY
                    })
                    .child((page + 1).to_string()),
            )
            .when(!editing, |d| keyboard_focus(d, Ring::Inside))
            .when(!editing, |d| {
                d.on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                    if event.click_count() < 2 {
                        view.activate_page(page, shown.clone(), window, cx);
                    } else {
                        view.start_rename(Row::Page(page), window, cx);
                    }
                }))
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    view.open_menu(Row::Page(page), menu_shown.clone(), event, window, cx)
                }),
            )
            .into_any_element()
    }

    /// A click on a page row makes the page the active page. The device is
    /// not the selection now. The selection of the canvas stays (decision
    /// 0029, section 8.3). `shown` is the active profile in the frame that
    /// the user clicked (`pages::in_profile`).
    pub(super) fn activate_page(
        &mut self,
        page: usize,
        shown: Option<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if std::mem::take(&mut self.device.open) {
            cx.notify();
        }
        // The same page still closes the open folders.
        if page != self.state.page || shown != self.state.profile || !self.state.open.is_empty() {
            (self.send)(in_profile(shown, Command::SelectPage { page }));
        }
    }

    fn folder_row(&self, id: opencraft_engine::FolderId, cx: &mut Context<Self>) -> AnyElement {
        let Some(name) = self.folder_name(id).map(str::to_string) else {
            return div().into_any_element();
        };
        let selected = self.state.open.last() == Some(&id);
        let focused = selected && !self.device.open;
        let editing = self.lists.edit == Some(NameEdit::Rename(Row::Folder(id)));
        let shown = self.state.profile.clone();
        let menu_shown = shown.clone();
        let icon_colour = if focused {
            theme::TEXT_ON_ACCENT
        } else {
            theme::TEXT_TERTIARY
        };
        row(format!("folder-{id}"), selected)
            .accessible(Role::TreeItem, name.clone())
            .aria_level(2)
            .aria_selected(selected)
            .pl(theme::SIDEBAR_PAGE_INDENT)
            .pr(theme::SPACE_8)
            .gap(theme::SPACE_8)
            .cursor_pointer()
            .when(focused, |d| {
                d.bg(theme::ACCENT)
                    .text_color(theme::TEXT_ON_ACCENT)
                    .font_weight(FontWeight::MEDIUM)
            })
            .when(selected && !focused, |d| d.bg(theme::SELECTED_UNFOCUSED))
            .child(if editing {
                self.name_field(cx)
            } else {
                row_text(name).into_any_element()
            })
            .child(
                gpui_kit::svg()
                    .path(theme::surface_icon_path("folder"))
                    .size(theme::ICON_ROW)
                    .flex_shrink_0()
                    .text_color(icon_colour),
            )
            .when(!editing, |d| keyboard_focus(d, Ring::Inside))
            .when(!editing, |d| {
                d.on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                    if event.click_count() < 2 {
                        if std::mem::take(&mut view.device.open) {
                            cx.notify();
                        }
                        (view.send)(in_profile(
                            shown.clone(),
                            Command::OpenFolder { folder: id },
                        ));
                    } else {
                        view.start_rename(Row::Folder(id), window, cx);
                    }
                }))
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    view.open_menu(Row::Folder(id), menu_shown.clone(), event, window, cx)
                }),
            )
            .into_any_element()
    }

    fn add_page_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let shown = self.state.profile.clone();
        row("page-add", false)
            .accessible(Role::Button, ADD_A_PAGE)
            .pl(theme::SIDEBAR_PAGE_INDENT)
            .pr(theme::SPACE_8)
            .gap(theme::SPACE_6)
            .text_size(theme::TEXT_CONTROL)
            .text_color(theme::TEXT_TERTIARY)
            .cursor_pointer()
            .child(Icon::new(IconName::Plus).size(theme::ICON_SMALL))
            .child(ADD_A_PAGE)
            .map(|d| keyboard_focus(d, Ring::Inside))
            .on_click(cx.listener(move |view, _, _, _| {
                (view.send)(in_profile(
                    shown.clone(),
                    Command::AddPage { copy_of: None },
                ))
            }))
            .into_any_element()
    }

    fn add_folder_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let shown = self.state.profile.clone();
        row("folder-add", false)
            .accessible(Role::Button, ADD_A_FOLDER)
            .pl(theme::SIDEBAR_PAGE_INDENT)
            .pr(theme::SPACE_8)
            .gap(theme::SPACE_6)
            .text_size(theme::TEXT_CONTROL)
            .text_color(theme::TEXT_TERTIARY)
            .cursor_pointer()
            .child(Icon::new(IconName::Plus).size(theme::ICON_SMALL))
            .child(ADD_A_FOLDER)
            .map(|d| keyboard_focus(d, Ring::Inside))
            .on_click(cx.listener(move |view, _, _, _| {
                let id = opencraft_engine::FolderId::random();
                (view.send)(in_profile(shown.clone(), view.new_folder(id)));
            }))
            .into_any_element()
    }

    /// The text field of the edit of a name, in place of the name. Return
    /// keeps the name, and Escape stops the edit with no change.
    fn name_field(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("row-name")
            .debug_selector(|| "row-name".into())
            .flex_1()
            .min_w_0()
            .font_weight(FontWeight::NORMAL)
            .text_color(theme::TEXT_PRIMARY)
            // The field gets no Return: after the event of the field, the
            // window puts the key into the text, and that removes a selected
            // name.
            .capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => view.cancel_name(window, cx),
                    "enter" => view.save_name(window, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            // The text in the field starts where the name starts.
            .ml(-theme::ROW_FIELD_INSET)
            .child(
                Input::new(&self.lists.name)
                    .aria_label("Name")
                    .small()
                    .h(theme::ROW_FIELD_HEIGHT)
                    .px(theme::ROW_FIELD_INSET - theme::SPACE_1)
                    .rounded(theme::RADIUS_SEGMENT)
                    .bg(theme::BG_FIELD)
                    .text_size(theme::TEXT_BODY),
            )
            .into_any_element()
    }

    /// The bottom group: "Settings" (section 7.4). A click opens the
    /// Settings window, as "Settings…" in the application menu does.
    fn bottom_group(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(theme::SPACE_1)
            .pt(theme::SPACE_8)
            .px(theme::SPACE_10)
            .pb(theme::SPACE_14)
            .border_t_1()
            .border_color(theme::LINE_SIDEBAR)
            .child(
                row("sidebar-settings", false)
                    .accessible(Role::Button, SETTINGS)
                    .px(theme::SPACE_8)
                    .gap(theme::SPACE_8)
                    .text_color(theme::TEXT_SECONDARY)
                    .cursor_pointer()
                    .map(|d| keyboard_focus(d, Ring::Inside))
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.shell.update(cx, |shell, cx| {
                            shell.command(ShellCommand::OpenSettings(None), cx)
                        })
                    }))
                    .child(Icon::new(IconName::Settings).size(theme::ICON_ROW))
                    .child(row_text(SETTINGS))
                    .children(
                        crate::settings_window::shortcuts::keys(&OpenSettings, cx).map(|keys| {
                            div()
                                .text_size(theme::TEXT_CAPTION)
                                .text_color(theme::TEXT_TERTIARY)
                                .child(keys)
                        }),
                    ),
            )
            .into_any_element()
    }
}
