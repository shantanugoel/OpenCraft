//! The icon of a key face or a bar panel (decision 0029, sections 10.2 and
//! 14; decision 0033): the row "Icon" of "Appearance", and the icon picker
//! popover.
//!
//! The icons are the icon set of the engine. The engine draws the icon on
//! the surface (rule 11): the window only sends `Command::SetIcon`. The
//! grid, the tile of the row, and the kind tiles show the same drawings
//! through the asset source [`theme::WindowAssets`].
//!
//! The popover:
//!
//! - The tab chips "Icons" and "Recent". The tabs "Packs" and "GIF" are
//!   places for later: a new tab is one more chip in the row.
//! - The search field, with the number of icons that the grid shows. It has
//!   the focus when the popover opens.
//! - The grid of 7 columns. With no search text, the tab "Icons" shows the
//!   icons in the groups of their category. A search shows the icons that
//!   agree with the text, the best first, with no groups.
//! - The icon colours, and the buttons "Image file…" and "No icon".
//!
//! A click on an icon or a colour applies it at once, as one step of the
//! history, and the popover stays open. Escape and a click out of the
//! popover close it.

use super::*;
use gpui_kit::{
    anchored, canvas, deferred, list, point, rgb, Bounds, KeyDownEvent, ListAlignment, ListState,
    MouseButton, Pixels, Rgba,
};
use opencraft_engine::{icon_color, icon_set, Icon, SurfaceContent, SurfaceIcon};
use std::cell::Cell;

/// The group name of a cell of the grid, for the colour of its icon under
/// the pointer.
const CELL_GROUP: &str = "icon-cell";

/// The tab of the popover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickerTab {
    Icons,
    Recent,
}

/// One row of the grid: the title of a group, or up to 7 icons.
#[derive(Debug, PartialEq)]
enum GridRow {
    Title(&'static str),
    Icons(Vec<&'static Icon>),
}

/// The state of the icon picker.
pub(super) struct IconPicker {
    /// The popover is open.
    pub(super) open: bool,
    tab: PickerTab,
    search: Entity<InputState>,
    _search_subscription: Subscription,
    /// The rows of the grid for the tab and the search text, and the number
    /// of icons in them.
    rows: Rc<Vec<GridRow>>,
    pub(super) count: usize,
    list: ListState,
    /// The colour for the next icon, when the surface has no icon yet.
    colour: Option<Color>,
    /// The bounds of the button "Choose icon…" in the last frame, in window
    /// coordinates. The popover and its arrow go next to it.
    button: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl IconPicker {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search icons"));
        let subscription = cx.subscribe_in(&search, window, |view, _, event, _, cx| {
            if matches!(event, TextEvent::Change) {
                view.refresh_icon_rows(cx);
                cx.notify();
            }
        });
        Self {
            open: false,
            tab: PickerTab::Icons,
            search,
            _search_subscription: subscription,
            rows: Rc::new(Vec::new()),
            count: 0,
            list: ListState::new(0, ListAlignment::Top, theme::ICON_CELL_HEIGHT * 4.),
            colour: None,
            button: Rc::new(Cell::new(None)),
        }
    }
}

/// The rows of the grid of `tab` for the search text `query`, and the
/// number of icons in them. `recent` is the list of the recent icons.
fn grid_rows(tab: PickerTab, query: &str, recent: &[String]) -> (Vec<GridRow>, usize) {
    let set = icon_set();
    let chunks = |icons: Vec<&'static Icon>| -> Vec<GridRow> {
        icons
            .chunks(theme::ICON_GRID_COLUMNS)
            .map(|c| GridRow::Icons(c.to_vec()))
            .collect()
    };
    match tab {
        PickerTab::Recent => {
            let found: Vec<&'static Icon> = set.search(query);
            let icons: Vec<&'static Icon> = recent
                .iter()
                .filter_map(|name| found.iter().copied().find(|i| i.name == name))
                .collect();
            let count = icons.len();
            (chunks(icons), count)
        }
        PickerTab::Icons if query.trim().is_empty() => {
            let mut rows = Vec::new();
            for (category, icons) in set.groups() {
                rows.push(GridRow::Title(category.title));
                rows.extend(chunks(icons));
            }
            (rows, set.icons().len())
        }
        PickerTab::Icons => {
            let icons = set.search(query);
            let count = icons.len();
            (chunks(icons), count)
        }
    }
}

/// The number of icons for the search field, for example "1,818 icons".
fn count_text(count: usize) -> String {
    let digits = count.to_string();
    let mut text = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    match count {
        1 => "1 icon".into(),
        _ => format!("{text} icons"),
    }
}

/// The text of a grid with no icon.
fn empty_text(tab: PickerTab, query: &str) -> String {
    match (tab, query.trim()) {
        (PickerTab::Recent, "") => "The icons that you choose show here.".into(),
        (_, text) => format!("No icon matches “{text}”."),
    }
}

/// The SVG of the arrow of the popover: a triangle that points up, 22 × 11.
const ARROW_SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"22\" height=\"11\" \
viewBox=\"0 0 22 11\"><path d=\"M0 11 L11 0 L22 11 Z\" fill=\"#000\"/></svg>";
/// The SVG of the arrow when it points down.
const ARROW_DOWN_SVG: &[u8] =
    b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"22\" height=\"11\" \
viewBox=\"0 0 22 11\"><path d=\"M0 0 L11 11 L22 0 Z\" fill=\"#000\"/></svg>";
/// The width and the height of the arrow: the half of a square of 16 that
/// turns by 45°.
const ARROW_WIDTH: f32 = 22.;
const ARROW_HEIGHT: f32 = 11.;

/// The place of the popover in the window.
struct Place {
    origin: gpui_kit::Point<Pixels>,
    height: Pixels,
    /// The arrow is above the popover (the popover is below the button).
    below: bool,
    /// The left edge of the arrow, from the left edge of the popover.
    arrow_x: Pixels,
}

/// The place of the popover next to `button` in a window of `viewport`:
/// below the button if it fits there, else above it if it fits there, else
/// on the side with more space, with a smaller height.
fn place(button: Bounds<Pixels>, viewport: gpui_kit::Size<Pixels>) -> Place {
    let margin = theme::ICON_PICKER_WINDOW_MARGIN;
    let width = theme::ICON_PICKER_WIDTH;
    let gap = px(ARROW_HEIGHT + 1.);
    let below_space = viewport.height - margin - (button.bottom() + gap);
    let above_space = button.top() - gap - margin;
    let full = theme::ICON_PICKER_HEIGHT;
    let below = below_space >= full || (above_space < full && below_space >= above_space);
    let height = full
        .min(if below { below_space } else { above_space })
        .max(theme::ICON_PICKER_MIN_HEIGHT);
    let centre = button.center().x;
    let left = (centre - width / 2.)
        .min(viewport.width - margin - width)
        .max(margin);
    let top = if below {
        button.bottom() + gap
    } else {
        button.top() - gap - height
    };
    let arrow_x = (centre - left - px(ARROW_WIDTH / 2.))
        .max(theme::SPACE_16)
        .min(width - theme::SPACE_16 - px(ARROW_WIDTH));
    Place {
        origin: point(left, top),
        height,
        below,
        arrow_x,
    }
}

impl DeviceView {
    /// The row "Icon" of "Appearance" (section 10.2): a tile of 28 with the
    /// icon on the fill colour, "Choose icon…", which opens the icon
    /// picker, and "Image file…".
    pub(super) fn icon_row(
        &self,
        id: SurfaceId,
        content: &SurfaceContent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open = self.inspector.icon_picker.open;
        let bounds = self.inspector.icon_picker.button.clone();
        let fill = content.color.map(colour_rgba);
        let icon = content.known_icon();
        let tile = div()
            .id("icon-tile")
            .debug_selector(|| "icon-tile".into())
            .flex_shrink_0()
            .size(theme::ICON_TILE)
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme::RADIUS_CONTROL)
            .bg(fill.unwrap_or(theme::BG_FIELD))
            .border_1()
            .border_color(theme::BORDER_FIELD)
            .map(|d| match (icon, content.image.is_some()) {
                (Some(icon), _) => {
                    let title: SharedString = format!("Icon “{}”", icon.title()).into();
                    d.child(
                        gpui_kit::svg()
                            .path(theme::surface_icon_path(icon.name))
                            .size(theme::ICON_TILE_ICON)
                            .text_color(colour_rgba(icon_color(content))),
                    )
                    .tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
                }
                (None, true) => d
                    .child(
                        gpui_kit::component::Icon::new(gpui_kit::assets::IconName::Image)
                            .size(theme::ICON_BUTTON)
                            .text_color(theme::TEXT_SECONDARY),
                    )
                    .tooltip(|window, cx| Tooltip::new("Image file").build(window, cx)),
                (None, false) => d,
            });
        let choose = div()
            .relative()
            .flex_shrink_0()
            .child(
                button("choose-icon", "Choose icon…")
                    .when(open, |d| d.border_color(theme::ACCENT_LINE))
                    .on_click(cx.listener(move |view, _, window, cx| {
                        if view.inspector.icon_picker.open {
                            view.close_icon_picker(window, cx);
                        } else {
                            view.open_icon_picker(window, cx);
                        }
                    })),
            )
            // The bounds of the button, for the place of the popover.
            .child(
                canvas(move |b, _, _| bounds.set(Some(b)), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            )
            .children(open.then(|| self.icon_picker(id, content, window, cx)));
        div()
            .flex()
            .items_center()
            .gap(theme::GAP_ITEMS)
            .child(tile)
            .child(choose)
            .child(
                button("choose-image", "Image file…")
                    .on_click(cx.listener(|view, _, window, cx| view.dialog(window, cx))),
            )
            .into_any_element()
    }

    /// Open the icon picker on the tab "Icons", with an empty search field
    /// that has the focus.
    fn open_icon_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_kinds();
        let picker = &mut self.inspector.icon_picker;
        picker.open = true;
        picker.tab = PickerTab::Icons;
        picker.colour = None;
        let search = picker.search.clone();
        search.update(cx, |search, cx| {
            search.set_value("", window, cx);
            search.focus(window, cx);
        });
        self.refresh_icon_rows(cx);
        cx.notify();
    }

    /// Close the icon picker, and give the focus back to the window.
    fn close_icon_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.inspector.icon_picker.open = false;
        self.focus_workspace(window, cx);
        cx.notify();
    }

    /// Make the rows of the grid again, for the tab, the search text, and
    /// the recent icons, and show the start of the grid.
    pub(super) fn refresh_icon_rows(&mut self, cx: &mut Context<Self>) {
        let query = self
            .inspector
            .icon_picker
            .search
            .read(cx)
            .value()
            .to_string();
        let recent = self.shell.read(cx).recent_icons().to_vec();
        let picker = &mut self.inspector.icon_picker;
        let (rows, count) = grid_rows(picker.tab, &query, &recent);
        if *picker.rows != rows {
            picker.list.reset(rows.len());
            picker.rows = Rc::new(rows);
        }
        picker.count = count;
    }

    /// Select the tab `tab` of the popover.
    fn select_icon_tab(&mut self, tab: PickerTab, cx: &mut Context<Self>) {
        self.inspector.icon_picker.tab = tab;
        self.refresh_icon_rows(cx);
        cx.notify();
    }

    /// Set the icon `name` on the surface `id`, with the colour of the
    /// current icon, or the colour that the user selected in the popover.
    fn choose_icon(&mut self, id: SurfaceId, name: &'static str, cx: &mut Context<Self>) {
        let current = self.state.surfaces.get(&id).map(|s| &s.content);
        let color = match current.and_then(|c| c.icon.as_ref()) {
            Some(icon) => icon.color,
            None => self.inspector.icon_picker.colour,
        };
        (self.send)(Command::SetIcon {
            surface: id,
            icon: Some(SurfaceIcon {
                name: name.into(),
                color,
            }),
        });
        self.shell
            .update(cx, |shell, cx| shell.add_recent_icon(name, cx));
        if self.inspector.icon_picker.tab == PickerTab::Recent {
            self.refresh_icon_rows(cx);
        }
        cx.notify();
    }

    /// Set the colour of the icon of the surface `id`. With no icon, keep
    /// the colour for the next icon.
    fn choose_icon_colour(&mut self, id: SurfaceId, color: Color, cx: &mut Context<Self>) {
        let icon = self
            .state
            .surfaces
            .get(&id)
            .and_then(|s| s.content.icon.clone());
        match icon {
            Some(icon) => (self.send)(Command::SetIcon {
                surface: id,
                icon: Some(SurfaceIcon {
                    color: Some(color),
                    ..icon
                }),
            }),
            None => self.inspector.icon_picker.colour = Some(color),
        }
        cx.notify();
    }

    /// The popover of the icon picker (section 14), next to the button
    /// "Choose icon…".
    fn icon_picker(
        &self,
        id: SurfaceId,
        content: &SurfaceContent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let picker = &self.inspector.icon_picker;
        let viewport = window.viewport_size();
        let Some(button) = picker.button.get() else {
            // The first frame has no bounds of the button yet.
            window.request_animation_frame();
            return div().into_any_element();
        };
        let place = place(button, viewport);
        let close = cx.listener(|view, _: &gpui_kit::MouseDownEvent, window, cx| {
            view.close_icon_picker(window, cx)
        });
        let arrow = |data: &'static [u8], colour: Rgba| {
            gpui_kit::svg()
                .data(data)
                .w(px(ARROW_WIDTH))
                .h(px(ARROW_HEIGHT))
                .text_color(colour)
        };
        // The border of the arrow, and its fill one pixel nearer to the
        // popover. The fill covers the border of the popover below the
        // arrow.
        let (data, border_y, fill_y) = if place.below {
            (ARROW_SVG, -px(ARROW_HEIGHT), -px(ARROW_HEIGHT - 1.))
        } else {
            (ARROW_DOWN_SVG, place.height, place.height - px(1.))
        };
        // An absolute element has its place from its parent (Taffy), thus
        // the layer of the arrow covers the popover.
        let arrows = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(
                div()
                    .absolute()
                    .left(place.arrow_x)
                    .top(border_y)
                    .child(arrow(data, theme::BORDER_OVERLAY)),
            )
            .child(
                div()
                    .absolute()
                    .left(place.arrow_x)
                    .top(fill_y)
                    .child(arrow(data, theme::BG_FIELD)),
            );
        div()
            .child(
                deferred(
                    anchored().position(point(px(0.), px(0.))).child(
                        div()
                            .id("icon-picker-backdrop")
                            .w(viewport.width)
                            .h(viewport.height)
                            .occlude()
                            .on_mouse_down(MouseButton::Left, close),
                    ),
                )
                .with_priority(1),
            )
            .child(
                deferred(
                    anchored().position(place.origin).child(
                        div()
                            .id("icon-picker")
                            .debug_selector(|| "icon-picker".into())
                            .occlude()
                            .relative()
                            .w(theme::ICON_PICKER_WIDTH)
                            .h(place.height)
                            .capture_key_down(cx.listener(
                                |view, event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.key == "escape" {
                                        cx.stop_propagation();
                                        view.close_icon_picker(window, cx);
                                    }
                                },
                            ))
                            .child(
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .rounded(theme::RADIUS_POPOVER)
                                    .bg(theme::BG_FIELD)
                                    .border_1()
                                    .border_color(theme::BORDER_OVERLAY)
                                    .shadow(theme::popover_shadow())
                                    .text_size(theme::TEXT_CONTROL)
                                    .font_weight(gpui_kit::FontWeight::NORMAL)
                                    .text_color(theme::TEXT_PRIMARY)
                                    .child(self.icon_picker_header(window, cx))
                                    .child(self.icon_grid(id, content, cx))
                                    .child(self.icon_picker_footer(id, content, cx)),
                            )
                            .child(arrows),
                    ),
                )
                .with_priority(2),
            )
            .into_any_element()
    }

    /// The header: the tab chips and the search field with the number of
    /// icons.
    fn icon_picker_header(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let picker = &self.inspector.icon_picker;
        let tab = |id: &'static str, text: &'static str, value: PickerTab| {
            let selected = picker.tab == value;
            controls::chip(id, text, selected).when(!selected, |d| {
                d.on_click(cx.listener(move |view, _, _, cx| view.select_icon_tab(value, cx)))
            })
        };
        div()
            .flex_shrink_0()
            .pt(theme::SPACE_12)
            .px(theme::SPACE_12)
            .pb(theme::SPACE_10)
            .flex()
            .flex_col()
            .gap(theme::SPACE_10)
            .border_b_1()
            .border_color(theme::BORDER_FIELD)
            .child(
                // The tabs "Packs" and "GIF" are places for later: more
                // chips in this row.
                div()
                    .flex()
                    .gap(theme::SPACE_4)
                    .child(tab("icon-tab-icons", "Icons", PickerTab::Icons))
                    .child(tab("icon-tab-recent", "Recent", PickerTab::Recent)),
            )
            .child(
                controls::search_field(
                    "icon-search",
                    &picker.search,
                    theme::HEIGHT_SEARCH_FIELD,
                    window,
                    cx,
                )
                .bg(theme::BG_BAR)
                .child(
                    div()
                        .id("icon-count")
                        .debug_selector(|| "icon-count".into())
                        .flex_shrink_0()
                        .text_size(theme::TEXT_CAPTION)
                        .text_color(theme::TEXT_TERTIARY)
                        .child(count_text(picker.count)),
                ),
            )
    }

    /// The grid of icons, or the text of a grid with no icon.
    fn icon_grid(&self, id: SurfaceId, content: &SurfaceContent, cx: &mut Context<Self>) -> Div {
        let picker = &self.inspector.icon_picker;
        let frame = div().flex_1().min_h_0().relative();
        if picker.rows.is_empty() {
            let query = picker.search.read(cx).value().to_string();
            return frame.child(
                div()
                    .id("icon-empty")
                    .debug_selector(|| "icon-empty".into())
                    .pt(theme::EMPTY_STATE_TOP)
                    .px(theme::SPACE_16)
                    .text_center()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_TERTIARY)
                    .child(empty_text(picker.tab, &query)),
            );
        }
        let rows = picker.rows.clone();
        let current = content
            .known_icon()
            .map(|i| i.name)
            .filter(|_| content.image.is_none());
        let view = cx.entity();
        let last = rows.len() - 1;
        frame
            .child(
                list(picker.list.clone(), move |ix, _, cx| {
                    view.update(cx, |view, cx| {
                        view.icon_grid_row(id, &rows[ix], ix, ix == last, current, cx)
                    })
                })
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(Scrollbar::vertical(&picker.list)),
            )
    }

    /// One row of the grid: the title of a group, or up to 7 cells.
    fn icon_grid_row(
        &self,
        id: SurfaceId,
        row: &GridRow,
        index: usize,
        last: bool,
        current: Option<&'static str>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let padding = div()
            .px(theme::SPACE_12)
            .when(index == 0, |d| d.pt(theme::SPACE_12))
            .when(last, |d| d.pb(theme::SPACE_12));
        match row {
            GridRow::Title(title) => padding
                .when(index > 0, |d| d.pt(theme::SPACE_8))
                .pb(theme::SPACE_6)
                .child(
                    div()
                        .text_size(theme::TEXT_SECTION)
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .text_color(theme::TEXT_TERTIARY)
                        .child(title.to_uppercase()),
                )
                .into_any_element(),
            GridRow::Icons(icons) => {
                padding
                    .when(!last, |d| d.pb(theme::SPACE_6))
                    .child(
                        div()
                            .grid()
                            .grid_cols(theme::ICON_GRID_COLUMNS as u16)
                            .gap(theme::SPACE_6)
                            .children(icons.iter().map(|icon| {
                                self.icon_cell(id, icon, current == Some(icon.name), cx)
                            })),
                    )
                    .into_any_element()
            }
        }
    }

    /// One cell of the grid: radius 8, `bg-bar`, border `line-pane`, the
    /// icon 20 in `text-icon-cell`. The icon of the surface has `accent`,
    /// border `accent-line`, and a white icon.
    fn icon_cell(
        &self,
        id: SurfaceId,
        icon: &'static Icon,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let name = icon.name;
        let title: SharedString = icon.title().into();
        let cell_id: SharedString = format!("icon-cell-{name}").into();
        div()
            .id(cell_id.clone())
            .debug_selector(move || cell_id.to_string())
            .h(theme::ICON_CELL_HEIGHT)
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme::RADIUS_CARD)
            .border_1()
            .cursor_pointer()
            .group(CELL_GROUP)
            .map(|d| {
                if selected {
                    d.bg(theme::ACCENT).border_color(theme::ACCENT_LINE)
                } else {
                    d.bg(theme::BG_BAR)
                        .border_color(theme::LINE_PANE)
                        .hover(|d| {
                            d.bg(theme::HOVER_ICON_CELL)
                                .border_color(theme::BORDER_CONTROL)
                        })
                        .active(|d| d.bg(theme::PRESSED))
                }
            })
            .tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
            .on_click(cx.listener(move |view, _, _, cx| view.choose_icon(id, name, cx)))
            .child(
                // An SVG takes the colour of its own text style, not of its
                // parent.
                gpui_kit::svg()
                    .path(theme::surface_icon_path(name))
                    .size(theme::ICON_CELL_ICON)
                    .map(|d| {
                        if selected {
                            d.text_color(theme::TEXT_ON_ACCENT)
                        } else {
                            d.text_color(theme::TEXT_ICON_CELL)
                                .group_hover(CELL_GROUP, |d| d.text_color(theme::TEXT_PRIMARY))
                        }
                    }),
            )
    }

    /// The footer: the icon colours, "Image file…", and "No icon".
    fn icon_picker_footer(
        &self,
        id: SurfaceId,
        content: &SurfaceContent,
        cx: &mut Context<Self>,
    ) -> Div {
        // The colour that the icon shows, or the colour for the next icon.
        let shown = match (&content.icon, self.inspector.icon_picker.colour) {
            (None, Some(colour)) => colour,
            _ => icon_color(content),
        };
        let circles = theme::ICON_COLOURS
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let colour = super::colour(value);
                controls::colour_circle(
                    format!("icon-colour-{index}"),
                    Some(rgb(value)),
                    colour == shown,
                    format!("Icon colour #{value:06X}"),
                )
                .when(value == theme::ICON_COLOUR_WITH_BORDER, |d| {
                    d.border_1().border_color(theme::BORDER_SAMPLE_OTHER)
                })
                .on_click(
                    cx.listener(move |view, _, _, cx| view.choose_icon_colour(id, colour, cx)),
                )
            });
        div()
            .flex_shrink_0()
            .px(theme::SPACE_12)
            .py(theme::SPACE_10)
            .flex()
            .flex_col()
            .gap(theme::SPACE_10)
            .border_t_1()
            .border_color(theme::BORDER_FIELD)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_8)
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(theme::FORM_LABEL)
                            .text_size(theme::TEXT_LABEL)
                            .text_color(theme::TEXT_SECONDARY)
                            .child("Icon colour"),
                    )
                    .children(circles),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        footer_button("icon-image-file", "Image file…", theme::BG_IMAGE_BUTTON)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.close_icon_picker(window, cx);
                                view.dialog(window, cx);
                            })),
                    )
                    .child(
                        footer_button("icon-no-icon", "No icon", theme::TRANSPARENT).on_click(
                            cx.listener(move |view, _, window, cx| {
                                (view.send)(Command::SetIcon {
                                    surface: id,
                                    icon: None,
                                });
                                view.close_icon_picker(window, cx);
                            }),
                        ),
                    ),
            )
    }
}

/// A button of the footer of the popover: height 28, padding 0 10, radius
/// 6, border `bg-segment-on`, `label` weight 500, on the background
/// `background`. Under the pointer, it is a little lighter.
fn footer_button(id: &'static str, text: &'static str, background: Rgba) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex_shrink_0()
        .h(theme::HEIGHT_CONTROL)
        .px(theme::SPACE_10)
        .flex()
        .items_center()
        .rounded(theme::RADIUS_CONTROL)
        .bg(background)
        .border_1()
        .border_color(theme::BG_SEGMENT_ON)
        .text_size(theme::TEXT_LABEL)
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .text_color(theme::TEXT_PRIMARY)
        .cursor_pointer()
        .hover(|d| d.bg(theme::HOVER_CONTROL))
        .active(|d| d.bg(theme::PRESSED))
        .child(text)
}

/// A colour of the engine as a colour of the window.
fn colour_rgba(colour: Color) -> Rgba {
    let [r, g, b] = colour.0;
    rgb(u32::from_be_bytes([0, r, g, b]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_count_has_a_comma_for_each_thousand() {
        assert_eq!(count_text(1818), "1,818 icons");
        assert_eq!(count_text(1), "1 icon");
        assert_eq!(count_text(0), "0 icons");
        assert_eq!(count_text(42), "42 icons");
        assert_eq!(count_text(1234567), "1,234,567 icons");
    }

    #[test]
    fn the_grid_has_groups_with_no_text_and_the_results_with_a_text() {
        let (rows, count) = grid_rows(PickerTab::Icons, "", &[]);
        assert_eq!(count, icon_set().icons().len());
        assert!(matches!(rows[0], GridRow::Title(_)));
        let titles = rows
            .iter()
            .filter(|r| matches!(r, GridRow::Title(_)))
            .count();
        assert_eq!(titles, icon_set().groups().len());
        assert!(rows.iter().all(|r| match r {
            GridRow::Icons(icons) => (1..=theme::ICON_GRID_COLUMNS).contains(&icons.len()),
            GridRow::Title(_) => true,
        }));

        let (rows, count) = grid_rows(PickerTab::Icons, "globe", &[]);
        assert!(count > 1);
        assert!(rows.iter().all(|r| matches!(r, GridRow::Icons(_))));
        let GridRow::Icons(first) = &rows[0] else {
            panic!()
        };
        assert_eq!(first[0].name, "globe");

        let (rows, count) = grid_rows(PickerTab::Icons, "zzzz", &[]);
        assert_eq!((rows.len(), count), (0, 0));
        assert_eq!(
            empty_text(PickerTab::Icons, " zzzz "),
            "No icon matches “zzzz”."
        );
    }

    #[test]
    fn the_recent_tab_shows_the_recent_icons_in_their_order() {
        let recent = ["mail".to_string(), "globe".into(), "not-an-icon".into()];
        let (rows, count) = grid_rows(PickerTab::Recent, "", &recent);
        assert_eq!(count, 2);
        let GridRow::Icons(icons) = &rows[0] else {
            panic!()
        };
        let names: Vec<_> = icons.iter().map(|i| i.name).collect();
        assert_eq!(names, ["mail", "globe"]);
        let (_, count) = grid_rows(PickerTab::Recent, "glo", &recent);
        assert_eq!(count, 1);
        assert_eq!(
            empty_text(PickerTab::Recent, ""),
            "The icons that you choose show here."
        );
    }

    #[test]
    fn the_popover_goes_below_the_button_or_above_it() {
        let viewport = gpui_kit::size(px(1440.), px(900.));
        let button = Bounds::new(
            point(px(1300.), px(200.)),
            gpui_kit::size(px(100.), px(28.)),
        );
        let below = place(button, viewport);
        assert!(below.below);
        assert_eq!(below.height, theme::ICON_PICKER_HEIGHT);
        assert_eq!(below.origin.y, px(240.));
        // The popover stays in the window, and the arrow points to the
        // centre of the button.
        assert_eq!(
            below.origin.x + theme::ICON_PICKER_WIDTH,
            px(1440.) - theme::ICON_PICKER_WINDOW_MARGIN
        );
        assert_eq!(
            below.origin.x + below.arrow_x + px(ARROW_WIDTH / 2.),
            px(1350.)
        );
        // Low in the window: above the button.
        let button = Bounds::new(
            point(px(1180.), px(700.)),
            gpui_kit::size(px(100.), px(28.)),
        );
        let above = place(button, viewport);
        assert!(!above.below);
        assert_eq!(above.origin.y + above.height, px(700. - ARROW_HEIGHT - 1.));
        // A low window: the side with more space, and a smaller height.
        let viewport = gpui_kit::size(px(1080.), px(700.));
        let button = Bounds::new(point(px(800.), px(300.)), gpui_kit::size(px(100.), px(28.)));
        let small = place(button, viewport);
        assert!(small.below);
        assert!(small.height < theme::ICON_PICKER_HEIGHT);
        assert!(small.origin.y + small.height <= px(700.) - theme::ICON_PICKER_WINDOW_MARGIN);
    }
}
