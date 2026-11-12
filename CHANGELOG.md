# Changelog

This file lists the changes for each version. Its form follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Not released

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

### Known limits

- Only the N4 Pro is supported.
- The application is not signed or notarized. There is no installer.
- A key event, a hotkey, or a text action needs the Accessibility permission.
