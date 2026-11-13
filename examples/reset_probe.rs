//! Issues the vendor SDK's transport-reset command to a connected HU20.

use secugen_hu20_rs::SecuGenDevice;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = SecuGenDevice::open_single()?;
    println!("opened HU20 at {}", scanner.info().id());
    scanner.reset()?;
    println!("transport reset completed; wait briefly before the next capture.");
    Ok(())
}
