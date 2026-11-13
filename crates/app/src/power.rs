//! Observe macOS notices. Device policy belongs to the engine, not AppKit.

use crate::shell::ShellCommand;
use async_channel::Sender;
use objc2::{
    define_class, msg_send, rc::Retained, runtime::NSObjectProtocol, sel, DefinedClass,
    MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{
    NSDistributedNotificationCenter, NSNotification, NSNotificationCenter, NSObject, NSString,
};
use opencraft_engine::PowerNotice;

define_class!(
    // SAFETY: NSObject has no subclass requirements. Callbacks only enqueue messages.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "OpenCraftPowerObserver"]
    #[ivars = Sender<ShellCommand>]
    struct Observer;
    unsafe impl NSObjectProtocol for Observer {}
    impl Observer {
        #[unsafe(method(willSleep:))]
        fn will_sleep(&self, _: &NSNotification) { self.send(PowerNotice::WillSleep); }
        #[unsafe(method(didWake:))]
        fn did_wake(&self, _: &NSNotification) { self.send(PowerNotice::DidWake); }
        #[unsafe(method(displaysSleep:))]
        fn displays_sleep(&self, _: &NSNotification) { self.send(PowerNotice::DisplaysSleep); }
        #[unsafe(method(displaysWake:))]
        fn displays_wake(&self, _: &NSNotification) { self.send(PowerNotice::DisplaysWake); }
        #[unsafe(method(locked:))]
        fn locked(&self, _: &NSNotification) { self.send(PowerNotice::Locked); }
        #[unsafe(method(unlocked:))]
        fn unlocked(&self, _: &NSNotification) { self.send(PowerNotice::Unlocked); }
    }
);
impl Observer {
    fn send(&self, notice: PowerNotice) {
        let _ = self.ivars().try_send(ShellCommand::PowerNotice(notice));
    }
}

/// Retain the observer until the application run loop ends, then unregister it.
pub struct Notices {
    observer: Retained<Observer>,
    workspace: Retained<NSNotificationCenter>,
    distributed: Retained<NSDistributedNotificationCenter>,
}
impl Notices {
    pub fn new(commands: Sender<ShellCommand>) -> Self {
        let mtm = MainThreadMarker::new().expect("Power observers need the main thread");
        let allocated = Observer::alloc(mtm).set_ivars(commands);
        // SAFETY: NSObject init initializes the allocated observer.
        let observer: Retained<Observer> = unsafe { msg_send![super(allocated), init] };
        let workspace = NSWorkspace::sharedWorkspace().notificationCenter();
        let distributed = NSDistributedNotificationCenter::defaultCenter();
        // SAFETY: Each selector takes one notification. The retained observer
        // outlives its registration, which Drop removes on this same thread.
        unsafe {
            for (name, selector) in [
                ("NSWorkspaceWillSleepNotification", sel!(willSleep:)),
                ("NSWorkspaceDidWakeNotification", sel!(didWake:)),
                (
                    "NSWorkspaceScreensDidSleepNotification",
                    sel!(displaysSleep:),
                ),
                ("NSWorkspaceScreensDidWakeNotification", sel!(displaysWake:)),
            ] {
                workspace.addObserver_selector_name_object(
                    &observer,
                    selector,
                    Some(&NSString::from_str(name)),
                    None,
                );
            }
            for (name, selector) in [
                ("com.apple.screenIsLocked", sel!(locked:)),
                ("com.apple.screenIsUnlocked", sel!(unlocked:)),
            ] {
                distributed.addObserver_selector_name_object(
                    &observer,
                    selector,
                    Some(&NSString::from_str(name)),
                    None,
                );
            }
        }
        Self {
            observer,
            workspace,
            distributed,
        }
    }
}
impl Drop for Notices {
    fn drop(&mut self) {
        // SAFETY: Remove the live observer before releasing it.
        unsafe {
            self.workspace.removeObserver(&self.observer);
            self.distributed.removeObserver(&self.observer);
        }
    }
}
