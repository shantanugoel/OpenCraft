//! Mouse settings are checked before the system receives an event.

use crate::descriptions::{setting_problem, MOUSE};
use crate::handlers::Result;
use opencraft_engine::{Action, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// One complete mouse operation. No operation leaves a button held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEvent {
    Click { button: Button, count: u8 },
    Move { x: i32, y: i32, relative: bool },
    Scroll { x: i32, y: i32 },
    Drag { button: Button, x: i32, y: i32 },
}

fn text<'a>(action: &'a Action, key: &str, default: &'a str) -> Result<&'a str> {
    match action.settings.get(key) {
        None => Ok(default),
        Some(Value::Text(value)) => Ok(value),
        _ => Err(setting_problem(MOUSE, key)),
    }
}

fn number(action: &Action, key: &str, default: i32, min: i32, max: i32) -> Result<i32> {
    match action.settings.get(key) {
        None => Ok(default),
        Some(Value::Integer(value)) if (i64::from(min)..=i64::from(max)).contains(value) => {
            Ok(*value as i32)
        }
        _ => Err(setting_problem(MOUSE, key)),
    }
}

pub(crate) fn work(action: &Action) -> Result<MouseEvent> {
    let button = || match text(action, "button", "left")? {
        "left" => Ok(Button::Left),
        "right" => Ok(Button::Right),
        "middle" => Ok(Button::Middle),
        _ => Err(setting_problem(MOUSE, "button")),
    };
    let point = || -> Result<(i32, i32)> {
        Ok((
            number(action, "x", 0, -100_000, 100_000)?,
            number(action, "y", 0, -100_000, 100_000)?,
        ))
    };
    Ok(match text(action, "operation", "click")? {
        operation @ ("click" | "double-click") => MouseEvent::Click {
            button: button()?,
            count: if operation == "click" { 1 } else { 2 },
        },
        operation @ ("move" | "move-relative") => {
            let (x, y) = point()?;
            MouseEvent::Move {
                x,
                y,
                relative: operation == "move-relative",
            }
        }
        "drag" => {
            let (x, y) = point()?;
            MouseEvent::Drag {
                button: button()?,
                x,
                y,
            }
        }
        operation @ ("scroll-up" | "scroll-down" | "scroll-left" | "scroll-right") => {
            let step = number(action, "step", 3, 1, 1000)?;
            let (x, y) = match operation {
                "scroll-up" => (0, step),
                "scroll-down" => (0, -step),
                "scroll-left" => (step, 0),
                _ => (-step, 0),
            };
            MouseEvent::Scroll { x, y }
        }
        _ => return Err(setting_problem(MOUSE, "operation")),
    })
}
