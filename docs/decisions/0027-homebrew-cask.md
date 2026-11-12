# 0027: Homebrew installs OpenCraft with a cask in a tap of the owner

- Status: Accepted
- Date: 2026-11-12

## Context

The user guide tells the user to type
`brew install --cask shantanugoel/opencraft/opencraft`. That command needs a
tap with the name `shantanugoel/opencraft`, thus the repository
`shantanugoel/homebrew-opencraft`, with a cask `opencraft`. The release form
of [decision 0025](0025-release-form.md) gives the download file: one
notarized disk image, and a file with its SHA-256.

The facts are in [kb/homebrew.md](../kb/homebrew.md). The important ones
are:

- The default repository `homebrew/cask` takes a cask for a self-submitted
  GitHub project only with at least 225 stars, 90 forks, or 90 watchers.
- The owner has a tap for another project, `shantanugoel/homebrew-ganit`,
  with one cask file and a `README.md`.
- A direct install from a tap that is not official trusts only the cask that
  it installs.
- A `launchctl:` directive in the `uninstall` stanza also runs at
  `brew upgrade`.

## Decision

### Where the cask is

- **The repository.** A tap of the owner: `shantanugoel/homebrew-opencraft`.
  The cask token is `opencraft`. A new project has no stars, thus it does not
  meet the numbers of `homebrew/cask`. Submit the cask to `homebrew/cask`
  when the project meets them. Write a new decision then.
- **The file.** The tap holds `Casks/opencraft.rb`. Homebrew reads only this
  file. The tap holds no other copy of the cask text.
- **The text.** The script `scripts/homebrew-cask.sh` in this repository
  holds the text of the cask. It writes the file for a version and a
  SHA-256. By default it takes both from the file
  `target/release-bundle/*.dmg.sha256` of `scripts/release.sh`. It stops if
  the disk image does not have that SHA-256, or if the disk image has no
  stapled notarization ticket. Thus the cask never names a disk image that is
  not notarized.
- **The README.** `scripts/homebrew-tap/README.md` is the first `README.md`
  of the tap. After the first publish, the copy in the tap is the only one.
- **A new release.** The steps are in
  [scripts/README.md](../../scripts/README.md#publish-the-homebrew-cask). In
  short: `scripts/release.sh`, the GitHub release with the disk image, the
  script into a checkout of the tap, a commit, and a push. No automation
  between the two repositories now. The notarization runs on the computer
  of the owner, thus a workflow in GitHub cannot make the disk image.

### The cask

| Stanza | Value | Reason |
|---|---|---|
| `url` | `https://github.com/shantanugoel/OpenCraft/releases/download/v#{version}/OpenCraft-#{version}-arm64.dmg` | The disk image of the release with the tag `v<version>`. The host is the same as the host of `homepage`, thus no `verified:`. |
| `depends_on` | `arch: :arm64`, `macos: :monterey` | [Decision 0025](0025-release-form.md#the-processor-architecture) and `LSMinimumSystemVersion` 12.0 |
| `livecheck` | `url :url`, `strategy :github_latest` | The same as the cask of the owner for Ganit. `brew livecheck` finds a new version from the latest GitHub release. |
| `auto_updates` | Not set | OpenCraft does not update itself. `brew upgrade` is the only way to update. |
| `app` | `OpenCraft.app` | |
| `uninstall quit:` | `com.shantanugoel.opencraft` | Homebrew quits a running OpenCraft with a quit Apple Event before it removes the bundle. The event does not need a Dock icon. It most likely goes to the same stop of the engine as a quit from the menu bar item (inference, not measured). |
| `zap launchctl:` | `opencraft.login` | The label that [`login.rs`](../../crates/app/src/login.rs) writes. It stops the job if launchd has it loaded. |
| `zap trash:` | `~/Library/Application Support/OpenCraft`, `~/Library/LaunchAgents/opencraft.login.plist`, `~/Library/Preferences/com.shantanugoel.opencraft.plist` | The files that OpenCraft makes. See [kb/homebrew.md](../kb/homebrew.md#the-files-that-opencraft-makes). |
| `caveats` | Quit VSD Craft first. The Accessibility permission for three action kinds. | The same two facts as in [Install and first start](../user/install.md). |

- **The LaunchAgent is in `zap`, not in `uninstall`.** `brew upgrade` runs
  the `uninstall` stanza of the old version. A `launchctl:` or `delete:`
  directive there would remove the file of "Start at login" at each upgrade,
  and the option would turn off with no message. The user guide already
  tells the user to turn the option off before the removal.
- **The profiles are in `zap`.** `brew uninstall --zap` moves the folder
  with all profiles to the Trash. The user guide says so, and tells the user
  to make a copy first.
- **No `launchctl load`.** The cask does not load or start the job. launchd
  reads the file at the next login, as for a start from the menu
  ([decision 0021](0021-start-at-login.md)).
- **No `--no-quarantine`.** Homebrew has deprecated it. The installed bundle
  keeps the quarantine attribute (measured). Gatekeeper accepts the
  quarantined bundle because Apple notarized it.

## Results

- `brew install --cask shantanugoel/opencraft/opencraft` installs the
  notarized bundle in `/Applications`. `brew upgrade` puts the new bundle at
  the same path, thus "Start at login" stays on.
- The owner publishes in the order that
  [scripts/README.md](../../scripts/README.md#publish-the-homebrew-cask)
  gives. The tap is public only after this repository is public and the
  release exists.
- The tests on a local tap are in [kb/homebrew.md](../kb/homebrew.md). Not
  tested: a quit of a running OpenCraft, an upgrade between two versions, and
  the real URL. The first one needs the device, the other two need a public
  repository and two releases.
- A removal with `brew uninstall --cask` (no zap) leaves the file of "Start
  at login" and the folder with the profiles. This is intended. A leftover
  LaunchAgent file points to a missing program. The user guide gives the
  steps to avoid it.
- When `brew` runs `quit:`, macOS can ask the user to allow the program that
  runs `brew` to control OpenCraft. If the user refuses, Homebrew continues
  after 10 seconds, and OpenCraft stays open.
- Look at this decision again when the project meets the numbers of
  `homebrew/cask`.
