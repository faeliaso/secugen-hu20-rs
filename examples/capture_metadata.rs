use secugen_hu20_rs::SecuGenDevice;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = SecuGenDevice::open_single()?;
    let frame = scanner.capture_frame()?;
    println!(
        "capture succeeded: geometry={}x{} dpi={}",
        frame.width(),
        frame.height(),
        frame.dpi()
    );
    Ok(())
}
