# 0036: A folder is a page of keys outside the page cycle, with the actions "Open a folder" and "Back"

- Status: Accepted
- Date: 2026-11-14

## Context

Phase 9 ([#201](https://github.com/shantanugoel/OpenCraft/issues/201)) adds
folders (PAGE-05 of the feature tables of
[#198](https://github.com/shantanugoel/OpenCraft/issues/198)). The owner
used a folder key in VSD Craft. Decision 7 of the calibration in #198, §19,
recommends this: a folder is a page outside the page cycle, with the actions
Open and Back. At stake are the schema, the two levels of the sidebar
([#177](https://github.com/shantanugoel/OpenCraft/issues/177)), and the
later import of VSD Craft folders (Phase 12).

The facts about VSD Craft folders are in
[kb/vsd-craft.md](../kb/vsd-craft.md#scenes-folders-and-the-switch-by-application-2026-11-14).
In short: "Create Folder" turns a key into a folder, key 1 of the folder is a
fixed "Go back" key, a folder in a folder works, a folder is a child page
with its own identifier, and a drop on a folder key deleted the folder with
no question.

The present code:

- The engine holds the number of the active page. `opencraft.page` asks for
  the next page, the previous page, or a page number, and the cycle wraps
  around ([decision 0016](0016-pages-and-engine-requests-from-a-handler.md)).
- `BindingId` is `{ page, control, gesture }`. The page number face of
  `opencraft.page` shows `binding.page + 1`.
- The engine does not read the settings of an action (rule 8), thus it
  cannot find the keys that open a folder.
- The undo history keeps copies of all pages and the active page
  ([decision 0030](0030-undo-and-redo-in-the-engine.md)).
- The sidebar has two levels: profiles, and the pages of the active
  profile ([decision 0029, section 7.3](0029-window-layout.md#73-the-profiles-section)).
  The Apple guidelines allow no more levels there.

[Decision 0035](0035-front-application-selects-the-profile.md) makes
profile schema version 5. This record adds its folder keys to the same
version.

## Decision

### 1. Words

| Word | Meaning |
|---|---|
| Folder | A set of key faces and key bindings in a profile, with an identity and a name. It is not in the page cycle. A key opens it. |
| Page cycle | The pages of the profile, in their order. "Next page" and "Previous page" go through them. A folder is not in it. |
| Open folders | The list of the folders that are open, from the first to the last. The device shows the last one. The list is empty when no folder is open. |
| The page under the folders | The page of the page cycle that was active when the first folder opened. It stays the active page while folders are open. |
| Location | The active page and the open folders |

### 2. The folders in the profile file

A profile file of schema version 5 can have the array of tables `folders`:

| Key | Value | Rule |
|---|---|---|
| `id` | The identity of the folder: 16 hexadecimal digits with small letters, for example `"9f2c4e1a7b3d5c60"` | Necessary. One time in the profile. Not `"0000000000000000"`. |
| `name` | The name for the user | Necessary. Not only spaces. Two folders can have the same name, as two pages can. |
| `surfaces` | As a page, but only key faces: `key-face-N` | Optional |
| `bindings` | As a page, but only keys: `key-N` | Optional |

A folder has no `background` and no first page mark. A `surfaces` key or a
`bindings` key of a different kind in a folder is an error of the file
(decision 0009: the engine keeps the file as a backup). The order of the
array is the order of the folders in the sidebar.

```toml
schema_version = 5
name = "Streaming"
model = "n4-pro"

[[pages]]
name = "Page 1"
[pages.surfaces.key-face-3]
label = "OBS"
icon = "folder"
[pages.bindings.key-3.press]
kind = "opencraft.open-folder"
[pages.bindings.key-3.press.settings]
folder = "9f2c4e1a7b3d5c60"

[[folders]]
id = "9f2c4e1a7b3d5c60"
name = "OBS"
[folders.surfaces.key-face-1]
label = "Back"
icon = "corner-up-left"
[folders.bindings.key-1.press]
kind = "opencraft.close-folder"
```

The version, the migration, and the tests are in
[decision 0035, section 13](0035-front-application-selects-the-profile.md#13-schema-version-5).
A file of version 4 or lower has no folders.

**In memory** (`crates/engine/src/profile.rs`):

```rust
/// The identity of a folder. The text form is 16 hexadecimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FolderId(u64);

impl FolderId {
    /// A new identity: a random number that is not 0. It uses
    /// `std::collections::hash_map::RandomState` and needs no new crate.
    pub fn random() -> Self;
}

pub struct Folder {
    pub id: FolderId,
    /// The name of the page is the name of the folder. The page has no
    /// background, is never the first page, and has only key faces and keys.
    pub page: Page,
}

pub struct Profile {
    // ... the present fields, and the links of decision 0035
    folders: Vec<Folder>,
}
```

`Profile` gets `folders()`, `folder(id)`, `folder_mut(id)`,
`add_folder(folder)`, `move_folder(id, to)`, `remove_folder(id)`,
`new_folder_name()` ("Folder *N*", as `new_page_name`), and
`replace_folders(folders)` for the undo history. `map_images` and
`image_uses` include the folders, thus the export, the import, and the
removal of unused images cover them.

### 3. The location in the engine

The engine keeps `active: usize`, the page under the folders, as now, and a
new `open: Vec<FolderId>`. None of them is in a file.

- **The page of a control.** For a key and a key face, it is the last open
  folder, or the active page if no folder is open. For each other control
  and surface (a knob, a bar panel, the touch bar, the background), it is
  always the active page.
- **`BindingId`** gets `folder: Option<FolderId>`. For a key binding while a
  folder is open, it is the last open folder. Else it is `None`. `page` is
  always the active page. Thus the page number face of `opencraft.page`
  shows the number of the page under the folders, with no change to the
  handler. A command with a `BindingId` that is not the binding of the
  location gives a problem and changes nothing, as now.
- **Open a folder** (`EngineRequest::OpenFolder` and `Command::OpenFolder`):
  - If the folder does not exist, the engine sends the problem "The folder
    of this key does not exist. Select a folder for the key." and changes
    nothing.
  - If the folder is the last open folder, nothing changes.
  - If the folder is in the list at an earlier position, the engine removes
    the folders after it. Thus the list has each folder one time at the
    most, and the depth is at most the number of folders. A key in folder B
    that opens folder A, which opened B, goes back to A.
  - Else the engine adds the folder at the end of the list.
- **Back** (`EngineRequest::CloseFolder` and `Command::CloseFolder`)
  removes the last open folder. With no open folder, nothing changes. Back
  returns to the place that showed before the folder: the folder before it
  in the list, or the page under the folders.
- **A change of the location** uses the steps of a page change of decision
  0016, for the keys only: Disappear for each key binding of the old
  location, the images of the key faces, a write of only the images that
  changed, and Appear for each key binding of the new location. The knobs,
  the bar panels, the touch bar, and the background do not change, thus
  their bindings get no Disappear and no Appear.

### 4. The pages and the folders

| Request or command | In a folder |
|---|---|
| "Next page", "Previous page" (`PageTarget::Next`, `Previous`) | The engine closes all folders and selects the page after or before the page under the folders. The cycle wraps around as now. A folder is never a target. |
| "Go to page" (`PageTarget::Number`), `Command::SelectPage` | The engine closes all folders and selects the page. If it is the page under the folders, only the folders close. |
| `Command::AddPage`, `RemovePage` of the page under the folders | The active page changes as now, and all folders close |
| `Command::MovePage` | The active page keeps its page, possibly with a new number. The folders stay open. |
| A change of the active profile (also the automatic switch of decision 0035) | All folders close. The new profile shows its first page. |
| `Command::CopyToAllPages` | It copies to the pages of the page cycle only, never to a folder. With a folder open, it gives the problem "Copy to All Pages is not possible in a folder." The window disables the item. |
| `PasteControl`, `ClearControl`, `SetLabel` and the other changes of a surface or a binding | They change the page of the control (section 3). In a text edit, the target of `SetLabel` on a key face holds the last open folder, thus a title on the page and a title in a folder are two steps ([decision 0030](0030-undo-and-redo-in-the-engine.md#one-edit-of-a-text-field-is-one-step)). |

### 5. Knobs, bar panels, and the touch bar in a folder

A folder has keys only. In a folder, the knobs, the bar panels, the swipes
of the touch bar, and the background are those of the page under the
folders, and they operate as on that page. A knob that changes the volume
thus also changes it in a folder. An opening of a folder writes only key
faces to the device.

A folder on the touch bar (KNOB-09) is not in this phase. A bar panel tap
or a knob can have the action "Open a folder": it opens a folder of keys.

### 6. The actions

Two new kinds in `crates/actions/src/flow.rs`, in the group "Navigation":

| Kind | Name | Fields | Icon | Request |
|---|---|---|---|---|
| `opencraft.open-folder` | "Open a folder" | `folder`: label "Folder", the new input type `InputType::Folder` | `folder` | `EngineRequest::OpenFolder(FolderId)` |
| `opencraft.close-folder` | "Back" | None | `corner-up-left` | `EngineRequest::CloseFolder` |

- `InputType::Folder` is an amendment of
  [decision 0014](0014-action-settings-description.md). The value is the
  text form of a `FolderId`. The shell shows a popup button with the
  folders of the active profile from `Event::Pages`, and the item "New
  Folder" (section 8). A value that is not a folder stays, and the popup
  shows "(not found)".
- A missing or incorrect `folder` gives the setting problem of decision
  0014. The handler does not know the folders. The engine checks the
  identity.
- `KindDescription` gets `folder_back: bool`. It is true only for
  `opencraft.close-folder`. The shell uses it for the default Back key of
  section 8 and for the help text of section 9, and compares no kind
  identifier (decision 0029, section 20).
- Both kinds can be in a toggle and in a sequence. They make the same
  requests there.

### 7. Commands and events

New commands in `crates/engine/src/engine.rs`. Each command that changes a
folder is one step of the history. `step_of` gets these lines:

| Command | Result | Step |
|---|---|---|
| `AddFolder { id: FolderId, name: Option<String>, copy_of: Option<FolderId> }` | Adds a folder at the end of the list, with the name, or "Folder *N*" with `None`. With `copy_of`, the new folder has the key faces and the bindings of that folder. Then the engine opens the new folder (section 3). An `id` that exists, or a `copy_of` that does not exist, gives a problem and changes nothing. | "Add Folder", or "Duplicate Folder" with `copy_of` |
| `RenameFolder { folder: FolderId, name: String }` | The folder gets the name. Only spaces give a problem. | "Rename Folder". In a text edit, the target is the name of the same folder. |
| `MoveFolder { folder: FolderId, to: usize }` | The folder gets the position `to` in the list | "Move Folder" |
| `RemoveFolder { folder: FolderId }` | The folder goes away. If it is open, the engine closes it and the folders after it. | "Remove Folder" |
| `OpenFolder { folder: FolderId }` | Section 3 | None |
| `CloseFolder` | Section 3 | None |

The shell makes the identity of a new folder with `FolderId::random()`,
before it sends the command. Thus one group can add a folder and bind a key
to it (section 8). This is the same as the opaque identifier that the shell
gives in `Command::Choices`.

**The history.** A step keeps the location before and after the change,
not only the active page, and the copy of the pages also holds the folders.
An undo and a redo restore the location. If a folder of the restored list
does not exist, the list ends before it. A change of the active profile
closes the folders, but it keeps the history of the profile for the
session ([decision 0030](0030-undo-and-redo-in-the-engine.md#a-change-of-the-active-profile)).
Thus an undo after a trip to a different profile and back restores a
removed folder.

**`Event::Pages`** gets two fields. The engine sends the event after each
change of the folders and of the location:

```rust
Pages {
    names: Vec<String>,
    active: usize,
    first: usize,
    /// The folders of the profile, in their order.
    folders: Vec<FolderEntry>, // FolderEntry { id: FolderId, name: String }
    /// The open folders, from the first to the last. Empty: no folder is open.
    open: Vec<FolderId>,
}
```

After it, the engine sends `Event::Binding` for each binding of the
location: the keys of the last open folder, and the other controls of the
active page.

### 8. A new folder and its Back key

A new folder has a Back key on key 1, as in VSD Craft. The user can change
it or remove it.

The engine makes no binding of a kind (rule 8). Thus the shell sends one
group with the name "New Folder" for each new folder:

1. From the folder popup of an "Open a folder" form: first `SetBinding` of
   the form, with the new identity in its `folder` field.
2. `AddFolder { id, name: None, copy_of: None }`. The new folder opens.
3. `SetBinding` for the press of key 1 of the new folder, with the kind
   that has `folder_back`, if a kind has it.
4. `SetLabel` "Back" and `SetIcon` with the icon of that kind on key face 1.

"Key 1" is the key with the number 1 of the layout. A model with no key
gets no Back key. The sidebar row "Add a folder" sends the group with steps
2 to 4. One undo removes all of it.

### 9. The window

- **The sidebar** keeps two levels. Under the active profile, the rows are:
  the page rows, the folder rows, "Add a page", and "Add a folder". A
  folder row is a page row with the icon `folder` 15 in place of the page
  number, and the folder name. All folders are at level 2, also a folder
  that a different folder opens: the file has no tree, thus the sidebar
  needs no third level. When a folder is open, the row of the last open
  folder is the selected row, and the page rows show no selection. A click
  on a folder row sends `OpenFolder`. A click on a page row sends
  `SelectPage`, which closes the folders. The right-click menu of a folder
  row is: "Rename", "Duplicate", "Move Up", "Move Down", separator,
  "Remove…".
- **The page header** of the canvas shows the path, for example
  "Page 1 › OBS › Mics", in the place of
  [decision 0029, section 21](0029-window-layout.md#21-places-kept-for-later).
  The page name in the path is a button that closes all folders.
- **The canvas** shows the key faces of the last open folder, and the knobs
  and the touch bar of the page under the folders.
- **The page inspector** (#181) for a folder: the header "Folder
  “*name*”" and the second line "Folder · Profile “*name*”". The section
  "Folder" has "Name", "Order", "Duplicate", and "Remove…". It has no
  section "Background" and no section "Touch bar gestures". The help text
  is "The knobs, the touch bar and the background are those of page
  “*name*”." If no binding of the location has a kind with `folder_back`,
  the inspector shows: "This folder has no Back key. On the device,
  “Next page” or a different profile closes it."
- **The confirmation of "Remove…"** says: "Remove the folder “*name*”? Keys
  that open it do nothing after this. You can undo it."
- **The library** shows "Open a folder" and "Back" in "Navigation". This
  uses the place "Open a folder" of decision 0029, section 21.

### 10. A folder that the user removes

The keys that open a removed folder stay as they are, because the engine
does not read their settings (rule 8). A press gives the problem of section
3, and the form shows "(not found)". An undo puts the folder back with the
same identity, and the keys operate again. Thus a removal loses no key, and
the history can repair it. The undo stays after an automatic switch to a
different profile and back, and after a manual selection (issue
[#349](https://github.com/shantanugoel/OpenCraft/issues/349)). Only a
restart, the removal of the profile, or more than 8 other profiles with a
history remove it. VSD Craft deleted a folder with no question; this
rule prevents that.

### 11. Other states

| State | Result |
|---|---|
| A reconnection, a wake from sleep, or the end of the dark state (decision 0034) | The location stays. The engine draws all surfaces of the location again. |
| The device is dark, or no device is connected | A command changes the location in memory. The device shows it at the next draw. A key cannot open a folder: decision 0034 consumes the first gesture. |
| A start of OpenCraft | No folder is open. The device shows the first page of the profile. |
| A change of the active profile | All folders close (section 4) |

### 12. The export, the import, and VSD Craft folders

- **The export file** ([decision 0031](0031-profile-export-file.md)) holds
  the profile table with `folders`. The identity of a folder is for its
  profile only, thus the keys that open a folder operate after an import on
  a different computer. The images of folder key faces go into the file as
  the other images.
- **A starter profile and a profile of PLUG-31** can have folders.
- **The import of VSD Craft scenes** (Phase 12) can map each VSD Craft
  folder (a child page with a `ProfileUUID`) to one folder with a new
  `FolderId::random()`, each "Create Folder" action to
  `opencraft.open-folder` with that identity, and the fixed "Go back" key to
  `opencraft.close-folder` on key 1. A folder in a folder needs no tree,
  because each folder is in the flat list. A folder action with a missing
  child page (4 of 16 in the analysed data) becomes a key with a missing
  folder, and the import reports it. The import of a VSD Craft folder with
  more than one page is not decided: the facts are not known.

### 13. The tasks

[#339](https://github.com/shantanugoel/OpenCraft/issues/339) (PAGE-05) does
all of this record: the schema keys with the migration of decision 0035,
the engine, the two kinds, the window, the user guide, and the device check.
Decision 0029 and decision 0014 get a note in the same pull request.

## Rejected alternatives

| Alternative | Reason |
|---|---|
| A folder belongs to one key, as in VSD Craft | A drop on the key or a clear of the key would delete the folder and its keys. Two keys cannot open one folder. The engine must read the settings of the key (rule 8). |
| A folder is a page with a mark in the list `pages` | Each page number after it changes when the user adds a folder. "Next page", the first page, and the page number face must skip it. A separate list keeps the page cycle as it is. |
| A folder is a full page with its own knobs, bar panels, background, and swipes | The user must set the knobs again in each folder, or the volume knob stops in a folder. A folder of keys writes fewer images and matches VSD Craft. Touch bar folders are out of the phase. |
| A folder as a tree in the file (a folder holds folders) | The sidebar has two levels at the maximum. A flat list with keys that open folders gives nested folders with no tree. |
| An index in the list as the reference of a key | A move or a removal of a folder changes the index, and the key opens a different folder. |
| The name as the reference of a key | A rename breaks the key, and two folders can have one name. |
| The engine makes the identity, and the shell binds the key after an event | The add and the binding are then two steps of the history, and the shell must wait for an event. |
| No default Back key | A user who forgets the key cannot leave the folder on the device. |
| A fixed Back key that the user cannot change (VSD Craft) | It takes one of the keys from the user. The engine would need code for a fixed control. |
| "Next page" in a folder goes to the next folder | A folder is not a page of the cycle. The user expects "Next page" to show a page. |
| Keep the folders open after a change of the profile | The folders of the old profile do not exist in the new profile. |
| A limit on the depth of the open folders | The rule "a folder is at most one time in the list" gives a limit: the number of folders. |

## Results

- A key opens a folder and Back returns to the place before it. "Next page"
  does not go into a folder.
- An opening of a folder writes only key faces. Thus it is faster than a
  page change with a background.
- The page numbers, `first_page`, and the page cycle do not change for a
  profile with no folders.
- The undo history covers each change of a folder, and an undo of a removal
  repairs the keys that open it. The copy of a step is larger by the
  folders. Decision 0030 says to look again at its memory cost if a
  profile with many folders becomes large.
- A key that opens a removed folder stays, and gives a clear problem.
- The sidebar keeps two levels, also with folders in folders.
- `BindingId` gets one field. Each place that makes a `BindingId` must set
  it: the compiler finds them.
- No rule of [ARCHITECTURE.md](../ARCHITECTURE.md) changes. The kinds are
  handlers that send requests (rule 8). The engine holds the location
  (rule 10).
- Decision 0016 stays: this record adds the folders to the location. Look
  at this decision again for touch bar folders (KNOB-09).
