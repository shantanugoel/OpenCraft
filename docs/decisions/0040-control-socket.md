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

**The target of a change.** A request that changes the content of a page
names the profile, the page, and the folder if there is one. A request about
a page names the profile and the page. A request about a folder, or about the
profiles, names the profile only. A page has an identity that stays with the
page (note of review 2, below). The control module sends one command,
`Command::AtPlace`. The engine checks the place, goes there, and obeys the
change. If the place is not there, the request fails and changes nothing. Thus
the device shows the page that the client changes. A request that changes the
pages is one step of the history; a request that changes no page is not.

Note (2026-11-16): issue #403 made these steps in the engine. A group does
not stop after a problem, thus "select the page, then change" in a group
could change a different page when the page was gone. The command
`Command::AtLocation` checked the page and the open folders, made them the
active location, and then obeyed one command. Review 2 replaced it with
`Command::AtPlace`: see the note of review 2. A request to read the content of
a location that is not active uses `Command::ReadProfile`, which gives the
profile in `Event::ProfileRead` and changes no location. A client that adds a
folder can give no identity; then OpenCraft makes one, and the result of the
request has it.

Note (2026-11-16): the result of a request is a function that the engine
thread calls after the events of the command
(`EngineSender::send_with_reply`). The read cache of the control module
follows the events on the engine thread, and the worker reads it under one
lock. Thus the response of a change comes after the events of the change,
and the next read shows the change. A problem that is a notice and not a
refusal (the device or a file fails after a correct change) is not the
result of the request.

Note (2026-11-16, review 1): these points hold for the socket.

- The worker never ends for an error of `accept` that can go away (for
  example no free file descriptor). It writes one log line, waits, and tries
  again. If it ends for another reason, it removes the socket file, and the
  next switch to "on" starts a new worker. Without a home directory, the
  socket does not start.
- A connection with no `hello` after 10 seconds closes. A connection that does
  not read a response for 10 seconds closes. The 17th connection gets the
  error `busy` and closes. `hello` reads only `version`; the text of the error
  `version_mismatch` has the two versions.
- A client that closes its write side after the requests still gets all
  responses.
- In a result, a text or the name of an object that begins with the home
  directory shows `~` in place of it. The rule is the rule of the export of a
  profile (ignore the case, act only at a path boundary). A text that has the
  home directory in the middle, for example a script, keeps it. A request can
  give `~/` in a path setting: the handler expands it.
- The wire text of a gesture and the wire form of a setting are in the control
  module. They do not depend on the file form of the profile.
- An undo or a redo with no step, and a change of the fit of a page with no
  image, are an error for a client. The window gets no problem for them. A
  refusal because the active profile changed has a text that is correct for
  the window and for a client; the window shows it as before.

Note (2026-11-16, review 2): these points hold for the changes. A client of
the command line reuses the types of `protocol.rs`.

- **One command for a place (M1, M2).** `Command::AtPlace` has a place and a
  command. The place is a profile, a page, and the open folders. The engine
  checks the profile, the page, and the folders first. It reads a profile that
  is not active from its file for this check. If one of them is not correct,
  the result is the refusal, the window gets no event, and nothing changes:
  also not the active profile. Else the engine makes the profile the active
  profile, and the page and the folders the active location, in one change.
  A request for the active place changes no location. Then the engine obeys the
  command as a step of its own, thus the name of the step is the name that the
  window uses for the same change, and an undo shows the page of the change.
  If the command gives a refusal, for example an image that does not open, the
  engine goes back to the profile and to the location of before. A request
  has no `SelectProfile` before it. `undo` and `redo` still need the active
  profile (`Command::InProfile`): the history is the history of the profile
  that the user sees.
- **A page has an identity (M4).** The protocol names a page by its identity,
  not by its number. The identity is 16 small hexadecimal digits. It stays with
  the page when the page moves, and the profile file keeps it. The reasons:
  a number names a different page after a move or a removal by the user, and
  the client gets a success. A name can be the same for two pages. The client
  reads the identities in `pages` and in `profiles` (also for a profile that is
  not active). The control module finds the number in its cache and gives both
  to the engine, which checks that the page with this number has this identity
  now. If the user moved the page in between, the request fails. The profile
  file has schema version 6 for the key `id` of a page. A file of version 5 or
  lower has no identity: the engine gives the page at place N the identity N+1
  until it writes the file again. A page that is a copy gets a new identity.
  Release 0.4.0 writes version 5 and does not read version 6. The owner
  said that no backward compatibility is necessary in this run, thus the
  engine has no way to write version 5 again.
- **The result of a change (M6).** Each change gives `{ "profile", "page",
  "folder" }`. The field of a thing that the request made has its identity:
  `profile` for `add_profile` and `import_profile`, `page` for `add_page` and
  `duplicate_page`, `folder` for `add_folder`. The other fields are null. The
  engine thread reads the identity from the cache in the function that gives the
  result, thus no other command can change it before.
- **No time limit for a change (M3).** A change waits for the engine without
  a limit: an error for a change that is done would let the client send it
  again. The worker always gets an answer: the engine drops the function at a
  stop, and the client gets an error. A read that waits for a handler keeps the
  limit of 5 seconds and the error `timeout`.
- **The forms of a request (L6, L7, M5).** The kinds, the gestures, the label
  positions, the fits, and the modes of the knob lights are enums in
  `protocol.rs` with snake case names, for example `key_face` and `turn_left`.
  The layout lists the gestures of each control, and `set_binding` refuses a
  gesture that the control cannot make. A request for a folder, for a new page,
  or for a profile names the profile only. A request about one page names the
  profile and the page, and does not name a folder. `add_page` makes an empty
  page, and `duplicate_page` makes a copy of the page of the target.
  `set_first_page` has a page target, and goes to that page, as the other
  requests about a page.
- **The response (M7).** The type of a response reads an error line as a
  failure, also when the type of the result is an `Option`. The function
  `into_result` gives a `Result`.
- **Smaller points.** A problem after a done change (the record of the active
  profile, of the default profile, or the removal of unused images) is a
  notice, not a refusal (L4). The switch of the Settings window shows the
  socket, not the setting. If the setting is on and the socket does not open,
  the switch is off and the message tells the problem (L3). The cache skips
  the events that it never uses before it takes the lock, and a read copies the
  data under the lock and makes the JSON text after it (L5). The first
  note of this record now says what the code does (L8).

Note (2026-11-16, issue #406, test 1 of the skill by an agent): these points
hold for the reads and the results.

- **A client does not change the default profile (finding 1).** A test agent
  saw `"default": "agent-test"` after `profile add`. The cause is not the
  protocol. With no default mark, the engine takes the first profile with no
  link in the order of the identities as the default profile
  ([decision 0035](0035-front-application-selects-the-profile.md), section 6),
  and the new profile was first. The window had the same effect. The engine
  now puts the mark on the default profile before an add (note of 2026-11-16 in
  decision 0035), thus no add of a client or of the window moves it. The
  active profile is a different value: each change makes the profile that it
  names the active profile (`AtPlace`), and `profile add` makes the new profile
  active. The skill says this.
- **A read takes an optional profile (finding 2).** `pages` and `folders` take
  `{ "profile": <identity> }` as `content` does. No profile, or the active
  profile: the cache answers, and `pages` has the active location. A profile that
  is not active: the engine reads its file (`Command::ReadProfile`) and
  selects nothing; the result of `pages` has `"location": null`. An identity
  that does not exist is `invalid_params`. `Pages` has the new field
  `profile`. The result of `profiles` already has the pages of each profile.
- **The result of a change is the place (finding 3).** Each change gives
  `{ "profile", "page", "folder" }`: the place that the request names (the
  target of the request), and for an add the identity of the new thing in its
  field. A part that the request has not is null. `open_folder` gives the
  folder that it opens. A request that removes a thing names it, and the thing
  is gone. This replaces the rule "null for each field of a thing that the
  request did not make" of review 2 (M6). The reason: an agent read the null
  fields as "not done".

Note (2026-11-16, review 3 and the second test of the skill by an agent).
These points change the interface of this record.

- **The history of one profile.** `history` takes `{}` or
  `{ "profile": <identity> }`, as `pages` and `folders` do, and its result has
  the field `profile`. No profile, or the active profile: the cache answers.
  The history of a profile that is not active cannot change while the profile
  is not active, thus the cache has its names from the time when it was
  active. A profile that was not active since the start has no step that
  OpenCraft knows, and the result has no names.
- **An empty history.** An undo or a redo with no step is the error
  `nothing_to_undo` or `nothing_to_redo`, and not `engine_problem`: the
  request is correct, and there is nothing to reverse. The exit status stays
  as the table of section 4 says: status 1, because OpenCraft refused the
  command. The text is the text of the engine, and the engine has it in one
  place (`NO_UNDO_STEP` and `NO_REDO_STEP`).
- **The place of an undo.** The result of `undo` and `redo` gives the place of
  the step that it reversed: the profile, the page, and the open folder that
  the device shows after the step. The other change requests keep the rule of
  the note above.
- **The page of a new profile.** The result of `add_profile` and
  `import_profile` has the identity of the first page of the new profile in
  the field `page`, because the profile has that page.
- **A folder that is not open.** `close_folder` names the folder that is open:
  the folder is a needed part of the target. The request fails with
  `invalid_params` if the profile is not active (it has no open folder) or if
  the named folder is not the last open folder of the active location. A
  previous request gave exit 0 and changed nothing.
- **The identity of a setting.** A setting of the type Profile or Folder must
  name a thing that exists: the identity of a profile, and the identity of a
  folder of the profile. A name in place of an identity is `invalid_params`.
  The engine checks the folder of a profile that is not active at the place,
  because the cache lists the folders of the active profile only.
- **A hidden field.** The check uses `shown_when` (decision 0038): a setting
  that the form does not show for the other settings is `invalid_params`. The
  limits and the key of the delay of a step come from the engine
  (`Field::step_delay`), not from a copy in the control module.
- **The status of the client.** Two more causes have their own text. A socket
  file with no answer while an OpenCraft process holds the lock is status 1:
  OpenCraft runs, and its worker ended, or the start did not remove the file.
  Status 4 says that OpenCraft can be starting. The client finds the process
  that holds the lock by the number in the lock file and the name of that
  process. A different OpenCraft process that got the same number, or
  OpenCraft at its start, can give status 4 when the setting is on.
- **The arguments of the application.** `opencraft --help`, `opencraft -h`,
  `opencraft help`, and `opencraft --version` write a text and stop with
  status 0. A first word that is not a command is an error with status 2.
  Neither starts the application, its lock, its device, or its engine. An
  argument that starts with `-` (for example `-psn_…` from the Finder) still
  does not stop the start.
- **Smaller points of the client.** `skill --install=DIRECTORY` is correct, as
  each other flag. `opencraft ctl help <command>` writes the help of the
  command. `opencraft ctl --version` writes the version of the client. A
  command with a read first checks its arguments before the connection, and
  each argument error has the line "See `opencraft ctl … --help`". The
  example of `choices` names a field that has choices (a sound device), and
  the skill says that the field `application` is a path.
- **The text of a setting that is not correct.** A setting that is null, or a
  number with a fraction, gave "The action settings are not correct.", with no
  setting named. The conversion names the setting now: "The setting \"step\"
  is not correct: it is not a whole number." The skill gives the form of a
  value of a text field that is JSON (`--setting 'text="2026"'`), because
  `--setting text=2026` is a number, and the kind refuses it.
- **The texts of the skill and of the user guide.** A change of a profile is
  not a step of the history (decision 0030), thus the skill says which
  requests are steps and which are not, and that no `profile` request has an
  undo. The skill gives the folder key ("Back"), the form of a colour, the
  run-time `status` of a kind, and the location that the device shows after a
  removal of the active profile. The user guide keeps each fact in one
  document ([the rules of the documentation](../README.md#rules)).

Note (2026-11-16, issue #425, test 3 of the skill by an agent): these points
hold for the reads and for the places.

- **The names of the icons (finding 1).** `icons` takes `{}` or
  `{ "search": <text> }`, and its result is `{ "set", "version", "icons" }`.
  The names are the names of the icon set that the build contains
  (`opencraft_engine::icon_set`). The engine draws an icon on a surface, and
  the icon picker of the window shows the same set: there is no second list.
  A search is the search of the icon picker: each word of the text is in the
  name or in a tag, and the icon that the text names comes first. The read
  needs no engine command and no cache. The help of `--icon-color` gives the
  form of a colour.
- **The places count from 1 (finding 2).** `move_page` and `move_folder` take
  the number of a place from 1, as the window numbers the pages and as the
  field "Page number" of `opencraft.page` counts. The control module gives
  the engine the number from 0, which it uses. A number that the list does
  not have is `invalid_params`: the engine moves nothing and reports no
  problem. Page 1 is the first page of `pages`, and the skill says so.
- **The skill (findings 2 to 5).** The skill says that a new profile has one
  page, that the name of a step of the history is the name that the window
  shows, that a client reads `content` before and after an undo to see the
  change, and what an empty value of a field with `default: null` does.

**One history.** A change of the pages from a client is a step of the same
history as a change from the window
([decision 0030](0030-undo-and-redo-in-the-engine.md)), thus the user can undo
it in the window. The other requests are not steps. They are the requests
about a profile, a selection of a page or a profile, a folder open or close,
`control copy`, and the device settings. Decision 0030 gives the rule
([What the history does not cover](0030-undo-and-redo-in-the-engine.md#what-the-history-does-not-cover)),
and [decision 0035](0035-front-application-selects-the-profile.md),
section 12, gives the links. A removal of a profile is not a step: OpenCraft
moves the profile directory into the folder `removed`.

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

Note (2026-11-16, issue #405): the client is in `crates/app/src/ctl/`. These
points hold.

- **One list.** `ctl/commands.rs` has the one list of the commands, `COMMANDS`.
  A command has a name of one or two words (`profile add`, `state`), the name of
  its request, a help text, an example, and its arguments. The parser, the help,
  and the tests read this list; the skill (section 5) and its test read it too.
  Each request of the protocol except `hello` has one command: a test checks
  this. The client sends `hello` by itself, with its version.
- **The arguments.** A flag has a value: `--profile work`, or `--profile=work`.
  A flag names a place in the `params` of the request. A value that the user
  does not give is not in the request, and for a request that sets or removes a
  value, no value means remove. `--setting KEY=VALUE` makes the settings of an
  action; the value is JSON if it is JSON, else text. A file path is made
  absolute by the client (`~/` and a relative path). `device set-settings` reads
  the device settings first and changes only the values that the user gives.
- **The check.** The client makes the request with the types of `protocol.rs`
  before it connects. A request that the types do not accept is status 2 and
  needs no OpenCraft. A command with a read first (`device set-settings`)
  makes its request after the read.
- **The output.** The result is JSON on one line. If the standard output is a
  terminal, the JSON has lines and indents. A problem is one text on the
  standard error. `--help` and `help` write to the standard output, status 0.
- **The status.** Status 3: the socket file does not exist and no OpenCraft
  process holds the lock, or the socket file is there and nobody serves it. Status
  4: the socket file does not exist and the lock file has the number of an
  OpenCraft process that runs. Status 5: the error `version_mismatch` of
  `hello`. Each other error of OpenCraft, a connection that closes, and a file
  that the client cannot reach are status 1.
- **The mode.** `main` runs the client before the lock, the log file, and the
  engine. Thus the client writes its messages to the terminal, and it never
  opens the device. The tests run the executable with no OpenCraft and find no
  file made.

### 5. The skill

The skill is a directory in the repository with the file `SKILL.md`. It
tells an agent the commands of the client, the terms of
[ARCHITECTURE.md](../ARCHITECTURE.md), and the limits of section 3. The
build puts the text of the skill into the executable.
`opencraft ctl skill` writes the text to the standard output.
`opencraft ctl skill --install <directory>` writes the skill into that
directory. Thus the skill is always correct for the installed version.

Note (2026-11-16, issue #406): these points hold for the skill.

- **The place.** The file is `skills/opencraft/SKILL.md` in the repository. The
  client includes it with `include_str!` (`ctl/skill.rs`). The frontmatter has
  the `name` `opencraft` and a `description`.
- **The install.** `opencraft ctl skill --install <directory>` writes
  `<directory>/opencraft/SKILL.md`, because `<directory>` is the directory of
  the skills of an agent (for example `~/.claude/skills`). It replaces an
  earlier file and writes the path to the standard output. Neither form needs
  a running OpenCraft: `skill` is not in `COMMANDS`, because it has no request.
- **The tests.** The skill has a table of the commands. A test checks that the
  table has each command of `COMMANDS` one time, that each flag of a row exists,
  and that a row has each flag that the command needs. A second test makes the
  request of each example line of the skill, with the checks of the client.
  A third test finds personal data.
- **A bad action in `binding set`.** The control module refuses a binding with
  an action kind that does not exist, a setting that the kind does not have,
  or a value that the kind refuses (issue #419). The rules come from the
  descriptions of the kinds (the input type, the limits, the options) and from
  the reader of the text of a hotkey. The same check is for the actions inside
  an action list. The refusal is `invalid_params`, and nothing changes. The
  skill tells the agent to use the full kind. A kind can still do nothing at
  run time, for example when it needs a permission: the skill tells the agent
  to read the `status` of the kind in `kinds`. `available` in `content` is not
  that test: a binding that OpenCraft accepts always has `available: true`,
  because the kind of each accepted binding exists.

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

Note (2026-11-16, issue #408): the MCP server is in the executable, and this
record needs no other change. These points hold.

- **The form.** The argument `mcp` makes the executable a server:
  `opencraft mcp`. The channel is stdio, as the transport of the Model Context
  Protocol says: one JSON-RPC message on one line, on the standard input and
  the standard output, and the messages for a person on the standard error.
  The code is `crates/app/src/mcp.rs` and `crates/app/src/mcp/` and uses
  `serde_json` only: no `rmcp`, no `tokio`. In this mode the process takes no
  lock, starts no engine, opens no device, and starts no GUI toolkit. `main`
  starts it before all other work, as it starts `ctl`.
- **Two revisions.** The server supports `2025-11-25` (the `initialize`
  handshake) and `2026-07-28` (the version in `_meta` of each request, a
  `resultType` in each result, and `server/discover`). The format of an answer
  follows the request: a request with
  `_meta.io.modelcontextprotocol/protocolVersion` gets the result of
  `2026-07-28`, and a request after `initialize` gets the result of
  `2025-11-25`. Each other request is `invalid_params`. A version that the
  server does not have is `UnsupportedProtocolVersionError` (`-32022`), with
  `data.supported`, and that list names both versions, as the example of the
  specification does. `supportedVersions` of `server/discover` names only
  `2026-07-28`, because a request of `2025-11-25` has a different format. The
  answer to `initialize` names only `2025-11-25`, because a client of that
  revision stops when the answer has a different version. The protocol facts
  are in [kb/mcp.md](../kb/mcp.md).
- **The tools.** Three tools come from `COMMANDS` when the server answers:
  `opencraft_read` (the read commands), `opencraft_change` (the other
  commands), and `opencraft_help` (the help of one command, or the skill). The
  enums, the descriptions, and the help are made from `COMMANDS`; a test
  checks that each command is in exactly one of the two tools and that the
  read tool holds exactly the read requests of `protocol.rs`
  (`Method::reads`). The arguments of a tool are the flags of `ctl` with JSON
  values: `{"profile": "work", "surface": "key_face:0"}`. The server turns
  them into the flags and calls `parse::params`, thus the kinds, the defaults,
  the checks of `protocol.rs`, and a command with a read first are the code of
  the client and not a copy.
- **The hints.** `opencraft_read` is `readOnlyHint`. A change is not
  `destructiveHint`: the user can undo a change of the pages, and the skill
  tells the agent to ask before `profile remove`. The owner keeps this choice.
  The specification gives the hint a narrower meaning, "the tool performs only
  additive updates", and `profile remove` is not additive; see
  [kb/mcp.md](../kb/mcp.md). The instructions of `initialize` and of
  `server/discover` are a short text with the rules that prevent damage;
  `opencraft_help` with no command gives the complete skill.
- **One client path.** A tool call runs `ctl::perform`, thus it opens one
  connection to the socket, sends `hello`, and gets the result or the problem
  of the client. A problem of OpenCraft (status 1, 3, 4, or 5) and a wrong
  argument of a tool are a tool result with `isError: true` and the text of
  the client. A protocol error is only for a wrong JSON-RPC message, a
  malformed `tools/call`, or an unknown tool. The server does not stop for a
  problem of OpenCraft. It waits 60 seconds for an answer, because its client
  cannot stop one request, and a timeout is a tool error that tells the model
  to read the state.
- **The installation.** The user guide gives one line for each agent, and
  `opencraft mcp --config [agent]` writes the same line with the path of the
  installed executable. It changes no file of the agent.

Note (2026-11-16, review 4 of PR #427): these points changed after the review.

- A wrong argument of a tool, including an argument that is not in `args`, is
  a tool result with `isError: true`, as the specification says. Only a wrong
  JSON-RPC message, a malformed `tools/call`, and an unknown tool are a
  protocol error.
- `instructions` is a short text with the rules that prevent damage, because
  Claude Code cuts a server instruction after 4,096 characters. The complete
  skill is the result of `opencraft_help` with no command. The descriptions of
  the tools name the commands; `opencraft_help` gives their help.
- The connection of the server waits 60 seconds for an answer, and a timeout
  is a tool error.
- `data.supported` of the version error names both versions.
- A message with no `jsonrpc`, an `id` that is not a text or a whole number,
  and `params` that are not an object are invalid. A response gets no answer
  and one line on the standard error. A line that is not UTF-8 is `-32700`.
- `opencraft mcp --config` puts a path with a character that the shell reads
  in single quotes.
- The facts of the protocol, with their sources and confidence, are in
  [kb/mcp.md](../kb/mcp.md).

Note (2026-11-16, choice 4 of the owner, issue #408): the class of a command
now selects its tool. These points of section 6 changed.

- **Four tools.** `COMMANDS` of `ctl/commands.rs` gives each command a class:
  `Read`, `Change`, or `Remove`. `opencraft_read` holds the reads,
  `opencraft_change` holds the additive changes, `opencraft_remove` holds the
  commands whose purpose is to remove data or to replace a file
  (`profile remove`, `profile unlink-app`, `page remove`, `folder remove`,
  `control clear`, and `profile export`), and `opencraft_help` gives the help.
  The class of a command is the one place that says which tool holds it, thus
  the lists of the tools cannot drift. A test checks that each command is in
  exactly one tool, that a read is exactly a read request of `protocol.rs`,
  and that each command whose name says remove, clear, or export, and
  `profile unlink-app`, is in `opencraft_remove`. A command in the wrong tool
  is a tool result with `isError: true` that names the right tool.
- **The hints.** `opencraft_remove` is `destructiveHint: true`, and
  `opencraft_change` keeps `destructiveHint: false` with the narrower meaning
  of the specification, "only additive updates"; see
  [kb/mcp.md](../kb/mcp.md). A client can thus ask the user before a removal,
  and the change tool stays a tool that a user rule can permit. The skill
  tells the agent to ask before a command of `opencraft_remove`.
- **The reasons.** The undo of the pages was the reason of the old choice, and
  it is true for a part of the commands only: `profile remove`, the links of
  an application, and the file that `profile export` replaces have no undo.
  The owner changed the choice after review 4 of PR #427 found that a client
  with `destructiveHint: false` asks no question for those commands.

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
