use secugen_hu20_rs::{RawFrame, ScannerError, SecuGenDevice};
use std::io::{self, Write};

fn capture_raw_with_settings(
    scanner: &mut SecuGenDevice,
    exposure: u16,
    gain: u8,
) -> Result<RawFrame, ScannerError> {
    scanner.set_exposure(exposure)?;
    scanner.set_gain(gain)?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    scanner.dump_frame()?;
    scanner.capture_frame()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = SecuGenDevice::open_single()?;
    println!("Opened HU20 at {}", scanner.info().id());

    scanner.set_led(true)?;
    println!();
    println!("============================================================");
    println!("  VARREDURA DE EXPOSIÇÃO E GANHO (HU20 CSIDO020A)");
    println!("  1. Posicione o seu dedo firmemente sobre o prisma do sensor.");
    println!("  2. Mantenha o dedo imóvel e pressione [ENTER] para iniciar...");
    println!("============================================================");
    print!("> ");
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;

    println!("\nIniciando varredura...\n");
    println!(
        "{:<10} | {:<6} | {:<5} | {:<5} | {:<5} | {:<6} | Avaliação",
        "Exposição", "Ganho", "Min", "Max", "Méd", "Delta"
    );
    println!(
        "{:-<10}-+-{:-<6}-+-{:-<5}-+-{:-<5}-+-{:-<5}-+-{:-<6}-+-----------------------",
        "", "", "", "", "", ""
    );

    let exposures = [500, 1000, 1500, 2000, 2500, 3000, 3500, 4000, 4500, 5000];
    let gains = [0x08, 0x10, 0x18];

    let mut best_delta = 0;
    let mut best_exp = 2500;
    let mut best_gain = 0x08;
    let mut best_frame: Option<RawFrame> = None;

    for &gain in &gains {
        for &exp in &exposures {
            match capture_raw_with_settings(&mut scanner, exp, gain) {
                Ok(frame) => {
                    let bytes = frame.as_bytes();
                    let min = *bytes.iter().min().unwrap_or(&0);
                    let max = *bytes.iter().max().unwrap_or(&0);
                    let avg = bytes.iter().map(|&b| b as u64).sum::<u64>() / bytes.len() as u64;
                    let delta = max.saturating_sub(min);

                    let eval = if delta > 100 {
                        "★ EXCELENTE CONTRASTE"
                    } else if delta > 50 {
                        "✓ Bom contraste"
                    } else if delta > 20 {
                        "~ Contraste fraco"
                    } else {
                        "✗ Baixo contraste"
                    };

                    println!(
                        "{:<10} | 0x{:02x}   | {:<5} | {:<5} | {:<5} | {:<6} | {}",
                        exp, gain, min, max, avg, delta, eval
                    );

                    if delta > best_delta {
                        best_delta = delta;
                        best_exp = exp;
                        best_gain = gain;
                        best_frame = Some(frame);
                    }
                }
                Err(e) => {
                    println!("{:<10} | 0x{:02x}   | ERRO: {}", exp, gain, e);
                }
            }
        }
    }

    scanner.set_led(false)?;

    println!("\n============================================================");
    println!("Melhor configuração encontrada:");
    println!("  Exposição: {best_exp}");
    println!("  Ganho:     0x{best_gain:02x}");
    println!("  Delta:     {best_delta} níveis de cinza");
    println!("============================================================");

    if let Some(frame) = best_frame {
        if best_delta > 15 {
            let output = "best_fingerprint.pgm";
            frame.write_pgm(output)?;
            println!("Melhor frame raw salvo em: {output}");

            let contrast_frame = frame.stretched();
            let contrast_pgm = "best_fingerprint_contrast.pgm";
            contrast_frame.write_pgm(contrast_pgm)?;
            println!("Melhor frame normalizado salvo em: {contrast_pgm}");

            let contrast_png = "best_fingerprint_contrast.png";
            if let Ok(()) = contrast_frame.write_png(contrast_png) {
                println!("Melhor frame PNG salvo em: {contrast_png}");
            }
        }
    }

    Ok(())
}
