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

The history is in the engine, in `crates/engine/src/history.rs`. It is for
the active profile only.

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
label of the same surface on the same page, the name of the same page, or
the action of the same binding. Each other change starts a new step. If the
user types text and removes it again, the step goes away. Out of a text
edit, each change is a step. Thus a click, for example on a toggle, is
always a step of its own.

### What the history does not cover

- **The profiles.** Add, rename, remove, and select a profile are not in the
  history. These commands change more than one file. A change of the active
  profile, also by a "Select a profile" action, empties both lists. The
  reason: the steps are copies of the pages of one profile, and the engine
  removes the unused image files of a profile when it loads it. A rename of
  the active profile stays after an undo.
- **The device settings.** They are not in the profile
  ([decision 0022](0022-device-settings-and-stop-state.md)).
- **A restart.** The history is in memory only.
- **A page change.** It changes no page content. An undo shows the page of
  the change.

### The window

The window parts are in `crates/app/src/view/history.rs`, the toolbar
buttons in `view/toolbar.rs`, and the Edit menu in `app_menu.rs`. They obey
decision 0029, sections 5 and 13. The names of the steps are those of
section 13, and also "Remove the Image" for an image that the user removes.
The toolbar button ends an open text edit first, and then sends Undo or Redo
to the engine.

## Rejected alternatives

| Alternative | Reason |
|---|---|
| The history in the shell, with an inverse command for each command | The shell must know the state before each change, also for the pages that it does not show: the shell holds only the bindings of the active page. Each new command needs its inverse command, and Copy to All Pages and an import need many. A second shell must do all of this again. Rule 12: the shell contains no logic of this kind. |
| An inverse command in the engine for each command | It is less data than a copy, but each new command needs its inverse, and an error in one inverse damages the profile. The copy is correct for each command with no special work. |
| One step for each character, joined by time | Decision 0029 says that one edit of a field is one step. A time limit makes a slow typist get many steps. |
| Keep the history across a change of the profile | The steps are for the pages of one profile. The engine removes the unused images when it loads a profile, thus a step could name an image that is gone. |

## Results

- Each change of the pages has undo and redo with no special code. A new
  command needs one line, and the compiler asks for it.
- The memory cost is one copy of the pages for each step. A page holds text,
  colours, image paths, and action settings, not image data. The profile
  file of a real page with seven keys, their actions, and a bar panel is
  1.7 KB. The copy in memory is of the same order (an inference, not a
  measurement). Thus 100 steps of a profile with 10 pages are a few
  megabytes at the most. Look at this decision again if a profile
  becomes much larger, for example with many pages of folders. Then a step
  can keep only the pages that changed.
- An undo draws the active page again. Only the images that changed go to
  the device.
- A shell must send `StartTextEdit` and `EndTextEdit` for each text field
  that edits the profile. If it does not, each character is one step: the
  result is less good, but correct.
- No rule of [ARCHITECTURE.md](../ARCHITECTURE.md) changes.
