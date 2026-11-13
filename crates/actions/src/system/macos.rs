//! The macOS code for [`super`].
//!
//! - Keys and text: Quartz Event Services (`CGEvent`).
//! - Media keys: a system-defined `NSEvent`, sent as a `CGEvent`.
//! - Sound volume: Core Audio, the default output device.
//! - Applications and URLs: the program `/usr/bin/open`.

use super::{Error, MediaKey, Result, Volume};
use crate::hotkey::Hotkey;
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2::encode::{Encode, Encoding, RefEncode};
use objc2::rc::autoreleasepool;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use std::ffi::c_void;
use std::process::{Command, Stdio};

// --- The permission to send keys ---------------------------------------------

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
}

pub fn can_send_keys() -> bool {
    // SAFETY: the function has no argument and reads the state of the process.
    unsafe { AXIsProcessTrusted() }
}

pub fn ask_to_send_keys() {
    // The value of the constant `kAXTrustedCheckOptionPrompt`.
    let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
    let options = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
    // SAFETY: `options` is a dictionary that lives until after the call.
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef().cast()) };
}

pub fn mouse_event(event: crate::mouse::MouseEvent) -> Result<()> {
    super::macos_mouse::send(event)
}

pub fn mac_function(function: crate::mac_function::MacFunction) -> Result<()> {
    super::macos_functions::run(function)
}

// --- Keys and text -----------------------------------------------------------

/// The virtual key code of a key name, for the ANSI keyboard layout.
fn key_code(name: &str) -> Option<CGKeyCode> {
    const LETTERS: [CGKeyCode; 26] = [
        0x00, 0x0B, 0x08, 0x02, 0x0E, 0x03, 0x05, 0x04, 0x22, 0x26, 0x28, 0x25, 0x2E, 0x2D, 0x1F,
        0x23, 0x0C, 0x0F, 0x01, 0x11, 0x20, 0x09, 0x0D, 0x07, 0x10, 0x06,
    ];
    const DIGITS: [CGKeyCode; 10] = [0x1D, 0x12, 0x13, 0x14, 0x15, 0x17, 0x16, 0x1A, 0x1C, 0x19];
    const FUNCTION: [CGKeyCode; 20] = [
        0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, 0x67, 0x6F, 0x69, 0x6B, 0x71,
        0x6A, 0x40, 0x4F, 0x50, 0x5A,
    ];
    let bytes = name.as_bytes();
    if let [c] = bytes {
        if c.is_ascii_lowercase() {
            return Some(LETTERS[usize::from(c - b'a')]);
        }
        if c.is_ascii_digit() {
            return Some(DIGITS[usize::from(c - b'0')]);
        }
    }
    if let Some(n) = name.strip_prefix('f').and_then(|n| n.parse::<usize>().ok()) {
        return FUNCTION.get(n.checked_sub(1)?).copied();
    }
    Some(match name {
        "return" => 0x24,
        "tab" => 0x30,
        "space" => 0x31,
        "delete" => 0x33,
        "forwarddelete" => 0x75,
        "escape" => 0x35,
        "left" => 0x7B,
        "right" => 0x7C,
        "down" => 0x7D,
        "up" => 0x7E,
        "home" => 0x73,
        "end" => 0x77,
        "pageup" => 0x74,
        "pagedown" => 0x79,
        "minus" => 0x1B,
        "equal" => 0x18,
        "leftbracket" => 0x21,
        "rightbracket" => 0x1E,
        "backslash" => 0x2A,
        "semicolon" => 0x29,
        "quote" => 0x27,
        "comma" => 0x2B,
        "period" => 0x2F,
        "slash" => 0x2C,
        "grave" => 0x32,
        _ => return None,
    })
}

fn key_event(code: CGKeyCode, down: bool) -> Result<CGEvent> {
    let source =
        CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|()| Error::System(0))?;
    CGEvent::new_keyboard_event(source, code, down).map_err(|()| Error::System(0))
}

pub fn send_hotkey(hotkey: &Hotkey) -> Result<()> {
    // Build every event before posting any. Allocation failure must not leave
    // a modifier down. Legacy chords keep their original two-event path.
    let events = hotkey_events(hotkey)?;
    for event in events {
        event.post(CGEventTapLocation::HID);
    }
    Ok(())
}

fn hotkey_events(hotkey: &Hotkey) -> Result<Vec<CGEvent>> {
    if hotkey.key.is_empty()
        || hotkey.right_ctrl
        || hotkey.right_alt
        || hotkey.right_shift
        || hotkey.right_cmd
        || hotkey.function
    {
        return extended_hotkey_events(hotkey);
    }
    let code = key_code(&hotkey.key).ok_or(Error::NotSupported)?;
    let mut flags = CGEventFlags::CGEventFlagNull;
    for (on, flag) in [
        (hotkey.ctrl, CGEventFlags::CGEventFlagControl),
        (hotkey.alt, CGEventFlags::CGEventFlagAlternate),
        (hotkey.shift, CGEventFlags::CGEventFlagShift),
        (hotkey.cmd, CGEventFlags::CGEventFlagCommand),
    ] {
        if on {
            flags |= flag;
        }
    }
    [true, false]
        .into_iter()
        .map(|down| {
            let event = key_event(code, down)?;
            event.set_flags(flags);
            Ok(event)
        })
        .collect()
}

fn extended_hotkey_events(hotkey: &Hotkey) -> Result<Vec<CGEvent>> {
    // Virtual codes: Events.h. Device-side masks: IOLLEvent.h.
    let modifiers: Vec<(u16, u64)> = [
        (hotkey.ctrl, 0x3B, (1 << 18) | 0x01),
        (hotkey.right_ctrl, 0x3E, (1 << 18) | 0x2000),
        (hotkey.alt, 0x3A, (1 << 19) | 0x20),
        (hotkey.right_alt, 0x3D, (1 << 19) | 0x40),
        (hotkey.shift, 0x38, (1 << 17) | 0x02),
        (hotkey.right_shift, 0x3C, (1 << 17) | 0x04),
        (hotkey.cmd, 0x37, (1 << 20) | 0x08),
        (hotkey.right_cmd, 0x36, (1 << 20) | 0x10),
        (hotkey.function, 0x3F, 1 << 23),
    ]
    .into_iter()
    .filter_map(|(on, code, mask)| on.then_some((code, mask)))
    .collect();
    if modifiers.is_empty() && hotkey.key.is_empty() {
        return Err(Error::NotSupported);
    }
    let mut events = Vec::new();
    let flags = |count: usize| {
        CGEventFlags::from_bits_retain(
            modifiers[..count]
                .iter()
                .fold(0, |flags, (_, mask)| flags | mask),
        )
    };
    for (index, &(code, _)) in modifiers.iter().enumerate() {
        let event = key_event(code, true)?;
        event.set_type(CGEventType::FlagsChanged);
        event.set_flags(flags(index + 1));
        events.push(event);
    }
    if !hotkey.key.is_empty() {
        let code = key_code(&hotkey.key).ok_or(Error::NotSupported)?;
        for down in [true, false] {
            let event = key_event(code, down)?;
            event.set_flags(flags(modifiers.len()));
            events.push(event);
        }
    }
    for (index, &(code, _)) in modifiers.iter().enumerate().rev() {
        let event = key_event(code, false)?;
        event.set_type(CGEventType::FlagsChanged);
        event.set_flags(flags(index));
        events.push(event);
    }
    Ok(events)
}

/// The largest number of UTF-16 units that one key event can hold.
const TEXT_UNITS: usize = 20;

pub fn type_text(text: &str) -> Result<()> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut start = 0;
    while start < units.len() {
        let mut end = (start + TEXT_UNITS).min(units.len());
        // Do not cut a character that has two units.
        if end < units.len() && (0xDC00..0xE000).contains(&units[end]) {
            end -= 1;
        }
        for down in [true, false] {
            let event = key_event(0, down)?;
            // The text must not get the modifier keys that the user holds.
            event.set_flags(CGEventFlags::CGEventFlagNull);
            event.set_string_from_utf16_unchecked(&units[start..end]);
            event.post(CGEventTapLocation::HID);
        }
        start = end;
    }
    Ok(())
}

// --- Media keys --------------------------------------------------------------

#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventPost(tap: u32, event: *const CGEventOpaque);
}

/// The type `struct __CGEvent` of Core Graphics.
#[repr(C)]
struct CGEventOpaque {
    _private: [u8; 0],
}

// SAFETY: the encoding is that of a pointer to `struct __CGEvent`.
unsafe impl RefEncode for CGEventOpaque {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("__CGEvent", &[]));
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

// SAFETY: the type has the layout and the encoding of `CGPoint`.
unsafe impl Encode for Point {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}

pub fn media_key(key: MediaKey) -> Result<()> {
    // The key types of `IOKit/hidsystem/ev_keymap.h`.
    let code: isize = match key {
        MediaKey::VolumeUp => 0,
        MediaKey::VolumeDown => 1,
        MediaKey::Mute => 7,
        MediaKey::PlayPause => 16,
        MediaKey::Next => 17,
        MediaKey::Previous => 18,
    };
    /// `NSEventTypeSystemDefined`.
    const SYSTEM_DEFINED: usize = 14;
    /// The subtype of a media key event.
    const AUX_CONTROL_BUTTONS: i16 = 8;
    autoreleasepool(|_| {
        for state in [0xA_isize, 0xB] {
            let flags = (state as usize) << 8;
            let data1 = (code << 16) | (state << 8);
            // SAFETY: the selector and the argument types are those of the
            // class method of `NSEvent`. The result is an object or nil.
            let event: *mut AnyObject = unsafe {
                msg_send![
                    class!(NSEvent),
                    otherEventWithType: SYSTEM_DEFINED,
                    location: Point { x: 0.0, y: 0.0 },
                    modifierFlags: flags,
                    timestamp: 0.0_f64,
                    windowNumber: 0_isize,
                    context: std::ptr::null::<AnyObject>(),
                    subtype: AUX_CONTROL_BUTTONS,
                    data1: data1,
                    data2: -1_isize,
                ]
            };
            if event.is_null() {
                return Err(Error::System(0));
            }
            // SAFETY: `event` is an `NSEvent`. Its `CGEvent` lives as long
            // as the event, which the pool keeps until the end of the block.
            let cg: *const CGEventOpaque = unsafe { msg_send![event, CGEvent] };
            if cg.is_null() {
                return Err(Error::System(0));
            }
            // SAFETY: `cg` is a valid event. 0 is the HID event tap.
            unsafe { CGEventPost(0, cg) };
        }
        Ok(())
    })
}

// --- Sound volume ------------------------------------------------------------

#[derive(Clone, Copy)]
#[repr(C)]
struct PropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectHasProperty(object: u32, address: *const PropertyAddress) -> bool;
    fn AudioObjectIsPropertySettable(
        object: u32,
        address: *const PropertyAddress,
        settable: *mut u8,
    ) -> i32;
    fn AudioObjectAddPropertyListener(
        object: u32,
        address: *const PropertyAddress,
        listener: AudioListener,
        data: *mut c_void,
    ) -> i32;
    fn AudioObjectRemovePropertyListener(
        object: u32,
        address: *const PropertyAddress,
        listener: AudioListener,
        data: *mut c_void,
    ) -> i32;
    fn AudioObjectGetPropertyData(
        object: u32,
        address: *const PropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        size: *mut u32,
        data: *mut c_void,
    ) -> i32;
    fn AudioObjectSetPropertyData(
        object: u32,
        address: *const PropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        size: u32,
        data: *const c_void,
    ) -> i32;
}

const fn four(code: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*code)
}

const SYSTEM_OBJECT: u32 = 1;
const DEFAULT_OUTPUT: PropertyAddress = PropertyAddress {
    selector: four(b"dOut"),
    scope: four(b"glob"),
    element: 0,
};
/// The volume of the device as the user sees it, from 0.0 to 1.0.
const MAIN_VOLUME: PropertyAddress = PropertyAddress {
    selector: four(b"vmvc"),
    scope: four(b"outp"),
    element: 0,
};
const MUTE: PropertyAddress = PropertyAddress {
    selector: four(b"mute"),
    scope: four(b"outp"),
    element: 0,
};

fn get<T: Copy>(object: u32, address: &PropertyAddress, mut value: T) -> Result<T> {
    let mut size = size_of::<T>() as u32;
    // SAFETY: `value` has the size `size`, and the property has the type `T`.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            address,
            0,
            std::ptr::null(),
            &mut size,
            (&raw mut value).cast(),
        )
    };
    if status == 0 {
        Ok(value)
    } else {
        Err(Error::System(status))
    }
}

fn set<T: Copy>(object: u32, address: &PropertyAddress, value: T) -> Result<()> {
    // SAFETY: `value` has the size that the call gives, and the property has
    // the type `T`.
    let status = unsafe {
        AudioObjectSetPropertyData(
            object,
            address,
            0,
            std::ptr::null(),
            size_of::<T>() as u32,
            (&raw const value).cast(),
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err(Error::System(status))
    }
}

fn output_device() -> Result<u32> {
    match get(SYSTEM_OBJECT, &DEFAULT_OUTPUT, 0_u32) {
        Ok(0) | Err(_) => Err(Error::NoVolume),
        Ok(device) => Ok(device),
    }
}

pub fn volume() -> Result<Volume> {
    let device = output_device()?;
    let level = get(device, &MAIN_VOLUME, 0.0_f32).map_err(|_| Error::NoVolume)?;
    // A device with no mute control is not muted.
    let muted = get(device, &MUTE, 0_u32).unwrap_or(0) != 0;
    Ok(Volume {
        level: (level.clamp(0.0, 1.0) * 100.0).round() as u8,
        muted,
    })
}

pub fn set_volume_level(level: u8) -> Result<()> {
    let device = output_device()?;
    set(device, &MAIN_VOLUME, f32::from(level) / 100.0).map_err(|_| Error::NoVolume)
}

pub fn set_muted(muted: bool) -> Result<()> {
    set(output_device()?, &MUTE, u32::from(muted))
}

// --- Input mute and property notifications ------------------------------------

const DEFAULT_INPUT: PropertyAddress = PropertyAddress {
    selector: four(b"dIn "),
    scope: four(b"glob"),
    element: 0,
};
const INPUT_MUTE: PropertyAddress = PropertyAddress {
    selector: four(b"mute"),
    scope: four(b"inpt"),
    element: 0,
};
const DEVICES: PropertyAddress = PropertyAddress {
    selector: four(b"dev#"),
    scope: four(b"glob"),
    element: 0,
};

fn input_device() -> Result<u32> {
    get(SYSTEM_OBJECT, &DEFAULT_INPUT, 0_u32)
}

fn writable_mute(device: u32) -> bool {
    let mut writable = 0_u8;
    // SAFETY: the address and writable byte live through these calls.
    unsafe {
        AudioObjectHasProperty(device, &INPUT_MUTE)
            && AudioObjectIsPropertySettable(device, &INPUT_MUTE, &mut writable) == 0
            && writable != 0
    }
}

pub fn microphone_muted() -> Result<Option<bool>> {
    let device = input_device()?;
    if device == 0 || !writable_mute(device) {
        return Ok(None);
    }
    get(device, &INPUT_MUTE, 0_u32).map(|value| Some(value != 0))
}

pub fn set_microphone_muted(muted: bool) -> Result<()> {
    let device = input_device()?;
    if device == 0 || !writable_mute(device) {
        return Err(Error::NoInputMute);
    }
    set(device, &INPUT_MUTE, u32::from(muted))
}

type AudioListener = unsafe extern "C" fn(u32, u32, *const PropertyAddress, *mut c_void) -> i32;
type Notifications = std::collections::BTreeMap<usize, std::sync::mpsc::Sender<()>>;
fn notifications() -> &'static std::sync::Mutex<Notifications> {
    static SENDERS: std::sync::OnceLock<std::sync::Mutex<Notifications>> =
        std::sync::OnceLock::new();
    SENDERS.get_or_init(Default::default)
}

unsafe extern "C" fn audio_changed(
    _: u32,
    _: u32,
    _: *const PropertyAddress,
    data: *mut c_void,
) -> i32 {
    // The pointer is only an opaque token, never dereferenced. A callback
    // already in flight after removal cannot access a dropped watch.
    if let Some(sender) = notifications()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&(data as usize))
    {
        let _ = sender.send(());
    }
    0
}

struct InputWatch {
    token: usize,
    properties: Vec<(u32, PropertyAddress)>,
    input: u32,
}

impl InputWatch {
    fn add(&mut self, object: u32, address: PropertyAddress) -> Result<()> {
        // SAFETY: callback and token remain valid even if removal races a callback.
        let status = unsafe {
            AudioObjectAddPropertyListener(
                object,
                &address,
                audio_changed,
                self.token as *mut c_void,
            )
        };
        if status != 0 {
            return Err(Error::System(status));
        }
        self.properties.push((object, address));
        Ok(())
    }
    fn remove(&self, object: u32, address: &PropertyAddress) {
        // SAFETY: this matches the registration. Tokens name no allocated memory.
        unsafe {
            AudioObjectRemovePropertyListener(
                object,
                address,
                audio_changed,
                self.token as *mut c_void,
            );
        }
    }
}

impl super::AudioWatch for InputWatch {
    fn refresh(&mut self) -> Result<()> {
        let input = input_device()?;
        if input != self.input {
            for (object, address) in self.properties.drain(2..).collect::<Vec<_>>() {
                self.remove(object, &address);
            }
            self.input = 0;
            if input != 0 && writable_mute(input) {
                self.add(input, INPUT_MUTE)?;
            }
            self.input = input;
        }
        Ok(())
    }
}

impl Drop for InputWatch {
    fn drop(&mut self) {
        notifications()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.token);
        for (object, address) in &self.properties {
            self.remove(*object, address);
        }
    }
}

pub fn watch_microphone(
    changed: std::sync::mpsc::Sender<()>,
) -> Result<Box<dyn super::AudioWatch>> {
    use super::AudioWatch;
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    let token = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    notifications()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(token, changed);
    let mut watch = InputWatch {
        token,
        properties: Vec::new(),
        input: 0,
    };
    watch.add(SYSTEM_OBJECT, DEFAULT_INPUT)?;
    watch.add(SYSTEM_OBJECT, DEVICES)?;
    watch.refresh()?;
    Ok(Box::new(watch))
}

// --- Applications and URLs ---------------------------------------------------

fn open(arguments: &[&str]) -> Result<()> {
    let status = Command::new("/usr/bin/open")
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    match status.code() {
        Some(0) => Ok(()),
        code => Err(Error::System(code.unwrap_or(-1))),
    }
}

pub fn open_application(application: &str) -> Result<()> {
    open(&["-a", application])
}

pub fn open_url(url: &str) -> Result<()> {
    open(&["--", url])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkey::{is_key_name, KEY_NAMES};

    #[test]
    fn extended_hotkeys_have_balanced_sides_and_flags() {
        use core_graphics::event::EventField;
        for (value, codes) in [
            ("rightctrl+fn+a", vec![0x3E, 0x3F, 0, 0, 0x3F, 0x3E]),
            ("rightalt", vec![0x3D, 0x3D]),
            ("fn", vec![0x3F, 0x3F]),
            ("ctrl+rightctrl", vec![0x3B, 0x3E, 0x3E, 0x3B]),
        ] {
            let events = hotkey_events(&value.parse().unwrap()).unwrap();
            assert_eq!(
                events
                    .iter()
                    .map(|e| e.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE))
                    .collect::<Vec<_>>(),
                codes
            );
            assert_eq!(
                events.last().unwrap().get_flags(),
                CGEventFlags::CGEventFlagNull
            );
            assert_eq!(
                events.first().unwrap().get_type() as u32,
                CGEventType::FlagsChanged as u32
            );
        }
        let events = hotkey_events(&"cmd+a".parse().unwrap()).unwrap();
        assert_eq!(events.len(), 2, "old profile events do not change");
        assert!(events
            .iter()
            .all(|e| e.get_flags() == CGEventFlags::CGEventFlagCommand));
    }

    #[test]
    fn each_key_name_has_a_key_code() {
        let mut names: Vec<String> = KEY_NAMES.iter().map(|n| n.to_string()).collect();
        names.extend(('a'..='z').map(String::from));
        names.extend(('0'..='9').map(String::from));
        names.extend((1..=20).map(|n| format!("f{n}")));
        let mut codes = Vec::new();
        for name in &names {
            assert!(is_key_name(name), "{name}");
            codes.push(key_code(name).unwrap_or_else(|| panic!("{name}")));
        }
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), names.len(), "two names have one code");
        assert_eq!(key_code("f21"), None);
        assert_eq!(key_code("f0"), None);
    }
}
