//! The menu bar item and the Dock activation, with AppKit (decision 0020).
//!
//! This module is for macOS only. The content of the menu is in the module
//! `menu`. The scan of application icons also calls AppKit
//! (`applications`).

use crate::{
    menu::{same_shape, Brightness, MenuState, Row, HIGHEST_BRIGHTNESS},
    shell::ShellCommand,
};
use async_channel::Sender;
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
    sel, AnyThread, ClassType, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAccessibility, NSApplication, NSApplicationActivationOptions, NSAutoresizingMaskOptions,
    NSColor, NSControlStateValueOn, NSFont, NSFontAttributeName, NSForegroundColorAttributeName,
    NSImage, NSImageView, NSMenu, NSMenuDelegate, NSMenuItem, NSRunningApplication, NSSlider,
    NSStatusBar, NSStatusItem, NSTextAlignment, NSTextField, NSVariableStatusItemLength, NSView,
    NSWorkspace,
};
use objc2_foundation::{
    NSMutableAttributedString, NSPoint, NSRange, NSRect, NSSize, NSString, NSURL,
};
use std::{
    cell::{Cell, RefCell},
    path::Path,
};

/// The SF Symbol on the button: a grid of keys.
const SYMBOL: &str = "square.grid.3x2";
/// The accessibility description of the button, and its title if the image is
/// not available.
const NAME: &str = "OpenCraft";

/// The size of the brightness row (decision 0029, section 19): the width of
/// the row, and the padding at the top, the sides, and the bottom, in points.
const ROW_WIDTH: f64 = 270.0;
const ROW_PAD_TOP: f64 = 4.0;
const ROW_PAD_SIDE: f64 = 14.0;
const ROW_PAD_BOTTOM: f64 = 8.0;
/// The height of the line with the title and the value, of the line with the
/// slider, and the space between them.
const ROW_TITLE_HEIGHT: f64 = 17.0;
const ROW_SLIDER_HEIGHT: f64 = 20.0;
const ROW_GAP: f64 = 2.0;
/// The width of the sun icons, and the space between an icon and the slider.
const SUN_WIDTH: f64 = 16.0;
const SUN_GAP: f64 = 6.0;

pub struct Ivars {
    commands: Sender<ShellCommand>,
    /// The command of each item that the user can select. The tag of an item
    /// is its position here.
    table: RefCell<Vec<Option<ShellCommand>>>,
    /// The label with the value of the brightness row.
    value: RefCell<Option<Retained<NSTextField>>>,
    /// True while the menu is open.
    open: Cell<bool>,
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

    unsafe impl NSMenuDelegate for MenuTarget {
        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, _menu: &NSMenu) {
            self.ivars().open.set(true);
        }

        #[unsafe(method(menuDidClose:))]
        fn menu_did_close(&self, _menu: &NSMenu) {
            self.ivars().open.set(false);
        }
    }

    impl MenuTarget {
        /// The action of each item. The tag of the item identifies it.
        #[unsafe(method(select:))]
        fn select(&self, sender: Option<&NSMenuItem>) {
            let command = sender.and_then(|item| {
                usize::try_from(item.tag())
                    .ok()
                    .and_then(|i| self.ivars().table.borrow().get(i).cloned().flatten())
            });
            if let Some(command) = command {
                let _ = self.ivars().commands.try_send(command);
            }
        }

        /// The action of the slider of the brightness row.
        #[unsafe(method(brightness:))]
        fn brightness(&self, sender: Option<&NSSlider>) {
            let Some(slider) = sender else {
                return;
            };
            let value = slider
                .doubleValue()
                .round()
                .clamp(0.0, f64::from(HIGHEST_BRIGHTNESS)) as u8;
            if let Some(label) = self.ivars().value.borrow().as_ref() {
                label.setStringValue(&NSString::from_str(&percent(value)));
            }
            let _ = self
                .ivars()
                .commands
                .try_send(ShellCommand::SetBrightness(value));
        }
    }
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker, commands: Sender<ShellCommand>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            commands,
            table: RefCell::new(Vec::new()),
            value: RefCell::new(None),
            open: Cell::new(false),
        });
        // SAFETY: `init` of `NSObject` is the designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

/// The text of the value of the brightness row, for example "80 %". The space
/// between the number and the sign is a no-break space.
fn percent(value: u8) -> String {
    format!("{value}\u{a0}%")
}

/// The custom view of the brightness row.
struct BrightnessRow {
    slider: Retained<NSSlider>,
    value: Retained<NSTextField>,
}

/// The menu bar item. Keep it until the process stops: the item goes away
/// from the menu bar when it is dropped.
pub struct MenuBar {
    _status_item: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    target: Retained<MenuTarget>,
    mtm: MainThreadMarker,
    brightness: Option<BrightnessRow>,
    shown: Vec<Row>,
}

fn menu_item(mtm: MainThreadMarker, title: &str, key: &str) -> Retained<NSMenuItem> {
    // SAFETY: the item has no action. `make_selectable` sets it.
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            None,
            &NSString::from_str(key),
        )
    }
}

fn symbol(name: &str) -> Option<Retained<NSImage>> {
    NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(name), None)
}

/// The title of the line with the connection state: a dot, and the text. The
/// dot is green if the device is connected, and grey if not. The colours are
/// the system colours, thus they follow the light and the dark appearance.
fn device_title(title: &str, connected: bool) -> Retained<NSMutableAttributedString> {
    let text = format!("●\u{2002}{title}");
    let all = NSRange::new(0, text.encode_utf16().count());
    let dot = NSRange::new(0, 1);
    let colour = if connected {
        NSColor::systemGreenColor()
    } else {
        NSColor::tertiaryLabelColor()
    };
    let font = NSFont::menuFontOfSize(0.0);
    let line = NSColor::secondaryLabelColor();
    let string = NSMutableAttributedString::initWithString(
        NSMutableAttributedString::alloc(),
        &NSString::from_str(&text),
    );
    // SAFETY: the values are an `NSFont` and `NSColor`, the types that these
    // attributes need. The ranges are inside the string.
    unsafe {
        string.addAttribute_value_range(NSFontAttributeName, &font, all);
        string.addAttribute_value_range(NSForegroundColorAttributeName, &line, all);
        string.addAttribute_value_range(NSForegroundColorAttributeName, &colour, dot);
    }
    string
}

fn frame(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

fn sun(mtm: MainThreadMarker, name: &str, x: f64, y: f64) -> Option<Retained<NSImageView>> {
    let image = symbol(name)?;
    let view = NSImageView::imageViewWithImage(&image, mtm);
    view.setFrame(frame(x, y, SUN_WIDTH, ROW_SLIDER_HEIGHT));
    view.setContentTintColor(Some(&NSColor::secondaryLabelColor()));
    Some(view)
}

impl MenuBar {
    /// Make the item. `None` if the caller is not on the main thread.
    pub fn new(commands: Sender<ShellCommand>) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let target = MenuTarget::new(mtm, commands);
        let menu = NSMenu::new(mtm);
        menu.setDelegate(Some(ProtocolObject::from_ref(&*target)));
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
        let mut bar = Self {
            _status_item: status_item,
            menu,
            target,
            mtm,
            brightness: None,
            shown: Vec::new(),
        };
        bar.show(&MenuState::default());
        Some(bar)
    }

    /// Show `state`. If only the value of the brightness changed, move the
    /// slider. Otherwise make the items again.
    pub fn show(&mut self, state: &MenuState) {
        let rows = state.rows();
        if rows == self.shown {
            return;
        }
        if !self.shown.is_empty() && same_shape(&rows, &self.shown) {
            // While the menu is open, the user moves the slider, and the
            // slider is the source of the value. A reply of the engine must
            // not move it back.
            if !self.target.ivars().open.get() {
                if let Some(brightness) = state.brightness {
                    self.set_brightness_value(brightness.value);
                }
            }
            self.shown = rows;
            return;
        }
        self.rebuild(&rows);
        self.shown = rows;
    }

    fn set_brightness_value(&self, value: u8) {
        if let Some(row) = &self.brightness {
            row.slider.setDoubleValue(f64::from(value));
            row.value
                .setStringValue(&NSString::from_str(&percent(value)));
        }
    }

    fn rebuild(&mut self, rows: &[Row]) {
        let mtm = self.mtm;
        self.menu.removeAllItems();
        self.brightness = None;
        *self.target.ivars().value.borrow_mut() = None;
        let mut table = Vec::new();
        for row in rows {
            let entry = match row {
                Row::Separator => NSMenuItem::separatorItem(mtm),
                Row::Section(title) => section_header(mtm, title),
                Row::Brightness(brightness) => {
                    let (item, made) = self.brightness_row(*brightness);
                    *self.target.ivars().value.borrow_mut() = Some(made.value.clone());
                    self.brightness = Some(made);
                    item
                }
                Row::Device { title, connected } => {
                    let entry = menu_item(mtm, title, "");
                    // `setImage` and `setOnStateImage` show no colour on a line that
                    // the user cannot select (docs/kb/gpui.md, "The menu with the
                    // brightness row"). The colour of a text run shows.
                    entry.setAttributedTitle(Some(&device_title(title, *connected)));
                    entry
                }
                Row::Pages { title, pages } => {
                    let entry = menu_item(mtm, title, "");
                    let submenu = NSMenu::new(mtm);
                    for page in pages {
                        let item = menu_item(mtm, &page.title, "");
                        self.make_selectable(&item, Some(page.command()), &mut table);
                        if page.checked {
                            item.setState(NSControlStateValueOn);
                        }
                        submenu.addItem(&item);
                    }
                    entry.setSubmenu(Some(&submenu));
                    entry
                }
                Row::Profile { checked, .. } => {
                    let entry = menu_item(mtm, row.title().unwrap_or_default(), "");
                    self.make_selectable(&entry, row.command(), &mut table);
                    if *checked {
                        entry.setState(NSControlStateValueOn);
                    }
                    entry
                }
                Row::Permission
                | Row::Open
                | Row::WakeDevice
                | Row::Settings
                | Row::About
                | Row::Quit => {
                    let entry = menu_item(mtm, row.title().unwrap_or_default(), row.key());
                    self.make_selectable(&entry, row.command(), &mut table);
                    entry
                }
            };
            self.menu.addItem(&entry);
        }
        *self.target.ivars().table.borrow_mut() = table;
    }

    /// Give `item` the action `select:`, and put `command` in the table.
    fn make_selectable(
        &self,
        item: &NSMenuItem,
        command: Option<ShellCommand>,
        table: &mut Vec<Option<ShellCommand>>,
    ) {
        item.setTag(table.len() as isize);
        table.push(command);
        // SAFETY: the action is `select:` of `MenuTarget`, and the target
        // stays alive as long as the menu.
        unsafe {
            item.setAction(Some(sel!(select:)));
            item.setTarget(Some(&self.target));
        }
    }

    /// The row with the slider: "Brightness" and the value on one line, and
    /// below it a small sun, the slider, and a large sun.
    fn brightness_row(&self, brightness: Brightness) -> (Retained<NSMenuItem>, BrightnessRow) {
        let mtm = self.mtm;
        let height = ROW_PAD_TOP + ROW_TITLE_HEIGHT + ROW_GAP + ROW_SLIDER_HEIGHT + ROW_PAD_BOTTOM;
        let view = NSView::initWithFrame(NSView::alloc(mtm), frame(0.0, 0.0, ROW_WIDTH, height));
        let inner = ROW_WIDTH - 2.0 * ROW_PAD_SIDE;
        let slider_y = ROW_PAD_BOTTOM;
        let title_y = slider_y + ROW_SLIDER_HEIGHT + ROW_GAP;
        // The menu can be wider than the row: the row follows its width. The
        // value and the large sun stay at the right edge.
        view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        let font = NSFont::menuFontOfSize(0.0);

        let title = NSTextField::labelWithString(&NSString::from_str("Brightness"), mtm);
        title.setFont(Some(&font));
        title.setTextColor(Some(&NSColor::labelColor()));
        title.setFrame(frame(ROW_PAD_SIDE, title_y, inner / 2.0, ROW_TITLE_HEIGHT));
        view.addSubview(&title);

        let value =
            NSTextField::labelWithString(&NSString::from_str(&percent(brightness.value)), mtm);
        value.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
            font.pointSize(),
            0.0,
        )));
        value.setTextColor(Some(&NSColor::secondaryLabelColor()));
        value.setAlignment(NSTextAlignment::Right);
        value.setFrame(frame(
            ROW_PAD_SIDE + inner / 2.0,
            title_y,
            inner / 2.0,
            ROW_TITLE_HEIGHT,
        ));
        value.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin);
        view.addSubview(&value);

        let slider_x = ROW_PAD_SIDE + SUN_WIDTH + SUN_GAP;
        let slider_width = inner - 2.0 * (SUN_WIDTH + SUN_GAP);
        // SAFETY: the action is `brightness:` of `MenuTarget`, and the
        // target stays alive as long as the menu.
        let slider = unsafe {
            NSSlider::sliderWithValue_minValue_maxValue_target_action(
                f64::from(brightness.value),
                f64::from(brightness.lowest),
                f64::from(HIGHEST_BRIGHTNESS),
                Some(&self.target),
                Some(sel!(brightness:)),
                mtm,
            )
        };
        slider.setContinuous(true);
        // VoiceOver reads this label, because the title is a different view.
        slider.setAccessibilityLabel(Some(&NSString::from_str("Brightness")));
        slider.setFrame(frame(slider_x, slider_y, slider_width, ROW_SLIDER_HEIGHT));
        slider.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        view.addSubview(&slider);
        if let Some(small) = sun(mtm, "sun.min", ROW_PAD_SIDE, slider_y) {
            view.addSubview(&small);
        }
        if let Some(large) = sun(
            mtm,
            "sun.max",
            ROW_WIDTH - ROW_PAD_SIDE - SUN_WIDTH,
            slider_y,
        ) {
            large.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin);
            view.addSubview(&large);
        }

        let item = menu_item(mtm, "Brightness", "");
        item.setView(Some(&view));
        (item, BrightnessRow { slider, value })
    }
}

/// A title for the rows below it. macOS 14 and later have the standard
/// section header. Before it, the title is a line that the user cannot select.
fn section_header(mtm: MainThreadMarker, title: &str) -> Retained<NSMenuItem> {
    // SAFETY: `respondsToSelector:` takes a selector and gives a boolean.
    let has_header: bool = unsafe {
        msg_send![NSMenuItem::class(), respondsToSelector: sel!(sectionHeaderWithTitle:)]
    };
    if has_header {
        NSMenuItem::sectionHeaderWithTitle(&NSString::from_str(title), mtm)
    } else {
        menu_item(mtm, title, "")
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

/// Open the file at `path` with the default application for its type, for
/// example TextEdit for a text file. False if macOS cannot open it.
pub fn open_file(path: &Path) -> bool {
    let Some(path) = path.to_str() else {
        return false;
    };
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    NSWorkspace::sharedWorkspace().openURL(&url)
}

/// Open `url` with the application for its scheme, for example the pane of
/// System Settings for an address that starts with `x-apple.systempreferences:`.
/// False if macOS cannot open it.
pub fn open_url(url: &str) -> bool {
    NSURL::URLWithString(&NSString::from_str(url))
        .is_some_and(|url| NSWorkspace::sharedWorkspace().openURL(&url))
}

/// Show `path` in the Finder: a file is selected in its folder, and a folder
/// opens. False if macOS cannot do it.
pub fn show_in_finder(path: &Path) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    let workspace = NSWorkspace::sharedWorkspace();
    if path.is_dir() {
        let url = NSURL::fileURLWithPath(&NSString::from_str(text));
        workspace.openURL(&url)
    } else {
        workspace.selectFile_inFileViewerRootedAtPath(
            Some(&NSString::from_str(text)),
            &NSString::from_str(""),
        )
    }
}
