# 0030: The engine keeps the history of undo and redo, as copies of the pages

- Status: Accepted
- Date: 2026-11-13

## Context

Issue [#186](https://github.com/shantanugoel/OpenCraft/issues/186) asks for
undo and redo of each change to the profile, with ⌘Z, ⇧⌘Z, the Edit menu,
and two toolbar buttons. [Decision 0029](0029-window-layout.md) gives the
fixed parts: the toolbar buttons (section 5), the Edit menu, the focus rule,
and the names of the changes (section 13). It does not say where the
history is.

The issue gives two possible places:

1. **In the shell.** For each command, the shell makes the inverse command
   and keeps it. Undo sends the inverse command.
2. **In the engine.** The engine writes the profile
   ([rule 9](../ARCHITECTURE.md#engine)), thus it knows the state before
   and after each change.

Later tasks of the overhaul add changes that also need undo:

| Task | Change | Form |
|---|---|---|
| [#187](https://github.com/shantanugoel/OpenCraft/issues/187) | Paste, Clear, Copy to All Pages | Each is one step. Copy to All Pages changes all pages. |
| [#185](https://github.com/shantanugoel/OpenCraft/issues/185) | A drop that replaces an action | One step with the name "Drop Action" |
| [#189](https://github.com/shantanugoel/OpenCraft/issues/189), [#190](https://github.com/shantanugoel/OpenCraft/issues/190), [#191](https://github.com/shantanugoel/OpenCraft/issues/191) | Label options, icon, page background | New content of a surface or of a page |
| [#195](https://github.com/shantanugoel/OpenCraft/issues/195) | Profile import | Possibly the replacement of all pages |

## Decision

### The engine keeps the history

The history is in the engine, in `crates/engine/src/history.rs`. Undo and
redo change the active profile only. The engine keeps the history of each
other profile until the process stops (see
[A change of the active profile](#a-change-of-the-active-profile)).

A **step** keeps a copy of all pages of the profile from before the change,
the number of the active page before the change, the number of the active
page after the change, and a name for the user. The engine keeps two lists:
the undo list and the redo list.

- **A change.** For each command that can change the pages, the engine
  copies the pages, obeys the command, and compares the pages. If they are
  different, it puts a step on the undo list, and empties the redo list. A
  command that changes nothing makes no step.
- **Undo** (`Command::Undo`). The engine takes the last step of the undo
  list. It puts the pages of the step in place of the pages of the profile,
  and puts the replaced pages in the step. It puts the step on the redo
  list. The page that was active before the change becomes the active page.
- **Redo** (`Command::Redo`) does the same from the redo list to the undo
  list, and the page that was active after the change becomes the active
  page.
- An undo and a redo use the steps of a page change of
  [decision 0016](0016-pages-and-engine-requests-from-a-handler.md):
  Disappear for each binding of the old page, the images of the page, a
  write of only the images that changed, and Appear for each binding. Thus
  the device, the shell, and the handlers get the result with no new path.
  The engine writes the file after the delay of each change.
- **The event.** After each change of the lists, the engine sends
  `Event::History { undo, redo }` with the name of the last step of each
  list, or `None`. The shell needs only this event for the menu titles, the
  tooltips, and the disabled state of the buttons.
- **The limit.** Each list keeps 100 steps. The oldest step goes away first.

### How a new change joins the history

The function `step_of` in `crates/engine/src/engine.rs` gives each command
its step: a name, or `None` for a command that changes no page. Its `match`
has no wildcard. Thus a new command does not compile until it has its line:

```rust
Command::Paste { .. } => step("Paste", None),
```

That line is all the work. The engine does not need an inverse command, and
the copy of the pages contains each part of a page, also a part that a later
task adds. Thus:

| Task | Work for undo |
|---|---|
| #187 Paste, Clear, Copy to All Pages | One line for each new engine command. Copy to All Pages changes many pages in one command, and it is one step. |
| #185 Drop Action | No new engine command is necessary: the shell sends `Command::Group { name: "Drop Action", commands: vec![Command::SetBinding { .. }] }`. |
| #189, #190, #191 | A new part of `SurfaceContent` or of `Page` needs no work. A new command needs one line. |
| #195 Profile import into the active profile | One line. An import as a new profile is a change of the active profile (see below). |

**`Command::Group`** obeys a list of commands as one step with the name that
the shell gives. A shell uses it for a change of many parts that has no
engine command of its own. The engine ignores Undo, Redo, and the text edit
commands in a group.

Note (2026-11-13): in a text edit, the target of a group is the target of
its first command, if each command of the group has a target. Thus the
letters of one text field of the "Turn" card of a knob, which writes the
two turn bindings in a group for each change, are one step (issue
[#182](https://github.com/shantanugoel/OpenCraft/issues/182),
`group_target` in `crates/engine/src/engine.rs`).

A part of the profile that is not in the pages, for example the name of the
profile, is not in the copy. If a later task adds such a part and wants
undo for it, it adds the part to the step in `history.rs`.

### One edit of a text field is one step

The shell sends a change for each character that the user types. Decision
0029 says that one edit of a field is one step, from the first character to
Return or to the loss of the focus. Only the shell knows the focus, thus the
shell tells the engine:

- `Command::StartTextEdit` when a text field that edits the profile gets the
  focus.
- `Command::EndTextEdit` when the field loses the focus. At Return, the shell
  sends `EndTextEdit` and then `StartTextEdit`, because the field keeps the
  focus.

In a text edit, a change joins the last step if the step came in the same
text edit, no step came after it, and the change has the same target: the
label of the same surface on the same page or in the same folder, the name
of the same page or of the same folder, or the action of the same binding.
A key face is in the last open folder ([decision 0036](0036-folders.md)),
thus the title of key 1 on the page and the title of key 1 in a folder are
two targets. Each other change starts a new step. If the
user types text and removes it again, the step goes away. Out of a text
edit, each change is a step. Thus a click, for example on a toggle, is
always a step of its own.

### A change of the active profile

Amended on 2026-11-14 (issue
[#349](https://github.com/shantanugoel/OpenCraft/issues/349)). Before this
change, each change of the active profile emptied both lists. With the
automatic switch of [decision 0035](0035-front-application-selects-the-profile.md),
a trip to a different application then lost the undo of a removal, and
[decision 0036](0036-folders.md), section 10, needs that undo.

- At each change of the active profile, the engine puts the two lists of the
  old profile aside, with its identity. If it has lists for the new
  profile, it gives them back. Then it sends `Event::History` with the names
  of the lists of the new profile. This is the same for a manual selection,
  a "Select a profile" key, and an automatic switch.
- The engine keeps the lists of at most 8 profiles other than the active
  profile (`KEPT_HISTORIES`). The lists of the profile that the user left
  first go away first. Each list keeps its limit of 100 steps.
- When the engine loads a profile, it removes the image files that the
  profile does not use, as before. It keeps each image that a kept step of
  that profile uses, so that an undo finds the image.
- A profile that the user removes loses its lists. A new profile (an add, a
  duplicate, an import, or a starter profile) has no lists, also when it
  gets the identity of a removed profile. A profile file that the engine
  cannot use (a backup of decision 0009) loses its lists.
- The lists stay in memory only. A restart empties them.
- The state of a text edit belongs to the shell, thus it does not change.
  The next change after a change of the profile is a new step.
- A change outside the pages of a profile that is not active (a link, the
  colour, the name) does not change its kept lists. A hand edit of the file
  while the profile is not active is not in the lists: an undo puts back
  the pages of the step.

### What the history does not cover

- **The profiles.** Add, rename, remove, and select a profile are not in the
  history. These commands change more than one file. A change of the active
  profile keeps the lists of each profile (see the section above). A rename
  of the active profile stays after an undo.
- **The device settings.** They are not in the profile
  ([decision 0022](0022-device-settings-and-stop-state.md)).
- **A test of an action.** `Command::TestAction` runs the action of a
  binding as a gesture does ([#188](https://github.com/shantanugoel/OpenCraft/issues/188)).
  It changes no page, thus it is not a step. Its line in `step_of` gives
  `None`.
- **A restart.** The history is in memory only.
- **A page change.** It changes no page content. An undo shows the page of
  the change.

### The window

The window parts are in `crates/app/src/view/history.rs`, the toolbar
buttons in `view/toolbar.rs`, and the Edit menu in `app_menu.rs`. They obey
decision 0029, sections 5 and 13. The names of the steps are those of
section 13, and also "Remove the Image" for an image that the user removes.
The label options of
[#189](https://github.com/shantanugoel/OpenCraft/issues/189) add "Set Title
Position", "Set Title Size", "Set Title Colour", and "Set Title Weight". The
profile settings of [#195](https://github.com/shantanugoel/OpenCraft/issues/195)
add "Set First Page": the mark of the first page is on a page, thus it is in
the copy of the pages. The page background of
[#191](https://github.com/shantanugoel/OpenCraft/issues/191) adds "Set
Background", "Remove the Background", and "Set Background Fit"
([decision 0032](0032-page-background.md)). The icon of a surface of
[#190](https://github.com/shantanugoel/OpenCraft/issues/190) adds "Set
Icon" (a new icon or a new icon colour) and "Remove the Icon" ("No icon",
which also removes an image); the icon is in the content of the surface,
thus the copy of the pages has it ([decision 0033](0033-surface-icons.md)).
The colour of a profile is not in the pages, and it has no step, as the name. An import adds a profile and makes it the active
profile, thus it empties the history. The
editors of a toggle and a sequence of
[#183](https://github.com/shantanugoel/OpenCraft/issues/183) send
`Command::Group` with the names "Add Step", "Move Step", "Remove Step", and
"Remove the Action" (a contained action). Before a change, they end the open
text edit, so that the change does not join the step of the text. The
names are in `step_of` in `crates/engine/src/engine.rs`. The toolbar button ends an open text edit first, and then sends Undo or Redo
to the engine.

## Rejected alternatives

| Alternative | Reason |
|---|---|
| The history in the shell, with an inverse command for each command | The shell must know the state before each change, also for the pages that it does not show: the shell holds only the bindings of the active page. Each new command needs its inverse command, and Copy to All Pages and an import need many. A second shell must do all of this again. Rule 12: the shell contains no logic of this kind. |
| An inverse command in the engine for each command | It is less data than a copy, but each new command needs its inverse, and an error in one inverse damages the profile. The copy is correct for each command with no special work. |
| One step for each character, joined by time | Decision 0029 says that one edit of a field is one step. A time limit makes a slow typist get many steps. |
| Empty the history at each change of the active profile (the first version of this record) | An automatic switch occurs each time the user goes to an application with a different profile. The user then loses the undo of a change, also of a removal that only undo can repair. The image problem has a solution: the load keeps the images of the kept steps. |
| Keep one history for all profiles, with the identity in each step | An undo could then change a profile that is not active, or select a different profile. The window shows only the active profile. |
| No limit on the number of kept profiles | The memory grows with each profile that the user edits in a long session. 8 profiles of 100 steps is enough for a trip between some applications. |

## Results

- Each change of the pages has undo and redo with no special code. A new
  command needs one line, and the compiler asks for it.
- The memory cost is one copy of the pages for each step. A page holds text,
  colours, image paths, and action settings, not image data. The profile
  file of a real page with seven keys, their actions, and a bar panel is
  1.7 KB. The copy in memory is of the same order (an inference, not a
  measurement). Thus 100 steps of a profile with 10 pages are a few
  megabytes at the most. The kept lists of at most 8 more profiles
  multiply that by 9 at the most. Look at this decision again if a profile
  becomes much larger, for example with many pages of folders. Then a step
  can keep only the pages that changed.
- An undo draws the active page again. Only the images that changed go to
  the device.
- A shell must send `StartTextEdit` and `EndTextEdit` for each text field
  that edits the profile. If it does not, each character is one step: the
  result is less good, but correct.
- No rule of [ARCHITECTURE.md](../ARCHITECTURE.md) changes.
