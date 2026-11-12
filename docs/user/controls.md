# Knobs, touch bar, pages, and profiles

## The gestures of each control

You select a control in the device picture. Then you select a gesture below
**Action** in the editor, and set an action for it. Each gesture can have a
different action.

| Control | How to select it | Gestures |
|---|---|---|
| Key | Click its key face | Press, Release |
| Knob | Click the knob below the bar panels | Turn left, Turn right, Push |
| Bar panel | Click the bar panel | Tap |
| Touch bar | Click the strip "◀ Touch bar ▶" below the bar panels | Swipe left, Swipe right |

## Knobs

1. Click a knob, for example **Knob 1**.
2. Below **Action**, click a gesture: **Turn left**, **Turn right**, or
   **Push**. **Turn right** is selected first.
3. Set an action for the gesture. See [Actions](actions.md).

- Each click of a turn does the action one time. A fast turn does the action
  many times.
- A knob has no display. The action of a knob shows its result on the bar
  panel above the knob. For example, the sound volume shows a bar on bar
  panel 1 for knob 1.
- A knob has no "release" gesture.

The colours of the knob lights are device settings. See
[Device settings](#device-settings).

## The touch bar

The touch bar is one long touch display. OpenCraft shows it as four bar
panels. Each bar panel has its own label, colour, and image, as a key face
does.

There are two types of touch gesture:

| Gesture | Control | Where |
|---|---|---|
| Tap | One bar panel | The action of the bar panel that you touch |
| Swipe left, Swipe right | The touch bar | The same action on all of the touch bar. A swipe has no position. |

To set the action of a tap:

1. Click a bar panel in the device picture.
2. Set the label, the colour, the image, and the action, as for a key. See
   [Configure a key](first-key.md).

To set the action of a swipe:

1. Click the strip "◀ Touch bar ▶" below the bar panels.
2. Click **Swipe left** or **Swipe right**.
3. Set an action. A good action for a swipe is "Change the page".

Sometimes the device does not identify a touch as a tap or as a swipe. Then
nothing occurs. Touch again.

## Pages

A page is one full set of key faces, bar panels, and bindings. The device
shows one page at a time. A profile has one page or more.

The page row is below the top of the window. The button of the active page
has a blue frame.

| To do this | Do this |
|---|---|
| Show a different page on the device | Click the button of the page |
| Add an empty page | Click **+ Page**. The new page becomes the active page. |
| Copy the active page | Click **Duplicate** |
| Give the active page a name | Click **Rename**, or double-click the page button. Type the name, and push Return or click **Save**. |
| Move the active page | Click **◀** or **▶** |
| Remove the active page | Click **Remove**, and then **Remove** again. A profile keeps one page at the minimum. |

When you click a page button, the device shows that page also. To change the
page from the device, set the action "Change the page" on a control. Use the
same control on each page, so that you can always go to the next page. See
[Change the page](actions.md#change-the-page).

Example: on each page, set "Swipe left" of the touch bar to "Next page", and
"Swipe right" to "Previous page".

## Profiles

A profile is a named set of pages. You can use one profile for each type of
work, for example one for video calls and one for photos. The device shows
the active profile.

Click the profile button at the top right, next to "Profile". The menu shows
all profiles. The active profile has a check mark.

| To do this | Do this in the menu |
|---|---|
| Select a different profile | Click the name of the profile |
| Add an empty profile | Click **New**. Type a name, and click **Add**. |
| Copy the active profile | Click **Duplicate**. Type a name, and click **Add**. |
| Give the active profile a different name | Click **Rename**. Type the name, and click **Save**. |
| Remove the active profile | Click **Remove**, and then **Remove** again. You cannot remove the last profile. |

- A new profile or a copy becomes the active profile.
- OpenCraft does not delete a removed profile. It moves the profile to the
  folder `removed` in the profiles folder. To get it back, see
  [The files of OpenCraft](background.md#the-files-of-opencraft).
- To change the profile from the device, set the action "Select a profile"
  on a control. See [Select a profile](actions.md#select-a-profile).

## Device settings

The device settings are for the device, not for a profile. A change of the
profile or of the page does not change them.

1. Click **Device settings** at the top right. The editor shows the device
   settings. No control has the blue frame.
2. Change a setting. To change a brightness, drag its slider. The number
   to the right of the setting name shows the value. The device shows the
   change immediately.
3. To go back to the editor of a control, click a control in the device
   picture.

![The device settings](images/device-settings.png)

| Setting | Values | Default |
|---|---|---|
| Display brightness | 15 to 100 percent. The device gives the same light for all values from 0 to 15, thus the lowest value is 15. | 80 percent |
| Haptic feedback | On or Off. The device vibrates a little when you operate a control. | On |
| Knob lights | **Effect**: the colours of the device change slowly. **Colours**: one colour for each knob. **Off**: no light. | Effect |
| Knob light brightness | 1 to 100 percent | 100 percent |

To give each knob light a colour:

1. Below **Knob lights**, click **Colours**.
2. For each knob, click a colour. Or type a colour in the form `#RRGGBB`,
   for example `#FF8000`.

A black knob light is off. If all knob lights are black, all knob lights are
off.

You can change the device settings when no device is connected. OpenCraft
sends them to the device when you connect it.
