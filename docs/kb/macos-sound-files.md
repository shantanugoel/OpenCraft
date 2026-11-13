# Sound files with AVAudioPlayer

Last update: 2026-11-13

The actions "Play a sound" and "Stop all sound" (issue
[#279](https://github.com/shantanugoel/OpenCraft/issues/279)) use
`AVAudioPlayer` of AVFoundation. The code is in
`crates/actions/src/system/macos_sound.rs`. It uses the crate `objc2`, which
OpenCraft had already. Thus the change adds no crate.

## The choice of the framework

| Fact | Confidence | Source |
|---|---|---|
| `AVAudioPlayer` plays a file from a URL. It has `volume` (0.0 to 1.0), `play`, `stop` and `isPlaying`. | Source | Apple documentation of `AVAudioPlayer` |
| On macOS, `AVAudioPlayer` has the property `currentDevice`: the UID of the output for this player. A player can thus use an output that is not the default output. | Source | Apple documentation of `AVAudioPlayer.currentDevice` |
| The UID of `currentDevice` is the same text as the Core Audio property `kAudioDevicePropertyDeviceUID`, which the action "Sound output" keeps. | Measured | The measurement below: the player with the UID of the built-in speakers played on the built-in speakers |
| `NSSound` can also select an output, with `playbackDeviceIdentifier`. OpenCraft does not use it, because `AVAudioPlayer` is the current API and has the same functions. | Source | Apple documentation of `NSSound` |

## Measurements

Date: 2026-11-13. A signed development build, with a scratch profile and the
real controller, used "Test the action". The test file was a WAV file of
4 s, a quiet sine wave, at the volume 10. A separate program read the Core
Audio property `kAudioDevicePropertyDeviceIsRunningSomewhere` of each output
one time each second.

| Fact | Confidence |
|---|---|
| The player plays on the engine thread of the handler, with no run loop. The output was "running" for about 4 s, the length of the file, and then stopped. | Measured |
| With `currentDevice` set to the UID of the built-in speakers, the built-in speakers ran. The default output was a different output. | Measured |
| `stop` on each player of the list stopped the output in less than 1 s. | Measured |
| A file that is not sound gives `nil` from `initWithContentsOfURL:error:`. The `NSError` has the code 1954115647 (`'typ?'`, `kAudioFileUnsupportedFileTypeError`). Its localized text is only "The operation couldn’t be completed", thus OpenCraft shows its own text with the code. | Measured |
| The player plays with no permission prompt. | Measured |

## Rules in the code

- The list of players is in one `Mutex`. A new sound removes the players at
  their end (`isPlaying` is false). Not more than 16 players stay. A 17th
  sound stops the oldest one.
- The handler checks that the selected output is in the output list before
  it plays. A missing output gives a message and no sound. The
  documentation does not say what `AVAudioPlayer` does with an unknown UID.

## Not known

- What `AVAudioPlayer` does with a UID of an output that is not connected.
- Whether a player continues on the new output when its output is
  disconnected during the sound.

## Sources

- Apple, AVFoundation, `AVAudioPlayer`:
  https://developer.apple.com/documentation/avfaudio/avaudioplayer
- Apple, `AVAudioPlayer.currentDevice`:
  https://developer.apple.com/documentation/avfaudio/avaudioplayer/currentdevice
- Apple, AppKit, `NSSound`: https://developer.apple.com/documentation/appkit/nssound
