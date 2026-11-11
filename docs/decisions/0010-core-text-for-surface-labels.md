# 0010: Use Core Text for surface labels on macOS

- Status: Accepted
- Date: 2026-11-11

## Context

Phase 2 needs text on key faces and bar panels. The engine makes each image
without the GUI toolkit. Text can use scripts other than Latin.

The comparison, sample checks, API notes, and limits are in
[kb/text-rendering.md](../kb/text-rendering.md). The priorities are correct
output, small footprint, and low cost to extend, in that order.

## Decision

Use Core Text through the `core-text` Rust binding for label layout and
drawing on macOS. Use `core-graphics` for an off-screen bitmap and
`core-foundation` for styled text. Start with a system sans-serif font and let
Core Text select fallback fonts. Do not ship a font file.

Keep this code inside the engine label module. It does not call GPUI and does
not need a window. Measure and draw with the same layout path. Wrap to the
area width. Try a smaller font when the block does not fit. At the minimum
size, cut at a text-cluster boundary and add an ellipsis. Keep these details
out of the engine callers.

For Phase 2 icons, use the existing `image` path for PNG and JPEG files of the
user. This record makes no SVG decision and selects no resize filter.

The implementation is a later task. This record adds no workspace dependency
and no engine code.

## Results

- Core Text, COSMIC Text, and Parley passed the named script samples. The two
  single-font candidates did not. Their lower-level APIs would need more
  shaping and fallback code. Correct output comes before a small binary.
- Of the candidates that passed, Core Text has the smallest added binary and
  the shortest measured first-label time. It needs fewer added crates. These
  results give room for the rest of the application at start.
- Core Text is not the lowest-memory choice. COSMIC Text used less RSS,
  especially after the script samples. We accept that cost for macOS in
  return for the smaller binary and shorter first-label time. The full
  application must still meet the memory target. This test does not prove it.
- Native layout supplies shaping, font fallback, line wrap, and line metrics.
  The engine still needs RGB copying, size selection, clipping, and ellipsis
  handling. The current binding needs a small FFI wrapper if the
  implementation uses the native truncation function.
- The output can change when system fonts or macOS change. Keep image tests
  for the script cases. Do not require identical pixels across OS versions.
- Linux and Windows will need another implementation. That work is not in
  Phase 2. Do not expose Core Text types through the label interface.
- The engine must build on Linux, because the continuous integration runs
  there. On an operating system that is not macOS, the label module draws no
  text. The tests that need text run only on macOS.
- Look at this decision again before a Linux or Windows port, if a required
  script fails, or if the full application misses a product target. Compare
  COSMIC Text first if memory or portability becomes the limit.
