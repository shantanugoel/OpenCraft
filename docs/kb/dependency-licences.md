# Dependency licences and licence notices

Last update: 2026-11-12

The decision that uses these facts is
[decision 0026](../decisions/0026-licence-notices.md).

## The crates of the release executable (2026-11-12)

The crate set is the normal dependencies of `opencraft-app` for
`aarch64-apple-darwin`, with no proc-macro crate:
`cargo tree -p opencraft-app --target aarch64-apple-darwin -e normal,no-proc-macro`.
A proc-macro crate runs in the compiler, and its code is not in the
executable. `Cargo.lock` of commit `5aeb977`.

| Fact | Confidence | Source |
|---|---|---|
| The executable contains 330 crates that are not in the workspace, and the 4 crates of the workspace. With the proc-macro crates and their dependencies, the normal dependency tree has 397 crates that are not in the workspace. | Measured | `cargo tree` |
| All 853 packages of `Cargo.lock` come from crates.io. No dependency comes from a Git repository, from a path outside the workspace, or from a different registry. | Measured | `Cargo.lock`; `cargo deny check sources` |
| Each crate of the set has an SPDX expression in the field `license`. No crate has only `license-file`, and no crate has no licence. | Measured | `cargo tree -f '{p} {l}'` |
| The crate `mirajazz` is not in `Cargo.lock`. | Measured | `Cargo.lock` |
| With `CARGO_TERM_COLOR=always`, as in CI, `cargo tree` writes colour codes also into a pipe, for example around the mark `(*)`. `scripts/notices.sh` uses `--color never`. | Measured | CI run of the pull request for issue #146 |

### The licence that applies to each crate

For an expression with `OR`, the notices use the first licence of this
order: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, Zlib, 0BSD, Unicode-3.0,
CC0-1.0, MPL-2.0, ISC. The order is `accepted` in `about.toml`.

| Licence that applies | Crates | Confidence |
|---|---|---|
| MIT | 279 | Measured |
| Apache-2.0 | 20: the GPUI crates (`gpui-pre*`, `gpui-base`, `gpui-component`, `gpui-kit`), `moxcms`, `pxfm`, `ryu`, `similar`, `unicode-linebreak` | Measured |
| Unicode-3.0 | 19: the `icu_*` crates, `litemap`, `potential_utf`, `tinystr`, `writeable`, `yoke`, `zerofrom`, `zerotrie`, `zerovec` | Measured |
| BSD-3-Clause | 5: `exr`, `instant`, `lebe`, `tiny-skia`, `tiny-skia-path` | Measured |
| Zlib | 2: `foldhash`, `slotmap` | Measured |
| BSD-2-Clause | 1: `arrayref` | Measured |
| 0BSD | 1: `enum-iterator` | Measured |
| CC0-1.0 | 1: `notify` | Measured |
| MPL-2.0 | 1: `option-ext` 0.2.0. The path is `dirs-sys`, `dirs`, `zed-font-kit`, `gpui-pre-apple`. | Measured |
| Apache-2.0 AND ISC AND MIT | 1: `gpui-kit-assets`. The crate declares Apache-2.0. The notices add the ISC and MIT texts of its Lucide icons. See "Works that are not crates". | Measured |

The tree has no GPL, LGPL, AGPL, OpenSSL, SSPL, or BUSL licence. `ryu`
declares "Apache-2.0 OR BSL-1.0", and the notices use Apache-2.0. (Measured)

### Duties of the licences

| Licence | Duty for a distribution of the executable | Confidence | Source |
|---|---|---|---|
| MIT, ISC, BSD-2-Clause, Unicode-3.0 | Include the copyright notice and the permission notice | Source | The licence texts |
| BSD-3-Clause | Include the copyright notice, the conditions, and the disclaimer in the documentation or other materials. Do not use the names of the authors to promote the product. | Source | The licence text |
| Apache-2.0 | Give a copy of the licence. If the work has a `NOTICE` file, include its notices. No crate of the set has a `NOTICE` file. | Source; Measured (no `NOTICE` file) | The licence text, section 4; the crate directories |
| MPL-2.0 | Make the source code of the covered files available, and tell the recipients how to get it (section 3.2). The duty is for each file. OpenCraft does not change the files of `option-ext`, thus the crates.io source of the same version is the source. | Source; Inference (the crates.io link is sufficient) | The licence text |
| Zlib, 0BSD, CC0-1.0 | No notice is necessary for a binary. The notices include the texts. | Source | The licence texts |

## The licence files of the crates (2026-11-12)

The tool is `cargo-about` 0.9.3 with `--offline`.

| Fact | Confidence | Source |
|---|---|---|
| `cargo-about` runs `cargo metadata`, which needs the crates of all platforms. With `--offline` and no `cargo fetch` before, it stops: "failed to download `accesskit_windows`". After `cargo fetch --locked`, it runs with no network. | Measured | cargo-about output |
| The run time is approximately 10 to 20 seconds on the computer of the owner. | Measured | `time` |
| 42 crates of the set have no file for the licence that applies. For them, `cargo-about` uses the standard text of the licence, which has no copyright line. They are the `objc2` family (21 crates), `accesskit` (3), `lyon` (5), `pathfinder_geometry`, `pathfinder_simd`, `profiling`, `rust-i18n-support`, `taffy`, `mac`, `seahash`, `svg_fmt`, `zune-inflate`, `gpui-kit`, and `gpui-kit-assets` (Apache-2.0 only). The notices give the authors of the crate manifest for each of them. | Measured | cargo-about JSON output, `source_path` is empty |
| A run with network gave the same 42 crates with the standard text. | Measured | cargo-about JSON output, with and without `--offline` |
| `cargo-about` does not identify the licence files of `chrono`, `exr`, `granit-parser`, `libm`, `ryu`, and `gpui-kit-assets` (`LICENSE-LUCIDE`). Each file contains more than one licence, or has text before the licence. `about.toml` clarifies them with the file of the crate and its SHA-256. `chrono` uses the built-in workaround of `cargo-about`. | Measured | cargo-about output; `about.toml` |
| The file `LICENSE` of `granit-parser` says: "Redistributions of this Work must include all inherited and new license notices". The notices include `LICENSE`, `.licenses/ChenYuheng-MIT`, and `.licenses/Ethiraric-MIT`. | Source | `granit-parser` 1.3.0 |
| The `clarify` subcommand of `cargo-about` 0.9.3 reads the end of a subsection one character too early (`pos + 1` after `!!`). The checksums of the subsections in `about.toml` came from a short script that does the same cut as `generate`. | Source | `cargo-about` 0.9.3 `src/cargo-about/clarify.rs` |
| The notices file has 330 crates, 6613 lines, and approximately 320 KB. Three licence texts contain an e-mail address in their copyright line. | Measured | `THIRD-PARTY-NOTICES.txt` |

## The licence check with cargo-deny (2026-11-12)

| Fact | Confidence | Source |
|---|---|---|
| With `targets = ["aarch64-apple-darwin"]`, `cargo deny check licenses sources` passes for the full workspace, with the build and dev dependencies, with the allow list of `deny.toml`. | Measured | cargo-deny 0.20.2 |
| With no MPL-2.0 in the allow list, the check fails: `error[rejected]` for `option-ext`. | Measured | cargo-deny 0.20.2 |
| ISC is not a declared licence of a crate. cargo-deny gives the warning `license-not-encountered` for an allowed licence that no crate uses. | Measured | cargo-deny 0.20.2 |
| `cargo deny check advisories` fails with 4 `unmaintained` errors: `instant`, `paste`, `rustybuzz`, and `ttf-parser`. All come from GPUI. It gives no vulnerability. | Measured | cargo-deny 0.20.2, advisory database of 2026-11-12 |

## Works that are not crates (2026-11-12)

| Fact | Confidence | Source |
|---|---|---|
| The crates of the workspace embed no file in the executable. The only `include_str!` calls are in tests, and they read source files of the project. | Measured | `grep` of `crates/` |
| The bundle contains `OpenCraft.icns`. `scripts/make-icon.swift` of the project draws it. | Source | [decision 0025](../decisions/0025-release-form.md) |
| GPUI embeds the fonts IBM Plex Sans and Lilex only in `#[cfg(test)]` code. The executable contains no font. | Source | `gpui-pre` 0.3.12 `src/svg_renderer.rs`; `gpui-pre-apple` 0.3.12 `src/text_system.rs` |
| The Lucide icon files of `gpui-kit-assets` are not in the executable. The executable contains two SVG texts, from `gpui-base`: a chevron down and a chevron right. Their paths are the same as the Lucide icons `chevron-down` and `chevron-right`, which Lucide lists as icons from Feather. | Measured (`strings` of the release executable); Source (`gpui-base` 0.9.1, `LICENSE-LUCIDE`) | Release executable; crate sources |
| The Mirabox StreamDock Device SDK (MIT) and `mirajazz` (MPL-2.0) are not dependencies. The project read them and took facts: operation codes, byte values, intervals, and JPEG quality values. No source text of them is in OpenCraft. | Source | [decision 0002](../decisions/0002-hidapi-transport.md); comments in `crates/core/src/protocol.rs`; [streamdock-protocol.md](streamdock-protocol.md#sources) |
| The MIT licence and the MPL-2.0 apply to copies of the software and of the covered source. Facts of a protocol are not a copy. Thus they ask for no notice in OpenCraft. The README of the repository gives credit to both. | Inference | The licence texts |

## Not known

- Whether the output of `cargo-about` on Linux (CI) is the same as on macOS.
  The CI run of the pull request for issue #146 answers this.
- The copyright lines of the 42 crates with no licence file in the package.
  Their Git repositories possibly have them. A `clarify` entry with `git`
  can fetch them, but it needs the network at each run.

## Sources

- cargo-about 0.9.3: <https://github.com/EmbarkStudios/cargo-about>, the
  book in `docs/src` of the crate.
- cargo-deny 0.20.2: <https://github.com/EmbarkStudios/cargo-deny>.
- The licence texts: <https://spdx.org/licenses/>. MPL-2.0:
  <https://www.mozilla.org/en-US/MPL/2.0/>.
