use thiserror::Error;

/// USB operation that failed without leaking a `rusb` error into the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbOperation {
    /// Enumerating host USB devices.
    Enumerate,
    /// Reading a USB descriptor.
    ReadDescriptor,
    /// Opening a USB device.
    OpenDevice,
    /// Selecting the scanner USB configuration.
    SetConfiguration,
    /// Claiming the scanner interface.
    ClaimInterface,
    /// Selecting the scanner interface alternate setting.
    SetAlternateSetting,
    /// Sending a vendor control transfer.
    SetLed,
    /// Resetting the scanner transport.
    ResetDevice,
    /// Initializing the CMOS sensor.
    InitializeSensor,
    /// Starting image streaming.
    StartCapture,
    /// Stopping image streaming.
    StopCapture,
    /// Reading bulk image data.
    ReadFrame,
    /// Flushing/dumping hardware frame buffer.
    DumpFrame,
    /// Reading device serial number via vendor request.
    ReadSerialNumber,
    /// Releasing the scanner interface.
    ReleaseInterface,
}

impl core::fmt::Display for UsbOperation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Enumerate => "enumerate devices",
            Self::ReadDescriptor => "read USB descriptor",
            Self::OpenDevice => "open device",
            Self::SetConfiguration => "select USB configuration",
            Self::ClaimInterface => "claim interface",
            Self::SetAlternateSetting => "select interface alternate setting",
            Self::SetLed => "set scanner LED",
            Self::ResetDevice => "reset scanner transport",
            Self::InitializeSensor => "initialize CMOS sensor",
            Self::StartCapture => "start image capture",
            Self::StopCapture => "stop image capture",
            Self::ReadFrame => "read fingerprint frame",
            Self::DumpFrame => "dump hardware frame",
            Self::ReadSerialNumber => "read device serial number",
            Self::ReleaseInterface => "release interface",
        })
    }
}

/// Failures returned by the scanner crate.
#[derive(Error, Debug)]
pub enum ScannerError {
    /// No HU20 was connected.
    #[error("no SecuGen HU20 scanner (1162:2200) was found on the USB bus")]
    DeviceNotFound,
    /// A selected topology identifier no longer identifies a HU20.
    #[error("HU20 scanner with ID '{0}' was not found on the USB bus")]
    DeviceWithIdNotFound(crate::discovery::DeviceId),
    /// More than one HU20 was found when exactly one was required.
    #[error("multiple SecuGen HU20 scanners found ({0}); select a device ID")]
    MultipleDevicesFound(usize),
    /// An operating-system USB operation failed.
    #[error("USB operation '{operation}' failed: {message}")]
    Usb {
        /// The USB operation that failed.
        operation: UsbOperation,
        /// The platform error message.
        message: String,
    },
    /// The scanner interface is already exclusively owned.
    #[error("the scanner USB interface is busy")]
    InterfaceBusy,
    /// The scanner does not expose interface zero in its active configuration.
    #[error("the scanner has no interface 0 in its active USB configuration")]
    UnsupportedUsbLayout,
    /// No image bytes arrived before the capture timeout.
    #[error("timed out while waiting for a fingerprint frame")]
    Timeout,
    /// The transfer timed out after a partial frame was received.
    #[error(
        "timed out while reading a fingerprint frame: received {received} of {expected} bytes"
    )]
    FrameReadTimeout {
        /// Number of bytes received before timeout.
        received: usize,
        /// Number of bytes required for the frame.
        expected: usize,
    },
    /// Bytes did not form an HU20 grayscale frame.
    #[error("incomplete fingerprint frame: received {received} of {expected} bytes")]
    IncompleteFrame {
        /// Number of bytes received.
        received: usize,
        /// Number of bytes required for an HU20 frame.
        expected: usize,
    },
    /// A PGM or PNG artifact could not be written.
    #[error("image export failed: {0}")]
    ImageExport(String),
}
