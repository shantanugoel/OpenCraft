# Checks of Phase 9 on the device

This procedure is for the owner. Answer each question with yes or no. Write
the answers in the issue of the check.

The checks use the connected N4 Pro and the OpenCraft window. They use no
personal data.

## Make ready

Release 0.2.0 does not read a profile of schema 5. A build of Phase 9
changes the profiles that it opens to schema 5. Thus do these checks with a
copy of the profiles, until you install a release with Phase 9.

1. Quit the installed OpenCraft from its menu bar item. Only one process
   can own the device.
2. In the repository, run:

   ```sh
   cp -R "$HOME/Library/Application Support/OpenCraft/profiles" /tmp/opencraft-phase-9
   OPENCRAFT_PROFILES_DIR=/tmp/opencraft-phase-9 cargo run --release -p opencraft-app
   ```

   If your profiles are in a different directory, copy that directory.
3. After the checks, quit this build, and start the installed OpenCraft
   again.

## Folders of keys

Issue: [#339](https://github.com/shantanugoel/OpenCraft/issues/339).

1. Start OpenCraft. Connect the N4 Pro. The device shows a page.
2. In the sidebar, click **Add a folder**. Does the device show a key with
   the title "Back" and the Back icon on key 1? yes/no
3. Click a different key of the device picture. Set its title to "Mic".
   Does that key on the device show "Mic"? yes/no
4. Click the page name in the path at the top of the device picture. Does
   the device show the keys of the page again? yes/no
5. Select one key of the page. Set its action to **Open a folder**, and
   select the folder. Press that key on the device. Does the device show
   the folder, with "Back" and "Mic"? yes/no
6. Press the **Back** key on the device. Does the device show the keys of
   the page again? yes/no
7. Press the key that opens the folder again. Then press the key that goes
   to the next page, or click **Next page** in the window if the page has
   no such key. Does the device show the next page, and not the folder?
   yes/no

## The profile follows the front application

Issue: [#338](https://github.com/shantanugoel/OpenCraft/issues/338).

Use three applications that are already on the Mac: Calculator, TextEdit,
and Preview. The steps make three profiles. They do not import a file. Save
your work before the screen lock.

1. Quit VSD Craft if it runs. Start OpenCraft. Connect the N4 Pro. The
   device shows a profile.
2. Open **Settings**, tab **General**. Is the switch **Change the profile
   with the front application** on? yes/no
3. In the sidebar, add three profiles. Click **+** at the right of
   **PROFILES**, then **New Profile**. Name them `Calculator`, `TextEdit`,
   and `Preview`. Push Return after each name.
4. Right-click **Calculator**, and select **Profile Settings…**. Click
   **Add an application…**. Select Calculator. Click **Done**.
5. Do the same for **TextEdit** with TextEdit, and for **Preview** with
   Preview.
6. Right-click a profile that is not one of these three, and select
   **Profile Settings…**. Turn on **Default profile** if the switch is off.
   If the switch is on and you cannot turn it off, this profile is already
   the default profile. Click **Done**.
7. In the sidebar, is **DEFAULT** on that profile when it is not the active
   profile? yes/no
8. Click Calculator in the Dock, or open it. Does the device show the
   profile Calculator? yes/no
9. In the OpenCraft window, does **ON DEVICE** show on Calculator? Point at
   **ON DEVICE**. Does the tooltip say that Calculator is in front? yes/no
10. Does the profile TextEdit show the name TextEdit where an inactive
    profile shows a mark? yes/no
11. Click TextEdit. Does the device show the profile TextEdit? yes/no
12. Click Preview. Does the device show the profile Preview? yes/no
13. Click an application that no profile lists, for example Finder. Does the
    device show the default profile? yes/no
14. Open the OpenCraft window. Click the profile Calculator in the sidebar.
    Does the device show Calculator? yes/no
15. Click TextEdit, so TextEdit is in front. Does the device show the profile
    TextEdit again? yes/no
16. Click the menu bar item of OpenCraft. Is **Follow the Front Application**
    checked? yes/no
17. Click **Follow the Front Application** to remove the check. Click Finder.
    Does the device stay on the profile TextEdit? yes/no
18. Open **Settings**, tab **General**. Is **Change the profile with the
    front application** off? yes/no
19. Turn the switch on again.
20. Lock the screen. Unlock it. Does the device still show the profile that
    it showed before the lock? yes/no
21. Click Calculator. Does the device show the profile Calculator? yes/no
22. Turn **Change the profile with the front application** to the state that
    you want to keep. On is the usual state.
23. Remove the three profiles if you do not want them. Right-click each one,
    select **Remove…**, and click **Remove**. You cannot remove the last
    profile. If one of them is the default profile, move **Default profile**
    to a profile that you keep before you remove it.

## A key selects a profile

Issue: [#337](https://github.com/shantanugoel/OpenCraft/issues/337).

1. Select one key of a page. Set its action to **Select a profile**. Select
   a different profile from the list.
2. Rename that different profile in its **Profile Settings…**.
3. Go back to the first profile. Press the key on the device. Does the
   device show the profile that you renamed? yes/no

## Ready profiles

Issue: [#340](https://github.com/shantanugoel/OpenCraft/issues/340).

1. In the sidebar, click **+** at the right of **PROFILES**. Select the
   ready profile **Finder**.
2. Does the sidebar show the profile Finder, and does the device picture
   show its keys? yes/no
3. Click a Finder window. Does the device show the profile Finder? yes/no
4. Press the key **Folder** on the device. Does Finder make a new folder?
   yes/no

## Look-ahead for Phase 10: the times of a key press

This step gives the data for long press and double press
([#202](https://github.com/shantanugoel/OpenCraft/issues/202)). Do it with
no OpenCraft process in operation.

On 2026-11-14 the owner did not record a file. Phase 9 does not use this
step. Phase 10 does these steps.

1. Run `cargo run --release -p opencraft-cli -- raw --save /tmp/opencraft-press.txt`.
2. Press key 1 short, one time. Wait 2 seconds.
3. Press key 1 and hold it for approximately 1 second. Wait 2 seconds.
4. Press key 1 two times fast. Wait 2 seconds.
5. Stop the command with Control-C. Tell the agent that the file is ready.

