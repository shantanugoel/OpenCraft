//! The canvas in the centre (decision 0029, section 8): the page header,
//! the device drawing, the help line, the messages, and the status bar.
//!
//! The drawing has one scale for all its parts. Each key face and bar panel
//! is at its position in the layout of the engine, and each knob is below
//! its feedback surface. The shell does not check for a model (rule 4).

use super::{controls::square_button, library::KindDrag, *};
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
fn body_below_display(layout: &DeviceLayout) -> (f32, f32) {
    if layout
        .controls
        .iter()
        .any(|c| c.id.kind == ControlKind::Knob)
    {
        (
            theme::DEVICE_KNOB_TOP
                + theme::DEVICE_KNOB_DIAMETER
                + theme::DEVICE_KNOB_LABEL_GAP
                + theme::DEVICE_BODY_BOTTOM,
            theme::DEVICE_KNOB_LABEL_HEIGHT,
        )
    } else {
        (theme::DEVICE_BODY_MARGIN, 0.)
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
pub(super) fn knob_ring(settings: Option<&DeviceSettings>, knob: ControlId) -> Rgba {
    settings
        .filter(|s| s.knob_lights.mode == KnobLightMode::Colors)
        .and_then(|s| {
            s.knob_lights
                .colors
                .get(usize::from(knob.number.max(1) - 1))
        })
        .filter(|c| **c != Color::BLACK)
        .map_or(theme::DEVICE_KNOB_RING, |c| {
            gpui_kit::rgb(u32::from(c.0[0]) << 16 | u32::from(c.0[1]) << 8 | u32::from(c.0[2]))
        })
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
        1.,
        gpui_kit::transparent_black().into(),
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
            height -= super::status_bar::MESSAGES_HEIGHT;
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
        header
            .child(
                div()
                    .id("page-header-name")
                    .debug_selector(|| "page-header-name".into())
                    .min_w_0()
                    .truncate()
                    .text_size(theme::TEXT_HEADING)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(pages::page_title(names, page)),
            )
            .child(
                div()
                    .id("page-header-count")
                    .debug_selector(|| "page-header-count".into())
                    .flex_shrink_0()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_TERTIARY)
                    .child(format!("Page {} of {count}", page + 1)),
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
                    d.on_click(cx.listener(move |view, _, _, _| {
                        (view.send)(Command::SelectPage { page: page - 1 })
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
                    d.on_click(cx.listener(move |view, _, _, _| {
                        (view.send)(Command::SelectPage { page: page + 1 })
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
        div()
            .id("help-line")
            .debug_selector(|| "help-line".into())
            .flex_shrink_0()
            .h(px(theme::HELP_LINE_HEIGHT))
            .line_height(px(theme::HELP_LINE_HEIGHT))
            .px(theme::SPACE_16)
            .text_center()
            .text_size(theme::TEXT_LABEL)
            .text_color(theme::TEXT_TERTIARY)
            .truncate()
            .child(match knob {
                _ if self.kind_drag(cx) => HELP_DRAG.into(),
                Some(id) => format!(
                    "The dashed outline shows the bar panel that gives feedback for {}",
                    control_name(id)
                ),
                None => HELP_NORMAL.into(),
            })
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
            self.load_label(window, cx);
            self.sync_form(window, cx);
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
            self.load_label(window, cx);
            self.sync_form(window, cx);
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
            self.load_label(window, cx);
            self.sync_form(window, cx);
            cx.notify();
        }
        self.focus_workspace(window, cx);
    }

    /// True if the key face has no label, no colour, no image, and no
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
            .when(empty, |d| {
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
    /// surfaces and the touch bar control, and the knobs.
    fn device(&self, width: f32, height: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Some(layout) = &self.state.layout else {
            return Some(
                div()
                    .text_color(theme::TEXT_TERTIARY)
                    .child("Load the device layout…")
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
            .absolute()
            .left(px(margin))
            .top(px(margin))
            .w(px(display_width))
            .h(px(display_height))
            .rounded(px(theme::DEVICE_DISPLAY_RADIUS * scale))
            .bg(theme::DEVICE_DISPLAY)
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
        Some(
            div()
                .id("device-body")
                .debug_selector(|| "device-body".into())
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
                        .children(knobs),
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
                        .tooltip(|window, cx| {
                            Tooltip::new("Touch bar: swipe gestures").build(window, cx)
                        })
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
}
