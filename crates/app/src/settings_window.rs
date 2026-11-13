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
//! them, and the window shows them again when the shell changes. The user
//! cannot change the size of the window. Its height follows the content of
//! its pane (see [`Fit`]): the window measures its content and asks the
//! platform for that height, so a new row needs no new number.

use crate::{
    app_menu::{CloseWindow, SETTINGS},
    settings::{Settings, SettingsPane},
    shell::{Shell, ShellCommand},
    state::ViewState,
    view::{controls::small_button, theme},
};
use gpui_kit::{
    assets::IconName, component::Icon, div, point, prelude::*, px, relative, size, AnyElement, App,
    Bounds, ClickEvent, Context, Div, Entity, EntityId, FocusHandle, FontWeight, Pixels, Render,
    SharedString, Stateful, Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use opencraft_engine::{Action, Status};
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
};

mod about;
mod general;
mod parts;
pub(crate) mod shortcuts;
#[cfg(test)]
mod tests;

/// The height of the content of the pane of `snapshot`, in a window that the
/// user never sees. The window opens hidden, draws one frame (the content
/// reports its height in that frame), and goes away at once. `None` if the
/// platform cannot open the window.
///
/// This is how the Settings window gets its first height: a window that
/// opens at a wrong height and then changes it shows a frame with the old
/// picture stretched to the new height (`docs/kb/gpui.md`).
fn measure_height(shell: &Entity<Shell>, snapshot: &Snapshot, cx: &mut App) -> Option<Pixels> {
    let probe = Rc::new(Cell::new(None));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(theme::SETTINGS_WIDTH), theme::settings_header_height()),
        ))),
        show: false,
        focus: false,
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };
    let handle = cx
        .open_window(options, {
            let shell = shell.clone();
            let snapshot = snapshot.clone();
            let probe = probe.clone();
            move |window, cx| {
                let view = cx.new(|cx| SettingsView::probe(&shell, snapshot, probe, window, cx));
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            }
        })
        .ok()?;
    let _ = handle.update(cx, |_, window, _| window.remove_window());
    probe.get()
}

/// The options of the Settings window. It opens in the middle of the main
/// screen, at the height of the content of its pane (see
/// [`measure_height`]). Thus the user never sees a wrong height, and the
/// window does not need a number for the height of a pane.
pub fn window_options(shell: &Entity<Shell>, snapshot: &Snapshot, cx: &mut App) -> WindowOptions {
    let pane = snapshot.settings.settings_pane;
    let height = measure_height(shell, snapshot, cx).unwrap_or(theme::settings_header_height());
    let window_size = size(px(theme::SETTINGS_WIDTH), height);
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

/// The state of the permission. The kinds that need it come from their
/// descriptions (`needs_accessibility`, decision 0029, section 20). A
/// handler gives a status for each kind that needs something from the user,
/// and the status "needs the user" of such a kind means that the permission
/// is not given. The window has no list of kinds (rule 8).
pub fn permission(state: &ViewState) -> Permission {
    let needs: Vec<_> = state
        .kinds
        .iter()
        .filter(|kind| kind.needs_accessibility)
        .collect();
    let given = !needs.iter().any(|kind| {
        matches!(
            state.statuses.get(&kind.kind),
            Some(Status::NeedsUser { .. })
        )
    });
    let kinds = needs.iter().map(|kind| kind.name.clone()).collect();
    Permission { given, kinds }
}

/// The name of the first kind of `actions` that needs the Accessibility
/// permission, if the permission is not given (see [`permission`]). The
/// Accessibility request of the first start uses it (decision 0029,
/// section 18). `None` if the permission is given, or if no action needs
/// it.
pub fn kind_waiting_for_permission(actions: &[Action], state: &ViewState) -> Option<String> {
    if permission(state).given {
        return None;
    }
    actions.iter().find_map(|action| {
        state
            .kinds
            .iter()
            .find(|kind| kind.kind == action.kind && kind.needs_accessibility)
            .map(|kind| kind.name.clone())
    })
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

/// What the window shows, and what decides the height of the window: the
/// pane, the message below it, and the permission text of the pane
/// "General" (a list of kinds can make it longer). The window shows a new
/// `Shown` only after the platform gave the window the height of the new
/// content (see [`Fit`]). All other values of the [`Snapshot`], for example
/// a switch, show at once.
#[derive(Debug, Clone, PartialEq)]
struct Shown {
    pane: SettingsPane,
    notice: Option<String>,
    permission: Permission,
}

impl Shown {
    fn of(snapshot: &Snapshot) -> Self {
        Self {
            pane: snapshot.settings.settings_pane,
            notice: snapshot.notice.clone(),
            permission: snapshot.permission.clone(),
        }
    }
}

/// Two heights are equal for the window (see
/// [`theme::SETTINGS_HEIGHT_TOLERANCE`]).
fn same_height(a: Pixels, b: Pixels) -> bool {
    (a - b).abs() <= theme::SETTINGS_HEIGHT_TOLERANCE
}

/// The height of the children of a column, from the top of the first child
/// to the bottom of the last, in whole points.
fn extent(children: &[Bounds<Pixels>]) -> Pixels {
    let top = children.first().map_or(px(0.), |first| first.origin.y);
    let bottom = children.last().map_or(px(0.), |last| last.bottom());
    px(f32::from(bottom - top).ceil())
}

/// The height of the window follows the content of the pane. The window has
/// no number for the height of a pane; it measures.
///
/// - **At the start.** A window that the user never sees measures the
///   content first ([`measure_height`]): the column of the content reports
///   its height in the first frame (`Div::on_children_prepainted`). The
///   window that the user sees opens at that height, so it needs no change
///   of its height.
/// - **A new pane or message.** The window keeps the old content on the
///   screen, and lays out the new content in a hidden copy (it is not
///   painted). [`Fit::measure`] reads the height of the copy and asks for
///   it. The platform changes the height of the window and draws a frame at
///   the new height. The view reads the new height in `render`, and only
///   then does it show the new content ([`SettingsView::commit`]). Thus no
///   frame shows the old content at the new height, or the new content at
///   the old height.
/// - **A change in place.** For a change that is not part of [`Shown`]
///   (for example a longer row), [`Fit::show`] asks for the height after the
///   frame.
///
/// The prepaint functions of GPUI have no access to the view. Thus this
/// state is shared (`Rc`) between the view and those functions.
struct Fit {
    view: EntityId,
    /// The `Shown` that the hidden copy was measured for, and its height.
    measured: RefCell<Option<(Shown, Pixels)>>,
    /// The platform did not give the height in `SETTINGS_RESIZE_TIMEOUT`:
    /// show the new content now.
    forced: Cell<bool>,
    /// The number of the last measurement or change of the shown content. A
    /// timer of an older number does nothing.
    token: Cell<u32>,
    /// A window that the user never sees reads the height of the content
    /// here (see [`measure_height`]).
    probe: Option<Rc<Cell<Option<Pixels>>>>,
}

impl Fit {
    fn new(view: EntityId, probe: Option<Rc<Cell<Option<Pixels>>>>) -> Rc<Self> {
        Rc::new(Self {
            view,
            measured: RefCell::new(None),
            forced: Cell::new(false),
            token: Cell::new(0),
            probe,
        })
    }

    /// Ask the platform for `height`, if the window has a different height.
    /// The platform tells the window when it has the new height, and then
    /// GPUI draws a frame. This function is safe to call in each frame.
    fn resize(window: &mut Window, height: Pixels) -> bool {
        let needed = !same_height(window.viewport_size().height, height);
        if needed {
            window.resize(size(px(theme::SETTINGS_WIDTH), height));
        }
        needed
    }

    /// The content on the screen has `height`: the window takes it.
    fn show(&self, height: Pixels, window: &mut Window) {
        match &self.probe {
            Some(probe) => probe.set(Some(height)),
            None => {
                Self::resize(window, height);
            }
        }
    }

    /// The hidden copy of `target` has `height`: ask the platform for it.
    fn measure(self: &Rc<Self>, target: &Shown, height: Pixels, window: &mut Window, cx: &mut App) {
        let changed = !self
            .measured
            .borrow()
            .as_ref()
            .is_some_and(|(shown, measured)| shown == target && *measured == height);
        if changed {
            *self.measured.borrow_mut() = Some((target.clone(), height));
            self.forced.set(false);
            self.token.set(self.token.get().wrapping_add(1));
        }
        if !Self::resize(window, height) {
            if changed {
                // The window has this height: the platform sends nothing.
                // Draw again to show the content.
                cx.notify(self.view);
            }
            return;
        }
        if changed {
            let fit = self.clone();
            let token = self.token.get();
            cx.spawn(async move |cx| {
                cx.background_executor()
                    .timer(theme::SETTINGS_RESIZE_TIMEOUT)
                    .await;
                cx.update(|cx| {
                    if fit.token.get() == token {
                        fit.forced.set(true);
                        cx.notify(fit.view);
                    }
                });
            })
            .detach();
        }
    }

    /// True if the window has the height that was measured for `target` (or
    /// the platform did not give it in time).
    fn ready(&self, target: &Shown, window: &Window) -> bool {
        match &*self.measured.borrow() {
            Some((shown, height)) if shown == target => {
                self.forced.get() || same_height(window.viewport_size().height, *height)
            }
            _ => false,
        }
    }

    /// The content on the screen is the content that the view wants: no
    /// measurement is open.
    fn settled(&self) {
        *self.measured.borrow_mut() = None;
        self.forced.set(false);
        self.token.set(self.token.get().wrapping_add(1));
    }
}

pub struct SettingsView {
    shell: Entity<Shell>,
    snapshot: Snapshot,
    _shell_subscription: Subscription,
    /// The focus of the root element. The key command ⌘W needs the focus in
    /// this element.
    focus: FocusHandle,
    /// The content on the screen. It agrees with the height of the window.
    /// It becomes the `Shown` of the snapshot after the window got the
    /// height of that content.
    shown: Shown,
    fit: Rc<Fit>,
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
        Self::build(shell, snapshot, None, window, cx)
    }

    /// A view for a window that the user never sees: it reports the height
    /// of its content to `probe`.
    fn probe(
        shell: &Entity<Shell>,
        snapshot: Snapshot,
        probe: Rc<Cell<Option<Pixels>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::build(shell, snapshot, Some(probe), window, cx)
    }

    fn build(
        shell: &Entity<Shell>,
        snapshot: Snapshot,
        probe: Option<Rc<Cell<Option<Pixels>>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscription = cx.observe(shell, |view, shell, cx| {
            // The shell notifies for each engine event, also for the input of
            // the device. Draw again only for a change that the window shows.
            let snapshot = shell.read(cx).settings_snapshot();
            if snapshot != view.snapshot {
                view.snapshot = snapshot;
                cx.notify();
            }
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            shell: shell.clone(),
            _shell_subscription: subscription,
            focus,
            // The options of the window have the title of the pane.
            shown: Shown::of(&snapshot),
            fit: Fit::new(cx.entity_id(), probe),
            snapshot,
            icon: std::sync::Arc::new(gpui_kit::Image::from_bytes(
                gpui_kit::ImageFormat::Png,
                include_bytes!("../assets/app-icon.png").to_vec(),
            )),
        }
    }

    /// Show the content of the snapshot if the window has its height now.
    fn commit(&mut self, window: &mut Window) {
        let target = Shown::of(&self.snapshot);
        if self.shown == target {
            // A measurement that was open is old now (the user selected the
            // pane that the window shows).
            self.fit.settled();
            return;
        }
        if !self.fit.ready(&target, window) {
            return;
        }
        if self.shown.pane != target.pane {
            window.set_window_title(target.pane.name());
        }
        self.shown = target;
        self.fit.settled();
    }

    /// The title area, the content of `shown.pane`, and the message. The row
    /// of tabs marks `tab`.
    fn column(&self, shown: &Shown, tab: SettingsPane, cx: &mut Context<Self>) -> Div {
        let content = match shown.pane {
            SettingsPane::General => self.general(&shown.permission),
            SettingsPane::Shortcuts => self.shortcuts(cx),
            SettingsPane::About => self.about(),
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .child(self.header(tab, cx))
            .child(
                div()
                    .id("settings-content")
                    .debug_selector(|| "settings-content".into())
                    .px(theme::SETTINGS_CONTENT_X)
                    .py(theme::SETTINGS_CONTENT_Y)
                    .child(content),
            )
            .children(self.message(shown.notice.as_deref(), cx))
    }

    /// A click handler that sends `command` to the shell.
    fn on_click(&self, command: ShellCommand) -> impl Fn(&ClickEvent, &mut Window, &mut App) {
        let shell = self.shell.clone();
        move |_, _, cx| shell.update(cx, |shell, cx| shell.command(command.clone(), cx))
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
    fn message(&self, text: Option<&str>, cx: &mut Context<Self>) -> Option<AnyElement> {
        let text = text?.to_string();
        Some(
            div()
                .id("settings-message")
                .debug_selector(|| "settings-message".into())
                .px(theme::SPACE_16)
                .py(theme::SETTINGS_MESSAGE_PADDING)
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.commit(window);
        let target = Shown::of(&self.snapshot);
        // The hidden copy is below the content on the screen, so that the
        // content on the screen gets the mouse.
        let measure = (self.shown != target).then(|| {
            // Keep the frames coming until the platform gave the height
            // (or `SETTINGS_RESIZE_TIMEOUT` passed): the view shows the new
            // content in the frame that comes after the new height.
            window.request_animation_frame();
            let fit = self.fit.clone();
            let measured = target.clone();
            self.column(&target, target.pane, cx)
                .on_children_prepainted(move |children, window, cx| {
                    fit.measure(&measured, extent(&children), window, cx)
                })
                .id("settings-measure")
                .absolute()
                .top_0()
                .left_0()
                .invisible()
        });
        let fit = self.fit.clone();
        let visible = self
            .column(&self.shown, target.pane, cx)
            .on_children_prepainted(move |children, window, _| fit.show(extent(&children), window));
        div()
            .id("settings")
            .key_context(SETTINGS)
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
            .size_full()
            .relative()
            .bg(theme::BG_BAR)
            .text_color(theme::TEXT_PRIMARY)
            .text_size(theme::TEXT_BODY)
            .line_height(relative(theme::SETTINGS_LINE_HEIGHT))
            .children(measure)
            .child(visible)
    }
}
