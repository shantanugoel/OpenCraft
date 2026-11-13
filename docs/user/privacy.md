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
See [Export and import a profile](controls.md#export-and-import-a-profile).

## Actions that you start

OpenCraft itself does not use the network. These two actions can use the
network when you run them:

- **Open a URL** asks macOS to open the address.
- **Run a shell command** runs the command that you write. The command can
  use the network.
