# Contributing

This document gives the rules for work on an issue. It also shows how the
issues are organized. The roadmap issue is
[#1](https://github.com/shantanugoel/OpenCraft/issues/1).

## How the issues are organized

- The roadmap issue is the top. Each phase has one issue with the label
  `phase`. Each phase issue lists its task issues.
- One milestone groups all issues of one phase.
- Make the task issues for a phase when that phase starts. A decision waits for
  the phase that needs it.
- The issues hold the state of each task. See
  [README.md](README.md#where-the-status-is).

| Label | Meaning |
|---|---|
| `roadmap` | The roadmap issue |
| `phase` | One phase of the roadmap |
| `task` | A unit of work that one pull request can complete |
| `measurement` | The result is a fact in `docs/kb`, not only code |
| `decision` | The result is a record in `docs/decisions` |
| `needs-hardware` | A person with the device must do a part of the work |

## Rules for work on an issue

1. **Read first.** Read [README.md](README.md), [ARCHITECTURE.md](ARCHITECTURE.md),
   and each knowledge base entry that the issue names.
2. **Say that you started.** Assign the issue to yourself, or write a comment.
   Two agents must not do the same issue.
3. **Check the blockers.** If the issue says "Blocked by", do not start until
   that issue is closed.
4. **One issue, one pull request.** Do the work on a branch. Write `Closes #N`
   in the pull request.
5. **Obey the architecture rules.** See [ARCHITECTURE.md](ARCHITECTURE.md#5-rules).
   If the work needs a change to a rule, write a decision record first and get
   approval from the owner.
6. **Record facts and keep the documents correct.** Do this in the same pull
   request as the code. The rules are in [README.md](README.md#rules) and in
   [kb/README.md](kb/README.md#rules).
7. **Do not make the scope larger.** If you find new work, make a new issue and
   add it to the phase issue.
8. **Issues with `needs-hardware`.** An agent with no device prepares the code
   and a test procedure, and writes the procedure in the issue. The owner does
   the procedure on the device and writes the result in the issue.
9. **Changes to the device layer.** A pull request that changes `crates/core`
   needs a test on the device before the merge. The owner runs
   `opencraft-probe test` and writes the result in the pull request.
10. **Write no personal data.** Do not put a device serial number, a home path,
    or a host name in the code, a document, an issue, or a commit.
11. **Changes to the window.** Read [DESIGN.md](DESIGN.md) first. Obey its
    rules, and do the checklist of its section 9 in the pull request.

## For a person outside the project

- Use an issue form to report a bug or to ask for a new device model. Do not
  report a vulnerability in an issue: see [SECURITY.md](../SECURITY.md).
- Read the rules above before you write a pull request. Rule 10 applies also
  to the log lines and to the probe output that you paste in an issue.
- The owner makes the task issues. If you want to do a task, write a comment
  in the issue first.
- The device layer needs the device for a test (rule 9). If you have no
  device, say so in the pull request.
