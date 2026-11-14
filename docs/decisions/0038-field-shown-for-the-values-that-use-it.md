# 0038: The shell shows a field only for the values that use it

- Status: Accepted
- Date: 2026-11-14

## Context

Issue [#333](https://github.com/shantanugoel/OpenCraft/issues/333). The form
of Sound volume shows Step when Operation is Mute. Mute does not use the
step. Device brightness, Mouse event and Change the page have the same
fault: the form shows a field that the selected operation does not use.

[Decision 0014](0014-action-settings-description.md) says that the shell
makes the form from the fields of the kind. Rule 12 says that the shell
has no action logic. The shell cannot test the name of a kind.

Microphone mute has no fields. It does not have this fault.

## Decision

A kind description can name when the shell shows a field. Each rule names
the field, the choice that controls it and the values of that choice.

The shell shows the field only when the choice has one of those values. A
missing choice uses the default of that choice. The shell removes the field
from the settings when it hides the field. When the field is shown again
and it has a default, and the settings have no value, the shell stores that
default.

One change of the choice is one step of the undo history. The handler does
not read a field that the operation does not use. A stored value for a
hidden field does not make the action fail.

The shell still has no list of kinds. A plug-in uses the same description.

## Results

- Mute does not show a step, and a change to Mute does not keep a step.
- Up or Down then stores the default step, 5, when the settings have no step.
- The same rule hides Brightness or Step, the mouse fields, and the page
  number, for an operation that does not use that field.
- A field with no rule stays in the form for every value.
