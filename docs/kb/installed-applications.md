# Installed applications

Last update: 2026-11-14

The shell builds this list for the application picker (issue 336). The
scan is in `crates/app/src/applications.rs`. It does not run an action.
The action "Open an application" still stores a name or a path. See
[macos-open-and-quit.md](macos-open-and-quit.md).

## Where the scan looks

| Fact | Confidence | Source |
|---|---|---|
| The scan reads one level of `*.app` bundles in `/Applications`, `/Applications/Utilities`, `/System/Applications`, `/System/Applications/Utilities`, and `Applications` in the home folder when `HOME` is set. | Measured | `standard_directories` and the scratch-directory test |
| A bundle inside another folder is not in the list. The file dialog can still select it. | Measured | The scratch-directory test: `Inner.app` under `Nested` is absent |
| A path that is not a directory, and a file that does not end with `.app`, is not an application. | Measured | The same test |
| The same path in two folders appears once. The list is sorted by the name, then by the path. The match ignores the case of letters. | Measured | The same test |

## The name and the bundle identifier

| Fact | Confidence | Source |
|---|---|---|
| The name is `CFBundleDisplayName` when that string is not empty. Else it is `CFBundleName`. Else it is the file name without `.app`. | Measured | The scratch-directory test, with an XML property list |
| The bundle identifier is `CFBundleIdentifier`. It is empty when the key is missing. | Measured | The same test |
| An XML property list can use the entities `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, and a numeric character reference. The scan decodes them. | Measured | The scratch-directory test: `Mail &amp; News` becomes `Mail & News` |
| A binary property list (`bplist`) gives the same three strings on macOS. `NSMutableDictionary.dictionaryWithContentsOfFile` reads it. The method on `NSDictionary` is deprecated. The method on `NSMutableDictionary` is not. | Measured | A scratch bundle converted with `/usr/bin/plutil -convert binary1`; the objc2 0.3.2 bindings |
| On a system that is not macOS, a binary property list adds no name and no bundle identifier. The file name is the name. | Inference | The binary reader is compiled for macOS only. Linux CI runs the XML test. |

## Icons

| Fact | Confidence | Source |
|---|---|---|
| The scan does not load icons. The window asks for an icon when a row needs it, and it keeps the bitmap for the process. | Source | `applications.rs` and `view/application_picker.rs` |
| `NSWorkspace.iconForFile` returns an `NSImage` for a path. The window draws it on the user interface thread with `lockFocus` and `NSBitmapImageRep.initWithFocusedViewRect`. That initializer is deprecated. Its replacement snapshots a view, and an icon is not a view. | Source | AppKit documentation; the objc2 0.3.2 bindings, which mark `initWithFocusedViewRect` deprecated |
| The window draws eight icons, then waits 1 ms, so the list can paint. A missing path gets no AppKit call. | Source | `view/application_picker.rs` |
| The field and the row show the icon at 16 points (`ICON_APPLICATION`). The request to AppKit is 32 pixels. The cache keeps the bitmap that AppKit returns. | Source | `ICON_PIXELS` and `theme::ICON_APPLICATION` |
| One run of the usual folders found 121 applications in 45 ms. The first icon took 1 ms. | Measured | A development run on 2026-11-14, with the window on a display at 1× |
| That icon bitmap is 32 by 32. It is not planar. It has 4 samples, 16 bits per sample, and 64 bits per pixel. A row is 256 bytes. The only format flag is `FloatingPointSamples`. | Measured | The same run, from `NSBitmapImageRep` after `lockFocus` |
| Each sample is a little-endian 16-bit float in the order red, green, blue, alpha. The first row is the top of the icon. | Measured | The same bitmap. The window showed the icons of Activity Monitor, App Store, and Safari with the right colours and the right orientation |
| The reader divides a colour by its alpha. The flag `AlphaNonpremultiplied` was not set. | Inference | The format flags of that bitmap, and Apple's default for a focused image |

## Sources

- Apple documentation for `CFBundleDisplayName`, `CFBundleName`, and `CFBundleIdentifier`.
- Apple documentation for `NSWorkspace` and `NSBitmapImageRep`.
- objc2 0.3.2 generated bindings in the Cargo registry.
