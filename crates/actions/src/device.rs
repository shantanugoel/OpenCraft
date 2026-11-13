//! Device actions use engine requests, never the device layer.

#[cfg(test)]
mod tests {
    use super::*;
    use opencraft_engine::{Action, BindingId, ControlId, ControlKind, GestureKind};
    #[test]
    fn device_kinds_send_requests_without_system_calls() {
        let mut handler = DeviceHandler::default();
        let (sender, messages) = std::sync::mpsc::channel();
        handler.start(sender);
        for (kind, state) in [
            (SCREEN_OFF, DisplayState::ScreenOff),
            (DEVICE_SLEEP, DisplayState::DeviceSleep),
        ] {
            let description = description(kind);
            assert_eq!(description.group, "Device");
            assert!(description.fields.is_empty());
            assert!(!description.needs_accessibility);
            handler.handle(Request::Invoke {
                binding: BindingId {
                    page: 0,
                    control: ControlId::new(ControlKind::Key, 1),
                    gesture: GestureKind::Press,
                },
                action: Action {
                    kind: kind.into(),
                    settings: Default::default(),
                },
            });
            assert_eq!(
                messages.try_recv().unwrap(),
                Message::Engine(EngineRequest::DeviceDisplay(state))
            );
        }
        assert!(messages.try_recv().is_err());
    }
}

use crate::descriptions::*;
use opencraft_engine::{
    DisplayState, EngineRequest, Handler, KindDescription, Message, MessageSender, Request,
};

#[derive(Default)]
pub(crate) struct DeviceHandler {
    sender: Option<MessageSender>,
}
impl Handler for DeviceHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        [SCREEN_OFF, DEVICE_SLEEP].map(description).into()
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        if let Request::Invoke { action, .. } = request {
            let state = match action.kind.as_str() {
                SCREEN_OFF => DisplayState::ScreenOff,
                DEVICE_SLEEP => DisplayState::DeviceSleep,
                _ => return,
            };
            if let Some(sender) = &self.sender {
                let _ = sender.send(Message::Engine(EngineRequest::DeviceDisplay(state)));
            }
        }
    }
}
