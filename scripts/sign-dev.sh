#!/bin/sh
# Build the GUI application and sign it with a stable identity.
#
# macOS connects the Accessibility permission to the code signature. The
# linker gives each build a new ad-hoc signature, so the permission of the
# last build does not apply to a new build. A signature from one identity
# stays the same for each build. See docs/kb/macos-permissions.md.
#
# Usage: scripts/sign-dev.sh [--release] [--run]
#
#   --release   Build and sign the release profile. The default is debug.
#   --run       Start the signed program after the signature.
#
# The identity is the value of OPENCRAFT_SIGN_IDENTITY: a name or a SHA-1
# hash that `security find-identity -v -p codesigning` shows. If the variable
# is not set, the script uses the first identity in that list.

set -eu

IDENTIFIER="opencraft-dev"
profile="debug"
run=0
for arg in "$@"; do
    case "$arg" in
        --release) profile="release" ;;
        --run) run=1 ;;
        *) echo "usage: $0 [--release] [--run]" >&2; exit 2 ;;
    esac
done

cd "$(dirname "$0")/.."

identity="${OPENCRAFT_SIGN_IDENTITY:-}"
if [ -z "$identity" ]; then
    identity="$(security find-identity -v -p codesigning | awk '/^ *1\)/ {print $2}')"
fi
if [ -z "$identity" ]; then
    cat >&2 <<'MSG'
No code signing identity found.
Make one: open Keychain Access, select Certificate Assistant > Create a
Certificate, give a name, and select the certificate type "Code Signing".
Then set OPENCRAFT_SIGN_IDENTITY to that name and start this script again.
MSG
    exit 1
fi

if [ "$profile" = "release" ]; then
    cargo build --release -p opencraft-app
else
    cargo build -p opencraft-app
fi

binary="target/$profile/opencraft"
codesign --force --sign "$identity" --identifier "$IDENTIFIER" "$binary"
echo "Signed $binary with the identifier $IDENTIFIER."

if [ "$run" -eq 1 ]; then
    exec "$binary"
fi
