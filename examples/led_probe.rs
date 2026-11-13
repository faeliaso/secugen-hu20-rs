use secugen_hu20_rs::SecuGenDevice;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = SecuGenDevice::open_single()?;
    println!("opened HU20 at {}", scanner.info().id());
    scanner.set_led(true)?;
    println!("LED on for three seconds; confirm it is illuminated.");
    std::thread::sleep(std::time::Duration::from_secs(3));
    scanner.set_led(false)?;
    println!("LED off.");
    Ok(())
}
