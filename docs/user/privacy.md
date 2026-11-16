# Privacy

OpenCraft does not connect to the network by itself. It does not send
telemetry. It has no account. You do not sign in.

Telemetry is a report about how you use the application.

## What OpenCraft stores

OpenCraft stores your data on this Mac. It does not send the data to a
server.

The folder is `~/Library/Application Support/OpenCraft`. It holds your
profiles, the device settings, the options of the window, and the log file.
The names of the files are in
[The files of OpenCraft](background.md#the-files-of-opencraft). The log file
stays in that folder. See [The log file](background.md#the-log-file).

When you turn on **Start at login**, OpenCraft writes the file
`~/Library/LaunchAgents/opencraft.login.plist`. See
[Start at login](background.md#start-at-login).

When you export a profile, you choose the folder and the name of the file.
OpenCraft writes `~` in place of your home folder in each path in that
file. See
[Export and import a profile](controls.md#export-and-import-a-profile).

The **User guide** and **Report a fault** links in Settings > About ask
macOS to open a fixed page in your browser. The browser uses the network
only when you click a link. OpenCraft fetches nothing for the About tab.

## Actions that you start

OpenCraft itself does not use the network. These two actions can use the
network when you run them:

- **Open a URL** asks macOS to open the address.
- **Run a shell command** runs the command that you write. The command can
  use the network.

## Control by other programs

The setting **Allow control by other programs** permits programs that run as
your user to read and change your profiles. See
[Turn on the control](control.md#turn-on-the-control) for the switch. While it
is off, OpenCraft makes no socket file, and no other program can send a
request.

When you turn it on, OpenCraft makes the file `control.sock` in the folder
`~/Library/Application Support/OpenCraft`, next to the profiles folder. The
file is a socket on this Mac. It is not a network connection. A computer on
your network, or on the internet, cannot reach it. Only you can open the
file: its mode is `0600`. OpenCraft also reads the user of each program that
connects, and it closes the connection if that user is not you. The file has a
row in [The files of OpenCraft](background.md#the-files-of-opencraft).

With the setting on, each program that runs as your user can do what the
list in [What a program can do](control.md#what-a-program-can-do) says. It
can read your profiles and their settings, and it can change them. This
includes the action of a key. A program that you do not trust must not run as
your user while the setting is on. This is the same risk as a program that
changes your profile files while OpenCraft does not run.

OpenCraft sends no data to a server because of this setting.
