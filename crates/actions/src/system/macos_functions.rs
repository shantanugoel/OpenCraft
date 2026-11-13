//! Public macOS calls only. See docs/kb/macos-system-functions.md.

use super::{Error, Result};
use crate::mac_function::MacFunction;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Call {
    OpenBundle(&'static str),
    Hotkey(&'static str),
    Sleep,
}

/// Keep the call selection separate from execution. Tests inspect sleep,
/// lock and screen saver without ever doing those operations.
fn call(function: MacFunction) -> Call {
    match function {
        MacFunction::MissionControl => Call::OpenBundle("com.apple.exposelauncher"),
        // Apple documents this shortcut for Launchpad on older macOS and
        // for its replacement, Apps, on macOS 26 and later.
        MacFunction::Launchpad => Call::Hotkey("shift+fn+a"),
        MacFunction::Screenshot => Call::Hotkey("shift+cmd+5"),
        MacFunction::ScreenSaver => Call::OpenBundle("com.apple.ScreenSaver.Engine"),
        MacFunction::Sleep => Call::Sleep,
        MacFunction::Lock => Call::Hotkey("ctrl+cmd+q"),
        MacFunction::ShowDesktop => Call::Hotkey("fn+f11"),
    }
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOPMFindPowerManagement(master_port: u32) -> u32;
    fn IOPMSleepSystem(connection: u32) -> i32;
    fn IOServiceClose(connection: u32) -> i32;
}

pub(super) fn run(function: MacFunction) -> Result<()> {
    match call(function) {
        Call::OpenBundle(identifier) => {
            let status = Command::new("/usr/bin/open")
                .args(["-b", identifier])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            if status.success() {
                Ok(())
            } else {
                Err(Error::System(status.code().unwrap_or(-1)))
            }
        }
        Call::Hotkey(text) => {
            let hotkey = text.parse().map_err(|_| Error::NotSupported)?;
            super::macos::send_hotkey(&hotkey)
        }
        Call::Sleep => {
            // SAFETY: MACH_PORT_NULL requests the root power domain. This
            // public IOPMLib call permits the console user, not just root.
            let connection = unsafe { IOPMFindPowerManagement(0) };
            if connection == 0 {
                return Err(Error::NotSupported);
            }
            // SAFETY: the connection was returned by IOPMFindPowerManagement.
            // Close it on both success and failure. No entitlement is added.
            let status = unsafe {
                let status = IOPMSleepSystem(connection);
                IOServiceClose(connection);
                status
            };
            if status == 0 {
                Ok(())
            } else {
                Err(Error::System(status))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_calls_are_selected_without_executing_them() {
        assert_eq!(
            call(MacFunction::MissionControl),
            Call::OpenBundle("com.apple.exposelauncher")
        );
        assert_eq!(call(MacFunction::Launchpad), Call::Hotkey("shift+fn+a"));
        assert_eq!(call(MacFunction::Screenshot), Call::Hotkey("shift+cmd+5"));
        assert_eq!(
            call(MacFunction::ScreenSaver),
            Call::OpenBundle("com.apple.ScreenSaver.Engine")
        );
        assert_eq!(call(MacFunction::Sleep), Call::Sleep);
        assert_eq!(call(MacFunction::Lock), Call::Hotkey("ctrl+cmd+q"));
        assert_eq!(call(MacFunction::ShowDesktop), Call::Hotkey("fn+f11"));
        for function in MacFunction::ALL {
            if let Call::Hotkey(text) = call(function) {
                assert!(text.parse::<crate::hotkey::Hotkey>().is_ok());
            }
        }
    }
}
