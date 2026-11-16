# 0040: A different program controls OpenCraft through a socket, when the user permits it

- Status: Accepted
- Date: 2026-11-15

## Context

The owner wants an agent to configure OpenCraft: a program that works for
the user, for example a coding assistant. The agent needs a tool that it can
call, and a document that tells it how to use the tool.

The facts:

- The engine holds the profile in memory and writes the file (rule 9 of
  [ARCHITECTURE.md](../ARCHITECTURE.md#5-rules)). The engine does not read
  the file again while it operates. Thus a change to `profile.toml` by a
  different program has no effect, and the next write of the engine removes
  it.
- A shell uses the engine through messages only (rule 2,
  [decision 0011](0011-engine-thread-model.md)). A second sender of the same
  messages needs no change to the rules.
- No different process can send a message to OpenCraft now, except the
  signal that opens the window
  ([decision 0028](0028-second-start-opens-the-window.md)).
  [Decision 0019](0019-engine-in-the-gui-process.md) keeps the engine in the
  process of the GUI application.
- Most commands that change a surface or a binding change the active page
  only. The engine can also change the active profile by itself, when the
  front application changes
  ([decision 0035](0035-front-application-selects-the-profile.md)).
- A binding holds an action, and an action can do work on the computer, for
  example send a hotkey or open an application. A program that can change a
  binding can thus prepare work that runs when the user pushes the key.

The owner made these choices on 2026-11-15: a socket, off by default;
configuration and navigation, with no gestures; a command-line client and a
skill first, and an MCP server later; an error when OpenCraft does not run;
a version in the protocol, with no promise of stability; the client in the
application executable; the skill in the repository and in the client.

## Decision

### 1. The control socket

**The shell owns the socket.** The code is a module of `crates/app`. It
sends engine commands through `EngineHandle`, as the window does. The
engine has no code for the socket.

**The location.** The socket is a Unix domain stream socket with the name
`control.sock`. It is in the parent directory of the profiles directory,
next to `opencraft.lock`. Its mode is `0600`. For each connection, the
shell reads the user of the other process and closes the connection if that
user is not the user of OpenCraft.

**Off by default.** The file `shell.toml` gets the setting
`allow_control`. Its default is `false`. The Settings window shows it as a
switch with the label "Allow control by other programs". While the setting
is off, the socket file does not exist and the shell has no thread for it.
A change of the setting has its effect immediately, with no new start. When
the setting goes off, the shell closes each open connection.

**The stop.** The shell removes the socket file at the stop. At the start,
the process that holds the lock removes a socket file that an earlier
process left.

### 2. The protocol

**The form.** Each message is one JSON object on one line. A request has an
`id`, a `method`, and `params`. A response has the same `id`, and a
`result` or an `error`. An error has a `code` and a text for a person.

**The version.** The first request is `hello`. It carries the protocol
version of the client. The response carries the protocol version and the
version of OpenCraft. If the two protocol versions are different, the shell
gives an error and closes the connection. The protocol version of this
record is 1.

**Its own types.** The protocol has its own request types and result types
in the control module. It does not put `Command` and `Event` of the engine
on the wire. Thus a change to an engine message cannot change the protocol
without a change to the control module, and a request that this record does
not permit has no form on the wire.

**No silent difference.** The control module has one function that names
each variant of `Command`, and one that names each variant of `Event`, with
no wildcard. Each line says that the protocol uses the variant, or that the
protocol does not permit it. Thus a new engine message does not compile
until a person selects one of the two.

**One response for each request.** A request that changes something gets
its response after the engine obeyed it. The response is a success, or it
has the problem text of the engine. The engine gets a way to give the
result of one command to its sender. The task that adds it selects the
form.

**No promise yet.** The protocol and the output of the client can change
in each release before version 1.0. The user guide says this.

### 3. What a client can do

**Read.** The layout of the device model, the kind descriptions and their
status, the choices of a field, the profiles, the pages, the folders, the
content and the bindings of a page, the device settings, the history, and
the connection state.

**Change.** A client can do these things, and no others:

| Area | Requests |
|---|---|
| Surface | Set or remove the label, the label style, the fill colour, the image, and the icon |
| Binding | Set or remove the action of a binding |
| Control | Copy, paste, clear, and copy to all pages |
| Page | Add, rename, move, remove, select; set the background and its fit |
| Folder | Add, rename, move, remove, open, close |
| Profile | Add, rename, remove, select; set the colour, the first page, and the default profile; add or remove a link to an application; turn the automatic switch on or off; export and import |
| Device | Set the device settings |
| History | Undo and redo |

**Not permitted.** A client cannot send a gesture, cannot run an action
(`Command::TestAction`), cannot stop the engine, and cannot send the
notices that only the shell knows: the power notices, the front
application, and the text edit marks. A client cannot change `shell.toml`,
thus it cannot turn the control on.

**The target of a change.** A request that changes a page names the
profile, the page, and the folder if there is one. The control module makes
that location the active location first, and sends the change in
`Command::InProfile`. If the active profile changed in that time, the
request fails and changes nothing. Thus the device shows the page that the
client changes. The steps of one request are one step of the history
(`Command::Group`).

Note (2026-11-16): issue #403 made these steps in the engine. A group does
not stop after a problem, thus "select the page, then change" in a group
could change a different page when the page was gone. The command
`Command::AtLocation` checks the page and the open folders, makes them the
active location, and then obeys one command. If the page or a folder is
gone, the engine gives a problem and the command does not run. A request
that names a location is
`InProfile(Group(AtLocation(change)))`, after a `SelectProfile` that the
control module sends first when the profile is not active. A request to
read the content of a location that is not active uses
`Command::ReadProfile`, which gives the profile in `Event::ProfileRead`
and changes no location. A client that adds a folder can give no identity;
then OpenCraft makes one.

Note (2026-11-16): the result of a request is a function that the engine
thread calls after the events of the command
(`EngineSender::send_with_reply`). The read cache of the control module
follows the events on the engine thread, and the worker reads it under one
lock. Thus the response of a change comes after the events of the change,
and the next read shows the change. A problem that is a notice and not a
refusal (the device or a file fails after a correct change) is not the
result of the request.

**One history.** A change from a client is a step of the same history as a
change from the window ([decision 0030](0030-undo-and-redo-in-the-engine.md)).
The user can undo it in the window.

**A file path.** A request with a file path, for example an image or an
export file, gives an absolute path. The engine reads or writes the file,
with the checks that it has now.

### 4. The command-line client

**The same executable.** The argument `ctl` makes the application
executable a client: `opencraft ctl <command>`. In this mode the process
does not take the lock, does not start the engine, and does not start the
GUI toolkit. It connects to the socket, sends the requests, writes the
result to the standard output, and stops.

**The output.** The output is JSON. A problem goes to the standard error
as text.

**The exit status.**

| Status | Meaning |
|---|---|
| 0 | The command was done |
| 1 | OpenCraft refused the command. The standard error has the problem text. |
| 2 | The arguments are not correct |
| 3 | OpenCraft does not run |
| 4 | OpenCraft runs, and the setting "Allow control by other programs" is off |
| 5 | The protocol version of the client is not the version of the OpenCraft that runs |

The client uses the lock file of
[decision 0028](0028-second-start-opens-the-window.md) to find the
difference between status 3 and status 4. For status 4, the process of the
number in the file runs, and the socket file does not exist. The client
does not start OpenCraft, and it does not change a profile file by itself.

**The path.** The Homebrew cask makes a link with the name `opencraft` in
the directory of the Homebrew commands. The link goes to the executable in
the bundle.

### 5. The skill

The skill is a directory in the repository with the file `SKILL.md`. It
tells an agent the commands of the client, the terms of
[ARCHITECTURE.md](../ARCHITECTURE.md), and the limits of section 3. The
build puts the text of the skill into the executable.
`opencraft ctl skill` writes the text to the standard output.
`opencraft ctl skill --install <directory>` writes the skill into that
directory. Thus the skill is always correct for the installed version.

### 6. Later

An MCP server is a second client of the same socket. It is a different
issue. It needs no change to this record if it uses the protocol of
section 2.

The reasons:

- The socket is the only option that lets a client read the live state and
  change the profile that runs, with the checks and the history of the
  engine. An import file needs a person for each change. A tool that
  changes the files needs a stop of OpenCraft for each change.
- The control module is a part of the shell, thus rules 1, 2, 9, and 12
  stay as they are. Decision 0019 said that a move to two processes changes
  the handle and not the messages. This record uses that property.
- Decision 0028 did not use a socket, because a signal was sufficient to
  open a window. A signal cannot carry a request and a response.
- The default "off" gives a user with no agent no new risk. A token gives
  little more: a process of the same user can read a token file.
- No gesture and no test of an action: a client cannot make an action run.
  A person must push the key.
- One executable has one signature, one notarization, and one version. A
  client and an application of two different versions cannot be installed
  together.
- A command-line client operates in each agent that has a shell, and a
  person can use it in a terminal and in a test.

## Results

- An agent can configure OpenCraft while it operates, and the user sees
  each change on the device and in the window.
- A client changes what the device shows, because a change is on the
  active page. An agent that works while the user uses the device makes
  the pages move. The skill tells the agent to select the first location
  again when it is done.
- With the setting on, each process of the user can change the bindings.
  The risk is the same as the risk of a process that changes the profile
  files while OpenCraft does not run. The user guide and
  [privacy.md](../user/privacy.md) say what the setting permits.
- The shell gets one more thread while the setting is on. Measure the
  footprint with the setting on, and write it in
  [kb/footprint.md](../kb/footprint.md).
- The control module must change when an engine command changes that the
  protocol uses. The compiler finds this, because the module makes engine
  commands from its own types.
- The argument `ctl` is a second argument of the executable, after
  `--login` ([decision 0021](0021-start-at-login.md)).
- [Decision 0019](0019-engine-in-the-gui-process.md) stays correct for the
  process and the lock. Its result "The application needs no code for a
  connection between processes" is not correct with this record.
- Look at this decision again for the plug-in host, for the MCP server,
  and before version 1.0, when the protocol gets a promise of stability.
