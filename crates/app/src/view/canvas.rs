//! The canvas in the centre (decision 0029, section 8): the page title, the
//! device drawing, the messages, and the status bar.
//!
//! The sidebar has the pages and their commands (#177). The page title is
//! in the place of the page header of #178. The drawing is the drawing of
//! Phase 6 until #178 changes it to section 8.2.

use super::*;

/// The diameter of a knob in the drawing.
const KNOB_DIAMETER: f32 = 48.;
/// The height of the row of the knobs and their labels.
const KNOB_ROW_HEIGHT: f32 = 76.;
/// The space above a knob in its row.
const KNOB_TOP: f32 = 12.;
/// The distance from the centre of a knob to its turn mark.
const KNOB_MARK_RADIUS: f32 = 14.;
/// The diameter of the turn mark of a knob.
const KNOB_MARK: f32 = 5.;
/// The width of the ring of a knob.
const KNOB_RING: f32 = 2.;
/// The space from the top of the knob row to the knob label.
const KNOB_LABEL_TOP: f32 = 52.;
/// The diameter of the mark of the touch point.
const TOUCH_POINT: f32 = 12.;
/// The height of the "Touch bar" strip below the bar panels.
const TOUCH_BAR_HEIGHT: f32 = 24.;
/// The height of the drawing that does not change with the scale: the
/// padding of the body, the touch bar strip, the knob row, and the gaps.
const DRAWING_FIXED_HEIGHT: f32 = 2. * 16. + TOUCH_BAR_HEIGHT + KNOB_ROW_HEIGHT + 2. * 8.;
/// The width of the drawing that does not change with the scale: the
/// padding of the body.
const DRAWING_FIXED_WIDTH: f32 = 2. * 16.;

/// The scale of the drawing for a device area of `width` × `height`
/// (section 8.2). The value is between 0.5 and 1.0.
pub(super) fn device_scale(
    layout: &opencraft_engine::DeviceLayout,
    width: f32,
    height: f32,
) -> f32 {
    let margins = 2. * theme::DEVICE_MARGIN;
    ((width - margins - DRAWING_FIXED_WIDTH) / f32::from(layout.display_width.max(1)))
        .min((height - margins - DRAWING_FIXED_HEIGHT) / f32::from(layout.display_height.max(1)))
        .clamp(theme::DEVICE_SCALE_MIN, theme::DEVICE_SCALE_MAX)
}

impl DeviceView {
    /// The canvas column. `width` is the width that the panes give it.
    pub(super) fn canvas(&self, width: f32, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let mut height = f32::from(window.viewport_size().height)
            - f32::from(theme::TOOLBAR_HEIGHT)
            - f32::from(theme::STATUS_BAR_HEIGHT);
        if !self.state.pages.is_empty() {
            height -= f32::from(theme::PAGE_HEADER_HEIGHT);
        }
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
            .when(!self.state.pages.is_empty(), |d| {
                d.child(
                    div()
                        .flex_shrink_0()
                        .h(theme::PAGE_HEADER_HEIGHT)
                        .px(theme::SPACE_20)
                        .flex()
                        .items_center()
                        .gap(theme::SPACE_10)
                        .child(self.page_title()),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(self.device(width, height, cx)),
            )
            .children(self.messages(cx))
            .child(self.status_bar())
            .into_any_element()
    }

    /// The name of the active page, and "Page *N* of *M*".
    fn page_title(&self) -> impl IntoElement {
        let pages = &self.state.pages;
        div()
            .id("page-title")
            .debug_selector(|| "page-title".into())
            .min_w_0()
            .flex()
            .items_baseline()
            .gap(theme::SPACE_10)
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(theme::TEXT_HEADING)
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(super::pages::page_title(pages, self.state.page)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(theme::TEXT_LABEL)
                    .text_color(theme::TEXT_TERTIARY)
                    .child(format!("Page {} of {}", self.state.page + 1, pages.len())),
            )
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

    fn select_control(&mut self, id: ControlId, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.device.open) {
            cx.notify();
        }
        if self.state.select_control(id) {
            self.close_kinds();
            self.sync_form(window, cx);
            self.focus_workspace(window, cx);
            cx.notify();
        }
    }

    /// The colour of the frame of a control: the live input, the selection,
    /// or the normal frame.
    fn frame_colour(active: bool, framed: bool, normal: gpui_kit::Rgba) -> gpui_kit::Rgba {
        if active {
            theme::INPUT_GLOW
        } else if framed {
            theme::ACCENT_LINE
        } else {
            normal
        }
    }

    fn surface(
        &self,
        surface: opencraft_engine::SurfaceLayout,
        scale: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = surface.id;
        let active = self.state.surface_active(id);
        div()
            .id(format!("surface-{id}"))
            .debug_selector(move || format!("surface-{id}"))
            .absolute()
            .left(px(f32::from(surface.x) * scale))
            .top(px(f32::from(surface.y) * scale))
            .w(px(f32::from(surface.width) * scale))
            .h(px(f32::from(surface.height) * scale))
            .bg(theme::DEVICE_DISPLAY)
            .cursor_pointer()
            // An inset overlay keeps the engine image at the exact layout size.
            .children(
                self.images
                    .get(&id)
                    .cloned()
                    .map(|image| img(image).size_full()),
            )
            .child(
                div()
                    .absolute()
                    .size_full()
                    .top_0()
                    .left_0()
                    .border_3()
                    .border_color(Self::frame_colour(
                        active,
                        self.framed_surface() == Some(id),
                        theme::DEVICE_SURFACE_BORDER,
                    )),
            )
            .on_click(cx.listener(move |view, _, window, cx| view.select(id, window, cx)))
            .into_any_element()
    }

    fn device(&self, width: f32, height: f32, cx: &mut Context<Self>) -> AnyElement {
        let Some(layout) = &self.state.layout else {
            return div()
                .text_color(theme::TEXT_TERTIARY)
                .child("Load the device layout…")
                .into_any_element();
        };
        let scale = device_scale(layout, width, height);
        let display_width = f32::from(layout.display_width) * scale;
        let display_height = f32::from(layout.display_height) * scale;
        let mut display = div()
            .relative()
            .w(px(display_width))
            .h(px(display_height))
            .bg(theme::DEVICE_DISPLAY);
        for surface in layout
            .surfaces
            .iter()
            .filter(|s| matches!(s.id.kind, SurfaceKind::KeyFace | SurfaceKind::BarPanel))
        {
            display = display.child(self.surface(*surface, scale, cx));
        }
        if let Some((x, y)) = self.state.touch_position() {
            display = display.child(
                div()
                    .absolute()
                    .left(px(x * scale - TOUCH_POINT / 2.))
                    .top(px(y * scale - TOUCH_POINT / 2.))
                    .size(px(TOUCH_POINT))
                    .rounded_full()
                    .bg(theme::INPUT_GLOW),
            );
        }
        // Columns, not model-specific knob coordinates, determine the row.
        let columns = layout
            .controls
            .iter()
            .filter(|c| c.id.kind == ControlKind::Knob)
            .map(|c| c.column)
            .max()
            .unwrap_or(1)
            .max(1);
        let knob_row = div()
            .relative()
            .w(px(display_width))
            .h(px(KNOB_ROW_HEIGHT))
            .children(
                layout
                    .controls
                    .iter()
                    .filter(|c| c.id.kind == ControlKind::Knob)
                    .map(|control| {
                        let center =
                            display_width * (f32::from(control.column) - 0.5) / f32::from(columns);
                        let angle = *self.state.knob_steps.get(&control.id).unwrap_or(&0) as f32
                            * std::f32::consts::TAU
                            / 12.;
                        let highlight = self.state.highlights.get(&control.id);
                        let id = control.id;
                        // The mark is inside the ring.
                        let middle = (KNOB_DIAMETER - 2. * KNOB_RING - KNOB_MARK) / 2.;
                        div()
                            .id(format!("control-{id}"))
                            .debug_selector(move || format!("control-{id}"))
                            .cursor_pointer()
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.select_control(id, window, cx)
                            }))
                            .absolute()
                            .left(px(center - KNOB_DIAMETER / 2.))
                            .top(px(KNOB_TOP))
                            .size(px(KNOB_DIAMETER))
                            .rounded_full()
                            .bg(theme::DEVICE_KNOB)
                            .border_2()
                            .border_color(Self::frame_colour(
                                highlight.is_some(),
                                self.framed_control() == Some(id),
                                theme::DEVICE_KNOB_RING,
                            ))
                            .child(
                                div()
                                    .absolute()
                                    .left(px(middle + angle.sin() * KNOB_MARK_RADIUS))
                                    .top(px(middle - angle.cos() * KNOB_MARK_RADIUS))
                                    .size(px(KNOB_MARK))
                                    .rounded_full()
                                    .bg(if highlight.is_some_and(|h| h.gesture == Gesture::Push) {
                                        theme::INPUT_GLOW
                                    } else {
                                        theme::DEVICE_KNOB_DOT
                                    }),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top(px(KNOB_LABEL_TOP))
                                    .w_full()
                                    .text_center()
                                    .text_size(theme::TEXT_CAPTION)
                                    .text_color(theme::TEXT_TERTIARY)
                                    .child(format!("Knob {}", control.id.number)),
                            )
                    }),
            );
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(theme::SPACE_8)
            .p(theme::SPACE_16)
            .bg(theme::DEVICE_BODY)
            .border_1()
            .border_color(theme::DEVICE_BODY_BORDER)
            .rounded(theme::RADIUS_POPOVER)
            .child(display)
            .children(self.touch_bar(layout, scale, display_width, cx))
            .child(knob_row)
            .into_any_element()
    }

    /// A strip below the bar panels for the touch bar control. The touch bar
    /// control has no surface, so the strip is the element that selects it.
    fn touch_bar(
        &self,
        layout: &opencraft_engine::DeviceLayout,
        scale: f32,
        display_width: f32,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let control = layout
            .controls
            .iter()
            .find(|c| c.id.kind == ControlKind::TouchBar)?;
        let panels = layout
            .surfaces
            .iter()
            .filter(|s| s.id.kind == SurfaceKind::BarPanel);
        let left = panels.clone().map(|s| s.x).min().unwrap_or(0);
        let right = panels
            .map(|s| s.x + s.width)
            .max()
            .unwrap_or(layout.display_width);
        let id = control.id;
        let highlight = self.state.highlights.get(&id).map(|h| h.gesture);
        let arrow = |gesture: Gesture, text: &'static str| {
            div()
                .text_color(if highlight == Some(gesture) {
                    theme::INPUT_GLOW
                } else {
                    theme::TEXT_SECONDARY
                })
                .child(text)
        };
        Some(
            div()
                .relative()
                .w(px(display_width))
                .h(px(TOUCH_BAR_HEIGHT))
                .child(
                    div()
                        .id(format!("control-{id}"))
                        .debug_selector(move || format!("control-{id}"))
                        .absolute()
                        .left(px(f32::from(left) * scale))
                        .w(px(f32::from(right.saturating_sub(left)) * scale))
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(theme::SPACE_12)
                        .rounded(theme::RADIUS_CONTROL)
                        .bg(theme::NEUTRAL_TINT)
                        .border_2()
                        .border_color(Self::frame_colour(
                            highlight.is_some(),
                            self.framed_control() == Some(id),
                            theme::DEVICE_TOUCH_BAR_BORDER,
                        ))
                        .text_size(theme::TEXT_CAPTION)
                        .cursor_pointer()
                        .child(arrow(Gesture::SwipeLeft, "◀"))
                        .child(div().text_color(theme::TEXT_SECONDARY).child("Touch bar"))
                        .child(arrow(Gesture::SwipeRight, "▶"))
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.select_control(id, window, cx)
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

    #[test]
    fn the_device_scale_is_between_its_limits() {
        let Event::Layout(layout) = layout() else {
            panic!()
        };
        let mut wide = (*layout).clone();
        wide.display_width = 800;
        wide.display_height = 480;
        // The canvas of the default window: 1440 - 240 - 360.
        let scale = device_scale(&wide, 840., 900. - 52. - 48. - 28.);
        assert!((0.9..=1.0).contains(&scale), "{scale}");
        // The minimum width of the canvas.
        assert_eq!(device_scale(&wide, 480., 572.), 0.5);
        assert_eq!(device_scale(&wide, 4000., 4000.), 1.0);
        assert_eq!(device_scale(&wide, 100., 100.), 0.5);
    }
}
