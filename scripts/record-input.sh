#!/usr/bin/env bash
# Record the input of each control of the N4 Pro.
# The probe shows one line for each gesture and goes to the next line by
# itself. The procedure is in GitHub issue #11. The file format is in
# crates/core/tests/fixtures/input/README.md.
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
