# 0006: Action kinds have a namespace, and all actions use one handler interface

- Status: Accepted
- Date: 2026-11-11

## Context

Plug-ins are not in a phase. The decisions about the plug-in language, the
package format, the installation, and the protocol can wait.

But Phase 3 makes the built-in actions. Three things from that phase are
expensive to change later:

- The profile file stores the kind of each action. If a built-in kind has a
  short name, a plug-in kind can have the same name later. A change of the
  names breaks the saved profiles.
- If the built-in actions call the engine internals directly, a plug-in host
  needs a new design of the engine.
- If each built-in action has a hand-made settings panel in the GUI, plug-ins
  need a second mechanism for their settings.

## Decision

1. The kind of an action is an identifier with a namespace. The built-in kinds
   use the namespace `opencraft`, for example `opencraft.hotkey`.
2. If no handler exists for a kind, the engine keeps the binding and its
   settings in the profile. The shell shows that the action is not available.
3. A built-in action uses the same handler interface that a plug-in will use.
   Only messages go through this interface. A handler can send a message to
   change a surface at any time.
4. Phase 3 decides how an action describes its settings to the GUI. That
   decision must also be correct for a plug-in.

## Not decided

These decisions wait until after the first release:

- The language of a plug-in.
- The package format and the installation procedure.
- The protocol between the engine and a plug-in, and whether a plug-in is a
  different process.
- Compatibility with Stream Deck plug-ins. This is a different feature from
  our own plug-in format, and it can conflict with the "Small" quality goal.
  See [../kb/prior-art.md](../kb/prior-art.md#stream-deck-plug-in-model).

## Results

- A plug-in host later is an addition to the engine, not a new design.
- A profile stays correct when the user removes a plug-in.
- The built-in actions test the handler interface from Phase 3.
- The handler interface needs more design work in Phase 3 than a direct
  function call.
- This decision changed rule 8 of the architecture.
