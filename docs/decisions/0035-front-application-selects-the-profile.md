# 0035: The shell reports the front application, and the engine selects the linked profile

- Status: Accepted
- Date: 2026-11-14

## Context

Phase 9 ([#201](https://github.com/shantanugoel/OpenCraft/issues/201)) adds
two rows of the feature tables of
[#198](https://github.com/shantanugoel/OpenCraft/issues/198):

- **AUTO-01.** The device shows the profile that is linked to the front
  application. When no link matches, it shows the default profile.
- **ACT-26.** The action "Select a profile" names a profile by a stable
  identity, not by its name. It also selects the next or the previous
  profile, and it can open the profile at a page.

Decision 3 of the calibration in #198, §19, recommends this: the shell
observes the notices of `NSWorkspace` and sends an engine command, the
engine selects the profile, and the profile stores the bundle identifier.
The record must give a clear rule for a clash, for the default profile, and
for a manual choice against an automatic switch. The phase issue names that
last point as the main risk.

The facts about VSD Craft are in
[kb/vsd-craft.md](../kb/vsd-craft.md#scenes-folders-and-the-switch-by-application-2026-11-14).
In short: a scene links up to three applications by their bundle paths, one
scene is the default, the switch comes only when the front application
changes, and the switch takes 0.47 s to 1.10 s. The facts about macOS are in
[kb/macos-front-application.md](../kb/macos-front-application.md).

The present code:

- The identity of a profile is the name of its directory
  ([decision 0018](0018-many-profiles.md)). The engine makes it from the
  first name, and does not change it at a rename.
- `state.toml` holds the active profile of each model. The engine writes it
  at each change of the active profile.
- `EngineRequest::SelectProfile { name }` selects a profile by its name for
  the user. A rename breaks the key.
- The shell observes the power notices of `NSWorkspace` in
  `crates/app/src/power.rs`, and sends `Command::PowerNotice`
  ([decision 0034](0034-device-dark-state.md)).
- Release 0.2.0 writes profile schema version 4. Thus Phase 9 cannot add
  keys to version 4 (see the comment at the top of `crates/engine/src/store.rs`).

## Decision

### 1. Words

| Word | Meaning |
|---|---|
| Front application | The application that macOS shows as active: its menu is in the menu bar. |
| Link | A bundle identifier in a profile. The profile is for that application. |
| Default profile | The profile that the device shows when the front application has no link. Each device model has one. |
| Automatic switch | The change of the active profile that the engine does when a different application comes to the front. The user can turn it off. |
| Manual selection | Each other change of the active profile: the window, the menu bar item, a "Select a profile" key, and the add, import, and remove of a profile. |

### 2. The shell observes the front application

The shell code is in a new file `crates/app/src/front_app.rs`, next to
`power.rs`. It uses the pattern of `power.rs`:

- At the start of the application (not at the open of the window), the
  shell adds an observer for `NSWorkspaceDidActivateApplicationNotification`
  to `NSWorkspace.sharedWorkspace.notificationCenter`. The observer stays
  until the application ends. It runs also when no window is open (rule 3).
- For each notice, the shell reads the `NSRunningApplication` in the key
  `NSWorkspaceApplicationKey` of `userInfo`, and sends
  `Command::FrontApplication`:
  - `FrontApplication::Shell` if its `processIdentifier` is the identifier
    of the OpenCraft process (`std::process::id()`). The shell compares the
    process, not the bundle identifier, because a development build has no
    bundle.
  - `FrontApplication::Bundle(id)` with its `bundleIdentifier`.
  - `FrontApplication::NoBundle` if `bundleIdentifier` is `nil`.
- One time at the start, right after the start of the engine, the shell
  sends the same command for `NSWorkspace.sharedWorkspace.frontmostApplication`.
- The shell sends each notice. It does not compare, filter, or delay them.
  It does not read the setting of section 7. All policy is in the engine
  (rule 12), and a test of the engine covers it with injected commands.
- The watcher does not poll and starts no thread and no timer. Between two
  changes of the front application it uses no processor time.

The engine type is in a new module `crates/engine/src/front.rs`:

```rust
/// The front application, as the shell reports it.
pub enum FrontApplication {
    /// An application with this bundle identifier, for example `com.apple.Safari`.
    Bundle(String),
    /// An application with no bundle identifier.
    NoBundle,
    /// The process of the shell itself.
    Shell,
}
```

### 3. The identity of a profile stays the directory name

A link and a "Select a profile" key use the identity of
[decision 0018](0018-many-profiles.md): the name of the profile directory.
The profile file gets no identifier.

- A rename changes only `name` in the file. The directory name does not
  change. Thus a link and a key stay correct after a rename.
- A link is in the file of its profile (section 4), thus the link has no
  reference to an identity. Only the key of section 10 and `state.toml` use
  the identity.
- A duplicate and an import get a new directory, thus a new identity. This
  is correct: they are new profiles.
- The export file has no identity ([decision 0031](0031-profile-export-file.md)).
  A "Select a profile" key in an exported profile that names a different
  profile can find no profile on a different computer. Then the key gives
  the problem of section 10, and the user selects the profile again.
- If the user removes a profile and later adds a profile with the same
  name, the new profile can get the old identity. A key for the old profile
  then selects the new profile. This is acceptable: the user gave the same
  name.

### 4. The links are in the profile file

A profile file of schema version 5 can have the array of tables
`applications`. Each table is one link:

| Key | Value | Rule |
|---|---|---|
| `bundle_id` | The bundle identifier, for example `"us.zoom.xos"` | 1 to 255 characters: ASCII letters, digits, `-`, and `.`. Necessary. |
| `name` | The name of the application for the user at the time of the link, for example `"zoom.us"` | Not empty. Necessary. The window shows it if the application is not installed. |

```toml
schema_version = 5
name = "Calls"
model = "n4-pro"

[[applications]]
bundle_id = "us.zoom.xos"
name = "zoom.us"

[[applications]]
bundle_id = "com.microsoft.teams2"
name = "Microsoft Teams"
```

- The engine compares two bundle identifiers with no difference between
  small and capital letters (ASCII). It keeps the text as the shell gave it.
- A profile has at most 32 links. One bundle identifier is at most one time
  in one profile.
- The order of the array is the order of the links in the window.
- The bundle identifier does not change when the user moves the
  application, thus a link stays correct after a move. VSD Craft stores the
  path, and its link breaks.
- The type in memory is a new field of `Profile` in
  `crates/engine/src/profile.rs`:

```rust
pub struct ApplicationLink {
    pub bundle_id: String,
    pub name: String,
}

pub struct Profile {
    // ... the present fields
    /// The applications that this profile is for (decision 0035).
    pub applications: Vec<ApplicationLink>,
}
```

The export file of decision 0031 keeps the links, because the export holds
the profile table with all its keys. A link is not a path of the computer,
and not personal data. Thus a profile of PLUG-31
([#340](https://github.com/shantanugoel/OpenCraft/issues/340)) comes with
its links.

### 5. One application has one profile: the clash rule

For one device model, one application links to one profile at the most.

- **The link command moves the link.** `Command::LinkApplication` for an
  application that a different profile links removes the link from that
  profile, and adds it to the new profile. The engine writes the two
  files. Before the change, the application list of the window shows the
  profile that has the link ("Linked to “Work”"), thus the move is not a
  surprise.
- **A new profile does not take a link.** An import, a starter profile, a
  profile of PLUG-31, and any other add of a profile with links keep the
  links that exist. The engine removes each clashing link from the new
  profile before it writes the new file, and sends `Event::Problem` with
  the text "“*application name*” stays linked to “*profile name*”." once for
  each removed link.
- **A duplicate has no links.** `Command::AddProfile` with `copy_of` copies
  the pages, not the links.
- **A clash in the files.** A person can make a clash with a hand edit or a
  copy of a directory. Then the first profile in the order of the
  identities wins. The engine does not change the files. The event of
  section 9 tells the shell which links do not apply.

### 6. The default profile

Each device model has one default profile. The engine finds it in this
order:

1. The profile with the default mark, if it exists for the model.
2. Else, the first profile with no link that applies, in the order of the
   identities.
3. Else, the first profile in the order of the identities.

The mark is in `state.toml`, in the new table `default`, with one identity
for each model, as the table `active`. It is not in the profile file,
because the mark is not a part of the profile: an export must not carry it,
and two imported files must not both have it. The engine removes the entry
when the user removes that profile, and then step 2 applies.

`Command::SetDefaultProfile { profile }` sets the mark. The mark always
names one profile. The user moves it to a different profile, and cannot
remove it. Thus the window always shows one default profile, and the user
always knows what an application with no link shows.

### 7. The setting "Change the profile with the front application"

- The top key `follow_front_application` of `state.toml` holds it: `true`
  or `false`. A missing key is `true`.
- With `true` and no link in any profile, the default profile is the only
  target. The rule of section 8 then never changes the profile, because
  the target does not change. Thus the default `true` changes nothing for a
  user with no link.
- `Command::SetFollowFrontApplication(bool)` changes the setting and writes
  `state.toml`. A change does not change the active profile. The next
  change of the front application applies the rule.
- The setting is in the engine, not in `shell.toml`, because it is a part of
  the policy of the engine, and a different shell must not do it again.

### 8. The rule of the automatic switch

The engine keeps three values in memory. They are not in a file.

- `shell_in_front`: true if the last `Command::FrontApplication` was
  `Shell`.
- `last_target`: the profile that the rule gave for the last front
  application that is not OpenCraft. At the start, and after
  `SetFollowFrontApplication(true)`, it is empty.
- `chosen_in_window`: the engine sets it to true at a manual selection
  while `shell_in_front` is true. It stays true until the next front
  application that is not OpenCraft.

When the engine gets `Command::FrontApplication(front)`:

1. If `front` is `Shell`, the engine sets `shell_in_front` to true, and
   does nothing more. The device keeps its profile. Thus the user can open
   the window and edit a profile with no switch.
2. If the setting is off, or the screen is locked, or the computer sleeps
   (the `locked` and `sleeping` states of decision 0034), the engine does
   nothing. At the lock, macOS can bring `loginwindow` to the front. That
   must not change the profile.
3. The engine finds the target: the profile with a link that applies for
   the bundle identifier, else the default profile. `NoBundle` has no link,
   thus its target is the default profile.
4. The engine selects the target if `chosen_in_window` is true, or if the
   target is different from `last_target`. Else it changes nothing. If the
   target to select is the active profile already (for example at the
   start), the engine loads nothing, and changes only the cause of section
   9. It sends `Event::Profiles` if the cause changed.
5. It sets `last_target` to the target, and `shell_in_front` and
   `chosen_in_window` to false.

In step 2, the engine also sets `shell_in_front` to false, and changes no
other value.

The selection uses the steps of `Engine::select_profile`: the engine writes
the old profile if it changed, shows the first page of the new profile,
writes `state.toml`, and puts the undo history of the old profile aside
and gives back the history of the new profile
([decision 0030](0030-undo-and-redo-in-the-engine.md#a-change-of-the-active-profile)).
A selection of the active profile does nothing.

The text for the user (the user guide and the help text of the Settings
window) is:

> OpenCraft changes the profile when you go to an application that needs a
> different profile. A profile that you select with a key or the menu bar
> item stays until then. A profile that you select in the OpenCraft window
> stays while OpenCraft is in front, so that you can edit it. When you go
> back to an application, its profile comes back.

| Case | Result |
|---|---|
| Safari (no link) → Zoom (linked to "Calls") | "Calls" |
| Zoom → Safari | The default profile |
| In Safari the user presses a key that selects "Music". Then Terminal (no link) comes to the front. | "Music" stays: Safari and Terminal have the same target. |
| In Zoom the user opens OpenCraft, and selects "Work" to edit it. Then the user goes back to Zoom. | "Calls": the selection was in the window. |
| In Zoom the user opens OpenCraft and edits "Calls" (the active profile). Then the user goes back to Zoom. | "Calls" stays, with no load and no draw. |
| The user turns the setting off | The profile stays until the user selects a different profile |
| The screen locks | No change. The first change of the front application after the unlock applies the rule. |
| No device is connected, or the device is dark | The engine changes the active profile in memory, and the window shows it. The device shows it after the connection or the wake (decision 0034). |
| A folder is open ([decision 0036](0036-folders.md)) | A switch to a different profile closes the folders |

### 9. Commands and events

New commands in `crates/engine/src/engine.rs`:

| Command | Result |
|---|---|
| `FrontApplication(FrontApplication)` | Section 8 |
| `LinkApplication { profile: String, link: ApplicationLink }` | Adds the link to the profile with that identity, at the end of its list. Moves it from a different profile (section 5). A link that the profile has and that applies gives no change. A link that the profile has in `clashes` stays at its place in the list, and the engine removes the link from each other profile: then it applies (amended by [#349](https://github.com/shantanugoel/OpenCraft/issues/349)). A link with a bundle identifier that is not correct, or a 33rd link, gives a problem and no change. |
| `UnlinkApplication { profile: String, bundle_id: String }` | Removes the link from the profile. A link that the profile does not have gives no change. |
| `SetDefaultProfile { profile: String }` | Section 6. An identity that is not a profile of the model gives a problem. |
| `SetFollowFrontApplication(bool)` | Section 7 |

The link commands change a profile that is possibly not the active profile.
For a profile that is not active, the engine reads the file, changes it, and
writes it at once, as `SetProfileColor` does now. For the active profile, it
changes the profile in memory and writes it after the usual delay.

Changes to the events:

- `ProfileEntry` gets `applications: Vec<ApplicationLink>` (the links of the
  profile, in their order) and `clashes: Vec<String>` (the bundle
  identifiers of this profile that a profile before it in the order of the
  identities also links; they do not apply). `ProfilesDir::list` reads them
  with the other top keys (`ProfileStore::summary`).
- `Event::Profiles` gets `default: String` (the identity of the default
  profile of section 6) and `cause: ActiveCause`:

```rust
/// Why the active profile is active.
pub enum ActiveCause {
    /// A manual selection, or the start.
    Selected,
    /// The front application has this link to the profile.
    Linked { bundle_id: String },
    /// The front application has no link, and this is the default profile.
    Default,
}
```

- New `Event::FollowFrontApplication(bool)`: at the start after the first
  `Event::Profiles`, and after each change of the setting.

The engine keeps the links of all profiles of the model in `entries`, thus
the rule of section 8 reads no file. Only the selection reads the file of
the new profile.

### 10. The action "Select a profile" (ACT-26)

The kind stays `opencraft.profile`. Its settings:

| Key | Field label | Input type | Default | Meaning |
|---|---|---|---|---|
| `target` | "Select" | Choice: `profile` "A profile", `next` "Next profile", `previous` "Previous profile" | `profile` | Which profile |
| `profile` | "Profile" | The new input type `InputType::Profile` | none | With `target` = `profile`: the identity of the profile |
| `page` | "Page" | Integer from 0 to 100 | 0 | 0: the first page of the profile ([decision 0031](0031-profile-export-file.md)). From 1: that page number. |

- **`InputType::Profile`.** The shell shows a popup button with the profiles
  of the model from `Event::Profiles`: the name, and the colour of the
  profile. It writes the identity. A value that is not an identity stays,
  and the popup shows it as the text of the value with "(not found)". The
  input type is an amendment of
  [decision 0014](0014-action-settings-description.md). It is not
  `RuntimeChoice`: a handler does not know the profiles, and the shell has
  the list already.
- **The request.** `EngineRequest::SelectProfile { name }` becomes:

```rust
SelectProfile {
    target: ProfileTarget,
    /// The page number from 0. `None`: the first page of the profile.
    page: Option<usize>,
}

pub enum ProfileTarget {
    /// An identity, or the name of an old binding: see below.
    Profile(String),
    /// The profile after the active profile in the order of the identities.
    /// After the last profile, the first profile.
    Next,
    /// The profile before the active profile. Before the first, the last.
    Previous,
}
```

- **How the engine finds `Profile(text)`.** First the profile with the
  identity `text`. Else the first profile, in the order of the identities,
  with the name `text`. Else the engine sends the problem
  "OpenCraft did not change the profile, because no profile has the
  identity or the name “*text*”."
- **An old binding.** A binding of releases up to 0.2.0 has `profile` with
  a name and no `target`. The handler reads a missing `target` as `profile`,
  thus the old binding sends `Profile(name)`, and the name rule finds the
  profile as before. The engine does not change the old setting (rule 8:
  the engine does not read settings). The popup shows the name with
  "(not found)", and a help text below the field says: "Select the profile
  again, so that a rename does not break this key." When the user selects
  it, the field gets the identity.
- **A page that the profile does not have.** The engine selects the profile
  at its first page, and sends the problem "The profile “*name*” has no page
  *N*."
- **A selection by the key is a manual selection** (section 8). When the
  target is the active profile and `page` is given, the engine selects the
  page. Else a selection of the active profile does nothing.

### 11. The window

The window uses the places of
[decision 0029, section 21](0029-window-layout.md#21-places-kept-for-later).
The exact look follows [DESIGN.md](../DESIGN.md) in the task of each part.

- **The profile settings sheet** (section 16 of 0029) gets the section
  "Applications" in the place of "Switch to this profile automatically":
  the list of the links (icon and name of each application, "Remove" for
  each), "Add an application…" (the application list of ACT-09,
  [#336](https://github.com/shantanugoel/OpenCraft/issues/336)), and the
  switch "Default profile" with the help text "The device shows this
  profile when the front application has no linked profile." The switch
  of the default profile is on and disabled: the user moves the mark by
  turning the switch on for a different profile. The window uses the
  switch of [DESIGN.md](../DESIGN.md). That document has no check box, and
  a switch is its on/off control. A link in `clashes` shows
  "Linked to “*profile*”. That link applies." The option "Go back to the
  last profile when the app is not in front" of section 21 is not made: the
  default profile and the rule of section 8 take its place.
- **The sidebar.** A profile row that is not active shows the name of its
  first link that applies (with "+*N*" for more links) in the place of
  "ON DEVICE". The default profile shows the mark "DEFAULT" when it is not
  active. The tooltip of "ON DEVICE" gives the cause: "Selected", "Because
  “*application*” is in front", or "The default profile: the front
  application has no linked profile".
- **The Settings window**, group "General": the switch "Change the
  profile with the front application". The description is the first
  sentence of the text of section 8. A row of this window shows one line.
  The user guide has the full text. The control is a switch, for the same
  reason as the default profile.
- **The menu bar item**: the item "Follow the Front Application" with a
  check mark, below the profile items. It sends the same command.

### 12. Undo

`LinkApplication`, `UnlinkApplication`, `SetDefaultProfile`, and
`SetFollowFrontApplication` are not steps of the history: their line in
`step_of` gives `None`. The links are not in the pages, as the name and the
colour of a profile ([decision 0030](0030-undo-and-redo-in-the-engine.md)),
and a move of a link changes two files. Each change applies at once in the
sheet, and the user can do the inverse change with one click. An automatic
switch puts the history of the old profile aside and gives back the history
of the new profile, as each change of the active profile does
([decision 0030](0030-undo-and-redo-in-the-engine.md#a-change-of-the-active-profile),
amended by issue [#349](https://github.com/shantanugoel/OpenCraft/issues/349)).
Thus a trip to a different application and back keeps the undo of a change.
The switch never occurs while OpenCraft is in front, thus it does not
interrupt an edit in progress.

### 13. Schema version 5

Phase 9 makes one new profile schema version: 5. This record and
[decision 0036](0036-folders.md) give all its new keys. No other task of
Phase 9 makes a new version. If a later task of Phase 9 needs a new key, it
adds the key to version 5 before the release of version 5.

| Key | Where | Record |
|---|---|---|
| `applications` (array of tables: `bundle_id`, `name`) | Top of the file | This record, section 4 |
| `folders` (array of tables: `id`, `name`, `surfaces`, `bindings`) | Top of the file | [Decision 0036](0036-folders.md#2-the-folders-in-the-profile-file) |

Migration, in `crates/engine/src/store.rs`:

- A file of version 1 to 4 loads as now, with no links and no folders. The
  next write writes version 5. A file of version 4 or lower with the key
  `applications` or `folders` has an error (the file is kept as a backup, as
  decision 0009 says).
- Set `SCHEMA_VERSION` to 5, and add the constant `FOLDERS_VERSION: u32 = 5`
  for the check of the new keys, as `LABEL_OPTIONS_VERSION` does for
  version 4.
- Keep the tests of the versions 1 to 4. Add an engine test that loads a
  file of version 3 and a file of version 4, changes it, and finds version
  5 in the written file with the same pages.
- An OpenCraft of version 0.2.0 or lower finds version 5 newer than it
  knows. It keeps the file as a backup and tells the user that a newer
  OpenCraft made it (decision 0009).
- `state.toml` has no schema version. The new keys `default` and
  `follow_front_application` are additive: an old engine ignores them, and
  keeps them when it writes the file. Release 0.2.0 changes only the table
  `active`, and keeps each other key of the file.

### 14. The tasks

| Task | Parts of this record |
|---|---|
| [#337](https://github.com/shantanugoel/OpenCraft/issues/337) ACT-26 | Section 10 |
| [#336](https://github.com/shantanugoel/OpenCraft/issues/336) ACT-09 | The application list for section 11. It gives the bundle identifier and the name. |
| [#338](https://github.com/shantanugoel/OpenCraft/issues/338) AUTO-01 | Sections 2 and 4 to 13 |
| [#340](https://github.com/shantanugoel/OpenCraft/issues/340) PLUG-31 | Section 5, "A new profile does not take a link" |
| [#341](https://github.com/shantanugoel/OpenCraft/issues/341) P9-M1 | The time from the notice to the new profile on the device, and the idle processor use with the observer |

## Rejected alternatives

| Alternative | Reason |
|---|---|
| The shell polls `frontmostApplication` with a timer | It uses processor time when idle (quality goal 2). The notice gives each change with no delay. |
| The shell finds the profile and sends `SelectProfile` | Rule 12: the shell contains no logic of this kind. A different shell must do it again, and no engine test covers it. |
| Store the path of the application, as VSD Craft does | A move of the application or an update to a different path breaks the link. The path contains the home folder (personal data in an export). |
| An identifier (for example a UUID) in each profile file | The directory name does not change at a rename, thus it is enough. A UUID in the file gives two profiles with one identity after a copy of a directory, and needs a migration and a check. |
| Keep the links in `state.toml` | The links go with the profile: in an export, in a removal, and in a ready profile of PLUG-31. |
| Keep the default mark in the profile file | An export and an import would carry it, and two files could have it. The engine must then write a second file to move the mark. |
| No default profile: the device keeps its profile when no link matches | The device then keeps "Calls" in Safari after a call. VSD Craft and the phase issue have a default. |
| Go back to the last manual selection when no link matches (the place of 0029, section 21) | The result depends on a history that the user cannot see. The selection of a linked profile to edit it would become that last selection. |
| A switch at each change of the front application (no compare with `last_target`) | A key selection is lost at each change between two applications with no link. |
| No switch until the user selects again after a manual selection | The user does not know when the automatic switch starts again. |
| Many profiles for one application, the first wins | The window must explain an order. One profile for each application is simple to show and to explain. |
| Remember the last page of each profile | The present rule (the first page of the profile) stays the same for all selections. A later task can add it. |

## Results

- The device follows the front application with no polling. The cost when
  idle is zero. The time of a switch is the load of one profile file, its
  first page, and a draw. P9-M1 measures it.
- A rename of a profile and a move of an application keep the links and the
  keys correct. An old "Select a profile" key with a name operates as before.
- The user can always see why a profile is on the device (the tooltip and
  the marks), and can turn the switch off in Settings or in the menu bar
  item.
- `state.toml` now holds two values of the user: the default mark and the
  setting. An error in the file loses them; the default profile then comes
  from the rule of section 6, and the setting is on.
- The engine writes `state.toml` at each automatic switch, as at each
  change of the active profile now. This is one small write after a change
  of the front application, not when idle.
- Decision 0018 stays, with these changes: the request selects by identity
  first, with the name for an old binding, and `state.toml` has more keys.
- No rule of [ARCHITECTURE.md](../ARCHITECTURE.md) changes. The shell
  reports a system notice and sends a command (rules 2 and 12). The engine
  holds the policy and operates with no window (rule 3).
- Look at this decision again when OpenCraft supports a second device: a
  link is for a model now.
