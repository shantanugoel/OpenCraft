//! The `hidapi` transport, and device enumeration.
//!
//! This is the only module that uses `hidapi` for I/O.

use super::Transport;
use crate::device::{Descriptor, VENDOR_USAGE_PAGES};
use crate::error::{Error, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::ffi::CStr;
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

/// One open HID interface.
///
/// The `hidapi` library state is process-global and is never released, so this
/// type holds only the device handle. It is `Send`: the engine can move it to
/// an I/O thread (architecture rule 5).
pub struct HidTransport {
    dev: HidDevice,
}

impl HidTransport {
    /// Open the HID interface at `path`.
    pub fn open(path: &CStr) -> Result<Self> {
        let api = HidApi::new()?;
        let dev = api.open_path(path).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not permitted") || msg.contains("0xE00002E2") {
                return Error::PermissionDenied;
            }
            Error::Hid(e)
        })?;
        Ok(Self { dev })
    }
}

impl Transport for HidTransport {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        self.dev.write(report)?;
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<Option<usize>> {
        // Ok(0) means the timeout elapsed with no report available.
        match self.dev.read_timeout(buf, timeout.as_millis() as i32)? {
            0 => Ok(None),
            n => Ok(Some(n)),
        }
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.dev.get_feature_report(buf)?)
    }
}

// Compile-time check: the real transport can move to another thread.
const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<HidTransport>();
};
