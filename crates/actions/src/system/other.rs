//! The system with no platform code: each function gives
//! [`Error::NotSupported`].

use super::{Error, MediaKey, Result, Volume};
use crate::hotkey::Hotkey;

pub fn can_send_keys() -> bool {
    false
}

pub fn ask_to_send_keys() {}

pub fn send_hotkey(_: &Hotkey) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn mouse_event(_: crate::mouse::MouseEvent) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn mac_function(_: crate::mac_function::MacFunction) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn paste_text(_: &str) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn type_text(_: &str) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn media_key(_: MediaKey) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn volume() -> Result<Volume> {
    Err(Error::NotSupported)
}

pub fn set_volume_level(_: u8) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn set_muted(_: bool) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn microphone_muted() -> Result<Option<bool>> {
    Err(Error::NotSupported)
}
pub fn set_microphone_muted(_: bool) -> Result<()> {
    Err(Error::NotSupported)
}
pub fn watch_microphone(_: std::sync::mpsc::Sender<()>) -> Result<Box<dyn super::AudioWatch>> {
    Err(Error::NotSupported)
}

pub fn output_devices() -> Result<Vec<super::OutputDevice>> {
    Err(Error::NotSupported)
}
pub fn default_output() -> Result<super::OutputDevice> {
    Err(Error::NotSupported)
}
pub fn set_output(_: &str) -> Result<()> {
    Err(Error::NotSupported)
}
pub fn watch_output(_: std::sync::mpsc::Sender<()>) -> Result<Box<dyn super::AudioWatch>> {
    Err(Error::NotSupported)
}
pub fn play_sound(_: &std::path::Path, _: u8, _: Option<&str>) -> Result<()> {
    Err(Error::NotSupported)
}
pub fn stop_sounds() -> Result<()> {
    Err(Error::NotSupported)
}

pub fn application_copies(_: &str) -> Vec<String> {
    Vec::new()
}

pub fn open_application(_: &str, _: &[String]) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn open_file(_: &str) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn quit_application(_: &str, _: bool) -> Result<()> {
    Err(Error::NotSupported)
}

pub fn open_url(_: &str, _: Option<&str>) -> Result<()> {
    Err(Error::NotSupported)
}
