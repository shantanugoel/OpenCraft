# Homebrew

Last update: 2026-11-12

The decision that uses these facts is
[decision 0027](../decisions/0027-homebrew-cask.md). The script is
`scripts/homebrew-cask.sh`.

## Test arrangement

Date: 2026-11-12. The computer and macOS are the same as in
[gpui.md](gpui.md#release-measurements): Apple silicon, macOS 27.2. Homebrew
is version 7.0.13. The tests used a local tap with no Git repository,
`shantanugoel/opencraft-localtest`, and a copy of the signed and not
notarized disk image of `scripts/release.sh`. The cask there had a `file:`
URL in place of the URL of the GitHub release. The repository of OpenCraft
is private, thus the real URL gives HTTP status 404. After the tests, the
tap, the cached download, and the application were gone.

## Which repository holds the cask

| Fact | Confidence | Source |
|---|---|---|
| The default repository `homebrew/cask` takes a cask for a GitHub project only if the project has at least 30 forks, 30 watchers, or 75 stars. | Source | [Package Acceptance Policy] |
| For a self-submission by the owner of the repository, the numbers are 90 forks, 90 watchers, or 225 stars. | Source | [Package Acceptance Policy] |
| The shared numbers can be too low a measure for a project that only hosts its binaries in the repository. A person can ask for a review. A review does not guarantee that Homebrew accepts the cask. | Source | [Acceptable Casks] |
| A cask must use a download that the developer publishes. The software must pass the Gatekeeper checks of Homebrew, and must not need Gatekeeper to be off. | Source | [Acceptable Casks] |
| Homebrew 5.0.0 deprecated the casks with no code signature, and said that it will disable all `homebrew/cask` casks that fail the Gatekeeper checks in September 2026. Homebrew 6.0.0 repeated this. | Source | [Homebrew 5.0.0]; [Homebrew 6.0.0] |
| Thus a notarized application passes the check. The release of OpenCraft is notarized (see [macos-distribution.md](macos-distribution.md#notarization)), but a new project has no stars. | Inference | The rows above |
| The cask token `opencraft` does not exist in `homebrew/cask`. The API answers HTTP status 404 for it. | Measured | `formulae.brew.sh/api/cask/opencraft.json` |

## A tap of the owner

| Fact | Confidence | Source |
|---|---|---|
| A tap on GitHub is a repository whose name starts with `homebrew-`. Casks are in the directory `Casks` at the top of the repository. | Source | [How to Create and Maintain a Tap] |
| `brew install user/repository/name` adds the tap, and then installs. The repository `shantanugoel/homebrew-opencraft` therefore gives the command `brew install --cask shantanugoel/opencraft/opencraft`. | Source | [How to Create and Maintain a Tap] |
| Homebrew 6.0.0 and later need the user to trust a tap that is not official, before Homebrew runs the code of the cask. A direct install trusts only the cask that it installs. | Source | [Homebrew 6.0.0]; [How to Create and Maintain a Tap] |
| `brew install --cask shantanugoel/opencraft-localtest/opencraft` with no earlier trust printed "Trusted cask", installed the cask, and added only that cask to `trust.json`. `brew audit` of a cask in an untrusted tap stopped with "Refusing to load cask". | Measured | Local tap; `brew install`; `brew audit` |
| The owner has a tap, `shantanugoel/homebrew-ganit`, with the file `Casks/ganit.rb` and a `README.md`. The cask has `version`, `sha256`, `url` with `v#{version}`, `name`, `desc`, `homepage`, `livecheck` with `strategy :github_latest`, `depends_on`, `app`, `uninstall quit:`, `caveats`, and `zap trash:`. The release workflow of that project updates the file. | Measured | `gh api` of the public repository |

## The stanzas

| Fact | Confidence | Source |
|---|---|---|
| `depends_on macos:` gives the lowest macOS version, and `depends_on arch: :arm64` means Apple silicon. | Source | [Cask Cookbook] |
| `auto_updates true` is for an application with a function like "Check for Updates". OpenCraft has no such function. | Source | [Cask Cookbook]; the menu in [background.md](../user/background.md#the-menu-bar-item) |
| `quit:` runs `osascript -l JavaScript` with `Application(<bundle identifier>).quit()` until the application stops, for 10 seconds at most. If it does not stop, Homebrew prints a warning and continues. The quit needs no Dock icon. | Source | `cask/artifact/abstract_uninstall.rb`, `uninstall_quit`, Homebrew 7.0.13 |
| An application that Homebrew did not quit with an Apple Event can need the Automation permission for the program that runs `brew`. Homebrew then prints instructions. | Source | `abstract_uninstall.rb`, `automation_access_instructions` |
| `quit:` for a running OpenCraft is not measured. The tests did not start OpenCraft. A quit Apple Event makes the same terminate request as the quit from the Dock menu. [Decision 0020](../decisions/0020-menu-bar-item-and-dock-icon.md#the-quit-path) sends that request to the stop of the engine. | Inference | `abstract_uninstall.rb`; decision 0020 |
| `launchctl: "<label>"` runs `launchctl remove <label>` if launchd has the job loaded, and then removes the file `~/Library/LaunchAgents/<label>.plist` if it exists. | Source | `abstract_uninstall.rb`, `uninstall_launchctl` |
| `launchctl:` in a zap stanza stopped a loaded job, and removed its plist file. The job (a `sleep`) and the file were gone after `brew uninstall --zap`. | Measured | Test cask `opencraft-zaptest` with another label |
| A `launchctl:` or `delete:` directive in the `uninstall` stanza also runs at `brew upgrade` and at `brew reinstall`. Only `signal:` does not run then. | Source | `abstract_uninstall.rb`, `UPGRADE_REINSTALL_SKIP_DIRECTIVES` |
| The `zap` stanza runs only at `brew uninstall --zap`. | Source | `abstract_uninstall.rb`, `dispatch_directives`; `brew uninstall --help` |
| `zap trash:` moves the files to the Trash. A path with `~` becomes a path in the home directory. A path that does not exist is skipped. | Measured | Test cask `opencraft-zaptest`; `brew ruby` |
| The test setup did not allow a change of `HOME` for `brew`. Thus the real `zap` stanza of OpenCraft was not run. `brew ruby` printed its paths and which of them exist. The test cask had other paths, in a scratch directory. | Measured | Test setup; `brew ruby` |

## The files that OpenCraft makes

Date: 2026-11-12. The sources are the code and the home directory of the
owner, who ran development bundles and the release bundle.

| Path in the home directory | Fact | Confidence | Source |
|---|---|---|---|
| `Library/Application Support/OpenCraft` | The directory holds all files of OpenCraft: `profiles/`, `shell.toml`, `opencraft.log`, `opencraft.log.1`, and `opencraft.lock`. No code writes a file for OpenCraft outside this directory and the next two paths. | Source | `crates/engine/src/store.rs`, `crates/app/src/log_file.rs`, `crates/app/src/lock.rs`, `crates/app/src/login.rs` |
| `Library/LaunchAgents/opencraft.login.plist` | "Start at login" writes this file. Its `Label` is `opencraft.login`. | Source | `crates/app/src/login.rs` |
| `Library/Preferences/com.shantanugoel.opencraft.plist` | The file exists after a start of the release bundle. It holds one key, `NSAutoFillHeuristicControllerEnabled`. AppKit makes it, not the code of OpenCraft. The development bundles have a file with their own identifier. | Measured | `defaults read` |
| `Library/Saved Application State/` | No item for OpenCraft exists after the starts of all bundles. | Measured | `ls` |

## The published cask

Date: 2026-11-12, after the repository became public. Homebrew 7.0.13,
macOS 27.2.

| Fact | Confidence | Source |
|---|---|---|
| `brew install --cask shantanugoel/opencraft/opencraft` got the disk image of the release `v0.1.0` and put `OpenCraft.app` in the Applications directory. It needed no step before it to trust the tap. It showed the caveats of the cask. | Measured | `brew install --cask` |
| The installed bundle passes `codesign --verify --deep --strict`. `spctl --assess --type exec` accepts it with `source=Notarized Developer ID`. The bundle has the attribute `com.apple.quarantine` with the flags `0181`. | Measured | `codesign`; `spctl`; `xattr` |
| `brew audit --cask --online --strict` found no offence for the published cask. | Measured | `brew audit` |
| Homebrew 7.0.13 does not accept the option `--signing` of `brew audit`: "Calling the `--[no-]signing` switch is disabled! There is no replacement." | Measured | `brew audit --cask --online --strict --signing` |
| The SHA-256 of the disk image that the release page gives is the SHA-256 that `scripts/release.sh` printed after the notarization. | Measured | `curl` and `shasum -a 256` |

## Not known

- `quit:` and `launchctl:` with a running OpenCraft. This needs a start of
  OpenCraft. A real device is connected, and the test must not use it.
- `brew upgrade` from one released version to the next one. The tests have
  only one version.
- `brew audit --new` is not for a tap. It checks the numbers of stars of
  the repository against the rules of `homebrew/cask`.

## Sources

[Package Acceptance Policy]: <https://docs.brew.sh/Package-Acceptance-Policy>
[Acceptable Casks]: <https://docs.brew.sh/Acceptable-Casks>
[Homebrew 5.0.0]: <https://brew.sh/2025/11/12/homebrew-5.0.0>
[Homebrew 6.0.0]: <https://brew.sh/2026/06/11/homebrew-6.0.0/>
[How to Create and Maintain a Tap]: <https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap>
[Cask Cookbook]: <https://docs.brew.sh/Cask-Cookbook>

- Homebrew documentation: [Package Acceptance Policy], [Acceptable Casks],
  [How to Create and Maintain a Tap], [Cask Cookbook].
- Homebrew release notes: [Homebrew 5.0.0], [Homebrew 6.0.0].
- The Ruby code of Homebrew 7.0.13 on the computer of the test, directory
  `Library/Homebrew/cask/artifact`.
