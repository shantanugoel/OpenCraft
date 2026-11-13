//! Play a sound file, and stop the sounds that OpenCraft plays. The platform
//! calls are behind the System interface.
use crate::backend::System;
use crate::descriptions::{description, setting_problem, PLAY_SOUND, STOP_SOUND};
use crate::handlers::{problem, send};
use opencraft_engine::{
    Action, ChoiceOption, Handler, KindDescription, Message, MessageSender, Request, Value,
};
use std::path::Path;
use std::sync::Arc;

/// The label of the empty value of the field "Output".
pub(crate) const DEFAULT_OUTPUT: &str = "Default output";

pub(crate) struct SoundHandler {
    system: Arc<dyn System>,
    sender: Option<MessageSender>,
}

impl SoundHandler {
    pub(crate) fn new(system: Arc<dyn System>) -> Self {
        Self {
            system,
            sender: None,
        }
    }

    fn play(&self, action: &Action) -> crate::handlers::Result<()> {
        let file = match action.settings.get("file") {
            Some(Value::Text(file)) if !file.trim().is_empty() => file,
            _ => return Err(setting_problem(PLAY_SOUND, "file")),
        };
        let volume = match action.settings.get("volume") {
            None => 100,
            Some(Value::Integer(volume)) => u8::try_from(*volume)
                .ok()
                .filter(|v| *v <= 100)
                .ok_or_else(|| setting_problem(PLAY_SOUND, "volume"))?,
            _ => return Err(setting_problem(PLAY_SOUND, "volume")),
        };
        let device = match action.settings.get("device") {
            None => None,
            Some(Value::Text(device)) if device.is_empty() => None,
            Some(Value::Text(device)) => Some(device.as_str()),
            _ => return Err(setting_problem(PLAY_SOUND, "device")),
        };
        if let Some(device) = device {
            let devices = self
                .system
                .output_devices()
                .map_err(|e| format!("Cannot list the sound outputs: {e}."))?;
            if !devices.iter().any(|d| d.id == device) {
                return Err("The selected sound output is not available.".into());
            }
        }
        self.system
            .play_sound(Path::new(file), volume, device)
            .map_err(|e| format!("Cannot play the sound file: {e}."))
    }
}

impl Handler for SoundHandler {
    fn kinds(&self) -> Vec<KindDescription> {
        vec![description(PLAY_SOUND), description(STOP_SOUND)]
    }
    fn start(&mut self, sender: MessageSender) {
        self.sender = Some(sender);
    }
    fn handle(&mut self, request: Request) {
        match request {
            Request::Choices { id, action, field }
                if action.kind == PLAY_SOUND && field == "device" =>
            {
                let mut options = vec![ChoiceOption {
                    value: String::new(),
                    label: DEFAULT_OUTPUT.into(),
                }];
                match self.system.output_devices() {
                    Ok(devices) => options.extend(devices.into_iter().map(|d| ChoiceOption {
                        value: d.id,
                        label: d.name,
                    })),
                    Err(error) => send(
                        &self.sender,
                        Message::Problem(format!("Cannot list the sound outputs: {error}.")),
                    ),
                }
                send(&self.sender, Message::Choices { id, options });
            }
            Request::Invoke { action, .. } if action.kind == PLAY_SOUND => {
                problem(&self.sender, self.play(&action));
            }
            Request::Invoke { action, .. } if action.kind == STOP_SOUND => problem(
                &self.sender,
                self.system
                    .stop_sounds()
                    .map_err(|e| format!("Cannot stop the sound: {e}.")),
            ),
            _ => {}
        }
    }
}
