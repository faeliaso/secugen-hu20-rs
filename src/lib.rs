//! Native userspace USB support for the SecuGen Hamster Pro 20 (HU20).
//!
//! This crate is limited to device discovery and uncompressed grayscale frames.
//! Biometric matching, templates, quality analysis, and application workflows
//! intentionally belong to higher-level consumers.
//!
//! Hardware capture, LED control, and sensor initialization are physically
//! validated against the SecuGen Hamster Pro 20; see `docs/PROTOCOL_REVERSE_ENGINEERING.md`.

#![warn(missing_docs)]

mod device;
/// USB discovery types and non-exclusive HU20 enumeration.
pub mod discovery;
/// Typed errors returned by this crate.
pub mod error;
/// Validated HU20 grayscale frame representation and image export.
pub mod frame;
mod protocol;

pub use device::{DeviceInfo, SecuGenDevice};
pub use discovery::{list_devices, DeviceId, DiscoveredDevice, ParseDeviceIdError};
pub use error::{ScannerError, UsbOperation};
pub use frame::{resample_bilinear, RawFrame, SensorFrame};
pub use protocol::{
    DEFAULT_EXPOSURE, DEFAULT_GAIN, EXPOSURE_MAX, EXPOSURE_MIN, EXPOSURE_STEP, FRAME_BYTES,
    GAIN_MAX, GAIN_MIN, GAIN_STEP, HU20_PID, PLATEN_ACTIVE_HEIGHT, PLATEN_ACTIVE_WIDTH,
    PLATEN_CROP_X0, PLATEN_CROP_X1, PLATEN_CROP_Y0, PLATEN_CROP_Y1, RAW_FRAME_BYTES,
    RAW_SENSOR_HEIGHT, RAW_SENSOR_WIDTH, SECUGEN_VID, SENSOR_DPI, SENSOR_HEIGHT, SENSOR_WIDTH,
    SERIAL_NUMBER_LENGTH,
};
