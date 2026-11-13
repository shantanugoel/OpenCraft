//! A test command must not invoke itself again through its own mouse event.

#[cfg(all(target_os = "macos", not(test)))]
pub(super) fn from_this_process() -> bool {
    use objc2::{
        class,
        encode::{Encoding, RefEncode},
        msg_send,
        runtime::AnyObject,
    };

    #[repr(C)]
    struct CGEventOpaque {
        _private: [u8; 0],
    }
    // SAFETY: the encoding is a pointer to the opaque Quartz event type.
    unsafe impl RefEncode for CGEventOpaque {
        const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("__CGEvent", &[]));
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventGetIntegerValueField(event: *const CGEventOpaque, field: u32) -> i64;
    }
    // SAFETY: the window thread is handling a local AppKit event. AppKit
    // keeps it and its CGEvent alive for the duration of this call.
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let event: *mut AnyObject = msg_send![app, currentEvent];
        if event.is_null() {
            return false;
        }
        let cg: *const CGEventOpaque = msg_send![event, CGEvent];
        // kCGEventSourceUnixProcessID, from CGEventTypes.h.
        !cg.is_null() && CGEventGetIntegerValueField(cg, 41) == i64::from(std::process::id())
    }
}

#[cfg(any(not(target_os = "macos"), test))]
pub(super) fn from_this_process() -> bool {
    false
}
