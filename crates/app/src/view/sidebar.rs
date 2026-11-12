//! The sidebar on the leading side (decision 0029, section 7).
//!
//! Now it shows the top area, with the button that hides the sidebar, and
//! the device: the model name and the connection state. Issue #177 adds the
//! profiles and the pages, and #180 makes the device card a button.

use super::{
    controls::{dot, icon_button},
    *,
};
use gpui_kit::{assets::IconName, component::Icon, FontWeight, Pixels};

impl DeviceView {
    pub(super) fn sidebar(&self, width: Pixels, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("sidebar")
            .debug_selector(|| "sidebar".into())
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
                    .pt(theme::SPACE_2)
                    .px(theme::SPACE_10)
                    .pb(theme::SPACE_8)
                    .child(self.device_card()),
            )
            .into_any_element()
    }

    /// The device: an icon, the model name, and the connection state.
    fn device_card(&self) -> AnyElement {
        let connected = self.state.connected;
        let name = self
            .state
            .layout
            .as_ref()
            .map_or_else(|| "Device".to_string(), |l| l.name.clone());
        div()
            .id("device-card")
            .debug_selector(|| "device-card".into())
            .p(theme::SPACE_8)
            .flex()
            .items_center()
            .gap(theme::SPACE_10)
            .rounded(theme::RADIUS_DEVICE_CARD)
            .bg(theme::BG_FIELD)
            .border_1()
            .border_color(theme::BORDER_DEVICE_CARD)
            .child(
                div()
                    .flex_shrink_0()
                    .size(theme::DEVICE_TILE)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme::RADIUS_CARD)
                    .bg(theme::BG_TILE)
                    .child(Icon::new(IconName::Grid3x2).size(theme::ICON_DEVICE)),
            )
            .child(
                div()
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
                            .text_color(theme::TEXT_SECONDARY)
                            .child(dot(if connected {
                                theme::SUCCESS
                            } else {
                                theme::TEXT_DISABLED
                            }))
                            .child(if connected {
                                "Connected"
                            } else {
                                "Not connected"
                            }),
                    ),
            )
            .into_any_element()
    }
}
