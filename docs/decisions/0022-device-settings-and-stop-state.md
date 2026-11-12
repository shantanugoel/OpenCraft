# 0022: Device settings are for each device model, outside the profile, and the device goes dark at the stop

- Status: Accepted
- Date: 2026-11-12

## Context

Phase 5 adds device settings (brightness, haptic feedback, knob lights) and a
defined device state when OpenCraft stops. See the
[roadmap](../ROADMAP.md#phase-5-background-operation). Tasks
[#124](https://github.com/shantanugoel/OpenCraft/issues/124) and
[#125](https://github.com/shantanugoel/OpenCraft/issues/125) implement this
decision.

Until now the engine sends the brightness `DEFAULT_BRIGHTNESS` (80) in the
start sequence. It sends no knob light command and no haptic command. At the
stop, it writes the profile and closes the device, and it sends no command.
Thus the images stay for 20 to 35 seconds, and the controls do nothing. Then
the device stops the USB connection, starts again, and shows its logo.

The facts are in the knowledge base:

- Brightness: [streamdock-protocol.md](../kb/streamdock-protocol.md#brightness)
- The knob lights: [n4-pro-hardware.md](../kb/n4-pro-hardware.md#knob-lights)
- The haptic feedback and the other device configuration:
  [streamdock-protocol.md](../kb/streamdock-protocol.md#device-configuration)
- The keep-alive timer and the measured stop states:
  [streamdock-protocol.md](../kb/streamdock-protocol.md#keep-alive-and-stop)
- How other software keeps the settings:
  [vsd-craft.md](../kb/vsd-craft.md#device-settings-2026-11-12) and
  [prior-art.md](../kb/prior-art.md#opendeck-akp05)

## Decision

### The settings

| Setting | Values | Default | Commands |
|---|---|---|---|
| Display brightness | 15 to 100 percent | 80 | `LIG` |
| Haptic feedback | On, off | On | `QUCMD` with the bytes of `mirajazz` (`protocol::set_vibration`) |
| Knob light mode | Effect, colours, off | Effect | Effect: `SETLB` with all colours black. Colours: `SETLB` with one colour for each knob. Off: `LBLIG 0`. |
| Knob light brightness | 1 to 100 percent | 100 | `LBLIG`. The mode "off" sends 0. |
| Knob light colours | One colour (`#RRGGBB`) for each knob | White | `SETLB`, in the order of the knobs, knob 1 first |

- The lowest brightness is 15, because the values 0 to 15 give the same
  light (measured). The device has no "off" for the displays other than
  `HAN`. A display-off setting is not in Phase 5.
- In the mode "colours", a black knob is off if a different knob has a
  colour (measured). If all colours are black, the device shows the effect
  (measured). Thus the engine then sends the commands of the mode "off".
- The defaults give the device of Phase 4. The brightness 80 is the value of
  Phase 4. Haptic feedback on, and the effect at full brightness, are the
  state of the device with no command (measured). A user who does not change
  a setting sees no change.
- The knob light brightness is kept when the mode changes, so "off" and back
  gives the last brightness.

### The descriptor

The descriptor gives the settings that a model has (rule 4). The N4 Pro
descriptors get:

- the lowest brightness that changes the light: 15
- the number of knob lights: 4
- haptic feedback: yes

The values for `0x1008` and `0x1021` are the same, and they are not tested.
The engine sends no knob light command to a model with no knob lights, and no
haptic command to a model with no haptic feedback. The shell shows only the
settings that the model has.

### Where the settings are kept

The file `device-settings.toml` in the profiles directory, next to
`state.toml`, holds the settings. It has one table for each model
identifier.

```toml
schema_version = 1

[n4-pro]
brightness = 80
haptic = true

[n4-pro.knob_lights]
mode = "effect"   # "effect", "colors", or "off"
brightness = 100
colors = ["#FFFFFF", "#FFFFFF", "#FFFFFF", "#FFFFFF"]
```

- A missing file, table, or value gives the default. The engine puts a value
  out of range into the range.
- If the file has an error or a newer schema version, the engine uses the
  defaults and tells the shell (`Event::Problem`). It does not write the file
  until the user changes a setting.
- The engine keeps the tables of other models when it writes the file.
- The engine writes the file after the same delay as a profile
  (`SAVE_DELAY`), and at the stop. Note (2026-11-12): the engine also writes
  the file before a change of the model of the device, as it does for the
  profile (`change_model` in `crates/engine/src/engine.rs`).

### The layers

- **Device layer (`opencraft-core`).** It builds the commands. The builders
  `set_brightness`, `set_led_colors`, `set_led_brightness`, `set_vibration`,
  and `sleep` exist. Add a builder for `DC` (`CLE`, `0x00 0x00`, `"DC"`), and
  a function that sends the stop sequence. The descriptor holds the values
  in "The descriptor".
- **Engine.** It reads and writes the settings file, holds the settings of
  the model of the device, and sends the commands. It reads the settings at
  the start and after a change of the model, as it does for the profiles.
- **Shell.** It shows the settings and changes them only through engine
  commands: one command with the full settings, and one event with the full
  settings at the start and after each change. The layout tells the shell
  which settings the model has. The shell has no device logic (rule 12).
  Note (2026-11-12): while the user moves a slider, the window sends one
  command each 50 ms at most, and then the last value (`SETTINGS_INTERVAL`
  in `crates/app/src/view/device.rs`).

The engine operates with no window (rule 3), so the engine, not the shell,
keeps and applies the settings.

### When the engine sends the settings

- **After each start sequence**: at each connection, at each reconnection,
  and at each full draw (rule 10). The order is: `DIS`, `LIG` (the setting),
  `CLE 0xFF`, `STP`, then `SETLB`, then `LBLIG`, then `QUCMD`, then the
  images, then `STP`. The device does not keep the knob light colours after
  `DC`, and it keeps no setting after a USB restart (measured). Thus the
  engine sends all settings each time. It sends `QUCMD` each time also,
  because a different program can change the device configuration, and the
  device keeps it after a USB restart (measured).
- **At a change**, only the commands of the setting that changed: `LIG`, or
  `SETLB` and `LBLIG`, or `QUCMD`.
- **With no device connected**, the engine keeps the change and sends it at
  the next connection.

### The device state at a normal stop

The displays are dark, and the knob lights are off. The device stays on the
USB bus, and it does not start again. The state stays until a program sends
a start sequence, or until the cable is disconnected. (Measured: 120 seconds
with no change.)

The stop sequence is:

1. Write the profile and the settings file, if they changed.
2. If a device is connected: send `DC` (`CLE`, `0x00 0x00`, `"DC"`), and then
   `HAN`. Send no pause between them, and no `CONNECT` after them.
3. Close the device.

- `DC` stops the keep-alive timer. Thus the device does not stop the USB
  connection, and it does not show its logo again (measured).
- `HAN` after `DC` makes the displays dark and switches off the knob lights
  (measured).
- The device got the two commands when the process exited directly after
  the writes (measured: 3 times). No wait is necessary after the writes.
- The engine sends the sequence when it gets `Command::Stop`, and when each
  sender of commands is gone. The shell sends `Command::Stop` and waits for
  the engine at each quit: from the menu, from Cmd-Q, and at the signals
  `SIGTERM`, `SIGINT`, and `SIGHUP`. Note (2026-11-12): the wait has a
  time limit. See the note in
  [decision 0020](0020-menu-bar-item-and-dock-icon.md#results).
- The next start of OpenCraft needs only the usual start sequence. Its
  brightness command wakes the displays, and its `DIS` starts a new timer
  (measured).
- Do not send a start sequence directly after `DC`. With no pause, the device
  did not show the start (measured: 2 times). A pause of 0.1 seconds was
  enough. A new process starts later than that. (Inference)

### A kill or a crash

OpenCraft cannot send a command when it is killed with `SIGKILL` or when the
process crashes. OpenCraft accepts the result: the images and the knob
lights stay for 20 to 35 seconds after the last `CONNECT`, and the controls
do nothing. Then the device stops the USB connection, starts again, and shows
its logo with the knob lights off (measured). A logo on the device thus tells
the user that OpenCraft did not stop normally, or that no program operates
the device. (Inference)

The next start of OpenCraft needs only the usual start sequence (measured
after each stop state). If OpenCraft starts again before the device stops the
connection, the device can still stop it one time. The link then connects
again in 7 to 8 seconds (measured), and the engine draws again.

A panic in the engine thread is the same as a crash, unless the
implementation can send the stop sequence with no risk. Note (2026-11-12):
the implementation sends no stop sequence after a panic. A panic can occur
in the middle of an image transfer, and then the device would read the stop
sequence as image data (`crates/engine/src/runtime.rs`).

## Rejected alternatives

| Alternative | Reason |
|---|---|
| Keep the settings in each profile | The brightness is a property of the device and of the room, not of a set of pages. The owner changes the profile from the device, and the brightness must not change with it. A profile copy would copy the settings. The schema version would become 4, and a build of Phase 4 cannot read a profile with a newer schema version (`store.rs`). VSD Craft and `opendeck-akp05` keep these settings outside the profiles. |
| Keep the settings in `state.toml` | The engine replaces `state.toml` when it has an error, because it holds no data of the user other than the active profile (`profiles.rs`). The settings are data of the user. A separate file keeps that property, and has its own schema version. |
| Keep the settings for each device serial number, as VSD Craft does | The profiles and `state.toml` use the model identifier (decision 0018). OpenCraft operates one device at a time. A serial number in a file is personal data that the user can share by mistake with a copy of the directory. |
| Keep the settings in the shell | The engine must apply them at each connection with no window (rule 3). |
| Brightness 0 to 100 | The values 0 to 15 give the same light. A control that does nothing for its first 15 % is not clear to the user. |
| Stop with `DC` only: the logo | The logo also shows that no binding operates. But the animated logo stays on the desk all day when OpenCraft does not operate the device. The vendor software and `opendeck-akp05` go dark at their stop. With `DC` and `HAN`, the logo shows only after a kill, a crash, or with no host, so the user can see the difference. |
| Stop with byte 1 of `QUCMD` set to `0xFF`, then `DC` | It also gives dark displays. But it changes a device configuration that stays after OpenCraft, also for other programs, and after a USB restart. `DC` and `HAN` give the same state with no stored change. |
| Stop with `HAN` only, with `CLE 0xFF` and `STP`, or with `LIG 0` | The keep-alive timer continues. The device stops the USB connection after 20 to 35 seconds and shows its logo. `CLE 0xFF` leaves the backlight on. `LIG 0` does not switch off the displays. |
| Send `DC` before each start sequence, to start a new timer after a crash | A start directly after `DC` is lost (measured). The gain is small: the link already connects again after a USB stop. |
| Send `QUCMD` only when the user changes the haptic setting | A different program can change the device configuration, and the device keeps it. The setting of OpenCraft is then not true on the device. |

## Results

- The device looks the same as in Phase 4 until the user changes a setting.
  A normal quit now makes the device dark at once.
- A Phase 4 build ignores `device-settings.toml`, and the profile schema
  stays at 3.
- The engine sends 3 more commands at each connection. That is less than
  5 ms (inference from the repaint times in the knowledge base).
- OpenCraft writes the device configuration at each connection. Whether the
  device writes it to permanent memory each time is not known.
- Two devices of one model share one set of settings. Look at this decision
  again when OpenCraft operates more than one device.
- What the haptic command changes is not measured. It needs a person. If the
  command does not switch the vibration, task #124 removes the haptic
  setting from the descriptor, and this decision gets a new record.
  Note of 2026-11-12: the owner did the check. The command switches the
  vibration off and on, thus the haptic setting stays. See
  [kb/streamdock-protocol.md](../kb/streamdock-protocol.md#device-configuration).
- The stop sequence is for a quit. A sleep of the computer is a different
  case: see issue [#43](https://github.com/shantanugoel/OpenCraft/issues/43).
