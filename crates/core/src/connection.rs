//! An open device: the command logic above the transport.

use crate::device::Descriptor;
use crate::error::{Error, Result};
use crate::protocol;
use crate::transport::{Found, HidTransport, Transport};
use std::time::Duration;

/// One connected controller.
///
/// `Device` owns a [`Transport`] and the descriptor of the model. It turns
/// commands into reports. It works with any transport, so tests run the real
/// command logic on a [`crate::transport::FakeTransport`].
pub struct Device<T> {
    transport: T,
    descriptor: &'static Descriptor,
    read_buf: Vec<u8>,
}

impl Device<HidTransport> {
    /// Open a device that the enumeration found.
    pub fn open(found: &Found) -> Result<Self> {
        let descriptor = found.known.ok_or(Error::DeviceNotFound)?;
        let transport = HidTransport::open(&found.path)?;
        Ok(Self::new(transport, descriptor))
    }
}

impl<T: Transport> Device<T> {
    /// Use `transport` to operate a device of the model `descriptor`.
    pub fn new(transport: T, descriptor: &'static Descriptor) -> Self {
        Self {
            transport,
            descriptor,
            read_buf: vec![0u8; descriptor.packet_size],
        }
    }

    pub fn descriptor(&self) -> &'static Descriptor {
        self.descriptor
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    /// Write one command, padded to the device's report size.
    pub fn send(&mut self, mut payload: Vec<u8>) -> Result<()> {
        payload.resize(1 + self.descriptor.packet_size, 0);
        self.transport.write(&payload)
    }

    /// Send an image to a slot: header packet, then payload chunks.
    pub fn send_image(&mut self, slot: u8, png: &[u8]) -> Result<()> {
        self.send(protocol::image_header(png.len(), slot))?;

        let chunk = self.descriptor.packet_size;
        for part in png.chunks(chunk) {
            let mut buf = Vec::with_capacity(1 + part.len());
            buf.push(protocol::REPORT_ID);
            buf.extend_from_slice(part);
            self.send(buf)?;
        }
        Ok(())
    }

    /// Block up to `timeout` for one input report.
    pub fn read(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        let n = self.transport.read(&mut self.read_buf, timeout)?;
        Ok(n.map(|n| self.read_buf[..n].to_vec()))
    }

    /// Firmware version, read via feature report 0x01.
    pub fn firmware_version(&mut self) -> Result<String> {
        let mut buf = [0u8; 20];
        buf[0] = 0x01;
        let n = self.transport.get_feature_report(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf[..n])
            .trim_end_matches('\0')
            .to_string())
    }

    pub fn set_brightness(&mut self, percent: u8) -> Result<()> {
        self.send(protocol::set_brightness(percent))
    }

    pub fn init(&mut self) -> Result<()> {
        self.send(protocol::init())
    }

    pub fn commit(&mut self) -> Result<()> {
        self.send(protocol::commit())
    }

    pub fn clear_all(&mut self) -> Result<()> {
        self.send(protocol::clear_slot(0xFF))?;
        self.commit()
    }

    pub fn keep_alive(&mut self) -> Result<()> {
        self.send(protocol::keep_alive())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DESCRIPTORS;
    use crate::transport::FakeTransport;

    const REPORT_LEN: usize = 1025;

    fn fake_device() -> Device<FakeTransport> {
        Device::new(FakeTransport::new(), &DESCRIPTORS[0])
    }

    /// Check that `report` is one full output report: the report ID, then
    /// `start`, then zero bytes.
    fn assert_report(report: &[u8], start: &[u8]) {
        assert_eq!(report.len(), REPORT_LEN);
        assert_eq!(report[0], 0x00, "report ID");
        assert_eq!(&report[1..1 + start.len()], start);
        assert!(
            report[1 + start.len()..].iter().all(|&b| b == 0),
            "padding is not zero"
        );
    }

    #[test]
    fn send_image_writes_header_parts_and_commit() {
        // 2500 = 1024 + 1024 + 452, so the last part is short before padding.
        let image: Vec<u8> = (0..2500u32).map(|i| (i % 251) as u8 + 1).collect();
        let mut dev = fake_device();

        dev.send_image(7, &image).unwrap();
        dev.commit().unwrap();

        let writes = dev.into_transport().take_writes();
        assert_eq!(writes.len(), 5, "header, 3 parts, commit");

        // Header: "CRT" 0x00 0x00 "BAT" 0x00 0x00, length (big-endian), slot.
        let header = [
            b'C', b'R', b'T', 0x00, 0x00, b'B', b'A', b'T', 0x00, 0x00, 0x09, 0xC4, 7,
        ];
        assert_report(&writes[0], &header);

        assert_report(&writes[1], &image[..1024]);
        assert_report(&writes[2], &image[1024..2048]);
        assert_report(&writes[3], &image[2048..]);
        assert_eq!(image[2048..].len(), 452);

        assert_report(&writes[4], b"CRT\x00\x00STP");
    }

    #[test]
    fn read_error_reaches_the_caller() {
        let mut fake = FakeTransport::new();
        fake.push_input(b"ACK".to_vec());
        let mut dev = Device::new(&mut fake, &DESCRIPTORS[0]);
        let timeout = Duration::from_millis(10);

        assert_eq!(dev.read(timeout).unwrap(), Some(b"ACK".to_vec()));

        dev.transport_mut().fail_reads("device unplugged");
        match dev.read(timeout) {
            Err(Error::Transport(message)) => assert_eq!(message, "device unplugged"),
            other => panic!("expected a transport error, got {other:?}"),
        }
    }

    #[test]
    fn write_error_reaches_the_caller() {
        let mut fake = FakeTransport::new();
        fake.fail_writes("device unplugged");
        let mut dev = Device::new(&mut fake, &DESCRIPTORS[0]);

        assert!(matches!(
            dev.send_image(7, &[1, 2, 3]),
            Err(Error::Transport(_))
        ));
        assert!(matches!(dev.commit(), Err(Error::Transport(_))));
        assert!(fake.writes().is_empty());
    }
}
