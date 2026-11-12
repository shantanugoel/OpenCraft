#!/bin/sh
# Make THIRD-PARTY-NOTICES.txt: the licence notices of the crates that the
# release executable of OpenCraft contains. The decision is
# docs/decisions/0026-licence-notices.md. The facts are in
# docs/kb/dependency-licences.md.
#
# Usage: scripts/notices.sh [--check | --key]
#
#   (no option)  Write THIRD-PARTY-NOTICES.txt in the root of the repository.
#   --check      Make the notices again and compare them with the file. Stop
#                with exit status 1 if the file is missing or not the same.
#                CI uses this option.
#   --key        Print the key of the crate set, and do nothing more. The
#                key is the SHA-256 of the crate list and of about.toml. The
#                file holds the key. scripts/release.sh compares the two
#                keys. This option needs only cargo, and no network after the
#                build.
#
# The crate set is the normal dependencies of opencraft-app for
# aarch64-apple-darwin, with no proc-macro crate, no build dependency, no dev
# dependency, and no crate of the workspace. A proc-macro crate runs in the
# compiler. Its code is not in the executable.
#
# The other options need cargo-about (`cargo install cargo-about --locked
# --features cli`) and jq. cargo-about runs with --offline, thus it reads
# only the files of the crates in the cargo registry. Run `cargo fetch` one
# time before, because cargo-about reads the metadata of all platforms.

set -eu

NOTICES="THIRD-PARTY-NOTICES.txt"
TARGET="aarch64-apple-darwin"
PACKAGE="opencraft-app"

mode="write"
case "${1:-}" in
    "") ;;
    --check) mode="check" ;;
    --key) mode="key" ;;
    *) echo "usage: $0 [--check | --key]" >&2; exit 2 ;;
esac

cd "$(dirname "$0")/.."

fail() {
    echo "notices.sh: $*" >&2
    exit 1
}

# One line "<name> v<version>" for each crate of the set, in sort order.
# A crate of the workspace has a path in parentheses after its version. No
# colour: CI sets CARGO_TERM_COLOR=always.
crate_list() {
    cargo tree --locked --color never -p "$PACKAGE" --target "$TARGET" \
        -e normal,no-proc-macro --prefix none -f '{p}' |
        sed 's/ (\*)$//' | grep -v ' (' | LC_ALL=C sort -u
}

sha256() {
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 | cut -d ' ' -f 1
    else
        sha256sum | cut -d ' ' -f 1
    fi
}

key() {
    { crate_list; cat about.toml; } | sha256
}

if [ "$mode" = "key" ]; then
    key
    exit 0
fi

command -v jq >/dev/null 2>&1 || fail "jq is not installed."
cargo about --version >/dev/null 2>&1 ||
    fail "cargo-about is not installed. Run: cargo install cargo-about --locked --features cli"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

crate_list > "$work/crates.txt"
[ -s "$work/crates.txt" ] || fail "the crate list is empty."
key_value="$(key)"

cargo about generate --format json --offline --locked \
    -c about.toml -m "crates/app/Cargo.toml" -o "$work/about.json"

# The jq program makes the text. It stops if a crate of the set has no
# licence text, or if cargo-about has no data for it. It keeps no path of the
# computer, and no e-mail address from the `authors` field.
jq -r --rawfile crates "$work/crates.txt" --arg key "$key_value" '
  def id: .name + " " + .version;
  ($crates | split("\n") | map(select(. != "") | sub(" v"; " "))) as $set
  | ($set | map({(.): true}) | add) as $in
  | ([.licenses[]
      | {name, id, text, fallback: (.source_path == null),
         crates: ([.used_by[].crate | select($in[id]) | id] | unique)}
      | select(.crates | length > 0)]
     | sort_by(.id, .text)) as $texts
  | (reduce .crates[] as $c ({}; if $in[$c.package | id]
      then .[$c.package | id] = $c.package else . end)) as $packages
  | ($set - ($packages | keys)) as $no_data
  | ($set - ([$texts[].crates[]] | unique)) as $no_text
  | if ($no_data | length) > 0 then
      error("cargo-about has no data for: " + ($no_data | join(", ")))
    elif ($no_text | length) > 0 then
      error("no licence text for: " + ($no_text | join(", ")))
    else . end
  | def used($c): [$texts[] | select(.crates | index($c)) | .id] | unique;
    def authors($p): [($p.authors // [])[] | sub("\\s*<[^>]*>"; "")]
      | map(select(. != "")) | join(", ");
  "Third-party notices for OpenCraft",
  "",
  "OpenCraft has the MIT licence. The file LICENSE in the source code has the",
  "text. The OpenCraft executable for macOS on Apple silicon contains the code",
  "of the crates below. Each crate has its own licence. This file gives each",
  "crate, the licence that applies to the copy in OpenCraft, and the licence",
  "texts with the copyright lines that the crates give.",
  "",
  "The source code of each crate is on crates.io:",
  "https://crates.io/crates/<name>/<version>.",
  "",
  ( [$set[] | select(used(.) | index("MPL-2.0"))] as $mpl
    | if ($mpl | length) > 0 then
        "These crates have the Mozilla Public License 2.0 (MPL-2.0). OpenCraft",
        "did not change their files. Their source code is available here:",
        ($mpl[] | split(" ") | "  https://crates.io/crates/\(.[0])/\(.[1])"),
        ""
      else empty end ),
  "Do not edit this file. scripts/notices.sh makes it from Cargo.lock and",
  "about.toml.",
  "Key: \($key)",
  "",
  "",
  "== Crates (\($set | length)) ==",
  "",
  "Each line gives the crate, its version, the licence of the crate manifest,",
  "and the licence that applies here.",
  "",
  ( $set[] as $c | $packages[$c] as $p
    | "\($c)  [\($p.license // "none")]  -> \(used($c) | join(" AND "))"
    , ( [$texts[] | select(.fallback and (.crates | index($c))) | .id] as $standard
        | if ($standard | length) > 0
          then "    The crate has no file for \($standard | join(", ")). The text below is the"
             , "    standard text. "
               + (authors($p) | if . == "" then "The crate manifest gives no authors."
                  else "The authors in the crate manifest: \(.)." end)
          else empty end ) ),
  "",
  "",
  "== Licence texts (\($texts | length)) ==",
  ( $texts[]
    | "",
      "----------------------------------------------------------------------",
      "\(.name) (\(.id))",
      "Used by: \(.crates | join(", "))",
      "----------------------------------------------------------------------",
      "",
      (.text | sub("\\s+$"; "")) )
' "$work/about.json" > "$work/notices.txt"

if [ "$mode" = "check" ]; then
    [ -f "$NOTICES" ] || fail "$NOTICES does not exist. Run scripts/notices.sh."
    if ! diff -u "$NOTICES" "$work/notices.txt" > "$work/diff.txt"; then
        head -n 60 "$work/diff.txt" >&2
        fail "$NOTICES is not correct for Cargo.lock and about.toml. Run scripts/notices.sh and commit the file."
    fi
    echo "$NOTICES is correct."
else
    cp "$work/notices.txt" "$NOTICES"
    echo "Wrote $NOTICES ($(wc -l < "$NOTICES" | tr -d ' ') lines, $(wc -l < "$work/crates.txt" | tr -d ' ') crates)."
fi
