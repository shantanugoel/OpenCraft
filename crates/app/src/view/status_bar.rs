//! The status bar and the messages below the canvas (decision 0029,
//! section 6).
//!
//! The status bar shows the last input from the device at the leading side,
//! and the save state at the trailing side. The messages of the engine show
//! directly above it, one row for each message.

use super::*;
use crate::state::LastInput;
use gpui_kit::{assets::IconName, component::Icon};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// For this time after an input, the status bar shows "now". Then it shows
/// the local time of the input.
pub(super) const NOW_TIME: Duration = Duration::from_secs(10);

/// The text of the save state. The engine writes the profile (rule 9) and
/// sends no event when the file is written: the text means that the engine
/// has each change.
pub(super) const SAVED: &str = "All changes saved";

/// The text before the first input.
pub(super) const NO_INPUT: &str = "No input yet";

/// A message shows in place of the last input for this time (section 6.1).
pub(super) const MESSAGE_TIME: Duration = Duration::from_secs(4);

/// The hour and the minute of `time` in the local time zone.
#[cfg(unix)]
fn hours_minutes(time: SystemTime) -> (u32, u32) {
    let seconds = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let Ok(clock) = libc::time_t::try_from(seconds) else {
        return utc_hours_minutes(seconds);
    };
    // SAFETY: an all-zero `tm` is a valid value, and `localtime_r` writes
    // only to the `tm` that it gets.
    let mut local: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&clock, &mut local) }.is_null() {
        return utc_hours_minutes(seconds);
    }
    (local.tm_hour as u32, local.tm_min as u32)
}

#[cfg(not(unix))]
fn hours_minutes(time: SystemTime) -> (u32, u32) {
    utc_hours_minutes(time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()))
}

fn utc_hours_minutes(seconds: u64) -> (u32, u32) {
    let minutes = seconds / 60;
    (((minutes / 60) % 24) as u32, (minutes % 60) as u32)
}

/// The text of the last input: "*control* · *gesture* · *time*", for
/// example "Knob 1 · Turn right × 3 · now".
pub(super) fn input_text(input: &LastInput, now: std::time::Instant) -> String {
    let mut gesture = input.gesture.to_string();
    if let Some(first) = gesture.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    if input.count > 1 {
        gesture = format!("{gesture} × {}", input.count);
    }
    let time = if now.saturating_duration_since(input.at) < NOW_TIME {
        "now".to_string()
    } else {
        let (hours, minutes) = hours_minutes(input.time);
        format!("{hours:02}:{minutes:02}")
    };
    format!("{} · {gesture} · {time}", control_name(input.control))
}

impl DeviceView {
    /// Draw the window again when the text of the last input changes from
    /// "now" to the time. One timer for each input: a new input stops the
    /// timer of the input before it.
    pub(super) fn start_input_timer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input_timer = Some(cx.spawn_in(window, async move |view, cx| {
            cx.background_executor().timer(NOW_TIME).await;
            let _ = view.update(cx, |view, cx| {
                view.input_timer = None;
                cx.notify();
            });
        }));
    }

    /// Show `text` in place of the last input for [`MESSAGE_TIME`], with no
    /// dot, for example after a click in the library. A new message replaces
    /// the message before it, and starts the time again.
    pub(super) fn show_message(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.status_message = Some(text.into());
        self.message_timer = Some(cx.spawn_in(window, async move |view, cx| {
            cx.background_executor().timer(MESSAGE_TIME).await;
            let _ = view.update(cx, |view, cx| {
                view.status_message = None;
                view.message_timer = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn status_bar(&self) -> AnyElement {
        let message = self.status_message.clone();
        let input = self.state.last_input.as_ref().filter(|_| message.is_none());
        div()
            .id("status-bar")
            .debug_selector(|| "status-bar".into())
            .flex_shrink_0()
            .h(theme::STATUS_BAR_HEIGHT)
            .px(theme::SPACE_16)
            .flex()
            .items_center()
            .gap(theme::SPACE_8)
            .bg(theme::BG_BAR)
            .border_t_1()
            .border_color(theme::LINE_SECTION)
            .text_size(theme::TEXT_HELP)
            .text_color(theme::TEXT_SECONDARY)
            .children(input.map(|_| controls::dot(theme::ACCENT_TEXT)))
            .child(
                div()
                    .id("last-input")
                    .debug_selector(|| "last-input".into())
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(match (message, input) {
                        (Some(message), _) => message.to_string(),
                        (None, Some(input)) => input_text(input, std::time::Instant::now()),
                        (None, None) => NO_INPUT.to_string(),
                    }),
            )
            .child(
                div()
                    .id("save-state")
                    .debug_selector(|| "save-state".into())
                    .flex_shrink_0()
                    .child(SAVED),
            )
            .into_any_element()
    }

    /// One row for each message of the engine, with a "Close" button.
    pub(super) fn messages(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.state.problems.is_empty() {
            return None;
        }
        Some(
            div()
                .id("problems")
                .debug_selector(|| "problems".into())
                .flex_shrink_0()
                .max_h(px(MESSAGES_HEIGHT))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .children(
                    self.state
                        .problems
                        .iter()
                        .enumerate()
                        .map(|(index, problem)| {
                            div()
                                .flex()
                                .items_center()
                                .gap(theme::SPACE_10)
                                .py(theme::SPACE_8)
                                .px(theme::SPACE_16)
                                .bg(theme::DESTRUCTIVE_TINT)
                                .border_t_1()
                                .border_color(theme::DESTRUCTIVE_LINE)
                                .text_size(theme::TEXT_LABEL)
                                .child(
                                    Icon::new(IconName::TriangleAlert)
                                        .size(theme::ICON_MESSAGE)
                                        .text_color(theme::WARNING),
                                )
                                .child(div().flex_1().min_w_0().child(problem.clone()))
                                .child(
                                    small_button(format!("dismiss-{index}"), "Close", true)
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            view.dismiss_problem(index, cx)
                                        })),
                                )
                        }),
                )
                .into_any_element(),
        )
    }
}

/// The highest height of the messages. More messages scroll.
pub(super) const MESSAGES_HEIGHT: f32 = 120.;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn the_last_input_has_the_control_the_gesture_the_count_and_the_time() {
        let now = Instant::now();
        let input = LastInput {
            control: ControlId::new(ControlKind::Knob, 1),
            gesture: "turn right",
            count: 3,
            at: now,
            time: SystemTime::now(),
        };
        assert_eq!(input_text(&input, now), "Knob 1 · Turn right × 3 · now");
        let key = LastInput {
            control: ControlId::new(ControlKind::Key, 1),
            gesture: "press",
            count: 1,
            ..input
        };
        assert_eq!(
            input_text(&key, now + Duration::from_secs(9)),
            "Key 1 · Press · now"
        );
        let (hours, minutes) = hours_minutes(key.time);
        assert_eq!(
            input_text(&key, now + NOW_TIME),
            format!("Key 1 · Press · {hours:02}:{minutes:02}")
        );
        assert_eq!(utc_hours_minutes(86_400 + 13 * 3600 + 7 * 60 + 59), (13, 7));
    }
}
