//! Constants that identify the validated hardware target.
//!
//! The image-stream commands are recovered from the supplied SDK and are being
//! physically validated on macOS. See the protocol-reverse-engineering record.

/// SecuGen's USB vendor identifier.
pub const SECUGEN_VID: u16 = 0x1162;
/// Hamster Pro 20 (HU20) product identifier.
pub const HU20_PID: u16 = 0x2200;
/// Width of an HU20 biometric frame in pixels.
pub const SENSOR_WIDTH: usize = 300;
/// Height of an HU20 biometric frame in pixels.
pub const SENSOR_HEIGHT: usize = 400;
/// Resolution of an HU20 biometric frame.
pub const SENSOR_DPI: u16 = 500;
/// Number of bytes in one L8 HU20 frame.
pub const FRAME_BYTES: usize = SENSOR_WIDTH * SENSOR_HEIGHT;

/// Width of the HU20 native CSIDO020A CMOS sensor matrix in pixels.
pub const RAW_SENSOR_WIDTH: usize = 1100;
/// Height of the HU20 native CSIDO020A CMOS sensor matrix in pixels.
pub const RAW_SENSOR_HEIGHT: usize = 800;
/// Number of bytes in one native HU20 CMOS frame.
pub const RAW_FRAME_BYTES: usize = RAW_SENSOR_WIDTH * RAW_SENSOR_HEIGHT;

/// Default sensor exposure setting (calibrated default in vendor SDK: 2500).
pub const DEFAULT_EXPOSURE: u16 = 2500;
/// Minimum operational exposure limit for CSIDO020A.
pub const EXPOSURE_MIN: u16 = 500;
/// Maximum operational exposure limit for CSIDO020A.
pub const EXPOSURE_MAX: u16 = 8000;
/// Step size for exposure auto-tuning.
pub const EXPOSURE_STEP: u16 = 250;

/// Default sensor gain setting.
pub const DEFAULT_GAIN: u8 = 0x08;
/// Minimum operational gain limit for CSIDO020A.
pub const GAIN_MIN: u8 = 0x04;
/// Maximum operational gain limit for CSIDO020A.
pub const GAIN_MAX: u8 = 0x28;
/// Step size for gain auto-tuning.
pub const GAIN_STEP: u8 = 0x04;

/// Left column offset of active optical prism platen window in native sensor frame.
pub const PLATEN_CROP_X0: usize = 140;
/// Right column limit of active optical prism platen window in native sensor frame.
pub const PLATEN_CROP_X1: usize = 960;
/// Top row offset of active optical prism platen window in native sensor frame.
pub const PLATEN_CROP_Y0: usize = 130;
/// Bottom row limit of active optical prism platen window in native sensor frame.
pub const PLATEN_CROP_Y1: usize = 750;
/// Active optical prism width in native sensor pixels.
pub const PLATEN_ACTIVE_WIDTH: usize = PLATEN_CROP_X1 - PLATEN_CROP_X0;
/// Active optical prism height in native sensor pixels.
pub const PLATEN_ACTIVE_HEIGHT: usize = PLATEN_CROP_Y1 - PLATEN_CROP_Y0;

/// Interface claimed by the vendor SDK for the HU20 transport.
pub(crate) const INTERFACE_NUMBER: u8 = 0;
/// Vendor request used by the SDK to control the optical LED.
pub(crate) const SET_LED_REQUEST: u8 = 0x11;
/// Alternate vendor request that begins the image stream in the SDK path.
///
/// Recovered from `CVxdInterface::StartSendingImg(true)`: `0x03` is the high-speed
/// capture start request used by `CSIDO020A::ReadImage`.
pub(crate) const START_IMAGE_REQUEST: u8 = 0x03;
/// Vendor request that stops the active image stream.
pub(crate) const STOP_IMAGE_REQUEST: u8 = 0x02;
/// Vendor request that dumps a diagnostic/preview frame (`wValue = 1`).
#[allow(dead_code)]
pub(crate) const DUMP_FRAME_REQUEST: u8 = 0x05;
/// Vendor request to query fake finger / live detection.
#[allow(dead_code)]
pub(crate) const CHECK_LIVE_FINGER_REQUEST: u8 = 0x16;
/// Vendor request used by the SDK during `GetDeviceInfo` to read the device serial.
pub(crate) const READ_SERIAL_REQUEST: u8 = 0x25;
/// Length in bytes of the serial number returned by `READ_SERIAL_REQUEST`.
pub const SERIAL_NUMBER_LENGTH: usize = 30;
/// Bulk-IN endpoint confirmed on the physical HU20.
pub(crate) const BULK_IN_ENDPOINT: u8 = 0x82;
/// Timeout used for control transfers.
///
/// The reference Linux SDK passes 2,000 ms to `usb_control_msg` for every
/// vendor request, including the sensor's I²C bridge operations.
pub(crate) const CONTROL_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(2_000);
/// Timeout for each bulk image read.
pub(crate) const FRAME_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
/// Maximum read request used while assembling a frame.
pub(crate) const BULK_READ_SIZE: usize = 4_096;
/// Vendor request used to read or write an I²C sensor register.
pub(crate) const I2C_REQUEST: u8 = 0x22;
/// Vendor request used by the SDK to reset the scanner transport.
pub(crate) const RESET_DEVICE_REQUEST: u8 = 0x14;
/// I²C address used by the HU20 CMOS sensor.
pub(crate) const SENSOR_I2C_ADDRESS: u16 = 0x37;

/// CMOS sensor register for high byte of exposure time (`exposure >> 8`).
#[allow(dead_code)]
pub(crate) const SENSOR_REG_EXPOSURE_HIGH: u16 = 0x30;
/// CMOS sensor register for low byte of exposure time (`exposure & 0xff`).
#[allow(dead_code)]
pub(crate) const SENSOR_REG_EXPOSURE_LOW: u16 = 0x31;
/// CMOS sensor register for analog gain value.
#[allow(dead_code)]
pub(crate) const SENSOR_REG_GAIN: u16 = 0x32;

/// CMOS register sequence recovered from `CSIDO020A::InitSensorReg`.
pub(crate) const SENSOR_REGISTERS: &[(u16, u8)] = &[
    (0x03, 0x02),
    (0x04, 0x81),
    (0x05, 0x0a),
    (0x08, 0x00),
    (0x09, 0x11),
    (0x0a, 0x11),
    (0x10, 0x11),
    (0x11, 0x23),
    (0x12, 0x85),
    (0x13, 0x00),
    (0x14, 0x27),
    (0x16, 0xb6),
    (0x30, 0x01),
    (0x31, 0xc0),
    (0x32, 0x08),
    (0x41, 0x00),
    (0x42, 0x00),
    (0x43, 0x06),
    (0x44, 0x43),
    (0x45, 0x00),
    (0x46, 0x00),
    (0x47, 0x04),
    (0x48, 0xb3),
    (0x49, 0x00),
    (0x4a, 0x20),
    (0x4b, 0x00),
    (0x4c, 0x00),
    (0x4d, 0x00),
    (0x4e, 0x00),
    (0x60, 0x0b),
    (0x61, 0x16),
    (0x62, 0x32),
    (0x63, 0x80),
    (0x71, 0x08),
    (0x80, 0xf8),
    (0x81, 0x06),
    (0x90, 0xaa),
    (0x91, 0x08),
    (0x92, 0x10),
    (0x93, 0x40),
    (0x94, 0x04),
    (0x95, 0x01),
    (0x96, 0x02),
    (0x97, 0x08),
    (0x98, 0x10),
    (0x99, 0x08),
    (0x9a, 0x03),
    (0x9b, 0xb0),
    (0x9c, 0x08),
    (0x9d, 0x24),
    (0x9e, 0x30),
    (0xb7, 0x15),
    (0xb8, 0x28),
    (0xb9, 0x04),
    (0x03, 0x05),
    (0xa5, 0x00),
    (0xa6, 0x00),
    (0xa7, 0x00),
    (0xa8, 0x00),
];

#[must_use]
pub(crate) const fn is_hu20(product_id: u16) -> bool {
    product_id == HU20_PID
}
