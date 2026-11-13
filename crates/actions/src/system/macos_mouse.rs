//! Quartz mouse events. No event monitor or Automation permission is used.

use super::{Error, Result};
use crate::mouse::{Button, MouseEvent};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton, EventField,
    ScrollEventUnit,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::time::Duration;

fn source() -> Result<CGEventSource> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|()| Error::System(0))
}

fn mouse(kind: CGEventType, point: CGPoint, button: CGMouseButton) -> Result<CGEvent> {
    let event =
        CGEvent::new_mouse_event(source()?, kind, point, button).map_err(|()| Error::System(0))?;
    event.set_flags(CGEventFlags::CGEventFlagNull);
    Ok(event)
}

fn buttons(button: Button) -> (CGMouseButton, CGEventType, CGEventType, CGEventType) {
    match button {
        Button::Left => (
            CGMouseButton::Left,
            CGEventType::LeftMouseDown,
            CGEventType::LeftMouseUp,
            CGEventType::LeftMouseDragged,
        ),
        Button::Right => (
            CGMouseButton::Right,
            CGEventType::RightMouseDown,
            CGEventType::RightMouseUp,
            CGEventType::RightMouseDragged,
        ),
        Button::Middle => (
            CGMouseButton::Center,
            CGEventType::OtherMouseDown,
            CGEventType::OtherMouseUp,
            CGEventType::OtherMouseDragged,
        ),
    }
}

/// Prepare the full operation before sending its first event. In particular,
/// an allocation failure must not leave a mouse button down.
fn events(work: MouseEvent, current: CGPoint) -> Result<Vec<(CGEvent, Duration)>> {
    let mut events = Vec::new();
    let zero = Duration::ZERO;
    match work {
        MouseEvent::Click { button, count } => {
            if !(1..=2).contains(&count) {
                return Err(Error::NotSupported);
            }
            let (button, down, up, _) = buttons(button);
            for click in 1..=count {
                for kind in [down, up] {
                    let event = mouse(kind, current, button)?;
                    event.set_integer_value_field(
                        EventField::MOUSE_EVENT_CLICK_STATE,
                        i64::from(click),
                    );
                    events.push((event, zero));
                }
            }
        }
        MouseEvent::Move { x, y, relative } => {
            let mut target = CGPoint::new(f64::from(x), f64::from(y));
            if relative {
                target.x += current.x;
                target.y += current.y;
            }
            events.push((
                mouse(CGEventType::MouseMoved, target, CGMouseButton::Left)?,
                zero,
            ));
        }
        MouseEvent::Scroll { x, y } => {
            let event = CGEvent::new_scroll_event(source()?, ScrollEventUnit::LINE, 2, y, x, 0)
                .map_err(|()| Error::System(0))?;
            event.set_flags(CGEventFlags::CGEventFlagNull);
            events.push((event, zero));
        }
        MouseEvent::Drag { button, x, y } => {
            let (button, down, up, dragged) = buttons(button);
            events.push((mouse(down, current, button)?, zero));
            // A short path lets applications observe a drag, not a jump from
            // a down to an up at a different position.
            const STEPS: u32 = 12;
            for step in 1..=STEPS {
                let fraction = f64::from(step) / f64::from(STEPS);
                let point = CGPoint::new(
                    current.x + (f64::from(x) - current.x) * fraction,
                    current.y + (f64::from(y) - current.y) * fraction,
                );
                events.push((mouse(dragged, point, button)?, Duration::from_millis(8)));
            }
            events.push((
                mouse(up, CGPoint::new(f64::from(x), f64::from(y)), button)?,
                zero,
            ));
        }
    }
    Ok(events)
}

pub(super) fn send(work: MouseEvent) -> Result<()> {
    let current = CGEvent::new(source()?)
        .map_err(|()| Error::System(0))?
        .location();
    for (event, delay) in events(work, current)? {
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }
        event.post(CGEventTapLocation::HID);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_moves_scrolls_and_drags_build_balanced_events_without_posting() {
        let here = CGPoint::new(20., 30.);
        for button in [Button::Left, Button::Right, Button::Middle] {
            let (_, down, up, _) = buttons(button);
            let clicks = events(MouseEvent::Click { button, count: 2 }, here).unwrap();
            assert_eq!(clicks.len(), 4);
            assert_eq!(clicks[0].0.get_type() as u32, down as u32);
            assert_eq!(clicks[3].0.get_type() as u32, up as u32);
            assert_eq!(
                clicks[2]
                    .0
                    .get_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE),
                2
            );
            let drag = events(
                MouseEvent::Drag {
                    button,
                    x: 90,
                    y: 110,
                },
                here,
            )
            .unwrap();
            assert_eq!(drag.len(), 14);
            assert_eq!(drag[0].0.get_type() as u32, down as u32);
            assert_eq!(drag[13].0.get_type() as u32, up as u32);
            assert_eq!(
                (drag[13].0.location().x, drag[13].0.location().y),
                (90., 110.)
            );
        }
        let moved = events(
            MouseEvent::Move {
                x: -5,
                y: 10,
                relative: true,
            },
            here,
        )
        .unwrap();
        assert_eq!(
            (moved[0].0.location().x, moved[0].0.location().y),
            (15., 40.)
        );
        let scroll = events(MouseEvent::Scroll { x: 2, y: -3 }, here).unwrap();
        assert_eq!(
            scroll[0]
                .0
                .get_integer_value_field(EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_1),
            -3
        );
        assert_eq!(
            scroll[0]
                .0
                .get_integer_value_field(EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_2),
            2
        );
    }
}
