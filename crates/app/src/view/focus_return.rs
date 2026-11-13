//! The focus goes back to the control that opened an overlay (issue 248,
//! `docs/DESIGN.md`, section 5).
//!
//! An overlay is a popover with a text field (a colour popover), the icon
//! picker, the application picker, a sheet, or the command palette. When it opens, the keyboard
//! focus moves into it. When it closes, the focus goes back to the control
//! that had the keyboard focus before, as on macOS. Thus the next Tab starts
//! after that control.
//!
//! The parts:
//!
//! - **The record.** [`track_keys`] notes the control that has the focus at
//!   each key. The control that has the focus when the user pushes Space on
//!   a button is the control that opens the overlay. A text field is not
//!   noted: the focus does not go back into a field.
//! - **The overlay.** [`FocusReturn`] wraps the element of an overlay. When
//!   the overlay is drawn for the first time, it takes the noted control if
//!   the last input was a key. After a click, no control has the focus, thus
//!   nothing is noted and a mouse user sees no focus ring.
//! - **The close.** [`FocusReturn`] ends with the overlay: the overlay is not
//!   drawn any more. `DeviceView::focus_workspace` gives the focus to the
//!   control at once, if the focus is in the overlay ([`opener_of_focus`]).
//!   If the overlay goes away with no call (a change of the page, a change of
//!   the engine), the lost focus gives it back ([`closed_opener`]). A click
//!   that closes the overlay does not give the focus back, as a click that
//!   opens it gives no focus: the focus goes to the root element, and no
//!   focus ring shows.
//!
//! A new popover gets this with `controls::popover_focus`, and a new sheet
//! gets it with [`modal::Modal`](super::modal::Modal): the call site does
//! nothing.

use gpui_kit::{
    component::WindowExt, AnyElement, App, Bounds, Element, ElementId, FocusHandle,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, SharedString, Subscription,
    WeakFocusHandle, Window,
};
use std::{cell::RefCell, panic::Location};

/// An overlay that is drawn now.
struct Open {
    token: u64,
    /// The focus trap of the overlay: the focus is in the overlay when the
    /// trap contains the focus.
    trap: FocusHandle,
    /// The control that had the keyboard focus when the overlay opened.
    opener: Option<WeakFocusHandle>,
}

#[derive(Default)]
struct Record {
    /// The control that had the focus at the last key.
    noted: Option<WeakFocusHandle>,
    next_token: u64,
    /// The overlays that are drawn now, the last one opened is the last in
    /// the list.
    open: Vec<Open>,
    /// The controls of the overlays that were not drawn in the last frame.
    closed: Vec<WeakFocusHandle>,
}

thread_local! {
    static RECORD: RefCell<Record> = RefCell::new(Record::default());
}

fn record<R>(f: impl FnOnce(&mut Record) -> R) -> Option<R> {
    // The thread can end while a window drops.
    RECORD.try_with(|record| f(&mut record.borrow_mut())).ok()
}

/// Note the control with the focus at each key. `root` is the root element
/// of the window: it is not a control. The subscription ends the record.
pub(super) fn track_keys(root: &FocusHandle, window: &Window, cx: &mut App) -> Subscription {
    let root = root.downgrade();
    let handle = window.window_handle();
    cx.intercept_keystrokes(move |_, window, cx| {
        if window.window_handle() != handle {
            return;
        }
        let noted = window
            .focused(cx)
            .filter(|focused| root != *focused && window.focused_input(cx).is_none())
            .map(|focused| focused.downgrade());
        record(|record| record.noted = noted);
    })
}

/// Forget the controls of the overlays that closed. The view calls this when
/// it draws, before the next close.
pub(super) fn begin_frame() {
    record(|record| record.closed.clear());
}

/// The control to give the focus to when the overlay that has the focus
/// closes: the control that opened the innermost overlay with the focus, if
/// it is drawn. `None` if the focus is not in an overlay, or if the overlay
/// opened with the mouse.
pub(super) fn opener_of_focus(window: &Window, cx: &App) -> Option<FocusHandle> {
    // A click that closes the overlay leaves no control with the focus, as
    // a click that opens it does.
    if !window.last_input_was_keyboard() {
        return None;
    }
    record(|record| {
        record
            .open
            .iter()
            .rev()
            .find(|open| open.trap.contains_focused(window, cx))
            .and_then(|open| open.opener.as_ref())
            .and_then(WeakFocusHandle::upgrade)
    })
    .flatten()
}

/// The control of an overlay that closed in the last frame, if it is drawn.
/// The view calls this when the element with the focus is not drawn any
/// more.
pub(super) fn closed_opener(window: &Window) -> Option<FocusHandle> {
    let closed = record(|record| std::mem::take(&mut record.closed))?;
    if !window.last_input_was_keyboard() {
        return None;
    }
    closed.iter().find_map(WeakFocusHandle::upgrade)
}

/// The state of a [`FocusReturn`]: it lives as long as the overlay is drawn.
struct Guard {
    token: u64,
}

impl Guard {
    fn new(trap: &FocusHandle, window: &Window) -> Self {
        // A key opened the overlay if the last input was a key. A click
        // opens it with no focus on the control.
        let keyboard = window.last_input_was_keyboard();
        let token = record(|record| {
            let noted = record.noted.take();
            let opener = noted
                .filter(|_| keyboard)
                .filter(|noted| noted.upgrade().is_some_and(|c| !trap.contains(&c, window)));
            record.next_token += 1;
            let token = record.next_token;
            record.open.push(Open {
                token,
                trap: trap.clone(),
                opener,
            });
            token
        })
        .unwrap_or_default();
        Self { token }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        record(|record| {
            if let Some(index) = record.open.iter().position(|o| o.token == self.token) {
                if let Some(opener) = record.open.remove(index).opener {
                    record.closed.push(opener);
                }
            }
        });
    }
}

/// The element of an overlay with the record of its control. `trap` is the
/// focus handle of the focus trap of the overlay.
pub(super) struct FocusReturn {
    id: ElementId,
    trap: FocusHandle,
    child: AnyElement,
}

impl FocusReturn {
    pub(super) fn new(
        id: impl Into<SharedString>,
        trap: &FocusHandle,
        child: impl IntoElement,
    ) -> Self {
        Self {
            id: ElementId::Name(id.into()),
            trap: trap.clone(),
            child: child.into_any_element(),
        }
    }
}

impl IntoElement for FocusReturn {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for FocusReturn {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
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
    ) -> (LayoutId, ()) {
        let id = id.expect("The overlay has an element identifier");
        let trap = self.trap.clone();
        window.with_element_state(id, |state: Option<Guard>, window| {
            ((), state.unwrap_or_else(|| Guard::new(&trap, window)))
        });
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
