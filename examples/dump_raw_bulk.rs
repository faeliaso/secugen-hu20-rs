use secugen_hu20_rs::SecuGenDevice;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = SecuGenDevice::open_single()?;
    println!("Opened HU20 at {}", scanner.info().id());

    scanner.set_exposure(5000)?;
    scanner.set_gain(0x18)?;

    println!("Lendo stream completo do Bulk IN (até 1.000.000 bytes)...");
    let stream = scanner.read_full_bulk_stream(1_000_000)?;
    println!("Total de bytes recebidos do Bulk IN: {}", stream.len());

    if stream.len() >= 880_000 {
        println!("CONFIRMADO! O sensor enviou 880.000 bytes (1100x800)!");
        std::fs::write("raw_sensor_880k.bin", &stream[..880_000])?;
        println!("Salvo raw_sensor_880k.bin (880.000 bytes)");
    } else {
        println!("Stream recebido: {} bytes", stream.len());
        std::fs::write("raw_sensor.bin", &stream)?;
    }

    Ok(())
}
