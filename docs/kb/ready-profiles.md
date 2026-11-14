# Ready profiles for Mac applications

Last update: 2026-11-14

Issue #340 adds six profiles. Each profile links to one Mac application.
The profile files are in the executable (`crates/app/assets/ready`). The
schema version stays 5. A profile keeps a link only when no profile of the
model already has that link
([decision 0035](../decisions/0035-front-application-selects-the-profile.md),
section 5). The engine does this in `drop_clashing_links`.

The shortcuts below are the official shortcuts of each application on the
Mac. The profile does not use the Windows shortcuts of the vendor. The
icons are names in the Lucide set of the engine. The profile has no image
file.

## Bundle identifiers

| Application | Bundle identifier | Display name in the link | Confidence | Source |
|---|---|---|---|---|
| Safari | `com.apple.Safari` | Safari | Measured | `mdls -name kMDItemCFBundleIdentifier` on `/Applications/Safari.app` |
| Finder | `com.apple.finder` | Finder | Measured | `mdls -name kMDItemCFBundleIdentifier` on `/System/Library/CoreServices/Finder.app` |
| zoom.us | `us.zoom.xos` | zoom.us | Measured | `mdls -name kMDItemCFBundleIdentifier` on `/Applications/zoom.us.app` |
| Xcode | `com.apple.dt.Xcode` | Xcode | Measured | `mdls -name kMDItemCFBundleIdentifier` on `/Applications/Xcode.app` |
| Keynote | `com.apple.iWork.Keynote` | Keynote | Source | The ready-profile plan. Keynote was not installed, so `mdls` was not run. |
| Final Cut Pro | `com.apple.FinalCut` | Final Cut Pro | Source | The ready-profile plan. Final Cut Pro was not installed, so `mdls` was not run. |

A Keynote build from Creator Studio on the Mac uses `com.apple.Keynote`
(Source: Daring Fireball, 2026-04-14). The ready profile links only
`com.apple.iWork.Keynote`. Add `com.apple.Keynote` in the profile sheet if
the front application is that build.

## Hotkey text

The profile file stores a shortcut as `hotkey = "shift+cmd+a"`. The parser
in `crates/actions/src/hotkey.rs` accepts `space`, `delete`, `left`,
`right`, `up`, `down`, `minus`, `equal`, `leftbracket`, `rightbracket`,
`period`, and `slash`. `ctrl` is Control. `alt` is Option. `cmd` is
Command.

| Written shortcut | Hotkey text | Confidence | Source |
|---|---|---|---|
| Command-Plus | `shift+cmd+equal` | Inference | On an ANSI keyboard, Plus is Shift-Equal. The sender maps Equal to `0x18` and Minus to `0x1B` (`crates/actions/src/system/macos.rs`). |
| Command-Minus | `cmd+minus` | Inference | The same key table. Minus is the key, with no Shift. |
| Control-Equal Sign | `ctrl+equal` | Inference | Final Cut Pro names the equal key. Shift is not part of that shortcut. |
| Control-Hyphen | `ctrl+minus` | Inference | The hyphen key is Minus. |
| Option | `alt` | Source | `video-calls.toml` already stores Option as `alt`. |
| Command-Period | `cmd+period` | Inference | The parser name of Period is `period`. |
| Shift-Question Mark | `shift+slash` | Inference | Question Mark is Shift-Slash on an ANSI keyboard. |

## Safari

Source: Apple, "Keyboard shortcuts and gestures in Safari on Mac".

| Key | Shortcut | Confidence |
|---|---|---|
| Close | Command-W | Source |
| Reopen | Shift-Command-T | Source |
| Back | Command-[ | Source |
| Forward | Command-] | Source |
| Address | Command-L | Source |
| Find | Command-F | Source |
| Reader | Shift-Command-R | Source |
| Next tab | Shift-Command-] | Source |
| Previous tab | Shift-Command-[ | Source |
| Zoom in | Command-Plus | Source, stored as `shift+cmd+equal` |
| Zoom out | Command-Minus | Source, stored as `cmd+minus` |
| New tab | Command-T | Inference. The Safari menu has the item "New Tab". The fetched shortcut tables do not list the key. |
| Reload | Command-R | Source. The Finder shortcut page names Safari for Command-R. |

## Finder

Source: Apple, "Mac keyboard shortcuts".

| Key | Shortcut | Confidence |
|---|---|---|
| New window | Command-N | Source |
| New folder | Shift-Command-N | Source |
| Get Info | Command-I | Source |
| Quick Look | Command-Y | Source |
| Duplicate | Command-D | Source |
| Move to Trash | Command-Delete | Source |
| Previous folder | Command-[ | Source |
| Enclosing folder | Command-Up Arrow | Source |
| Downloads | Option-Command-L | Source |
| Home | Shift-Command-H | Source |
| Icon view | Command-1 | Source |
| List view | Command-2 | Source |
| Column view | Command-3 | Source |
| Gallery view | Command-4 | Source |

The knob is the sound volume of the Mac. Finder has no official volume knob.

## Zoom

Source: Zoom, "Hot keys and keyboard shortcuts". The macOS column.

| Key | Shortcut | Confidence |
|---|---|---|
| Mute | Command-Shift-A | Source |
| Video | Command-Shift-V | Source |
| Share | Command-Shift-S | Source |
| Participants | Command-U | Source |
| Chat | Command-Shift-H | Source |
| Raise hand | Option-Y | Source |
| Local record | Command-Shift-R | Source |
| Full screen | Command-Shift-F | Source |
| Speaker, gallery, or multi-speaker | Command-Shift-W | Source. The key title is "Views". |
| Close | Command-W | Source. Zoom names this "Close current window". |

The knob is the sound volume of the Mac. Zoom has no official volume knob.

## Keynote

Source: Apple, "Keynote keyboard shortcuts on Mac", section "Play a slideshow".

| Key | Shortcut | Confidence |
|---|---|---|
| Play | Option-Command-P | Source |
| Next slide or build | Right Arrow, and Down Arrow | Source. The key and the knob turn right send Right Arrow. |
| Previous slide or build | Left Arrow, and Up Arrow | Source. The key and the knob turn left send Left Arrow. |
| Black screen | B | Source |
| White screen | W | Source |
| Presenter notes | Shift-Command-P | Source |
| New slide | Shift-Command-N | Source |
| Skip slide | Shift-Command-H | Source |
| Undo | Command-Z | Source |
| Stop the presentation | Escape, and Q | Source. The key sends Escape. |

## Xcode

Source: Apple, "Xcode menu commands and keyboard shortcuts".

| Key | Shortcut | Confidence |
|---|---|---|
| Run | Command-R | Source |
| Test | Command-T | Source |
| Stop | Command-Period | Source. The Product menu shows ⌘. |
| Build For Running | Shift-Command-R | Source. The key title is "Build". |
| Clean | Shift-Command-K | Source |
| Debug area | Shift-Command-Y | Source |
| Find | Command-F | Source |
| Find in Workspace | Shift-Command-F | Source |
| Profile | Command-I | Source |
| Project navigator | Command-1 | Source |
| Symbol navigator | Command-2 | Source |
| Find navigator | Command-3 | Source |
| Issue navigator | Command-4 | Source |
| Debug navigator | Command-5 | Source |
| Breakpoint navigator | Command-6 | Source |
| Report navigator | Command-7 | Source |

The knob is the sound volume of the Mac. Xcode has no official knob for a build.

## Final Cut Pro

Source: Apple, "Final Cut Pro keyboard shortcuts".

| Key | Shortcut | Confidence |
|---|---|---|
| Play / Pause | Space | Source |
| Play Forward | L | Source |
| Play Reverse | J | Source |
| Play Full Screen | Shift-Command-F | Source |
| Loop Playback | Command-L | Source |
| Play Selection | Slash | Source |
| Play Around | Shift-? | Source, stored as `shift+slash` |
| Play from Playhead | Option-Space | Source |
| Play Rate .5 | Option-L | Source |
| Blade | Command-B | Source |
| Blade All | Shift-Command-B | Source |
| Trim Start | Option-[ | Source |
| Trim End | Option-] | Source |
| Add Marker | M | Source |
| Undo | Command-Z | Source |
| Raise Volume 1 dB | Control-Equal | Source, stored as `ctrl+equal` |
| Lower Volume 1 dB | Control-Hyphen | Source, stored as `ctrl+minus` |

Control-L is "Adjust Volume Relative". The knob does not use it.

## Not known

- The bundle identifier of Keynote from Creator Studio, on a Mac that has
  that application. The published identifier is `com.apple.Keynote`. This
  computer did not have Keynote.
- The bundle identifier of a Final Cut Pro build from Creator Studio, if
  that build uses an identifier other than `com.apple.FinalCut`.
- The key equivalent of Safari "New Tab" in the menu nib. `ibtool` did not
  write the key equivalent. The profile uses Command-T.

## Sources

- Apple, "Keyboard shortcuts and gestures in Safari on Mac": https://support.apple.com/guide/safari/keyboard-shortcuts-and-gestures-cpsh003/mac
- Apple, "Mac keyboard shortcuts": https://support.apple.com/en-us/102650
- Zoom, "Hot keys and keyboard shortcuts": https://support.zoom.com/hc/en/article?id=zm_kb&sysparm_article=KB0067050
- Apple, "Keynote keyboard shortcuts on Mac": https://support.apple.com/guide/keynote/keyboard-shortcuts-tanfde4a3e6d/mac
- Apple, "Xcode menu commands and keyboard shortcuts": https://developer.apple.com/library/archive/documentation/IDEs/Conceptual/xcode_help-command_shortcuts/MenuCommands/MenuCommands014.html
- Apple, "Final Cut Pro keyboard shortcuts": https://support.apple.com/guide/final-cut-pro/keyboard-shortcuts-ver90ba5929/mac
- Daring Fireball, "Pages, Numbers, and Keynote: Old Versions, Mac App Store", 2026-04-14: https://daringfireball.net/linked/2026/04/14/pages-numbers-keynote-old-versions-mac-app-store
- The hotkey parser: `crates/actions/src/hotkey.rs`
- The key codes of the sender: `crates/actions/src/system/macos.rs`
