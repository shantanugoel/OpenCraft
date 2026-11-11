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
- **Simplified Technical English for documents.** [docs/README.md](docs/README.md#rules)
- **No personal data.** The rule "Write no personal data" in [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md#rules-for-work-on-an-issue)

## A real device can be connected

- Do not run an `opencraft-probe` command that opens the device, unless your
  task says so. Only `list` does not open the device.
- Two processes must not operate one device at the same time. See rule 1 in
  [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#ownership).
