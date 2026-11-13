use secugen_hu20_rs::SecuGenDevice;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = SecuGenDevice::open_single()?;
    println!("Opened HU20 at {}", scanner.info().id());

    scanner.set_exposure(5000)?;
    scanner.set_gain(0x18)?;

    // 1. Capture with LED OFF
    println!("1. Desligando LED e capturando (LED OFF)...");
    scanner.set_led(false)?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    let frame_off = scanner.capture_frame_raw()?;
    let bytes_off = frame_off.as_bytes();
    let min_off = *bytes_off.iter().min().unwrap();
    let max_off = *bytes_off.iter().max().unwrap();
    let avg_off = bytes_off.iter().map(|&b| b as u64).sum::<u64>() / bytes_off.len() as u64;
    println!("   LED OFF: min={min_off}, max={max_off}, avg={avg_off}");

    // 2. Capture with LED ON
    println!("2. Ligando LED e capturando (LED ON)...");
    scanner.set_led(true)?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    let frame_on = scanner.capture_frame_raw()?;
    let bytes_on = frame_on.as_bytes();
    let min_on = *bytes_on.iter().min().unwrap();
    let max_on = *bytes_on.iter().max().unwrap();
    let avg_on = bytes_on.iter().map(|&b| b as u64).sum::<u64>() / bytes_on.len() as u64;
    println!("   LED ON:  min={min_on}, max={max_on}, avg={avg_on}");

    let diff = avg_on as i64 - avg_off as i64;
    println!("   Diferença (ON - OFF): {diff} níveis");

    let mut changed = 0;
    for (a, b) in bytes_off.iter().zip(bytes_on.iter()) {
        if a.abs_diff(*b) > 5 {
            changed += 1;
        }
    }
    println!("   Pixels com mudança > 5: {changed} / 120000");

    scanner.set_led(false)?;
    Ok(())
}
