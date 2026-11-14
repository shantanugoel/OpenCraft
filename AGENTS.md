# Guide for agents

OpenCraft is a native Rust application. It operates StreamDock-class
controllers without the vendor software.

## Where to start

1. Read [docs/README.md](docs/README.md). It shows which document holds which fact.
2. Open the [roadmap issue](https://github.com/shantanugoel/OpenCraft/issues/1). It lists the phases and the issues.
3. Read [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) before you work on an issue.

## Commands

Run these commands from the repository root. Run all four before you commit.

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Rules that agents break most easily

Each line names a rule. The link goes to the one place that holds the rule.

- **Architecture rules.** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#5-rules)
- **Knowledge base.** [docs/kb/README.md](docs/kb/README.md#rules)
- **Design language of the window.** Read all of
  [docs/DESIGN.md](docs/DESIGN.md) before a change to the window. Use the
  tokens of `view/theme.rs` and the shared controls; write no literal colour
  or size. Its checklist is for each window pull request.
- **Simplified Technical English for documents.** [docs/README.md](docs/README.md#rules)
- **No personal data.** The rule "Write no personal data" in [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md#rules-for-work-on-an-issue)
- **Licence notices.** After a change to `Cargo.lock` or `about.toml`, run
  `scripts/notices.sh` and commit `THIRD-PARTY-NOTICES.txt`. CI fails if the
  file is not correct. [scripts/README.md](scripts/README.md#noticessh)

## Publish only when the owner tells you

A release is public and cannot be taken back. Do not run
`scripts/release.sh` with `OPENCRAFT_NOTARY_PROFILE`, do not make a tag or a
GitHub release, and do not push to the Homebrew tap, unless the owner tells
you to do that step in your task. The procedure is in
[Publish the Homebrew cask](scripts/README.md#publish-the-homebrew-cask).

## A real device can be connected

- Do not run an `opencraft-probe` command that opens the device, unless your
  task says so. Only `list` does not open the device.
- The engine opens the device. Do not run the engine example `headless` or
  the GUI application, unless your task says so. The engine tests use a fake
  transport and do not open the device.
- Two processes must not operate one device at the same time. See rule 1 in
  [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#ownership).

## Agent skills

### Issue tracker

The issues are GitHub issues in this repository. Use the `gh` CLI. See
[docs/agents/issue-tracker.md](docs/agents/issue-tracker.md).

### Triage labels

The five default triage labels: `needs-triage`, `needs-info`,
`ready-for-agent`, `ready-for-human`, `wontfix`. See
[docs/agents/triage-labels.md](docs/agents/triage-labels.md).

### Domain docs

One context. The terms are in `docs/ARCHITECTURE.md` and the decision records
are in `docs/decisions/`. See [docs/agents/domain.md](docs/agents/domain.md).
