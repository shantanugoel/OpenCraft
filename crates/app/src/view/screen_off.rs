//! The device inactivity choice. The engine owns the deadline and dark state.

use super::*;
use gpui_kit::{assets::IconName, component::Icon};

pub(super) const SCREEN_OFF_LABEL: &str = "Screen off after";
const MINUTES: [u16; 7] = [0, 1, 5, 10, 15, 30, 60];

fn text(minutes: u16) -> String {
    match minutes {
        0 => "Never".into(),
        1 => "1 minute".into(),
        _ => format!("{minutes} minutes"),
    }
}

impl DeviceView {
    pub(super) fn screen_off_popup(
        &self,
        minutes: u16,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open = self.device.timer_open;
        let menu = open.then(|| {
            let rows = MINUTES
                .into_iter()
                .map(|item| {
                    controls::menu_row(format!("screen-off-{item}"))
                        .child(
                            div()
                                .flex_shrink_0()
                                .w(theme::ICON_SMALL)
                                .when(item == minutes, |d| {
                                    d.child(Icon::new(IconName::Check).size(theme::ICON_SMALL))
                                }),
                        )
                        .child(text(item))
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.close_kinds();
                            view.focus_workspace(window, cx);
                            view.change_settings(|s| s.screen_off_minutes = item, window, cx);
                        }))
                        .into_any_element()
                })
                .collect();
            controls::popup_menu(
                "screen-off-menu",
                controls::MenuPlace {
                    button_height: theme::HEIGHT_CONTROL,
                    width: theme::WIDTH_SCREEN_OFF_MENU,
                    align_end: true,
                },
                rows,
                window,
                cx.listener(|view, _, window, cx| {
                    view.close_kinds();
                    view.focus_workspace(window, cx);
                    cx.notify();
                }),
            )
        });
        div()
            .relative()
            .flex()
            .w(theme::WIDTH_SCREEN_OFF_MENU)
            .child(
                controls::popup_button(
                    "screen-off-after",
                    SCREEN_OFF_LABEL,
                    text(minutes),
                    theme::HEIGHT_CONTROL,
                    open,
                    div().truncate().child(text(minutes)),
                )
                .debug_selector(|| "screen-off-after".into())
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.close_kinds();
                    view.device.timer_open = !open;
                    view.focus_workspace(window, cx);
                    cx.notify();
                })),
            )
            .children(menu)
            .into_any_element()
    }
}
