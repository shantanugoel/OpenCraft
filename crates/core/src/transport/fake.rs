//! An in-memory transport for tests.

use super::Transport;
use crate::error::{Error, Result};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

/// A transport with no device behind it.
///
/// - It keeps a record of each output report that the caller writes.
/// - It returns the input reports that the test queues, in order. When the
///   queue is empty, a read returns `Ok(None)` at once, as if the time limit
///   expired. The fake does not wait.
/// - It returns the feature reports that the test sets.
/// - On request it fails each read or each write with [`Error::Transport`], or
///   one read only, to simulate an error that stays or that goes away.
/// - On request it simulates a disconnection: each call gives
///   [`Error::Disconnected`], as [`super::HidTransport`] does when the device
///   is gone.
///
/// The fake is part of the public API, so that other crates can use it in
/// their tests.
#[derive(Debug, Default)]
pub struct FakeTransport {
    writes: Vec<Vec<u8>>,
    inputs: VecDeque<Vec<u8>>,
    features: HashMap<u8, Vec<u8>>,
    read_error: Option<String>,
    write_error: Option<String>,
    next_read_error: Option<String>,
    disconnected: bool,
}

impl FakeTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Each output report that a write gave, in order. A failed write is not
    /// in the record.
    pub fn writes(&self) -> &[Vec<u8>] {
        &self.writes
    }

    /// Remove and return the record of writes.
    pub fn take_writes(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.writes)
    }

    /// Queue one input report for a later read.
    pub fn push_input(&mut self, report: impl Into<Vec<u8>>) {
        self.inputs.push_back(report.into());
    }

    /// Set the feature report that `get_feature_report` returns for the report
    /// ID in `report[0]`. Include the report ID byte, as a device does.
    ///
    /// # Panics
    ///
    /// Panics if `report` is empty.
    pub fn set_feature_report(&mut self, report: impl Into<Vec<u8>>) {
        let report = report.into();
        let id = *report.first().expect("a feature report needs a report ID");
        self.features.insert(id, report);
    }

    /// Make each read fail with `message` until [`Self::clear_errors`]. The
    /// queued input reports stay in the queue.
    pub fn fail_reads(&mut self, message: impl Into<String>) {
        self.read_error = Some(message.into());
    }

    /// Make each write fail with `message` until [`Self::clear_errors`].
    pub fn fail_writes(&mut self, message: impl Into<String>) {
        self.write_error = Some(message.into());
    }

    /// Make only the next read fail with `message`. The reads after it work.
    pub fn fail_next_read(&mut self, message: impl Into<String>) {
        self.next_read_error = Some(message.into());
    }

    /// Stop the failures that [`Self::fail_reads`], [`Self::fail_writes`] and
    /// [`Self::fail_next_read`] started. This does not stop a disconnection.
    pub fn clear_errors(&mut self) {
        self.read_error = None;
        self.write_error = None;
        self.next_read_error = None;
    }

    /// Simulate a disconnection. From now on, each read, write and feature
    /// report gives [`Error::Disconnected`]. The fake cannot connect again: a
    /// new connection is a new transport.
    pub fn disconnect(&mut self) {
        self.disconnected = true;
    }
}

impl Transport for FakeTransport {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        if self.disconnected {
            return Err(Error::Disconnected);
        }
        if let Some(message) = &self.write_error {
            return Err(Error::Transport(message.clone()));
        }
        self.writes.push(report.to_vec());
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<Option<usize>> {
        if self.disconnected {
            return Err(Error::Disconnected);
        }
        if let Some(message) = self.next_read_error.take() {
            return Err(Error::Transport(message));
        }
        if let Some(message) = &self.read_error {
            return Err(Error::Transport(message.clone()));
        }
        let Some(report) = self.inputs.pop_front() else {
            return Ok(None);
        };
        // Like a HID read, a report longer than the buffer is cut to fit.
        let n = report.len().min(buf.len());
        buf[..n].copy_from_slice(&report[..n]);
        Ok(Some(n))
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.disconnected {
            return Err(Error::Disconnected);
        }
        let id = *buf
            .first()
            .ok_or_else(|| Error::Transport("the feature report buffer is empty".to_string()))?;
        let report = self
            .features
            .get(&id)
            .ok_or_else(|| Error::Transport(format!("no feature report 0x{id:02X}")))?;
        let n = report.len().min(buf.len());
        buf[..n].copy_from_slice(&report[..n]);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_return_queued_inputs_in_order_then_timeout() {
        let mut fake = FakeTransport::new();
        fake.push_input(vec![1, 2, 3]);
        fake.push_input(vec![4, 5]);

        let mut buf = [0u8; 8];
        let timeout = Duration::from_millis(10);

        assert_eq!(fake.read(&mut buf, timeout).unwrap(), Some(3));
        assert_eq!(&buf[..3], &[1, 2, 3]);
        assert_eq!(fake.read(&mut buf, timeout).unwrap(), Some(2));
        assert_eq!(&buf[..2], &[4, 5]);
        assert_eq!(fake.read(&mut buf, timeout).unwrap(), None);
    }

    #[test]
    fn fail_next_read_fails_one_read_only() {
        let mut fake = FakeTransport::new();
        fake.push_input(vec![1]);
        fake.fail_next_read("hiccup");

        let mut buf = [0u8; 8];
        let timeout = Duration::from_millis(10);

        assert!(matches!(
            fake.read(&mut buf, timeout),
            Err(Error::Transport(_))
        ));
        assert_eq!(fake.read(&mut buf, timeout).unwrap(), Some(1));
    }

    #[test]
    fn a_disconnection_fails_each_call_and_stays() {
        let mut fake = FakeTransport::new();
        fake.push_input(vec![1]);
        fake.set_feature_report(vec![1, 2]);
        fake.disconnect();
        fake.clear_errors();

        let mut buf = [0u8; 8];
        let timeout = Duration::from_millis(10);

        assert!(matches!(
            fake.read(&mut buf, timeout),
            Err(Error::Disconnected)
        ));
        assert!(matches!(fake.write(&[0, 1]), Err(Error::Disconnected)));
        buf[0] = 1;
        assert!(matches!(
            fake.get_feature_report(&mut buf),
            Err(Error::Disconnected)
        ));
        assert!(fake.writes().is_empty());
    }
}
