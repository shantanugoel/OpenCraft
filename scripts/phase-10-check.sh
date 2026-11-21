#!/usr/bin/env bash
# Guided owner check script for Phase 10 (issue #202).
# Prompts advance automatically after each yes/no answer and save a summary
# to /tmp/opencraft-phase-10-answers.txt.
#
# Usage: scripts/phase-10-check.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
ANSWERS="/tmp/opencraft-phase-10-answers.txt"
PROFILE="$ROOT/docs/checks/phase-10-checks.opencraft"
SCRATCH="/tmp/opencraft-phase-10"
PRESS_FILE="/tmp/opencraft-press.txt"

cd "$ROOT"

: > "$ANSWERS"
echo "# Phase 10 owner check answers ($(date '+%Y-%m-%d %H:%M'))" >> "$ANSWERS"

ask_yn() {
  local id="$1"
  local prompt="$2"
  local ans=""
  printf "\n[%s] %s [y/n]: " "$id" "$prompt"
  while true; do
    IFS= read -r -s -n 1 ans || exit 1
    case "$ans" in
      y|Y)
        printf "yes\n"
        printf "%s: yes — %s\n" "$id" "$prompt" >> "$ANSWERS"
        break
        ;;
      n|N)
        printf "no\n"
        printf "%s: no — %s\n" "$id" "$prompt" >> "$ANSWERS"
        break
        ;;
    esac
  done
}

echo "=== Phase 10 Owner Check (Issue #202) ==="
echo "1. Quit your installed OpenCraft from the menu bar item first."
printf "Press Enter when no OpenCraft process is running... "
IFS= read -r _

echo ""
echo "Building release binaries (opencraft-cli and opencraft-app)..."
cargo build --release --quiet -p opencraft-cli -p opencraft-app

echo ""
echo "=== Step 0: Record press and release times (/tmp/opencraft-press.txt) ==="
echo "When recording starts:"
echo "  1. Press key 1 short, one time. Wait 2 seconds."
echo "  2. Press and hold key 1 for ~1 second. Wait 2 seconds."
echo "  3. Press key 1 two times fast. Wait 2 seconds."
echo "  4. Press Enter to stop recording and continue."
printf "Press Enter to start recording... "
IFS= read -r _

rm -f "$PRESS_FILE"
"$TARGET_DIR/release/opencraft-probe" raw --seconds 60 --until-enter --save "$PRESS_FILE"
if [ -s "$PRESS_FILE" ]; then
  echo "Saved press recording to $PRESS_FILE."
  echo "Step 0: recorded $PRESS_FILE" >> "$ANSWERS"
else
  echo "Warning: $PRESS_FILE is empty or missing."
  echo "Step 0: missing $PRESS_FILE" >> "$ANSWERS"
fi

echo ""
echo "=== Starting Phase 10 build with scratch profiles directory ==="
rm -rf "$SCRATCH" && mkdir -p "$SCRATCH"
OPENCRAFT_PROFILES_DIR="$SCRATCH" "$TARGET_DIR/release/opencraft" > "$SCRATCH/app.log" 2>&1 &
APP_PID=$!
cleanup() {
  if kill -0 "$APP_PID" 2>/dev/null; then
    kill -TERM "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

sleep 2
if ! kill -0 "$APP_PID" 2>/dev/null; then
  echo "Error: OpenCraft exited during startup. Log ($SCRATCH/app.log):" >&2
  cat "$SCRATCH/app.log" >&2 || true
  exit 1
fi
echo "OpenCraft is running (PID $APP_PID)."
echo "In the sidebar, click '+' -> 'Import a Profile...' and select:"
echo "  $PROFILE"
echo "Open a plain text editor with an empty document and put the cursor in it."
printf "Press Enter once the profile is imported and visible on the N4 Pro... "
IFS= read -r _

ask_yn "1-single-delay" "Press key 1 (Press only): did 'now ' appear immediately with no pause?"
ask_yn "2a-short" "Press key 2 (Gestures) short: did 'short ' appear (and not 'long ' or 'double ')?"
ask_yn "2b-long" "Hold key 2 (Gestures) ~1 s then release: did 'long ' appear while down (and no 'short ' on release)?"
ask_yn "2c-double" "Press key 2 (Gestures) two times fast: did 'double ' appear (and no 'short ')?"
ask_yn "3a-inert" "Select key 3 (PTT Off) in the window: does Long press show 'Does not run: the Press action holds the key'?"
ask_yn "3b-ptt-hold" "In a call app (muted), hold key 3 (PTT Off): did key 3 show red PTT Live and keep the mic open while down?"
ask_yn "3c-ptt-release" "Release key 3: did the mic mute again and key 3 return to PTT Off without sticking space?"
ask_yn "4a-toggle-on" "Press key 4 (Focus): did key 4 switch to green Break with no border frame on device and window?"
ask_yn "4b-toggle-off" "Press key 4 (Break) again: did key 4 return to blue Focus on device and window?"
ask_yn "5-cycle" "Press key 5 (Cycle) three times: did it type 'one one two ' and move the Next badge 1 -> 2 -> 3 -> 1?"
ask_yn "6a-timer-bell" "Short-press key 6 (00:05): did it frame, count down to 00:00, and sound the system bell?"
ask_yn "6b-timer-reset" "Start key 6 again, wait 2 s, then hold 1 s: did it reset to 00:05 with no frame and no bell?"
ask_yn "7a-knob-vol" "Turn knob 1 right and left: did Mac sound volume change and show on bar panel 1?"
ask_yn "7b-knob-push" "Push knob 1: did bar panel 1 switch to Brightness?"
ask_yn "7c-knob-bright" "Turn knob 1 right and left: did N4 Pro display brightness change?"
ask_yn "7d-knob-wrap" "Push knob 1 again: did bar panel 1 return to Volume?"

printf "\n[8-p11-newline] Look-ahead for Phase 11 (fact 14): select key 7, try Option-Return or pasting two lines in Title. Type a short note on what happened and press Enter: "
IFS= read -r p11_note
printf "8-p11-newline: %s\n" "$p11_note" >> "$ANSWERS"

echo ""
echo "=== Done! ==="
echo "Answers saved to: $ANSWERS"
echo "Press recording:  $PRESS_FILE"
echo "Stopping scratch OpenCraft. Please restart your installed OpenCraft."
