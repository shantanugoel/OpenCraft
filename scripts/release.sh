#!/bin/sh
# Make the release of OpenCraft for macOS: the bundle OpenCraft.app with a
# Developer ID signature, and one disk image (DMG) for Apple silicon. The
# decision is docs/decisions/0025-release-form.md. The facts are in
# docs/kb/macos-distribution.md.
#
# Usage: scripts/release.sh [--notarize-only]
#
#   (no option)       Check the licence notices, build, make the bundle,
#                     sign it, make the disk image, and sign the disk image.
#                     If OPENCRAFT_NOTARY_PROFILE is set, also notarize and
#                     staple the bundle and the disk image.
#   --notarize-only   Do not build. Start from the signed bundle of an
#                     earlier run, notarize and staple it, make the disk
#                     image again, and notarize and staple the disk image.
#
# The output is in target/release-bundle/:
#
#   OpenCraft.app                         The signed bundle
#   OpenCraft-<version>-arm64.dmg         The disk image
#   OpenCraft-<version>-arm64.dmg.sha256  The SHA-256 of the disk image
#
# Environment:
#
#   OPENCRAFT_SIGN_IDENTITY   A name or a SHA-1 hash that
#                             `security find-identity -v -p codesigning`
#                             shows. If it is not set, the script uses the
#                             first "Developer ID Application" identity.
#   OPENCRAFT_NOTARY_PROFILE  The name of a keychain profile of
#                             `xcrun notarytool store-credentials`. If it is
#                             not set, the script does not notarize.
#
# Do not write the name of an identity, a team ID, or a profile in the
# repository.

set -eu

NAME="OpenCraft"
IDENTIFIER="com.shantanugoel.opencraft"
EXECUTABLE="opencraft"
ARCH="arm64"
# The lowest macOS version that the dependencies need. See
# docs/kb/macos-distribution.md.
MINIMUM_SYSTEM="12.0"
ICON="assets/macos/OpenCraft.icns"
# The file of the licence notices (decision 0026). scripts/notices.sh makes
# it. The script stops if the file is missing or is not correct for the
# crates of the build, and copies it into Contents/Resources of the bundle.
NOTICES="THIRD-PARTY-NOTICES.txt"

notarize_only=0
for arg in "$@"; do
    case "$arg" in
        --notarize-only) notarize_only=1 ;;
        *) echo "usage: $0 [--notarize-only]" >&2; exit 2 ;;
    esac
done

cd "$(dirname "$0")/.."

fail() {
    echo "release.sh: $*" >&2
    exit 1
}

step() {
    echo
    echo "== $*"
}

# The version of the application crate. It is the workspace version in
# Cargo.toml.
version="$(cargo pkgid -p opencraft-app | sed 's/.*[#@]//')"
echo "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' ||
    fail "the version '$version' is not three numbers. macOS needs three numbers in CFBundleShortVersionString."

out="target/release-bundle"
app="$out/$NAME.app"
dmg="$out/$NAME-$version-$ARCH.dmg"
profile="${OPENCRAFT_NOTARY_PROFILE:-}"

identity="${OPENCRAFT_SIGN_IDENTITY:-}"
if [ -z "$identity" ]; then
    identity="$(security find-identity -v -p codesigning |
        awk '/"Developer ID Application: / { print $2; exit }')"
fi
[ -n "$identity" ] || fail "no \"Developer ID Application\" identity is in the keychain. Set OPENCRAFT_SIGN_IDENTITY."

# Write the Info.plist of the bundle to the standard output.
info_plist() {
    # The copyright line of the licence, with XML entities.
    copyright="$(grep -m 1 '^Copyright' LICENSE |
        sed 's/(c)/©/; s/&/\&amp;/g; s/</\&lt;/g; s/>/\&gt;/g')"
    cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>en</string>
    <key>CFBundleDisplayName</key><string>$NAME</string>
    <key>CFBundleExecutable</key><string>$EXECUTABLE</string>
    <key>CFBundleIconFile</key><string>$NAME</string>
    <key>CFBundleIdentifier</key><string>$IDENTIFIER</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleName</key><string>$NAME</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
    <key>LSMinimumSystemVersion</key><string>$MINIMUM_SYSTEM</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSHumanReadableCopyright</key><string>$copyright</string>
</dict>
</plist>
PLIST
}

# The key in the notices file must be the key of the crates of Cargo.lock.
# This check needs no cargo-about and no network. CI compares the full file.
check_notices() {
    step "Check $NOTICES"
    [ -f "$NOTICES" ] || fail "$NOTICES does not exist. Run scripts/notices.sh and commit the file."
    key="$(scripts/notices.sh --key)"
    grep -qx "Key: $key" "$NOTICES" ||
        fail "$NOTICES is not correct for Cargo.lock and about.toml. Run scripts/notices.sh and commit the file."
    echo "$NOTICES is correct for the crates of Cargo.lock."
}

build_bundle() {
    step "Build the release profile"
    cargo build --release -p opencraft-app
    binary="target/release/$EXECUTABLE"
    archs="$(lipo -archs "$binary")"
    [ "$archs" = "$ARCH" ] || fail "the executable is for '$archs', not for '$ARCH'. Use a Mac with Apple silicon."

    step "Make $app"
    rm -rf "$app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp "$binary" "$app/Contents/MacOS/$EXECUTABLE"
    cp "$ICON" "$app/Contents/Resources/$NAME.icns"
    cp "$NOTICES" "$app/Contents/Resources/$NOTICES"
    info_plist > "$app/Contents/Info.plist"
    plutil -lint "$app/Contents/Info.plist"
}

sign_app() {
    step "Sign $app (Developer ID, hardened runtime, secure timestamp)"
    # No entitlements. See decision 0025 before you add one.
    codesign --force --options runtime --timestamp --sign "$identity" "$app"
}

verify_app() {
    step "Verify the signature of $app"
    codesign --verify --deep --strict --verbose=2 "$app"
    echo "Entitlements (empty if none):"
    codesign -d --entitlements - "$app" 2>/dev/null || true
    details="$(codesign -d --verbose=2 "$app" 2>&1)"
    echo "$details" | grep -q "^Identifier=$IDENTIFIER\$" || fail "the identifier is not $IDENTIFIER."
    echo "$details" | grep -q 'flags=0x10000(runtime)' || fail "the signature has no hardened runtime."
    echo "$details" | grep -q '^Timestamp=' || fail "the signature has no secure timestamp."
    echo "$details" | grep -q '^Authority=Developer ID Application: ' ||
        fail "the identity is not a \"Developer ID Application\" identity."
    [ -s "$app/Contents/Resources/$NOTICES" ] || fail "the bundle has no $NOTICES."
    plist_version="$(plutil -extract CFBundleShortVersionString raw -o - "$app/Contents/Info.plist")"
    [ "$plist_version" = "$version" ] || fail "the bundle has the version $plist_version, not $version."
    echo "Identifier $IDENTIFIER, version $version, hardened runtime, secure timestamp, Developer ID, $NOTICES: correct."
    echo "Gatekeeper assessment of the bundle:"
    spctl --assess --type exec --verbose "$app" 2>&1 || true
}

make_dmg() {
    step "Make $dmg"
    staging="$out/dmg-staging"
    rm -rf "$staging" "$dmg" "$dmg.sha256"
    mkdir -p "$staging"
    ditto "$app" "$staging/$NAME.app"
    ln -s /Applications "$staging/Applications"
    hdiutil create -volname "$NAME $version" -srcfolder "$staging" \
        -fs HFS+ -format UDZO -ov "$dmg"
    rm -rf "$staging"
    codesign --force --timestamp --sign "$identity" "$dmg"
    codesign --verify --strict --verbose=2 "$dmg"
}

# Submit the file $1 to the notary service and wait for the result.
notarize() {
    step "Notarize $1 (this can take some minutes)"
    result="$(xcrun notarytool submit "$1" --keychain-profile "$profile" \
        --wait --timeout 1h --output-format json)" ||
        { echo "$result" >&2; fail "notarytool failed for $1."; }
    echo "$result"
    # notarytool has no documented exit status for a rejected submission.
    # Thus read the status.
    status="$(printf '%s' "$result" | plutil -extract status raw -o - - 2>/dev/null || true)"
    if [ "$status" != "Accepted" ]; then
        id="$(printf '%s' "$result" | plutil -extract id raw -o - - 2>/dev/null || true)"
        if [ -n "$id" ]; then
            xcrun notarytool log "$id" --keychain-profile "$profile" >&2 || true
        fi
        fail "the notary service did not accept $1 (status '$status')."
    fi
}

notarize_app() {
    zip="$out/$NAME-notarize.zip"
    rm -f "$zip"
    ditto -c -k --keepParent "$app" "$zip"
    notarize "$zip"
    rm -f "$zip"
    step "Staple the ticket to $app"
    xcrun stapler staple "$app"
    xcrun stapler validate "$app"
    spctl --assess --type exec --verbose "$app"
}

notarize_dmg() {
    notarize "$dmg"
    step "Staple the ticket to $dmg"
    xcrun stapler staple "$dmg"
    xcrun stapler validate "$dmg"
    spctl --assess --type open --context context:primary-signature --verbose "$dmg"
}

checksum() {
    step "SHA-256 of the disk image"
    # The stapler changes the file. Thus this is the last step.
    name="$(basename "$dmg")"
    (cd "$out" && shasum -a 256 "$name" | tee "$name.sha256")
}

if [ "$notarize_only" -eq 1 ]; then
    [ -n "$profile" ] || fail "--notarize-only needs OPENCRAFT_NOTARY_PROFILE."
    [ -d "$app" ] || fail "$app does not exist. Run scripts/release.sh first."
    verify_app
    notarize_app
    make_dmg
    notarize_dmg
    checksum
    exit 0
fi

check_notices
build_bundle
sign_app
verify_app
if [ -n "$profile" ]; then
    notarize_app
fi
make_dmg
if [ -n "$profile" ]; then
    notarize_dmg
else
    cat <<MSG

OPENCRAFT_NOTARY_PROFILE is not set, thus the script did not notarize.
The bundle and the disk image are signed, but they are NOT notarized.
Do not publish this disk image. To notarize it later, with no new build:

    OPENCRAFT_NOTARY_PROFILE=<profile> scripts/release.sh --notarize-only
MSG
fi
checksum
