#!/usr/bin/env bash
# Record the input of each control of the connected device.
# The probe reads the descriptor and shows one line for each gesture. It
# goes to the next line by itself. The steps are in
# docs/ADDING-A-DEVICE-MODEL.md, section "Record the input". The file format
# is in crates/core/tests/fixtures/input/README.md.
#
# Usage: scripts/record-input.sh <directory> [--from <step number>]
#
# The directory is the --out directory of `opencraft-probe record`. The
# command does not choose a directory. Give the directory, so a recording
# of one model does not replace the fixture files of another model.
#
# Needs: the device connected, VSD Craft closed, and cargo.

set -eu

if [ "$#" -lt 1 ] || [ "${1#-}" != "$1" ]; then
  echo "Usage: scripts/record-input.sh <directory> [--from <step number>]" >&2
  exit 1
fi
out="$1"
shift

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "Quit VSD Craft first. Building the probe..."
cargo build --release --quiet
exec "${CARGO_TARGET_DIR:-$ROOT/target}/release/opencraft-probe" record --out "$out" "$@"
