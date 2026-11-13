# secugen-hu20-rs

Pure Rust, userspace USB driver for the SecuGen Hamster Pro 20 (HU20, USB VID `0x1162`, PID `0x2200`), with zero proprietary SDK, C library, or runtime dependencies.

Built entirely on `rusb` with clean-room protocol documentation and hardware validation.

---

## Features

- **Zero Proprietary Dependencies:** Pure Rust userspace implementation on top of `rusb`.
- **Targeted Hardware Filtering:** Safe discovery restricted strictly to `1162:2200` (HU20). Model `1162:2201` (UPx) is deliberately excluded.
- **Hardware Serial Number:** Reads the 30-byte factory serial number via vendor control transfer `0x25`.
- **Optical LED Control:** Controls the blue LED via vendor request `0x11`.
- **High-Speed Synchronous Capture:** Implements vendor request `0x03` on Bulk IN endpoint `0x82` streaming native 1100×800 CMOS sensor frames (880,000 bytes).
- **Closed-Loop Auto-Exposure:** Closed-loop feedback controller adjusting sensor integration time (I²C registers `0x30`, `0x31`) and gain (`0x32`) in real-time, preventing ridge collapse on heavy pressure and boosting contrast on light or dry touches.
- **Clean Framing & Platen Feathering:** Eliminates bezel moldura artifacts through edge feathering, preserving the natural field of view and biometric positioning.
- **Sub-Pixel Bilinear Resampling:** Produces smooth, scanline-free biometric portrait images in standard $300 \times 400$ (500 DPI) and high-definition $600 \times 800$.
- **Image Export:** Built-in zero-dependency export to binary PGM (`P5`) and grayscale PNG.

---

## Installation

Add the published crate to an application with Cargo:

```sh
cargo add secugen-hu20-rs
```

---

## Requirements

- `libusb-1.0` (native on macOS; package `libusb-1.0-0-dev` on Debian/Ubuntu).
- Linux users must configure udev permissions for `ATTRS{idVendor}=="1162", ATTRS{idProduct}=="2200"`.
- Never commit, log, or publish biometric frames or images (`.gitignore` protects `*.pgm`, `*.png`, `*.bmp`, `*.raw`).

---

## Quick Start & Examples

```bash
# 1. Enumerate connected SecuGen HU20 devices:
cargo run --example list_devices

# 2. Test optical LED:
cargo run --example led_probe

# 3. Read 30-byte hardware serial number:
cargo run --example serial_probe

# 4. Interactive Capture with Auto-Exposure:
cargo run --example capture_fingerprint

# 5. Interactive exposure and gain calibration sweep:
cargo run --example tune_exposure
```

---

## Quality Verification

```bash
make validate
```

Runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`.

Documentation:
- [Protocol Reverse Engineering Record](docs/PROTOCOL_REVERSE_ENGINEERING.md)
- [Hardware Validation Guide](docs/HARDWARE_VALIDATION.md)
- [Release Process](docs/RELEASING.md)
