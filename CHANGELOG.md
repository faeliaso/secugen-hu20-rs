# Changelog

All notable changes to this project are documented in this file.

## Unreleased

## 0.1.1 - 2026-11-13

### Added

- `Installation` section with `cargo add secugen-hu20-rs` in `README.md`.

## 0.1.0 - 2026-11-13

### Added

- Pure-Rust userspace USB driver (`rusb`) for the SecuGen Hamster Pro 20 (HU20, `1162:2200`).
- Non-exclusive USB discovery (`list_devices`, `DeviceId`, `DiscoveredDevice`).
- Clean-room protocol implementation (`src/protocol.rs`) and RAII device handle (`SecuGenDevice`) with LED control, serial number read, sensor initialization over I²C (`0x22`), exposure/gain tuning, and closed-loop auto-exposure (`auto_tune_exposure`, `capture_smart_frame`).
- Validated frame types (`RawFrame` 300×400 and `SensorFrame` 1100×800) with contrast stretching, 3×3 smoothing, bilinear resampling, and PGM/PNG export.
- Hardware validation guide, reverse-engineering record, release checklist, examples, unit/integration tests, `Makefile`, and GitHub Actions workflows (`ci.yml`, `release-check.yml`, `publish.yml`).
