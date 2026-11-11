//! HID transport.

use crate::device::{Descriptor, VENDOR_USAGE_PAGES};
use crate::error::{Error, Result};
use crate::protocol;
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::time::Duration;

/// A device we found on the bus.
#[derive(Debug, Clone)]
pub struct Found {
    pub vid: u16,
    pub pid: u16,
    pub product: String,
    pub serial: Option<String>,
    pub usage_page: u16,
    pub interface: i32,
    pub known: Option<&'static Descriptor>,
    pub path: std::ffi::CString,
}

impl Found {
    pub fn is_vendor_interface(&self) -> bool {
        VENDOR_USAGE_PAGES.contains(&self.usage_page)
    }

    pub fn label(&self) -> String {
        match self.known {
            Some(d) => d.name.to_string(),
            None => self.product.clone(),
        }
    }
}

/// Enumerate every HID device, flagging the ones we know about.
pub fn scan(api: &HidApi) -> Vec<Found> {
    api.device_list()
        .map(|info: &DeviceInfo| {
            let vid = info.vendor_id();
            let pid = info.product_id();
            Found {
                vid,
                pid,
                product: info.product_string().unwrap_or("").to_string(),
                serial: info.serial_number().map(|s| s.to_string()),
                usage_page: info.usage_page(),
                interface: info.interface_number(),
                known: crate::device::find(vid, pid),
                path: info.path().to_owned(),
            }
        })
        .collect()
}

/// Enumerate every HID device. Convenience wrapper that owns the `HidApi`
/// handle so callers never need to depend on hidapi directly.
pub fn list_devices() -> Result<Vec<Found>> {
    let api = HidApi::new()?;
    Ok(scan(&api))
}

/// An open device connection.
pub struct Transport {
    // Kept alive alongside the handle; libhidapi state is process-global.
    _api: HidApi,
    dev: HidDevice,
    pub descriptor: &'static Descriptor,
    pub read_buf: Vec<u8>,
}

impl Transport {
    pub fn open(found: &Found) -> Result<Self> {
        let descriptor = found.known.ok_or(Error::DeviceNotFound)?;
        let api = HidApi::new()?;
        let dev = api.open_path(&found.path).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not permitted") || msg.contains("0xE00002E2") {
                return Error::PermissionDenied;
            }
            Error::Hid(e)
        })?;
        Ok(Self {
            _api: api,
            dev,
            descriptor,
            read_buf: vec![0u8; descriptor.packet_size],
        })
    }

    /// Write one command, padded to the device's report size.
    pub fn send(&self, mut payload: Vec<u8>) -> Result<()> {
        payload.resize(1 + self.descriptor.packet_size, 0);
        self.dev.write(&payload)?;
        Ok(())
    }

    /// Send an image to a slot: header packet, then payload chunks.
    pub fn send_image(&self, slot: u8, png: &[u8]) -> Result<()> {
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
        // Ok(0) means the timeout elapsed with no report available.
        match self
            .dev
            .read_timeout(&mut self.read_buf, timeout.as_millis() as i32)
        {
            Ok(0) => Ok(None),
            Ok(n) => Ok(Some(self.read_buf[..n].to_vec())),
            Err(e) => Err(e.into()),
        }
    }

    /// Firmware version, read via feature report 0x01.
    pub fn firmware_version(&self) -> Result<String> {
        let mut buf = [0u8; 20];
        buf[0] = 0x01;
        let n = self.dev.get_feature_report(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf[..n])
            .trim_end_matches('\0')
            .to_string())
    }

    pub fn set_brightness(&self, percent: u8) -> Result<()> {
        self.send(protocol::set_brightness(percent))
    }

    pub fn init(&self) -> Result<()> {
        self.send(protocol::init())
    }

    pub fn commit(&self) -> Result<()> {
        self.send(protocol::commit())
    }

    pub fn clear_all(&self) -> Result<()> {
        self.send(protocol::clear_slot(0xFF))?;
        self.commit()
    }

    pub fn keep_alive(&self) -> Result<()> {
        self.send(protocol::keep_alive())
    }
}
