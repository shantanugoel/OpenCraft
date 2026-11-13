//! Quit an application with the running-application calls of AppKit.
//!
//! `NSRunningApplication` `terminate` and `forceTerminate` send no Apple
//! event, so macOS asks for no Automation permission. See
//! `docs/kb/macos-open-and-quit.md`.

use super::{Error, Result};
use objc2::rc::autoreleasepool;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use std::ffi::{c_char, CStr};

/// `NSApplicationActivationPolicyRegular` and `…Accessory`: the applications
/// with a Dock icon or a menu bar item. The system and background processes
/// have the third policy, and the action never stops them.
const REGULAR: isize = 0;
const ACCESSORY: isize = 1;

/// What the system tells about one running application.
struct Running<'a> {
    name: &'a str,
    bundle_identifier: &'a str,
    bundle_path: &'a str,
}

/// True if `query` names `application`. A query with a `/` is the path of
/// the bundle. Another query is the name (with or without `.app`) or the
/// bundle identifier. The comparison ignores the case of letters.
fn matches(query: &str, application: &Running) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return false;
    }
    if query.contains('/') {
        let path = |p: &str| p.trim_end_matches('/').to_lowercase();
        return path(query) == path(application.bundle_path);
    }
    let name = query
        .strip_suffix(".app")
        .or_else(|| query.strip_suffix(".APP"))
        .unwrap_or(query);
    [application.name, application.bundle_identifier]
        .iter()
        .any(|candidate| {
            candidate.eq_ignore_ascii_case(name) || candidate.eq_ignore_ascii_case(query)
        })
}

/// The text of an `NSString`, or an empty text for nil.
///
/// # Safety
/// `string` is nil or an `NSString`.
unsafe fn text<'a>(string: *mut AnyObject) -> &'a str {
    if string.is_null() {
        return "";
    }
    let bytes: *const c_char = msg_send![string, UTF8String];
    if bytes.is_null() {
        return "";
    }
    CStr::from_ptr(bytes).to_str().unwrap_or("")
}

/// Ask each running application that `application` names to quit, or force
/// it to stop. The call does not wait. An application can show a dialog (for
/// example "Save changes?") or refuse the normal request. `force` ends it
/// at once and loses unsaved changes.
pub(super) fn quit(application: &str, force: bool) -> Result<()> {
    let own = std::process::id() as i32;
    autoreleasepool(|_| {
        // SAFETY: the selectors and the argument types are those of
        // `NSWorkspace` and `NSRunningApplication`. Each object comes from
        // the system and stays valid inside this autorelease pool.
        unsafe {
            let workspace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
            let all: *mut AnyObject = msg_send![workspace, runningApplications];
            let count: usize = msg_send![all, count];
            let (mut found, mut sent) = (false, false);
            for index in 0..count {
                let app: *mut AnyObject = msg_send![all, objectAtIndex: index];
                let policy: isize = msg_send![app, activationPolicy];
                let pid: i32 = msg_send![app, processIdentifier];
                if !(policy == REGULAR || policy == ACCESSORY) || pid == own {
                    continue;
                }
                let url: *mut AnyObject = msg_send![app, bundleURL];
                let path: *mut AnyObject = if url.is_null() {
                    std::ptr::null_mut()
                } else {
                    msg_send![url, path]
                };
                let running = Running {
                    name: text(msg_send![app, localizedName]),
                    bundle_identifier: text(msg_send![app, bundleIdentifier]),
                    bundle_path: text(path),
                };
                if !matches(application, &running) {
                    continue;
                }
                found = true;
                let accepted: bool = if force {
                    msg_send![app, forceTerminate]
                } else {
                    msg_send![app, terminate]
                };
                sent |= accepted;
            }
            match (found, sent) {
                (false, _) => Err(Error::NotRunning),
                // The system refused each request.
                (true, false) => Err(Error::System(-1)),
                (true, true) => Ok(()),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calculator() -> Running<'static> {
        Running {
            name: "Calculator",
            bundle_identifier: "com.apple.calculator",
            bundle_path: "/System/Applications/Calculator.app",
        }
    }

    #[test]
    fn a_name_an_identifier_or_a_path_matches_with_any_letter_case() {
        for query in [
            "Calculator",
            "calculator",
            " Calculator.app ",
            "com.apple.Calculator",
            "/System/Applications/Calculator.app",
            "/system/applications/calculator.app/",
        ] {
            assert!(matches(query, &calculator()), "{query}");
        }
    }

    #[test]
    fn other_names_and_empty_text_do_not_match() {
        for query in [
            "",
            "  ",
            "Calc",
            "Calculator2",
            "/Applications/Calculator.app",
            "apple",
        ] {
            assert!(!matches(query, &calculator()), "{query}");
        }
    }

    #[test]
    fn an_application_that_does_not_run_is_an_error_and_nothing_stops() {
        assert!(matches!(
            quit("OpenCraft Test Application That Does Not Run", true),
            Err(Error::NotRunning)
        ));
    }
}
