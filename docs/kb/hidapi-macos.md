# `hidapi` on macOS: disconnection and errors

Last update: 2026-11-11

These facts are for the Rust crate `hidapi` 2.6.7, the version in
`Cargo.lock`. On macOS, this crate compiles the C library `hidapi` 0.15.0,
file `etc/hidapi/mac/hid.c`. A line number in the table is a line in that
file, unless the row names a different file.

## Disconnection

| Fact | Confidence | Source |
|---|---|---|
| When the device is removed, IOKit calls `hid_device_removal_callback`. This function sets the `disconnected` flag of the handle and stops the run loop of the read thread. | Source | Lines 851–861. The callback is registered at line 1075. |
| No code sets the `disconnected` flag back to 0 after the open. A handle that is disconnected cannot operate again. A new connection needs a new open. | Source | Line 157 is the only line that sets 0. |
| After a disconnection, `hid_read_timeout` first returns the reports that are in its queue. Then it returns -1 at once, with the text `hid_read_timeout: device disconnected`. It does not wait for the time limit. | Source | Lines 1242–1261 |
| A read that waits when the disconnection occurs wakes up. It returns -1 with the text `hid_read_timeout:  error waiting for more data` (two spaces). | Source | Lines 975–982, 1226–1237, 1303–1307 |
| After a disconnection, `hid_write` returns -1 at once, with the text `Device is disconnected`. Before the flag is set, a write can fail with the text `IOHIDDeviceSetReport failed: (0x…)`. | Source | Lines 1120–1134 |
| The Rust crate gives each -1 as `HidError::HidApiError`, with the text of `hid_error()`. It has no separate error kind for a disconnection. | Source | `src/hidapi.rs`, lines 177–199 |
| On macOS, the path of an interface is `DevSrvsID:` and the IOKit registry entry ID of the service. | Source | Lines 572–588 |
| After a new connection of the cable, the path is different, because IOKit makes a new registry entry. | Inference | |
| `hid_close` is safe after a disconnection on macOS 10.10 and later. | Source | Lines 1354–1400 |

The owner removed the cable 2 times during `opencraft-probe raw --draw` on
2026-11-11:

| Fact | Confidence | Source |
|---|---|---|
| The probe reported the disconnection less than approximately 1 second after the cable was removed | Measured: 2 times | Camera and probe |
| The first failed read gave the disconnection at once. The probe printed no other error before it. Thus the path was already absent from the enumeration at the first failed read. | Measured: 2 times | Probe |
| After the cable was connected again, the device layer opened the device and the probe drew all surfaces. One time the cable was removed 2.4 seconds after a draw. | Measured: 2 times | Camera and probe |

Thus the text of the error is a weak signal. A stronger signal is the
enumeration: after a disconnection, the path of the open interface is not in
the list. `HidTransport` uses this signal (`crates/core/src/transport/hid.rs`).

## Read errors that go away

| Fact | Confidence | Source |
|---|---|---|
| `hid_read_timeout` returns -1 in three cases only: the device is disconnected, the read thread stopped (`shutdown_thread`), or the wait on the condition variable failed. | Source | Lines 1242–1316 |
| The read thread stops if its run loop returns a result that is not "timed out" and not "handled source". Then `shutdown_thread` is 1 until the close, and each read fails. | Source | Lines 955–972 |
| A read error on macOS that goes away is not probable. A read error that stays and is not a disconnection is possible. The earlier probe text "transient IOKit hiccup" had no source. | Inference | The two rows above |

Thus the device layer treats a read error that stays as a disconnection after
a short time. Then it closes the handle and opens the device again.

## Enumeration

| Fact | Confidence | Source |
|---|---|---|
| Each enumeration runs the run loop of the calling thread until no event is waiting (each run is 1 ms at most). Then it copies the device set of the HID manager. | Source | Lines 492–497, 713, 737 |
| The HID manager is scheduled on the run loop of the thread that calls `hid_init` first. The first `HidApi::new()` calls `hid_init`. | Source | Lines 436–444, 464–473 |
| If a different thread enumerates, the device set can be old, because the HID manager gets its changes on the run loop of the first thread. Thus the engine must create the first `HidApi` and enumerate on one thread. | Inference | The row above |
| `hid_init` sets exclusive open (`kIOHIDOptionsTypeSeizeDevice`). Only the crate feature `macos-shared-device` changes this. OpenCraft does not use that feature. | Source | Lines 469, 1501–1504; `src/lib.rs`, lines 179–182 |
| Because the open is exclusive, the old handle must be closed before the same device opens again. | Inference | The row above |

## Not known

- Which error `IOHIDDeviceSetReport` gives when the cable is removed during a
  write.
- Whether a write can block for a long time when the device does not answer.
- Whether macOS removes the device and adds it again when the computer sleeps
  and wakes. If it does not, the device layer sees no disconnection. The
  check is issue [#43](https://github.com/shantanugoel/OpenCraft/issues/43).
  See [n4-pro-hardware.md](n4-pro-hardware.md#not-known) for what the device
  shows after a wake.

## Sources

- `hidapi` crate 2.6.7: <https://crates.io/crates/hidapi/2.6.7>
- `hidapi` C library, macOS backend: <https://github.com/libusb/hidapi/blob/master/mac/hid.c>
