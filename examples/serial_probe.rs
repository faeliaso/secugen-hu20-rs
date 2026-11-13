//! Probes the vendor SDK's serial-number query command (`0x25`) on a connected HU20.

use secugen_hu20_rs::SecuGenDevice;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = SecuGenDevice::open_single()?;
    println!("Opened HU20 at {}", scanner.info().id());

    println!("Querying hardware serial number via vendor control request 0x25...");
    match scanner.read_serial_number() {
        Ok(raw_serial) => {
            println!("Request 0x25 succeeded! Received 30 bytes:");
            print!("Hex: ");
            for byte in &raw_serial {
                print!("{:02x} ", byte);
            }
            println!();

            let ascii: String = raw_serial
                .iter()
                .map(|&b| {
                    if b.is_ascii_graphic() || b == b' ' {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            println!("ASCII representation: {}", ascii.trim());
        }
        Err(err) => {
            eprintln!("Serial query request 0x25 failed: {err}");
        }
    }

    Ok(())
}
