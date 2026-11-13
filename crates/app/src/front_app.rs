//! Observe the front application. The engine holds the policy (decision 0035).
//!
//! The observer uses `NSWorkspaceDidActivateApplicationNotification`. It does
//! not poll, and it starts no timer.

use crate::shell::ShellCommand;
use async_channel::Sender;
use objc2::{
    define_class, msg_send, rc::Retained, runtime::NSObjectProtocol, sel, DefinedClass,
    MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey,
    NSWorkspaceDidActivateApplicationNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObject, NSString};
use opencraft_engine::FrontApplication;

define_class!(
    // SAFETY: NSObject has no subclass requirements. The callback only
    // enqueues a message.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "OpenCraftFrontApplicationObserver"]
    #[ivars = Sender<ShellCommand>]
    struct Observer;
    unsafe impl NSObjectProtocol for Observer {}
    impl Observer {
        #[unsafe(method(didActivate:))]
        fn did_activate(&self, notification: &NSNotification) {
            let Some(application) = application_of(notification) else {
                return;
            };
            self.send(front_of(&application));
        }
    }
);

impl Observer {
    fn send(&self, front: FrontApplication) {
        let _ = self.ivars().try_send(ShellCommand::FrontApplication(front));
    }
}

/// The front application of an activation notice.
fn application_of(notification: &NSNotification) -> Option<Retained<NSRunningApplication>> {
    let info = notification.userInfo()?;
    // SAFETY: The notice stores an NSRunningApplication under this key.
    let info = unsafe { info.cast_unchecked::<NSString, objc2::runtime::AnyObject>() };
    let value = info.objectForKey(unsafe { NSWorkspaceApplicationKey })?;
    value.downcast().ok()
}

/// The report for `application`. The process number wins over the bundle
/// identifier, because a development build has no bundle.
fn front_of(application: &NSRunningApplication) -> FrontApplication {
    classify(
        application.processIdentifier(),
        application
            .bundleIdentifier()
            .map(|identifier| identifier.to_string())
            .as_deref(),
    )
}

/// `Shell` when `process` is this process. `Bundle` when `bundle_id` is
/// present. `NoBundle` when it is not.
pub(crate) fn classify(process: i32, bundle_id: Option<&str>) -> FrontApplication {
    if process as u32 == std::process::id() {
        return FrontApplication::Shell;
    }
    match bundle_id {
        Some(id) if !id.is_empty() => FrontApplication::Bundle(id.to_string()),
        _ => FrontApplication::NoBundle,
    }
}

/// Retain the observer until the application ends, then unregister it.
/// Also send the front application that is active now.
pub struct Notices {
    observer: Retained<Observer>,
    workspace: Retained<NSNotificationCenter>,
}

impl Notices {
    pub fn new(commands: Sender<ShellCommand>) -> Self {
        let mtm =
            MainThreadMarker::new().expect("The front application observer needs the main thread");
        let allocated = Observer::alloc(mtm).set_ivars(commands.clone());
        // SAFETY: NSObject init initializes the allocated observer.
        let observer: Retained<Observer> = unsafe { msg_send![super(allocated), init] };
        let workspace = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: The selector takes one notification. The retained observer
        // outlives its registration, which Drop removes on this same thread.
        unsafe {
            workspace.addObserver_selector_name_object(
                &observer,
                sel!(didActivate:),
                Some(NSWorkspaceDidActivateApplicationNotification),
                None,
            );
        }
        if let Some(application) = NSWorkspace::sharedWorkspace().frontmostApplication() {
            let _ = commands.try_send(ShellCommand::FrontApplication(front_of(&application)));
        }
        Self {
            observer,
            workspace,
        }
    }
}

impl Drop for Notices {
    fn drop(&mut self) {
        // SAFETY: Remove the live observer before releasing it.
        unsafe {
            self.workspace.removeObserver(&self.observer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_process_of_opencraft_is_the_shell_even_with_a_bundle() {
        let process = std::process::id() as i32;
        assert_eq!(
            classify(process, Some("com.example.OpenCraft")),
            FrontApplication::Shell
        );
        assert_eq!(classify(process, None), FrontApplication::Shell);
    }

    #[test]
    fn another_process_reports_its_bundle_or_the_lack_of_one() {
        assert_eq!(
            classify(1, Some("com.apple.calculator")),
            FrontApplication::Bundle("com.apple.calculator".into())
        );
        assert_eq!(classify(1, None), FrontApplication::NoBundle);
        assert_eq!(classify(1, Some("")), FrontApplication::NoBundle);
    }
}
