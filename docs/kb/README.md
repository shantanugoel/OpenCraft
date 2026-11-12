# Knowledge base

The knowledge base holds the facts that the project found by research or by
measurement. The other documents link to these facts and do not repeat them.

## Rules

- Add a fact here when you find it. Do this during the research, not after.
- Give each fact a confidence label and a source.
- Give each group of facts a date.
- If a fact is not known, write it in a "Not known" list. An item leaves the
  list when a measurement gives the answer.
- If a fact becomes incorrect, correct it. Do not keep the incorrect text.

## Confidence labels

| Label | Meaning |
|---|---|
| **Measured** | We saw it on our hardware or on our computer. |
| **Source** | We read it in the named source. We did not test it. |
| **Inference** | We concluded it from other facts. It can be incorrect. |

## Index

| Entry | Subject |
|---|---|
| [n4-pro-hardware.md](n4-pro-hardware.md) | The VSD N4 Pro: identity, layout, surfaces, controls |
| [streamdock-protocol.md](streamdock-protocol.md) | The StreamDock USB protocol, version 3 |
| [prior-art.md](prior-art.md) | Other software for this device family |
| [gpui.md](gpui.md) | The GPUI toolkit and its crates |
| [text-rendering.md](text-rendering.md) | Text libraries, label samples, and image resize cost |
| [vsd-craft.md](vsd-craft.md) | The vendor software: footprint and behaviour |
| [footprint.md](footprint.md) | The footprint of OpenCraft: memory, processor use, and start time |
| [macos-permissions.md](macos-permissions.md) | macOS privacy permissions that apply to OpenCraft |
| [macos-start-at-login.md](macos-start-at-login.md) | LaunchAgent files, background items, and SMAppService for a start at login |
| [hidapi-macos.md](hidapi-macos.md) | What `hidapi` on macOS does at a disconnection, and after an error |
| [sleep-and-wake.md](sleep-and-wake.md) | What the clocks of a process do when the computer sleeps |

## Template

```markdown
# Subject

Last update: YYYY-MM-DD

## Group of facts

| Fact | Confidence | Source |
|---|---|---|

## Not known

- A question that a measurement can answer.

## Sources

- Name and URL.
```
