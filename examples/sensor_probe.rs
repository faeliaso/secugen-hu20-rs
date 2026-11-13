//! Safe, isolated probe to test CMOS sensor I²C read/write over vendor request 0x22.

use secugen_hu20_rs::SecuGenDevice;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = SecuGenDevice::open_single()?;
    println!("Opened HU20 at {}", scanner.info().id());

    // Handshake
    let serial = scanner.read_serial_number()?;
    println!("Hardware serial query succeeded ({} bytes).", serial.len());

    // Test reading CMOS register 0x03
    println!("Reading CMOS register 0x03 (initial state)...");
    match scanner.read_sensor_register(0x03) {
        Ok(val) => println!("CMOS register 0x03 = 0x{val:02x}"),
        Err(err) => {
            eprintln!("Failed to read CMOS register 0x03: {err}");
            return Err(err.into());
        }
    }

    // Test writing CMOS register 0x03 with value 0x02 (which verifies readback)
    println!("Writing 0x02 to CMOS register 0x03 (with readback verification)...");
    match scanner.write_sensor_register(0x03, 0x02) {
        Ok(()) => println!("Register 0x03 write and readback verification SUCCEEDED!"),
        Err(err) => {
            eprintln!("Failed to write register 0x03: {err}");
            return Err(err.into());
        }
    }

    // Try full sensor initialization
    println!("Testing full CMOS register sequence (SENSOR_REGISTERS + frame window)...");
    match scanner.capture_frame() {
        Ok(frame) => {
            let bytes = frame.as_bytes();
            println!(
                "SUCCESS! Captured frame with {} bytes ({}x{}, {} DPI)",
                bytes.len(),
                frame.width(),
                frame.height(),
                frame.dpi()
            );
            // Print some sample stats without dumping biometric image
            let non_zero = bytes.iter().filter(|&&b| b > 0).count();
            let avg = bytes.iter().map(|&b| b as u64).sum::<u64>() / bytes.len() as u64;
            println!(
                "Frame statistics: non-zero pixels = {non_zero}/{}, average luminance = {avg}",
                bytes.len()
            );
        }
        Err(err) => {
            println!("Sensor init or capture returned: {err}");
        }
    }

    Ok(())
}
