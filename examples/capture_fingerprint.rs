use secugen_hu20_rs::SecuGenDevice;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = SecuGenDevice::open_single()?;
    println!("Opened HU20 at {}", scanner.info().id());

    // Turn optical LED ON so user can see and align finger
    scanner.set_led(true)?;
    println!();
    println!("============================================================");
    println!("  LED AZUL DO LEITOR ACESO - SMART CAPTURE AUTO-EXPOSURE");
    println!("  1. Posicione o seu dedo suavemente no centro do prisma.");
    println!("     (Dica do manual SecuGen: use pressão leve, como segurar");
    println!("      uma folha de papel, para evitar colapso das cristas)");
    println!("  2. Pressione [ENTER] no terminal para capturar...");
    println!("============================================================");
    print!("> ");
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;

    println!("Executando Auto-Exposure em malha fechada...");
    let (tuned_exp, tuned_gain, tuned_delta) = scanner.auto_tune_exposure()?;
    println!(
        "✓ Auto-Exposure convergiu: Exposição = {tuned_exp}, Ganho = 0x{tuned_gain:02x}, Contraste Delta = {tuned_delta}"
    );

    println!("Capturando frame de alta precisão...");
    let sensor_frame = scanner.capture_sensor_frame_quick()?;
    scanner.set_led(false)?;

    if let Some((x0, y0, x1, y1, cx, cy)) = sensor_frame.detect_finger() {
        println!("✓ Dedo detectado: Área [{x0}..{x1}, {y0}..{y1}], Centro em ({cx}, {cy})");
    } else {
        println!(
            "! Aviso: Nenhum contato biométrico firme detectado, usando centro padrão da platina."
        );
    }

    // 1. Salvar formato biométrico padrão 300x400 (limpo, centralizado, sem bordas)
    let standard_file = "digital_biometrica_300x400.png";
    let bio_frame = sensor_frame.to_biometric_frame()?;
    bio_frame.write_png(standard_file)?;
    println!("✓ Salvo formato biométrico comercial padronizado (300x400) em: {standard_file}");

    // 2. Salvar formato biométrico em alta definição 600x800 (sem bordas, proporção anatômica 3:4)
    let hd_file = "digital_biometrica_alta_res_600x800.png";
    let hd_bytes = sensor_frame.render_biometric_portrait(600, 800);
    let file = std::fs::File::create(hd_file)?;
    let mut encoder = png::Encoder::new(file, 600, 800);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&hd_bytes)?;
    println!("✓ Salvo formato biométrico em alta resolução (600x800) em: {hd_file}");

    println!("\n============================================================");
    println!("  CAPTURA BIOMÉTRICA CONCLUÍDA COM SUCESSO!");
    println!("  Visualize os resultados limpos e sem borda no macOS:");
    println!("    open digital_biometrica_300x400.png");
    println!("    open digital_biometrica_alta_res_600x800.png");
    println!("============================================================");

    Ok(())
}
