# OpenCraft documentation

Each document has one job. Each fact is in one place only.

| Document | Job | Change it when |
|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | The stable design: terms, layers, rules, deferred decisions | A decision record changes a rule |
| [ROADMAP.md](ROADMAP.md) | Phases, exit criteria, product targets | A phase starts or ends |
| [CONTRIBUTING.md](CONTRIBUTING.md) | The rules for work on an issue, and how the issues are organized | The process changes |
| [ADDING-A-DEVICE-MODEL.md](ADDING-A-DEVICE-MODEL.md) | The procedure to add a new device model as data | The descriptor, the probe, or a limit of the procedure changes |
| [decisions/](decisions/README.md) | One record for each design decision | You make a decision |
| [kb/](kb/README.md) | Knowledge base: facts from research and from measurement | You learn a fact |

## Where the status is

- The [roadmap](ROADMAP.md#status) holds the state of each phase.
- The issues hold the state of each task. The
  [roadmap issue](https://github.com/shantanugoel/OpenCraft/issues/1) links to
  all of them.
- Do not write the state of a task in a document.

## Rules

- Write all documents in ASD-STE100 Simplified Technical English.
- Do not write a fact in two documents. Write it one time and link to it.
- If the code and a document disagree, correct one of them in the same change.
- Put a research result in `kb/` first. Then refer to it from the other documents.

## History

The first plan was `docs/PLAN.md`. It was a feasibility study, a hardware
reference, an architecture, and a schedule in one file. Its content is now in
the documents above, with the incorrect facts corrected. The Git history keeps
the original file.
