#!/usr/bin/env bash
# Record the input of each control of the N4 Pro, in 4 short passes.
# Each pass is one recording. Do the gestures of the pass in the given order,
# then press Enter. The order tells which report is which control.
# The procedure is in GitHub issue #11. The file format is in
# crates/core/tests/fixtures/input/README.md.
#
# Usage: scripts/record-input.sh [pass number]
#   No argument: all 4 passes. With a number: only that pass, to do it again.
#
# Needs: the device connected, VSD Craft closed, and cargo.
# Works with bash 3.2 (the version on macOS).

set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/crates/core/tests/fixtures/input"
PROBE="${CARGO_TARGET_DIR:-$ROOT/target}/release/opencraft-probe"
ONLY="${1:-}"

case "$ONLY" in
    '' | 1 | 2 | 3 | 4) ;;
    *)
        echo "Usage: $0 [pass number 1 to 4]"
        exit 2
        ;;
esac

# pass <number> <file name> <note for the file> <instruction lines...>
pass() {
    local n="$1" name="$2" note="$3" file line
    shift 3
    if [ -n "$ONLY" ] && [ "$ONLY" != "$n" ]; then
        return
    fi
    file="$OUT/session-$n-$name.txt"
    echo
    echo "=== Pass $n of 4: $name ==="
    for line in "$@"; do
        echo "  $line"
    done
    echo "  Then press Enter."
    echo
    # The probe draws the panels first, then records. The filter keeps the
    # serial number and the other start lines off the screen.
    "$PROBE" raw --draw --save "$file" --note "$note" --until-enter --seconds 900 |
        grep --line-buffered -e '^\[' -e '^Saved' -e 'rror'
    if [ "$(grep -vc '^#' "$file" 2>/dev/null || true)" = "0" ]; then
        echo "  NO REPORT in this pass. Stop and tell the agent."
    fi
}

echo "Quit VSD Craft first. Building the probe..."
(cd "$ROOT" && cargo build --release --quiet) || exit 1
mkdir -p "$OUT" || exit 1
echo "Each gesture prints a line that starts with [512]. If no line shows, stop."

pass 1 keys \
    "keys: each key one time (top row left to right, then second row left to right), then 2 adjacent keys together, then 1 key held for 3 seconds" \
    "1. Press each key one time: top row left to right, then second row left to right." \
    "2. Press 2 adjacent keys at the same time." \
    "3. Hold 1 key for 3 seconds."

pass 2 knobs \
    "knobs: for each knob from left to right: turn left 5 clicks, turn right 5 clicks, push. Then the left knob turned fast for 5 seconds" \
    "1. Left knob: turn LEFT 5 clicks, turn RIGHT 5 clicks, push it one time." \
    "2. Do the same with the 2nd, 3rd, and 4th knob." \
    "3. Turn the left knob as fast as you can for 5 seconds."

pass 3 touch-bar \
    "touch bar: 4 taps from left to right, slow slide from the left end to the right end, slow slide from the top edge to the bottom edge, finger held for 2 seconds at the left end" \
    "1. Tap the touch bar 4 times, from left to right." \
    "2. Slide one finger slowly from the left end to the right end." \
    "3. Slide one finger slowly from the top edge to the bottom edge." \
    "4. Hold one finger on the left end for 2 seconds." \
    "Wait 2 seconds between the items."

pass 4 swipes \
    "touch bar: 20 swipes left, then 20 swipes right, approximately 1 for each second" \
    "1. Swipe LEFT 20 times, approximately 1 swipe for each second." \
    "2. Swipe RIGHT 20 times, approximately 1 swipe for each second."

"$PROBE" clear >/dev/null 2>&1
echo
echo "Done. Tell the agent."
