use crate::error::{ScannerError, UsbOperation};
use crate::protocol::{is_hu20, HU20_PID, SECUGEN_VID};
use core::fmt;
use core::str::FromStr;

/// A stable USB topology identifier for a connected scanner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId {
    /// Host USB bus number.
    pub bus_number: u8,
    /// Host USB device address.
    pub address: u8,
}

impl DeviceId {
    /// Creates an identifier from a host bus number and device address.
    #[must_use]
    pub const fn new(bus_number: u8, address: u8) -> Self {
        Self {
            bus_number,
            address,
        }
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.bus_number, self.address)
    }
}

/// Error returned for an invalid [`DeviceId`] string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDeviceIdError {
    details: String,
}

impl fmt::Display for ParseDeviceIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid device ID: {}", self.details)
    }
}

impl std::error::Error for ParseDeviceIdError {}

impl FromStr for DeviceId {
    type Err = ParseDeviceIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        let parts: Vec<_> = value.split(':').collect();
        if parts.len() != 2 {
            return Err(ParseDeviceIdError {
                details: "expected bus:address".into(),
            });
        }
        let bus_number = parts[0].trim().parse().map_err(|_| ParseDeviceIdError {
            details: "bus number must be in 0..=255".into(),
        })?;
        let address = parts[1].trim().parse().map_err(|_| ParseDeviceIdError {
            details: "address must be in 0..=255".into(),
        })?;
        Ok(Self::new(bus_number, address))
    }
}

/// A HU20 discovered without claiming exclusive use of its USB interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDevice {
    /// USB topology identifier.
    pub id: DeviceId,
    /// USB vendor identifier (`0x1162`).
    pub vendor_id: u16,
    /// USB product identifier (`0x2200`).
    pub product_id: u16,
    /// Physical hub-port chain, if the host reports one.
    pub port_numbers: Option<Vec<u8>>,
    /// USB product string, if readable without a claim.
    pub product_name: Option<String>,
    /// USB serial string, if readable without a claim.
    pub serial_number: Option<String>,
}

/// Lists attached HU20 scanners without claiming their interface.
pub fn list_devices() -> Result<Vec<DiscoveredDevice>, ScannerError> {
    let devices = rusb::devices().map_err(|error| ScannerError::Usb {
        operation: UsbOperation::Enumerate,
        message: error.to_string(),
    })?;
    let mut result = Vec::new();
    for device in devices.iter() {
        let descriptor = match device.device_descriptor() {
            Ok(value) => value,
            Err(_) => continue,
        };
        if descriptor.vendor_id() != SECUGEN_VID || !is_hu20(descriptor.product_id()) {
            continue;
        }
        let strings = device.open().ok().map(|handle| {
            (
                handle.read_product_string_ascii(&descriptor).ok(),
                handle.read_serial_number_string_ascii(&descriptor).ok(),
            )
        });
        let (product_name, serial_number) = strings.unwrap_or((None, None));
        result.push(DiscoveredDevice {
            id: DeviceId::new(device.bus_number(), device.address()),
            vendor_id: SECUGEN_VID,
            product_id: HU20_PID,
            port_numbers: device.port_numbers().ok(),
            product_name,
            serial_number,
        });
    }
    result.sort_by_key(|device| (device.id.bus_number, device.id.address));
    Ok(result)
}
