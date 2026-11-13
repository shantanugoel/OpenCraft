# 0034: The engine holds the dark state and the computer can always wake it

- Status: Accepted
- Date: 2026-11-13

## Context

Phase 7 needs one rule for screen lock, computer sleep, the screen-off timer,
and the device actions. This record extends decisions
[0022](0022-device-settings-and-stop-state.md) and
[0023](0023-wake-from-the-sleep-time-of-the-clocks.md). It replaces the choice
in 0023 to use only the clocks. The shell observes system notices; the
engine still owns all device state.

The camera checks are in
[the protocol entry](../kb/streamdock-protocol.md#dark-state-check-2026-11-13).
Brightness 0 is not screen off. Thus the proposed brightness-0 dark state
cannot meet the requirement. Whether input reports arrive after `HAN` is
not known. No wake path may depend only on such a report.

## Decision

The engine holds three states. None changes the stored brightness or the
profile.

| State | Device commands and link | Input |
|---|---|---|
| Awake | The usual full draw and stored settings | Dispatch actions |
| Screen off | Send `HAN`; keep the handle, input reads, and `CONNECT` timer in operation | If a user input report arrives, wake and consume the first gesture; do not dispatch its action |
| Device sleep | Send `DC`, then `HAN`, as in 0022; suspend the keep-alive, but keep input reads possible | The same rule if input arrives; input is not a required wake source |

A wake sends the full start sequence, all settings, and all surfaces. A
`CONNECT` alone is not a wake command. After `DC`, allow at least 100 ms
before the start sequence. A connection or recovery while screen off must
finish dark, not display the profile. A wake that fails uses the normal
reconnection path.

The computer always has wake paths: open the window, select the menu bar
wake item, unlock the screen, or wake the computer or its displays. The
screen-off and device-sleep actions must explain this fallback. Do not claim
that a device key always wakes either state until the owner check gives the
result. Thus either answer about input after `HAN` leaves the design useful.

| Cause | Engine result |
|---|---|
| Screen lock | Screen off; suppress actions until unlock |
| Screen unlock | Wake and full draw |
| Displays sleep | Screen off |
| Displays wake | Wake, unless the screen is still locked |
| Computer will sleep | Device sleep before sleep; save pending data |
| Computer did wake | Wake and full draw, unless still locked |
| Clock sleep-time jump (0023) | The same recovery as computer did wake; retain as a second source |
| Idle timeout | Screen off, not device sleep |
| Screen off action | Screen off |
| Device sleep action | Device sleep |

While locked, input cannot wake the device or dispatch an action. A window
or menu wake cannot override a lock or a pending computer sleep. Suppress the
release and touch completion of a gesture consumed for wake. Ignore output
from action feedback for device writes while dark; keep the surface state
for the next draw. Settings edits while dark are saved but must not light the
device. A brightness action while awake changes the stored setting through
the same engine request path as these actions.

The shell in `crates/app` observes NSWorkspace will-sleep/did-wake,
screens-did-sleep/screens-did-wake, and the distributed screen-lock/unlock
notices. It sends platform-neutral engine commands. The engine contains no
AppKit code. Duplicate notices are safe. The clock jump is still necessary
for missed notices and non-macOS shells.

The screen-off timer is a device setting, outside the profile. Its default
is Never. User input and an explicit computer-side wake restart the idle
period. It measures device inactivity, not computer inactivity. With Never,
there is no new timer wake-up. With a timeout, use the existing engine loop
and one deadline, not a polling thread. Check the deadline with a fake clock
in tests. Reconnection must not cancel a lock or an explicit dark state.

### Timer setting (2026-11-13)

The device inspector offers Never, 1, 5, 10, 15, 30 and 60 minutes. The file
`device-settings.toml` keeps `screen_off_minutes` in each model table. Zero
means Never. A missing value also means Never. The engine accepts 0 to 1440
minutes and fits a larger value to that range. This additive field uses
settings schema 1: old builds ignore it, but remove it if they save settings.
It changes no profile schema. No timer or light state is in the profile.

One optional engine deadline uses the link clock. Only device input and an
explicit wake restart it; action feedback, file saves and USB keep-alives
do not. A change of the timeout restarts it if the device is awake. Never
cancels it but does not wake a dark device. Reconnection keeps a dark state.
The engine checks the deadline in its existing loop. With Never it does not
read a clock for idle work, and with a timer it limits an existing device
wait to the deadline. It starts no thread and no repeating timer.

## Results

The computer can recover the device without a device key. The owner check
can improve the guidance without a new architecture. Screen off keeps the
link alive; device sleep stops the firmware timer before computer sleep.
Neither state needs a change to persistent firmware configuration.

The physical input check after `HAN`, computer sleep, and lock/unlock remain
owner checks in [#255](https://github.com/shantanugoel/OpenCraft/issues/255).
A fake transport tests the engine policy, not the unmeasured firmware input.
