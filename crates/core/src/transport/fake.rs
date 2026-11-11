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
/// - On request it fails each read or each write with [`Error::Transport`], to
///   simulate a device that is gone.
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

    /// Stop the failures that [`Self::fail_reads`] and [`Self::fail_writes`]
    /// started.
    pub fn clear_errors(&mut self) {
        self.read_error = None;
        self.write_error = None;
    }
}

impl Transport for FakeTransport {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        if let Some(message) = &self.write_error {
            return Err(Error::Transport(message.clone()));
        }
        self.writes.push(report.to_vec());
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<Option<usize>> {
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
}
