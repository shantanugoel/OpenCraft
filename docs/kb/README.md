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
| [ui-research.md](ui-research.md) | The layouts of other controller software, the Apple Human Interface Guidelines, and the icon set of the GUI toolkit |
| [gpui.md](gpui.md) | The GPUI toolkit and its crates |
| [text-rendering.md](text-rendering.md) | Text libraries, label samples, and image resize cost |
| [vsd-craft.md](vsd-craft.md) | The vendor software: footprint and behaviour |
| [footprint.md](footprint.md) | The footprint of OpenCraft: memory, processor use, and start time |
| [macos-permissions.md](macos-permissions.md) | macOS privacy permissions that apply to OpenCraft |
| [macos-system-functions.md](macos-system-functions.md) | Public calls for Mission Control, Launchpad, screenshot, screen saver, sleep, lock and Show desktop |
| [macos-sound-files.md](macos-sound-files.md) | `AVAudioPlayer` for the action "Play a sound": output, volume, stop, threads, and errors |
| [macos-media-keys.md](macos-media-keys.md) | The key types of the media key action, the missing Stop key, and what was observed on macOS 27 |
| [macos-front-application.md](macos-front-application.md) | The notices and calls that give the front application, and its bundle identifier |
| [macos-open-and-quit.md](macos-open-and-quit.md) | Public calls to open applications, files, folders, and URLs, and to quit an application |
| [macos-start-at-login.md](macos-start-at-login.md) | LaunchAgent files, background items, and SMAppService for a start at login |
| [dependency-licences.md](dependency-licences.md) | The licences of the crates in the release executable, their duties, and the tools for the notices |
| [macos-distribution.md](macos-distribution.md) | The release bundle: signature, hardened runtime, disk image, notarization, and the lowest macOS version |
| [homebrew.md](homebrew.md) | Homebrew: the rules for a cask, taps, the cask stanzas, and the files that OpenCraft makes |
| [hidapi-macos.md](hidapi-macos.md) | What `hidapi` on macOS does at a disconnection, and after an error |
| [profile-removal.md](profile-removal.md) | The removal of an active profile, and the window that shows an old frame (issue #321) |
| [process-lock.md](process-lock.md) | The `flock` lock of the application, child processes, and the failure of the lock test |
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
