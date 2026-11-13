# 0014: An action kind describes its settings as a list of fields

- Status: Accepted
- Date: 2026-11-12

## Context

The user selects and configures an action in the window. Rule 12 says that
the shell contains no action logic. [Decision 0006](0006-action-kinds-and-handler-interface.md)
says that the answer must also be correct for a plug-in: a hand-made panel
for each built-in action would need a second mechanism for plug-ins.

The Stream Deck software lets each plug-in give an HTML page for its
settings: see [kb/prior-art.md](../kb/prior-art.md#stream-deck-plug-in-model).
That needs a web view in the application, which conflicts with the quality
goal "Small".

Some actions need something from the user before they can operate. The
hotkey action needs the Accessibility permission: see
[kb/macos-permissions.md](../kb/macos-permissions.md).

## Decision

**The description of a kind is data.** A handler gives one description for
each kind:

| Part | Meaning |
|---|---|
| Kind | The identifier, for example `opencraft.hotkey` |
| Name | The name for the user, for example "Send a hotkey" |
| Group | The group of the kind in the library and in the kind menu, for example "System" |
| Help | Optional. A short text for the library row, for example "Accessibility" |
| Needs Accessibility | Yes or no: the kind operates only with the Accessibility permission of macOS |
| Fields | The list of the settings, in the order in which the shell shows them |
| Direction | Optional. The direction field for a turn of a knob: the key of a choice field, the value for a turn to the right, and the value for a turn to the left. For "Sound volume": `operation`, `up`, `down`. |
| Two states | Optional. The kind has the states Off and On, and the key of the text field with the label of the state On, if the kind has one |

Note (2026-11-13): issue
[#184](https://github.com/shantanugoel/OpenCraft/issues/184) added the
group, the help text, and the need for the Accessibility permission
(decision [0029](0029-window-layout.md#20-data-that-the-shell-needs-from-the-engine)).
The shell shows the groups in the order of their first kind in the event of
the kinds. It gives the tile colour of a kind from its group. It names the
kinds that need the permission in the Settings window. The shell still has
no list of kinds and no check for a specific kind.

Note (2026-11-13): issue
[#182](https://github.com/shantanugoel/OpenCraft/issues/182) added the
direction (decision
[0029](0029-window-layout.md#104-the-knob-inspector-182), section 10.4).
The knob inspector shows the two bindings "Turn left" and "Turn right" as
one "Turn" card when they have the same kind and settings, and the values
of the direction field agree with the direction. A change of that card
writes the two bindings in one `Command::Group`: the right value to "Turn
right" and the left value to "Turn left", or the other way for "Left is
up". The values come from the description, thus the shell has no action
logic (rule 12). A kind with no direction gets the same settings on the two
bindings.

Note (2026-11-13): issue
[#183](https://github.com/shantanugoel/OpenCraft/issues/183) added the two
states (decision
[0029](0029-window-layout.md#105-toggle-and-sequence-183), section 10.5).
The toggle has them, with the field `on_label`. The shell then shows the
section "Appearance for each state", and shows the field of the label there,
not in the card. The labels of the two action fields of the toggle are now
"From Off to On" (`on`) and "From On to Off" (`off`).

A field has a key, a label for the user, an input type, and an optional
default value. The key is the name of the value in the settings of the
action.

| Input type | The user gives | Value in the settings |
|---|---|---|
| Text | One line of text | Text |
| Long text | Many lines of text | Text |
| Integer, with a minimum and a maximum | A number | Integer |
| Choice, with a list of options | One option. An option has a value and a label. | Text: the value of the option |
| Toggle | On or off | Boolean |
| Hotkey | A key with modifier keys | Text, for example `shift+cmd+4` |
| Path, of a file or of an application | A file that the user selects or types | Text |

The form of a hotkey text: the modifiers `ctrl`, `alt`, `shift`, and `cmd`,
in this order, and then one key name, all in small letters with `+` between
them.

**The shell makes the form.** The engine sends the descriptions to the shell
in an event. The shell makes the form for an action from the fields of its
kind. The shell has no code for a specific kind. When the user changes a
field, the shell sends the full settings of the action to the engine in a
command.

**The handler checks the settings.** The shell and the engine do not check
that the settings agree with the fields. A handler must accept settings that
are incomplete or incorrect, and it sends a Problem message when it cannot
use them.
Note (2026-11-12): the built-in handlers make the text of such a problem from
the description of the kind: the name of the kind, the label of the field,
and what to do for its input type. The text never shows the key
(`setting_problem` in `crates/actions/src/descriptions.rs`, issue
[#162](https://github.com/shantanugoel/OpenCraft/issues/162)).

**Status of a kind.** A handler can send a Status message for a kind:

| Status | Content | The shell |
|---|---|---|
| Ready | None | Shows nothing |
| Needs the user | A text for the user, and the label of a button | Shows the text and the button near each action of the kind |

When the user presses the button, the shell sends a command, and the engine
sends the request Resolve to the handler. The handler does what is necessary,
for example it starts the prompt of macOS for the Accessibility permission.
This is the onboarding for a permission. The shell does not ask macOS for a
permission. It knows only which kinds need the Accessibility permission,
from their descriptions.

## Results

- A new action kind needs no change to the shell and no change to the engine.
- A plug-in gives the same data, for example in a file of its package.
- The window stays native and small. No web view is necessary.
- An action cannot have a custom control in its form. If an input type is
  missing, add it to the list. A shell that does not know an input type shows
  a text field.
- The profile keeps settings that do not agree with the fields, for example
  after an update of a handler. The handler decides what to do with them.
- Look at this decision again when the plug-in package format is designed.
