//! Device actions use engine requests, never the device layer.

#[cfg(test)]
mod tests {
    use super::*;
    use opencraft_engine::{Action, BindingId, ControlId, ControlKind, GestureKind};
    #[test]
    fn brightness_checks_defaults_directions_set_and_bad_settings() {
        let action = |settings| Action {
            kind: DEVICE_BRIGHTNESS.into(),
            settings,
        };
        assert_eq!(
            brightness_request(&action(Default::default())),
            Ok(BrightnessChange::Step(5))
        );
        for (operation, expected) in [
            ("up", BrightnessChange::Step(7)),
            ("down", BrightnessChange::Step(-7)),
            ("set", BrightnessChange::Set(35)),
        ] {
            let mut settings = opencraft_engine::Settings::new();
            settings.insert("operation".into(), Value::Text(operation.into()));
            settings.insert("step".into(), Value::Integer(7));
            settings.insert("value".into(), Value::Integer(35));
            assert_eq!(brightness_request(&action(settings)), Ok(expected));
        }
        for (key, value) in [
            ("operation", Value::Text("bad".into())),
            ("operation", Value::Integer(1)),
            ("step", Value::Integer(0)),
            ("step", Value::Integer(101)),
        ] {
            let settings = opencraft_engine::Settings::from([(key.into(), value)]);
            assert!(brightness_request(&action(settings)).is_err());
        }
        let description = description(DEVICE_BRIGHTNESS);
        let direction = description.direction.unwrap();
        assert_eq!(
            (direction.right.as_str(), direction.left.as_str()),
            ("up", "down")
        );
    }

    #[test]
    fn screen_off_sends_a_request_without_a_system_call() {
        let mut handler = DeviceHandler::default();
        let (sender, messages) = std::sync::mpsc::channel();
        handler.start(sender);
        let description = description(SCREEN_OFF);
        assert_eq!(description.group, "Device");
        assert!(description.fields.is_empty());
        assert!(!description.needs_accessibility);
        assert!(handler
            .kinds()
            .iter()
            .all(|kind| kind.kind != "opencraft.device-sleep"));
        handler.handle(Request::Invoke {
            binding: BindingId {
                page: 0,
                control: ControlId::new(ControlKind::Key, 1),
                gesture: GestureKind::Press,
                folder: None,
            },
            action: Action {
                kind: SCREEN_OFF.into(),
                settings: Default::default(),
            },
        });
        assert_eq!(
            messages.try_recv().unwrap(),
            Message::Engine(EngineRequest::DeviceDisplay(DisplayState::ScreenOff))
        );
        assert!(messages.try_recv().is_err());
    }
}

use crate::descriptions::*;
use opencraft_engine::{
    Action, BrightnessChange, DisplayState, EngineRequest, Handler, KindDescription, Message,
    MessageSender, Request, Value,
};

fn brightness_request(action: &Action) -> Result<BrightnessChange, String> {
    let integer = |key: &str, default: i64, min: i64| match action.settings.get(key) {
        None => Ok(default),
        Some(Value::Integer(n)) if (min..=100).contains(n) => Ok(*n),
        _ => Err(setting_problem(DEVICE_BRIGHTNESS, key)),
    };
    let operation = match action.settings.get("operation") {
        None => "up",
        Some(Value::Text(value)) => value.as_str(),
        _ => return Err(setting_problem(DEVICE_BRIGHTNESS, "operation")),
    };
    match operation {
        "set" => Ok(BrightnessChange::Set(integer("value", 80, 0)? as u8)),
        "up" | "down" => {
            let step = integer("step", 5, 1)? as i16;
            Ok(BrightnessChange::Step(if operation == "down" {
                -step
            } else {
                step
            }))
        }
        _ => Err(setting_problem(DEVICE_BRIGHTNESS, "operation")),
    }
}

#[derive(Default)]
pub(crate) struct DeviceHandler {
    sender: Option<MessageSender>,
}
impl Handler for DeviceHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        [SCREEN_OFF, DEVICE_BRIGHTNESS].map(description).into()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        if let Request::Invoke { action, .. } = request {
            let request = match action.kind.as_str() {
                SCREEN_OFF => Ok(EngineRequest::DeviceDisplay(DisplayState::ScreenOff)),
                DEVICE_BRIGHTNESS => {
                    brightness_request(&action).map(EngineRequest::DeviceBrightness)
                }
                _ => return,
            };
            match request {
                Ok(request) => crate::handlers::send(&self.sender, Message::Engine(request)),
                Err(text) => crate::handlers::send(&self.sender, Message::Problem(text)),
            }
        }
    }
}
