#!/bin/sh
# Write the Homebrew cask file of OpenCraft for one version. The decision is
# docs/decisions/0027-homebrew-cask.md. This script holds the only copy of
# the cask text. The tap repository holds the file that this script writes.
#
# Usage: scripts/homebrew-cask.sh [options] OUTPUT
#
#   OUTPUT                 The path of the cask file to write, for example
#                          ../homebrew-opencraft/Casks/opencraft.rb
#   --version VERSION      The version, three numbers. Use it only together
#                          with --sha256.
#   --sha256 SHA256        The SHA-256 of the disk image, 64 hexadecimal
#                          characters.
#   --allow-unnotarized    Do not check the notarization ticket of the disk
#                          image. Use it only for a test.
#
# With no --version and no --sha256, the script reads both from the one file
# target/release-bundle/*.dmg.sha256 that scripts/release.sh wrote. It then
# checks three things, and stops at the first error:
#
#   1. The disk image next to the .sha256 file has that SHA-256.
#   2. The disk image has a stapled notarization ticket
#      (xcrun stapler validate). The stapler changes the file, thus a disk
#      image from an earlier stage has a different SHA-256.
#   3. The name of the file is OpenCraft-<version>-arm64.dmg.

set -eu

cd "$(dirname "$0")/.."

fail() {
    echo "homebrew-cask.sh: $*" >&2
    exit 1
}

usage() {
    echo "usage: $0 [--version VERSION --sha256 SHA256] [--allow-unnotarized] OUTPUT" >&2
    exit 2
}

version=""
sha256=""
allow_unnotarized=0
output=""
while [ $# -gt 0 ]; do
    case "$1" in
        --version) [ $# -ge 2 ] || usage; version="$2"; shift 2 ;;
        --sha256) [ $# -ge 2 ] || usage; sha256="$2"; shift 2 ;;
        --allow-unnotarized) allow_unnotarized=1; shift ;;
        -*) usage ;;
        *) [ -z "$output" ] || usage; output="$1"; shift ;;
    esac
done
[ -n "$output" ] || usage
[ -z "$version" ] && [ -z "$sha256" ] || { [ -n "$version" ] && [ -n "$sha256" ]; } ||
    fail "--version and --sha256 go together."

if [ -z "$version" ]; then
    set -- target/release-bundle/*.dmg.sha256
    [ $# -eq 1 ] && [ -f "$1" ] ||
        fail "target/release-bundle must hold exactly one .dmg.sha256 file. Run scripts/release.sh first."
    checksum_file="$1"
    dmg="${checksum_file%.sha256}"
    [ -f "$dmg" ] || fail "$dmg does not exist."
    name="$(basename "$dmg")"
    version="$(echo "$name" | sed -n 's/^OpenCraft-\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)-arm64\.dmg$/\1/p')"
    [ -n "$version" ] || fail "the name '$name' is not OpenCraft-<version>-arm64.dmg."
    sha256="$(awk '{ print $1 }' "$checksum_file")"
    actual="$(shasum -a 256 "$dmg" | awk '{ print $1 }')"
    [ "$actual" = "$sha256" ] ||
        fail "$dmg has the SHA-256 $actual, and $checksum_file says $sha256. Run scripts/release.sh again."
    if [ "$allow_unnotarized" -eq 0 ]; then
        xcrun stapler validate "$dmg" >/dev/null 2>&1 ||
            fail "$dmg has no stapled notarization ticket. Do not publish it. Run scripts/release.sh with OPENCRAFT_NOTARY_PROFILE."
    fi
fi

echo "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' ||
    fail "the version '$version' is not three numbers."
echo "$sha256" | grep -Eq '^[0-9a-f]{64}$' ||
    fail "the SHA-256 '$sha256' is not 64 lowercase hexadecimal characters."

temporary="$output.tmp.$$"
trap 'rm -f "$temporary"' EXIT

cat > "$temporary" <<EOF
cask "opencraft" do
  version "$version"
  sha256 "$sha256"

  url "https://github.com/shantanugoel/OpenCraft/releases/download/v#{version}/OpenCraft-#{version}-arm64.dmg"
  name "OpenCraft"
  desc "Operates the VSD StreamDock N4 Pro controller"
  homepage "https://github.com/shantanugoel/OpenCraft"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on arch: :arm64
  depends_on macos: :monterey

  app "OpenCraft.app"

  uninstall quit: "com.shantanugoel.opencraft"

  zap launchctl: "opencraft.login",
      trash:     [
        "~/Library/Application Support/OpenCraft",
        "~/Library/LaunchAgents/opencraft.login.plist",
        "~/Library/Preferences/com.shantanugoel.opencraft.plist",
      ]

  caveats <<~EOS
    Quit the vendor software VSD Craft before you start OpenCraft. Two
    applications must not operate one device at the same time.

    The actions "Send a hotkey", "Type text", and "Media key" need the
    Accessibility permission of macOS. OpenCraft shows a button to give it.
  EOS
end
EOF

mv "$temporary" "$output"
trap - EXIT
echo "homebrew-cask.sh: wrote $output for version $version"
