# Checks of Phase 7 and Phase 8 on the device

This procedure is for the owner. It has one profile with one key for each new
action of [Phase 7](https://github.com/shantanugoel/OpenCraft/issues/199) and
[Phase 8](https://github.com/shantanugoel/OpenCraft/issues/200). Press each
key one time, and answer each question with yes or no. Write the answers in
the issues #199 and #200.

The file `phase-7-8-checks.opencraft` is in this directory. The profile has 3
pages. Each page has 10 keys. The key at the end of each page changes the
page, and it shows the page number. The profile has no personal data.

## Make ready

1. Start OpenCraft with the release bundle. Connect the N4 Pro.
2. Give the Accessibility permission to OpenCraft. See
   [Give the Accessibility permission](../user/install.md#give-the-accessibility-permission).
3. In the sidebar, click the name of the active profile with the right mouse
   button. Click **Profile Settings…**.
4. Click **Import…**. Select `phase-7-8-checks.opencraft`, and click
   **Import**. See
   [Export and import a profile](../user/controls.md#export-and-import-a-profile).
5. Click **Done**. The profile "Phase 7 and 8 checks" is now the active
   profile. The device shows page 1.
6. Open a text editor with an empty document. Keep it ready for the keys
   that type text.
7. Open a Finder window that shows some files. Open a media player, and start
   a song.
8. Write down the name of the sound output and the brightness of the
   displays. You put them back at the end.
9. Save all your work. The keys **Lock** and **Sleep** lock the screen and
   make the computer sleep.

## Page 1: Device and media

1. Press **Screen off**. Did the displays and the knob lights go dark? yes/no
2. Press any key of the dark device. Did the device show the profile again?
   yes/no
   - If the answer is no, this is not a fault. Select **Wake Device** in the
     menu bar item. Did the device show the profile again? yes/no
3. Press **Device sleep**. Did the displays and the knob lights go dark?
   yes/no
4. Press any key of the dark device. Did the device show the profile again?
   yes/no
   - If the answer is no, select **Wake Device** in the menu bar item. Did
     the device show the profile again? yes/no
5. Press **Brightness up**. Did the displays get brighter? yes/no
6. Press **Brightness down**. Did the displays get less bright? yes/no
7. Turn knob 1 to the right. Did the displays get brighter? yes/no
8. Turn knob 1 to the left. Did the displays get less bright? yes/no
9. Press **Fast forward**. Did the media player go forward or skip to the next
   track? yes/no
10. Press **Rewind**. Did the media player go back or skip to the previous
    track? yes/no
11. Press **Mic mute**. Did the key show "Microphone muted"? yes/no
12. Press **Mic mute** again. Did the key show "Microphone on"? yes/no
13. Press **Next output**. The key shows the name of the sound output. Did the
    name change to a different output? yes/no
14. Does the sound of the Mac come from that output? yes/no
15. Press **Next output** again, until the first name shows. Did the first name
    come back? yes/no
16. Put the pointer on a file in the Finder window. Press **Mouse click**. Did
    Finder select the file? yes/no
17. Put the pointer on a long list, for example in Finder. Turn knob 2 to the
    right. Did the list scroll down? yes/no
18. Turn knob 2 to the left. Did the list scroll up? yes/no
19. Press the key with the arrow and the number 1. Did the device show page 2?
    yes/no
20. Does the key show the number 2 on page 2? yes/no

## Page 2: Apps and input

1. Press **Play sound**. Did you hear a sound? yes/no
2. Press **Play sound**. Then press **Stop sound** in less than 2 seconds. Did
   the sound stop at once? yes/no
3. Click in the empty text document. Press **Type text**. Did the text "Hello
   from OpenCraft" show in the document? yes/no
4. Press **Open folder**. Did Finder open the folder "Shared"? yes/no
5. Press **Open URL**. Did Safari open the page example.com? yes/no
6. Press **Open Calculator**. Did Calculator open? yes/no
7. Press **Quit Calculator**. Did Calculator quit? yes/no
8. Click in the empty text document. Press **Hotkey right**. Did a capital A
   show in the document? yes/no
9. Click in the empty text document. Press **Hotkey Fn**. Did the window
   "Emoji & Symbols" open? yes/no
   - Push Esc to close the window.
10. Press the key with the arrow and the number 2. Did the device show page 3?
    yes/no

## Page 3: System

1. Press **Mission Control**. Did Mission Control show the windows? yes/no
   - Push Esc to close it.
2. Press **Show desktop**. Did the windows move away? yes/no
3. Press **Show desktop** again. Did the windows come back? yes/no
4. Press **Launchpad**. Did Launchpad, or Apps on macOS 26 and later, open?
   yes/no
   - Push Esc to close it.
5. Press **Screenshot**. Did the screenshot controls show? yes/no
   - Push Esc to close them.
6. Click in the empty text document. Press **Sequence 1 2 3**. Did "1" show at
   once, "2" show after approximately 1.5 seconds, and "3" show a short time
   after "2"? yes/no
7. Press **Brightness 80**. Is the brightness of the displays 80 % again?
   yes/no
8. Press **Screen saver**. Did the screen saver start? yes/no
   - Move the pointer to stop it.
9. Press **Lock**. Did the screen lock? yes/no
10. Did the device go dark? yes/no
11. Sign in. Did the device show the profile again? yes/no
12. Press **Sleep**. Did the computer go to sleep? yes/no
13. Wake the computer. Did the device show the profile in 10 seconds or less?
    yes/no
14. Press the key with the arrow and the number 3. Did the device show page 1?
    yes/no

## After the checks

1. Set the brightness and the sound output to the values that you wrote down.
2. Quit the media player and the text editor.
3. Remove the profile "Phase 7 and 8 checks". In the sidebar, click it with
   the right mouse button, and click **Remove…**.

## How the file was made

The file is an export of the profile from the window of OpenCraft, with the
function **Export as a file…**. The format is in
[decision 0031](../decisions/0031-profile-export-file.md). The file has no
image and no personal data. The paths in it are `/Users/Shared`,
`/System/Library/Sounds/Funk.aiff`, and the names of the applications Safari
and Calculator. The action **Next output** has an empty list. Thus it uses
all sound outputs that are connected.
