# 0003: The device layer blocks and has no async runtime

- Status: Accepted
- Date: 2026-11-11. The code obeyed this decision from the first commit.

## Context

The first plan gave the core a tokio runtime. The code does not use one. The
`hidapi` calls block, and the device has one reader and one writer.

## Decision

The device layer uses blocking calls. The caller supplies the threads. The
device layer has no async runtime.

## Results

- The device layer stays small and has few dependencies. This helps the
  footprint goal.
- The probe and the tests call the device layer with no runtime.
- The engine must keep device I/O off the user interface thread (rule 5). The
  thread model of the engine is a decision for Phase 2.
- Look at this decision again if the engine needs many I/O tasks at the same
  time, for example a plug-in host with network connections. That need is in
  the engine. It is not a reason to change the device layer.
