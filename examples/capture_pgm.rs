use secugen_hu20_rs::{SecuGenDevice, DEFAULT_EXPOSURE, DEFAULT_GAIN};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let output = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "hu20-frame.pgm".to_string());
    let exposure: u16 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_EXPOSURE);
    let gain: u8 = args
        .get(3)
        .and_then(|s| {
            if let Some(hex) = s.strip_prefix("0x") {
                u8::from_str_radix(hex, 16).ok()
            } else {
                s.parse().ok()
            }
        })
        .unwrap_or(DEFAULT_GAIN);

    let mut scanner = SecuGenDevice::open_single()?;
    println!("Opened HU20 at {}", scanner.info().id());

    // Configure sensor settings
    scanner.set_exposure(exposure)?;
    scanner.set_gain(gain)?;
    println!("Sensor configurado: Exposição = {exposure}, Ganho = 0x{gain:02x}");

    // Turn LED on so the optical prism is illuminated
    scanner.set_led(true)?;
    println!();
    println!("=======================================================");
    println!("  LED ACESO!");
    println!("  1. Posicione o seu dedo firmemente sobre o leitor.");
    println!("  2. Pressione [ENTER] no terminal para capturar...");
    println!("=======================================================");
    print!("> ");
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;

    println!("Capturando...");
    let frame = scanner.capture_frame()?;
    frame.write_pgm(&output)?;
    println!(
        "Salvo frame raw {}x{} PGM em {output}",
        frame.width(),
        frame.height()
    );

    // Save contrast-stretched version for easy human viewing
    let contrast_frame = frame.stretched();
    let contrast_output = if let Some(stripped) = output.strip_suffix(".pgm") {
        format!("{stripped}_contrast.pgm")
    } else {
        format!("{output}_contrast.pgm")
    };
    contrast_frame.write_pgm(&contrast_output)?;
    println!("Salvo frame normalizado (auto-contraste) em {contrast_output}");

    // Also export PNG for instant preview in macOS Finder / Preview.app
    let png_output = if let Some(stripped) = output.strip_suffix(".pgm") {
        format!("{stripped}_contrast.png")
    } else {
        format!("{output}_contrast.png")
    };
    if let Ok(()) = contrast_frame.write_png(&png_output) {
        println!("Salvo frame em PNG (alta visibilidade) em {png_output}");
    }

    let bytes = frame.as_bytes();
    let min = *bytes.iter().min().unwrap_or(&0);
    let max = *bytes.iter().max().unwrap_or(&0);
    let avg = bytes.iter().map(|&b| b as u64).sum::<u64>() / bytes.len() as u64;
    let delta = max.saturating_sub(min);

    println!("Estatísticas do frame: min={min}, max={max}, avg={avg}, delta={delta}");
    if delta > 80 {
        println!("★ EXCELENTE CONTRASTE! Cristas e vales nítidos na imagem capturada.");
    } else if delta > 40 {
        println!("✓ Bom contraste detectado (delta = {delta} níveis de cinza).");
    } else {
        println!(
            "Aviso: Variação de luminância baixa (delta = {delta}). Use 'cargo run --example tune_exposure' para encontrar a exposição ideal."
        );
    }

    Ok(())
}
