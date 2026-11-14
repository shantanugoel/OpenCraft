//! The canvas in the centre (decision 0029, section 8): the page header,
//! the device drawing, the help line, the messages, and the status bar.
//!
//! The drawing has one scale for all its parts. Each key face and bar panel
//! is at its position in the layout of the engine, and each knob is below
//! its feedback surface. A control with no surface, other than a knob or
//! the touch bar, is below the display in the order of its row and its
//! column. The shell does not check for a model (rule 4).

use super::{controls::square_button, library::KindDrag, pages::in_profile, *};
use crate::settings::RightPanelTab;
use gpui_kit::{assets::IconName, component::Icon, FontWeight, MouseButton, MouseDownEvent, Rgba};
use opencraft_engine::{DeviceLayout, DeviceSettings, KnobLightMode, SurfaceLayout};

/// The help line when no knob is selected (section 8.4).
pub(super) const HELP_NORMAL: &str = "Click a control to change it · Drag an action onto a control · Right-click for Copy, Paste and Clear";

/// The help line while a row of the library is in a drag (section 8.4).
pub(super) const HELP_DRAG: &str =
    "Drop on a key, a bar panel or a knob. Drop on a key with an action to replace it. ⌘Z undoes it.";

/// The text "Drop here" of the drop target.
pub(super) const DROP_HERE: &str = "Drop here";

/// The parts of the body below the display: in pixels of the display (they
/// change with the scale), and in points (they do not change).
/// The tooltip and the accessible label of the touch bar control.
const TOUCH_BAR_LABEL: &str = "Touch bar: swipe gestures";

/// The accessible label of the display of the device drawing.
const DISPLAY: &str = "Display";

fn body_below_display(layout: &DeviceLayout) -> (f32, f32) {
    let knobs = layout
        .controls
        .iter()
        .any(|c| c.id.kind == ControlKind::Knob);
    let plain = plain_block(layout).map(|block| block.height);
    match (knobs, plain) {
        (true, Some(height)) => (
            theme::DEVICE_KNOB_TOP
                + theme::DEVICE_KNOB_DIAMETER
                + theme::DEVICE_KNOB_LABEL_GAP
                + theme::DEVICE_PLAIN_TOP
                + height
                + theme::DEVICE_BODY_BOTTOM,
            theme::DEVICE_KNOB_LABEL_HEIGHT,
        ),
        (true, None) => (
            theme::DEVICE_KNOB_TOP
                + theme::DEVICE_KNOB_DIAMETER
                + theme::DEVICE_KNOB_LABEL_GAP
                + theme::DEVICE_BODY_BOTTOM,
            theme::DEVICE_KNOB_LABEL_HEIGHT,
        ),
        (false, Some(height)) => (
            theme::DEVICE_PLAIN_TOP + height + theme::DEVICE_BODY_BOTTOM,
            0.,
        ),
        (false, None) => (theme::DEVICE_BODY_MARGIN, 0.),
    }
}

/// The scale of the drawing for a device area of `width` × `height`
/// (section 8.2): the largest value between 0.5 and 1.0 for which the body
/// fits with [`theme::DEVICE_MARGIN`] at each side and the help line below.
pub(super) fn device_scale(layout: &DeviceLayout, width: f32, height: f32) -> f32 {
    let margins = 2. * theme::DEVICE_MARGIN;
    let (below, fixed) = body_below_display(layout);
    let body_width = f32::from(layout.display_width.max(1)) + 2. * theme::DEVICE_BODY_MARGIN;
    let body_height = theme::DEVICE_BODY_MARGIN + f32::from(layout.display_height.max(1)) + below;
    let help = f32::from(theme::DEVICE_AREA_GAP) + theme::HELP_LINE_HEIGHT;
    ((width - margins) / body_width)
        .min((height - margins - help - fixed) / body_height)
        .clamp(theme::DEVICE_SCALE_MIN, theme::DEVICE_SCALE_MAX)
}

/// The size of the body of the device at `scale`, in points.
pub(super) fn body_size(layout: &DeviceLayout, scale: f32) -> (f32, f32) {
    let (below, fixed) = body_below_display(layout);
    (
        (f32::from(layout.display_width) + 2. * theme::DEVICE_BODY_MARGIN) * scale,
        (theme::DEVICE_BODY_MARGIN + f32::from(layout.display_height) + below) * scale + fixed,
    )
}

/// The horizontal centre of a knob, in pixels of the display: below the
/// centre of its feedback surface. A knob with no feedback surface gets an
/// equal part of the width of the display, from its column.
pub(super) fn knob_centre(layout: &DeviceLayout, knob: &opencraft_engine::ControlLayout) -> f32 {
    if let Some(surface) = knob.feedback_surface.and_then(|id| layout.surface(id)) {
        return f32::from(surface.x) + f32::from(surface.width) / 2.;
    }
    let columns = layout
        .controls
        .iter()
        .filter(|c| c.id.kind == ControlKind::Knob)
        .map(|c| c.column)
        .max()
        .unwrap_or(1)
        .max(1);
    f32::from(layout.display_width) * (f32::from(knob.column.max(1)) - 0.5) / f32::from(columns)
}

/// One control that has no surface, other than a knob or the touch bar.
/// `x` and `y` are the top-left corner: `x` from the left of the display,
/// `y` from the top of the block. `size` is the side of the square. All
/// three are pixels of the display.
#[derive(Clone, Copy)]
struct PlainPlace {
    id: ControlId,
    x: f32,
    y: f32,
    size: f32,
}

struct PlainBlock {
    /// The height of the block, in pixels of the display.
    height: f32,
    places: Vec<PlainPlace>,
}

/// The controls with no surface, other than a knob or the touch bar, in
/// row and column order. Row 1 is the top row. In one row, column 1 is the
/// left control. The controls of one row share the width of the display.
///
/// A knob and the touch bar have their own elements (section 8.2). The
/// shell does not check for a model (rule 4).
fn plain_block(layout: &DeviceLayout) -> Option<PlainBlock> {
    let mut controls: Vec<_> = layout
        .controls
        .iter()
        .filter(|control| {
            control.surface.is_none()
                && !matches!(control.id.kind, ControlKind::Knob | ControlKind::TouchBar)
        })
        .collect();
    if controls.is_empty() {
        return None;
    }
    controls.sort_by_key(|control| (control.row, control.column, control.id));
    let mut rows: Vec<Vec<&opencraft_engine::ControlLayout>> = Vec::new();
    for control in controls {
        match rows.last_mut() {
            Some(row) if row[0].row == control.row => row.push(control),
            _ => rows.push(vec![control]),
        }
    }
    let max_n = rows.iter().map(|row| row.len()).max().unwrap_or(1).max(1) as f32;
    let gap = theme::DEVICE_PLAIN_GAP;
    let width = f32::from(layout.display_width.max(1));
    let fitted = (width - gap * (max_n - 1.)).max(0.) / max_n;
    let size = fitted.min(theme::DEVICE_PLAIN_SIZE);
    let mut places = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let count = row.len() as f32;
        let y = index as f32 * (size + gap);
        for (column, control) in row.iter().enumerate() {
            let centre = width * (column as f32 + 0.5) / count;
            places.push(PlainPlace {
                id: control.id,
                x: centre - size / 2.,
                y,
                size,
            });
        }
    }
    let row_count = rows.len() as f32;
    let height = row_count * size + (row_count - 1.).max(0.) * gap;
    Some(PlainBlock { height, places })
}

/// The top of the block of [`plain_block`], in points from the top of the
/// body.
fn plain_origin(layout: &DeviceLayout, scale: f32) -> f32 {
    let display_bottom =
        theme::DEVICE_BODY_MARGIN * scale + f32::from(layout.display_height) * scale;
    let knobs = layout
        .controls
        .iter()
        .any(|control| control.id.kind == ControlKind::Knob);
    if knobs {
        let label_top = display_bottom
            + (theme::DEVICE_KNOB_TOP + theme::DEVICE_KNOB_DIAMETER + theme::DEVICE_KNOB_LABEL_GAP)
                * scale;
        label_top + theme::DEVICE_KNOB_LABEL_HEIGHT + theme::DEVICE_PLAIN_TOP * scale
    } else {
        display_bottom + theme::DEVICE_PLAIN_TOP * scale
    }
}

fn plain_radius(kind: ControlKind) -> f32 {
    if kind == ControlKind::BarPanel {
        theme::DEVICE_PANEL_RADIUS
    } else {
        theme::DEVICE_KEY_RADIUS
    }
}

/// The vertical centre of the "◀ Touch bar ▶" control, in pixels of the
/// display: in the middle of the space between the lowest key face and the
/// highest bar panel.
pub(super) fn touch_bar_centre(layout: &DeviceLayout) -> f32 {
    let keys = layout
        .surfaces
        .iter()
        .filter(|s| s.id.kind == SurfaceKind::KeyFace)
        .map(|s| s.y + s.height)
        .max()
        .unwrap_or(0);
    let panels = layout
        .surfaces
        .iter()
        .filter(|s| s.id.kind == SurfaceKind::BarPanel)
        .map(|s| s.y)
        .min()
        .unwrap_or(layout.display_height);
    (f32::from(keys) + f32::from(panels)) / 2.
}

/// The colour of the ring of a knob: the colour of its light in the knob
/// light mode "Colours", else neutral. A black light is off.
pub(crate) fn knob_ring(settings: Option<&DeviceSettings>, knob: ControlId) -> Rgba {
    settings
        .filter(|s| s.knob_lights.mode == KnobLightMode::Colors)
        .and_then(|s| {
            s.knob_lights
                .colors
                .get(usize::from(knob.number.max(1) - 1))
        })
        .filter(|c| **c != Color::BLACK)
        .map_or(theme::DEVICE_KNOB_RING, |c| theme::colour(*c))
}

/// An outline around an element of `width` × `height` with the corner
/// `radius`: a line of `line`, `gap` outside the element. The parent of the
/// outline must have no border.
fn outline(width: f32, height: f32, radius: f32, gap: f32, line: f32, colour: Rgba) -> Div {
    let inset = gap + line;
    div()
        .absolute()
        .left(px(-inset))
        .top(px(-inset))
        .w(px(width + 2. * inset))
        .h(px(height + 2. * inset))
        .rounded(px(radius + inset))
        .border(px(line))
        .border_color(colour)
}

/// The outline of a control under the pointer. It shows only while the
/// pointer is on the element of the group `group`.
fn hover_outline(group: SharedString, width: f32, height: f32, radius: f32, gap: f32) -> Div {
    outline(
        width,
        height,
        radius,
        gap,
        theme::HOVER_OUTLINE_WIDTH,
        theme::TRANSPARENT,
    )
    .group_hover(group, |s| s.border_color(theme::ACCENT_LINE_HOVER))
}

/// The drop target on a control with the corner `radius` (section 12). It
/// is on top of the image of the control, and it shows only while the
/// pointer of a drag from the library is on the control. GPUI gives the
/// style of `drag_over` from the hit test of the frame, thus only the control
/// under the pointer shows it.
fn drop_target(radius: f32) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .rounded(px(radius))
        .bg(theme::ACCENT_TINT_DROP)
        .border(theme::DROP_TARGET_BORDER)
        .border_dashed()
        .border_color(theme::ACCENT_LINE)
        .flex()
        .items_center()
        .justify_center()
        .text_size(theme::TEXT_CAPTION)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::TEXT_ON_ACCENT)
        .child(DROP_HERE)
        .opacity(0.)
        .drag_over::<KindDrag>(|style, _, _, _| style.opacity(1.))
}

impl DeviceView {
    /// The canvas column. `width` is the width that the panes give it.
    pub(super) fn canvas(&self, width: f32, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let mut height = f32::from(window.viewport_size().height)
            - f32::from(theme::TOOLBAR_HEIGHT)
            - f32::from(theme::STATUS_BAR_HEIGHT)
            - f32::from(theme::PAGE_HEADER_HEIGHT);
        if !self.state.problems.is_empty() {
            height -= theme::MESSAGES_HEIGHT;
        }
        div()
            .id("canvas")
            .debug_selector(|| "canvas".into())
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::BG_CANVAS)
            .child(self.page_header(cx))
            .child(
                div()
                    .id("device-area")
                    .debug_selector(|| "device-area".into())
                    // A click on an empty part selects the page: the group
                    // has the name of the page.
                    .accessible(Role::Group, self.page_name())
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .gap(theme::DEVICE_AREA_GAP)
                    // A click on an empty part of the canvas selects the
                    // page. A control stops the click.
                    .on_click(cx.listener(|view, _, window, cx| view.select_page(window, cx)))
                    .children(self.device(width, height, cx))
                    .child(self.help_line(cx)),
            )
            .children(self.messages(cx))
            .child(self.status_bar())
            .into_any_element()
    }

    /// The page header (section 8.1): the page name, "Page N of M", and the
    /// buttons for the pages. It has no content until the engine sends the
    /// pages.
    fn page_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let names = &self.state.pages;
        let header = div()
            .id("page-header")
            .debug_selector(|| "page-header".into())
            .flex_shrink_0()
            .h(theme::PAGE_HEADER_HEIGHT)
            .px(theme::SPACE_20)
            .flex()
            .items_center()
            .gap(theme::SPACE_10);
        if names.is_empty() {
            return header.into_any_element();
        }
        let page = self.state.page;
        let count = names.len();
        let previous = page > 0;
        let next = page + 1 < count;
        let page_count = format!("Page {} of {count}", page + 1);
        // The profile of this frame: a click acts on what the user saw.
        let shown = self.state.profile.clone();
        header
            .child(self.page_header_name(page, cx))
            .child(
                div()
                    .id("page-header-count")
                    .debug_selector(|| "page-header-count".into())
                    .readable_text(page_count.clone())
                    .flex_shrink_0()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_TERTIARY)
                    .child(page_count),
            )
            .child(div().flex_1())
            .child(
                square_button(
                    "previous-page",
                    IconName::ChevronLeft,
                    "Previous page",
                    previous,
                    theme::HEIGHT_SEGMENT,
                )
                .when(previous, |d| {
                    let shown = shown.clone();
                    d.on_click(cx.listener(move |view, _, _, _| {
                        (view.send)(in_profile(
                            shown.clone(),
                            Command::SelectPage { page: page - 1 },
                        ))
                    }))
                }),
            )
            .child(
                square_button(
                    "next-page",
                    IconName::ChevronRight,
                    "Next page",
                    next,
                    theme::HEIGHT_SEGMENT,
                )
                .when(next, |d| {
                    let shown = shown.clone();
                    d.on_click(cx.listener(move |view, _, _, _| {
                        (view.send)(in_profile(
                            shown.clone(),
                            Command::SelectPage { page: page + 1 },
                        ))
                    }))
                }),
            )
            .child(
                small_button("page-settings", "Page settings", true)
                    .bg(theme::BG_FIELD)
                    .border_color(theme::BORDER_FIELD)
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.select_page(window, cx);
                        view.show_inspector(window, cx);
                    })),
            )
            .into_any_element()
    }

    /// The page name, or the path of the open folders (decision 0036).
    /// The page name is a button that closes the folders. A folder that is
    /// not the last one is a button that opens that folder.
    fn page_header_name(&self, page: usize, cx: &mut Context<Self>) -> AnyElement {
        let title = pages::page_title(&self.state.pages, page);
        let open: Vec<_> = self
            .state
            .open
            .iter()
            .filter_map(|id| self.folder_name(*id).map(|name| (*id, name.to_string())))
            .collect();
        let base = div()
            .id("page-header-name")
            .debug_selector(|| "page-header-name".into())
            .min_w_0()
            .flex()
            .items_center()
            .gap(theme::SPACE_6)
            .text_size(theme::TEXT_HEADING)
            .font_weight(FontWeight::SEMIBOLD);
        if open.is_empty() {
            return base
                .readable_text(title.clone())
                .truncate()
                .child(title)
                .into_any_element();
        }
        let shown = self.state.profile.clone();
        let mut path = base.child(Self::path_button(
            "page-header-page",
            title,
            cx.listener(move |view, _, _, _| {
                (view.send)(in_profile(shown.clone(), Command::SelectPage { page }));
            }),
        ));
        for (index, (id, name)) in open.iter().enumerate() {
            path = path.child(
                div()
                    .flex_shrink_0()
                    .text_color(theme::TEXT_TERTIARY)
                    .child("›"),
            );
            let last = index + 1 == open.len();
            if last {
                path = path.child(
                    div()
                        .id("page-header-folder")
                        .debug_selector(|| "page-header-folder".into())
                        .readable_text(name.clone())
                        .min_w_0()
                        .truncate()
                        .child(name.clone()),
                );
            } else {
                let id = *id;
                let shown = self.state.profile.clone();
                path = path.child(Self::path_button(
                    format!("page-header-folder-{id}"),
                    name.clone(),
                    cx.listener(move |view, _, _, _| {
                        (view.send)(in_profile(
                            shown.clone(),
                            Command::OpenFolder { folder: id },
                        ));
                    }),
                ));
            }
        }
        path.into_any_element()
    }

    /// A name in the page path. It is a button: Space and Return operate it.
    fn path_button(
        id: impl Into<SharedString>,
        label: String,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let id: SharedString = id.into();
        let selector = id.clone();
        controls::keyboard_focus(
            div().id(id).debug_selector(move || selector.to_string()),
            controls::Ring::Inside,
        )
        .accessible(Role::Button, label.clone())
        .cursor_pointer()
        .min_w_0()
        .truncate()
        .child(label)
        .on_click(on_click)
        .into_any_element()
    }

    /// The line of help below the device (section 8.4).
    fn help_line(&self, cx: &App) -> AnyElement {
        let knob = self
            .framed_control()
            .filter(|id| id.kind == ControlKind::Knob)
            .filter(|id| {
                self.state.layout.as_ref().is_some_and(|l| {
                    l.controls
                        .iter()
                        .any(|c| c.id == *id && c.feedback_surface.is_some())
                })
            });
        let text = match knob {
            _ if self.kind_drag(cx) => HELP_DRAG.into(),
            Some(id) => format!(
                "The dashed outline shows the bar panel that gives feedback for {}",
                control_name(id)
            ),
            None => HELP_NORMAL.into(),
        };
        div()
            .id("help-line")
            .debug_selector(|| "help-line".into())
            .readable_text(text.clone())
            .flex_shrink_0()
            .h(px(theme::HELP_LINE_HEIGHT))
            .line_height(px(theme::HELP_LINE_HEIGHT))
            .px(theme::SPACE_16)
            .text_center()
            .text_size(theme::TEXT_LABEL)
            .text_color(theme::TEXT_TERTIARY)
            .truncate()
            .child(text)
            .into_any_element()
    }

    /// Show the Inspector tab of the right panel, and the panel if it is
    /// hidden.
    pub(super) fn show_inspector(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.panes.inspector_visible {
            self.toggle_inspector(window, cx);
        }
        self.select_tab(RightPanelTab::Inspector, window, cx);
    }

    /// The surface with the blue frame of the selection. The device settings
    /// are not for a surface, thus no surface has the frame while they show.
    pub(crate) fn framed_surface(&self) -> Option<SurfaceId> {
        self.state.selected.filter(|_| !self.device.open)
    }

    /// The control with no surface that has the blue frame of the selection.
    pub(crate) fn framed_control(&self) -> Option<ControlId> {
        self.state.selected_control.filter(|_| !self.device.open)
    }

    fn select(&mut self, id: SurfaceId, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.device.open) {
            cx.notify();
        }
        if self.state.select(id) {
            self.close_kinds();
            self.sync_inspector_fields(window, cx);
            self.focus_workspace(window, cx);
            cx.notify();
        }
    }

    pub(super) fn select_control(
        &mut self,
        id: ControlId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if std::mem::take(&mut self.device.open) {
            cx.notify();
        }
        if self.state.select_control(id) {
            self.close_kinds();
            // A knob shows the title of its bar panel.
            self.sync_inspector_fields(window, cx);
            self.focus_workspace(window, cx);
            cx.notify();
        }
    }

    /// Make the page the selection (section 8.3).
    pub(super) fn select_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.device.open) {
            cx.notify();
        }
        if self.state.select_page() {
            self.close_kinds();
            self.sync_inspector_fields(window, cx);
            self.sync_page_name(window, cx);
            cx.notify();
        }
        self.focus_workspace(window, cx);
    }

    /// The name of the active page, as the page header shows it.
    fn page_name(&self) -> String {
        if self.state.pages.is_empty() {
            String::new()
        } else {
            pages::page_title(&self.state.pages, self.state.page)
        }
    }

    /// The accessible label of a key face or a bar panel: the name of its
    /// control and its title, for example "Key 2, “Mail”".
    fn surface_label(&self, id: SurfaceId) -> String {
        let name = self
            .state
            .layout
            .as_ref()
            .and_then(|l| l.controls.iter().find(|c| c.surface == Some(id)))
            .map_or_else(|| id.to_string(), |c| control_name(c.id));
        let title = self
            .state
            .surfaces
            .get(&id)
            .and_then(|s| s.content.label.clone())
            .filter(|title| !title.trim().is_empty());
        match title {
            Some(title) => format!("{name}, “{title}”"),
            None => name,
        }
    }

    /// True if the key face has no label, no colour, no image, no icon, and no
    /// binding. It then shows a dashed border and a plus icon.
    fn empty_key_face(&self, id: SurfaceId) -> bool {
        if id.kind != SurfaceKind::KeyFace {
            return false;
        }
        let content = self
            .state
            .surfaces
            .get(&id)
            .is_none_or(|s| s.content.is_empty());
        let control = self
            .state
            .layout
            .as_ref()
            .and_then(|l| l.controls.iter().find(|c| c.surface == Some(id)))
            .map(|c| c.id);
        content
            && !self
                .state
                .bindings
                .keys()
                .any(|b| Some(b.control) == control)
    }

    fn surface(
        &self,
        surface: SurfaceLayout,
        scale: f32,
        knob_feedback: Option<SurfaceId>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = surface.id;
        let active = self.state.surface_active(id);
        let selected = self.framed_surface() == Some(id);
        let width = f32::from(surface.width) * scale;
        let height = f32::from(surface.height) * scale;
        let radius = scale
            * if id.kind == SurfaceKind::KeyFace {
                theme::DEVICE_KEY_RADIUS
            } else {
                theme::DEVICE_PANEL_RADIUS
            };
        let empty = self.empty_key_face(id);
        let drag = self.kind_drag(cx);
        let group: SharedString = format!("surface-{id}").into();
        div()
            .id(format!("surface-{id}"))
            .debug_selector(move || format!("surface-{id}"))
            .accessible(Role::Button, self.surface_label(id))
            .aria_selected(selected)
            .group(group.clone())
            .absolute()
            .left(px(f32::from(surface.x) * scale))
            .top(px(f32::from(surface.y) * scale))
            .w(px(width))
            .h(px(height))
            .rounded(px(radius))
            .bg(theme::DEVICE_DISPLAY)
            .cursor_pointer()
            .when(active, |d| d.shadow(theme::input_glow_shadow()))
            // The image of the engine, at the exact layout size (rule 11).
            .children(
                self.images
                    .get(&id)
                    .cloned()
                    .map(|image| img(image).size_full().rounded(px(radius))),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .rounded(px(radius))
                    .when(active, |d| d.border_2().border_color(theme::INPUT_GLOW))
                    .when(!active && empty, |d| {
                        d.border_1()
                            .border_dashed()
                            .border_color(theme::DEVICE_KEY_EMPTY_BORDER)
                    })
                    .when(!active && !empty, |d| {
                        d.border_1().border_color(theme::DEVICE_SURFACE_BORDER)
                    }),
            )
            // During a drag the plus icon is not there, so that the text
            // "Drop here" of the drop target is readable on an empty key.
            .when(empty && !drag, |d| {
                d.child(
                    div()
                        .id(format!("empty-{id}"))
                        .debug_selector(move || format!("empty-{id}"))
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme::DEVICE_KEY_EMPTY_ICON)
                        .child(
                            Icon::new(IconName::Plus)
                                .size(px(theme::DEVICE_KEY_EMPTY_ICON_SIZE * scale)),
                        ),
                )
            })
            .map(|d| {
                if selected {
                    d.child(
                        outline(
                            width,
                            height,
                            radius,
                            theme::OUTLINE_GAP,
                            theme::OUTLINE_WIDTH,
                            theme::ACCENT_LINE,
                        )
                        .debug_selector(move || format!("selection-{id}")),
                    )
                } else if knob_feedback == Some(id) {
                    d.child(
                        outline(
                            width,
                            height,
                            radius,
                            theme::OUTLINE_GAP,
                            theme::OUTLINE_WIDTH,
                            theme::ACCENT_LINE,
                        )
                        .border_dashed()
                        .debug_selector(move || format!("feedback-{id}")),
                    )
                } else if drag {
                    // The drop target shows the control under the pointer.
                    d
                } else {
                    d.child(hover_outline(
                        group,
                        width,
                        height,
                        radius,
                        theme::OUTLINE_GAP,
                    ))
                }
            })
            .when(drag, |d| {
                d.child(drop_target(radius).debug_selector(move || format!("drop-{id}")))
            })
            .on_click(cx.listener(move |view, _, window, cx| {
                cx.stop_propagation();
                view.select(id, window, cx)
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    let control = view.state.layout.as_ref().and_then(|l| {
                        l.controls
                            .iter()
                            .find(|c| c.surface == Some(id))
                            .map(|c| c.id)
                    });
                    if let Some(control) = control {
                        view.right_click_control(control, event.position, window, cx);
                    }
                }),
            )
            .on_drop(cx.listener(move |view, drag: &KindDrag, window, cx| {
                view.select(id, window, cx);
                if let Some(control) = view.state.selected_control {
                    view.drop_kind(control, &drag.kind, window, cx);
                }
            }))
            .into_any_element()
    }

    /// The device drawing (section 8.2): the body, the display with the
    /// surfaces and the touch bar control, the knobs, and each other
    /// control that has no surface.
    fn device(&self, width: f32, height: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Some(layout) = &self.state.layout else {
            return Some(
                div()
                    .text_color(theme::TEXT_TERTIARY)
                    .child("The device layout is not available yet.")
                    .into_any_element(),
            );
        };
        let scale = device_scale(layout, width, height);
        let (body_width, body_height) = body_size(layout, scale);
        let margin = theme::DEVICE_BODY_MARGIN * scale;
        let body_radius = theme::DEVICE_BODY_RADIUS * scale;
        let display_width = f32::from(layout.display_width) * scale;
        let display_height = f32::from(layout.display_height) * scale;
        // The feedback surface of a selected knob has a dashed outline.
        let knob_feedback = self
            .framed_control()
            .filter(|id| id.kind == ControlKind::Knob)
            .and_then(|id| layout.controls.iter().find(|c| c.id == id))
            .and_then(|c| c.feedback_surface);
        let mut display = div()
            .id("display")
            .debug_selector(|| "display".into())
            .accessible(Role::Group, DISPLAY)
            .absolute()
            .left(px(margin))
            .top(px(margin))
            .w(px(display_width))
            .h(px(display_height))
            .rounded(px(theme::DEVICE_DISPLAY_RADIUS * scale))
            .bg(theme::DEVICE_DISPLAY)
            // The background of the page from the engine, behind the
            // surfaces (rule 11, #191).
            .children(self.background_image.clone().map(|image| {
                img(image)
                    .debug_selector(|| "display-background".into())
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .rounded(px(theme::DEVICE_DISPLAY_RADIUS * scale))
            }))
            // A click on an empty part of the display selects the page. The
            // body of the device would select the device.
            .on_click(cx.listener(|view, _, window, cx| {
                cx.stop_propagation();
                view.select_page(window, cx)
            }));
        for surface in layout
            .surfaces
            .iter()
            .filter(|s| matches!(s.id.kind, SurfaceKind::KeyFace | SurfaceKind::BarPanel))
        {
            display = display.child(self.surface(*surface, scale, knob_feedback, cx));
        }
        display = display.children(self.touch_bar(layout, scale, display_width, cx));
        if let Some((x, y)) = self.state.touch_position() {
            let point = theme::DEVICE_TOUCH_POINT;
            display = display.child(
                div()
                    .absolute()
                    .left(px(x * scale - point / 2.))
                    .top(px(y * scale - point / 2.))
                    .size(px(point))
                    .rounded_full()
                    .bg(theme::INPUT_GLOW)
                    .shadow(theme::input_glow_shadow()),
            );
        }
        let knobs = layout
            .controls
            .iter()
            .filter(|c| c.id.kind == ControlKind::Knob)
            .map(|knob| self.knob(layout, knob, scale, cx))
            .collect::<Vec<_>>();
        let origin = plain_origin(layout, scale);
        let plain = plain_block(layout)
            .map(|block| {
                block
                    .places
                    .into_iter()
                    .map(|place| self.plain_control(place, origin, scale, cx))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Some(
            div()
                .id("device-body")
                .debug_selector(|| "device-body".into())
                // A group, not a button: VoiceOver goes into it to the
                // controls. A click on the body selects the device.
                .accessible(Role::Group, format!("Device {}", layout.name))
                .aria_selected(self.device.open)
                .flex_shrink_0()
                .relative()
                .w(px(body_width))
                .h(px(body_height))
                .rounded(px(body_radius))
                .bg(theme::DEVICE_BODY)
                .shadow(theme::device_body_shadow())
                // A click on the body, outside the display and the knobs,
                // selects the device. A control stops its click.
                .cursor_pointer()
                .on_click(cx.listener(|view, _, window, cx| {
                    cx.stop_propagation();
                    view.select_device(window, cx)
                }))
                // The border is a layer, so that the parts have their
                // positions from the edge of the body.
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .rounded(px(body_radius))
                        .border_1()
                        .border_color(theme::DEVICE_BODY_BORDER),
                )
                .child(display)
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .children(knobs)
                        .children(plain),
                )
                // The selected device: the outline of section 8.3.
                .when(self.device.open, |d| {
                    d.child(outline(
                        body_width,
                        body_height,
                        body_radius,
                        theme::OUTLINE_GAP_ROUND,
                        theme::OUTLINE_WIDTH,
                        theme::ACCENT_LINE,
                    ))
                })
                .into_any_element(),
        )
    }

    /// A control that has no surface, other than a knob or the touch bar.
    /// It is below the display, so the user can select it and give it a
    /// binding. The name is the accessible label (section 3.4).
    fn plain_control(
        &self,
        place: PlainPlace,
        origin: f32,
        scale: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = place.id;
        let size = place.size * scale;
        let left = theme::DEVICE_BODY_MARGIN * scale + place.x * scale;
        let top = origin + place.y * scale;
        let radius = plain_radius(id.kind) * scale;
        let selected = self.framed_control() == Some(id);
        // A key press is in `pressed`. A turn, a push, a tap, and a swipe
        // are in `highlights`. A key face reads both through `surface_active`.
        let highlight = self.state.pressed.contains(&id) || self.state.highlights.contains_key(&id);
        let drag = self.kind_drag(cx);
        let group: SharedString = format!("control-{id}").into();
        let name = control_name(id);
        div()
            .id(format!("control-{id}"))
            .debug_selector(move || format!("control-{id}"))
            .accessible(Role::Button, name.clone())
            .aria_selected(selected)
            .group(group.clone())
            .absolute()
            .left(px(left))
            .top(px(top))
            .size(px(size))
            .rounded(px(radius))
            .bg(theme::DEVICE_DISPLAY)
            .cursor_pointer()
            .when(highlight, |d| d.shadow(theme::input_glow_shadow()))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .rounded(px(radius))
                    .when(highlight, |d| d.border_2().border_color(theme::INPUT_GLOW))
                    .when(!highlight, |d| {
                        d.border_1()
                            .border_dashed()
                            .border_color(theme::DEVICE_KEY_EMPTY_BORDER)
                    }),
            )
            .child(
                div()
                    .w_full()
                    .px(theme::SPACE_4)
                    .text_center()
                    .truncate()
                    .text_size(theme::TEXT_CAPTION)
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme::TEXT_SECONDARY)
                    .child(name),
            )
            .map(|d| {
                if selected {
                    d.child(
                        outline(
                            size,
                            size,
                            radius,
                            theme::OUTLINE_GAP,
                            theme::OUTLINE_WIDTH,
                            theme::ACCENT_LINE,
                        )
                        .debug_selector(move || format!("selection-{id}")),
                    )
                } else if drag {
                    d
                } else {
                    d.child(hover_outline(group, size, size, radius, theme::OUTLINE_GAP))
                }
            })
            .when(drag, |d| {
                d.child(drop_target(radius).debug_selector(move || format!("drop-{id}")))
            })
            .on_click(cx.listener(move |view, _, window, cx| {
                cx.stop_propagation();
                view.select_control(id, window, cx)
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    view.right_click_control(id, event.position, window, cx);
                }),
            )
            .on_drop(cx.listener(move |view, drag: &KindDrag, window, cx| {
                view.select_control(id, window, cx);
                view.drop_kind(id, &drag.kind, window, cx);
            }))
            .into_any_element()
    }

    /// One knob and its label, below its feedback surface.
    fn knob(
        &self,
        layout: &DeviceLayout,
        knob: &opencraft_engine::ControlLayout,
        scale: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = knob.id;
        let diameter = theme::DEVICE_KNOB_DIAMETER * scale;
        let ring = theme::DEVICE_KNOB_RING_WIDTH * scale;
        let mark = theme::DEVICE_KNOB_MARK * scale;
        let margin = theme::DEVICE_BODY_MARGIN * scale;
        let centre = margin + knob_centre(layout, knob) * scale;
        let top = margin + (f32::from(layout.display_height) + theme::DEVICE_KNOB_TOP) * scale;
        let highlight = self.state.highlights.get(&id);
        let selected = self.framed_control() == Some(id);
        let drag = self.kind_drag(cx);
        let group: SharedString = format!("control-{id}").into();
        let angle =
            *self.state.knob_steps.get(&id).unwrap_or(&0) as f32 * std::f32::consts::TAU / 12.;
        // The mark turns on a circle inside the ring.
        let radius = diameter / 2. - ring - theme::DEVICE_KNOB_MARK_GAP * scale - mark / 2.;
        let middle = (diameter - mark) / 2.;
        let label_top = top + diameter + theme::DEVICE_KNOB_LABEL_GAP * scale;
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(
                div()
                    .id(format!("control-{id}"))
                    .debug_selector(move || format!("control-{id}"))
                    .accessible(Role::Button, control_name(id))
                    .aria_selected(selected)
                    .group(group.clone())
                    .absolute()
                    .left(px(centre - diameter / 2.))
                    .top(px(top))
                    .size(px(diameter))
                    .rounded_full()
                    .cursor_pointer()
                    .when(highlight.is_some(), |d| {
                        d.shadow(theme::input_glow_shadow())
                    })
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .rounded_full()
                            .bg(theme::DEVICE_KNOB)
                            .border(px(ring))
                            .border_color(if highlight.is_some() {
                                theme::INPUT_GLOW
                            } else {
                                knob_ring(self.state.device_settings.as_ref(), id)
                            }),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(middle + angle.sin() * radius))
                            .top(px(middle - angle.cos() * radius))
                            .size(px(mark))
                            .rounded_full()
                            .bg(if highlight.is_some_and(|h| h.gesture == Gesture::Push) {
                                theme::INPUT_GLOW
                            } else {
                                theme::DEVICE_KNOB_DOT
                            }),
                    )
                    .map(|d| {
                        if selected {
                            d.child(
                                outline(
                                    diameter,
                                    diameter,
                                    diameter / 2.,
                                    theme::OUTLINE_GAP_ROUND,
                                    theme::OUTLINE_WIDTH,
                                    theme::ACCENT_LINE,
                                )
                                .debug_selector(move || format!("selection-{id}")),
                            )
                        } else if drag {
                            d
                        } else {
                            d.child(hover_outline(
                                group,
                                diameter,
                                diameter,
                                diameter / 2.,
                                theme::OUTLINE_GAP_ROUND,
                            ))
                        }
                    })
                    .when(drag, |d| {
                        d.child(
                            drop_target(diameter / 2.).debug_selector(move || format!("drop-{id}")),
                        )
                    })
                    .on_click(cx.listener(move |view, _, window, cx| {
                        cx.stop_propagation();
                        view.select_control(id, window, cx)
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            view.right_click_control(id, event.position, window, cx);
                        }),
                    )
                    .on_drop(cx.listener(move |view, drag: &KindDrag, window, cx| {
                        view.select_control(id, window, cx);
                        view.drop_kind(id, &drag.kind, window, cx);
                    })),
            )
            .child(
                // A second click target of the knob. It has no role, so that
                // VoiceOver reads the knob one time (`docs/DESIGN.md`,
                // section 10).
                div()
                    .id(format!("label-{id}"))
                    .debug_selector(move || format!("label-{id}"))
                    .absolute()
                    .left(px(centre) - theme::DEVICE_KNOB_LABEL_WIDTH / 2.)
                    .top(px(label_top))
                    .w(theme::DEVICE_KNOB_LABEL_WIDTH)
                    .h(px(theme::DEVICE_KNOB_LABEL_HEIGHT))
                    .line_height(px(theme::DEVICE_KNOB_LABEL_HEIGHT))
                    .text_center()
                    .text_size(theme::TEXT_CAPTION)
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if selected {
                        theme::ACCENT_TEXT
                    } else {
                        theme::TEXT_TERTIARY
                    })
                    .cursor_pointer()
                    .child(control_name(id))
                    .on_click(cx.listener(move |view, _, window, cx| {
                        cx.stop_propagation();
                        view.select_control(id, window, cx)
                    })),
            )
            .into_any_element()
    }

    /// The "◀ Touch bar ▶" control (section 8.2). The touch bar has no
    /// surface, so this control selects it: it selects the page, which has
    /// the swipe gestures (section 10.6). It looks selected while the page is
    /// the selection. It is centred on the display, between the key faces
    /// and the bar panels.
    fn touch_bar(
        &self,
        layout: &DeviceLayout,
        scale: f32,
        display_width: f32,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let control = layout
            .controls
            .iter()
            .find(|c| c.id.kind == ControlKind::TouchBar)?;
        let id = control.id;
        let highlight = self.state.highlights.get(&id).map(|h| h.gesture);
        let selected = self.framed_control() == Some(id);
        let normal = if selected {
            theme::TEXT_ON_ACCENT
        } else {
            theme::TEXT_SECONDARY
        };
        let arrow = |gesture: Gesture, icon: IconName| {
            div()
                .flex()
                .items_center()
                .text_color(if highlight == Some(gesture) {
                    theme::INPUT_GLOW
                } else {
                    normal
                })
                .child(Icon::new(icon).size(theme::ICON_SMALL))
        };
        let top = touch_bar_centre(layout) * scale - f32::from(theme::DEVICE_TOUCH_BAR_HEIGHT) / 2.;
        Some(
            div()
                .absolute()
                .left_0()
                .top(px(top))
                .w(px(display_width))
                .flex()
                .justify_center()
                .child(
                    div()
                        .id(format!("control-{id}"))
                        .debug_selector(move || format!("control-{id}"))
                        // The label of decision 0029, section 8.2.
                        .accessible(Role::Button, TOUCH_BAR_LABEL)
                        .aria_selected(selected)
                        .h(theme::DEVICE_TOUCH_BAR_HEIGHT)
                        .px(theme::SPACE_10)
                        .flex()
                        .items_center()
                        .gap(theme::SPACE_6)
                        .rounded(theme::DEVICE_TOUCH_BAR_RADIUS)
                        .border_1()
                        .text_size(theme::TEXT_CAPTION)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(normal)
                        .cursor_pointer()
                        .map(|d| {
                            if selected {
                                d.bg(theme::ACCENT).border_color(theme::ACCENT_LINE)
                            } else {
                                d.bg(theme::NEUTRAL_TINT)
                                    .border_color(theme::DEVICE_TOUCH_BAR_BORDER)
                                    .hover(|d| d.bg(theme::DEVICE_TOUCH_BAR_HOVER))
                            }
                        })
                        .when(highlight.is_some(), |d| {
                            d.border_color(theme::INPUT_GLOW)
                                .shadow(theme::input_glow_shadow())
                        })
                        .tooltip(|window, cx| Tooltip::new(TOUCH_BAR_LABEL).build(window, cx))
                        .child(arrow(Gesture::SwipeLeft, IconName::ChevronLeft))
                        .child("Touch bar")
                        .child(arrow(Gesture::SwipeRight, IconName::ChevronRight))
                        .on_click(cx.listener(move |view, _, window, cx| {
                            cx.stop_propagation();
                            view.select_page_swipes(window, cx)
                        })),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::tests::layout;

    fn n4_size() -> DeviceLayout {
        let Event::Layout(layout) = layout() else {
            panic!()
        };
        let mut wide = (*layout).clone();
        wide.display_width = 800;
        wide.display_height = 480;
        wide
    }

    #[test]
    fn the_device_scale_is_between_its_limits() {
        let wide = n4_size();
        // The canvas of the default window: 1440 - 240 - 360, and
        // 900 - 52 - 48 - 28. The width sets the scale.
        let scale = device_scale(&wide, 840., 900. - 52. - 48. - 28.);
        assert!((scale - (840. - 48.) / 856.).abs() < 1e-4, "{scale}");
        // The minimum width of the canvas.
        let scale = device_scale(&wide, 480., 572.);
        assert!((scale - (480. - 48.) / 856.).abs() < 1e-4, "{scale}");
        assert_eq!(device_scale(&wide, 4000., 4000.), 1.0);
        assert_eq!(device_scale(&wide, 100., 100.), 0.5);
        // A low canvas: the height sets the scale, and the body and the
        // help line fit.
        let scale = device_scale(&wide, 2000., 600.);
        let (_, body) = body_size(&wide, scale);
        assert!(
            (body + 48. + 18. + theme::HELP_LINE_HEIGHT - 600.).abs() < 1e-3,
            "{body}"
        );
    }

    #[test]
    fn the_body_has_the_size_of_the_record() {
        let wide = n4_size();
        // About 857 × 690 for the N4 Pro.
        let (width, height) = body_size(&wide, 1.);
        assert_eq!(width, 856.);
        assert_eq!(height, 28. + 480. + 22. + 90. + 8. + 44. + 14.);
        // With no knob, the margin below the display is the margin at the
        // sides.
        let mut no_knob = wide.clone();
        no_knob.controls.retain(|c| c.id.kind != ControlKind::Knob);
        assert_eq!(body_size(&no_knob, 0.5), (428., 268.));
    }

    #[test]
    fn the_ring_of_a_knob_has_the_colour_of_its_light_in_the_mode_colours() {
        let knob = |n| ControlId::new(ControlKind::Knob, n);
        let mut settings = DeviceSettings {
            brightness: 80,
            screen_off_minutes: 0,
            haptic: true,
            knob_lights: opencraft_engine::KnobLights {
                mode: KnobLightMode::Colors,
                brightness: 100,
                colors: vec![Color([255, 0, 0]), Color::BLACK],
            },
        };
        assert_eq!(knob_ring(Some(&settings), knob(1)), gpui_kit::rgb(0xff0000));
        // A black light is off, and a knob with no light is neutral.
        assert_eq!(knob_ring(Some(&settings), knob(2)), theme::DEVICE_KNOB_RING);
        assert_eq!(knob_ring(Some(&settings), knob(3)), theme::DEVICE_KNOB_RING);
        assert_eq!(knob_ring(None, knob(1)), theme::DEVICE_KNOB_RING);
        settings.knob_lights.mode = KnobLightMode::Effect;
        assert_eq!(knob_ring(Some(&settings), knob(1)), theme::DEVICE_KNOB_RING);
    }

    #[test]
    fn a_control_with_no_surface_keeps_its_row_and_its_column() {
        // The N4 Pro has no such control.
        assert!(plain_block(&n4_size()).is_none());
        let mut layout = n4_size();
        layout.display_width = 300;
        layout.display_height = 200;
        layout.controls.clear();
        let bare = |kind, number, row, column| opencraft_engine::ControlLayout {
            id: ControlId::new(kind, number),
            row,
            column,
            surface: None,
            feedback_surface: None,
        };
        // The push order is not the row and column order. A knob, the touch
        // bar, and a key that has a surface are not in the block.
        layout.controls.extend([
            bare(ControlKind::Key, 8, 3, 2),
            bare(ControlKind::TouchBar, 1, 1, 1),
            bare(ControlKind::Key, 9, 4, 1),
            bare(ControlKind::Knob, 1, 1, 1),
            bare(ControlKind::Key, 7, 3, 1),
            bare(ControlKind::BarPanel, 1, 1, 1),
            opencraft_engine::ControlLayout {
                id: ControlId::new(ControlKind::Key, 1),
                row: 1,
                column: 1,
                surface: Some(SurfaceId::new(SurfaceKind::KeyFace, 1)),
                feedback_surface: None,
            },
        ]);
        let block = plain_block(&layout).unwrap();
        // Two controls in the widest row, so the square stays 96.
        assert_eq!(block.height, 3. * 96. + 2. * 16.);
        let ids: Vec<_> = block.places.iter().map(|place| place.id).collect();
        assert_eq!(
            ids,
            [
                ControlId::new(ControlKind::BarPanel, 1),
                ControlId::new(ControlKind::Key, 7),
                ControlId::new(ControlKind::Key, 8),
                ControlId::new(ControlKind::Key, 9),
            ]
        );
        // One control in a row is centred. Two controls share the width.
        // Column 1 is to the left of column 2. A higher row is lower.
        assert_eq!(block.places[0].x, 150. - 48.);
        assert_eq!(block.places[0].y, 0.);
        assert_eq!(block.places[1].x, 75. - 48.);
        assert_eq!(block.places[2].x, 225. - 48.);
        assert_eq!(block.places[1].y, 112.);
        assert_eq!(block.places[2].y, 112.);
        assert_eq!(block.places[3].x, 150. - 48.);
        assert_eq!(block.places[3].y, 224.);
        assert!(block.places.iter().all(|place| place.size == 96.));
        // No knob: the block is below the display, and the margin stays.
        let mut no_knob = layout.clone();
        no_knob.controls.retain(|c| c.id.kind != ControlKind::Knob);
        assert_eq!(
            body_size(&no_knob, 1.).1,
            28. + 200. + 22. + block.height + 44.
        );
        assert_eq!(plain_origin(&no_knob, 1.), 28. + 200. + 22.);
        // A knob keeps its place. The block is below the knob label.
        assert_eq!(
            body_size(&layout, 1.).1,
            28. + 200. + 22. + 90. + 8. + 14. + 22. + block.height + 44.
        );
        assert_eq!(
            plain_origin(&layout, 1.),
            28. + 200. + 22. + 90. + 8. + 14. + 22.
        );
    }
}
