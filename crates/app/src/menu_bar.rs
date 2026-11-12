//! The menu bar item and the Dock activation, with AppKit (decision 0020).
//!
//! This module is for macOS only. It is the only module with `unsafe` code
//! for AppKit. The content of the menu is in the module `menu`.

use crate::{
    menu::{Item, Kind, MenuState, ITEMS},
    shell::ShellCommand,
};
use async_channel::Sender;
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{NSObject, NSObjectProtocol, Sel},
    sel, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSControlStateValueOff, NSControlStateValueOn,
    NSImage, NSMenu, NSMenuItem, NSRunningApplication, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength,
};
use objc2_foundation::NSString;

/// The SF Symbol on the button: a grid of keys.
const SYMBOL: &str = "square.grid.3x2";
/// The accessibility description of the button, and its title if the image is
/// not available.
const NAME: &str = "OpenCraft";

pub struct Ivars {
    commands: Sender<ShellCommand>,
}

define_class!(
    // SAFETY: `NSObject` has no subclass requirements, and the class does
    // not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "OpenCraftMenuTarget"]
    #[ivars = Ivars]
    struct MenuTarget;

    unsafe impl NSObjectProtocol for MenuTarget {}

    impl MenuTarget {
        /// The action of each item. The tag of the item identifies it.
        #[unsafe(method(select:))]
        fn select(&self, sender: Option<&NSMenuItem>) {
            if let Some(command) = sender
                .and_then(|item| Item::from_tag(item.tag()))
                .and_then(Item::command)
            {
                let _ = self.ivars().commands.try_send(command);
            }
        }
    }
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker, commands: Sender<ShellCommand>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars { commands });
        // SAFETY: `init` of `NSObject` is the designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

/// The menu bar item. Keep it until the process stops: the item goes away
/// from the menu bar when it is dropped.
pub struct MenuBar {
    _status_item: Retained<NSStatusItem>,
    _menu: Retained<NSMenu>,
    _target: Retained<MenuTarget>,
    items: Vec<(Item, Retained<NSMenuItem>)>,
    shown: Option<MenuState>,
}

fn menu_item(mtm: MainThreadMarker, action: Option<Sel>, key: &str) -> Retained<NSMenuItem> {
    // SAFETY: the action is `select:` of `MenuTarget`, or no action.
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(""),
            action,
            &NSString::from_str(key),
        )
    }
}

impl MenuBar {
    /// Make the item. `None` if the caller is not on the main thread.
    pub fn new(commands: Sender<ShellCommand>) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let target = MenuTarget::new(mtm, commands);
        let menu = NSMenu::new(mtm);
        let mut items = Vec::new();
        for item in ITEMS {
            let entry = match item.kind() {
                Kind::Separator => NSMenuItem::separatorItem(mtm),
                Kind::Text => {
                    let entry = menu_item(mtm, None, "");
                    entry.setEnabled(false);
                    entry
                }
                Kind::Command | Kind::Check => {
                    let entry = menu_item(mtm, Some(sel!(select:)), item.key());
                    // SAFETY: the target stays alive as long as the menu.
                    unsafe { entry.setTarget(Some(&target)) };
                    entry
                }
            };
            entry.setTag(item.tag());
            menu.addItem(&entry);
            if item.kind() != Kind::Separator {
                items.push((item, entry));
            }
        }
        let status_item =
            NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        if let Some(button) = status_item.button(mtm) {
            let name = NSString::from_str(NAME);
            match NSImage::imageWithSystemSymbolName_accessibilityDescription(
                &NSString::from_str(SYMBOL),
                Some(&name),
            ) {
                Some(image) => {
                    // A template image follows the light and the dark menu bar.
                    image.setTemplate(true);
                    button.setImage(Some(&image));
                }
                None => button.setTitle(&name),
            }
        }
        status_item.setMenu(Some(&menu));
        Some(Self {
            _status_item: status_item,
            _menu: menu,
            _target: target,
            items,
            shown: None,
        })
    }

    /// Show `state` in the titles and the check marks.
    pub fn show(&mut self, state: &MenuState) {
        if self.shown.as_ref() == Some(state) {
            return;
        }
        for (item, entry) in &self.items {
            match state.title(*item) {
                Some(title) => {
                    entry.setTitle(&NSString::from_str(&title));
                    entry.setHidden(false);
                }
                None => entry.setHidden(true),
            }
            if item.kind() == Kind::Check {
                entry.setState(if state.checked(*item) {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
        }
        self.shown = Some(state.clone());
    }
}

/// If the application is active, activate the Dock application and give
/// true. Call this before the change to the regular activation policy, and
/// activate the application again some time after the change. See
/// `docs/kb/gpui.md`, "Windows, Dock icon, and activation".
pub fn activate_dock_if_active() -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    if !NSApplication::sharedApplication(mtm).isActive() {
        return false;
    }
    let docks = NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(
        "com.apple.dock",
    ));
    if let Some(dock) = docks.firstObject() {
        #[allow(deprecated)]
        dock.activateWithOptions(NSApplicationActivationOptions::empty());
    }
    true
}
