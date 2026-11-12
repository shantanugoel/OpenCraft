# Configure a key

This procedure makes key 1 open a web page. It uses the label, the colour,
the image, and the action of one key. Do the steps in their order.

Before you start, do the steps in [Install and first start](install.md).

## 1. Select the key

1. In the device picture, click the key face at the top left. This is key 1.

The key face gets a blue frame. The editor at the right shows "Key face 1".
It has these parts, from the top: **Action**, **Label**, **Colour**, and
**Image**. If the editor is not tall enough, it shows a scrollbar at the
right. Scroll the editor to see the other parts.

## 2. Set the label and the colour

1. Click the field **Label**. Type a short text, for example `Docs`.
2. Below **Colour**, click a colour.

The key face in the window and key 1 on the device show the label on the
colour. OpenCraft selects black or white for the text, so that you can read
it on the colour.

To remove the colour, click **No colour** to the right of **Colour**. To
remove the label, delete the text in the field.

## 3. Set an image (optional)

1. Below **Image**, click **Select image…**.
2. In the file dialog, select a PNG or JPEG file, and click **Open**.

The key face shows the image above the label. A transparent part of the image
shows the colour of the key face.

- OpenCraft makes a copy of the image in the profile. You can move or delete
  the original file after this step.
- A large image becomes smaller. A square image is best for a key face.
- To remove the image, click **Remove image**.

## 4. Set an action

Below **Action**, the editor shows the gestures of the key: **Press** and
**Release**. **Press** is selected. An action on **Press** occurs when you
push the key down. An action on **Release** occurs when you let the key come
back up. Most actions are on **Press**.

1. Click **No action ▾**. The list of the action kinds opens.
2. Click **Open a URL**.
3. Click the field **URL**. Type `https://example.com`.

A gesture that has an action shows a dot, for example **Press •**.

![The editor with the sound volume action on knob 1](images/action-editor.png)

The picture shows a different action: the sound volume on knob 1. Each
action kind has its own settings. They are in [Actions](actions.md).

To remove the action, click the button with the action name, and click
**No action**.

## 5. Push the key

1. Push key 1 on the device.

The browser opens the page. The input line at the bottom of the window shows
"key 1: press", and then "key 1: release".

## How OpenCraft saves your changes

OpenCraft saves each change immediately. The window has no "Save" button.
You can close the window at any time. You can also make changes when no
device is connected.

## If the key does nothing

- Look at the bottom of the window. If a message shows, read it. See
  [Messages in the window](faults.md#messages-in-the-window).
- Make sure that the connection shows "● Connected".
- Make sure that the action is on **Press**, not on **Release**.
- For "Send a hotkey", "Type text", and "Media key", OpenCraft needs the
  Accessibility permission. See
  [Give the Accessibility permission](install.md#give-the-accessibility-permission).

More corrections are in [Faults and corrections](faults.md).

## Next steps

- Configure the other keys in the same way.
- Use a knob, a bar panel, or the touch bar. See
  [Knobs, touch bar, pages, and profiles](controls.md).
- Make more pages for more actions. See [Pages](controls.md#pages).
