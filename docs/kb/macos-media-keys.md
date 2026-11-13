# Media keys

Last update: 2026-11-14

The implementation is `media_key` in `crates/actions/src/system/macos.rs`.
It posts a system-defined event with the subtype "auxiliary control
buttons" at the HID event tap. The data of the event holds a key type and a
state: `0xA` for key down and `0xB` for key up.

## Key types

| Key | Key type | Confidence and source |
|---|---|---|
| Sound up, Sound down, Mute | 0, 1, 7 | Source: `NX_KEYTYPE_SOUND_UP`, `NX_KEYTYPE_SOUND_DOWN`, `NX_KEYTYPE_MUTE` in `IOKit/hidsystem/ev_keymap.h` of the SDK |
| Play or pause, Next, Previous | 16, 17, 18 | Source: `NX_KEYTYPE_PLAY`, `NX_KEYTYPE_NEXT`, `NX_KEYTYPE_PREVIOUS` in the same header |
| Fast forward, Rewind | 19, 20 | Source: `NX_KEYTYPE_FAST`, `NX_KEYTYPE_REWIND` in the same header |
| Stop | None | Source: the header has no key type for Stop. The open-source `IOHIDConsumer` of the kernel maps the HID usage "Stop or Eject" to `NX_KEYTYPE_EJECT`. |

## What was observed

Date of the check: 2026-11-13. The computer has macOS 27.2. A program
started from the shell called `media_key` through the example program
`system`. It stayed alive 300 ms after the event. A second program in the
shell watched the session with a listen-only event tap.

| Fact | Confidence | Source |
|---|---|---|
| One call of Fast forward made two events: key type 19 with state `0xA`, then key type 19 with state `0xB`. | Measured | Event tap |
| One call of Rewind made two events: key type 20 with state `0xA`, then key type 20 with state `0xB`. | Measured | Event tap |
| An older call with Mute changed the mute of the sound output. The same program with the new keys uses the same code path. | Measured | Volume read before and after |
| A global `NSEvent` monitor did not report the events that the program posted. The event tap did report them. | Measured | Two observer programs |
| The check did not start a media player. It does not show what a player does with Fast forward and Rewind. | Measured | Not applicable |
| The result of the key depends on the application that holds the "now playing" role. Some players skip a track at a short press. | Inference | Behaviour of the system media keys on a keyboard |

## Owner check (2026-11-14)

The owner used the check profile of that date with the signed development
release bundle. A media player had a track that played. The procedure does
not name the player.

| Fact | Confidence | Source |
|---|---|---|
| One press of Fast forward moved the track forward or skipped to the next track. | Measured | Owner check, [issue #255](https://github.com/shantanugoel/OpenCraft/issues/255) |
| One press of Rewind moved the track back or skipped to the previous track. | Measured | Owner check, issue #255 |

## Not known

- What Music, Spotify, and a browser video do with one short press, when
  the player is one of those three. The owner check used one media player
  and did not name it.
- Whether any public call can stop a player. Stop exists in the private
  MediaRemote framework. OpenCraft does not use a private framework.
- The result on macOS 12 to 26.

## Sources

- macOS SDK: `IOKit/hidsystem/ev_keymap.h`.
- Apple open source, `IOHIDConsumer.cpp` of the IOHIDFamily project.
