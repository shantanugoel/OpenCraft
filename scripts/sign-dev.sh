#!/bin/sh
# Build the GUI application, put it in an application bundle, and sign the
# bundle with a stable identity.
#
# macOS connects the Accessibility permission to the code signature. The
# linker gives each build a new ad-hoc signature, so the permission of the
# last build does not apply to a new build. A signature from one identity
# stays the same for each build. See docs/kb/macos-permissions.md.
#
# The bundle is target/<profile>/OpenCraft.app. macOS gives a program that
# starts from a terminal the permissions of the terminal. A bundle that
# starts through `open`, the Finder, or the Dock has its own permissions.
#
# Usage: scripts/sign-dev.sh [--release] [--run]
#
#   --release   Build and sign the release profile. The default is debug.
#   --run       Start the bundle with `open` after the signature.
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

bundle="target/$profile/OpenCraft.app"
rm -rf "$bundle"
mkdir -p "$bundle/Contents/MacOS"
cp "target/$profile/opencraft" "$bundle/Contents/MacOS/opencraft"
cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>opencraft</string>
    <key>CFBundleIdentifier</key><string>$IDENTIFIER</string>
    <key>CFBundleName</key><string>OpenCraft</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign "$identity" --identifier "$IDENTIFIER" "$bundle"
echo "Signed $bundle with the identifier $IDENTIFIER."

if [ "$run" -eq 1 ]; then
    exec open "$bundle"
fi
