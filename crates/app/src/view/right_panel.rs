//! The right panel and its tabs "Inspector" and "Library" (decision 0029,
//! section 9).
//!
//! The tab header does not scroll. The content below it scrolls when it is
//! higher than the panel. The tab stays when the selection changes.

use super::*;
use crate::settings::RightPanelTab;
use gpui_kit::{FontWeight, Pixels};

/// The tabs, in their order. To add a tab, add it here and to
/// [`RightPanelTab`].
const TABS: [(RightPanelTab, &str, &str); 2] = [
    (RightPanelTab::Inspector, "tab-inspector", "Inspector"),
    (RightPanelTab::Library, "tab-library", "Library"),
];

impl DeviceView {
    pub(super) fn right_panel(
        &self,
        width: Pixels,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let content = match self.panes.right_panel_tab {
            RightPanelTab::Inspector => self.inspector(window, cx),
            RightPanelTab::Library => self.library(),
        };
        div()
            .id("right-panel")
            .debug_selector(|| "right-panel".into())
            .flex_shrink_0()
            .w(width)
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::BG_PANEL)
            .border_l_1()
            .border_color(theme::LINE_PANE)
            .child(self.tab_header(cx))
            .child(content)
            .into_any_element()
    }

    /// A segmented control of full width with one segment for each tab.
    fn tab_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let selected = self.panes.right_panel_tab;
        div()
            .flex_shrink_0()
            .py(theme::SPACE_12)
            .px(theme::SPACE_16)
            .border_b_1()
            .border_color(theme::LINE_SECTION)
            .child(
                div()
                    .id("right-tabs")
                    .flex()
                    .p(theme::SPACE_2)
                    .rounded(theme::RADIUS_TRACK)
                    .bg(theme::BG_FIELD)
                    .children(TABS.into_iter().map(|(tab, id, name)| {
                        let on = tab == selected;
                        div()
                            .id(id)
                            .debug_selector(move || id.into())
                            .flex_1()
                            .h(theme::HEIGHT_SEGMENT)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(theme::RADIUS_SEGMENT)
                            .text_size(theme::TEXT_LABEL)
                            .font_weight(FontWeight::MEDIUM)
                            .when(on, |d| {
                                d.bg(theme::BG_SEGMENT_ON).text_color(theme::TEXT_ON_ACCENT)
                            })
                            .when(!on, |d| {
                                d.text_color(theme::TEXT_SECONDARY)
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme::HOVER_ROW))
                            })
                            .child(name)
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.select_tab(tab, window, cx)
                            }))
                    })),
            )
            .into_any_element()
    }
}
