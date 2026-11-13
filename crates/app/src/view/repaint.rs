//! A guard against a window that does not draw a new frame (issue #321).
//!
//! GPUI on macOS draws a frame only when the display link of the window asks
//! for one. AppKit starts and stops that link when the occlusion state of
//! the window changes. If the link does not start again, for example after
//! a screen lock, the window keeps its old picture while its state changes.
//! The clicks then act on the hit areas of the old frame. See
//! `docs/kb/gpui.md`, "Frames after sleep, lock, and occlusion".
//!
//! The guard has two parts:
//!
//! - After each engine event, the view expects a frame within
//!   [`FRAME_DEADLINE`]. If no frame came and the window is on the screen,
//!   the view forces one.
//! - After a wake, an unlock, or a display wake, and when the window becomes
//!   visible or active, the window forces a frame.
//! - A click or a key in a window that GPUI takes as hidden forces a frame:
//!   the user sees the window.
//!
//! A forced frame asks AppKit to display the layer of the window. GPUI then
//! draws at once and starts its display link again (`displayLayer:` in
//! `gpui_macos`). On other systems, a forced frame is a refresh.

use super::*;
use std::time::Duration;

/// The time after an engine event in which the window must draw a frame. A
/// window that is not the key window draws at 30 frames per second at the
/// minimum rate of GPUI, thus this time is 15 frames.
pub(super) const FRAME_DEADLINE: Duration = Duration::from_millis(500);

/// The frame count of the view, and the check that waits for a frame.
#[derive(Default)]
pub(super) struct Repaint {
    /// The number of frames that the view rendered.
    frames: u64,
    /// The check after an engine event, while it waits.
    check: Option<Task<()>>,
    /// True after a forced frame, until the window draws a frame with no
    /// help. The log gets one line for each stall, not one for each frame.
    stalled: bool,
}

impl Repaint {
    /// Count a frame. Call this from `render`.
    pub(super) fn frame(&mut self) {
        self.frames += 1;
    }

    /// The result of the check after [`FRAME_DEADLINE`]: true if the window
    /// must get a forced frame. `frames` is the count at the start of the
    /// check, and `on_screen` tells if the window is on the screen now.
    pub(super) fn needs_frame(&mut self, frames: u64, on_screen: bool) -> bool {
        self.check = None;
        if self.frames != frames {
            self.stalled = false;
            return false;
        }
        on_screen
    }
}

impl DeviceView {
    /// Expect a frame after an engine event. If none comes within
    /// [`FRAME_DEADLINE`], force one.
    pub(super) fn expect_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.repaint.check.is_some() {
            return;
        }
        let frames = self.repaint.frames;
        self.repaint.check = Some(cx.spawn_in(window, async move |view, cx| {
            cx.background_executor().timer(FRAME_DEADLINE).await;
            let _ = view.update_in(cx, |view, window, _| {
                if view.repaint.needs_frame(frames, on_screen(window)) {
                    if !std::mem::replace(&mut view.repaint.stalled, true) {
                        eprintln!(
                            "The window drew no frame {} ms after a change. OpenCraft \
                             makes it draw.",
                            FRAME_DEADLINE.as_millis()
                        );
                    }
                    force_frame(window);
                }
            });
        }));
    }
}

/// Force a frame at each mouse button and key event in a window that GPUI
/// takes as hidden. Put this on the root element of the window.
pub(super) fn draw_on_input(root: Div) -> Div {
    root.capture_any_mouse_down(|_, window, _| draw_if_hidden(window))
        .capture_any_mouse_up(|_, window, _| draw_if_hidden(window))
        .capture_key_down(|_, window, _| draw_if_hidden(window))
}

fn draw_if_hidden(window: &mut Window) {
    if !window.is_visible() {
        force_frame(window);
    }
}

/// Make the window draw a frame now, and start its frame source again. The
/// state of the views does not change.
pub(crate) fn force_frame(window: &mut Window) {
    window.refresh();
    #[cfg(target_os = "macos")]
    macos::display_layer(window);
}

/// True if the window is on the screen: shown, not in the Dock, and on the
/// active Space. Occlusion does not count, because issue #321 can come from
/// an occlusion state that is not correct.
fn on_screen(window: &Window) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::on_screen(window)
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.is_visible()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use gpui_kit::Window;
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    /// The content view of GPUI for `window`.
    fn view(window: &Window) -> Option<&NSView> {
        let handle = HasWindowHandle::window_handle(window).ok()?;
        match handle.as_raw() {
            // SAFETY: GPUI gives the pointer of its NSView, which lives as
            // long as the window. The caller is on the main thread (a GPUI
            // window update).
            RawWindowHandle::AppKit(handle) => {
                Some(unsafe { handle.ns_view.cast::<NSView>().as_ref() })
            }
            _ => None,
        }
    }

    /// Ask AppKit to display the layer of the view. GPUI draws in
    /// `displayLayer:`, and then starts its display link again if the window
    /// is visible. GPUI uses the same call in `MacWindow::activate`.
    pub(super) fn display_layer(window: &Window) {
        if let Some(view) = view(window) {
            view.setNeedsDisplay(true);
        }
    }

    pub(super) fn on_screen(window: &Window) -> bool {
        let Some(ns_window) = view(window).and_then(|view| view.window()) else {
            return false;
        };
        ns_window.isVisible() && !ns_window.isMiniaturized() && ns_window.isOnActiveSpace()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_after_the_change_needs_no_help() {
        let mut repaint = Repaint::default();
        repaint.frame();
        let frames = repaint.frames;
        repaint.frame();
        assert!(!repaint.needs_frame(frames, true));
    }

    #[test]
    fn no_frame_on_the_screen_needs_a_forced_frame() {
        let mut repaint = Repaint::default();
        repaint.frame();
        assert!(repaint.needs_frame(repaint.frames, true));
        // A window that is not on the screen draws no frame, correctly.
        assert!(!repaint.needs_frame(repaint.frames, false));
    }

    #[test]
    fn a_frame_with_no_help_ends_the_stall() {
        let mut repaint = Repaint {
            stalled: true,
            ..Default::default()
        };
        let frames = repaint.frames;
        repaint.frame();
        assert!(!repaint.needs_frame(frames, true));
        assert!(!repaint.stalled);
    }
}
