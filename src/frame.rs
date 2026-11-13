use crate::{ScannerError, FRAME_BYTES, SENSOR_DPI, SENSOR_HEIGHT, SENSOR_WIDTH};
use std::io::Write;
use std::path::Path;

/// A complete uncompressed 8-bit grayscale HU20 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFrame {
    data: Vec<u8>,
}

impl RawFrame {
    /// Validates and stores exactly one HU20 frame.
    pub fn new(data: Vec<u8>) -> Result<Self, ScannerError> {
        if data.len() == FRAME_BYTES {
            Ok(Self { data })
        } else {
            Err(ScannerError::IncompleteFrame {
                received: data.len(),
                expected: FRAME_BYTES,
            })
        }
    }

    /// Returns the image width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        SENSOR_WIDTH as u32
    }

    /// Returns the image height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        SENSOR_HEIGHT as u32
    }

    /// Returns the image resolution in DPI.
    #[must_use]
    pub const fn dpi(&self) -> u16 {
        SENSOR_DPI
    }

    /// Borrows the raw L8 pixels in row-major order.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Consumes the frame and returns its raw L8 pixels.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Returns a new `RawFrame` with linearly stretched contrast (normalized across 0..=255).
    ///
    /// The physical CSIDO020A sensor outputs raw uncalibrated ADC values in a narrow
    /// dynamic range (FTIR optical reflection). Normalizing the histogram expands the
    /// subtle contrast between papillary ridges and valleys to the full 8-bit range.
    #[must_use]
    pub fn stretched(&self) -> Self {
        let min = *self.data.iter().min().unwrap_or(&0);
        let max = *self.data.iter().max().unwrap_or(&255);
        if max <= min {
            return self.clone();
        }
        let range = (max - min) as u32;
        let stretched = self
            .data
            .iter()
            .map(|&p| {
                let scaled = (u32::from(p.saturating_sub(min)) * 255) / range;
                scaled as u8
            })
            .collect();
        Self { data: stretched }
    }

    /// Returns a new `RawFrame` with a 3x3 spatial smoothing filter applied.
    ///
    /// Smooths high-frequency CMOS ADC quantization noise while preserving
    /// biometric papillary ridge and valley continuity.
    #[must_use]
    pub fn smoothed(&self) -> Self {
        let w = self.width() as usize;
        let h = self.height() as usize;
        let mut out = vec![0_u8; w * h];

        for y in 0..h {
            let y_prev = y.saturating_sub(1);
            let y_next = (y + 1).min(h - 1);
            for x in 0..w {
                let x_prev = x.saturating_sub(1);
                let x_next = (x + 1).min(w - 1);

                let sum = u32::from(self.data[y_prev * w + x_prev])
                    + u32::from(self.data[y_prev * w + x]) * 2
                    + u32::from(self.data[y_prev * w + x_next])
                    + u32::from(self.data[y * w + x_prev]) * 2
                    + u32::from(self.data[y * w + x]) * 4
                    + u32::from(self.data[y * w + x_next]) * 2
                    + u32::from(self.data[y_next * w + x_prev])
                    + u32::from(self.data[y_next * w + x]) * 2
                    + u32::from(self.data[y_next * w + x_next]);

                out[y * w + x] = (sum / 16) as u8;
            }
        }

        Self { data: out }
    }

    /// Writes a binary PGM (`P5`) image.
    pub fn write_pgm(&self, path: impl AsRef<Path>) -> Result<(), ScannerError> {
        let mut file = std::fs::File::create(path).map_err(image_error)?;
        write!(file, "P5\n{} {}\n255\n", self.width(), self.height()).map_err(image_error)?;
        file.write_all(&self.data).map_err(image_error)
    }

    /// Writes an 8-bit grayscale PNG image.
    pub fn write_png(&self, path: impl AsRef<Path>) -> Result<(), ScannerError> {
        let file = std::fs::File::create(path).map_err(image_error)?;
        let mut encoder = png::Encoder::new(file, self.width(), self.height());
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(image_error)?;
        writer.write_image_data(&self.data).map_err(image_error)
    }
}

/// A complete uncompressed 1100x800 native CMOS frame from the HU20 CSIDO020A sensor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorFrame {
    data: Vec<u8>,
}

impl SensorFrame {
    /// Validates and stores exactly one native 1100x800 HU20 sensor frame.
    pub fn new(data: Vec<u8>) -> Result<Self, ScannerError> {
        if data.len() == crate::RAW_FRAME_BYTES {
            Ok(Self { data })
        } else {
            Err(ScannerError::IncompleteFrame {
                received: data.len(),
                expected: crate::RAW_FRAME_BYTES,
            })
        }
    }

    /// Returns the image width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        crate::RAW_SENSOR_WIDTH as u32
    }

    /// Returns the image height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        crate::RAW_SENSOR_HEIGHT as u32
    }

    /// Borrows the raw L8 pixels in row-major order.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Consumes the frame and returns its raw L8 pixels.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Returns a new `SensorFrame` with a 3x3 spatial smoothing filter applied.
    #[must_use]
    pub fn smoothed(&self) -> Self {
        let w = crate::RAW_SENSOR_WIDTH;
        let h = crate::RAW_SENSOR_HEIGHT;
        let mut out = vec![0_u8; w * h];

        for y in 0..h {
            let y_prev = y.saturating_sub(1);
            let y_next = (y + 1).min(h - 1);
            for x in 0..w {
                let x_prev = x.saturating_sub(1);
                let x_next = (x + 1).min(w - 1);

                let sum = u32::from(self.data[y_prev * w + x_prev])
                    + u32::from(self.data[y_prev * w + x]) * 2
                    + u32::from(self.data[y_prev * w + x_next])
                    + u32::from(self.data[y * w + x_prev]) * 2
                    + u32::from(self.data[y * w + x]) * 4
                    + u32::from(self.data[y * w + x_next]) * 2
                    + u32::from(self.data[y_next * w + x_prev])
                    + u32::from(self.data[y_next * w + x]) * 2
                    + u32::from(self.data[y_next * w + x_next]);

                out[y * w + x] = (sum / 16) as u8;
            }
        }

        Self { data: out }
    }

    /// Returns a new `SensorFrame` with the outer bezel (outside the optical prism glass)
    /// masked to uniform clean background (`255`) with a smooth 10-pixel feathering margin.
    #[must_use]
    pub fn masked_platen(&self) -> Self {
        let w = crate::RAW_SENSOR_WIDTH;
        let h = crate::RAW_SENSOR_HEIGHT;
        let mut out = self.data.clone();

        let x0 = crate::PLATEN_CROP_X0;
        let x1 = crate::PLATEN_CROP_X1;
        let y0 = crate::PLATEN_CROP_Y0;
        let y1 = crate::PLATEN_CROP_Y1;
        let margin = 12_usize;

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                if x < x0 || x >= x1 || y < y0 || y >= y1 {
                    out[idx] = 255;
                } else {
                    // Compute distance to boundary
                    let dist_x = (x - x0).min(x1 - 1 - x);
                    let dist_y = (y - y0).min(y1 - 1 - y);
                    let dist = dist_x.min(dist_y);
                    if dist < margin {
                        // Blend smoothly into 255 background
                        let alpha = dist as f64 / margin as f64;
                        let pixel = f64::from(self.data[idx]);
                        out[idx] = (pixel * alpha + 255.0 * (1.0 - alpha)).round() as u8;
                    }
                }
            }
        }

        Self { data: out }
    }

    /// Detects the fingerprint bounding box and center of mass within the active platen.
    ///
    /// Returns `Some((min_x, min_y, max_x, max_y, center_x, center_y))` if a finger touch is present.
    #[must_use]
    pub fn detect_finger(&self) -> Option<(usize, usize, usize, usize, usize, usize)> {
        let w = crate::RAW_SENSOR_WIDTH;
        let x0 = crate::PLATEN_CROP_X0;
        let x1 = crate::PLATEN_CROP_X1;
        let y0 = crate::PLATEN_CROP_Y0;
        let y1 = crate::PLATEN_CROP_Y1;

        let mut count = 0_usize;
        let mut sum_x = 0_u64;
        let mut sum_y = 0_u64;
        let mut min_x = x1;
        let mut max_x = x0;
        let mut min_y = y1;
        let mut max_y = y0;

        for y in y0..y1 {
            for x in x0..x1 {
                let val = self.data[y * w + x];
                // Bare glass has high luminance (>185). Skin contact lowers luminance.
                if val < 185 {
                    count += 1;
                    sum_x += x as u64;
                    sum_y += y as u64;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }

        // Require at least 5000 contact pixels for a valid touch
        if count >= 5_000 && max_x > min_x && max_y > min_y {
            let cx = (sum_x / count as u64) as usize;
            let cy = (sum_y / count as u64) as usize;
            Some((min_x, min_y, max_x, max_y, cx, cy))
        } else {
            None
        }
    }

    /// Renders a biometric portrait image in arbitrary dimensions (e.g. 300x400 or 600x800).
    ///
    /// Applies 3x3 spatial smoothing, soft optical platen feathering (eliminating bezel moldura),
    /// soft percentile contrast enhancement, and continuous bilinear resampling.
    #[must_use]
    pub fn render_biometric_portrait(&self, dst_w: usize, dst_h: usize) -> Vec<u8> {
        let smoothed = self.smoothed();

        // Use the full active optical platen window (820x620)
        let platen_w = crate::PLATEN_ACTIVE_WIDTH;
        let platen_h = crate::PLATEN_ACTIVE_HEIGHT;
        let x0 = crate::PLATEN_CROP_X0;
        let y0 = crate::PLATEN_CROP_Y0;

        let mut cropped = vec![255_u8; platen_w * platen_h];
        let feather = 18_usize;

        for cy in 0..platen_h {
            let sy = y0 + cy;
            for cx in 0..platen_w {
                let sx = x0 + cx;
                let val = smoothed.data[sy * crate::RAW_SENSOR_WIDTH + sx];

                // Distance to outer platen boundary
                let dist_x = cx.min(platen_w - 1 - cx);
                let dist_y = cy.min(platen_h - 1 - cy);
                let dist = dist_x.min(dist_y);

                if dist < feather {
                    // Soft fade out to clean white background at the very perimeter
                    let alpha = dist as f64 / feather as f64;
                    cropped[cy * platen_w + cx] =
                        (f64::from(val) * alpha + 255.0 * (1.0 - alpha)).round() as u8;
                } else {
                    cropped[cy * platen_w + cx] = val;
                }
            }
        }

        // Contrast optimization: percentile clipping (2%..98%) + gamma curve
        let mut hist = [0_u32; 256];
        for &b in &cropped {
            hist[b as usize] += 1;
        }
        let total = cropped.len() as u32;
        let mut acc = 0_u32;
        let mut p_low = 0_u8;
        let mut p_high = 255_u8;
        for (v, &cnt) in hist.iter().enumerate() {
            acc += cnt;
            if acc >= total * 2 / 100 && p_low == 0 {
                p_low = v as u8;
            }
            if acc >= total * 98 / 100 {
                p_high = v as u8;
                break;
            }
        }
        let p_range = f64::from(p_high.saturating_sub(p_low).max(1));
        let gamma = 1.15_f64;

        let enhanced: Vec<u8> = cropped
            .iter()
            .map(|&p| {
                let norm = (f64::from(p.saturating_sub(p_low)) / p_range).clamp(0.0, 1.0);
                (norm.powf(gamma) * 255.0).round() as u8
            })
            .collect();

        // Bilinear resampling to exact requested portrait dimensions
        resample_bilinear(&enhanced, platen_w, platen_h, dst_w, dst_h)
    }

    /// Converts the native sensor frame into a standard 300x400 biometric `RawFrame`.
    pub fn to_biometric_frame(&self) -> Result<RawFrame, ScannerError> {
        let pixels = self.render_biometric_portrait(crate::SENSOR_WIDTH, crate::SENSOR_HEIGHT);
        RawFrame::new(pixels)
    }

    /// Writes a binary PGM (`P5`) image of the full sensor frame.
    pub fn write_pgm(&self, path: impl AsRef<Path>) -> Result<(), ScannerError> {
        let mut file = std::fs::File::create(path).map_err(image_error)?;
        write!(file, "P5\n{} {}\n255\n", self.width(), self.height()).map_err(image_error)?;
        file.write_all(&self.data).map_err(image_error)
    }

    /// Writes an 8-bit grayscale PNG image of the full sensor frame.
    pub fn write_png(&self, path: impl AsRef<Path>) -> Result<(), ScannerError> {
        let file = std::fs::File::create(path).map_err(image_error)?;
        let mut encoder = png::Encoder::new(file, self.width(), self.height());
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(image_error)?;
        writer.write_image_data(&self.data).map_err(image_error)
    }
}

/// Resamples an 8-bit grayscale image using continuous sub-pixel bilinear interpolation.
#[must_use]
pub fn resample_bilinear(
    src: &[u8],
    src_w: usize,
    src_h: usize,
    dst_w: usize,
    dst_h: usize,
) -> Vec<u8> {
    let mut dst = vec![0_u8; dst_w * dst_h];
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return dst;
    }
    let scale_x = src_w as f64 / dst_w as f64;
    let scale_y = src_h as f64 / dst_h as f64;

    for dy in 0..dst_h {
        let gy = ((dy as f64 + 0.5) * scale_y - 0.5).clamp(0.0, (src_h - 1) as f64);
        let y0 = gy.floor() as usize;
        let y1 = (y0 + 1).min(src_h - 1);
        let fy = gy - y0 as f64;

        for dx in 0..dst_w {
            let gx = ((dx as f64 + 0.5) * scale_x - 0.5).clamp(0.0, (src_w - 1) as f64);
            let x0 = gx.floor() as usize;
            let x1 = (x0 + 1).min(src_w - 1);
            let fx = gx - x0 as f64;

            let p00 = f64::from(src[y0 * src_w + x0]);
            let p10 = f64::from(src[y0 * src_w + x1]);
            let p01 = f64::from(src[y1 * src_w + x0]);
            let p11 = f64::from(src[y1 * src_w + x1]);

            let top = p00 * (1.0 - fx) + p10 * fx;
            let bot = p01 * (1.0 - fx) + p11 * fx;
            let val = top * (1.0 - fy) + bot * fy;

            dst[dy * dst_w + dx] = val.round().clamp(0.0, 255.0) as u8;
        }
    }

    dst
}

fn image_error(error: impl core::fmt::Display) -> ScannerError {
    ScannerError::ImageExport(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::RawFrame;
    use crate::{ScannerError, FRAME_BYTES};

    #[test]
    fn accepts_exact_hu20_frame_size() {
        let frame = RawFrame::new(vec![0; FRAME_BYTES]).unwrap();
        assert_eq!(frame.width(), 300);
        assert_eq!(frame.height(), 400);
        assert_eq!(frame.as_bytes().len(), FRAME_BYTES);
    }

    #[test]
    fn rejects_short_frame() {
        assert!(matches!(
            RawFrame::new(vec![0; 12]),
            Err(ScannerError::IncompleteFrame { .. })
        ));
    }

    #[test]
    fn exports_valid_image_signatures() {
        let frame = RawFrame::new(vec![127; crate::FRAME_BYTES]).unwrap();
        let base = std::env::temp_dir().join(format!("secugen-hu20-rs-{}", std::process::id()));
        let pgm = base.with_extension("pgm");
        let png = base.with_extension("png");

        frame.write_pgm(&pgm).unwrap();
        frame.write_png(&png).unwrap();
        assert!(std::fs::read(&pgm)
            .unwrap()
            .starts_with(b"P5\n300 400\n255\n"));
        assert_eq!(&std::fs::read(&png).unwrap()[..8], b"\x89PNG\r\n\x1a\n");
        std::fs::remove_file(pgm).unwrap();
        std::fs::remove_file(png).unwrap();
    }

    #[test]
    fn stretched_normalizes_dynamic_range() {
        let mut data = vec![200; FRAME_BYTES];
        data[0] = 100;
        data[1] = 200;
        let frame = RawFrame::new(data).unwrap();
        let stretched = frame.stretched();
        assert_eq!(stretched.as_bytes()[0], 0);
        assert_eq!(stretched.as_bytes()[1], 255);
    }

    #[test]
    fn smoothed_attenuates_isolated_impulse() {
        let mut data = vec![100; FRAME_BYTES];
        let mid_idx = 200 * 300 + 150;
        data[mid_idx] = 200;
        let frame = RawFrame::new(data).unwrap();
        let smoothed = frame.smoothed();
        // The isolated peak (200) should be smoothed down significantly
        assert!(smoothed.as_bytes()[mid_idx] < 150);
        assert!(smoothed.as_bytes()[mid_idx] > 100);
    }

    #[test]
    fn sensor_frame_accepts_exact_raw_geometry() {
        use super::SensorFrame;
        use crate::RAW_FRAME_BYTES;

        let frame = SensorFrame::new(vec![240; RAW_FRAME_BYTES]).unwrap();
        assert_eq!(frame.width(), 1100);
        assert_eq!(frame.height(), 800);
        assert_eq!(frame.as_bytes().len(), RAW_FRAME_BYTES);
    }

    #[test]
    fn sensor_frame_masked_platen_sets_outer_bezel_to_pure_white() {
        use super::SensorFrame;
        use crate::RAW_FRAME_BYTES;

        let frame = SensorFrame::new(vec![50; RAW_FRAME_BYTES]).unwrap();
        let masked = frame.masked_platen();
        // Outside the platen (e.g. at pixel 10, 10), bezel must be pure 255
        assert_eq!(masked.as_bytes()[10 * 1100 + 10], 255);
    }

    #[test]
    fn sensor_frame_converts_to_clean_biometric_frame() {
        use super::SensorFrame;
        use crate::{RAW_FRAME_BYTES, SENSOR_HEIGHT, SENSOR_WIDTH};

        let mut data = vec![230; RAW_FRAME_BYTES];
        // Inject a simulated finger touch in the center of the platen
        for y in 350..450 {
            for x in 500..600 {
                data[y * 1100 + x] = 80;
            }
        }
        let frame = SensorFrame::new(data).unwrap();
        let bio = frame.to_biometric_frame().unwrap();
        assert_eq!(bio.width() as usize, SENSOR_WIDTH);
        assert_eq!(bio.height() as usize, SENSOR_HEIGHT);
    }

    #[test]
    fn resample_bilinear_handles_arbitrary_dimensions() {
        use super::resample_bilinear;

        let src = vec![100_u8; 10 * 10];
        let dst = resample_bilinear(&src, 10, 10, 20, 30);
        assert_eq!(dst.len(), 20 * 30);
        assert!(dst.iter().all(|&p| p == 100));
    }
}
