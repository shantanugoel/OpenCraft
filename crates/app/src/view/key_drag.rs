//! A drag of a key face onto another key face on the canvas (KEY-14).
//!
//! The drag moves the appearance and all the bindings of the key to an
//! empty key, or swaps the two keys when the destination has content. The
//! engine does the work in one step of the history, "Move Key"
//! ([`opencraft_engine::Command::MoveControl`]). The window sends one
//! command, thus the engine cancels the gesture timers of both keys and
//! the device, the shell, and the handlers get each change.
//!
//! A key with nothing on it is not a source: it has nothing to move. A drop
//! of a key onto itself sends nothing.

use super::*;
use gpui_kit::{CursorStyle, FontWeight, Pixels, Point};

/// The value of a drag of a key face.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct KeyDrag {
    /// The control that moves. The drop knows the destination.
    pub(super) control: ControlId,
}

/// The picture at the pointer during a drag of a key (section 12): a small
/// copy of the key face.
pub(super) struct KeyDragPicture {
    image: Option<Arc<RenderImage>>,
    title: SharedString,
    /// The point of the press in the key face. GPUI puts the top-left corner
    /// of the picture at the pointer minus this offset (kb/gpui.md).
    offset: Point<Pixels>,
}

impl Render for KeyDragPicture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (x, y) = theme::DRAG_PICTURE_OFFSET;
        // GPUI puts the corner of this element at the pointer minus the
        // offset. The margin moves the chip to its place from the pointer.
        div().child(
            div()
                .flex()
                .items_center()
                .gap(theme::SPACE_8)
                .pl(theme::SPACE_8)
                .pr(theme::SPACE_12)
                .py(theme::DRAG_PICTURE_PADDING_Y)
                .rounded(theme::RADIUS_CARD)
                .bg(theme::bg_field())
                .border_1()
                .border_color(theme::bg_segment_on())
                .shadow(theme::drag_shadow())
                .text_size(theme::TEXT_CONTROL)
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme::text_primary())
                // GPUI gives the drag picture its smallest size.
                .whitespace_nowrap()
                .children(self.image.clone().map(|image| {
                    img(image)
                        .w(theme::SIDE_KEY_DRAG_IMAGE)
                        .h(theme::SIDE_KEY_DRAG_IMAGE)
                        .rounded(theme::RADIUS_BADGE)
                }))
                .child(self.title.clone())
                .ml(self.offset.x + x)
                .mt(self.offset.y + y),
        )
    }
}

impl DeviceView {
    /// True while a key of the canvas is in a drag. The drop targets of the
    /// key faces show then, and the bar panels and the knobs show none.
    pub(crate) fn key_drag(&self, cx: &App) -> bool {
        self.dragging_key.is_some() && cx.has_active_drag()
    }

    /// The drag of the key face `surface` with the control `control`.
    pub(super) fn key_drag_source(
        &self,
        surface: SurfaceId,
        control: ControlId,
        cx: &mut Context<Self>,
    ) -> impl Fn(&KeyDrag, Point<Pixels>, &mut Window, &mut App) -> Entity<KeyDragPicture> + 'static
    {
        let image = self.images.get(&surface).cloned();
        let title: SharedString = self
            .state
            .surfaces
            .get(&surface)
            .and_then(|state| state.content.label.clone())
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| control_name(control))
            .into();
        let view = cx.entity().downgrade();
        move |drag: &KeyDrag, offset, window, cx| {
            let _ = view.update(cx, |view, cx| {
                view.dragging_key = Some(drag.control);
                cx.notify();
            });
            // The closed hand during the drag.
            window.defer(cx, |window, cx| {
                cx.set_active_drag_cursor_style(CursorStyle::ClosedHand, window);
            });
            let picture = cx.new(|_| KeyDragPicture {
                image: image.clone(),
                title: title.clone(),
                offset,
            });
            // The drag picture goes away at the end of each drag, also with
            // no drop.
            let view = view.clone();
            cx.observe_release(&picture, move |_, cx| {
                let _ = view.update(cx, |view, cx| {
                    view.dragging_key = None;
                    cx.notify();
                });
            })
            .detach();
            picture
        }
    }

    /// A drop of the key `from` on the key `to`: move it to an empty key,
    /// or swap the two keys. The engine does the change in one step of the
    /// history. This sends nothing for the same key.
    pub(super) fn drop_key(
        &mut self,
        from: ControlId,
        to: ControlId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if from == to || from.kind != to.kind {
            return;
        }
        let swap = self.key_has_content(to);
        (self.send)(Command::MoveControl { from, to });
        let text = if swap {
            format!(
                "{} and {} are swapped",
                control_name(from),
                control_name(to)
            )
        } else {
            format!("{} moves to {}", control_name(from), control_name(to))
        };
        self.show_message(text, window, cx);
    }

    /// True when the selected control is a key face with an appearance or a
    /// binding: a key that the commands of the palette can move (issue 485).
    pub(super) fn can_move_key(&self) -> bool {
        self.state.selected_control.is_some_and(|control| {
            control.kind == ControlKind::Key && self.key_has_content(control)
        })
    }

    /// Move the selected key face by one place of the grid: the commands of
    /// the palette, the keyboard path of a move (issue 485). `row` is 1 for
    /// the row below and -1 for the row above; `column` is 1 for the key at
    /// the right and -1 for the key at the left.
    ///
    /// It sends the same [`Command::MoveControl`] as a drop of the key, thus
    /// the engine cancels the gesture timers of both keys and one step of
    /// the history restores the key.
    pub(super) fn move_key_by(
        &mut self,
        row: i8,
        column: i8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(from) = self.state.selected_control else {
            return;
        };
        if from.kind != ControlKind::Key {
            self.show_message("Select a key face first", window, cx);
            return;
        }
        if !self.key_has_content(from) {
            self.show_message(
                format!("{} has nothing to move", control_name(from)),
                window,
                cx,
            );
            return;
        }
        let Some(to) = self.key_beside(from, row, column) else {
            self.show_message(
                format!(
                    "{} has no key at its {}",
                    control_name(from),
                    if row < 0 {
                        "top"
                    } else if row > 0 {
                        "bottom"
                    } else if column < 0 {
                        "left"
                    } else {
                        "right"
                    }
                ),
                window,
                cx,
            );
            return;
        };
        self.select_control(to, window, cx);
        self.drop_key(from, to, window, cx);
    }

    /// The key face next to `control` at `row` and `column` places on the
    /// layout of the device. `None` when the device has no key there, or
    /// when the key is on a page or in a folder that does not have that
    /// place: the engine refuses a move to another page.
    fn key_beside(&self, control: ControlId, row: i8, column: i8) -> Option<ControlId> {
        let layout = self.state.layout.as_ref()?;
        let from = layout.controls.iter().find(|c| c.id == control)?;
        let row = u8::try_from(i16::from(from.row) + i16::from(row)).ok()?;
        let column = u8::try_from(i16::from(from.column) + i16::from(column)).ok()?;
        layout
            .controls
            .iter()
            .find(|c| c.id.kind == control.kind && c.row == row && c.column == column)
            .map(|c| c.id)
    }

    /// True if the control `control` has an appearance or a binding on the
    /// active page or folder.
    fn key_has_content(&self, control: ControlId) -> bool {
        let surface = self
            .state
            .layout
            .as_ref()
            .and_then(|layout| layout.controls.iter().find(|c| c.id == control))
            .and_then(|c| c.surface);
        let face = surface
            .and_then(|surface| self.state.surfaces.get(&surface))
            .is_some_and(|surface| !surface.content.is_empty());
        face || self
            .state
            .bindings
            .keys()
            .any(|binding| binding.control == control)
    }
}
