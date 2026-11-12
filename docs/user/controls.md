# Knobs, touch bar, pages, and profiles

## The gestures of each control

You select a control in the device picture. Then you set an action on the
card of a gesture, below **Gestures** in the inspector. Each gesture has its
own card, and each gesture can have a different action. The procedure is in
[Configure a key](first-key.md#4-set-an-action).

| Control | How to select it | Gestures |
|---|---|---|
| Key | Click its key face | Press, Release |
| Knob | Click the knob below the bar panels | Turn left, Turn right, Push |
| Bar panel | Click the bar panel | Tap |
| Touch bar | Click the small control "◀ Touch bar ▶" above the bar panels. This selects the page: the swipes are part of the page. | Swipe left, Swipe right |

## The device picture

The canvas in the middle of the window shows the device. Each key face and
each bar panel is at its real position on the display of the device, and
shows the same image as the device. The knobs are below the bar panels.
Knob 1 is below bar panel 1.

| Part | What it shows |
|---|---|
| A blue outline | The selected control. At the start, the selected control is the key at the top left. |
| A thin blue outline | The control below the pointer |
| A dashed blue outline on a bar panel | The bar panel that gives feedback for the selected knob. The line below the device tells which knob. |
| A dashed border and a plus (+) on a key face | The key has no label, no colour, no image, and no action |
| The ring of a knob | The colour of the knob light, when the knob lights are set to **Colours**. Else the ring is grey. |
| A yellow glow | The control that you operate on the device now. A yellow dot shows where you touch the touch bar. |
| "◀ Touch bar ▶" | The control for the swipe gestures, between the keys and the bar panels. It is blue when the page is selected. |

![The device picture with knob 1 selected, and the knob lights set to Colours](images/device-picture.png)

Click an empty part of the canvas to select the page. The inspector then
shows the page. See [The page inspector](#the-page-inspector).

Above the device is the page header. It shows the name of the active page
and "Page 1 of 3", for example. At its right end are these buttons:

| Button | What it does |
|---|---|
| The arrow to the left (**Previous page**) | Shows the page before the active page. It is dim on the first page. |
| The arrow to the right (**Next page**) | Shows the page after the active page. It is dim on the last page. |
| **Page settings** | Selects the page, and shows the **Inspector** tab of the right panel |

The device shows the same page as the window. If the window is small, the
device picture is smaller, down to half of its full size.

## Knobs

1. Click a knob, for example **Knob 1**. The inspector shows the cards
   **Turn left**, **Turn right**, and **Push**. The header shows the bar
   panel that gives the feedback of the knob, for example "Feedback on bar
   panel 1".
2. On the card of a gesture, click **Add an action**, and set an action.
   See [Actions](actions.md).

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

1. Click the small control "◀ Touch bar ▶" above the bar panels. The
   inspector shows the page, and moves down to the cards **Swipe left** and
   **Swipe right** below **TOUCH BAR GESTURES**.
2. On one of the cards, click **Add an action**, and set an action. A good
   action for a swipe is "Change the page".

Sometimes the device does not identify a touch as a tap or as a swipe. Then
nothing occurs. Touch again.

## Pages

A page is one full set of key faces, bar panels, and bindings. The device
shows one page at a time. A profile has one page or more.

The sidebar at the left shows the pages of the active profile, below the
name of the profile. The number at the right of a page is its position. The
blue row is the active page. The name of the active page is also at the top
of the device picture.

![The sidebar with two profiles. The active profile "Work" has three pages, and "Main" is the active page.](images/sidebar.png)

| To do this | Do this |
|---|---|
| Show a different page on the device | Click the page in the sidebar |
| Add an empty page | Click **Add a page** below the pages. The new page becomes the active page. |
| Copy a page | Right-click the page, and select **Duplicate**. The copy goes after the last page and becomes the active page. |
| Give a page a different name | Double-click the page. Or right-click the page, and select **Rename**. Type the name, and push Return. |
| Move a page up or down in the list | Right-click the page, and select **Move Up** or **Move Down** |
| Remove a page | Right-click the page, and select **Remove…**. Then click **Remove**. A profile keeps one page at the minimum. |

When you edit a name, push Escape to keep the old name. A click outside the
name keeps the new name. A name cannot be empty.

You can also do these changes in the page inspector. See
[The page inspector](#the-page-inspector).

### The page inspector

The page inspector shows the settings of the active page. To show it, do
one of these steps:

- Click an empty part of the canvas.
- Click **Page settings** in the page header.
- Click the small control "◀ Touch bar ▶". The inspector then moves down to
  the swipe gestures.

The header shows a small picture of the page, the name of the page, and
"Page 2 of 3 · Profile “Work”", for example.

| Part | What it does |
|---|---|
| **Name** | The name of the page. Type a name, and push Return or click outside the field. Push Escape to keep the old name. |
| **Order** | The arrow up moves the page up in the list. The arrow down moves the page down. An arrow is dim on the first or the last page. |
| **Duplicate** | Adds a copy of the page |
| **Remove…** | Asks for a confirmation, and then removes the page. It is dim when the profile has one page only. |
| **TOUCH BAR GESTURES** | The cards **Swipe left** and **Swipe right** of this page. See [The touch bar](#the-touch-bar). |

⌘Z undoes each of these changes. One edit of the name is one step.

![The page inspector with an action on Swipe left](images/page-inspector.png)

When you click a page, the device shows that page also. To change the
page from the device, set the action "Change the page" on a control. Use the
same control on each page, so that you can always go to the next page. See
[Change the page](actions.md#change-the-page).

Example: on each page, set "Swipe left" of the touch bar to "Next page", and
"Swipe right" to "Previous page".

To hide the pages of the active profile, click the arrow at the left of its
name. Click the arrow again to show them.

## Profiles

A profile is a named set of pages. You can use one profile for each type of
work, for example one for video calls and one for photos. The device shows
the active profile.

The sidebar shows all profiles below **PROFILES**. The active profile has
the mark **ON DEVICE**, and its pages are below it.

| To do this | Do this in the sidebar |
|---|---|
| Select a different profile | Click the name of the profile |
| Add an empty profile | Click **+** at the right of **PROFILES**. Type a name in the new row, and push Return. |
| Copy a profile | Right-click the profile, and select **Duplicate**. The copy has the name "*name* copy". |
| Give a profile a different name | Double-click the profile. Or right-click the profile, and select **Rename**. Type the name, and push Return. |
| Remove a profile | Right-click the profile, and select **Remove…**. Then click **Remove**. You cannot remove the last profile. |

- A new profile or a copy becomes the active profile.
- A double-click on a profile that is not active also makes it the active
  profile.
- OpenCraft does not delete a removed profile. It moves the profile to the
  folder `removed` in the profiles folder. To get it back, see
  [The files of OpenCraft](background.md#the-files-of-opencraft).
- To change the profile from the device, set the action "Select a profile"
  on a control. See [Select a profile](actions.md#select-a-profile).

## Device settings

The device settings are for the device, not for a profile. A change of the
profile or of the page does not change them.

1. Click the device at the top of the sidebar, or **Device settings** in the
   toolbar. The **Inspector** tab of the right panel shows the device
   settings. The body of the device picture has a blue outline. No control
   has a blue outline.
2. Change a setting. To change a brightness, drag its slider. The number
   to the right of the setting name shows the value. The device shows the
   change immediately.
3. To go back to the settings of a control, click a control in the device
   picture, or click the active page in the sidebar.

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
