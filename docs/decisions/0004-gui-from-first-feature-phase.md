# 0004: The GUI is in each feature phase

- Status: Accepted
- Date: 2026-11-11

## Context

There were two ways to order the work:

- Make a daily driver with no GUI first. The user edits the profile file by
  hand. Add the GUI later.
- Make the GUI from the first feature phase. Each phase adds a small feature
  through all layers.

The owner selected the second way. A profile that the user edits by hand is not
part of the product.

## Decision

Each phase from Phase 2 adds its features to the device layer, the engine, and
the GUI together. The GUI is the only editor of the profile.

## Results

- The application is usable by a person who does not edit files, from the first
  feature phase.
- The decisions about the GUI toolkit come early, in Phase 2. See
  [decision 0005](0005-gpui.md).
- The profile format can change freely between phases, because no person writes
  it by hand. The file must stay readable and must have a schema version
  (rule 9).
- The engine must stay independent of the GUI (rules 2, 3, and 12). If it does
  not, the GUI toolkit becomes expensive to replace.
- Phase 1 has no GUI. It is the foundation of the device layer.
