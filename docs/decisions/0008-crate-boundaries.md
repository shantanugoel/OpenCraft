# 0008: One crate for the engine and one crate for the GUI application

- Status: Accepted
- Date: 2026-11-11

## Context

Phase 2 adds the engine and the GUI application. The layers in
[ARCHITECTURE.md](../ARCHITECTURE.md#4-layers) are the limit: the engine does
not know the GUI toolkit, and the shell does not know device reports.

A rule that only a document states is easy to break. The compiler can enforce
a rule that is a crate boundary.

## Decision

The workspace has four crates.

| Crate | Directory | Layer | Depends on |
|---|---|---|---|
| `opencraft-core` | `crates/core` | Device layer | No OpenCraft crate |
| `opencraft-cli` | `crates/cli` | Probe | `opencraft-core` |
| `opencraft-engine` | `crates/engine` | Engine | `opencraft-core` |
| `opencraft-app` | `crates/app` | GUI application (shell) | `opencraft-engine` |

- `opencraft-engine` is a library. It holds the profile, the images for
  surfaces, the thread that owns the device, and the messages. It has no
  dependency on a GUI toolkit.
- `opencraft-app` makes the program `opencraft`. It does not have
  `opencraft-core` in its manifest. Thus the shell cannot call the device layer
  (rules 2 and 12).
- The shell needs some data types of the device layer: the identity of a
  surface and of a control, and the input event. The engine gives these types
  to the shell again as a part of its messages. The engine gives the layout of
  the device as a message that contains data only, not a reference to the
  descriptor.
- The code that makes the image of a surface is a module of the engine. It is
  not a separate crate.

## Results

- The compiler refuses GUI code that uses the device layer, and engine code
  that uses the GUI toolkit.
- The engine tests build with no GUI toolkit. They are quick, and they run on
  a computer with no display.
- Each message contains data only. Thus the engine can go to a different
  process in Phase 5 with no change to the shell logic.
- A built-in action of Phase 3 starts as a module of the engine. Look at this
  decision again when the handler interface of Phase 3 is made, and when the
  plug-in host is designed.
