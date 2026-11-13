//! The pane "About" (decision 0029, section 17): the application icon, the
//! name, the version, and the button that opens the licence notices
//! (decision 0026).

use super::SettingsView;
use crate::{
    shell::{ShellCommand, NOTICES_FILE},
    view::{
        controls::{button, button_if, Accessible},
        theme,
    },
};
use gpui_kit::{div, img, prelude::*, AnyElement, FontWeight};

/// The version of the application: the workspace version in `Cargo.toml`.
/// The release script writes the same version into the bundle (decision
/// 0025).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The text for a copy that is not in a bundle, for example after
/// `cargo run`: it has no licence notices file.
pub fn no_notices_text() -> String {
    format!(
        "This copy of OpenCraft has no licence notices. The file {NOTICES_FILE} of the source \
         code has them."
    )
}

impl SettingsView {
    pub(super) fn about(&self) -> AnyElement {
        let notices = self.snapshot.has_notices;
        div()
            .id("about")
            .debug_selector(|| "about".into())
            .flex()
            .flex_col()
            .items_center()
            .gap(theme::SPACE_6)
            .child(
                img(self.icon.clone())
                    .id("about-icon")
                    .size(theme::APP_ICON)
                    .mb(theme::SPACE_6),
            )
            .child(
                div()
                    .id("about-name")
                    .readable_text("OpenCraft")
                    .text_size(theme::TEXT_SHEET_TITLE)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("OpenCraft"),
            )
            .child(
                div()
                    .id("about-version")
                    .debug_selector(|| "about-version".into())
                    .readable_text(format!("Version {VERSION}"))
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_SECONDARY)
                    .child(format!("Version {VERSION}")),
            )
            .child(div().h(theme::SPACE_12))
            .child(if notices {
                button("licence-notices", "Licence notices")
                    .on_click(self.on_click(ShellCommand::OpenNotices))
                    .into_any_element()
            } else {
                button_if("licence-notices", "Licence notices", false).into_any_element()
            })
            .child(
                button("copy-diagnostics", "Copy diagnostics")
                    .on_click(self.on_click(ShellCommand::CopyDiagnostics)),
            )
            .when(!notices, |d| {
                d.child(
                    div()
                        .id("no-notices")
                        .debug_selector(|| "no-notices".into())
                        .readable_text(no_notices_text())
                        .text_center()
                        .text_size(theme::TEXT_HELP)
                        .text_color(theme::TEXT_TERTIARY)
                        .child(no_notices_text()),
                )
            })
            .into_any_element()
    }
}
