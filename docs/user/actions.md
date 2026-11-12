# Actions

An action is the work that OpenCraft does when you operate a control. This
chapter gives each action kind and its settings. The procedure to set an
action is in [Configure a key](first-key.md#4-set-an-action).

The menu of the action kinds in the window has this order:

| Action kind | Needs the Accessibility permission |
|---|---|
| [Open an application](#open-an-application) | No |
| [Open a URL](#open-a-url) | No |
| [Run a shell command](#run-a-shell-command) | No |
| [Send a hotkey](#send-a-hotkey) | Yes |
| [Type text](#type-text) | Yes |
| [Media key](#media-key) | Yes |
| [Sound volume](#sound-volume) | No |
| [Change the page](#change-the-page) | No |
| [Select a profile](#select-a-profile) | No |
| [Toggle](#toggle) | Only for the actions that it contains |
| [Sequence of actions](#sequence-of-actions) | Only for the actions that it contains |

The permission is in
[Install and first start](install.md#give-the-accessibility-permission).

## The settings of an action

The open card of a gesture shows the settings of its action, one setting on
each row. The control of a setting agrees with its type:

| Type of setting | Control |
|---|---|
| Text | A text field. Some text fields have three lines or more. |
| Number | A small text field for digits. A number out of the range changes to the nearest permitted number. |
| Selection | A button with the selected value. Click it, and click a value in the menu. |
| On or off | A switch |
| Hotkey | A field that records a hotkey. See [Send a hotkey](#send-a-hotkey). |
| File or application | A text field and **Choose…** |
| Action | A card in the card, with its own action kind and settings. See [Toggle](#toggle). |
| List of actions | A list of cards. See [Sequence of actions](#sequence-of-actions). |

If a setting has a long name, the name is above the control.

## An action that is not available

A profile can contain an action that this version of OpenCraft does not
have, for example from a newer version. Then the button of the card shows
**Not available**. The card shows the identifier of the action kind, and the
text "This version of OpenCraft does not have this action. The binding
stays in the profile."

- OpenCraft keeps the action and its settings in the profile.
- To use a different action, click **Not available**, and click an action
  kind. To remove the action, click **Remove the action**.

## How OpenCraft saves a setting

- OpenCraft saves each change immediately. The window has no "Save" button
  for an action.
- OpenCraft does not check a setting when you type it. It checks the
  setting when you operate the control. If OpenCraft cannot use a setting,
  the window shows a message above the status bar. See
  [Messages in the window](faults.md#messages-in-the-window).

## Open an application

| Setting | What to give |
|---|---|
| Application | The name of an application, for example `Safari`, or the path of an application |

1. Type the name of the application. Or click **Choose…** and select the
   application in the file dialog.

If the application runs, macOS brings it to the front.

## Open a URL

| Setting | What to give |
|---|---|
| URL | A full URL, for example `https://example.com` |

macOS opens the URL in the default application for that type of URL.

## Run a shell command

| Setting | What to give |
|---|---|
| Command | One or more lines for the shell `/bin/sh` |

- OpenCraft does not show the output of the command.
- OpenCraft does not wait for the command. You can operate a different
  control while the command runs.
- If the command ends with an error code, the window shows the message
  "The shell command ends with code N."
- OpenCraft can have a short search path for programs, for example after a
  start from the Finder or at login. Thus give the full path of a program
  that is not in `/usr/bin` or `/bin`.

## Send a hotkey

| Setting | What to give |
|---|---|
| Hotkey | One key, with or without the modifier keys Control, Option, Shift, and Command |

1. Click the field **Record hotkey…**. The field gets a blue frame and shows
   "Press a key…".
2. Push the key combination on the keyboard of the computer, for example
   Shift-Command-4.
3. The field shows the hotkey as text, for example `shift+cmd+4`.

To cancel the record, push Esc. To remove the hotkey, click **Clear**.

OpenCraft sends the hotkey to the application in front.

These keys are possible: the letters, the digits, F1 to F20, Return, Tab,
Space, Delete, Forward Delete, Esc, the arrow keys, Home, End, Page Up,
Page Down, and the punctuation keys. You cannot record Esc with no modifier
key, because Esc cancels the record. You cannot record a combination with
the Fn key.

## Type text

| Setting | What to give |
|---|---|
| Text | One or more lines of text |

OpenCraft types the text into the application in front. The text can have
letters with accents, currency signs, and emoji.

## Media key

| Setting | What to give |
|---|---|
| Key | Play or pause, Next, Previous, Volume up, Volume down, or Mute |

OpenCraft sends the media key, as the media keys of a keyboard do.

## Sound volume

| Setting | What to give |
|---|---|
| Operation | Up, Down, or Mute. The default is Up. |
| Step | The change for each operation, from 1 to 25 percent. The default is 5. |

- This action changes the volume of the default sound output directly. It
  does not need the Accessibility permission.
- Mute changes between muted and not muted. Up and Down also stop the mute.
- The key face or the bar panel of the binding shows the volume as a bar
  at the bottom. When the sound is muted, it shows "Muted". OpenCraft reads
  the volume again each 2 seconds, thus the bar also shows a change that you
  make with a different method.
- A good use is a knob: set Up on "Turn right", Down on "Turn left", and Mute
  on "Push". The bar shows on the bar panel above the knob.

## Change the page

| Setting | What to give |
|---|---|
| Page | Next page, Previous page, or Page number. The default is Next page. |
| Page number | The number of the page, from 1. OpenCraft uses it only for "Page number". |

- After the last page, "Next page" goes to the first page. Before the first
  page, "Previous page" goes to the last page.
- If the profile has no page with that number, the window shows a message.

Pages are in [Knobs, touch bar, pages, and profiles](controls.md#pages).

## Select a profile

| Setting | What to give |
|---|---|
| Profile name | The name of a profile, as the sidebar shows it |

- The device shows the first page of that profile.
- If no profile has that name, the window shows a message.
- If two profiles have the same name, OpenCraft selects one of them. Give
  each profile a different name.

Profiles are in
[Knobs, touch bar, pages, and profiles](controls.md#profiles).

## Toggle

A toggle has two states, on and off. Each operation of the control changes
the state, and does the action for the new state.

| Setting | What to give |
|---|---|
| Action for on | The action to do when the state changes to on |
| Action for off | The action to do when the state changes to off |
| Label for on | Optional. The label that the surface shows in the state on. |

1. Below "Action for on", click **Add an action**. Select an action kind.
2. Set the settings of that action on its card.
3. Do the same steps below "Action for off".
4. Optional: type a label in "Label for on".

To remove the action for on or off, click the button with its action name,
and click **Remove the action**.

- A toggle starts in the state off. Thus the first operation does the
  action for on.
- In the state on, the key face or the bar panel shows a frame. The frame
  has the colour of the label.
- OpenCraft keeps the state while it runs. When OpenCraft starts again, each
  toggle is off.
- If you change the settings of a toggle, its state becomes off.
- A toggle does not read the state of the computer. If you change the same
  thing with a different method, the state of the toggle can be incorrect.
  Then operate the control again.

## Sequence of actions

A sequence does a list of actions, one after the other.

| Setting | What to give |
|---|---|
| Actions | The list of actions, in their order |
| Delay between two actions, in milliseconds | From 0 to 10000. The default is 100. |

1. Below "Actions", click **Add an action**. Select an action kind.
2. Set the settings of that action on its card.
3. Do steps 1 and 2 again for each action.
4. To change the order, click **↑** or **↓** above an action. To remove an
   action, click **Remove** above it.

- The sequence does not wait until an action is complete. The delay is the
  only wait. For an action that needs time, for example the start of an
  application, set a longer delay.
- A sequence and a toggle can contain each action kind, also a different
  toggle or sequence.
