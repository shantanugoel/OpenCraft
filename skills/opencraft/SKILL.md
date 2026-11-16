---
name: opencraft
description: Configure a running OpenCraft (StreamDock controller app) with `opencraft ctl`. Use to read or change its profiles, pages, folders, key labels, icons, colours, backgrounds, and bindings (actions on keys, knobs, panels), or the device settings.
---

# Configure OpenCraft with `opencraft ctl`

`opencraft ctl <command>` changes the OpenCraft that runs now. It prints JSON on
the standard output and a problem as text on the standard error. The user sees
each change at once on the device and in the window, if a device is connected.
With no device, OpenCraft accepts and saves each change, and reads it back. The
device shows it when it connects. `opencraft ctl connection` says
`"connected": false` then.

## Start

1. Run `opencraft ctl state`. It reads everything in one result: `layout`,
   `kinds` (long: filter it, for example `jq 'del(.kinds, .layout)'`),
   `profiles`, `pages`, `content`, `device_settings`, `history`, `connection`.
2. Read the exit status.

| Status | Meaning | You do |
|---|---|---|
| 0 | Done | Continue |
| 1 | OpenCraft refused. The standard error has the reason. | Correct the request |
| 2 | The arguments are wrong | Run `opencraft ctl <command> --help` |
| 3 | OpenCraft does not run | Ask the user to start OpenCraft. Stop. |
| 4 | Control is off | Ask the user to turn on "Allow control by other programs" in the Settings window. Stop. |
| 5 | Client and OpenCraft have different protocol versions | Ask the user to update. Stop. |

Never start OpenCraft yourself. Never edit a profile file: OpenCraft does not
read it again and the next save removes your edit.

3. Write down your **home** from the state: the active profile (`profiles.active`),
   and the active location (`pages.location`: a page, and the open folders).

## If your client has the MCP tools

If your client has the tools `opencraft_read`, `opencraft_change`, and
`opencraft_help` in place of a shell, use them in place of `opencraft ctl`:

- `opencraft_read` with `command` and `args` reads. Its `command` is the name
  of a read command, for example `state`, `icons`, or `page list`. A read
  changes nothing.
- `opencraft_change` with `command` and `args` changes. Its `command` is the
  name of a change command, for example `surface set-label` or `page add`.
  Ask the user before `profile remove`.
- `args` has the flags of `opencraft ctl` as keys, without `--`: `{"profile":
  "work", "page": "0123456789abcdef", "surface": "key_face:0", "label":
  "Mute"}`. A text is a string, a whole number is a number, and true or false
  is a boolean. `setting` is an object of settings, as `{"hotkey":
  "cmd+c"}`, and `knob-colors` is an array of colours. Leave out a value to
  remove it.
- `opencraft_help` with `command` gives the arguments and an example of one
  command. With no `command`, it gives this text.

A problem is the result with `"isError": true`. Its text is the text that
`opencraft ctl` gives for the same request. The rules, the terms, and the
limits below are the same for the tools.

## Rules

- **A change with `--page` moves the device.** It makes its profile the active
  profile and shows the page. A change to a different page closes the folder
  that was open. A request that has `--folder` opens that folder, and it closes
  the open folder when you give no `--folder`. `profile add`,
  `profile import`, and `page add` move the device too, for the new profile or
  page. When you are done, go home: run `profile select` for your home profile,
  `page select` for your home page, and `folder open` for each open folder of
  your home.
- **A read does not move the device.** `state.content` is the active location.
  `content`, `page list`, `folder list`, and `history` read a profile that is
  not active without selecting it.
- **The default profile is not the active profile.** The default profile is
  what the device shows for an application that has no link. Only
  `profile set-default` changes it (and `profile remove` for the default
  profile itself). Going home needs no `profile set-default`.
- **You cannot run an action.** A person must push the key. Set the binding,
  then tell the user which key to push.
- **You cannot** send a gesture, stop OpenCraft, or change the control setting.
- **Read identities. Never guess them.** A profile, a page, and a folder have an
  identity. `profile list`, `page list`, and `folder list` give them. A name can
  be the same for two pages. Each change gives the place that it changed:
  `{"profile": "demo", "page": "dbf2edfee20f62ab", "folder": null}`. A part
  that the request does not have is null. A request that adds a thing gives
  the identity of the new thing in its field (`page add`: the new page).
  Use that identity at once. A request that removes a thing names it; it is
  gone.
- **Exit 0 means OpenCraft accepted the request.** `binding set` refuses an
  action with an unknown kind, an unknown setting, or a value that the kind
  refuses (exit 1, `invalid_params`, with a text that names the problem). A
  field that `shown_when` does not show for the other settings is a refusal
  too. The binding does not change. Use `opencraft ctl kinds` to read the
  kinds, the settings, and the limits. `"available": true` in `content` only
  says that the kind exists, and each binding that OpenCraft accepts has it.
  A kind can still do nothing at run time, for example when it needs a
  permission: read the `status` of the kind in `opencraft ctl kinds` (see
  below), and tell the user what its `text` says.
- **`history` gives names, and the names are the names of the window.** The
  name of the next undo and of the next redo is the name that the window shows
  for that change. To see what an undo changes, read `content` (or `state`)
  before and after the undo, and tell the user the difference: the name alone
  does not give the page, the key, or the value that the undo changes.
- **A change of the pages is a step of the history.** These requests make
  a step: `surface set-label`, `surface set-label-style`, `surface set-color`,
  `surface set-image`, `surface set-icon`, `binding set`, `page add`,
  `page duplicate`, `page rename`, `page move`, `page remove`,
  `page set-first`, `page set-background`, `page set-background-fit`,
  `folder add`, `folder rename`, `folder move`, `folder remove`,
  `control paste`, `control clear`, and `control copy-to-all-pages`. The other
  requests are not steps: all `profile` commands, `page select`, `folder open`,
  `folder close`, `control copy`, and `device set-settings`. The command
  `undo --profile ID` reverses the last step of the active profile, and the
  result gives the place of the reversed step. `history` reads the next undo
  and the next redo of the active profile, or of the profile that you name.
- **No `profile` request has an undo.** The window cannot undo a change of a
  profile either. Ask the user before `profile remove`. Then tell the user that
  OpenCraft does not delete the profile: it moves it to the folder `removed` in
  the profiles folder. If you remove the active profile, another profile
  becomes active: the one that is now at the same position in the list.
  OpenCraft keeps one profile, thus a removal of the last profile is a refusal.
- **An empty history is not a wrong request.** With no step, `undo` gives the
  error `nothing_to_undo` (`nothing_to_redo` for `redo`) and exit 1. The
  request is correct: there is nothing to reverse. Tell the user, and do not
  change the request.
- The user can use the device while you work. If a request fails because a page
  is gone or moved, read the state again.
- A file path in an argument is absolute, or relative to the current directory.
- If the user runs OpenCraft with a different profiles directory, set
  `OPENCRAFT_PROFILES_DIR` to the same directory for the client.

## Terms

| Term | Meaning |
|---|---|
| Profile | A named set of pages and folders. It has an identity, for example `default`. |
| Page | A full set of bindings and surface content. Its identity is 16 hexadecimal digits. |
| Folder | A set of keys that a key opens and a "Back" key closes. It has an identity. |
| Surface | An area to draw on, as `kind:number`: `key_face:1`, `bar_panel:2`, or `background:1`. |
| Control | A part that the user operates, as `kind:number`: `key:1`, `knob:1`, `bar_panel:1`, `touch_bar:1`. |
| Gesture | What the user does to a control, for example `press`, `turn_left`, `tap`. |
| Binding | The link from one gesture of one control to one action. |
| Action kind | The type of an action, for example `opencraft.hotkey`. |

Numbers start at 1. The pages count from 1 in the order of `page list`: page 1
is the first page of that list, and it is the page number that the action
`opencraft.page` and `page move --to` use. A place in a list (`--to`) counts
from 1: `--to 1` is the first place. `opencraft ctl layout` gives the surfaces,
the controls, the surface of each control, and the gestures of each control.
Read it for the device model; do not assume it.

## Find an action kind and its settings

`opencraft ctl kinds` is long. Filter it before you read it (for example with
`jq`). Each kind has:

- `kind`: the identity. Use it in full: `opencraft.hotkey`, not `hotkey`.
- `fields`: the settings. Each field has a `key` and an `input.type`. A choice
  has its `options` in `input`. `default` is the value that OpenCraft uses when
  you give none. A field with `default: null` has no value until you give one,
  and for such a field a text of no characters is the same as no value: for
  example, `--setting 'application=""'` of `opencraft.open-url` opens the URL
  in the default browser. A field of another type has no empty value: leave
  it out.
- `status`: `{"state": "ready"}`, or `{"state": "needs_user", "text": …,
  "button": …}` for a kind that needs a permission. If the state is not
  `ready`, the key does nothing until the user acts as the `text` says; tell
  the user that text. For example, `opencraft.hotkey`, `opencraft.type-text`,
  `opencraft.media-key`, `opencraft.mouse`, and `opencraft.mac-function`
  show `needs_user` while OpenCraft has no Accessibility permission. `status`
  is `null` for a kind that never needs the permission, or before OpenCraft
  reads it: `null` is not a fault.

A key opens a folder with the kind `opencraft.open-folder`. Its setting
`folder` is the identity of the folder. A folder opens only with such a key.
To leave the folder, the user pushes a key with the kind
`opencraft.close-folder` (the action "Back"). OpenCraft adds no key to a folder
that you add. A page selection and a profile selection also close the folder.

Give a setting as `--setting KEY=VALUE`, one flag for each field. A value is
JSON when it can be JSON (`step=5` is a number), else text. Thus a text field
needs a JSON string for a value that is a number, `true`, `false`, `null`, or
that starts with `"`, `[`, or `{`: `--setting 'text="2026"'`. Without the
quotation marks, OpenCraft refuses the value, and the text of the refusal names
the setting. A hotkey is the modifiers and one key with `+`: `cmd+c`,
`shift+cmd+4`. The modifiers can be in any order (`cmd+shift+4` is the same
hotkey), and capital letters are correct. The modifiers are `ctrl`, `alt`,
`shift`, `cmd`, `fn`, and the right-side `rightctrl`, `rightalt`, `rightshift`,
`rightcmd`.

`opencraft ctl choices --action-kind opencraft.play-sound --field device` reads
a choice that depends on the computer, for example the sound devices. Only the
sound output fields have choices that depend on the computer. The field
`application` of `opencraft.open-application` is a path: give it as
`--setting application=/Applications/Safari.app`, and, if you know it, give the
bundle identifier here too: `--setting bundle_id=com.apple.Safari`.

An icon is a name of the Lucide set (for example `copy`, `compass`, `mic`).
Do not guess a name: `opencraft ctl icons --search mic` gives the names that
this version has, and each name in that result is correct. A search agrees
with a name and with the tags of an icon. An unknown name gives exit 1.

A colour is six hexadecimal digits with `#` in front: `#ff8800`. The letters
can be small, and the `#` is optional. A colour name (`red`) and a short form
(`#f00`) are refused. A read gives the colour in capital letters (`#FF8800`).

## Example: a profile with two pages

Each line uses the identities that the lines before it gave. The identities
here are examples: use the ones that OpenCraft gives you.

```sh
opencraft ctl profile add --name Demo
opencraft ctl page list --profile demo
opencraft ctl page rename --profile demo --page 4853c7f9bf374716 --name Main
opencraft ctl page add --profile demo
opencraft ctl surface set-label --profile demo --page 4853c7f9bf374716 --surface key_face:1 --label Copy
opencraft ctl surface set-icon --profile demo --page 4853c7f9bf374716 --surface key_face:1 --icon-name copy
opencraft ctl surface set-label --profile demo --page 4853c7f9bf374716 --surface key_face:2 --label Paste
opencraft ctl surface set-icon --profile demo --page 4853c7f9bf374716 --surface key_face:2 --icon-name clipboard-paste --icon-color '#ff8800'
opencraft ctl surface set-label --profile demo --page 4853c7f9bf374716 --surface key_face:3 --label Safari
opencraft ctl surface set-icon --profile demo --page 4853c7f9bf374716 --surface key_face:3 --icon-name compass
opencraft ctl binding set --profile demo --page 4853c7f9bf374716 --control key:1 --gesture press --action-kind opencraft.hotkey --setting hotkey=cmd+c
opencraft ctl content --profile demo --page 4853c7f9bf374716
```

`profile add` makes the new profile the active profile, and does not change the
default profile. A new profile has one page, `Page 1`; `--copy-of` copies each
page of the other profile. The result gives the identity of the new profile and
the identity of its first page, and `page list` gives the identity of the page
`Page 1`. `page add` gives the identity of the new page in the result. To
remove a surface value or a binding, leave out the value:
`opencraft ctl surface set-label` with no `--label`.

## Commands

Flags: `target` means `--profile`, `--page`, and, inside a folder, `--folder`.
Run `opencraft ctl <command> --help` for the details and an example.

| Command | Flags | Use |
|---|---|---|
| `state` | | Read all state |
| `layout` | | Read the surfaces, controls, gestures |
| `kinds` | | Read the action kinds |
| `icons` | `--search` | Read the icon names; no flag: each name |
| `choices` | `--action-kind` `--field` `--setting` | Read the choices of a field |
| `content` | `--profile` `--page` `--folder` | Read a location; no flags: the active one |
| `device settings` | | Read the device settings |
| `history` | `--profile` | Read the next undo and redo |
| `connection` | | Read the device connection |
| `profile list` | | Read the profiles |
| `profile add` | `--name` `--copy-of` | Add a profile |
| `profile rename` | `--profile` `--name` | Rename |
| `profile remove` | `--profile` | Remove with its pages |
| `profile select` | `--profile` | Make it the active profile |
| `profile set-color` | `--profile` `--color` | Set or remove the colour |
| `profile set-default` | `--profile` | Make it the default profile |
| `profile link-app` | `--profile` `--bundle-id` `--name` | Link an application |
| `profile unlink-app` | `--profile` `--bundle-id` | Remove the link |
| `profile follow` | `--follow` | Turn the automatic switch to the front application on or off |
| `profile export` | `--profile` `--path` | Write a profile to a file |
| `profile import` | `--path` | Read a profile from a file |
| `page list` | `--profile` | Read the pages, the folders, and the first page; no flag: the active profile, with its location |
| `page add` | `--profile` | Add an empty page at the end |
| `page duplicate` | `--profile` `--page` | Copy a page |
| `page rename` | `--profile` `--page` `--name` | Rename |
| `page move` | `--profile` `--page` `--to` | Move to a place, from 1 |
| `page remove` | `--profile` `--page` | Remove |
| `page select` | `--profile` `--page` | Show the page on the device |
| `page set-first` | `--profile` `--page` | Make it the first page |
| `page set-background` | `--profile` `--page` `--color` `--image` `--fit` | Set or remove the background |
| `page set-background-fit` | `--profile` `--page` `--fit` | `fill` or `fit` |
| `folder list` | `--profile` | Read the folders; no flag: the active profile |
| `folder add` | `--profile` `--name` `--copy-of` `--id` | Add a folder |
| `folder rename` | `--profile` `--folder` `--name` | Rename |
| `folder move` | `--profile` `--folder` `--to` | Move to a place, from 1 |
| `folder remove` | `--profile` `--folder` | Remove |
| `folder open` | `--profile` `--page` `--folder` `--inside` | Show a folder |
| `folder close` | target | Close the folder that is open |
| `surface set-label` | target `--surface` `--label` | Set or remove the text |
| `surface set-label-style` | target `--surface` `--position` `--size` `--color` `--bold` | Set the text style |
| `surface set-color` | target `--surface` `--color` | Set or remove the fill colour |
| `surface set-image` | target `--surface` `--path` | Set or remove the image |
| `surface set-icon` | target `--surface` `--icon-name` `--icon-color` | Set or remove the icon |
| `binding set` | target `--control` `--gesture` `--action-kind` `--setting` `--action` | Set an action; with none: remove it |
| `control copy` | target `--control` | Copy a control |
| `control paste` | target `--control` | Paste onto a control |
| `control clear` | target `--control` | Remove surface and bindings |
| `control copy-to-all-pages` | target `--control` | Copy to the other pages |
| `device set-settings` | `--brightness` `--screen-off-minutes` `--haptic` `--knob-mode` `--knob-brightness` `--knob-colors` | Change the given values only |
| `undo` | `--profile` | Undo the last change |
| `redo` | `--profile` | Redo |
