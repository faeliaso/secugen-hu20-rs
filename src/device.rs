use crate::error::{ScannerError, UsbOperation};
use crate::protocol::{
    is_hu20, BULK_IN_ENDPOINT, BULK_READ_SIZE, CONTROL_TIMEOUT, DEFAULT_EXPOSURE, DEFAULT_GAIN,
    DUMP_FRAME_REQUEST, FRAME_TIMEOUT, I2C_REQUEST, INTERFACE_NUMBER, READ_SERIAL_REQUEST,
    RESET_DEVICE_REQUEST, SECUGEN_VID, SENSOR_I2C_ADDRESS, SENSOR_REGISTERS,
    SENSOR_REG_EXPOSURE_HIGH, SENSOR_REG_EXPOSURE_LOW, SENSOR_REG_GAIN, SERIAL_NUMBER_LENGTH,
    SET_LED_REQUEST, START_IMAGE_REQUEST, STOP_IMAGE_REQUEST,
};
use crate::{DeviceId, RawFrame, FRAME_BYTES};
use rusb::{Device, DeviceHandle, GlobalContext, TransferType};

/// Stable metadata for an opened HU20.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// USB vendor identifier.
    pub vendor_id: u16,
    /// USB product identifier.
    pub product_id: u16,
    /// Host USB bus number.
    pub bus_number: u8,
    /// Host USB device address.
    pub address: u8,
    /// Product string, when readable.
    pub product_name: Option<String>,
}

impl DeviceInfo {
    /// Returns the topology identifier for this opened device.
    #[must_use]
    pub const fn id(&self) -> DeviceId {
        DeviceId::new(self.bus_number, self.address)
    }
}

/// An exclusively claimed HU20 USB handle.
pub struct SecuGenDevice {
    handle: DeviceHandle<GlobalContext>,
    info: DeviceInfo,
    initialized: bool,
    exposure: u16,
    gain: u8,
    #[cfg(target_os = "linux")]
    detached_kernel_driver: bool,
}

impl SecuGenDevice {
    /// Opens the first HU20 found on the USB bus.
    pub fn open_first() -> Result<Self, ScannerError> {
        let devices = rusb::devices().map_err(usb_error_enumerate)?;
        for device in devices.iter() {
            let descriptor = device
                .device_descriptor()
                .map_err(|error| ScannerError::Usb {
                    operation: UsbOperation::ReadDescriptor,
                    message: error.to_string(),
                })?;
            if descriptor.vendor_id() == SECUGEN_VID && is_hu20(descriptor.product_id()) {
                return Self::open_device(device, descriptor);
            }
        }
        Err(ScannerError::DeviceNotFound)
    }

    /// Opens the HU20 identified by a USB topology ID.
    pub fn open_by_id(id: &DeviceId) -> Result<Self, ScannerError> {
        let devices = rusb::devices().map_err(usb_error_enumerate)?;
        for device in devices.iter() {
            if device.bus_number() != id.bus_number || device.address() != id.address {
                continue;
            }
            let descriptor = device
                .device_descriptor()
                .map_err(|error| ScannerError::Usb {
                    operation: UsbOperation::ReadDescriptor,
                    message: error.to_string(),
                })?;
            if descriptor.vendor_id() == SECUGEN_VID && is_hu20(descriptor.product_id()) {
                return Self::open_device(device, descriptor);
            }
        }
        Err(ScannerError::DeviceWithIdNotFound(*id))
    }

    /// Opens the only HU20 found on the USB bus.
    pub fn open_single() -> Result<Self, ScannerError> {
        let devices = crate::list_devices()?;
        match devices.as_slice() {
            [] => Err(ScannerError::DeviceNotFound),
            [device] => Self::open_by_id(&device.id),
            many => Err(ScannerError::MultipleDevicesFound(many.len())),
        }
    }

    /// Returns metadata for the exclusively opened scanner.
    #[must_use]
    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    /// Turns the HU20 optical LED on or off.
    ///
    /// This command is statically recovered from the vendor SDK and must be
    /// physically validated with `examples/led_probe.rs` before capture code is
    /// enabled.
    pub fn set_led(&mut self, on: bool) -> Result<(), ScannerError> {
        let request_type = rusb::request_type(
            rusb::Direction::Out,
            rusb::RequestType::Vendor,
            rusb::Recipient::Device,
        );
        self.handle
            .write_control(
                request_type,
                SET_LED_REQUEST,
                u16::from(on),
                0,
                &[],
                CONTROL_TIMEOUT,
            )
            .map_err(|error| ScannerError::Usb {
                operation: UsbOperation::SetLed,
                message: error.to_string(),
            })?;
        Ok(())
    }

    /// Resets the scanner transport using the SDK's named recovery command.
    ///
    /// This does not re-enumerate the USB device. It is intentionally explicit
    /// so applications can recover a scanner without making reset part of the
    /// normal capture path.
    pub fn reset(&mut self) -> Result<(), ScannerError> {
        self.vendor_control(RESET_DEVICE_REQUEST, 0, UsbOperation::ResetDevice)
    }

    /// Reads the 30-byte hardware serial number using vendor request `0x25`.
    ///
    /// Statically recovered from `CVxdInterface::ReadSerialNum` / `GetDeviceInfo`,
    /// which the vendor SDK always issues before sensor initialization.
    pub fn read_serial_number(&mut self) -> Result<[u8; SERIAL_NUMBER_LENGTH], ScannerError> {
        let request_type = rusb::request_type(
            rusb::Direction::In,
            rusb::RequestType::Vendor,
            rusb::Recipient::Device,
        );
        let mut buf = [0_u8; SERIAL_NUMBER_LENGTH];
        self.handle
            .read_control(
                request_type,
                READ_SERIAL_REQUEST,
                0,
                0,
                &mut buf,
                CONTROL_TIMEOUT,
            )
            .map_err(|error| ScannerError::Usb {
                operation: UsbOperation::ReadSerialNumber,
                message: error.to_string(),
            })?;
        Ok(buf)
    }

    /// Flushes/dumps the hardware frame buffer using vendor request `0x05`.
    ///
    /// Statically recovered from `CVxdInterface::DumpFrame`.
    pub fn dump_frame(&mut self) -> Result<(), ScannerError> {
        self.vendor_control(DUMP_FRAME_REQUEST, 1, UsbOperation::DumpFrame)
    }

    /// Captures a frame using the current LED state without turning it on or off.
    pub fn capture_frame_raw(&mut self) -> Result<RawFrame, ScannerError> {
        self.initialize_sensor()?;
        self.start_image_stream()?;
        let result = self.read_frame_bytes().and_then(RawFrame::new);
        let stop_result = self.stop_image_stream();
        match (result, stop_result) {
            (Ok(frame), Ok(())) => Ok(frame),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    /// Reads all available bytes from the Bulk IN endpoint until completion.
    pub fn read_full_bulk_stream(&mut self, max_bytes: usize) -> Result<Vec<u8>, ScannerError> {
        self.initialize_sensor()?;
        self.set_led(true)?;
        std::thread::sleep(std::time::Duration::from_millis(150));
        self.start_image_stream()?;
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 16384];
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while buffer.len() < max_bytes && std::time::Instant::now() < deadline {
            match self.handle.read_bulk(
                BULK_IN_ENDPOINT,
                &mut chunk,
                std::time::Duration::from_millis(500),
            ) {
                Ok(read) if read > 0 => {
                    let needed = max_bytes - buffer.len();
                    buffer.extend_from_slice(&chunk[..read.min(needed)]);
                }
                _ => break,
            }
        }
        let _ = self.stop_image_stream();
        let _ = self.set_led(false);
        Ok(buffer)
    }

    /// Captures a complete native 1100x800 CMOS sensor frame (880,000 bytes) from the scanner.
    pub fn capture_raw_sensor_frame(&mut self) -> Result<Vec<u8>, ScannerError> {
        let raw = self.read_full_bulk_stream(crate::RAW_FRAME_BYTES)?;
        if raw.len() < crate::RAW_FRAME_BYTES {
            return Err(ScannerError::FrameReadTimeout {
                received: raw.len(),
                expected: crate::RAW_FRAME_BYTES,
            });
        }
        let mut frame = raw[..crate::RAW_FRAME_BYTES].to_vec();
        frame.iter_mut().for_each(|pixel| *pixel = !*pixel);
        Ok(frame)
    }

    /// Captures a native `SensorFrame` quickly without altering optical LED state.
    pub fn capture_sensor_frame_quick(&mut self) -> Result<crate::SensorFrame, ScannerError> {
        self.initialize_sensor()?;
        self.start_image_stream()?;
        let mut buffer = Vec::with_capacity(crate::RAW_FRAME_BYTES);
        let mut chunk = [0_u8; 16384];
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1500);
        while buffer.len() < crate::RAW_FRAME_BYTES && std::time::Instant::now() < deadline {
            match self.handle.read_bulk(
                BULK_IN_ENDPOINT,
                &mut chunk,
                std::time::Duration::from_millis(300),
            ) {
                Ok(read) if read > 0 => {
                    let needed = crate::RAW_FRAME_BYTES - buffer.len();
                    buffer.extend_from_slice(&chunk[..read.min(needed)]);
                }
                _ => break,
            }
        }
        let _ = self.stop_image_stream();
        if buffer.len() < crate::RAW_FRAME_BYTES {
            return Err(ScannerError::FrameReadTimeout {
                received: buffer.len(),
                expected: crate::RAW_FRAME_BYTES,
            });
        }
        let mut frame = buffer[..crate::RAW_FRAME_BYTES].to_vec();
        frame.iter_mut().for_each(|pixel| *pixel = !*pixel);
        crate::SensorFrame::new(frame)
    }

    /// Dynamically tunes sensor exposure and gain (Smart Capture) in a closed-loop.
    ///
    /// Evaluates contact luminance and ridge-to-valley contrast:
    /// - Heavy press / wet finger: decreases exposure/gain to prevent ridge collapse.
    /// - Light touch / dry finger: increases exposure/gain to reveal faint minutiae.
    ///
    /// Returns `(exposure, gain, contrast_delta)`.
    pub fn auto_tune_exposure(&mut self) -> Result<(u16, u8, u8), ScannerError> {
        self.set_led(true)?;
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut best_exposure = self.exposure;
        let mut best_gain = self.gain;
        let mut best_delta = 0_u8;

        for _ in 0..7 {
            let frame = self.capture_sensor_frame_quick()?;
            let bytes = frame.as_bytes();

            // Evaluate touch statistics within active optical platen window
            let mut contact_count = 0_usize;
            let mut contact_sum = 0_u64;
            let mut deep_black_count = 0_usize;
            let mut min_val = 255_u8;
            let mut max_val = 0_u8;

            let x0 = crate::PLATEN_CROP_X0;
            let x1 = crate::PLATEN_CROP_X1;
            let y0 = crate::PLATEN_CROP_Y0;
            let y1 = crate::PLATEN_CROP_Y1;
            let w = crate::RAW_SENSOR_WIDTH;

            for y in y0..y1 {
                for x in x0..x1 {
                    let p = bytes[y * w + x];
                    if p < 185 {
                        contact_count += 1;
                        contact_sum += u64::from(p);
                        if p < 35 {
                            deep_black_count += 1;
                        }
                        min_val = min_val.min(p);
                        max_val = max_val.max(p);
                    }
                }
            }

            if contact_count < 5_000 {
                // No finger or barely touching, retain factory settings
                break;
            }

            let mean_touch = (contact_sum / contact_count as u64) as u8;
            let delta = max_val.saturating_sub(min_val);
            let deep_black_pct = (deep_black_count * 100) / contact_count;

            if delta > best_delta {
                best_delta = delta;
                best_exposure = self.exposure;
                best_gain = self.gain;
            }

            // Smart Capture Closed-Loop Control:
            if deep_black_pct > 8 || mean_touch < 75 {
                // Saturated / heavy pressure -> reduce exposure and/or gain
                let new_exp = self.exposure.saturating_sub(crate::EXPOSURE_STEP * 2);
                if new_exp < crate::EXPOSURE_MIN {
                    let new_gain = self
                        .gain
                        .saturating_sub(crate::GAIN_STEP)
                        .max(crate::GAIN_MIN);
                    self.set_gain(new_gain)?;
                } else {
                    self.set_exposure(new_exp)?;
                }
            } else if mean_touch > 130 || delta < 25 {
                // Underexposed / light touch / dry -> increase exposure and/or gain
                let new_exp = (self.exposure + crate::EXPOSURE_STEP * 2).min(crate::EXPOSURE_MAX);
                if new_exp >= crate::EXPOSURE_MAX {
                    let new_gain = (self.gain + crate::GAIN_STEP).min(crate::GAIN_MAX);
                    self.set_gain(new_gain)?;
                } else {
                    self.set_exposure(new_exp)?;
                }
            } else {
                // Optimal balance achieved
                best_exposure = self.exposure;
                best_gain = self.gain;
                best_delta = delta;
                break;
            }
        }

        self.set_exposure(best_exposure)?;
        self.set_gain(best_gain)?;
        Ok((best_exposure, best_gain, best_delta))
    }

    /// Captures a professionally tuned biometric frame using Smart Capture and clean framing.
    pub fn capture_smart_frame(&mut self) -> Result<RawFrame, ScannerError> {
        let _ = self.auto_tune_exposure()?;
        let sensor_frame = self.capture_sensor_frame_quick()?;
        let _ = self.set_led(false);
        sensor_frame.to_biometric_frame()
    }

    /// Captures a complete HU20 frame.
    pub fn capture_frame(&mut self) -> Result<RawFrame, ScannerError> {
        self.initialize_sensor()?;
        self.set_led(true)?;
        // Allow optical illumination to stabilize on the platen prism
        std::thread::sleep(std::time::Duration::from_millis(150));
        let frame = self.capture_frame_raw();
        let _ = self.set_led(false);
        frame
    }

    fn open_device(
        device: Device<GlobalContext>,
        descriptor: rusb::DeviceDescriptor,
    ) -> Result<Self, ScannerError> {
        let config = device
            .active_config_descriptor()
            .map_err(|error| ScannerError::Usb {
                operation: UsbOperation::ReadDescriptor,
                message: error.to_string(),
            })?;
        let has_capture_endpoint = config.interfaces().any(|interface| {
            interface.descriptors().any(|descriptor| {
                descriptor.interface_number() == INTERFACE_NUMBER
                    && descriptor.endpoint_descriptors().any(|endpoint| {
                        endpoint.address() == BULK_IN_ENDPOINT
                            && endpoint.transfer_type() == TransferType::Bulk
                            && endpoint.direction() == rusb::Direction::In
                    })
            })
        });
        if !has_capture_endpoint {
            return Err(ScannerError::UnsupportedUsbLayout);
        }
        #[allow(unused_mut)]
        let mut handle = device.open().map_err(|error| ScannerError::Usb {
            operation: UsbOperation::OpenDevice,
            message: error.to_string(),
        })?;
        // Match `FDU_USBOpenSecugenDevice` in the supplied Linux SDK.  The
        // explicit configuration/alternate-setting selection matters on hosts
        // that leave an interface in a partially configured state, but on macOS
        // re-selecting an already-active configuration can reset or stall EP0.
        if handle.active_configuration().unwrap_or(0) != 1 {
            handle
                .set_active_configuration(1)
                .map_err(|error| ScannerError::Usb {
                    operation: UsbOperation::SetConfiguration,
                    message: error.to_string(),
                })?;
        }
        #[cfg(target_os = "linux")]
        let detached_kernel_driver = if handle
            .kernel_driver_active(INTERFACE_NUMBER)
            .unwrap_or(false)
        {
            handle
                .detach_kernel_driver(INTERFACE_NUMBER)
                .map_err(|error| ScannerError::Usb {
                    operation: UsbOperation::ClaimInterface,
                    message: error.to_string(),
                })?;
            true
        } else {
            false
        };
        handle
            .claim_interface(INTERFACE_NUMBER)
            .map_err(|error| match error {
                rusb::Error::Busy => ScannerError::InterfaceBusy,
                other => ScannerError::Usb {
                    operation: UsbOperation::ClaimInterface,
                    message: other.to_string(),
                },
            })?;
        // macOS already activates alternate setting zero when configuration 1
        // is selected.  Its libusb backend reports `Other` when asked to set
        // that same default altsetting explicitly, despite exposing the
        // verified `0x82` endpoint. Linux follows the SDK call verbatim.
        #[cfg(not(target_os = "macos"))]
        handle
            .set_alternate_setting(INTERFACE_NUMBER, 0)
            .map_err(|error| ScannerError::Usb {
                operation: UsbOperation::SetAlternateSetting,
                message: error.to_string(),
            })?;
        let info = DeviceInfo {
            vendor_id: descriptor.vendor_id(),
            product_id: descriptor.product_id(),
            bus_number: device.bus_number(),
            address: device.address(),
            product_name: handle.read_product_string_ascii(&descriptor).ok(),
        };
        Ok(Self {
            handle,
            info,
            initialized: false,
            exposure: DEFAULT_EXPOSURE,
            gain: DEFAULT_GAIN,
            #[cfg(target_os = "linux")]
            detached_kernel_driver,
        })
    }

    fn start_image_stream(&mut self) -> Result<(), ScannerError> {
        self.vendor_control(START_IMAGE_REQUEST, 0, UsbOperation::StartCapture)
    }

    fn initialize_sensor(&mut self) -> Result<(), ScannerError> {
        if self.initialized {
            return Ok(());
        }
        // Match vendor SDK handshake (OpenUSB -> GetDeviceInfo): query serial number
        // before starting CMOS register configuration over the I²C bridge.
        let _ = self.read_serial_number().ok();
        for &(register, value) in SENSOR_REGISTERS {
            self.write_sensor_register(register, value)?;
        }
        // `SetFrameWindowPos(250, 40, 1100, 800, true)` from the SDK.
        for (register, value) in [
            (0x41, 0x00),
            (0x42, 0xfa),
            (0x43, 0x04),
            (0x44, 0x4f),
            (0x45, 0x00),
            (0x46, 0x28),
            (0x47, 0x03),
            (0x48, 0x23),
        ] {
            self.write_sensor_register(register, value)?;
        }
        // Apply configured or calibrated factory brightness defaults
        self.write_sensor_register(SENSOR_REG_EXPOSURE_HIGH, (self.exposure >> 8) as u8)?;
        self.write_sensor_register(SENSOR_REG_EXPOSURE_LOW, (self.exposure & 0xff) as u8)?;
        self.write_sensor_register(SENSOR_REG_GAIN, self.gain)?;
        self.initialized = true;
        Ok(())
    }

    /// Sets the CMOS sensor exposure time.
    ///
    /// Statically recovered from `CVxdInterface::SetExposure`. For the HU20 (PID `0x2200`),
    /// this sets sensor registers `0x30` (MSB) and `0x31` (LSB).
    pub fn set_exposure(&mut self, exposure: u16) -> Result<(), ScannerError> {
        self.initialize_sensor()?;
        self.exposure = exposure;
        self.write_sensor_register(SENSOR_REG_EXPOSURE_HIGH, (exposure >> 8) as u8)?;
        self.write_sensor_register(SENSOR_REG_EXPOSURE_LOW, (exposure & 0xff) as u8)
    }

    /// Sets the CMOS sensor analog gain value.
    ///
    /// Statically recovered from `CVxdInterface::SetGainValue`. For the HU20 (PID `0x2200`),
    /// this sets sensor register `0x32`.
    pub fn set_gain(&mut self, gain: u8) -> Result<(), ScannerError> {
        self.initialize_sensor()?;
        self.gain = gain;
        self.write_sensor_register(SENSOR_REG_GAIN, gain)
    }

    /// Reads a CMOS sensor register via the vendor I²C bridge (`0xc0/0x22`).
    ///
    /// Statically recovered from `CVxdInterface::GetSensorRegister` / `GetI2C`.
    pub fn read_sensor_register(&mut self, register: u16) -> Result<u8, ScannerError> {
        let request_type = rusb::request_type(
            rusb::Direction::In,
            rusb::RequestType::Vendor,
            rusb::Recipient::Device,
        );
        let mut buf = [0_u8; 1];
        self.handle
            .read_control(
                request_type,
                I2C_REQUEST,
                SENSOR_I2C_ADDRESS,
                register,
                &mut buf,
                CONTROL_TIMEOUT,
            )
            .map_err(|error| ScannerError::Usb {
                operation: UsbOperation::InitializeSensor,
                message: error.to_string(),
            })?;
        Ok(buf[0])
    }

    /// Writes a CMOS sensor register via the vendor I²C bridge (`0x40/0x22`)
    /// and immediately reads it back to verify equality, matching
    /// `CVxdInterface::SetSensorRegister`.
    pub fn write_sensor_register(&mut self, register: u16, value: u8) -> Result<(), ScannerError> {
        let request_type = rusb::request_type(
            rusb::Direction::Out,
            rusb::RequestType::Vendor,
            rusb::Recipient::Device,
        );
        self.handle
            .write_control(
                request_type,
                I2C_REQUEST,
                SENSOR_I2C_ADDRESS,
                register,
                &[value],
                CONTROL_TIMEOUT,
            )
            .map_err(|error| ScannerError::Usb {
                operation: UsbOperation::InitializeSensor,
                message: error.to_string(),
            })?;
        // Mirror CVxdInterface::SetSensorRegister verification
        let readback = self.read_sensor_register(register)?;
        if readback != value {
            return Err(ScannerError::Usb {
                operation: UsbOperation::InitializeSensor,
                message: format!(
                    "Sensor register 0x{register:02x} readback mismatch: expected 0x{value:02x}, got 0x{readback:02x}"
                ),
            });
        }
        Ok(())
    }

    fn stop_image_stream(&mut self) -> Result<(), ScannerError> {
        self.vendor_control(STOP_IMAGE_REQUEST, 0, UsbOperation::StopCapture)
    }

    fn vendor_control(
        &mut self,
        request: u8,
        value: u16,
        operation: UsbOperation,
    ) -> Result<(), ScannerError> {
        let request_type = rusb::request_type(
            rusb::Direction::Out,
            rusb::RequestType::Vendor,
            rusb::Recipient::Device,
        );
        self.handle
            .write_control(request_type, request, value, 0, &[], CONTROL_TIMEOUT)
            .map(|_| ())
            .map_err(|error| ScannerError::Usb {
                operation,
                message: error.to_string(),
            })
    }

    fn read_frame_bytes(&mut self) -> Result<Vec<u8>, ScannerError> {
        let mut frame = Vec::with_capacity(FRAME_BYTES);
        let mut chunk = [0_u8; BULK_READ_SIZE];
        while frame.len() < FRAME_BYTES {
            match self
                .handle
                .read_bulk(BULK_IN_ENDPOINT, &mut chunk, FRAME_TIMEOUT)
            {
                Ok(0) => break,
                Ok(read) => {
                    let needed = FRAME_BYTES - frame.len();
                    let to_take = read.min(needed);
                    frame.extend_from_slice(&chunk[..to_take]);
                }
                Err(rusb::Error::Timeout) if frame.is_empty() => return Err(ScannerError::Timeout),
                Err(rusb::Error::Timeout) => {
                    return Err(ScannerError::FrameReadTimeout {
                        received: frame.len(),
                        expected: FRAME_BYTES,
                    });
                }
                Err(error) => {
                    return Err(ScannerError::Usb {
                        operation: UsbOperation::ReadFrame,
                        message: error.to_string(),
                    });
                }
            }
        }
        if frame.len() < FRAME_BYTES {
            return Err(ScannerError::FrameReadTimeout {
                received: frame.len(),
                expected: FRAME_BYTES,
            });
        }
        // Invert polarity per CSIDO020A::DoModify (`notb` instruction)
        frame.iter_mut().for_each(|pixel| *pixel = !*pixel);
        Ok(frame)
    }
}

impl Drop for SecuGenDevice {
    fn drop(&mut self) {
        let _ = self.set_led(false);
        let _ = self.handle.release_interface(INTERFACE_NUMBER);
        #[cfg(target_os = "linux")]
        if self.detached_kernel_driver {
            let _ = self.handle.attach_kernel_driver(INTERFACE_NUMBER);
        }
    }
}

fn usb_error_enumerate(error: rusb::Error) -> ScannerError {
    ScannerError::Usb {
        operation: UsbOperation::Enumerate,
        message: error.to_string(),
    }
}
