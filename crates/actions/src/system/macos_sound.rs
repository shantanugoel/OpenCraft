//! Sound files through `AVAudioPlayer` of AVFoundation, a public framework of
//! macOS. Playback needs no permission and opens no input. The players live
//! in one list, thus "Stop all sound" can stop them. See
//! `docs/kb/macos-sound-files.md`.

use super::{Error, Result};
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use objc2::rc::{autoreleasepool, Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{class, msg_send};
use std::io;
use std::path::Path;
use std::sync::Mutex;

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

/// The largest number of sounds that play at the same time. A new sound
/// above this number stops the oldest sound.
const LIMIT: usize = 16;

struct Player(Retained<AnyObject>);
// SAFETY: Apple documents no thread affinity for `AVAudioPlayer`. OpenCraft
// sends messages to a player only while it holds the lock of `PLAYERS`.
unsafe impl Send for Player {}

static PLAYERS: Mutex<Vec<Player>> = Mutex::new(Vec::new());

fn failure(text: String) -> Error {
    Error::Io(io::Error::other(text))
}

/// The text for a file that macOS cannot read as sound. The localized text
/// of AVFoundation only gives a code, thus the text gives the code.
///
/// # Safety
/// `error` is null or an `NSError`.
unsafe fn error_text(error: *mut AnyObject) -> String {
    const TEXT: &str = "macOS cannot read the file as sound";
    if error.is_null() {
        return TEXT.into();
    }
    let code: isize = msg_send![error, code];
    format!("{TEXT} (code {code})")
}

pub fn play(file: &Path, volume: u8, device: Option<&str>) -> Result<()> {
    // A clear message for a missing file, before macOS reads it.
    if !std::fs::metadata(file)?.is_file() {
        return Err(failure("the path is not a file".into()));
    }
    let path = file
        .to_str()
        .ok_or_else(|| failure("the path is not UTF-8 text".into()))?;
    if AnyClass::get(c"AVAudioPlayer").is_none() {
        return Err(Error::NotSupported);
    }
    let mut players = PLAYERS.lock().unwrap_or_else(|e| e.into_inner());
    autoreleasepool(|_| {
        let path = CFString::new(path);
        // SAFETY: each message has the arguments and the result types of the
        // AVFoundation and Foundation headers. A CFString is an NSString.
        unsafe {
            let url: *mut AnyObject = msg_send![
                class!(NSURL),
                fileURLWithPath: path.as_concrete_TypeRef().cast::<AnyObject>()
            ];
            if url.is_null() {
                return Err(failure("macOS cannot make a URL of the path".into()));
            }
            let mut error: *mut AnyObject = std::ptr::null_mut();
            let player: Allocated<AnyObject> = msg_send![class!(AVAudioPlayer), alloc];
            let player: Option<Retained<AnyObject>> =
                msg_send![player, initWithContentsOfURL: url, error: &raw mut error];
            let Some(player) = player else {
                return Err(failure(error_text(error)));
            };
            if let Some(device) = device {
                let device = CFString::new(device);
                let _: () = msg_send![
                    &*player,
                    setCurrentDevice: device.as_concrete_TypeRef().cast::<AnyObject>()
                ];
            }
            let _: () = msg_send![&*player, setVolume: f32::from(volume.min(100)) / 100.0];
            let _: bool = msg_send![&*player, prepareToPlay];
            let started: bool = msg_send![&*player, play];
            if !started {
                return Err(failure("macOS cannot start the sound".into()));
            }
            // Forget the sounds that are at their end.
            players.retain(|p| {
                let playing: bool = msg_send![&*p.0, isPlaying];
                playing
            });
            if players.len() >= LIMIT {
                let oldest = players.remove(0);
                let _: () = msg_send![&*oldest.0, stop];
            }
            players.push(Player(player));
            Ok(())
        }
    })
}

pub fn stop_all() -> Result<()> {
    let mut players = PLAYERS.lock().unwrap_or_else(|e| e.into_inner());
    autoreleasepool(|_| {
        for player in players.drain(..) {
            // SAFETY: `stop` has no arguments and no result.
            let _: () = unsafe { msg_send![&*player.0, stop] };
        }
    });
    Ok(())
}
