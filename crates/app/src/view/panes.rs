//! The three panes of the window, the dividers between them, and the
//! commands that show and hide the side panes (decision 0029, section 4).
//!
//! ```text
//! +----------+-------------------------------+-------------+
//! |          | toolbar                                     |
//! | sidebar  +-------------------------------+-------------+
//! |          | canvas                        | right panel |
//! |          +-------------------------------+             |
//! |          | status bar                    |             |
//! +----------+-------------------------------+-------------+
//! ```
//!
//! The widths of the side panes stay when the window changes size: only the
//! canvas changes. If the window is too narrow for the canvas, the side panes
//! show narrower, down to their minimum widths, and keep their widths in
//! `shell.toml`.

use super::*;
use crate::settings::RightPanelTab;
use gpui_kit::{DragMoveEvent, EmptyView, Pixels};

/// A side pane with a divider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Sidebar,
    Inspector,
}

/// The data of a drag on a divider.
#[derive(Clone, Copy, Debug)]
pub(super) struct DividerDrag(Side);

/// The widths of the side panes that the window shows, in points. A hidden
/// pane has the width 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Shown {
    pub sidebar: f32,
    pub inspector: f32,
}

/// The widths that fit in a window of `width`: the canvas keeps its minimum
/// width. The right panel becomes narrower first, then the sidebar. A pane
/// never becomes narrower than its minimum width.
pub(super) fn shown(panes: &Panes, width: f32) -> Shown {
    let mut sidebar = if panes.sidebar_visible {
        panes.sidebar_width
    } else {
        0.
    };
    let mut inspector = if panes.inspector_visible {
        panes.inspector_width
    } else {
        0.
    };
    let mut excess = sidebar + inspector + theme::CANVAS_MIN - width;
    if excess > 0. && inspector > 0. {
        let less = excess.min(inspector - theme::INSPECTOR_MIN).max(0.);
        inspector -= less;
        excess -= less;
    }
    if excess > 0. && sidebar > 0. {
        sidebar -= excess.min(sidebar - theme::SIDEBAR_MIN).max(0.);
    }
    Shown { sidebar, inspector }
}

impl DeviceView {
    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.panes.sidebar_visible = !self.panes.sidebar_visible;
        self.save_panes(cx);
    }

    pub(super) fn toggle_inspector(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.panes.inspector_visible = !self.panes.inspector_visible;
        // A text field of the inspector cannot keep the focus when the
        // inspector hides: the key commands need the focus in the window.
        if !self.panes.inspector_visible {
            self.focus_workspace(window, cx);
        }
        self.save_panes(cx);
    }

    pub(super) fn select_tab(
        &mut self,
        tab: RightPanelTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.panes.right_panel_tab != tab {
            self.panes.right_panel_tab = tab;
            // A text field of the tab that goes away cannot keep the focus.
            self.focus_workspace(window, cx);
            self.save_panes(cx);
        }
    }

    /// Give the panes to the shell. The shell writes `shell.toml` and sets
    /// the titles of the View menu.
    fn save_panes(&mut self, cx: &mut Context<Self>) {
        let panes = self.panes;
        self.shell
            .update(cx, |shell, cx| shell.set_panes(panes, cx));
        cx.notify();
    }

    /// A drag of a divider moves the edge of its pane to the pointer.
    fn drag_divider(
        &mut self,
        event: &DragMoveEvent<DividerDrag>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DividerDrag(side) = *event.drag(cx);
        let width = f32::from(window.viewport_size().width);
        let x = f32::from(event.event.position.x);
        let now = shown(&self.panes, width);
        match side {
            Side::Sidebar => {
                let most = (width - now.inspector - theme::CANVAS_MIN).min(theme::SIDEBAR_MAX);
                self.panes.sidebar_width = x.min(most).max(theme::SIDEBAR_MIN).round();
            }
            Side::Inspector => {
                let most = (width - now.sidebar - theme::CANVAS_MIN).min(theme::INSPECTOR_MAX);
                self.panes.inspector_width =
                    (width - x).min(most).max(theme::INSPECTOR_MIN).round();
            }
        }
        cx.notify();
    }

    /// The divider at the edge of a side pane: a line of 1, and a pointer
    /// area of 6 on the line. `left` is the position of the line.
    fn divider(&self, side: Side, left: f32, top: Pixels) -> AnyElement {
        let id = match side {
            Side::Sidebar => "sidebar-divider",
            Side::Inspector => "inspector-divider",
        };
        div()
            .id(id)
            .debug_selector(move || id.into())
            .absolute()
            .top(top)
            .bottom_0()
            .left(px(left) - theme::DIVIDER_AREA / 2.)
            .w(theme::DIVIDER_AREA)
            .cursor_col_resize()
            .on_drag(DividerDrag(side), |_, _, _, cx| cx.new(|_| EmptyView))
            .into_any_element()
    }

    /// The panes, in their order, and the dividers.
    pub(super) fn panes_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let size = window.viewport_size();
        let shown = shown(&self.panes, f32::from(size.width));
        let canvas_width = f32::from(size.width) - shown.sidebar - shown.inspector;
        div()
            .relative()
            .size_full()
            .flex()
            .on_drag_move(cx.listener(Self::drag_divider))
            .on_drop(cx.listener(|view, _: &DividerDrag, _, cx| view.save_panes(cx)))
            .when(shown.sidebar > 0., |d| {
                d.child(self.sidebar(px(shown.sidebar), cx))
            })
            .child(
                div()
                    .id("main")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(self.toolbar(cx))
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .child(self.canvas(canvas_width, window, cx))
                            .when(shown.inspector > 0., |d| {
                                d.child(self.right_panel(px(shown.inspector), cx))
                            }),
                    ),
            )
            .when(shown.sidebar > 0., |d| {
                d.child(self.divider(Side::Sidebar, shown.sidebar, px(0.)))
            })
            .when(shown.inspector > 0., |d| {
                d.child(self.divider(
                    Side::Inspector,
                    f32::from(size.width) - shown.inspector,
                    theme::TOOLBAR_HEIGHT,
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_side_panes_become_narrower_only_when_the_canvas_needs_the_space() {
        let panes = Panes::default();
        assert_eq!(
            shown(&panes, 1440.),
            Shown {
                sidebar: 240.,
                inspector: 360.
            }
        );
        let wide = Panes {
            sidebar_width: 320.,
            inspector_width: 480.,
            ..panes
        };
        // 1080 - 480 for the canvas leaves 600: the right panel goes to its
        // minimum first, then the sidebar.
        assert_eq!(
            shown(&wide, 1080.),
            Shown {
                sidebar: 280.,
                inspector: 320.
            }
        );
        assert_eq!(
            shown(&wide, 1200.),
            Shown {
                sidebar: 320.,
                inspector: 400.
            }
        );
        let hidden = Panes {
            sidebar_visible: false,
            inspector_visible: false,
            ..wide
        };
        assert_eq!(
            shown(&hidden, 1080.),
            Shown {
                sidebar: 0.,
                inspector: 0.
            }
        );
    }
}
