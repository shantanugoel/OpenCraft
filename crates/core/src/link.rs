//! Keep a connection to a device: detect a disconnection and connect again.
//!
//! [`Link`] holds one device that can go away and come back. The caller asks
//! for the next event with [`Link::next_event`]. The call blocks, so the caller
//! runs it on a thread that is not the user interface thread (architecture
//! rule 5). The link uses no async runtime (decision 0003).
//!
//! The events are:
//!
//! - [`LinkEvent::Connected`] after each connection, also the first one. The
//!   caller must then draw all surfaces again (architecture rule 10). It uses
//!   [`Link::with_device`] for this.
//! - [`LinkEvent::Disconnected`] one time after each loss of the device.
//! - [`LinkEvent::Input`] for each input report.
//! - [`LinkEvent::Timeout`] when the time limit expires with no event.
//!
//! While no device is connected, the link asks its [`Connector`] for a device
//! one time each [`RETRY_INTERVAL`]. Between two tries it sleeps. It does not
//! poll in a loop.
//!
//! While a device is connected, the link sends the keep-alive command each
//! [`KEEP_ALIVE_INTERVAL`] from [`Link::next_event`]. Without it, the device
//! stops the connection. The caller does not send the keep-alive. It calls
//! [`Link::next_event`] in a loop, as for the input.
//!
//! The link also finds a wake of the computer from sleep: see
//! [`LinkEvent::Woke`]. After a sleep, the device possibly lost its state or
//! its keep-alive. The caller then draws all surfaces again, as after a
//! connection. The reasons are in
//! `docs/decisions/0023-wake-from-the-sleep-time-of-the-clocks.md`.
//!
//! A [`Connector`] and a [`Clock`] are interfaces, so a test can supply fakes
//! and run with no device and no real wait (architecture rule 7).

use crate::connection::Device;
use crate::error::{Error, Result};
use crate::transport::{list_devices, select, HidTransport, Transport};
use std::time::{Duration, Instant};

/// The time between two tries to find the device while it is not connected.
pub const RETRY_INTERVAL: Duration = Duration::from_secs(1);

/// The number of errors in sequence after which the link stops the use of the
/// device and opens it again. An error that is not [`Error::Disconnected`]
/// can go away. If it does not go away, the handle is possibly broken.
pub const ERROR_LIMIT: u32 = 10;

/// The time between two keep-alive commands while a device is connected.
///
/// After the start command, the device counts periods of 15 seconds. If a
/// period has no keep-alive, the device stops the USB connection. Thus the
/// interval must be less than 15 seconds. The vendor SDK also uses 10
/// seconds. See the section "Keep-alive and stop" in
/// `docs/kb/streamdock-protocol.md`.
pub const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(10);

/// The pause after an error that is not a disconnection. A failed read can
/// return at once, so this pause prevents a busy loop. With [`ERROR_LIMIT`],
/// an error that stays becomes a disconnection after approximately 1 second.
pub const ERROR_PAUSE: Duration = Duration::from_millis(100);

/// A sleep of the computer that is longer than this gives
/// [`LinkEvent::Woke`].
///
/// The sleep time of [`SystemClock::asleep`] does not change while the
/// computer is awake (measured: no change in 6.9 days). Thus a short limit
/// gives no false wake. The limit is not zero, so that a small difference
/// between two clocks is never a wake.
pub const WAKE_THRESHOLD: Duration = Duration::from_secs(2);

/// Finds the device and opens it.
pub trait Connector {
    type Transport: Transport;

    /// Try to open the device one time.
    ///
    /// - `Ok(Some(device))`: the device is open.
    /// - `Ok(None)`: the device is not present now.
    /// - `Err(e)`: the device is present, but it did not open.
    fn connect(&mut self) -> Result<Option<Device<Self::Transport>>>;
}

/// The real connector. It enumerates the HID devices and opens only the vendor
/// interface of a known device, through [`select`] and [`Device::open`]
/// (architecture rule 6).
#[derive(Debug, Default, Clone, Copy)]
pub struct HidConnector;

impl Connector for HidConnector {
    type Transport = HidTransport;

    fn connect(&mut self) -> Result<Option<Device<HidTransport>>> {
        let found = list_devices()?;
        match select(&found) {
            Ok(f) => Device::open(f).map(Some),
            Err(Error::DeviceNotFound) => Ok(None),
            // During a new connection, the operating system can show the
            // interfaces of the device one at a time. Try again later.
            Err(Error::NoVendorInterface) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// The time source of a [`Link`]. A test supplies a clock that does not wait.
pub trait Clock {
    /// The time for the waits and the keep-alive. On macOS and on Linux,
    /// [`Instant`] does not count the time while the computer sleeps.
    fn now(&self) -> Instant;

    /// Wait for `duration`.
    fn sleep(&mut self, duration: Duration);

    /// The total time that the computer slept, from a start that does not
    /// change while the process operates. `None` if the clock cannot tell.
    /// Then the link finds no wake.
    ///
    /// The link compares two values to find a sleep. The default is `None`.
    fn asleep(&self) -> Option<Duration> {
        None
    }
}

/// The real clock: [`Instant::now`], [`std::thread::sleep`], and
/// [`computer_sleep_time`].
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }

    fn asleep(&self) -> Option<Duration> {
        computer_sleep_time()
    }
}

/// The total time that the computer slept since it started. `None` on a
/// platform with no such clock.
///
/// - macOS: `CLOCK_MONOTONIC_RAW` minus `CLOCK_UPTIME_RAW`. The two clocks
///   count at the same rate, but `CLOCK_UPTIME_RAW` stops while the computer
///   sleeps.
/// - Linux: `CLOCK_BOOTTIME` minus `CLOCK_MONOTONIC`. `CLOCK_MONOTONIC`
///   stops while the computer is suspended.
///
/// The facts and their sources are in `docs/kb/sleep-and-wake.md`. The
/// value does not depend on the wall clock, so a change of the time of day
/// is not a sleep.
pub fn computer_sleep_time() -> Option<Duration> {
    #[cfg(target_vendor = "apple")]
    {
        let awake = read_clock(libc::CLOCK_UPTIME_RAW)?;
        let total = read_clock(libc::CLOCK_MONOTONIC_RAW)?;
        Some(total.saturating_sub(awake))
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let awake = read_clock(libc::CLOCK_MONOTONIC)?;
        let total = read_clock(libc::CLOCK_BOOTTIME)?;
        Some(total.saturating_sub(awake))
    }
    #[cfg(not(any(target_vendor = "apple", target_os = "linux", target_os = "android")))]
    {
        None
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
fn read_clock(clock: libc::clockid_t) -> Option<Duration> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `time` is a valid `timespec` that the call writes.
    if unsafe { libc::clock_gettime(clock, &mut time) } != 0 {
        return None;
    }
    let secs = u64::try_from(time.tv_sec).ok()?;
    let nanos = u32::try_from(time.tv_nsec).ok()?;
    Some(Duration::new(secs, nanos))
}

/// One event from [`Link::next_event`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkEvent {
    /// A device is now connected. Draw all surfaces again.
    Connected,
    /// The device is gone. The link looks for it again.
    Disconnected,
    /// One input report from the device. Decode it with [`crate::decode`].
    Input(Vec<u8>),
    /// The computer slept for `slept`, more than [`WAKE_THRESHOLD`], and is
    /// awake now. The device possibly lost its images, its start, or its
    /// keep-alive timer during the sleep. If a device is connected, do the
    /// same as for [`LinkEvent::Connected`]: send the start sequence and draw
    /// all surfaces again (architecture rule 10).
    ///
    /// The link sent the keep-alive just before this event. If a write fails
    /// after the wake, the usual rules apply: the link drops the device and
    /// connects again, and [`LinkEvent::Connected`] follows. With no device
    /// connected, the link tries to connect at once.
    Woke { slept: Duration },
    /// The time limit expired with no other event.
    Timeout,
}

/// A device that can disconnect and connect again. See the module
/// documentation.
pub struct Link<C: Connector, K: Clock = SystemClock> {
    connector: C,
    clock: K,
    device: Option<Device<C::Transport>>,
    /// A `Connected` or `Disconnected` event that the caller did not get yet.
    pending: Option<LinkEvent>,
    /// The time of the next try to connect. `None` means "try at once".
    next_try: Option<Instant>,
    /// Errors in sequence that are not a disconnection.
    errors: u32,
    /// When the next keep-alive goes to the device.
    keep_alive: KeepAlive,
    /// True after [`Link::stop`]. The link then sends nothing and does not
    /// connect again.
    stopped: bool,
    /// The last value of [`Clock::asleep`].
    asleep: Option<Duration>,
    /// The sleep time of a wake that the caller did not get yet.
    woke: Option<Duration>,
}

/// The keep-alive policy. All decisions about the keep-alive are here.
///
/// The keep-alive goes out on a fixed interval from the last keep-alive. The
/// policy does not skip a keep-alive because a different command went to the
/// device: a draw does not count as a keep-alive for the device (measured).
#[derive(Debug, Clone, Copy)]
struct KeepAlive {
    /// The interval. `None` means "send no keep-alive".
    interval: Option<Duration>,
    /// The time of the next keep-alive. `None` while no device is connected.
    next: Option<Instant>,
}

impl KeepAlive {
    fn new() -> Self {
        Self {
            interval: Some(KEEP_ALIVE_INTERVAL),
            next: None,
        }
    }

    /// A device is connected, or a keep-alive went out, at `now`. The interval
    /// starts again.
    fn restart(&mut self, now: Instant) {
        self.next = self.interval.map(|i| now + i);
    }

    /// No device is connected. Send nothing.
    fn stop(&mut self) {
        self.next = None;
    }

    /// Make the next keep-alive due at `now`, if a keep-alive is scheduled.
    fn due_at(&mut self, now: Instant) {
        if self.next.is_some() {
            self.next = Some(now);
        }
    }

    /// The computer slept for `slept`. [`Instant`] did not count that time,
    /// but the device did. Move the next keep-alive earlier by `slept`, so
    /// that the keep-alive follows the time of the device.
    fn after_sleep(&mut self, now: Instant, slept: Duration) {
        if let Some(next) = self.next {
            self.next = Some(match next.checked_sub(slept) {
                Some(earlier) if earlier > now => earlier,
                _ => now,
            });
        }
    }

    /// True if the keep-alive must go out at `now`.
    fn is_due(&self, now: Instant) -> bool {
        self.next.is_some_and(|t| now >= t)
    }

    /// The longest time that a read can wait from `now`, so that the next
    /// keep-alive is not late. `None` if no keep-alive is scheduled.
    fn wait_limit(&self, now: Instant) -> Option<Duration> {
        self.next.map(|t| t.saturating_duration_since(now))
    }
}

impl<C: Connector> Link<C> {
    /// Make a link with no device. The first call to [`Self::next_event`]
    /// tries to connect.
    pub fn new(connector: C) -> Self {
        Self::with_clock(connector, SystemClock)
    }

    /// Make a link with a device that is already open. The first event is
    /// [`LinkEvent::Connected`].
    pub fn from_device(connector: C, device: Device<C::Transport>) -> Self {
        let mut link = Self::new(connector);
        link.set_device(device);
        link
    }
}

impl<C: Connector, K: Clock> Link<C, K> {
    /// Make a link with no device that uses `clock` for all waits.
    pub fn with_clock(connector: C, clock: K) -> Self {
        let asleep = clock.asleep();
        Self {
            connector,
            clock,
            device: None,
            pending: None,
            next_try: None,
            errors: 0,
            keep_alive: KeepAlive::new(),
            stopped: false,
            asleep,
            woke: None,
        }
    }

    /// Stop the use of the device: send the stop sequence of
    /// [`Device::stop`] (`DC`, then `HAN`) if a device is connected, and
    /// close the device.
    ///
    /// After this call, the link sends no keep-alive and no other command,
    /// and it does not connect again. [`Self::next_event`] then waits for its
    /// time limit and gives [`LinkEvent::Timeout`], and
    /// [`Self::with_device`] gives [`Error::Disconnected`].
    ///
    /// The function writes 2 reports, and it does not retry. If the device is
    /// gone, the first write fails at once. A write that blocks blocks this
    /// call: the transport has no time limit for a write. A caller that must
    /// not wait for a long time puts its own limit around the thread that
    /// calls this function.
    ///
    /// Returns `Ok(true)` if the stop sequence went to the device, `Ok(false)`
    /// if no device was connected, and the write error if a write failed.
    /// The device is closed in each case. A second call gives `Ok(false)`.
    pub fn stop(&mut self) -> Result<bool> {
        self.stopped = true;
        self.pending = None;
        self.keep_alive.stop();
        let Some(mut device) = self.device.take() else {
            return Ok(false);
        };
        let result = device.stop();
        // Close the device after the writes. On macOS, `hid_close` is safe
        // also after a disconnection (kb: hidapi-macos.md).
        drop(device);
        result.map(|()| true)
    }

    /// True after [`Self::stop`].
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// Change the keep-alive interval. `None` or zero stops the keep-alive. The
    /// default is [`KEEP_ALIVE_INTERVAL`]. Use a different value only for a
    /// measurement: without the keep-alive, the device stops the connection.
    /// If a device is connected, the new interval starts now.
    pub fn set_keep_alive_interval(&mut self, interval: Option<Duration>) {
        self.keep_alive.interval = interval.filter(|i| !i.is_zero());
        if self.device.is_some() {
            self.keep_alive.restart(self.clock.now());
        }
    }

    /// True while a device is connected.
    pub fn is_connected(&self) -> bool {
        self.device.is_some()
    }

    /// The connected device, if there is one. To write to it, use
    /// [`Self::with_device`].
    pub fn device(&self) -> Option<&Device<C::Transport>> {
        self.device.as_ref()
    }

    /// The connector.
    pub fn connector(&self) -> &C {
        &self.connector
    }

    /// The clock.
    pub fn clock(&self) -> &K {
        &self.clock
    }

    /// Run `f` on the connected device, for example to draw all surfaces.
    ///
    /// If no device is connected, this returns [`Error::Disconnected`] and
    /// does not run `f`. If `f` gives [`Error::Disconnected`], the link drops
    /// the device, and the next event is [`LinkEvent::Disconnected`]. Do all
    /// writes through this function, so that the link sees each disconnection.
    pub fn with_device<R>(
        &mut self,
        f: impl FnOnce(&mut Device<C::Transport>) -> Result<R>,
    ) -> Result<R> {
        let device = self.device.as_mut().ok_or(Error::Disconnected)?;
        match f(device) {
            Ok(value) => {
                self.errors = 0;
                Ok(value)
            }
            Err(e) => {
                if self.count_error(&e) {
                    self.drop_device();
                    self.pending = Some(LinkEvent::Disconnected);
                }
                Err(e)
            }
        }
    }

    /// Wait up to `timeout` for the next event.
    ///
    /// The call blocks. While a device is connected, it waits for an input
    /// report. While no device is connected, it tries to connect each
    /// [`RETRY_INTERVAL`] and sleeps between the tries.
    ///
    /// While a device is connected, the call also sends the keep-alive when
    /// [`KEEP_ALIVE_INTERVAL`] expires. A long `timeout` does not make the
    /// keep-alive late: the call waits for input in shorter parts, and gives
    /// [`LinkEvent::Timeout`] only when the full `timeout` expires. Call this
    /// function again at once after each event, so that the keep-alive is not
    /// late. A keep-alive that fails is a write error, as in
    /// [`Self::with_device`].
    ///
    /// An `Err` does not stop the link. Call again. It is one of these:
    ///
    /// - An error from a read that can go away. The link paused for
    ///   [`ERROR_PAUSE`] before it returned, so a loop of calls does not use
    ///   the processor. After [`ERROR_LIMIT`] such errors in sequence, the
    ///   event is [`LinkEvent::Disconnected`].
    /// - An error from the connector: the device is present but did not open.
    ///   The link tries again after [`RETRY_INTERVAL`].
    pub fn next_event(&mut self, timeout: Duration) -> Result<LinkEvent> {
        if self.stopped {
            // No busy loop for a caller that continues after the stop.
            self.clock.sleep(timeout);
            return Ok(LinkEvent::Timeout);
        }
        if let Some(event) = self.pending.take() {
            return Ok(event);
        }
        let deadline = self.clock.now() + timeout;
        loop {
            self.check_wake();
            if let Some(slept) = self.woke.take() {
                if self.device.is_some() {
                    // A write error after the wake is handled as always. A
                    // disconnection becomes the next event.
                    let _ = self.send_keep_alive_if_due();
                }
                return Ok(LinkEvent::Woke { slept });
            }
            if self.device.is_some() {
                self.send_keep_alive_if_due()?;
                if let Some(event) = self.pending.take() {
                    return Ok(event);
                }
            }
            if let Some(device) = self.device.as_mut() {
                let now = self.clock.now();
                let remaining = deadline.saturating_duration_since(now);
                let wait = match self.keep_alive.wait_limit(now) {
                    Some(limit) if limit < remaining => limit,
                    _ => remaining,
                };
                return match device.read(wait) {
                    Ok(Some(report)) => {
                        self.errors = 0;
                        Ok(LinkEvent::Input(report))
                    }
                    Ok(None) => {
                        self.errors = 0;
                        if wait < remaining {
                            // The wait stopped for the keep-alive, not for
                            // the caller.
                            continue;
                        }
                        Ok(LinkEvent::Timeout)
                    }
                    Err(e) => {
                        if self.count_error(&e) {
                            self.drop_device();
                            Ok(LinkEvent::Disconnected)
                        } else {
                            self.clock.sleep(ERROR_PAUSE);
                            Err(e)
                        }
                    }
                };
            }

            let now = self.clock.now();
            if self.next_try.is_none_or(|t| now >= t) {
                self.next_try = Some(now + RETRY_INTERVAL);
                if let Some(device) = self.connector.connect()? {
                    self.set_device(device);
                    self.pending = None;
                    return Ok(LinkEvent::Connected);
                }
            }

            let now = self.clock.now();
            if now >= deadline {
                return Ok(LinkEvent::Timeout);
            }
            let next_try = self.next_try.unwrap_or(now);
            self.clock
                .sleep(next_try.min(deadline).saturating_duration_since(now));
        }
    }

    /// Send the keep-alive if it is due. A write error is handled as in
    /// [`Self::with_device`]: [`Error::Disconnected`] drops the device, and
    /// the pending event becomes [`LinkEvent::Disconnected`]. A different
    /// error counts toward [`ERROR_LIMIT`]. Then the link pauses for
    /// [`ERROR_PAUSE`], returns the error, and tries again at the next call.
    fn send_keep_alive_if_due(&mut self) -> Result<()> {
        let now = self.clock.now();
        if !self.keep_alive.is_due(now) {
            return Ok(());
        }
        match self.with_device(|d| d.keep_alive()) {
            Ok(()) => {
                self.keep_alive.restart(now);
                Ok(())
            }
            // The device is gone. The pending event tells the caller.
            Err(_) if !self.is_connected() => Ok(()),
            Err(e) => {
                self.clock.sleep(ERROR_PAUSE);
                Err(e)
            }
        }
    }

    /// Read the sleep time of the clock. If the computer slept, move the
    /// keep-alive. If it slept for more than [`WAKE_THRESHOLD`], record a
    /// wake: the keep-alive is due at once, and with no device, the next try
    /// to connect is at once.
    fn check_wake(&mut self) {
        let Some(total) = self.clock.asleep() else {
            return;
        };
        let Some(last) = self.asleep else {
            self.asleep = Some(total);
            return;
        };
        // The two clocks are read one after the other, so the value changes
        // by some nanoseconds at each read. Such a change is not a sleep, and
        // it must not add up.
        let slept = total.saturating_sub(last);
        if slept < Duration::from_millis(1) {
            return;
        }
        self.asleep = Some(total);
        let now = self.clock.now();
        self.keep_alive.after_sleep(now, slept);
        if slept > WAKE_THRESHOLD {
            if self.device.is_some() {
                self.keep_alive.due_at(now);
            }
            self.next_try = None;
            self.woke = Some(self.woke.unwrap_or_default() + slept);
        }
    }

    fn set_device(&mut self, device: Device<C::Transport>) {
        self.device = Some(device);
        self.errors = 0;
        self.pending = Some(LinkEvent::Connected);
        self.keep_alive.restart(self.clock.now());
    }

    /// Close the device. Try to connect again at once: if the handle broke but
    /// the device is still there, it opens again with no wait.
    fn drop_device(&mut self) {
        // Drop the handle before a new open. On macOS, `hidapi` opens the
        // device for exclusive use.
        self.device = None;
        self.errors = 0;
        self.next_try = None;
        self.keep_alive.stop();
    }

    /// Count one error. Return true if the link must drop the device.
    fn count_error(&mut self, error: &Error) -> bool {
        if matches!(error, Error::Disconnected) {
            return true;
        }
        self.errors += 1;
        self.errors >= ERROR_LIMIT
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DESCRIPTORS;
    use crate::protocol;
    use crate::transport::FakeTransport;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::ops::{Deref, DerefMut};
    use std::rc::Rc;

    /// The fake time, and the record of all write tries to all transports.
    /// The clock and each transport of one test share it.
    struct World {
        start: Instant,
        now: Cell<Instant>,
        /// The fake sleep time of the computer. A test adds to it to
        /// simulate a sleep. `now` does not move then, as on macOS.
        asleep: Cell<Duration>,
        /// The time from the start, and the report, of each write try.
        writes: RefCell<Vec<(Duration, Vec<u8>)>>,
    }

    impl World {
        fn new() -> Rc<Self> {
            let start = Instant::now();
            Rc::new(Self {
                start,
                now: Cell::new(start),
                asleep: Cell::new(Duration::from_secs(1000)),
                writes: RefCell::new(Vec::new()),
            })
        }

        fn elapsed(&self) -> Duration {
            self.now.get() - self.start
        }

        fn advance(&self, duration: Duration) {
            self.now.set(self.now.get() + duration);
        }

        /// The times of the keep-alive write tries.
        fn keep_alive_times(&self) -> Vec<Duration> {
            let mut expected = protocol::keep_alive();
            expected.resize(1 + DESCRIPTORS[0].packet_size, 0);
            self.writes
                .borrow()
                .iter()
                .filter(|(_, report)| *report == expected)
                .map(|(at, _)| *at)
                .collect()
        }
    }

    /// A [`FakeTransport`] in the fake time. A read with no report waits for
    /// its full time limit, as a real read does. Each write try goes to the
    /// record of the [`World`].
    struct Timed {
        fake: FakeTransport,
        world: Rc<World>,
    }

    impl Deref for Timed {
        type Target = FakeTransport;
        fn deref(&self) -> &FakeTransport {
            &self.fake
        }
    }

    impl DerefMut for Timed {
        fn deref_mut(&mut self) -> &mut FakeTransport {
            &mut self.fake
        }
    }

    impl Transport for Timed {
        fn write(&mut self, report: &[u8]) -> Result<()> {
            self.world
                .writes
                .borrow_mut()
                .push((self.world.elapsed(), report.to_vec()));
            self.fake.write(report)
        }

        fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<Option<usize>> {
            let result = self.fake.read(buf, timeout);
            if matches!(result, Ok(None)) {
                self.world.advance(timeout);
            }
            result
        }

        fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize> {
            self.fake.get_feature_report(buf)
        }
    }

    /// A connector that gives the transports of a script, in order. `None` in
    /// the script, or an empty script, means "not present".
    struct FakeConnector {
        script: VecDeque<Option<FakeTransport>>,
        tries: u32,
        world: Rc<World>,
    }

    impl FakeConnector {
        fn new(world: &Rc<World>, script: impl IntoIterator<Item = Option<FakeTransport>>) -> Self {
            Self {
                script: script.into_iter().collect(),
                tries: 0,
                world: Rc::clone(world),
            }
        }
    }

    impl Connector for FakeConnector {
        type Transport = Timed;

        fn connect(&mut self) -> Result<Option<Device<Timed>>> {
            self.tries += 1;
            Ok(self.script.pop_front().flatten().map(|fake| {
                let timed = Timed {
                    fake,
                    world: Rc::clone(&self.world),
                };
                Device::new(timed, &DESCRIPTORS[0])
            }))
        }
    }

    /// A clock that does not wait. A sleep moves the time forward.
    struct FakeClock {
        world: Rc<World>,
        sleeps: Vec<Duration>,
    }

    impl FakeClock {
        fn new(world: &Rc<World>) -> Self {
            Self {
                world: Rc::clone(world),
                sleeps: Vec::new(),
            }
        }

        fn slept(&self) -> Duration {
            self.sleeps.iter().sum()
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> Instant {
            self.world.now.get()
        }

        fn sleep(&mut self, duration: Duration) {
            self.sleeps.push(duration);
            self.world.advance(duration);
        }

        fn asleep(&self) -> Option<Duration> {
            Some(self.world.asleep.get())
        }
    }

    type TestLink = Link<FakeConnector, FakeClock>;

    const STEP: Duration = Duration::from_millis(100);

    fn link(script: Vec<Option<FakeTransport>>) -> TestLink {
        let world = World::new();
        Link::with_clock(FakeConnector::new(&world, script), FakeClock::new(&world))
    }

    /// A link that is connected to a fake. The `Connected` event is taken.
    fn connected_link(script: Vec<Option<FakeTransport>>) -> TestLink {
        let mut script = script;
        script.insert(0, Some(FakeTransport::new()));
        let mut link = link(script);
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Connected);
        link
    }

    fn disconnect(link: &mut TestLink) {
        link.with_device(|d| {
            d.transport_mut().disconnect();
            Ok(())
        })
        .unwrap();
    }

    /// Collect the events of `n` calls. Errors become `None`.
    fn events(link: &mut TestLink, n: usize) -> Vec<Option<LinkEvent>> {
        (0..n).map(|_| link.next_event(STEP).ok()).collect()
    }

    fn world(link: &TestLink) -> &World {
        &link.clock().world
    }

    /// Call `next_event(STEP)` until the fake time is `until` from the start.
    /// Give each event that is not `Timeout`, with its time.
    fn run_until(link: &mut TestLink, until: Duration) -> Vec<(Duration, Option<LinkEvent>)> {
        let mut got = Vec::new();
        while world(link).elapsed() < until {
            let event = link.next_event(STEP).ok();
            if event != Some(LinkEvent::Timeout) {
                got.push((world(link).elapsed(), event));
            }
        }
        got
    }

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn a_read_disconnection_gives_one_disconnected_event() {
        let mut link = connected_link(vec![]);
        disconnect(&mut link);

        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Disconnected);
        assert!(!link.is_connected());
        let later = events(&mut link, 20);
        assert!(later.iter().all(|e| *e == Some(LinkEvent::Timeout)));
    }

    #[test]
    fn while_not_present_the_link_tries_each_interval_and_gives_no_signal() {
        let mut link = link(vec![]);

        // One call of 5 seconds: tries at 0, 1, 2, 3, 4 and 5 seconds.
        let timeout = Duration::from_secs(5);
        assert_eq!(link.next_event(timeout).unwrap(), LinkEvent::Timeout);
        assert_eq!(link.connector().tries, 6);
        assert_eq!(link.clock().slept(), timeout);
        assert!(link.clock().sleeps.iter().all(|&s| s == RETRY_INTERVAL));

        // Short calls: one try each second, not one try each call.
        let later = events(&mut link, 30);
        assert!(later.iter().all(|e| *e == Some(LinkEvent::Timeout)));
        assert_eq!(link.connector().tries, 6 + 3);
        assert!(!link.is_connected());
    }

    #[test]
    fn a_new_device_gives_connected_and_takes_the_redraw() {
        let mut link = connected_link(vec![None, None, Some(FakeTransport::new())]);
        disconnect(&mut link);
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Disconnected);

        let mut got = Vec::new();
        for _ in 0..50 {
            let event = link.next_event(STEP).unwrap();
            if event != LinkEvent::Timeout {
                got.push(event);
                break;
            }
        }
        assert_eq!(got, vec![LinkEvent::Connected]);
        // A try right after the loss, then one each second.
        assert_eq!(link.connector().tries, 1 + 3);

        link.with_device(|d| d.clear_all()).unwrap();
        let writes = link.device().unwrap().transport().writes();
        assert_eq!(writes.len(), 2, "clear and commit go to the new device");
        assert_eq!(&writes[1][1..9], b"CRT\x00\x00STP");
    }

    #[test]
    fn a_write_disconnection_during_a_draw_is_a_disconnection() {
        let mut fake = FakeTransport::new();
        fake.disconnect();
        let mut link = link(vec![Some(fake)]);
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Connected);

        assert!(matches!(
            link.with_device(|d| d.clear_all()),
            Err(Error::Disconnected)
        ));
        assert!(!link.is_connected());
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Disconnected);
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Timeout);
        assert!(matches!(
            link.with_device(|d| d.clear_all()),
            Err(Error::Disconnected)
        ));
    }

    #[test]
    fn a_read_error_that_goes_away_is_not_a_disconnection() {
        let mut link = connected_link(vec![]);
        link.with_device(|d| {
            d.transport_mut().fail_next_read("hiccup");
            d.transport_mut().push_input(b"ACK".to_vec());
            Ok(())
        })
        .unwrap();

        assert!(matches!(link.next_event(STEP), Err(Error::Transport(_))));
        assert_eq!(link.clock().sleeps, vec![ERROR_PAUSE], "no busy loop");
        assert_eq!(
            link.next_event(STEP).unwrap(),
            LinkEvent::Input(b"ACK".to_vec())
        );
        assert!(link.is_connected());
        assert_eq!(link.connector().tries, 1);
    }

    #[test]
    fn errors_in_sequence_below_the_limit_are_not_a_disconnection() {
        let mut link = connected_link(vec![]);
        for recovery in 0..3 {
            // Do not use with_device here: a successful closure resets the
            // error count and would hide a missing reset after a good read.
            link.device
                .as_mut()
                .unwrap()
                .transport_mut()
                .fail_reads("hiccup");
            for _ in 0..ERROR_LIMIT - 1 {
                assert!(matches!(link.next_event(STEP), Err(Error::Transport(_))));
            }
            link.device.as_mut().unwrap().transport_mut().clear_errors();
            match recovery {
                0 => assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Timeout),
                1 => {
                    link.device
                        .as_mut()
                        .unwrap()
                        .transport_mut()
                        .push_input(vec![7]);
                    assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Input(vec![7]));
                }
                _ => link.with_device(|d| d.commit()).unwrap(),
            }
        }
        // The last success also resets the count.
        link.device
            .as_mut()
            .unwrap()
            .transport_mut()
            .fail_reads("hiccup");
        for _ in 0..ERROR_LIMIT - 1 {
            assert!(matches!(link.next_event(STEP), Err(Error::Transport(_))));
        }
        assert!(link.is_connected());
    }

    #[test]
    fn an_error_that_stays_becomes_a_disconnection_and_the_device_opens_again() {
        let mut link = connected_link(vec![Some(FakeTransport::new())]);
        link.with_device(|d| {
            d.transport_mut().fail_reads("broken handle");
            Ok(())
        })
        .unwrap();

        let got = events(&mut link, ERROR_LIMIT as usize + 1);
        let errors = got.iter().filter(|e| e.is_none()).count();
        assert_eq!(errors, ERROR_LIMIT as usize - 1);
        assert_eq!(got[ERROR_LIMIT as usize - 1], Some(LinkEvent::Disconnected));
        // The device is still present, so it opens again at once.
        assert_eq!(got[ERROR_LIMIT as usize], Some(LinkEvent::Connected));
        assert!(link.clock().slept() <= Duration::from_secs(1));
    }

    #[test]
    fn disconnect_and_connect_again_two_times() {
        let mut link = connected_link(vec![
            None,
            Some(FakeTransport::new()),
            None,
            Some(FakeTransport::new()),
        ]);
        let mut got = Vec::new();
        for _ in 0..2 {
            disconnect(&mut link);
            loop {
                let event = link.next_event(STEP).unwrap();
                if event == LinkEvent::Connected {
                    got.push(event);
                    break;
                }
                if event != LinkEvent::Timeout {
                    got.push(event);
                }
            }
        }
        assert_eq!(
            got,
            vec![
                LinkEvent::Disconnected,
                LinkEvent::Connected,
                LinkEvent::Disconnected,
                LinkEvent::Connected
            ]
        );
    }

    #[test]
    fn from_device_gives_connected_first() {
        let world = World::new();
        let timed = Timed {
            fake: FakeTransport::new(),
            world: Rc::clone(&world),
        };
        let device = Device::new(timed, &DESCRIPTORS[0]);
        let mut link = Link::from_device(FakeConnector::new(&world, []), device);
        assert_eq!(
            link.next_event(Duration::ZERO).unwrap(),
            LinkEvent::Connected
        );
        assert_eq!(link.next_event(Duration::ZERO).unwrap(), LinkEvent::Timeout);
        assert_eq!(link.connector().tries, 0);
    }

    #[test]
    fn input_reports_pass_through_in_order() {
        let mut link = connected_link(vec![]);
        link.with_device(|d| {
            d.transport_mut().push_input(vec![1]);
            d.transport_mut().push_input(vec![2]);
            Ok(())
        })
        .unwrap();
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Input(vec![1]));
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Input(vec![2]));
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Timeout);
    }

    #[test]
    fn a_connector_error_is_returned_and_the_link_tries_again() {
        struct Failing(u32);
        impl Connector for Failing {
            type Transport = FakeTransport;
            fn connect(&mut self) -> Result<Option<Device<FakeTransport>>> {
                self.0 += 1;
                if self.0 == 1 {
                    Err(Error::PermissionDenied)
                } else {
                    Ok(Some(Device::new(FakeTransport::new(), &DESCRIPTORS[0])))
                }
            }
        }
        let mut link = Link::with_clock(Failing(0), FakeClock::new(&World::new()));
        assert!(matches!(
            link.next_event(STEP),
            Err(Error::PermissionDenied)
        ));
        let mut event = LinkEvent::Timeout;
        for _ in 0..20 {
            event = link.next_event(STEP).unwrap();
            if event != LinkEvent::Timeout {
                break;
            }
        }
        assert_eq!(event, LinkEvent::Connected);
        assert_eq!(link.connector().0, 2);
        assert_eq!(link.clock().slept(), RETRY_INTERVAL);
    }

    #[test]
    fn the_keep_alive_goes_out_each_interval() {
        let mut link = connected_link(vec![]);
        assert_eq!(KEEP_ALIVE_INTERVAL, secs(10));

        let got = run_until(&mut link, secs(35));
        assert!(got.is_empty(), "no event other than Timeout: {got:?}");
        assert_eq!(
            world(&link).keep_alive_times(),
            vec![secs(10), secs(20), secs(30)]
        );
        assert_eq!(world(&link).writes.borrow().len(), 3, "only keep-alives");
    }

    #[test]
    fn a_long_timeout_does_not_make_the_keep_alive_late() {
        let mut link = connected_link(vec![]);

        // One call of 60 seconds gives one Timeout after the full time.
        assert_eq!(link.next_event(secs(60)).unwrap(), LinkEvent::Timeout);
        assert_eq!(world(&link).elapsed(), secs(60));
        assert_eq!(
            world(&link).keep_alive_times(),
            vec![secs(10), secs(20), secs(30), secs(40), secs(50)]
        );

        // The keep-alive that is due at the end goes out at the next call.
        assert_eq!(link.next_event(Duration::ZERO).unwrap(), LinkEvent::Timeout);
        assert_eq!(world(&link).keep_alive_times().len(), 6);
    }

    #[test]
    fn input_does_not_delay_the_keep_alive() {
        let mut link = connected_link(vec![]);
        run_until(&mut link, secs(9));
        // A different command does not move the keep-alive (fixed interval).
        link.with_device(|d| d.commit()).unwrap();
        link.with_device(|d| {
            d.transport_mut().push_input(vec![1]);
            Ok(())
        })
        .unwrap();
        let got = run_until(&mut link, secs(21));
        assert_eq!(got, vec![(secs(9), Some(LinkEvent::Input(vec![1])))]);
        assert_eq!(world(&link).keep_alive_times(), vec![secs(10), secs(20)]);
    }

    #[test]
    fn no_keep_alive_while_disconnected() {
        let mut link = connected_link(vec![]);
        run_until(&mut link, secs(5));
        disconnect(&mut link);

        let got = run_until(&mut link, secs(60));
        assert_eq!(got, vec![(secs(5), Some(LinkEvent::Disconnected))]);
        assert!(world(&link).writes.borrow().is_empty());
    }

    #[test]
    fn the_interval_starts_again_after_a_new_connection() {
        // Connected at 0 s. Disconnected at 5 s. The tries at 5 s and 6 s
        // find nothing. The try at 7 s connects.
        let mut link = connected_link(vec![None, None, Some(FakeTransport::new())]);
        run_until(&mut link, secs(5));
        disconnect(&mut link);

        let got = run_until(&mut link, secs(30));
        assert_eq!(
            got,
            vec![
                (secs(5), Some(LinkEvent::Disconnected)),
                (secs(7), Some(LinkEvent::Connected)),
            ]
        );
        assert_eq!(world(&link).keep_alive_times(), vec![secs(17), secs(27)]);
    }

    #[test]
    fn a_keep_alive_that_fails_with_disconnected_gives_one_disconnected_event() {
        let mut link = connected_link(vec![]);
        run_until(&mut link, secs(10));
        // The device goes away just before the keep-alive. The keep-alive is
        // the first call that sees it.
        disconnect(&mut link);

        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Disconnected);
        assert_eq!(world(&link).keep_alive_times(), vec![secs(10)]);
        assert!(!link.is_connected());
        let later = run_until(&mut link, secs(60));
        assert!(later.is_empty(), "one Disconnected only: {later:?}");
        assert_eq!(world(&link).keep_alive_times(), vec![secs(10)]);
    }

    #[test]
    fn a_keep_alive_error_that_stays_counts_toward_the_limit() {
        let mut link = connected_link(vec![Some(FakeTransport::new())]);
        run_until(&mut link, secs(10));
        link.with_device(|d| {
            d.transport_mut().fail_writes("broken handle");
            Ok(())
        })
        .unwrap();

        let got = events(&mut link, ERROR_LIMIT as usize + 1);
        let errors = got.iter().filter(|e| e.is_none()).count();
        assert_eq!(errors, ERROR_LIMIT as usize - 1);
        assert_eq!(got[ERROR_LIMIT as usize - 1], Some(LinkEvent::Disconnected));
        // The device is still present, so it opens again at once.
        assert_eq!(got[ERROR_LIMIT as usize], Some(LinkEvent::Connected));
        // Each failed try paused, so the tries did not use the processor.
        assert_eq!(world(&link).keep_alive_times().len(), ERROR_LIMIT as usize);
        assert!(link.clock().sleeps.iter().all(|&s| s == ERROR_PAUSE));
    }

    #[test]
    fn a_failed_keep_alive_retries_before_the_next_interval() {
        let mut link = connected_link(vec![]);
        run_until(&mut link, secs(10));
        link.with_device(|d| {
            d.transport_mut().fail_writes("temporary error");
            Ok(())
        })
        .unwrap();
        assert!(matches!(
            link.next_event(Duration::ZERO),
            Err(Error::Transport(_))
        ));
        assert_eq!(world(&link).elapsed(), secs(10) + ERROR_PAUSE);
        link.device.as_mut().unwrap().transport_mut().clear_errors();
        assert_eq!(link.next_event(Duration::ZERO).unwrap(), LinkEvent::Timeout);
        let retried_at = secs(10) + ERROR_PAUSE;
        assert_eq!(world(&link).keep_alive_times(), vec![secs(10), retried_at]);
        assert!(link.is_connected());

        assert_eq!(link.next_event(secs(10)).unwrap(), LinkEvent::Timeout);
        assert_eq!(world(&link).keep_alive_times().len(), 2);
        assert_eq!(link.next_event(Duration::ZERO).unwrap(), LinkEvent::Timeout);
        assert_eq!(
            world(&link).keep_alive_times(),
            vec![secs(10), retried_at, retried_at + secs(10)]
        );
    }

    #[test]
    fn repeated_write_errors_disconnect_once_and_a_success_resets_the_count() {
        let mut link = connected_link(vec![]);
        for cycle in 0..2 {
            link.device
                .as_mut()
                .unwrap()
                .transport_mut()
                .fail_writes("write failed");
            for _ in 0..ERROR_LIMIT - 1 {
                assert!(matches!(
                    link.with_device(|d| d.commit()),
                    Err(Error::Transport(_))
                ));
                assert!(link.is_connected());
            }
            if cycle == 0 {
                link.device.as_mut().unwrap().transport_mut().clear_errors();
                link.with_device(|d| d.commit()).unwrap();
            }
        }
        assert!(matches!(
            link.with_device(|d| d.commit()),
            Err(Error::Transport(_))
        ));
        assert!(!link.is_connected());
        let mut called = false;
        assert!(matches!(
            link.with_device(|_| {
                called = true;
                Ok(())
            }),
            Err(Error::Disconnected)
        ));
        assert!(!called, "a closure must not run without a device");
        assert_eq!(
            link.next_event(Duration::ZERO).unwrap(),
            LinkEvent::Disconnected
        );
        assert_eq!(link.next_event(Duration::ZERO).unwrap(), LinkEvent::Timeout);
    }

    /// Simulate a sleep of the computer for `duration`.
    fn sleep_computer(link: &TestLink, duration: Duration) {
        let world = world(link);
        world.asleep.set(world.asleep.get() + duration);
    }

    #[test]
    fn no_sleep_gives_no_wake() {
        let mut link = connected_link(vec![]);
        let got = run_until(&mut link, secs(120));
        assert!(got.is_empty(), "{got:?}");
        assert_eq!(world(&link).keep_alive_times().len(), 11);
    }

    #[test]
    fn a_long_sleep_gives_woke_after_a_keep_alive() {
        let mut link = connected_link(vec![]);
        run_until(&mut link, secs(5));
        sleep_computer(&link, secs(70));

        assert_eq!(
            link.next_event(STEP).unwrap(),
            LinkEvent::Woke { slept: secs(70) }
        );
        // The keep-alive went out at once, and the interval starts again.
        assert_eq!(world(&link).keep_alive_times(), vec![secs(5)]);
        assert!(link.is_connected());
        let got = run_until(&mut link, secs(30));
        assert!(got.is_empty(), "one Woke only: {got:?}");
        assert_eq!(
            world(&link).keep_alive_times(),
            vec![secs(5), secs(15), secs(25)]
        );
    }

    #[test]
    fn a_short_sleep_gives_no_wake_but_moves_the_keep_alive() {
        let mut link = connected_link(vec![]);
        run_until(&mut link, secs(5));
        // Two sleeps of 1.5 seconds: the device counted 3 seconds more than
        // `Instant`. The keep-alive that was due at 10 s must go out 3
        // seconds earlier.
        let short = Duration::from_millis(1500);
        assert!(short < WAKE_THRESHOLD);
        sleep_computer(&link, short);
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Timeout);
        sleep_computer(&link, short);
        let got = run_until(&mut link, secs(15));
        assert!(got.is_empty(), "{got:?}");
        assert_eq!(world(&link).keep_alive_times(), vec![secs(7)]);
    }

    #[test]
    fn a_wake_with_a_device_that_is_gone_gives_woke_then_disconnected() {
        let mut link = connected_link(vec![Some(FakeTransport::new())]);
        run_until(&mut link, secs(5));
        disconnect(&mut link);
        sleep_computer(&link, secs(600));

        assert_eq!(
            link.next_event(STEP).unwrap(),
            LinkEvent::Woke { slept: secs(600) }
        );
        assert!(!link.is_connected());
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Disconnected);
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Connected);
    }

    #[test]
    fn a_wake_with_no_device_tries_to_connect_at_once() {
        let mut link = link(vec![None, Some(FakeTransport::new())]);
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Timeout);
        assert_eq!(link.connector().tries, 1);
        sleep_computer(&link, secs(60));

        assert_eq!(
            link.next_event(STEP).unwrap(),
            LinkEvent::Woke { slept: secs(60) }
        );
        assert!(world(&link).writes.borrow().is_empty());
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Connected);
        assert_eq!(link.connector().tries, 2, "no wait for the retry interval");
    }

    #[test]
    fn the_sleep_time_of_the_computer_does_not_change_while_it_is_awake() {
        let has_clock = cfg!(any(target_vendor = "apple", target_os = "linux"));
        let Some(first) = computer_sleep_time() else {
            // A platform with no such clock: the link finds no wake.
            assert!(!has_clock);
            return;
        };
        std::thread::sleep(Duration::from_millis(20));
        let second = computer_sleep_time().unwrap();
        let change = second.abs_diff(first);
        assert!(change < Duration::from_millis(5), "{change:?}");
        assert!(SystemClock.asleep().is_some());
    }

    /// The full report of `payload`, as the device gets it.
    fn report(payload: Vec<u8>) -> Vec<u8> {
        let mut report = payload;
        report.resize(1 + DESCRIPTORS[0].packet_size, 0);
        report
    }

    #[test]
    fn stop_sends_dc_then_han_and_then_nothing() {
        let mut link = connected_link(vec![Some(FakeTransport::new())]);
        run_until(&mut link, secs(12));
        assert_eq!(world(&link).keep_alive_times(), vec![secs(10)]);

        assert!(link.stop().unwrap());
        assert!(!link.is_connected());
        assert!(link.is_stopped());

        // The keep-alive is due at 20 s. The link sends it no more, and it
        // does not connect to the device that is present.
        let got = run_until(&mut link, secs(60));
        assert!(got.is_empty(), "{got:?}");
        assert_eq!(link.connector().tries, 1);
        let writes: Vec<Vec<u8>> = world(&link)
            .writes
            .borrow()
            .iter()
            .map(|(_, r)| r.clone())
            .collect();
        assert_eq!(
            writes,
            vec![
                report(protocol::keep_alive()),
                report(protocol::disconnected()),
                report(protocol::sleep()),
            ]
        );
        // The two commands of the stop go out with no pause between them.
        let times: Vec<Duration> = world(&link).writes.borrow().iter().map(|w| w.0).collect();
        assert_eq!(times[1], times[2]);
        assert!(matches!(
            link.with_device(|d| d.keep_alive()),
            Err(Error::Disconnected)
        ));
        assert!(!link.stop().unwrap(), "a second stop sends nothing");
        assert_eq!(world(&link).writes.borrow().len(), 3);
    }

    #[test]
    fn stop_with_no_device_sends_nothing() {
        let mut link = link(vec![]);
        assert_eq!(link.next_event(STEP).unwrap(), LinkEvent::Timeout);
        assert!(!link.stop().unwrap());
        assert!(world(&link).writes.borrow().is_empty());

        // The device is gone, and the link did not see it yet.
        let mut link = connected_link(vec![]);
        disconnect(&mut link);
        assert!(matches!(link.stop(), Err(Error::Disconnected)));
        assert!(!link.is_connected());
        assert_eq!(world(&link).writes.borrow().len(), 1, "one try of DC");
        let got = run_until(&mut link, secs(30));
        assert!(got.is_empty(), "no Disconnected after the stop: {got:?}");
    }

    #[test]
    fn stop_does_not_wait_or_retry_when_writes_fail() {
        let mut link = connected_link(vec![]);
        link.with_device(|d| {
            d.transport_mut().fail_writes("broken handle");
            Ok(())
        })
        .unwrap();
        let before = world(&link).elapsed();
        assert!(matches!(link.stop(), Err(Error::Transport(_))));
        assert_eq!(world(&link).elapsed(), before, "no pause");
        assert_eq!(world(&link).writes.borrow().len(), 1, "no HAN after DC");
        assert!(link.clock().sleeps.is_empty());
        assert!(!link.is_connected());
    }

    #[test]
    fn the_keep_alive_interval_can_change_or_stop() {
        let mut link = connected_link(vec![]);
        link.set_keep_alive_interval(Some(secs(3)));
        run_until(&mut link, secs(10));
        assert_eq!(
            world(&link).keep_alive_times(),
            vec![secs(3), secs(6), secs(9)]
        );

        link.set_keep_alive_interval(None);
        run_until(&mut link, secs(60));
        assert_eq!(world(&link).keep_alive_times().len(), 3);

        // Zero is the same as no keep-alive. It does not make a busy loop.
        link.set_keep_alive_interval(Some(Duration::ZERO));
        assert_eq!(link.next_event(secs(1)).unwrap(), LinkEvent::Timeout);
        assert_eq!(world(&link).keep_alive_times().len(), 3);
    }
}
