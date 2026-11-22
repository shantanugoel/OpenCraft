//! The panel that shows the control that the keyboard focus reaches
//! (`docs/DESIGN.md`, section 5 "Focus"; issue 484).
//!
//! GPUI moves the focus with Tab and Shift-Tab, but a panel that scrolls
//! does not follow the focus: the focus ring stays below the visible part,
//! thus a person who uses only the keyboard cannot see it. This module
//! scrolls the panel to the row that the focus moved into.
//!
//! - [`in_panel`] makes a panel for the rows that a part builds inside it.
//! - [`row`] wraps a row of that panel. With no panel, it returns the row
//!   unchanged, thus the other parts of the window pay nothing.
//! - The [`Panel`] keeps the control that it showed, and the scroll of the
//!   panel. A row calls it in its own prepaint, thus the frame of the key
//!   paints the new position (GPUI reads the scroll position in `paint`).
//!
//! The panel scrolls only when the focus moves to another control. A wheel
//! or a scrollbar keeps its position while the focus stays on a control, as
//! in other applications (the fact of GPUI is in `docs/kb/gpui.md`, "The
//! panel follows the keyboard focus").

use super::theme;
use gpui_kit::{
    point, px, AnyElement, App, Bounds, Element, ElementId, FocusHandle, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, ScrollHandle, SharedString, Window,
};
use std::{
    cell::{Cell, RefCell},
    panic::Location,
    rc::Rc,
};

thread_local! {
    /// The panel that a part builds in this frame. [`row`] reads it, and it
    /// goes back to its earlier value when the part has built its rows.
    static CURRENT: RefCell<Option<Rc<Panel>>> = const { RefCell::new(None) };
}

/// The panel that scrolls to the row that the keyboard focus enters.
pub(crate) struct Panel {
    scroll: ScrollHandle,
    /// The control that the panel showed last: it scrolls only when the
    /// focus moves to another control.
    shown: RefCell<Option<FocusHandle>>,
    /// True when a row of the panel had the focus in the last frame.
    inside: Cell<bool>,
}

impl Panel {
    /// Make the panel of `scroll`.
    pub(crate) fn new(scroll: ScrollHandle) -> Rc<Self> {
        Rc::new(Self {
            scroll,
            shown: RefCell::new(None),
            inside: Cell::new(false),
        })
    }

    /// Forget the control that the panel showed, when the last frame had no
    /// row with the focus: the focus left the panel, thus it shows the row
    /// again when the focus comes back. The part calls it at the start of
    /// its draw.
    pub(crate) fn begin_frame(&self) {
        if !self.inside.replace(false) {
            *self.shown.borrow_mut() = None;
        }
    }

    /// Scroll the panel so that `bounds`, the bounds of the row that the
    /// focus is in, shows. [`Row::prepaint`] calls it, thus the frame with
    /// the key paints the new position ([`ScrollHandle::set_offset`]).
    ///
    /// A nested row asks after the row inside it: the first request of the
    /// frame is the deepest row, and it names the control with the focus.
    /// The panel does nothing while the same control keeps the focus, thus
    /// the wheel and the scrollbar keep the position that the user chose.
    fn reveal(&self, focused: &FocusHandle, bounds: Bounds<Pixels>) {
        self.inside.set(true);
        if self.shown.borrow().as_ref() == Some(focused) {
            return;
        }
        *self.shown.borrow_mut() = Some(focused.clone());
        let view = self.scroll.bounds();
        let max = self.scroll.max_offset().y;
        if view.size.height <= px(0.) || max <= px(0.) {
            return;
        }
        let margin = theme::SPACE_8;
        let mut y = self.scroll.offset().y;
        if bounds.top() < view.top() + margin {
            y += view.top() + margin - bounds.top();
        } else if bounds.bottom() > view.bottom() - margin {
            y -= bounds.bottom() - (view.bottom() - margin);
        } else {
            return;
        }
        self.scroll.set_offset(point(px(0.), y.clamp(-max, px(0.))));
    }
}

/// Build the rows of `panel`: each [`row`] inside `build` shows itself when
/// the keyboard focus reaches it.
pub(crate) fn in_panel<R>(panel: &Rc<Panel>, build: impl FnOnce() -> R) -> R {
    let outer = CURRENT.with(|current| current.borrow_mut().replace(panel.clone()));
    let result = build();
    CURRENT.with(|current| *current.borrow_mut() = outer);
    result
}

/// Wrap the control, or the group of controls, of one row so that the panel
/// of [`in_panel`] scrolls to it when the keyboard focus reaches it and
/// shows the focus ring. `key` names the row: it is the same at each frame,
/// because GPUI keeps the state of the row by that name. With no panel, the
/// row is returned unchanged and no element is added.
pub(crate) fn row(key: impl Into<SharedString>, child: impl IntoElement) -> AnyElement {
    let Some(panel) = CURRENT.with(|current| current.borrow().clone()) else {
        return child.into_any_element();
    };
    Row {
        key: key.into(),
        panel,
        child: Some(child.into_any_element()),
    }
    .into_any_element()
}

/// One row of a [`Panel`]: it makes the panel scroll to its bounds when the
/// keyboard focus is inside it.
struct Row {
    key: SharedString,
    panel: Rc<Panel>,
    child: Option<AnyElement>,
}

impl Row {
    /// The child. GPUI calls the methods of an element one time at each
    /// frame, in the order of the frame, thus the child is there.
    fn child(&mut self) -> &mut AnyElement {
        self.child
            .as_mut()
            .expect("GPUI prepares an element after its layout request")
    }
}

impl IntoElement for Row {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Row {
    /// The handle that makes the row an ancestor of the control that the
    /// focus is in: `FocusHandle::contains_focused` reads it.
    type RequestLayoutState = FocusHandle;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name(self.key.clone()))
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, FocusHandle) {
        let id = id.expect("a row has an element identifier");
        let handle = window.with_element_state(id, |state: Option<FocusHandle>, _: &mut Window| {
            let handle = state.unwrap_or_else(|| cx.focus_handle());
            (handle.clone(), handle)
        });
        (self.child().request_layout(window, cx), handle)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        handle: &mut FocusHandle,
        window: &mut Window,
        cx: &mut App,
    ) {
        // The row is a focus group, not a tab stop: the control inside it
        // keeps the tab order of the window. GPUI reads the handle of the
        // group when it makes the focus path, thus the row knows if the
        // focus is inside it.
        window.set_focus_handle(handle, cx);
        self.child().prepaint(window, cx);
        let Some(focused) = window.focused(cx) else {
            return;
        };
        if handle.contains(&focused, window) {
            self.panel.reveal(&focused, bounds);
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut FocusHandle,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child().paint(window, cx);
    }
}
