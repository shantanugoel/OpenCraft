# Actions

An action is the work that OpenCraft does when you operate a control. This
chapter gives each action kind and its settings. The procedure to set an
action is in [Configure a key](first-key.md#4-set-an-action).

The action kinds are in four groups. The [library](#the-library) and the
menu of the action kinds show the groups and the kinds in this order:

| Group | Action kind | Needs the Accessibility permission |
|---|---|---|
| System | [Open an application](#open-an-application) | No |
| System | [Open a URL](#open-a-url) | No |
| System | [Run a shell command](#run-a-shell-command) | No |
| System | [Send a hotkey](#send-a-hotkey) | Yes |
| System | [Type text](#type-text) | Yes |
| Media | [Media key](#media-key) | Yes |
| Media | [Sound volume](#sound-volume) | No |
| Device | [Screen off and Device sleep](#screen-off-and-device-sleep) | No |
| Device | [Device brightness](#device-brightness) | No |
| Navigation | [Change the page](#change-the-page) | No |
| Navigation | [Select a profile](#select-a-profile) | No |
| Logic | [Toggle](#toggle) | Only for the actions that it contains |
| Logic | [Sequence of actions](#sequence-of-actions) | Only for the actions that it contains |

The permission is in
[Install and first start](install.md#give-the-accessibility-permission).

## The library

The tab **Library** of the right panel shows all action kinds in their
groups. Each row shows the tile of the kind, its name, and at the right a
short help text: "Accessibility" for a kind that needs the Accessibility
permission, and "Knob" for **Sound volume**, which is best on a knob. The
tile has the colour of the group and the icon of the kind, for example a
globe for **Open a URL**.

![The library, after a click on "Open a URL" with key 7 selected](images/library.png)

To set an action with the library:

1. In the device picture, click a key face, a bar panel, or a knob.
2. Click **Library** at the top of the right panel.
3. Click an action kind.

The action goes on the open card of the control, with the default settings
of the kind. If no card is open, the action goes on the first gesture:
**Press** for a key, **Tap** for a bar panel, and **Turn** for a knob (the
action goes on the two turn directions).
If the gesture has an action already, the new action replaces it. To get the
old action back, push Command-Z. The status bar shows the change for 4
seconds, for example "Key 7 gets “Open a URL” for Press". The right panel
stays on **Library**. Click **Inspector** to change the settings of the new
action.

If the page or the device is selected, a click on an action kind does
nothing, and the status bar shows "Select a control first".

### Drag an action onto a control

You can also drag an action kind from the library onto a control. You do
not have to select the control first.

![A drag of "Open a URL" onto an empty key face](images/drag-and-drop.png)

1. Click **Library** at the top of the right panel.
2. Push and hold the mouse button on an action kind, and move the pointer.
   The row of the kind gets a dashed border, and a small picture of the
   kind moves with the pointer.
3. Move the pointer onto a key face, a bar panel, or a knob. Only the
   control below the pointer gets a dashed blue border and the text
   "Drop here".
4. Release the mouse button.

The action goes on the first gesture of the control, with the default
settings of the kind: **Press** for a key, **Tap** for a bar panel, and
**Turn** for a knob. This is also true when a different card of the control
is open. The drop selects the control and opens the card of that gesture.
The status bar shows the change, for example "Key 9 gets “Open a URL” for
Press". The right panel stays on **Library**.

- If the gesture has an action already, the new action replaces it. One
  Command-Z (**Undo Drop Action**) gets the old action back.
- If you release the mouse button outside a control, nothing changes.
- During the drag, the line of help below the device tells where you can
  drop the action.

To find a kind, type a part of its name in the search field at the top of
the library, for example `text`. The library then shows only the kinds with
that text in their names. If no kind agrees, it shows "No action matches"
and your text. To see all kinds again, delete the text.

## The settings of an action

The open card of a gesture shows the settings of its action, one setting on
each row. The control of a setting agrees with its type:

| Type of setting | Control |
|---|---|
| Text | A text field. Some text fields have three lines or more. |
| Number | A small text field for digits. A number out of the range changes to the nearest permitted number. |
| Selection | A button with the selected value. Click it, and click a value in the menu. |
| Current selection | A menu that the action fills when the form opens. A saved value that is not available stays visible and stays in the profile. Open the form again to refresh the list. |
| Colour | The fill colour samples and a field for `#RRGGBB`. An incomplete value changes nothing. |
| On or off | A switch |
| Hotkey | A field that records a hotkey. See [Send a hotkey](#send-a-hotkey). |
| File or application | A text field and **Choose…** |
| Action | A card in the card, with its own action kind and settings. See [Toggle](#toggle). |
| List of actions | A list of steps above the other settings. See [Sequence of actions](#sequence-of-actions). |

If a setting has a long name, the name is above the control.

This test form shows a current selection and a colour field:

![A test form with a current choice menu and shared colour samples](images/action-field-types.png)

## Test an action

You can run the action of a gesture with no push on the device.

1. Click the card of the gesture to open it.
2. Click **Test the action** at the end of the card.

OpenCraft runs the action in the same way as when you do the gesture on the
device. The status bar shows, for 4 seconds, which action OpenCraft tests, for
example "Testing “Open a URL” of Key 8 for Press".

- If the action fails, the window shows a message above the status bar. See
  [Messages in the window](faults.md#messages-in-the-window).
- If the action needs the Accessibility permission and OpenCraft does not
  have it, you get the same message as from the device. See
  [Give the Accessibility permission](install.md#give-the-accessibility-permission).
- The test uses the settings that the card shows now.
- A test does not change the profile. **Undo** does not undo a test.
- A card of an action that is not available has no button.

## An action that is not available

A profile can contain an action that this version of OpenCraft does not
have, for example from a newer version. Then the button of the card shows
**Not available**. The card shows the identifier of the action kind, and the
text "This version of OpenCraft does not have this action. The binding
stays in the profile."

- OpenCraft keeps the action and its settings in the profile.
- To use a different action, click **Not available**, and click an action
  kind. To remove the action, click **Remove the action**.

## How OpenCraft checks a setting

OpenCraft saves each change immediately, as in
[Configure a key](first-key.md#how-opencraft-saves-your-changes).

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
| Hotkey | A key with modifiers, or modifiers only. Left and right Control, Option, Shift and Command, and Fn are available. |

1. Click the field **Record hotkey…**. The field gets a blue frame and shows
   "Push a key…".
2. Push the key combination on the keyboard of the computer, for example
   Shift-Command-4.
3. The field shows the hotkey with symbols, in the order of macOS, for
   example "⇧⌘4". The order is Control (⌃), Option (⌥), Shift (⇧), and
   Command (⌘), and then the key.

For modifiers only, push the modifiers and release them all. The record
completes on release. A right-side modifier has `R` before its symbol, for
example `R⌃` for right Control. Fn shows as `Fn`.

To cancel the record, push Esc. To remove the hotkey, click **Clear**.

OpenCraft sends the hotkey to the application in front.

These keys are possible: the letters, the digits, F1 to F20, Return, Tab,
Space, Delete, Forward Delete, Esc, the arrow keys, Home, End, Page Up,
Page Down, and the punctuation keys. You cannot record Esc with no modifier
key, because Esc cancels the record. A hotkey sends a complete push and
release, not a hold. Old profiles keep their behaviour. Some applications
do not accept a synthetic Fn event.

![The hotkey form with right Control and Fn.](images/hotkey.png)

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
- A good use is a knob: set **Sound volume** on "Turn", and Mute on "Push".
  The "Turn" card sets Up for a turn to the right and Down for a turn to the
  left. See [The Turn card](controls.md#the-turn-card). The bar shows on the
  bar panel above the knob.

## Device brightness

This action changes the displays of the controller, not the computer display.
It is in the **Device** group. No Accessibility permission is necessary.

- **Up** or **Down** changes the brightness by **Step** percentage points.
  The default step is 5. The value stops at the limits of the device.
- **Set value** uses **Brightness**. Values below the lowest brightness of
  the model use that lowest brightness. On the N4 Pro it is 15 %. Brightness
  0 does not switch off the display: use **Screen off** for that.
- On a knob, assign this kind to **Turn**. Right goes up and left goes down.
  On a key, select Up, Down or Set value.

The new value shows in the device inspector, in **Device settings**, and in
the menu bar brightness slider. OpenCraft saves it for the next start. The
knob lights do not change. The action does not wake a dark or locked device.

![Device brightness settings](images/device-brightness-action.png)

## Screen off and Device sleep

These kinds are in the **Device** group of the library. They have no settings.
They do not change the saved brightness.

- **Screen off** makes the displays and knob lights dark. OpenCraft keeps
  the connection and its keep-alive in operation.
- **Device sleep** makes them dark and stops the device keep-alive timer.
  Use this when you do not need the device for a time.

To wake either state, select **Wake Device** in the menu bar item or open the
OpenCraft window. The full profile and stored settings return. A device input
report also wakes it, if the firmware sends that report while dark. That
first gesture, including its release, does not run an action. Whether this
firmware sends input after sleep is not verified. Do not rely on a device
key as the only wake path. While the screen is locked, no wake or device
action can override the lock. See
[Sleep and screen lock](background.md#sleep-and-screen-lock).

![The Device actions in the library](images/device-actions.png)

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
| From Off to On | The action to do when the state changes to on |
| From On to Off | The action to do when the state changes to off |
| Label for on | Optional. The label that the surface shows in the state on. |

![A toggle on key 6. The action "From Off to On" is a sequence with its second step open. The action "From On to Off" sends a hotkey.](images/toggle-and-sequence.png)

The card of the toggle shows each action as a card in the card:

1. Below "From Off to On", click **Add an action**. Select an action kind.
   The card of the new action opens.
2. Set the settings of that action on its card.
3. Do the same steps below "From On to Off".

- A closed card shows the action kind and a short value, for example the
  keys of a hotkey. Click the card to open it. Click the top row of an open
  card to close it.
- To change the action kind, click the kind name at the top of the open
  card, and select a different kind. To remove the action, select **Remove
  the action**.

**Appearance for each state.** When the action of **Press** of a key, or
of **Tap** of a bar panel, is a toggle, the inspector shows the section
"Appearance for each state" in place of "Appearance".

- The section shows the face **Off** and the face **On** next to each other,
  as the device shows them. The face of the state now has the text "· now".
  The second line of the inspector header also shows the state, for example
  "Toggle · the state now is Off".
- Click **Off** to change the image, the title, the look of the title, and
  the fill of the key.
- Click **On** to change the label for on in **Title**. The face On uses the
  fill and the image of the face Off, with a frame.

Notes:

- A toggle starts in the state off. Thus the first operation does the
  action for on.
- In the state on, the key face or the bar panel shows a frame. The frame
  has the colour of the title.
- OpenCraft keeps the state while it runs. When OpenCraft starts again, each
  toggle is off.
- If you change the settings of a toggle, its state becomes off.
- A toggle does not read the state of the computer. If you change the same
  thing with a different method, the state of the toggle can be incorrect.
  Then operate the control again.

## Sequence of actions

A sequence does a list of actions, one after the other. Each action is a
step.

| Setting | What to give |
|---|---|
| The steps | The list of actions, in their order |
| Delay between two actions, in milliseconds | From 0 to 10000. The default is 100. |

![A sequence of four steps. Step 1 is open.](images/sequence.png)

The card of the sequence shows one row for each step: a handle, the number,
the action kind, and a short value.

1. Click **Add a step**. Select an action kind. The new step opens.
2. Set the settings of that step on its card.
3. Do steps 1 and 2 again for each action.

- Click a step to open it. Click the top row of the open step to close it.
- To move a step, drag its handle (six dots) to a different row. You can
  also right-click the step, and click **Move Up** or **Move Down**.
- To remove a step, right-click it and click **Remove**. An open step also
  has the button **⋯** with the same menu.
- Each change is one step of the undo history, for example "Undo Move Step".
  Push Command-Z to undo it.

Notes:

- The sequence does not wait until an action is complete. The delay is the
  only wait. For an action that needs time, for example the start of an
  application, set a longer delay.
- A sequence and a toggle can contain each action kind, also a different
  toggle or sequence. For example, the action "From Off to On" of a toggle
  can be a sequence.
