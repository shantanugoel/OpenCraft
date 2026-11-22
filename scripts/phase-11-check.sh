#!/usr/bin/env bash
# Guided owner check script for Phase 11 (issue #459).
#
# The script copies the profiles directory of the owner to a temporary
# directory, adds the check profile of docs/checks/phase-11-checks.opencraft,
# and starts a release build on the copy. The new build never opens the
# profiles of the owner: a Phase 11 build writes schema version 8, and release
# 0.6.0 cannot read that version. The copy is removed at the end, and the
# installed OpenCraft starts again, also when the owner quit it before the
# check. A Ctrl-C at a prompt stops the build, removes the copy, starts the
# installed OpenCraft, and exits with a code that is not 0.
#
# Each question takes one key: y or n. The answers go to
# /tmp/opencraft-phase-11-answers.txt.
#
# Usage: scripts/phase-11-check.sh [--dry-run] [--no-device]
#
#   --dry-run    Print the steps and test the check profile. Do not build or
#                start OpenCraft, and do not wait for an answer.
#   --no-device  Start the build with OPENCRAFT_NO_DEVICE=1, for a look at the
#                window with no device.

set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
ANSWERS="/tmp/opencraft-phase-11-answers.txt"
CHECK_PROFILE="$ROOT/docs/checks/phase-11-checks.opencraft"
INSTALLED="/Applications/OpenCraft.app"
INSTALLED_BINARY="$INSTALLED/Contents/MacOS/opencraft"
PROFILES_SRC="$HOME/Library/Application Support/OpenCraft/profiles"
# Test hook: OPENCRAFT_PHASE11_RESTART_LOG names a file. The script writes the
# start command of the installed OpenCraft to that file, and starts nothing.
RESTART_LOG="${OPENCRAFT_PHASE11_RESTART_LOG:-}"

DRY_RUN=0
NO_DEVICE=0
for argument in "$@"; do
  case "$argument" in
    --dry-run) DRY_RUN=1 ;;
    --no-device) NO_DEVICE=1 ;;
    -h | --help)
      awk 'NR > 1 { if ($0 !~ /^#/) exit; sub(/^# ?/, ""); print }' "$0"
      exit 0
      ;;
    *)
      echo "Unknown argument: $argument" >&2
      echo "Usage: scripts/phase-11-check.sh [--dry-run] [--no-device]" >&2
      exit 2
      ;;
  esac
done

if ! grep -q 'pub const SCHEMA_VERSION: u32 = 8;' "$ROOT/crates/engine/src/store.rs" 2>/dev/null; then
  echo "This checkout has no Phase 11 profile schema (schema version 8)." >&2
  echo "Use the branch or the release of Phase 11." >&2
  exit 1
fi

if [ ! -f "$CHECK_PROFILE" ]; then
  echo "Missing $CHECK_PROFILE" >&2
  exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/opencraft-phase-11.XXXXXX")" || exit 1
SCRATCH_PROFILES="$SCRATCH/config/profiles"
CHECK_DIR="$SCRATCH_PROFILES/phase-11-checks"
APP_PID=""

ask_yn() {
  local id="$1" prompt="$2" answer=""
  if [ "$DRY_RUN" -eq 1 ]; then
    printf "\n[%s] %s [y/n]: (dry run)\n" "$id" "$prompt"
    printf "%s: not answered (dry run)\n" "$id" >>"$ANSWERS"
    return 0
  fi
  printf "\n[%s] %s [y/n]: " "$id" "$prompt"
  while true; do
    IFS= read -r -s -n 1 answer || exit 1
    case "$answer" in
      y | Y)
        printf "yes\n"
        printf "%s: yes - %s\n" "$id" "$prompt" >>"$ANSWERS"
        return 0
        ;;
      n | N)
        printf "no\n"
        printf "%s: no - %s\n" "$id" "$prompt" >>"$ANSWERS"
        return 0
        ;;
    esac
  done
}

# A check that does not stop the release: y, n, or s to skip it.
ask_optional() {
  local id="$1" prompt="$2" answer=""
  if [ "$DRY_RUN" -eq 1 ]; then
    printf "\n[%s] %s [y/n/s to skip]: (dry run)\n" "$id" "$prompt"
    printf "%s: not answered (dry run)\n" "$id" >>"$ANSWERS"
    return 0
  fi
  printf "\n[%s] (optional) %s [y/n/s to skip]: " "$id" "$prompt"
  while true; do
    IFS= read -r -s -n 1 answer || exit 1
    case "$answer" in
      y | Y)
        printf "yes\n"
        printf "%s: yes - %s\n" "$id" "$prompt" >>"$ANSWERS"
        return 0
        ;;
      n | N)
        printf "no\n"
        printf "%s: no - %s\n" "$id" "$prompt" >>"$ANSWERS"
        return 0
        ;;
      s | S)
        printf "skipped\n"
        printf "%s: skipped\n" "$id" >>"$ANSWERS"
        return 0
        ;;
    esac
  done
}

wait_enter() {
  if [ "$DRY_RUN" -eq 1 ]; then
    printf "%s (dry run)\n" "$1"
    return 0
  fi
  printf "%s " "$1"
  # A closed input (Ctrl-D, a pipe that ends) stops the script: cleanup runs.
  IFS= read -r _ || exit 1
}

stop_app() {
  if [ -n "$APP_PID" ] && kill -0 "$APP_PID" 2>/dev/null; then
    kill -TERM "$APP_PID" 2>/dev/null || true
    for _ in $(seq 1 60); do
      kill -0 "$APP_PID" 2>/dev/null || break
      sleep 0.5
    done
    kill -0 "$APP_PID" 2>/dev/null && kill -KILL "$APP_PID" 2>/dev/null
  fi
  APP_PID=""
}

start_installed() {
  # The procedure tells the owner to quit the installed OpenCraft before the
  # check. Thus the start at the end is right also when OpenCraft did not run
  # when this script started. When it still runs, `open` only brings it
  # forward and starts no second process.
  if [ "$DRY_RUN" -eq 1 ] || [ ! -d "$INSTALLED" ]; then
    return 0
  fi
  if [ -n "$RESTART_LOG" ]; then
    printf 'open -g -j -a %s --args --login\n' "$INSTALLED" >>"$RESTART_LOG"
    return 0
  fi
  # Give the device back to the installed OpenCraft.
  open -g -j -a "$INSTALLED" --args --login >/dev/null 2>&1 || true
}

CLEANED=0

cleanup() {
  # EXIT, INT, and TERM can arrive together: run the cleanup one time.
  if [ "$CLEANED" -eq 1 ]; then
    return 0
  fi
  CLEANED=1
  # A second Ctrl-C must not stop the cleanup before its end.
  trap '' INT TERM
  stop_app
  # The copy holds the profiles of the owner: remove it.
  rm -rf "$SCRATCH"
  start_installed
}
trap cleanup EXIT
# Ctrl-C exits with 130 and Ctrl-\ exits with 143, as a shell does.
# Exit runs cleanup after the signal trap.
trap 'exit 130' INT
trap 'exit 143' TERM

# The check profile is a profile export file. Take the profile file out of it:
# the table `[profile]` holds the profile, and its tables are one level deeper.
write_check_profile() {
  mkdir -p "$CHECK_DIR" || return 1
  awk '
    /^\[profile\]$/ { inside = 1; next }
    inside != 1 { next }
    /^\[\[profile\./ { sub(/^\[\[profile\./, "[["); print; next }
    /^\[profile\./ { sub(/^\[profile\./, "["); print; next }
    { print }
  ' "$CHECK_PROFILE" >"$CHECK_DIR/profile.toml" || return 1
  test -s "$CHECK_DIR/profile.toml"
}

echo "=== Phase 11 owner check (issue #459) ==="
echo "The check uses a copy of your profiles in a temporary directory."
echo "Your own profiles are not changed. The copy is removed at the end,"
echo "and your installed OpenCraft starts again."
echo ""

mkdir -p "$SCRATCH/config" || exit 1
if [ "$DRY_RUN" -eq 1 ]; then
  # A dry run reads no profile of the owner.
  mkdir -p "$SCRATCH_PROFILES" || exit 1
elif [ -d "$PROFILES_SRC" ]; then
  echo "Copying the profiles to the temporary directory (it is not changed)..."
  cp -R "$PROFILES_SRC" "$SCRATCH_PROFILES" || {
    echo "Cannot copy $PROFILES_SRC" >&2
    exit 1
  }
else
  mkdir -p "$SCRATCH_PROFILES" || exit 1
fi

write_check_profile || {
  echo "Cannot make the check profile from $CHECK_PROFILE" >&2
  exit 1
}

# The check profile is the active profile of the copy.
printf 'last_model = "n4-pro"\n\n[active]\nn4-pro = "phase-11-checks"\n' \
  >"$SCRATCH_PROFILES/state.toml" || exit 1

: >"$ANSWERS"
echo "# Phase 11 owner check answers ($(date '+%Y-%m-%d %H:%M'))" >>"$ANSWERS"
echo "# Check profile: docs/checks/phase-11-checks.opencraft" >>"$ANSWERS"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "Dry run: the check profile has $(wc -l <"$CHECK_DIR/profile.toml" | tr -d ' ') lines."
  head -3 "$CHECK_DIR/profile.toml" | sed 's/^/  /'
else
  if [ "$NO_DEVICE" -eq 0 ] && pgrep -f "$INSTALLED_BINARY" >/dev/null 2>&1; then
    echo "OpenCraft is running. Quit it from its menu bar item first: only one"
    echo "process can open the device."
    wait_enter "Press Enter when OpenCraft has stopped..."
    for _ in $(seq 1 20); do
      pgrep -f "$INSTALLED_BINARY" >/dev/null 2>&1 || break
      sleep 0.5
    done
    if pgrep -f "$INSTALLED_BINARY" >/dev/null 2>&1; then
      echo "OpenCraft still runs. Quit it, then run this script again." >&2
      exit 1
    fi
  fi

  echo ""
  echo "Building the release build of OpenCraft (opencraft-app)..."
  (cd "$ROOT" && cargo build --release --quiet -p opencraft-app) || {
    echo "The build failed." >&2
    exit 1
  }

  # Run the binary from a temporary directory, not from the checkout: a
  # checkout on a removable volume makes macOS ask for access at each start.
  mkdir -p "$SCRATCH/bin" || exit 1
  cp "$TARGET_DIR/release/opencraft" "$SCRATCH/bin/opencraft" || exit 1
  codesign -s - --force "$SCRATCH/bin/opencraft" >/dev/null 2>&1 || true

  echo ""
  echo "Starting the Phase 11 build with the copy of the profiles."
  # The variable OPENCRAFT_NO_DEVICE counts when it is set, also to 0: write
  # it only for --no-device. The start script keeps the PID of the binary.
  {
    echo "#!/bin/sh"
    echo "export OPENCRAFT_PROFILES_DIR=\"$SCRATCH_PROFILES\""
    if [ "$NO_DEVICE" -eq 1 ]; then
      echo "export OPENCRAFT_NO_DEVICE=1"
    fi
    echo "exec \"$SCRATCH/bin/opencraft\" \"\$@\""
  } >"$SCRATCH/start.sh"
  chmod +x "$SCRATCH/start.sh"
  nohup "$SCRATCH/start.sh" >>"$SCRATCH/app.log" 2>&1 &
  APP_PID=$!
  sleep 3
  if ! kill -0 "$APP_PID" 2>/dev/null; then
    echo "OpenCraft stopped during the start. The log:" >&2
    cat "$SCRATCH/app.log" >&2
    exit 1
  fi
  if [ "$NO_DEVICE" -eq 1 ]; then
    echo "The window shows the check profile with no device."
  else
    echo "The window shows the check profile. Its status must be 'Live on the device'."
  fi
  wait_enter "Press Enter when the window is open..."
fi

echo ""
echo "=== Step 1: drop an image and an app from Finder (KEY-01, KEY-13) ==="
echo "Key 1 (top row, first) and key 2 (top row, second) are empty."
ask_yn "1a-image" "Drag one image file from Finder onto key 1. Do the window and the device show the picture?"
ask_yn "1b-app" "Drag one .app from Finder onto key 2. Do the window and the device show the app icon?"

echo ""
echo "=== Step 2: the four keys on the device and in the window (ICON-06, P11-T1, ICON-04, ACT-31) ==="
echo "Look at key 1 (the image), key 3 (Menlo, italic, underlined), key 4 (the emoji at 80 pt),"
echo "and key 5 (the clock)."
ask_yn "2a-colour" "Are the colours of the four keys the same on the device and in the window?"
ask_yn "2b-sharp" "Is the small text sharp on the device (the clock and the label 'Menlo')?"

echo ""
echo "=== Step 3: move a key by a drag in the window (KEY-14) ==="
ask_yn "3a-move" "Drag key 3 onto key 6 in the window. Do the window and the device show the Menlo key at place 6?"
ask_yn "3b-undo" "Press Command-Z in the window. Does the Menlo key return to place 3?"

echo ""
echo "=== Optional checks (they do not stop the release) ==="
ask_optional "4-appearance" "Switch macOS to the Light appearance and back with the window open (System Settings > Appearance). Does the window change with it?"
ask_optional "5-voiceover" "Turn VoiceOver on, press Tab and Space a few times in the window, then turn it off. Does it speak sensible labels?"

# With --no-device, the installed OpenCraft can still own the device: do not
# send the owner to VSD Craft then.
if [ "$NO_DEVICE" -eq 0 ]; then
  echo ""
  echo "=== Optional look-ahead for Phase 12: VSD Craft (it opens the device) ==="
  echo "OpenCraft stops now, so that VSD Craft can open the device."
  stop_app
  if [ "$DRY_RUN" -eq 0 ]; then
    sleep 2
  fi
  ask_optional "6-vsd-export" "Open VSD Craft, export two scenes to files, then quit VSD Craft. Did it write two files?"
fi

echo ""
echo "=== Done ==="
echo "Answers saved to: $ANSWERS"
echo "Copy of the profiles: removed"
if [ "$DRY_RUN" -eq 0 ] && [ -d "$INSTALLED" ]; then
  echo "Your installed OpenCraft starts again."
elif [ "$DRY_RUN" -eq 0 ]; then
  echo "This Mac has no $INSTALLED. Start OpenCraft as usual."
fi
echo "Send the answers file to the agent."
