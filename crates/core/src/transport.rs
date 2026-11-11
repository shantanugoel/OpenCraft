//! Raw device I/O behind one interface.
//!
//! [`Transport`] moves raw reports to and from a device. It knows nothing about
//! commands: the command logic is in [`crate::connection::Device`], which works
//! with any transport. This is what lets the code above the transport run in a
//! test with no hardware (architecture rule 7).
//!
//! - [`HidTransport`] is the real implementation. It is the only code that uses
//!   `hidapi` for I/O. It opens only the vendor interface of a known device
//!   (architecture rule 6). [`select`] chooses that interface from a list.
//! - [`FakeTransport`] is an in-memory implementation for tests.

mod fake;
mod hid;

pub use fake::FakeTransport;
pub use hid::{list_devices, scan, select, Found, HidTransport};

use crate::error::Result;
use std::time::Duration;

/// Blocking raw I/O for one open device.
///
/// All calls block the calling thread. The caller supplies the threads
/// (decision 0003). The trait is object-safe, so `Box<dyn Transport + Send>`
/// is a transport too. A `&mut` reference to a transport is also a transport:
/// a test can lend a [`FakeTransport`] to a device and examine it after.
///
/// When the device is gone, a call gives [`crate::Error::Disconnected`], and
/// all later calls fail. Any other error can go away. [`crate::link::Link`]
/// uses this difference to connect again.
pub trait Transport {
    /// Write one output report. `report[0]` is the report ID.
    fn write(&mut self, report: &[u8]) -> Result<()>;

    /// Wait up to `timeout` for one input report and copy it into `buf`.
    ///
    /// Returns `Ok(None)` when the time limit expires with no report, and
    /// `Ok(Some(n))` when `n` bytes of a report are in `buf`.
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<Option<usize>>;

    /// Get one feature report. Put the report ID in `buf[0]` before the call.
    ///
    /// Returns the number of bytes that the device put in `buf`.
    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize>;
}

impl<T: Transport + ?Sized> Transport for &mut T {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        (**self).write(report)
    }

    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<Option<usize>> {
        (**self).read(buf, timeout)
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).get_feature_report(buf)
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        (**self).write(report)
    }

    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<Option<usize>> {
        (**self).read(buf, timeout)
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).get_feature_report(buf)
    }
}
