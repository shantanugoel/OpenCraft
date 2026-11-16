# Control by other programs

A different program on your Mac can configure OpenCraft for you. For
example, a coding assistant can set the title of a key or add a page. The
program sends its requests to OpenCraft through a socket. This chapter shows
how to turn this on and how to use it.

A socket is a special file that two programs on one computer use to send
messages to each other. [Privacy](privacy.md#control-by-other-programs) tells
what the setting permits, and where the socket file is.

**The interface can change before version 1.0.** The commands, the names of
the fields, and the output of `opencraft ctl` can change in each release.
OpenCraft makes no promise that a program or a script that works now works
with the next version.

## Turn on the control

1. Open the Settings window. See
   [The Settings window](background.md#the-settings-window).
2. In the tab **General**, turn on **Allow control by other programs**.

[The tab General](background.md#the-tab-general) describes the switch.

## Use the command-line client

OpenCraft has a command-line client. It is the same executable as the
application. Write `opencraft ctl` and a command.

If you installed OpenCraft with Homebrew, the command `opencraft` is in your
`PATH`. See [Install with Homebrew](install.md#install-with-homebrew). If you
did not, write the full path of the executable:

```sh
/Applications/OpenCraft.app/Contents/MacOS/opencraft ctl state
```

OpenCraft must run, and the control must be on. The client does not start
OpenCraft.

```sh
opencraft ctl --help
opencraft ctl profile list
opencraft ctl surface set-label --profile work --page 0123456789abcdef \
  --surface key_face:1 --label Mute
```

- `opencraft ctl --help` lists all commands.
  `opencraft ctl <command> --help` gives the arguments and an example of one
  command.
- The result is JSON on the standard output. If the standard output is a
  terminal, the JSON has lines and indents. A problem is a text on the
  standard error.
- A profile, a page, and a folder have an identity. Read it with
  `profile list`, `page list`, and `folder list`. `page list`, `folder list`,
  and `history` take `--profile` for a profile that is not active.
- `opencraft ctl icons` lists the names of the icons that you can set on a
  surface. `--search` gives only the icons whose name or tag agrees with a
  text.
- The result of a command that changes something names the profile, the page,
  and the folder of the command. A part that the command does not have is
  `null`. A command that makes a profile, a page, or a folder gives its
  identity.
- A command that changes a page names the profile and the page. The device
  then shows that page. After its work, a program should select the page that
  you used before. See [What a program can do](#what-a-program-can-do).

The exit status of the client tells what occurred:

| Status | Meaning |
|---|---|
| 0 | The command was done |
| 1 | OpenCraft refused the command. The standard error has the cause. |
| 2 | The arguments are not correct |
| 3 | OpenCraft does not run |
| 4 | OpenCraft runs, and the control is off |
| 5 | The client and OpenCraft have different protocol versions |

For the messages of statuses 3, 4, and 5, see
[The control by other programs does not work](faults.md#the-control-by-other-programs-does-not-work).

## Install the skill

A skill is a text that tells an agent how to use a tool. The OpenCraft skill
tells an agent the commands of the client, the terms of OpenCraft, and what
an agent cannot do. The text is in the OpenCraft executable, thus it is
always correct for the version that you installed.

- `opencraft ctl skill` writes the text of the skill to the standard output.
- `opencraft ctl skill --install <directory>` writes the skill to
  `<directory>/opencraft/SKILL.md`. The directory is the skills directory
  that your agent reads, for example `~/.claude/skills`. The command
  replaces an older copy of the file.

These two commands do not need a running OpenCraft, and the control can be
off.

## Use an agent through MCP

MCP (the Model Context Protocol) is a message format for an agent. OpenCraft
has an MCP server. The server is a second client of the same socket: OpenCraft
must run, and the control must be on. The server uses the same commands and
the same texts as `opencraft ctl`.

An agent starts the server. You do not start it. The server has four tools:

| Tool | What it does |
|---|---|
| `opencraft_read` | Reads the state. It changes nothing. |
| `opencraft_change` | Changes a surface, a binding, a page, a folder, a profile, or a setting. It only adds data or changes it. |
| `opencraft_remove` | Removes a page, a folder, a profile, or an application link, or writes a profile to a file. The tool is destructive, thus the agent asks you before it runs one of its commands. |
| `opencraft_help` | Gives the arguments and an example of one command. With no command, it gives the skill. |

`opencraft_remove` does not delete a profile: `profile remove` moves the
profile to the folder `removed` in the profiles folder.

If OpenCraft does not run, or the control is off, the tool gives the text of
the problem, and the server continues.

`opencraft mcp --config` writes the installation text with the path of the
executable that you installed. `--config claude` writes the line for Claude
Code, and `--config codex` writes the line for the Codex CLI. It changes no
file of the agent.

Claude Code:

```sh
claude mcp add --scope user opencraft -- /opt/homebrew/bin/opencraft mcp
```

The Codex CLI:

```sh
codex mcp add opencraft -- /opt/homebrew/bin/opencraft mcp
```

A client that reads a JSON file (Claude Desktop, Cursor, and other clients
with `mcpServers`):

```json
{
  "mcpServers": {
    "opencraft": {
      "command": "/opt/homebrew/bin/opencraft",
      "args": ["mcp"]
    }
  }
}
```

If you did not install OpenCraft with Homebrew, write the full path of the
executable: `/Applications/OpenCraft.app/Contents/MacOS/opencraft`. A client
that a window starts does not have the `PATH` of Homebrew. Thus write the
full path.

## What a program can do

A program can read and change the configuration, and it can do the
navigation that shows a page on the device.

| Area | What a program can do |
|---|---|
| Read | The layout of the device, the kinds of action, the names of the icons, the profiles, the pages, the folders, the content of a page, the device settings, the history, and the connection |
| Surface | Set or remove the title, the title style, the fill colour, the image, and the icon |
| Binding | Set or remove the action of a gesture |
| Control | Copy, paste, clear, and copy to all pages |
| Page | Add, duplicate, rename, move, remove, and select a page. Set its first-page mark, its background, and the fit of the background. |
| Folder | Add, rename, move, remove, open, and close a folder |
| Profile | Add, rename, remove, and select a profile. Set its colour, the default profile, and its application links. Turn the automatic switch to the front application on or off. Export and import. |
| Device | Set the device settings |
| History | Undo and redo |

A change of the pages is a step of the history of OpenCraft, thus you can undo
it in the window. A change of a profile is not a step of the history: you
cannot undo an add, a rename, a removal, an import, or a selection of a
profile. [Profile settings](controls.md#profile-settings) tells which changes
of a profile you can undo.

A program cannot do these things:

- It cannot push a key, turn a knob, or swipe. It cannot make an action run,
  also not with **Test the action**. Only you can run an action when you push
  the key.
- It cannot quit OpenCraft.
- It cannot change the Settings window options, thus it cannot turn the
  control on or off.

A program can set the action of a key. For example, it can set "Run a shell
command". The action runs when you push the key. Thus read the action of a
key before you push a key that a program changed.

A change from a program shows on the device and in the window at once. A
program that selects a page or a profile changes what the device shows. While
a program works, the device can show pages that you did not select.
