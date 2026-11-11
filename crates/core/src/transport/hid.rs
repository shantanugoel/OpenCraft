//! The `hidapi` transport, and device enumeration.
//!
//! This is the only module that uses `hidapi` for I/O.

use super::Transport;
use crate::device::{Descriptor, VENDOR_USAGE_PAGES};
use crate::error::{Error, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice, HidError};
use std::ffi::{CStr, CString};
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
    /// True if the usage page of this interface is a vendor usage page.
    pub fn is_vendor_interface(&self) -> bool {
        VENDOR_USAGE_PAGES.contains(&self.usage_page)
    }

    /// Check that this entry is an interface that OpenCraft can open, and
    /// return the descriptor of its model.
    ///
    /// OpenCraft opens only the vendor interface of a known device
    /// (architecture rule 6). This is the one check for that rule. Every code
    /// path that opens an interface calls it first.
    ///
    /// - An unknown device gives [`Error::DeviceNotFound`].
    /// - A known device with an interface that is not a vendor interface gives
    ///   [`Error::NoVendorInterface`].
    pub fn openable(&self) -> Result<&'static Descriptor> {
        let descriptor = self.known.ok_or(Error::DeviceNotFound)?;
        if self.is_vendor_interface() {
            Ok(descriptor)
        } else {
            Err(Error::NoVendorInterface)
        }
    }

    pub fn label(&self) -> String {
        match self.known {
            Some(d) => d.name.to_string(),
            None => self.product.clone(),
        }
    }
}

/// Select the interface to open from a list that [`scan`] or [`list_devices`]
/// gave.
///
/// The function returns the first entry that is a known device AND has a
/// vendor usage page. It uses no `hidapi` call and opens nothing.
///
/// - No known device in the list: [`Error::DeviceNotFound`].
/// - A known device in the list, but none of its interfaces is a vendor
///   interface: [`Error::NoVendorInterface`].
pub fn select(found: &[Found]) -> Result<&Found> {
    if let Some(f) = found.iter().find(|f| f.openable().is_ok()) {
        return Ok(f);
    }
    if found.iter().any(|f| f.known.is_some()) {
        Err(Error::NoVendorInterface)
    } else {
        Err(Error::DeviceNotFound)
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
/// type holds only the device handle and its path. It is `Send`: the engine can
/// move it to an I/O thread (architecture rule 5).
///
/// # Disconnection
///
/// When a call fails, the transport enumerates the devices again. If the path
/// of this interface is not in the list, the call gives
/// [`Error::Disconnected`]. Otherwise it gives [`Error::Hid`]. The error text
/// of `hidapi` is not the signal. See `docs/kb/hidapi-macos.md`.
pub struct HidTransport {
    dev: HidDevice,
    path: CString,
}

impl HidTransport {
    /// Open the HID interface of `found`.
    ///
    /// This function is private to the crate. It refuses an entry that is not
    /// the vendor interface of a known device, before any `hidapi` call.
    pub(crate) fn open(found: &Found) -> Result<Self> {
        found.openable()?;
        let api = HidApi::new()?;
        let dev = api.open_path(&found.path).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not permitted") || msg.contains("0xE00002E2") {
                return Error::PermissionDenied;
            }
            Error::Hid(e)
        })?;
        Ok(Self {
            dev,
            path: found.path.clone(),
        })
    }

    /// Turn a failed `hidapi` call into an error. This enumerates the devices
    /// one time (a few milliseconds), and only after a failure.
    fn classify(&self, error: HidError) -> Error {
        classify(error, &self.path, list_devices())
    }
}

/// Give [`Error::Disconnected`] if the enumeration worked and `path` is not in
/// it. Give [`Error::Hid`] in all other cases: the device is still there, so
/// the error can go away.
fn classify(error: HidError, path: &CStr, found: Result<Vec<Found>>) -> Error {
    match found {
        Ok(found) if !found.iter().any(|f| f.path.as_c_str() == path) => Error::Disconnected,
        _ => Error::Hid(error),
    }
}

impl Transport for HidTransport {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        self.dev.write(report).map_err(|e| self.classify(e))?;
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<Option<usize>> {
        // Ok(0) means the timeout elapsed with no report available.
        match self
            .dev
            .read_timeout(buf, timeout.as_millis() as i32)
            .map_err(|e| self.classify(e))?
        {
            0 => Ok(None),
            n => Ok(Some(n)),
        }
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.dev
            .get_feature_report(buf)
            .map_err(|e| self.classify(e))
    }
}

// Compile-time check: the real transport can move to another thread.
const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<HidTransport>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DESCRIPTORS;

    /// A made-up entry. The path does not exist.
    fn entry(known: bool, usage_page: u16, interface: i32) -> Found {
        let d = &DESCRIPTORS[0];
        Found {
            vid: if known { d.vid } else { 0x1234 },
            pid: if known { d.pid } else { 0x5678 },
            product: "Test device".to_string(),
            serial: None,
            usage_page,
            interface,
            known: if known { Some(d) } else { None },
            path: CString::new(format!("test-path-{interface}-{usage_page:04x}")).unwrap(),
        }
    }

    #[test]
    fn selects_the_vendor_interface_in_any_order() {
        let vendor = entry(true, 0xFFA0, 0);
        let keyboard = entry(true, 0x0001, 1);

        for list in [
            vec![vendor.clone(), keyboard.clone()],
            vec![keyboard.clone(), vendor.clone()],
        ] {
            let chosen = select(&list).unwrap();
            assert_eq!(chosen.usage_page, 0xFFA0);
            assert_eq!(chosen.path, vendor.path);
        }
    }

    #[test]
    fn selects_every_vendor_usage_page() {
        for &page in VENDOR_USAGE_PAGES {
            let list = [entry(true, page, 0)];
            assert_eq!(select(&list).unwrap().usage_page, page);
        }
    }

    #[test]
    fn known_device_without_vendor_interface_is_a_specific_error() {
        let list = [entry(true, 0x0001, 1), entry(true, 0x000C, 1)];
        assert!(matches!(select(&list), Err(Error::NoVendorInterface)));
    }

    #[test]
    fn unknown_device_with_vendor_page_is_not_selected() {
        let list = [entry(false, 0xFFA0, 0)];
        assert!(matches!(select(&list), Err(Error::DeviceNotFound)));
    }

    #[test]
    fn unknown_device_does_not_hide_a_known_device_without_vendor_interface() {
        let list = [entry(false, 0xFFA0, 0), entry(true, 0x0001, 1)];
        assert!(matches!(select(&list), Err(Error::NoVendorInterface)));
    }

    #[test]
    fn empty_list_is_not_found() {
        assert!(matches!(select(&[]), Err(Error::DeviceNotFound)));
    }

    #[test]
    fn open_refuses_an_interface_that_is_not_a_vendor_interface() {
        // The path does not exist. If the check did not run first, the error
        // would come from `hidapi`, and it would not be this variant.
        for page in [0x0001, 0x000C] {
            let found = entry(true, page, 1);
            assert!(matches!(
                HidTransport::open(&found),
                Err(Error::NoVendorInterface)
            ));
        }
    }

    #[test]
    fn open_refuses_an_unknown_device() {
        let found = entry(false, 0xFFA0, 0);
        assert!(matches!(
            HidTransport::open(&found),
            Err(Error::DeviceNotFound)
        ));
    }

    fn hid_error() -> HidError {
        HidError::HidApiError {
            message: "hid_read_timeout: device disconnected".to_string(),
        }
    }

    #[test]
    fn a_failure_is_a_disconnection_when_the_path_is_gone() {
        let open = entry(true, 0xFFA0, 0);
        let other = entry(true, 0x0001, 1);
        let error = classify(hid_error(), &open.path, Ok(vec![other]));
        assert!(matches!(error, Error::Disconnected));
    }

    #[test]
    fn a_failure_is_not_a_disconnection_when_the_path_is_still_there() {
        let open = entry(true, 0xFFA0, 0);
        let error = classify(hid_error(), &open.path, Ok(vec![open.clone()]));
        assert!(matches!(error, Error::Hid(_)));
    }

    #[test]
    fn a_failure_is_not_a_disconnection_when_the_enumeration_fails() {
        let open = entry(true, 0xFFA0, 0);
        let error = classify(hid_error(), &open.path, Err(Error::DeviceNotFound));
        assert!(matches!(error, Error::Hid(_)));
    }
}
