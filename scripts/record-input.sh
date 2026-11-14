#!/usr/bin/env bash
# Record the input of each control of the connected device.
# The probe reads the descriptor and shows one line for each gesture. It
# goes to the next line by itself. The steps are in
# docs/ADDING-A-DEVICE-MODEL.md, section "Record the input". The file format
# is in crates/core/tests/fixtures/input/README.md.
#
# Usage: scripts/record-input.sh [--from <step number>]
#
# Needs: the device connected, VSD Craft closed, and cargo.

set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "Quit VSD Craft first. Building the probe..."
cargo build --release --quiet
exec "${CARGO_TARGET_DIR:-$ROOT/target}/release/opencraft-probe" record "$@"
