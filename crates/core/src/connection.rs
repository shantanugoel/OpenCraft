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
    ///
    /// This opens only the vendor interface of a known device (architecture
    /// rule 6). For any other entry, the function returns an error before it
    /// opens anything: [`Error::NoVendorInterface`] for a known device, and
    /// [`Error::DeviceNotFound`] for an unknown device. To choose the entry,
    /// use [`crate::transport::select`].
    pub fn open(found: &Found) -> Result<Self> {
        let descriptor = found.openable()?;
        let transport = HidTransport::open(found)?;
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

    /// Send an image to the surface with the slot `slot`: the header, then
    /// the data in parts. Send [`Self::commit`] after the images.
    ///
    /// The image must be ready for the surface: see
    /// [`crate::image::encode_for`]. A slot that is not in the descriptor
    /// gives [`Error::NoSuchSlot`], and the method sends nothing.
    pub fn send_image(&mut self, slot: u8, image: &[u8]) -> Result<()> {
        if self.descriptor.surface_at_slot(slot).is_none() {
            return Err(Error::NoSuchSlot(slot));
        }
        self.send(protocol::image_header(image_len(image)?, slot))?;
        self.send_parts(image)
    }

    /// Send an image to the background of the device. Send [`Self::commit`]
    /// after it, as for the key faces.
    ///
    /// The image must be ready for the background surface of the descriptor
    /// (see [`crate::image::render_background_test`]). The method puts the
    /// image at 0, 0 on frame buffer layer 0, as the SDK does for a full
    /// background. The SDK calls this image a temporary background, and sends
    /// one for each frame of an animation. This is not the boot logo.
    pub fn send_background(&mut self, image: &[u8]) -> Result<()> {
        let bg = self.descriptor.background().ok_or(Error::NoBackground)?;
        self.send(protocol::background_header(
            image_len(image)?,
            0,
            0,
            bg.width,
            bg.height,
            0,
        ))?;
        self.send_parts(image)
    }

    /// Remove the background image. The SDK uses layer `0x03` for this. Send
    /// [`Self::commit`] after it.
    pub fn clear_background(&mut self) -> Result<()> {
        if self.descriptor.background().is_none() {
            return Err(Error::NoBackground);
        }
        self.send(protocol::clear_background(0x03))
    }

    /// Send image data in parts of one report payload each. Each part has the
    /// report ID in front.
    fn send_parts(&mut self, data: &[u8]) -> Result<()> {
        let chunk = self.descriptor.packet_size;
        for part in data.chunks(chunk) {
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

/// The length of image data for the 4-byte length field of an image header.
fn image_len(data: &[u8]) -> Result<u32> {
    u32::try_from(data.len()).map_err(|_| Error::ImageTooLarge(data.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{Descriptor, DESCRIPTORS};
    use crate::transport::FakeTransport;
    use std::ffi::CString;

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

    fn entry(usage_page: u16, interface: i32) -> Found {
        Found {
            vid: DESCRIPTORS[0].vid,
            pid: DESCRIPTORS[0].pid,
            product: "Test device".to_string(),
            serial: None,
            usage_page,
            interface,
            known: Some(&DESCRIPTORS[0]),
            // This path does not exist. A call to `hidapi` would fail with a
            // different error.
            path: CString::new("opencraft-test-path-that-does-not-exist").unwrap(),
        }
    }

    #[test]
    fn open_refuses_a_keyboard_interface_before_it_opens_anything() {
        for page in [0x0001, 0x000C] {
            assert!(matches!(
                Device::open(&entry(page, 1)),
                Err(Error::NoVendorInterface)
            ));
        }
    }

    #[test]
    fn open_refuses_an_unknown_device() {
        let mut found = entry(0xFFA0, 0);
        found.known = None;
        assert!(matches!(Device::open(&found), Err(Error::DeviceNotFound)));
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

    /// Send `image` to slot 6 and return the writes: the header and the parts.
    fn image_writes(image: &[u8]) -> Vec<Vec<u8>> {
        let mut dev = fake_device();
        dev.send_image(6, image).unwrap();
        dev.into_transport().take_writes()
    }

    #[test]
    fn an_image_of_an_exact_multiple_of_the_part_size_has_no_empty_part() {
        let image: Vec<u8> = (0..2048u32).map(|i| (i % 251) as u8 + 1).collect();
        let writes = image_writes(&image);

        assert_eq!(writes.len(), 3, "header and 2 parts only");
        assert_report(
            &writes[0],
            &[
                b'C', b'R', b'T', 0, 0, b'B', b'A', b'T', 0, 0, 0x08, 0x00, 6,
            ],
        );
        assert_report(&writes[1], &image[..1024]);
        assert_report(&writes[2], &image[1024..]);
        // The last part is full: 1024 data bytes and no padding.
        assert!(writes[2][1..].iter().all(|&b| b != 0));
    }

    #[test]
    fn an_image_of_one_part_size_is_one_part() {
        let image = vec![0xAB; 1024];
        let writes = image_writes(&image);

        assert_eq!(writes.len(), 2, "header and 1 part");
        assert_report(&writes[1], &image);
    }

    #[test]
    fn one_byte_over_a_part_boundary_makes_a_short_second_part() {
        let image = vec![0xCD; 1025];
        let writes = image_writes(&image);

        assert_eq!(writes.len(), 3, "header and 2 parts");
        assert_report(&writes[1], &image[..1024]);
        assert_report(&writes[2], &[0xCD]);
    }

    #[test]
    fn an_image_smaller_than_one_part_is_one_padded_part() {
        let writes = image_writes(&[1, 2, 3]);

        assert_eq!(writes.len(), 2, "header and 1 part");
        assert_report(&writes[0], b"CRT\x00\x00BAT\x00\x00\x00\x03\x06");
        assert_report(&writes[1], &[1, 2, 3]);
    }

    #[test]
    fn an_empty_image_sends_the_header_and_no_part() {
        // A header that announces zero bytes, and no data part. No caller has
        // a reason to send this, but the transfer must not write an empty part.
        let writes = image_writes(&[]);

        assert_eq!(writes.len(), 1, "header only");
        assert_report(&writes[0], b"CRT\x00\x00BAT\x00\x00\x00\x00\x06");
    }

    #[test]
    fn each_part_of_an_image_is_a_full_report_and_the_parts_join_to_the_image() {
        for len in [1usize, 1023, 1024, 1025, 3000, 4096, 5121] {
            let image: Vec<u8> = (0..len).map(|i| (i % 255) as u8 + 1).collect();
            let writes = image_writes(&image);

            assert_eq!(writes.len(), 1 + len.div_ceil(1024), "length {len}");
            let mut joined = Vec::new();
            for part in &writes[1..] {
                assert_eq!(part.len(), REPORT_LEN, "length {len}");
                assert_eq!(part[0], 0x00, "report ID, length {len}");
                joined.extend_from_slice(&part[1..]);
            }
            // The data bytes are never zero, so the padding is the zero tail.
            let end = joined.iter().rposition(|&b| b != 0).unwrap() + 1;
            assert_eq!(&joined[..end], &image[..], "length {len}");
        }
    }

    #[test]
    fn the_header_has_the_slot_of_the_call() {
        let mut dev = fake_device();
        dev.send_image(11, &[1]).unwrap();
        dev.send_image(1, &[1]).unwrap();
        let writes = dev.into_transport().take_writes();

        assert_eq!(writes[0][13], 11);
        assert_eq!(writes[2][13], 1);
    }

    #[test]
    fn clear_all_sends_the_clear_command_for_all_slots_and_a_commit() {
        let mut dev = fake_device();
        dev.clear_all().unwrap();
        let writes = dev.into_transport().take_writes();

        assert_eq!(writes.len(), 2);
        assert_report(&writes[0], b"CRT\x00\x00CLE\x00\x00\x00\xFF");
        assert_report(&writes[1], b"CRT\x00\x00STP");
    }

    #[test]
    fn short_commands_are_padded_to_a_full_report() {
        let mut dev = fake_device();
        dev.init().unwrap();
        dev.set_brightness(80).unwrap();
        dev.keep_alive().unwrap();
        let writes = dev.into_transport().take_writes();

        assert_eq!(writes.len(), 3);
        assert_report(&writes[0], b"CRT\x00\x00DIS");
        assert_report(&writes[1], b"CRT\x00\x00LIG\x00\x00\x50");
        assert_report(&writes[2], b"CRT\x00\x00CONNECT");
    }

    /// Image data of `len` bytes with no zero byte, so that the padding is
    /// clear in each part.
    fn image_data(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8 + 1).collect()
    }

    /// Check that `parts` are the reports for `data` in parts of 1024 bytes.
    fn assert_parts(parts: &[Vec<u8>], data: &[u8]) {
        assert_eq!(parts.len(), data.len().div_ceil(1024), "number of parts");
        for (report, part) in parts.iter().zip(data.chunks(1024)) {
            assert_report(report, part);
        }
    }

    #[test]
    fn send_image_larger_than_65535_bytes_has_the_full_length() {
        // 70 000 = 0x0001_1170 = 68 parts of 1024 bytes and one part of 368.
        let image = image_data(70_000);
        let mut dev = fake_device();

        dev.send_image(6, &image).unwrap();

        let writes = dev.into_transport().take_writes();
        assert_eq!(writes.len(), 1 + 69, "header and 69 parts");
        let header = [
            b'C', b'R', b'T', 0x00, 0x00, b'B', b'A', b'T', 0x00, 0x01, 0x11, 0x70, 6,
        ];
        assert_report(&writes[0], &header);
        assert_parts(&writes[1..], &image);
        assert_eq!(image.len() - 68 * 1024, 368);
    }

    #[test]
    fn send_background_writes_header_parts_and_commit() {
        // A real JPEG of 800 x 480 can be larger than 65 535 bytes.
        let image = image_data(70_000);
        let mut dev = fake_device();

        dev.send_background(&image).unwrap();
        dev.commit().unwrap();

        let writes = dev.into_transport().take_writes();
        assert_eq!(writes.len(), 1 + 69 + 1, "header, 69 parts, commit");

        // "BGPIC", length 0x0001_1170, x 0, y 0, width 800 (0x0320),
        // height 480 (0x01E0), 0x00, layer 0.
        let header = [
            b'C', b'R', b'T', 0x00, 0x00, b'B', b'G', b'P', b'I', b'C', 0x00, 0x01, 0x11, 0x70,
            0x00, 0x00, 0x00, 0x00, 0x03, 0x20, 0x01, 0xE0, 0x00, 0x00,
        ];
        assert_report(&writes[0], &header);
        assert_parts(&writes[1..70], &image);
        assert_report(&writes[70], b"CRT\x00\x00STP");
    }

    #[test]
    fn background_needs_a_background_in_the_descriptor() {
        static NO_BACKGROUND: Descriptor = Descriptor {
            surfaces: &[],
            ..DESCRIPTORS[0]
        };
        let mut fake = FakeTransport::new();
        let mut dev = Device::new(&mut fake, &NO_BACKGROUND);

        assert!(matches!(
            dev.send_background(&[1, 2, 3]),
            Err(Error::NoBackground)
        ));
        assert!(matches!(dev.clear_background(), Err(Error::NoBackground)));
        assert!(fake.writes().is_empty());
    }

    #[test]
    fn clear_background_writes_one_report() {
        let mut dev = fake_device();
        dev.clear_background().unwrap();
        let writes = dev.into_transport().take_writes();
        assert_eq!(writes.len(), 1);
        assert_report(&writes[0], b"CRT\x00\x00BGCLE\x03");
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
    fn send_image_refuses_a_slot_that_is_not_in_the_descriptor() {
        let mut fake = FakeTransport::new();
        let mut dev = Device::new(&mut fake, &DESCRIPTORS[0]);
        // The N4 Pro does not use slot 5.
        for slot in [0, 5, 16, 0xFF] {
            assert!(
                matches!(dev.send_image(slot, &[1, 2, 3]), Err(Error::NoSuchSlot(s)) if s == slot),
                "slot {slot}"
            );
        }
        assert!(fake.writes().is_empty());
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
