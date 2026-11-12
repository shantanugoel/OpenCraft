//! The three panes, the View commands, the dividers, the tabs, and the
//! status bar (decision 0029, sections 4, 5, 6, and 9).

use super::*;
use crate::{
    app_menu::{inspector_title, sidebar_title, ToggleInspector, ToggleSidebar},
    settings::{self, RightPanelTab},
    shell::ShellCommand,
};
use gpui_kit::{Bounds, MouseButton, OwnedMenuItem, Pixels};
use opencraft_engine::InputEvent;

fn bounds(cx: &mut VisualTestContext, id: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"))
}

/// The titles of the items of the View menu.
fn view_menu(cx: &mut VisualTestContext) -> Vec<String> {
    let menus = cx.update(|_, cx| cx.get_menus()).unwrap_or_default();
    let view = menus
        .into_iter()
        .find(|m| m.name.as_ref() == "View")
        .expect("a View menu");
    view.items
        .into_iter()
        .filter_map(|item| match item {
            OwnedMenuItem::Action { name, .. } => Some(name.to_string()),
            _ => None,
        })
        .collect()
}

/// A drag with the left button from `from` to `to`, in eight steps.
fn drag(cx: &mut VisualTestContext, from: gpui_kit::Point<Pixels>, to: gpui_kit::Point<Pixels>) {
    cx.simulate_mouse_move(from, None, Modifiers::default());
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    for step in 1..=8 {
        let f = step as f32 / 8.;
        let at = gpui_kit::point(from.x + (to.x - from.x) * f, from.y + (to.y - from.y) * f);
        cx.simulate_mouse_move(at, MouseButton::Left, Modifiers::default());
    }
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
}

#[gpui_kit::test]
fn the_three_panes_have_the_bounds_of_the_record(cx: &mut TestAppContext) {
    let (_, cx, _, _) = window(cx);
    let (width, height) = theme::WINDOW_SIZE;
    let toolbar = f32::from(theme::TOOLBAR_HEIGHT);
    let status = f32::from(theme::STATUS_BAR_HEIGHT);
    let expect = |cx: &mut VisualTestContext, id: &'static str, x: f32, y: f32, w: f32, h: f32| {
        let b = bounds(cx, id);
        assert_eq!(
            (b.left(), b.top(), b.size.width, b.size.height),
            (px(x), px(y), px(w), px(h)),
            "{id}"
        );
    };
    let sidebar = theme::SIDEBAR_WIDTH;
    let inspector = theme::INSPECTOR_WIDTH;
    let canvas = width - sidebar - inspector;
    // The sidebar has the full height. The toolbar is above the canvas and
    // the right panel. The status bar is below the canvas only.
    expect(cx, "sidebar", 0., 0., sidebar, height);
    expect(cx, "toolbar", sidebar, 0., width - sidebar, toolbar);
    expect(cx, "canvas", sidebar, toolbar, canvas, height - toolbar);
    expect(cx, "status-bar", sidebar, height - status, canvas, status);
    expect(
        cx,
        "right-panel",
        sidebar + canvas,
        toolbar,
        inspector,
        height - toolbar,
    );
    assert_eq!(canvas, 840.);
    // The dividers are on the lines between the panes.
    let divider = bounds(cx, "sidebar-divider");
    assert_eq!(divider.center().x, px(sidebar));
    assert_eq!(divider.size.width, theme::DIVIDER_AREA);
    let divider = bounds(cx, "inspector-divider");
    assert_eq!(divider.center().x, px(sidebar + canvas));
    assert_eq!(divider.top(), px(toolbar));
    // The content of each pane.
    for (id, pane) in [
        ("hide-sidebar", "sidebar"),
        ("device-card", "sidebar"),
        ("toolbar-title", "toolbar"),
        ("connection", "toolbar"),
        ("toggle-inspector", "toolbar"),
        ("surface-key face 2", "canvas"),
        ("last-input", "status-bar"),
        ("save-state", "status-bar"),
        ("tab-inspector", "right-panel"),
        ("tab-library", "right-panel"),
        ("editor", "right-panel"),
    ] {
        let inner = bounds(cx, id);
        let outer = bounds(cx, pane);
        assert!(
            inner.left() >= outer.left()
                && inner.right() <= outer.right()
                && inner.top() >= outer.top()
                && inner.bottom() <= outer.bottom(),
            "{id} is not in {pane}"
        );
    }
    // The status bar has the last input on the left and the save state on
    // the right.
    assert!(bounds(cx, "last-input").right() <= bounds(cx, "save-state").left());
}

#[gpui_kit::test]
fn the_view_commands_hide_and_show_the_side_panes(cx: &mut TestAppContext) {
    let (view, cx, _, _) = window(cx);
    let (width, _) = theme::WINDOW_SIZE;
    assert_eq!(view_menu(cx), ["Hide Sidebar", "Hide Inspector"]);

    // The menu item sends its action to the focused element.
    cx.dispatch_action(ToggleSidebar);
    cx.run_until_parked();
    assert!(cx.debug_bounds("sidebar").is_none());
    assert!(cx.debug_bounds("sidebar-divider").is_none());
    assert_eq!(view_menu(cx), ["Show Sidebar", "Hide Inspector"]);
    // The toolbar starts at the edge of the window, with space for the
    // window buttons, and its first item shows the sidebar.
    assert_eq!(bounds(cx, "toolbar").left(), px(0.));
    assert_eq!(bounds(cx, "canvas").left(), px(0.));
    let show = bounds(cx, "show-sidebar");
    assert_eq!(show.left(), theme::TOOLBAR_PADDING_NO_SIDEBAR);
    assert!(show.right() <= bounds(cx, "toolbar-title").left());
    cx.update(|_, cx| assert!(!view.read(cx).panes.sidebar_visible));

    // The key command shows it again.
    cx.simulate_keystrokes("ctrl-cmd-s");
    assert_eq!(bounds(cx, "sidebar").size.width, px(theme::SIDEBAR_WIDTH));
    assert_eq!(view_menu(cx), ["Hide Sidebar", "Hide Inspector"]);

    // The right panel: the canvas and the status bar fill its space. The
    // toolbar stays full width.
    cx.simulate_keystrokes("alt-cmd-i");
    assert!(cx.debug_bounds("right-panel").is_none());
    assert!(cx.debug_bounds("inspector-divider").is_none());
    assert_eq!(bounds(cx, "canvas").right(), px(width));
    assert_eq!(bounds(cx, "status-bar").right(), px(width));
    assert_eq!(bounds(cx, "toolbar").right(), px(width));
    assert_eq!(view_menu(cx), ["Hide Sidebar", "Show Inspector"]);
    cx.dispatch_action(ToggleInspector);
    cx.run_until_parked();
    assert_eq!(bounds(cx, "right-panel").right(), px(width));

    // The buttons do the same.
    click(cx, "toggle-inspector");
    assert!(cx.debug_bounds("right-panel").is_none());
    click(cx, "toggle-inspector");
    assert!(cx.debug_bounds("right-panel").is_some());
    click(cx, "hide-sidebar");
    assert!(cx.debug_bounds("sidebar").is_none());
    click(cx, "show-sidebar");
    assert!(cx.debug_bounds("sidebar").is_some());
    assert!(cx.debug_bounds("show-sidebar").is_none());

    // The key commands operate also when a text field has the focus, and
    // the field keeps the focus.
    click(cx, "surface-key face 2");
    let input = cx.update(|_, cx| view.read(cx).input.clone());
    cx.update(|window, cx| window.focus(&input.focus_handle(cx), cx));
    cx.simulate_keystrokes("ctrl-cmd-s");
    assert!(cx.debug_bounds("sidebar").is_none());
    cx.update(|window, cx| assert!(input.focus_handle(cx).is_focused(window)));
    cx.simulate_keystrokes("ctrl-cmd-s");
    assert!(cx.debug_bounds("sidebar").is_some());
    assert_eq!(sidebar_title(true), "Hide Sidebar");
    assert_eq!(inspector_title(false), "Show Inspector");
}

#[gpui_kit::test]
fn a_divider_changes_the_width_of_its_pane_between_the_limits(cx: &mut TestAppContext) {
    let (view, cx, _, _) = window(cx);
    let (width, height) = theme::WINDOW_SIZE;
    let sidebar = |cx: &mut VisualTestContext| bounds(cx, "sidebar").size.width;
    let inspector = |cx: &mut VisualTestContext| bounds(cx, "right-panel").size.width;
    let start = bounds(cx, "sidebar-divider").center();
    drag(cx, start, gpui_kit::point(start.x + px(40.), start.y));
    assert_eq!(sidebar(cx), px(280.));
    let start = bounds(cx, "sidebar-divider").center();
    drag(cx, start, gpui_kit::point(px(900.), start.y));
    assert_eq!(sidebar(cx), px(theme::SIDEBAR_MAX));
    let start = bounds(cx, "sidebar-divider").center();
    drag(cx, start, gpui_kit::point(px(20.), start.y));
    assert_eq!(sidebar(cx), px(theme::SIDEBAR_MIN));

    let start = bounds(cx, "inspector-divider").center();
    drag(cx, start, gpui_kit::point(start.x - px(60.), start.y));
    assert_eq!(inspector(cx), px(420.));
    let start = bounds(cx, "inspector-divider").center();
    drag(cx, start, gpui_kit::point(px(200.), start.y));
    assert_eq!(inspector(cx), px(theme::INSPECTOR_MAX));
    let start = bounds(cx, "inspector-divider").center();
    drag(cx, start, gpui_kit::point(px(width - 10.), start.y));
    assert_eq!(inspector(cx), px(theme::INSPECTOR_MIN));
    cx.update(|_, cx| {
        let panes = view.read(cx).panes;
        assert_eq!(panes.sidebar_width, theme::SIDEBAR_MIN);
        assert_eq!(panes.inspector_width, theme::INSPECTOR_MIN);
    });

    // A change of the window size changes only the canvas.
    cx.simulate_resize(size(px(width + 200.), px(height)));
    cx.run_until_parked();
    assert_eq!(sidebar(cx), px(theme::SIDEBAR_MIN));
    assert_eq!(inspector(cx), px(theme::INSPECTOR_MIN));
    assert_eq!(
        bounds(cx, "canvas").size.width,
        px(width + 200. - theme::SIDEBAR_MIN - theme::INSPECTOR_MIN)
    );
    // At the minimum window size, the canvas keeps its minimum width.
    let (min_width, min_height) = theme::WINDOW_MIN;
    cx.simulate_resize(size(px(min_width), px(min_height)));
    cx.run_until_parked();
    assert!(bounds(cx, "canvas").size.width >= px(theme::CANVAS_MIN));
    assert!(bounds(cx, "right-panel").right() <= px(min_width));
}

#[gpui_kit::test]
fn the_tabs_change_the_content_of_the_right_panel(cx: &mut TestAppContext) {
    let (view, cx, _, _) = window(cx);
    assert!(cx.debug_bounds("editor").is_some());
    assert!(cx.debug_bounds("library").is_none());
    click(cx, "tab-library");
    assert!(cx.debug_bounds("library").is_some());
    assert!(cx.debug_bounds("editor").is_none());
    cx.update(|_, cx| assert_eq!(view.read(cx).panes.right_panel_tab, RightPanelTab::Library));
    // The tab stays when the selection changes.
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("library").is_some());
    // The tab header does not move with the content.
    let header = bounds(cx, "tab-library");
    click(cx, "tab-inspector");
    assert!(cx.debug_bounds("editor").is_some());
    assert_eq!(bounds(cx, "tab-library"), header);
    // "Device settings" shows in the Inspector tab.
    click(cx, "tab-library");
    click(cx, "device-card");
    click(cx, "tab-inspector");
    assert!(cx.debug_bounds("device-settings").is_some());
}

#[gpui_kit::test]
fn the_status_bar_shows_the_last_input(cx: &mut TestAppContext) {
    let (view, cx, tx, _) = window(cx);
    cx.update(|_, cx| assert!(view.read(cx).state.last_input.is_none()));
    for _ in 0..3 {
        tx.try_send(Event::Input(InputEvent {
            control: ControlId::new(ControlKind::Knob, 1),
            gesture: Gesture::TurnRight,
        }))
        .unwrap();
    }
    cx.run_until_parked();
    let text = cx.update(|_, cx| {
        let input = view.read(cx).state.last_input.unwrap();
        status_bar::input_text(&input, input.at)
    });
    assert_eq!(text, "Knob 1 · Turn right × 3 · now");
    // One timer draws the window again when "now" changes to the time.
    cx.update(|_, cx| assert!(view.read(cx).input_timer.is_some()));
    cx.executor().advance_clock(status_bar::NOW_TIME);
    cx.run_until_parked();
    cx.update(|_, cx| assert!(view.read(cx).input_timer.is_none()));
}

/// A shell with `shell.toml` in `dir`, and its window.
fn shell_window(
    cx: &mut TestAppContext,
    dir: &std::path::Path,
) -> (Entity<Shell>, async_channel::Sender<ShellCommand>) {
    let (settings, problem) = settings::load(dir);
    assert_eq!(problem, None);
    cx.update(|cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        crate::app_menu::install(cx, &settings.panes);
    });
    let (commands, command_receiver) = async_channel::unbounded();
    let dir = dir.to_path_buf();
    let shell = cx.update(|cx| {
        cx.new(|cx| {
            let mut shell = Shell::new(dir, settings, Rc::new(|_| {}));
            shell.listen(async_channel::unbounded().1, command_receiver, cx);
            shell
        })
    });
    commands.try_send(ShellCommand::Open).unwrap();
    cx.run_until_parked();
    (shell, commands)
}

fn window_view(cx: &mut TestAppContext) -> Entity<DeviceView> {
    cx.update(|cx| {
        let window = cx.windows().into_iter().next()?;
        window
            .update(cx, |root, _, cx| {
                root.downcast::<gpui_kit::base::Root>()
                    .ok()
                    .and_then(|root| root.read(cx).view().clone().downcast().ok())
            })
            .ok()
            .flatten()
    })
    .expect("a window")
}

#[gpui_kit::test]
fn the_panes_are_kept_for_the_next_window_and_the_next_start(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (_shell, commands) = shell_window(cx, dir.path());
    let view = window_view(cx);
    let window = cx.update(|cx| cx.windows()[0]);
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    view.toggle_sidebar(cx);
                    view.toggle_inspector(window, cx);
                    view.panes.sidebar_width = 300.;
                    view.select_tab(RightPanelTab::Library, window, cx);
                })
            })
            .unwrap()
    });
    cx.run_until_parked();
    let kept = settings::load(dir.path()).0.panes;
    assert!(!kept.sidebar_visible && !kept.inspector_visible);
    assert_eq!(kept.sidebar_width, 300.);
    assert_eq!(kept.right_panel_tab, RightPanelTab::Library);
    let menus = cx.update(|cx| cx.get_menus()).unwrap();
    let view_items = menus.iter().find(|m| m.name.as_ref() == "View").unwrap();
    assert_eq!(view_items.items.len(), 2, "The View menu has two items");

    // A new window of the same process.
    cx.update(|cx| {
        window
            .update(cx, |_, window, _| window.remove_window())
            .unwrap()
    });
    cx.run_until_parked();
    commands.try_send(ShellCommand::Open).unwrap();
    cx.run_until_parked();
    let view = window_view(cx);
    cx.update(|cx| assert_eq!(view.read(cx).panes, kept));
}

#[gpui_kit::test]
fn a_new_start_reads_the_panes_of_shell_toml(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let panes = Panes {
        sidebar_visible: false,
        inspector_visible: true,
        sidebar_width: 260.,
        inspector_width: 400.,
        right_panel_tab: RightPanelTab::Library,
    };
    settings::save(
        dir.path(),
        &settings::Settings {
            panes,
            ..Default::default()
        },
    )
    .unwrap();
    let _ = shell_window(cx, dir.path());
    let view = window_view(cx);
    cx.update(|cx| assert_eq!(view.read(cx).panes, panes));
    let menus = cx.update(|cx| cx.get_menus()).unwrap();
    let titles: Vec<String> = menus
        .iter()
        .find(|m| m.name.as_ref() == "View")
        .expect("a View menu")
        .items
        .iter()
        .filter_map(|item| match item {
            OwnedMenuItem::Action { name, .. } => Some(name.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(titles, ["Show Sidebar", "Hide Inspector"]);
}
