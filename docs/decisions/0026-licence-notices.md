# 0026: cargo-about makes a committed notices file, CI checks it, and the menu bar item opens it

- Status: Accepted
- Date: 2026-11-12

## Context

The first public release (Phase 6) distributes an executable that contains
330 crates of other authors. Their licences ask for the licence texts and the
copyright notices in each copy. One crate has the MPL-2.0, which also asks
that the recipients can get its source code.
[Decision 0025](0025-release-form.md) put a place for the file
`THIRD-PARTY-NOTICES.txt` in the bundle, but the file did not exist.

The facts are in
[kb/dependency-licences.md](../kb/dependency-licences.md). The important
ones are:

- All crates come from crates.io, and each crate declares an SPDX licence.
  The licences are MIT, Apache-2.0, Unicode-3.0, BSD-3-Clause,
  BSD-2-Clause, Zlib, 0BSD, CC0-1.0, and MPL-2.0 (`option-ext` only).
- `cargo-about` makes the notices with `--offline` in 10 to 20 seconds,
  after `cargo fetch`. A run with network gave no more copyright lines.
- 42 crates have no licence file in their package.
- The Mirabox SDK and `mirajazz` gave only facts. No code of them is in
  OpenCraft. The executable contains no font and no icon file of another
  project.

## Decision

### The tool and the crate set

- `cargo-about` collects the licence texts. `about.toml` holds its
  configuration: the accepted licences in order of priority, the target
  `aarch64-apple-darwin`, and the clarifications for 6 crates whose files it
  does not identify. Each clarification uses a file of the crate and its
  SHA-256. Thus the notices need no network.
- `scripts/notices.sh` makes the file. It keeps only the crates that the
  executable contains: the normal dependencies of `opencraft-app` for
  `aarch64-apple-darwin`, with no proc-macro crate and no crate of the
  workspace (`cargo tree -e normal,no-proc-macro`). It stops if a crate of
  the set has no licence text.
- For a crate with no licence file, the file gives the standard text and the
  authors of the crate manifest, with no e-mail address.
- For each MPL-2.0 crate, the file gives the crates.io address of its
  source code.
- The Lucide licence (ISC, and MIT for the icons from Feather) of
  `gpui-kit-assets` is in the file. The executable contains two chevron
  paths that are the same as two Lucide icons. The cost is small.
- The Mirabox SDK and `mirajazz` get no notice, because no code of them is
  in OpenCraft. The README gives credit to both.

### The file is in the repository, and two checks keep it correct

- `THIRD-PARTY-NOTICES.txt` is in the root of the repository. A person can
  read it in a pull request when a dependency changes.
- The file holds a key: the SHA-256 of the crate list and of `about.toml`.
  `scripts/notices.sh --key` calculates it with `cargo tree` only.
- `scripts/release.sh` stops if the file is missing, or if its key is not
  the key of the current `Cargo.lock`. This check needs no `cargo-about` and
  no network. The script also stops if the bundle has no notices file.
  Thus a release with no notices, or with old notices, is not possible.
- CI makes the file again with `scripts/notices.sh --check` and stops if it
  is not the same as the committed file. This finds all other differences,
  for example a change of `scripts/notices.sh`.
- Thus a change to `Cargo.lock` that changes the crate set needs a new
  notices file in the same pull request: run `scripts/notices.sh`.

### The licence check

- `deny.toml` holds the accepted licences for `cargo-deny`, with the target
  `aarch64-apple-darwin`. The list is the licences of the tree: MIT,
  Apache-2.0, BSD-2-Clause, BSD-3-Clause, Zlib, 0BSD, Unicode-3.0, CC0-1.0,
  and MPL-2.0. The check also stops for a crate from a registry other than
  crates.io or from a Git repository.
- A new licence needs a change of `deny.toml` and of `about.toml`, and its
  duties in the knowledge base.
- The advisories check is not on. It fails now for 4 `unmaintained` crates
  that GPUI uses, and OpenCraft cannot change them. Look at this again when
  GPUI changes these crates.
- CI has a separate job "Licences and notices". It uses
  `EmbarkStudios/cargo-deny-action@v2` and the prebuilt `cargo-about` of
  `taiki-e/install-action@v2`. Thus it compiles no tool.

### The user opens the notices from the menu bar item

- The menu of the menu bar item ([decision 0020](0020-menu-bar-item-and-dock-icon.md))
  has the item "Licence notices" after the item "OpenCraft *version*" of
  decision 0025, before the separator.
- The item opens `Contents/Resources/THIRD-PARTY-NOTICES.txt` of the bundle
  with the default application for text, usually TextEdit, through
  `NSWorkspace.openURL`. This adds the features `NSWorkspace` of
  `objc2-app-kit` and `NSURL` of `objc2-foundation`. No new crate comes
  into `Cargo.lock`.
- The shell finds the file at the start, from the path of its executable.
  If the executable is not in a bundle with the file, for example after
  `cargo run` or in the development bundle of `scripts/sign-dev.sh`, the
  menu does not show the item.
- If macOS cannot open the file, the window shows a problem.

## Results

- Each release bundle contains the notices, and the user can open them.
- A dependency update can need two more steps: a new notices file, and
  possibly a new clarification in `about.toml` when a crate changes its
  licence file.
- The notices give the standard text with no copyright line for 42 crates.
  Look at this again if the owner wants the copyright lines of their Git
  repositories: a `clarify` entry with `git` can fetch them, but it needs
  the network for each run.
- The window has no "About" panel with the notices. Look at this again if
  the window gets an "About" panel.
