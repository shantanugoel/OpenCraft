//! The "Library" tab of the right panel (decision 0029, section 11).
//!
//! Until #184, the tab shows an empty state.

use super::*;

/// The text of the empty state.
pub(super) const EMPTY: &str = "The action library comes in a later version.";

impl DeviceView {
    pub(super) fn library(&self) -> AnyElement {
        div()
            .id("library")
            .debug_selector(|| "library".into())
            .flex_1()
            .min_h_0()
            .pt(theme::EMPTY_STATE_TOP)
            .px(theme::SECTION_PADDING_SIDE)
            .text_center()
            .text_size(theme::TEXT_LABEL)
            .text_color(theme::TEXT_TERTIARY)
            .child(EMPTY)
            .into_any_element()
    }
}
