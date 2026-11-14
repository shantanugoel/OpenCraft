# Domain documents

This file tells the engineering skills where the domain documents of
OpenCraft are. OpenCraft has one context. It has no `GLOSSARY.md` and no
`docs/adr/`. Do not make them.

## Read these before you examine the code

- **Terms.** [ARCHITECTURE.md, section 3](../ARCHITECTURE.md#3-terms). This
  section is the glossary.
- **Rules.** [ARCHITECTURE.md, section 5](../ARCHITECTURE.md#5-rules).
- **Decision records.** [docs/decisions/](../decisions/README.md). These are
  the ADRs. Read each record that is related to the area of your work. The
  index gives the status of each record. A record with the status
  "Replaced by NNNN" is not current.
- **Facts.** [docs/kb/](../kb/README.md). Facts from research and from
  measurement of the device.

[docs/README.md](../README.md) gives the job of each document.

## Where to write

- **A new term.** Add it to section 3 of `ARCHITECTURE.md`. Do not make a
  `GLOSSARY.md`.
- **A new decision.** Write a record in `docs/decisions/` and add it to the
  index. Obey the rules in
  [decisions/README.md](../decisions/README.md#rules). Write a record only
  when a phase forces the decision.
- **A new fact.** Write it in `docs/kb/` with its confidence label. Obey the
  rules in [kb/README.md](../kb/README.md#rules).
- Write all documents in ASD-STE100 Simplified Technical English.

## Use the terms of the architecture

When your output names a domain concept (an issue title, a refactor proposal,
a hypothesis, a test name), use the term from section 3 of
`ARCHITECTURE.md`. Do not use a synonym.

If the concept is not in that section, examine the cause. Possibly you use
words that the project does not use. Or possibly there is a real gap. In that
case, record the gap for `/domain-modeling`.

## Tell about a conflict with a decision record

If your output does not agree with an accepted decision record, say so
clearly. Do not ignore the record. For example:

> _This does not agree with 0011 (engine thread model). It is possibly
> necessary to change it, because ..._

To change an accepted decision, write a new record that replaces it.
