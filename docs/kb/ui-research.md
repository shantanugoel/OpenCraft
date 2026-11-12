# Research for the layout of the main window

Last update: 2026-11-12

This entry holds the facts from other software and from the Apple Human
Interface Guidelines that the layout of the main window uses. The decision
that uses them is [decision 0029](../decisions/0029-window-layout.md). The
facts about the vendor software are in [vsd-craft.md](vsd-craft.md). The
facts about other open-source software for this device family are in
[prior-art.md](prior-art.md).

We did not operate the software below. Each fact comes from a document of
the vendor or from a report about the software.

## Elgato Stream Deck 7

| Fact | Confidence | Source |
|---|---|---|
| The user drags an action from the action list on the right side of the window onto a key | Source | Elgato, "How to set up Stream Deck hotkeys" and "How to use Multi Actions" |
| A right-click on an empty key on the canvas also adds an action, for example a Multi Action | Source | Elgato, "How to use Multi Actions" |
| "Key Logic" is an action that holds up to three actions on one key: one for a press, one for a double press, and one for a press and hold | Source | Elgato, Stream Deck 7.0 release notes; Elgato, "Key Logic" article |
| The user adds Key Logic with a right-click on a key ("Create Key Logic Action"), or drags it from the category "Multi Action" | Source | Elgato, "Key Logic" article |
| The sources do not agree on the version that added Key Logic: the 7.0 notes list it, and the 7.0.2 notes also say that they add it | Source | Elgato release notes 7.0 and 7.0.2 |
| A user report says that a single press on a Key Logic key runs at the release, so it comes a little later | Source | Keyboard Maestro forum, "Stream Deck 7.0.3". Elgato does not confirm it. |
| Version 7 adds a "Virtual Stream Deck": keys on the screen that the user pins in place or opens at the pointer | Source | Matthew Cassinelli, "Stream Deck 7 adds virtual decks, Key Logic, weather, and app status" |
| The action "Open Application" shows a green dot when the application runs ("App Status") | Source | Matthew Cassinelli, same article |
| A "Smart Profile" changes the layout of the device when a specified application is open | Source | Elgato, "How to set up Stream Deck hotkeys" |

## Loupedeck and Logi Options+

| Fact | Confidence | Source |
|---|---|---|
| Loupedeck Software 4.3 gave a preview of a new user interface and a simplified profile for Loupedeck Live. Version 5.0 made the new interface the standard. | Source | Loupedeck blog, "Loupedeck software update unveils a new UI"; Loupedeck release notes 5.0 |
| In the new interface, the "Action Panel" is on the right. It shows an icon for each plug-in, and a filter tab to find actions. | Source | ProVideo Coalition, "Loupedeck introduces new user interface" |
| The user assigns actions from the Action Panel to the pages of the touch display | Source | Videomaker, "Here are the 3 main features of the new Loupedeck Live UI" |
| The aim of the change was an easier first experience for new users. The CEO said that streamers must get to their actions "without the need for a complicated set-up process". | Source | ProVideo Coalition, same article; Videomaker, same article |
| The old interface stayed available as "Classic UI", for more detailed configuration | Source | ProVideo Coalition; Videomaker |
| Issue #173 says that Loupedeck changed the layout because users said that the old setup was difficult. We found no source that says this. The sources give only the aim above. | Inference | The sources above |
| In Logi Options+ (MX Creative Console), the user finds an action in the action list with a search, and drags it onto a key. A configuration window then opens. | Source | Logitech support, "Plugins – MX Creative Console" |
| Logi Options+ has up to 15 pages for each profile of the MX Creative Keypad. The user can cut and paste actions across pages. | Source | Logitech MX Creative Console product page; Logi Options+ update note |

## Bitfocus Companion

| Fact | Confidence | Source |
|---|---|---|
| Each button has groups of actions. The "Press" actions run when the button is pushed or triggered. The "Release" actions run when the button is released. | Source | Companion user guide 5.0, "Actions" |
| The user can add timed groups to a button for a long press. Each group has a time, and runs at the release or while the button is held. | Source | Companion user guide 5.0, "Actions" |
| With a timed group, the "Release" actions become "Short release" actions. They run only if the release comes before the time of the first group. | Source | Companion user guide 5.0, "Actions", as a web search quoted it. The page that we read did not show this sentence. |
| Steps, feedbacks, and the actions of a button are in the one button editor. The guide has a page for each of them below "Buttons". | Source | Companion user guide 5.0, navigation |
| A trigger can use each action that a button can use. Companion 3.0 changed the trigger editor to look like the button editor. | Source | Companion user guide, "Triggers"; Companion 3.0 release notes |
| Companion supports the Mirabox N4. See [prior-art.md](prior-art.md#projects). | Source | prior-art.md |

## Apple Human Interface Guidelines

| Fact | Confidence | Source |
|---|---|---|
| A sidebar is on the leading side of a view. It lets people go between areas of the app or top-level collections. | Source | HIG, "Sidebars" |
| "In general, show no more than two levels of hierarchy in a sidebar." For a deeper hierarchy, use a split view with a content list. | Source | HIG, "Sidebars" |
| In macOS, an app can have a show and hide button for the sidebar, or the commands "Show Sidebar" and "Hide Sidebar" in the View menu. Do not hide the sidebar by default. | Source | HIG, "Sidebars" |
| Do not put critical information or actions at the bottom of a sidebar, because a window can be moved so that its bottom edge is hidden | Source | HIG, "Sidebars", macOS |
| Use disclosure controls to group a large hierarchy in a sidebar | Source | HIG, "Sidebars" |
| The guide shows a split view with three areas: a sidebar, a canvas, and an inspector. Keynote puts the inspector in a pane next to the slide canvas. | Source | HIG, "Split views" |
| Highlight the current selection in each pane at all times. Let people drag and drop content between panes. | Source | HIG, "Split views" |
| Set reasonable minimum and maximum pane sizes, so that the divider stays visible. Prefer the thin divider of 1 point. | Source | HIG, "Split views", macOS |
| Give more than one way to show a hidden pane, for example a toolbar button and a menu command with a key command | Source | HIG, "Split views", macOS |
| On macOS, the inspector shows as a trailing column, not as a sheet | Source | WWDC 2023, session 10161 |
| Option-Command-I is the standard key command to show an inspector window. Command-Comma opens the settings window. Command-Z is Undo, and Shift-Command-Z is Redo. | Source | HIG, "Keyboards", standard keyboard shortcuts |
| The standard key commands do not include Command-K and Control-Command-S. Finder uses Control-Command-S for its sidebar. | Source (Command-K); Inference (Finder, from use of macOS) | HIG, "Keyboards" |
| Each show and hide item of the View menu has a title that agrees with the current state: "Show Toolbar" when the toolbar is hidden, "Hide Toolbar" when it is visible | Source | HIG, "The menu bar", View menu |
| Undo and Redo name their target, for example "Undo Paste and Match Style", or "Undo Typing" for text | Source | HIG, "The menu bar", Edit menu |
| The settings window opens from the item "Settings…" in the app menu. Do not put a settings button in the toolbar of a window. | Source | HIG, "Settings", macOS |
| A settings window has a toolbar that is not customizable, with one button for each pane. The window title is the name of the visible pane. The minimize and zoom buttons are dimmed. The window opens at the pane that was visible last. | Source | HIG, "Settings", macOS |
| Options that apply to one task belong in the view of that task, not in the settings window | Source | HIG, "Settings" |
| A menu bar extra shows a menu, not a popover, unless the function is too complex for a menu. Its icon is a symbol or a template image. The menu bar is 24 points high. | Source | HIG, "The menu bar", menu bar extras |
| The system can hide a menu bar extra. Give its functions also in a different place. | Source | HIG, "The menu bar", menu bar extras |

## The icon set in the GUI toolkit

| Fact | Confidence | Source |
|---|---|---|
| `gpui-kit-assets` 0.9.1 (licence Apache-2.0) contains all 1,818 icons of Lucide 1.43.0 and 12 other icons, as the enum `gpui_kit::assets::IconName`. The Lucide licence file is `LICENSE-LUCIDE` in the crate. | Source | README of the published crate `gpui-kit-assets` 0.9.1 |
| The default `Assets` source embeds only 101 icons. The macro `icon_assets!` embeds only the icons that it names. `AllAssets` embeds all 1,830 icons: about 731 KiB of SVG data, and 1.02 MiB more in the binary. | Source | README of `gpui-kit-assets` 0.9.1. The crate measured it on Linux x86_64. |
| The wireframes of the overhaul draw line icons on a grid of 24, with a stroke of 1.6 to 2.0. This is the style of Lucide. | Inference | The wireframes of issue #173 |

## Not known

- Whether GPUI gives each part of the layout: a divider that changes the
  width of a pane, a popover attached to a button, a modal layer with focus,
  and drag and drop between panes. Issue
  [#175](https://github.com/shantanugoel/OpenCraft/issues/175) tests this and
  writes the result in [gpui.md](gpui.md).
- Whether Elgato or Loupedeck measured the use of their layouts with users.
  The sources give no numbers.

## Sources

- Elgato, Stream Deck 7.0 release notes:
  <https://help.elgato.com/hc/en-us/articles/38011940576273-Elgato-Stream-Deck-7-0-Release-Notes>
  and <https://edge.elgato.com/egc/sd_content/releasenotes/7.0.0.html>
- Elgato, Stream Deck 7.0.2 release notes:
  <https://edge.elgato.com/egc/sd_content/releasenotes/7.0.2.html>
- Elgato, "Key Logic":
  <https://www.elgato.com/us/zh/explorer/products/stream-deck/key-logic-stream-deck/>
- Elgato, "How to use Multi Actions":
  <https://www.elgato.com/explorer/products/stream-deck/how-to-use-multi-actions/>
- Elgato, "How to set up Stream Deck hotkeys":
  <https://www.elgato.com/explorer/products/stream-deck/how-to-set-up-stream-deck-hotkeys/>
- Matthew Cassinelli, "Stream Deck 7 adds virtual decks, Key Logic, weather,
  and app status":
  <https://matthewcassinelli.com/stream-deck-7-virtual-deck-key-logic-weather-app-status/>
- Keyboard Maestro forum, "Stream Deck 7.0.3":
  <https://forum.keyboardmaestro.com/t/stream-deck-7-0-3/50434>
- Loupedeck blog, "Loupedeck software update unveils a new UI":
  <https://loupedeck.com/blog/loupedeck-software-update-unveils-a-new-ui-phillips-hue-plugin-and-much-more/>
- Loupedeck release notes 5.0: <https://support.loupedeck.com/release-notes-5.0>
- ProVideo Coalition, "Loupedeck introduces new user interface":
  <https://www.provideocoalition.com/loupedeck-introduces-new-user-interface/>
- Videomaker, "Here are the 3 main features of the new Loupedeck Live UI":
  <https://www.videomaker.com/news/here-are-the-3-main-features-of-the-new-loupedeck-live-ui/>
- Logitech support, "Plugins – MX Creative Console":
  <https://support.logi.com/hc/ko/articles/25438961774743-Plugins-MX-Creative-Console-MX-Keypad>
- Logitech, MX Creative Console:
  <https://futureisnow.logitech.com/en-hk/products/keyboards/mx-creative-console.html>
- Logi Options+ update note, cut and paste across pages:
  <https://devicebase.net/en/logitech-logi-options/updates/you-can-now-cut-and-paste-actions-across-pages-on-the-mx-creative-keypad-configuration-page/5nb>
- Bitfocus Companion user guide 5.0, "Actions":
  <https://companion.free/user-guide/v5.0/config/buttons/creating/actions>
- Bitfocus Companion user guide, "Triggers":
  <https://companion.free/user-guide/config/triggers>
- Bitfocus Companion 3.0 release notes:
  <https://newreleases.io/project/github/bitfocus/companion/release/v3.0.0-rc1>
- Apple HIG, "Sidebars":
  <https://developer.apple.com/design/human-interface-guidelines/sidebars>
- Apple HIG, "Split views":
  <https://developer.apple.com/design/human-interface-guidelines/split-views>
- Apple HIG, "The menu bar":
  <https://developer.apple.com/design/human-interface-guidelines/the-menu-bar>
- Apple HIG, "Settings":
  <https://developer.apple.com/design/human-interface-guidelines/settings>
- Apple HIG, "Keyboards":
  <https://developer.apple.com/design/human-interface-guidelines/keyboards>
- Apple, WWDC 2023 session 10161, "Inspectors in SwiftUI":
  <https://developer.apple.com/videos/play/wwdc2023/10161/>
- `gpui-kit-assets` 0.9.1: <https://crates.io/crates/gpui-kit-assets> and
  <https://github.com/longbridge/gpui-kit>
