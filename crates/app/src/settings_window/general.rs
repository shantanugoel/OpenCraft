//! The pane "General" (decision 0029, section 17): three switches, the
//! state of the Accessibility permission, and the folders of OpenCraft.

use super::{
    names_text,
    parts::{group, row},
    tilde, Permission, SettingsView,
};
use crate::shell::{Place, ShellCommand};
use crate::view::{
    controls::{button, small_button, switch},
    theme,
};
use gpui_kit::{
    assets::IconName, component::Icon, div, prelude::*, AnyElement, Div, FontWeight, Stateful,
};

/// The three switches: the name, the description, and the id.
pub const START_AT_LOGIN: (&str, &str) = (
    "Start at login",
    "The device operates after you log in, with no window",
);
pub const SHOW_DOCK_ICON: (&str, &str) = (
    "Show the Dock icon",
    "Off: OpenCraft is only in the menu bar",
);
pub const SHOW_WINDOW_AT_START: (&str, &str) = (
    "Show the window at start",
    "Off: the window opens from the menu bar item",
);

/// The two texts of the permission row: the state, and the kinds that need
/// the permission. The second text is empty if no kind needs it.
pub fn permission_texts(permission: &Permission) -> (&'static str, String) {
    let state = if permission.given {
        "given"
    } else {
        "not given"
    };
    let kinds = if permission.kinds.is_empty() {
        String::new()
    } else {
        format!("Necessary for {}", names_text(&permission.kinds))
    };
    (state, kinds)
}

impl SettingsView {
    pub(super) fn general(&self) -> AnyElement {
        let settings = &self.snapshot.settings;
        let login = self.snapshot.start_at_login;
        let permission = self.snapshot.permission.clone();
        let profiles = tilde(&self.snapshot.profiles_dir);
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_14)
            .child(group([
                row(
                    START_AT_LOGIN.0,
                    START_AT_LOGIN.1,
                    switch("switch-start-at-login", login, theme::SWITCH_SETTINGS)
                        .on_click(self.on_click(ShellCommand::ToggleStartAtLogin)),
                ),
                row(
                    SHOW_DOCK_ICON.0,
                    SHOW_DOCK_ICON.1,
                    switch(
                        "switch-dock-icon",
                        settings.show_dock_icon,
                        theme::SWITCH_SETTINGS,
                    )
                    .on_click(self.on_click(ShellCommand::ToggleDockIcon)),
                ),
                row(
                    SHOW_WINDOW_AT_START.0,
                    SHOW_WINDOW_AT_START.1,
                    switch(
                        "switch-window-at-start",
                        settings.show_window_at_start,
                        theme::SWITCH_SETTINGS,
                    )
                    .on_click(self.on_click(ShellCommand::ToggleShowWindowAtStart)),
                ),
            ]))
            .child(self.permission_group(&permission))
            .child(group([
                row(
                    "Profiles folder",
                    profiles,
                    small_button("show-profiles", "Show in Finder", true)
                        .on_click(self.on_click(ShellCommand::ShowInFinder(Place::Profiles))),
                ),
                row(
                    "Log file",
                    "Attach it when you report a fault",
                    small_button("show-log-file", "Show in Finder", true)
                        .on_click(self.on_click(ShellCommand::ShowInFinder(Place::LogFile))),
                ),
            ]))
            .into_any_element()
    }

    /// The group box of the Accessibility permission.
    fn permission_group(&self, permission: &Permission) -> Stateful<Div> {
        let (state, kinds) = permission_texts(permission);
        let colour = if permission.given {
            theme::SUCCESS
        } else {
            theme::WARNING
        };
        div()
            .id("permission")
            .debug_selector(|| "permission".into())
            .flex()
            .items_center()
            .gap(theme::SPACE_12)
            .px(theme::SPACE_14)
            .py(theme::SPACE_12)
            .rounded(theme::RADIUS_GROUP)
            .bg(theme::BG_CARD)
            .border_1()
            .border_color(theme::LINE_PANE)
            .child(
                Icon::new(if permission.given {
                    IconName::ShieldCheck
                } else {
                    IconName::ShieldAlert
                })
                .size(theme::ICON_TAB)
                .text_color(colour),
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
                            .flex()
                            .gap(theme::SPACE_4)
                            .child("Accessibility permission:")
                            .child(
                                div()
                                    .id("permission-state")
                                    .debug_selector(|| "permission-state".into())
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colour)
                                    .child(state),
                            ),
                    )
                    .when(!kinds.is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(theme::TEXT_HELP)
                                .text_color(theme::TEXT_TERTIARY)
                                .child(kinds),
                        )
                    }),
            )
            .child(
                button("open-system-settings", "Open System Settings")
                    .on_click(self.on_click(ShellCommand::OpenAccessibilitySettings)),
            )
    }
}
