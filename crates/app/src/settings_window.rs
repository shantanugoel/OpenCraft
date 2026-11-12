//! The Settings window (decision 0029, section 17): the options of the
//! application, the key commands of the window, and the information about
//! the application.
//!
//! It is a standard macOS settings window. The user opens it with
//! "Settings…" (⌘,) in the application menu, with the menu bar item, and
//! with the sidebar row "Settings". The shell makes one window at the most
//! and brings it to the front when it is open (`Shell::open_settings`).
//!
//! | Module | Part |
//! |---|---|
//! | `general` | The pane "General": the three switches, the permission, the folders |
//! | `shortcuts` | The pane "Shortcuts": the key commands |
//! | `about` | The pane "About": the version and the licence notices |
//! | `parts` | The group box and its rows |
//!
//! The window has no state of its own for the options: the [`Shell`] keeps
//! them, and the window shows them again when the shell changes. The window
//! is not resizable. Its height follows its pane, so the window changes its
//! height when the user selects a different tab.

use crate::{
    app_menu::{CloseWindow, SETTINGS},
    settings::{Settings, SettingsPane},
    shell::{Shell, ShellCommand},
    state::ViewState,
    view::{controls::small_button, theme},
};
use gpui_kit::{
    assets::IconName, component::Icon, div, point, prelude::*, px, relative, size, AnyElement, App,
    Bounds, ClickEvent, Context, Div, Entity, FocusHandle, FontWeight, Render, SharedString,
    Stateful, Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use opencraft_engine::Status;
use std::path::{Path, PathBuf};

mod about;
mod general;
mod parts;
pub(crate) mod shortcuts;
#[cfg(test)]
mod tests;

/// The height of the window for a pane, in points (see
/// [`theme::SETTINGS_HEIGHT_GENERAL`]). The pane "Shortcuts" has `rows` rows,
/// and the pane "About" has more text without the notices file.
fn pane_height(pane: SettingsPane, rows: usize, notices: bool) -> f32 {
    match pane {
        SettingsPane::General => theme::SETTINGS_HEIGHT_GENERAL,
        SettingsPane::Shortcuts => {
            theme::SETTINGS_HEIGHT_SHORTCUTS + rows as f32 * theme::SETTINGS_SHORTCUT_ROW
        }
        SettingsPane::About if notices => theme::SETTINGS_HEIGHT_ABOUT,
        SettingsPane::About => theme::SETTINGS_HEIGHT_ABOUT_NO_NOTICES,
    }
}

/// The options of the Settings window. It opens in the middle of the main
/// screen, at the height that its pane needs.
pub fn window_options(snapshot: &Snapshot, cx: &App) -> WindowOptions {
    let pane = snapshot.settings.settings_pane;
    let height = pane_height(pane, shortcuts::rows(cx).len(), snapshot.has_notices);
    let window_size = size(px(theme::SETTINGS_WIDTH), px(height));
    let bounds = match cx.primary_display() {
        Some(display) => Bounds::centered_at(display.visible_bounds().center(), window_size),
        None => Bounds::centered(None, window_size, cx),
    };
    let (x, y) = theme::SETTINGS_WINDOW_BUTTONS;
    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(pane.name().into()),
            appears_transparent: true,
            traffic_light_position: Some(point(x, y)),
        }),
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        // The user cannot change the size. The zoom button needs a size that
        // the user can change, thus it is dim, as the minimize button.
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    }
}

/// What the window shows about the Accessibility permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    /// No kind of action waits for the permission.
    pub given: bool,
    /// The names of the kinds that need the permission.
    pub kinds: Vec<String>,
}

/// The state of the permission, from the statuses that the engine sends. A
/// handler gives a status for each kind that needs something from the user,
/// and the status "needs the user" means that the permission is not given.
/// The window has no list of kinds (decision 0029, section 20).
pub fn permission(state: &ViewState) -> Permission {
    let given = !state
        .statuses
        .values()
        .any(|status| matches!(status, Status::NeedsUser { .. }));
    let kinds = state
        .kinds
        .iter()
        .filter(|kind| state.statuses.contains_key(&kind.kind))
        .map(|kind| kind.name.clone())
        .collect();
    Permission { given, kinds }
}

/// The kinds with a status "needs the user".
pub fn kinds_waiting_for_permission(state: &ViewState) -> Vec<String> {
    state
        .statuses
        .iter()
        .filter(|(_, status)| matches!(status, Status::NeedsUser { .. }))
        .map(|(kind, _)| kind.clone())
        .collect()
}

/// The names in one text: "A", "A and B", or "A, B and C".
pub fn names_text(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [first @ .., last] => format!("{} and {last}", first.join(", ")),
    }
}

/// A path for the user: the home folder is "~".
pub fn tilde(path: &Path) -> String {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    match home
        .as_deref()
        .and_then(|home| path.strip_prefix(home).ok())
    {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// What the window shows of the shell. The window keeps a copy, as the main
/// window does with the state of the shell: the shell is being updated while
/// it opens the window, thus the window cannot read the shell then.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub settings: Settings,
    pub start_at_login: bool,
    /// The bundle has the licence notices file.
    pub has_notices: bool,
    pub profiles_dir: PathBuf,
    pub permission: Permission,
    /// The last problem of a command of this window.
    pub notice: Option<String>,
}

pub struct SettingsView {
    shell: Entity<Shell>,
    snapshot: Snapshot,
    _shell_subscription: Subscription,
    /// The focus of the root element. The key command ⌘W needs the focus in
    /// this element.
    focus: FocusHandle,
    /// The pane and the message that the window height and the title agree
    /// with.
    shown: Option<(SettingsPane, bool)>,
    /// The picture of the application icon, for the pane "About".
    icon: std::sync::Arc<gpui_kit::Image>,
}

impl SettingsView {
    /// The window opens with the pane of `snapshot`.
    pub fn new(
        shell: &Entity<Shell>,
        snapshot: Snapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscription = cx.observe_in(shell, window, |view, shell, window, cx| {
            // The shell notifies for each engine event, also for the input of
            // the device. Draw again only for a change that the window shows.
            let snapshot = shell.read(cx).settings_snapshot();
            if snapshot != view.snapshot {
                view.snapshot = snapshot;
                view.sync(window, cx);
                cx.notify();
            }
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            shell: shell.clone(),
            _shell_subscription: subscription,
            focus,
            // The options of the window have the title and the height of
            // the pane. The message starts closed.
            shown: Some((snapshot.settings.settings_pane, false)),
            snapshot,
            icon: std::sync::Arc::new(gpui_kit::Image::from_bytes(
                gpui_kit::ImageFormat::Png,
                include_bytes!("../assets/app-icon.png").to_vec(),
            )),
        }
    }

    /// The pane that the window shows now.
    fn pane(&self) -> SettingsPane {
        self.snapshot.settings.settings_pane
    }

    /// Make the title and the height of the window agree with the pane and
    /// with the message.
    fn sync(&mut self, window: &mut Window, cx: &App) {
        let pane = self.pane();
        let message = self.snapshot.notice.is_some();
        if self.shown == Some((pane, message)) {
            return;
        }
        if self.shown.is_none_or(|(shown, _)| shown != pane) {
            window.set_window_title(pane.name());
        }
        let extra = if message {
            theme::SETTINGS_MESSAGE_HEIGHT
        } else {
            0.
        };
        let height = pane_height(pane, shortcuts::rows(cx).len(), self.snapshot.has_notices);
        window.resize(size(px(theme::SETTINGS_WIDTH), px(height + extra)));
        self.shown = Some((pane, message));
    }

    /// A click handler that sends `command` to the shell.
    fn on_click(&self, command: ShellCommand) -> impl Fn(&ClickEvent, &mut Window, &mut App) {
        let shell = self.shell.clone();
        move |_, _, cx| shell.update(cx, |shell, cx| shell.command(command, cx))
    }

    /// The title bar and the row of tabs.
    fn header(&self, pane: SettingsPane, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("settings-header")
            .debug_selector(|| "settings-header".into())
            .flex_shrink_0()
            .px(theme::SPACE_16)
            .pb(theme::SPACE_8)
            .bg(theme::BG_SIDEBAR)
            .border_b_1()
            .border_color(theme::LINE_SECTION)
            .child(
                // The window buttons are at the leading side of this row.
                div()
                    .id("settings-title")
                    .debug_selector(|| "settings-title".into())
                    .h(theme::SETTINGS_TITLE_HEIGHT)
                    .flex()
                    .items_center()
                    .justify_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(pane.name()),
            )
            .child(
                div()
                    .flex()
                    .justify_center()
                    .gap(theme::SPACE_4)
                    .children(SettingsPane::ALL.map(|tab| self.tab(tab, tab == pane, cx))),
            )
            .into_any_element()
    }

    /// One tab: an icon and the name of the pane.
    fn tab(&self, pane: SettingsPane, selected: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let id: SharedString = format!("settings-tab-{}", pane.name().to_lowercase()).into();
        let (width, height) = theme::SETTINGS_TAB;
        let icon = match pane {
            SettingsPane::General => IconName::Settings,
            SettingsPane::Shortcuts => IconName::Keyboard,
            SettingsPane::About => IconName::Info,
        };
        div()
            .id(id.clone())
            .debug_selector(move || id.to_string())
            .w(width)
            .h(height)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(theme::SPACE_3)
            .rounded(theme::RADIUS_TRACK)
            .text_size(theme::TEXT_CAPTION)
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .when(selected, |d| {
                d.bg(theme::BG_SEGMENT_ON).text_color(theme::TEXT_ON_ACCENT)
            })
            .when(!selected, |d| {
                d.text_color(theme::TEXT_SECONDARY)
                    .hover(|d| d.bg(theme::HOVER_ROW))
            })
            .child(Icon::new(icon).size(theme::ICON_TAB))
            .child(pane.name())
            .on_click(cx.listener(move |view, _, _, cx| {
                view.shell
                    .update(cx, |shell, cx| shell.set_settings_pane(pane, cx));
            }))
    }

    /// The message below the pane: the last problem of a command of this
    /// window, and a button that closes it.
    fn message(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let text = self.snapshot.notice.clone()?;
        Some(
            div()
                .id("settings-message")
                .debug_selector(|| "settings-message".into())
                .flex_shrink_0()
                .h(px(theme::SETTINGS_MESSAGE_HEIGHT))
                .px(theme::SPACE_16)
                .flex()
                .items_center()
                .gap(theme::SPACE_10)
                .bg(theme::DESTRUCTIVE_TINT)
                .border_t_1()
                .border_color(theme::DESTRUCTIVE_LINE)
                .text_size(theme::TEXT_LABEL)
                .child(
                    Icon::new(IconName::TriangleAlert)
                        .size(theme::ICON_MESSAGE)
                        .text_color(theme::WARNING),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .max_h(theme::SETTINGS_MESSAGE_TEXT_MAX)
                        .overflow_hidden()
                        .child(text),
                )
                .child(
                    small_button("settings-message-close", "Close", true).on_click(cx.listener(
                        |view, _, _, cx| {
                            view.shell.update(cx, |shell, cx| shell.dismiss_notice(cx));
                        },
                    )),
                )
                .into_any_element(),
        )
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pane = self.pane();
        let content = match pane {
            SettingsPane::General => self.general(),
            SettingsPane::Shortcuts => self.shortcuts(cx),
            SettingsPane::About => self.about(),
        };
        div()
            .id("settings")
            .key_context(SETTINGS)
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::BG_BAR)
            .text_color(theme::TEXT_PRIMARY)
            .text_size(theme::TEXT_BODY)
            .line_height(relative(theme::SETTINGS_LINE_HEIGHT))
            .child(self.header(pane, cx))
            .child(
                div()
                    .id("settings-content")
                    .debug_selector(|| "settings-content".into())
                    .flex_1()
                    .min_h_0()
                    .px(theme::SETTINGS_CONTENT_X)
                    .py(theme::SETTINGS_CONTENT_Y)
                    .child(content),
            )
            .children(self.message(cx))
    }
}
