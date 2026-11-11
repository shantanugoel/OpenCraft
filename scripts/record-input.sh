#!/usr/bin/env bash
# Guided recording of the input reports of each control of the N4 Pro.
# The procedure is in GitHub issue #11. The file format is in
# crates/core/tests/fixtures/input/README.md.
#
# Usage: scripts/record-input.sh [output-dir] [--from <step>]
#
# Needs: the device connected, VSD Craft closed, and cargo (to build the probe).
# Works with bash 3.2 (the version on macOS).

set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/crates/core/tests/fixtures/input"
FROM=1
LIST=0
PROBE="${CARGO_TARGET_DIR:-$ROOT/target}/release/opencraft-probe"

usage() {
    echo "Usage: $0 [output-dir] [--from <step number>] [--list]"
    echo "  output-dir   where to write the files (default: crates/core/tests/fixtures/input)"
    echo "  --from N     start at step N (use it to continue after a stop)"
    echo "  --list       print the file name and the gesture of each step, then exit"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --from)
            if [ $# -lt 2 ]; then
                usage
                exit 2
            fi
            FROM="$2"
            shift 2
            ;;
        --list)
            LIST=1
            shift
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        -*)
            usage
            exit 2
            ;;
        *)
            OUT="$1"
            shift
            ;;
    esac
done

case "$FROM" in
    '' | *[!0-9]*)
        echo "--from needs a step number."
        exit 2
        ;;
esac

# ---------------------------------------------------------------- steps ----
# One gesture for each step. The arrays hold the file name, the note that goes
# in the file, the time limit (0 = stop with Enter, N = fixed N seconds), the
# instruction, and the controls of the step ("key 6", "bar 1", "key 7 8", or
# nothing). The script prints the physical position of these controls, because
# the slot numbers do not stay on the screens.

S_NAME=()
S_NOTE=()
S_SECS=()
S_TEXT=()
S_WHERE=()

add_step() {
    S_NAME+=("$1")
    S_NOTE+=("$2")
    S_SECS+=("$3")
    S_TEXT+=("$4")
    S_WHERE+=("${5:-}")
}

k=6
while [ "$k" -le 15 ]; do
    add_step "key-$(printf '%02d' "$k")" "key $k: press and release" 0 \
        "Press the key of slot $k. Release it. Do this one time." "key $k"
    k=$((k + 1))
done

pos=("" first second third fourth)
k=1
while [ "$k" -le 4 ]; do
    p="${pos[$k]}"
    add_step "knob-$k-left" "knob $k ($p from the left): turn left about 5 clicks" 0 \
        "Knob $k is the $p knob from the left. Turn it LEFT about 5 clicks. Do not push it."
    add_step "knob-$k-right" "knob $k ($p from the left): turn right about 5 clicks" 0 \
        "Knob $k is the $p knob from the left. Turn it RIGHT about 5 clicks. Do not push it."
    add_step "knob-$k-push" "knob $k ($p from the left): push and release" 0 \
        "Knob $k is the $p knob from the left. Push it in. Release it. Do this one time. Do not turn it."
    k=$((k + 1))
done

k=1
while [ "$k" -le 4 ]; do
    add_step "bar-tap-$k" "bar panel $k: tap" 0 \
        "Tap bar panel $k on the touch bar. Do this one time. Lift your finger at once." "bar $k"
    k=$((k + 1))
done

add_step "bar-hold-1" "bar panel 1: press and hold for 2 seconds, then release" 0 \
    "Put one finger on bar panel 1 on the touch bar. Hold it there for 2 seconds. Lift your finger." "bar 1"
add_step "swipe-left-1" "touch bar: swipe left, 1 time" 0 \
    "Swipe one finger LEFT across the touch bar. Do this one time."
add_step "swipe-right-1" "touch bar: swipe right, 1 time" 0 \
    "Swipe one finger RIGHT across the touch bar. Do this one time."
add_step "swipe-left-20" "touch bar: swipe left, 20 times" 0 \
    "Swipe LEFT across the touch bar 20 times. Wait about 1 second between swipes. Count them. Press Enter after the 20th."
add_step "swipe-right-20" "touch bar: swipe right, 20 times" 0 \
    "Swipe RIGHT across the touch bar 20 times. Wait about 1 second between swipes. Count them. Press Enter after the 20th."
add_step "slide-left-to-right" "touch bar: one finger moves slowly from the left end to the right end" 0 \
    "Put one finger on the LEFT end of the touch bar. Move it slowly to the RIGHT end in about 3 seconds. Lift your finger."
add_step "slide-top-to-bottom" "touch bar: one finger moves slowly from the top edge to the bottom edge" 0 \
    "Put one finger on the TOP edge of the touch bar, in the middle. Move it slowly down to the BOTTOM edge in about 2 seconds. Lift your finger."
add_step "corner-top-left" "touch bar: touch at the top-left corner" 0 \
    "Touch the TOP-LEFT corner of the touch bar. Lift your finger."
add_step "corner-top-right" "touch bar: touch at the top-right corner" 0 \
    "Touch the TOP-RIGHT corner of the touch bar. Lift your finger."
add_step "corner-bottom-left" "touch bar: touch at the bottom-left corner" 0 \
    "Touch the BOTTOM-LEFT corner of the touch bar. Lift your finger."
add_step "corner-bottom-right" "touch bar: touch at the bottom-right corner" 0 \
    "Touch the BOTTOM-RIGHT corner of the touch bar. Lift your finger."
add_step "knob-1-fast" "knob 1 (first from the left): turn as fast as possible for 5 seconds" 5 \
    "Hold knob 1 (the first knob from the left). Press Enter. When you see 'Hexdumping raw input reports for 5s', turn the knob as fast as you can, in one direction, for 5 seconds. The recording stops by itself."
add_step "keys-two" "keys 7 and 8: pressed at the same time, then released" 0 \
    "Press the keys of slot 7 and slot 8 at the same time. Release them. Do this one time." "key 7 8"
add_step "key-hold" "key 8: press and hold for 3 seconds, then release" 0 \
    "Press the key of slot 8. Hold it for 3 seconds. Release it." "key 8"

TOTAL=${#S_NAME[@]}

if [ "$LIST" = 1 ]; then
    i=0
    while [ "$i" -lt "$TOTAL" ]; do
        printf '%02d-%s.txt\t%s\n' "$((i + 1))" "${S_NAME[$i]}" "${S_NOTE[$i]}"
        i=$((i + 1))
    done
    exit 0
fi

# -------------------------------------------------------------- helpers ----

# Read one answer. An end of input (no terminal) counts as "q".
ASK=""
ask() {
    ASK=""
    if ! read -r -p "$1" ASK; then
        ASK="q"
        echo
    fi
}

# Ask for $2 slot numbers between $3 and $4, with no repeat. Result in SLOTS.
SLOTS=""
ask_slots() {
    while true; do
        ask "$1: "
        if [ "$ASK" = "q" ]; then
            echo "Stopped."
            exit 1
        fi
        local clean count seen ok n
        clean="$(echo "$ASK" | tr ',' ' ')"
        count=0
        seen=" "
        ok=1
        for n in $clean; do
            case "$n" in
                '' | *[!0-9]*) ok=0 ;;
                *)
                    if [ "$n" -lt "$3" ] || [ "$n" -gt "$4" ]; then ok=0; fi
                    case "$seen" in *" $n "*) ok=0 ;; esac
                    seen="$seen$n "
                    ;;
            esac
            count=$((count + 1))
        done
        if [ "$ok" = 1 ] && [ "$count" = "$2" ]; then
            # shellcheck disable=SC2086
            set -- $clean
            SLOTS="$*"
            return
        fi
        echo "  Type $2 different numbers from $3 to $4, with a space between them."
    done
}

# The slot numbers at each physical position, left to right. Part 1 sets them.
ROW1=""
ROW2=""
BAR=""
ORD=("" 1st 2nd 3rd 4th 5th)

# Read ROW1, ROW2, and BAR from layout.txt. Return 1 if a line is missing.
load_layout() {
    ROW1="$(sed -n 's/^key_row_1: *//p' "$1")"
    ROW2="$(sed -n 's/^key_row_2: *//p' "$1")"
    BAR="$(sed -n 's/^bar_panels: *//p' "$1")"
    [ -n "$ROW1" ] && [ -n "$ROW2" ] && [ -n "$BAR" ]
}

# Print the position of slot $2 in the list $1, with the text $3 before it and
# the text $4 after it. Return 1 if the slot is not in the list.
find_in() {
    local i n
    i=1
    for n in $1; do
        if [ "$n" = "$2" ]; then
            echo "$3${ORD[$i]} $4"
            return 0
        fi
        i=$((i + 1))
    done
    return 1
}

# Print where the controls of a step are. $1 is "key" or "bar", the other
# arguments are slot numbers.
print_positions() {
    local kind="$1" slot
    shift
    for slot in "$@"; do
        if [ "$kind" = "bar" ]; then
            find_in "$BAR" "$slot" "  Position of bar panel $slot: the " "quarter of the touch bar from the left" ||
                echo "  Position of bar panel $slot: not known"
        else
            find_in "$ROW1" "$slot" "  Position of slot $slot: top row, " "key from the left" ||
                find_in "$ROW2" "$slot" "  Position of slot $slot: second row, " "key from the left" ||
                echo "  Position of slot $slot: not known"
        fi
    done
}

count_reports() {
    if [ -f "$1" ]; then
        grep -vc '^#' "$1" || true
    else
        echo 0
    fi
}

# ------------------------------------------------------------ preparation ----

echo "Record the input of each control of the N4 Pro"
echo "==============================================="
echo
echo "Quit VSD Craft now. Two programs must not use the device at the same time."
echo "The device must be connected."
echo
echo "Building the probe..."
if ! (cd "$ROOT" && cargo build --release); then
    echo "The build failed."
    exit 1
fi
if [ ! -x "$PROBE" ]; then
    echo "Cannot find the probe at: $PROBE"
    exit 1
fi
mkdir -p "$OUT" || exit 1
OUT="$(cd "$OUT" && pwd)"

echo
echo "Files go to: $OUT"
echo

# ----------------------------------------------------------------- layout ----

echo "Part 1: the slot numbers"
echo "------------------------"
echo "The probe now draws its slot number on each key and on each bar panel."
echo "Look at the device and read the numbers."
echo
ask "Press Enter to draw the numbers (q = stop): "
if [ "$ASK" = "q" ]; then
    echo "Stopped."
    exit 1
fi
if ! "$PROBE" test --keep --seconds 1; then
    echo "The probe could not use the device. Check the cable and quit VSD Craft."
    exit 1
fi

USE_OLD=0
if [ -f "$OUT/layout.txt" ] && load_layout "$OUT/layout.txt"; then
    echo
    echo "A saved layout is in $OUT/layout.txt:"
    echo "  Key row 1 (top row): $ROW1"
    echo "  Key row 2 (second row): $ROW2"
    echo "  Bar panels: $BAR"
    ask "Enter = use it, n = type the numbers again, q = stop: "
    if [ "$ASK" = "q" ]; then
        echo "Stopped."
        exit 1
    fi
    if [ "$ASK" != "n" ]; then
        USE_OLD=1
    fi
fi
if [ "$USE_OLD" = 0 ]; then
    echo
    echo "Type the numbers from left to right. Put a space between them."
    echo "The numbers of the keys are from 6 to 15. The numbers of the bar panels are from 1 to 4."
    echo
    ask_slots "Key row 1 (top row), 5 numbers" 5 6 15
    ROW1="$SLOTS"
    ask_slots "Key row 2 (second row), 5 numbers" 5 6 15
    ROW2="$SLOTS"
    ask_slots "Bar panels, 4 numbers" 4 1 4
    BAR="$SLOTS"
    {
        echo "# Slot numbers at each physical position, left to right."
        echo "# The owner read them from the screens after: opencraft-probe test --keep"
        echo "key_row_1: $ROW1"
        echo "key_row_2: $ROW2"
        echo "bar_panels: $BAR"
        echo "# Knobs: knob 1 to 4 are the knobs from left to right."
        echo "# The recordings of the knob steps confirm which code is which knob."
    } >"$OUT/layout.txt"
    echo
    echo "Saved $OUT/layout.txt"
fi

# ------------------------------------------------------------------ steps ----

echo
echo "Part 2: the gestures ($TOTAL steps)"
echo "-----------------------------------"
echo "Do one gesture for each step. Do not touch anything else during a recording."
echo "To stop a recording: press Enter (a step with a 5 second limit stops by itself)."
echo "The slot numbers go off the screens after some time. That is not a fault."
echo "Each step gives the position of its control. If a step records no report, record it again."

i=$((FROM - 1))
if [ "$i" -lt 0 ]; then i=0; fi
KEPT=0
SKIPPED=0
while [ "$i" -lt "$TOTAL" ]; do
    n=$((i + 1))
    name="${S_NAME[$i]}"
    note="${S_NOTE[$i]}"
    secs="${S_SECS[$i]}"
    file="$OUT/$(printf '%02d' "$n")-$name.txt"

    while true; do
        echo
        echo "=== Step $n of $TOTAL: $name ==="
        echo "${S_TEXT[$i]}"
        if [ -n "${S_WHERE[$i]}" ]; then
            # shellcheck disable=SC2086
            print_positions ${S_WHERE[$i]}
        fi
        echo
        ask "Enter = start recording, s = skip, q = stop: "
        if [ "$ASK" = "q" ]; then
            echo
            echo "Stopped before step $n. The panels keep their numbers."
            echo "To continue: $0 \"$OUT\" --from $n"
            exit 1
        fi
        if [ "$ASK" = "s" ]; then
            rm -f "$file"
            SKIPPED=$((SKIPPED + 1))
            echo "Skipped."
            break
        fi

        if [ "$secs" = 0 ]; then
            "$PROBE" raw --save "$file" --note "$note" --until-enter --seconds 120
        else
            "$PROBE" raw --save "$file" --note "$note" --seconds "$secs"
        fi
        status=$?
        reports="$(count_reports "$file")"
        echo
        if [ "$status" != 0 ]; then
            echo "The probe reported an error."
        fi
        echo "File: $(basename "$file") has $reports report(s)."
        if [ "$reports" = 0 ]; then
            echo "No report. Did you do the gesture? Is the device connected?"
        fi

        ask "Enter = keep, r = record again, s = skip, q = stop: "
        if [ "$ASK" = "r" ]; then
            continue
        fi
        if [ "$ASK" = "s" ]; then
            rm -f "$file"
            SKIPPED=$((SKIPPED + 1))
            echo "Skipped. The file is removed."
            break
        fi
        if [ "$ASK" = "q" ]; then
            rm -f "$file"
            echo
            echo "Stopped at step $n. This recording is removed."
            echo "To continue: $0 \"$OUT\" --from $n"
            exit 1
        fi
        KEPT=$((KEPT + 1))
        break
    done
    i=$((i + 1))
done

# ----------------------------------------------------------------- finish ----

echo
echo "Clearing the panels..."
"$PROBE" clear || echo "Could not clear the panels."

echo
echo "Done. $KEPT recording(s) kept, $SKIPPED step(s) skipped."
echo "The files are in: $OUT"
echo "Check that no file has a serial number, then commit the files."
