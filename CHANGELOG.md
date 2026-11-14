# Changelog

This file lists the changes for each version. Its form follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-11-14

This version changes the profile when its application comes to the front,
and a key can open a folder of keys. Version 0.3.0 was not published.

### Added

- The device shows the profile that lists the front application. When no
  profile lists that application, the device shows the default profile.
  The setting is on until you turn it off. Settings and the menu bar item
  can turn it off.
- A profile can list one or more applications. You pick each one from the
  applications on the Mac.
- Select a profile by its identity, so a new name does not break the key.
  The key can also select the next or the previous profile, and it can
  open the profile at a page.
- Open an application from a list of the installed applications.
- Folders of keys. **Open a folder** shows the folder, and **Back** returns
  to the page. **Next page** does not open a folder. A new folder has a
  **Back** key on key 1.
- Ready profiles for Safari, Finder, Zoom, Keynote, Xcode, and Final Cut
  Pro. Each profile is linked to its application. You add one from the
  first start window or from the **+** menu of **PROFILES**.

### Changed

- OpenCraft writes profile schema 5. It reads a profile from version 0.2.0
  and changes it to schema 5. Version 0.2.0 cannot read a profile that
  version 0.4.0 has saved.

### Known limits

- Only the N4 Pro is supported.
- Only Mac computers with Apple silicon and macOS 12 or later. The project
  tested OpenCraft on macOS 27 only.
- Send a hotkey, Type text, Media key, Mouse event, and Mac system
  function need the Accessibility permission.
- The Sleep function does nothing when macOS does not allow sleep.
  OpenCraft then names the cause.
- OpenCraft has no plug-ins.

## [0.2.0] - 2026-11-14

This version has the new window, and the actions for daily work on a Mac.

### Added

- A window with three panes: the sidebar, the device picture, and the
  inspector. The library is the second tab of the right pane.
- A first start. The user chooses a starter profile: Everyday Mac, Media
  and sound, Video calls, or Empty.
- Undo and redo. Copy, paste, and clear of a control.
- A command palette (Command-K). It finds actions, controls, pages,
  profiles, and commands.
- An icon on a key face or a bar panel, from a built-in set or from an
  image file.
- Label options: the position, the size, and bold text.
- A background image or a colour for a page.
- Drag an action from the library onto a control. Test an action from the
  inspector.
- Profile settings, with export and import. Import is also in the sidebar
  and in the command palette.
- A Settings window: the Dock icon, the start at login, the shortcuts, and
  the licence notices.
- The menu bar item changes the profile, the page, and the brightness.
- The keyboard operates the window. A focus ring shows the control with
  the focus. Each control of the window has an accessible name.
- Screen off, a screen-off timer, and an action for the brightness of the
  device. The inspector shows the firmware of the connected device.
- Fast forward and rewind in the media key action.
- Type text can paste from the clipboard, and it can press Return at the
  end.
- Open a file or a folder. Give arguments to an application. Open a URL in
  a chosen browser. Quit an application.
- The key of a page action shows the page number.
- Mute the microphone. Switch the sound output. Play a sound, and stop
  all sound.
- Mac system functions: Mission Control, Launchpad, Screenshot, Screen
  saver, Sleep, Lock, and Show desktop.
- A mouse event: click, move, and scroll. A knob can scroll.
- Each step of a sequence has its own delay.
- Copy diagnostics from the About pane. The text has no serial number.
  The About pane also opens the help pages.

### Changed

- At a screen lock the device goes dark. A key press wakes it. The first
  press starts no action.
- After a lock, the window does not keep an old picture of the device.
- After login, the window shows the current state of the Accessibility
  permission.

### Known limits

- Only the N4 Pro is supported.
- Only Mac computers with Apple silicon and macOS 12 or later. The project
  tested OpenCraft on macOS 27 only.
- Send a hotkey, Type text, Media key, Mouse event, and Mac system
  function need the Accessibility permission.
- The Sleep function does nothing when macOS does not allow sleep.
  OpenCraft then names the cause.
- OpenCraft has no plug-ins.

## [0.1.0] - 2026-11-12

This is the first version. It operates the VSD StreamDock N4 Pro on macOS,
with no vendor software.

### Added

- A device layer for the N4 Pro: key faces, bar panels, the background
  image, brightness, and decoded input from keys, knobs, and the touch bar.
  It connects again after a disconnection.
- `opencraft-probe`, a command line tool to find the device and to test it.
- A window with the layout of the device. A pressed key shows as pressed.
  You set the label, the colour, and the image of a key face or of a bar
  panel. OpenCraft saves the profile and shows it again after a restart.
- Actions for the gestures of keys and knobs: open an application, open a
  URL, send a hotkey, type text, media keys, sound volume, and run a shell
  command. A knob can change the sound volume and show the level on a bar
  panel.
- Pages, with an action to change the page.
- Swipe and tap gestures on the touch bar.
- A toggle action that shows its state on the key face, and a sequence
  action that runs other actions in order.
- Many profiles, and an action to select a profile.
- A menu bar item, an option to hide the Dock icon, and an option to start
  OpenCraft at login. The device continues to operate when you close the
  window.
- Recovery after the computer sleeps and wakes.
- Device settings: display brightness, haptic feedback, and knob lights.
- A defined state when OpenCraft stops: the device goes dark.
- A log file, `opencraft.log`, when the process has no terminal.
- A second start of OpenCraft opens the window of the OpenCraft that runs.
- Messages in the window that tell the user what to do, with no words for
  developers.
- A signed and notarized disk image for Mac computers with Apple silicon.
- A Homebrew cask in the tap `shantanugoel/opencraft`.
- The menu bar item shows the version of OpenCraft, and opens the licence
  notices of the dependencies.
- A user guide in `docs/user/`.

### Known limits

- Only the N4 Pro is supported.
- Only Mac computers with Apple silicon and macOS 12 or later. The project
  tested OpenCraft on macOS 27 only.
- "Send a hotkey", "Type text", and "Media key" need the Accessibility
  permission.
- OpenCraft has no plug-ins.
