//! The toolbar above the canvas and the right panel (decision 0029,
//! section 5).
//!
//! The items, from the leading side: the title block, the button "Device
//! settings" (until #180), a flexible space, the live state pill, and the
//! button that hides the right panel. The places of
//! undo and redo (#186) are after the title block, and the place of the
//! search field (#192) is after the pill.

use super::{
    controls::{dot, icon_button},
    pages::page_title,
    *,
};
use gpui_kit::{assets::IconName, FontWeight, MouseButton, MouseDownEvent, MouseMoveEvent};

/// The text below the page name: the profile and the number of pages.
pub(super) fn profile_line(profile: &str, pages: usize) -> String {
    format!(
        "{profile} · {pages} {}",
        if pages == 1 { "page" } else { "pages" }
    )
}

impl DeviceView {
    /// An empty part of the toolbar or of the top of the sidebar moves the
    /// window, as a title bar does. The move starts at the first move of the
    /// pointer with the button down. A double click does the double-click
    /// action of the title bar that the user chose in System Settings.
    pub(super) fn moves_window(
        &self,
        element: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        element
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, event: &MouseDownEvent, window, _| {
                    view.window_move = event.click_count < 2;
                    if !view.window_move {
                        window.titlebar_double_click();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|view, _, _, _| view.window_move = false),
            )
            .on_mouse_move(cx.listener(|view, event: &MouseMoveEvent, window, _| {
                if view.window_move && event.dragging() {
                    view.window_move = false;
                    window.start_window_move();
                }
            }))
    }

    pub(super) fn toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let sidebar = self.panes.sidebar_visible;
        let inspector = self.panes.inspector_visible;
        div()
            .id("toolbar")
            .debug_selector(|| "toolbar".into())
            .flex_shrink_0()
            .h(theme::TOOLBAR_HEIGHT)
            .pl(if sidebar {
                theme::SPACE_18
            } else {
                theme::TOOLBAR_PADDING_NO_SIDEBAR
            })
            .pr(theme::SPACE_14)
            .flex()
            .items_center()
            .gap(theme::GAP_ITEMS)
            .bg(theme::BG_BAR)
            .border_b_1()
            .border_color(theme::LINE_SECTION)
            .when(!sidebar, |d| {
                d.child(
                    icon_button("show-sidebar", IconName::PanelLeft, "Show the sidebar")
                        .on_click(cx.listener(|view, _, _, cx| view.toggle_sidebar(cx))),
                )
            })
            .child(self.title_block(cx))
            .child(
                small_button("open-device-settings", "Device settings", true)
                    .when(self.device.open, |d| {
                        d.border_color(theme::ACCENT_LINE)
                            .text_color(theme::ACCENT_TEXT)
                    })
                    .on_click(
                        cx.listener(|view, _, window, cx| view.toggle_device_panel(window, cx)),
                    ),
            )
            .child(self.moves_window(div().id("toolbar-space").flex_1().min_w_0().h_full(), cx))
            .child(self.live_pill())
            .child(
                icon_button(
                    "toggle-inspector",
                    IconName::PanelRight,
                    if inspector {
                        "Hide the inspector"
                    } else {
                        "Show the inspector"
                    },
                )
                .on_click(cx.listener(|view, _, window, cx| view.toggle_inspector(window, cx))),
            )
            .into_any_element()
    }

    /// The page name, and below it the profile and the number of pages.
    fn title_block(&self, cx: &mut Context<Self>) -> AnyElement {
        let pages = &self.state.pages;
        let profile = self
            .state
            .profile
            .as_ref()
            .and_then(|id| self.state.profiles.iter().find(|p| &p.id == id));
        let title = if pages.is_empty() {
            "OpenCraft".to_string()
        } else {
            page_title(pages, self.state.page)
        };
        self.moves_window(
            div()
                .id("toolbar-title")
                .debug_selector(|| "toolbar-title".into())
                .flex_shrink_0()
                .max_w(theme::TOOLBAR_TITLE_MAX)
                .mr(theme::SPACE_4)
                .flex()
                .flex_col()
                .gap(theme::SPACE_1)
                .child(
                    div()
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .children(profile.filter(|_| !pages.is_empty()).map(|profile| {
                    div()
                        .truncate()
                        .text_size(theme::TEXT_CAPTION)
                        .text_color(theme::TEXT_TERTIARY)
                        .child(profile_line(&profile.name, pages.len()))
                })),
            cx,
        )
        .into_any_element()
    }

    /// "Live on the device" or "Not connected".
    fn live_pill(&self) -> AnyElement {
        let connected = self.state.connected;
        div()
            .id("connection")
            .debug_selector(|| "connection".into())
            .flex_shrink_0()
            .h(theme::HEIGHT_PILL)
            .px(theme::SPACE_10)
            .flex()
            .items_center()
            .gap(theme::SPACE_6)
            .rounded_full()
            .text_size(theme::TEXT_HELP)
            .font_weight(FontWeight::MEDIUM)
            .when(connected, |d| {
                d.bg(theme::SUCCESS_TINT)
                    .text_color(theme::SUCCESS_TEXT)
                    .child(dot(theme::SUCCESS))
                    .child("Live on the device")
            })
            .when(!connected, |d| {
                d.bg(theme::NEUTRAL_TINT)
                    .text_color(theme::TEXT_SECONDARY)
                    .child(dot(theme::TEXT_DISABLED))
                    .child("Not connected")
                    .tooltip(|window, cx| {
                        Tooltip::new(
                            "Connect the device with a USB cable. If VSD Craft runs, quit it.",
                        )
                        .build(window, cx)
                    })
            })
            .into_any_element()
    }
}
