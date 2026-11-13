# HU20 USB Protocol Specification

## 1. Scope

This document describes the clean-room USB protocol and image-processing pipeline for the **SecuGen Hamster Pro 20 (HU20)** (`VID 0x1162`, `PID 0x2200`).

- **Zero proprietary dependencies:** `secugen-hu20-rs` operates exclusively over `rusb` in userspace with no external C libraries, kernel modules, or proprietary runtime files.
- **Out-of-scope hardware IDs:** `1162:2201` (UPx) and U20-A family PIDs (`1162:2220`, `1162:2230`, `1162:2240`, `1162:2250`) are excluded.

---

## 2. USB Descriptor and Transport Layout

| Property | Value | Notes |
| --- | --- | --- |
| Vendor ID (`idVendor`) | `0x1162` | SecuGen |
| Product ID (`idProduct`) | `0x2200` | Hamster Pro 20 (HU20) |
| Configuration | `1` | Skip `set_active_configuration(1)` if already `1` (macOS `IOUSBHost` resets EP0 pipes on redundant selection) |
| Interface | `0` | Detach kernel driver on Linux if active |
| Alternate Setting | `0` | Select explicitly on Linux; omit redundant call on macOS (libusb reports `Other` error) |
| Bulk IN Endpoint | `0x82` | Max packet size: `512` bytes |
| Control Endpoint | `EP0` | Standard + Vendor requests (`2000 ms` timeout) |
| Interrupt Endpoint | None | No interrupt-transfer path is used |

---

## 3. Vendor Control Requests (`EP0`)

| Operation | `bmRequestType` | `bRequest` | `wValue` | `wIndex` | Direction / Payload |
| --- | ---: | ---: | ---: | ---: | --- |
| Start image stream (standard) | `0x40` | `0x01` | `0` | `0` | OUT, no payload |
| Stop image stream | `0x40` | `0x02` | `0` | `0` | OUT, no payload |
| Start image stream (high-speed) | `0x40` | `0x03` | `0` | `0` | OUT, no payload |
| Dump / flush frame buffer | `0x40` | `0x05` | `1` | `0` | OUT, no payload |
| Write internal memory | `0x40` | `0x08` | `0` or block | offset | OUT, variable payload |
| Read internal memory | `0xc0` | `0x09` | `0` or block | offset | IN, variable payload |
| Set LED parameters | `0x40` | `0x0a` | `data[0..1]` | `data[2..3]` | OUT, no payload |
| Get LED parameters | `0xc0` | `0x0a` | `0` | `0` | IN, 4 bytes |
| Set optical LED | `0x40` | `0x11` | `0` or `1` | `0` | OUT, no payload |
| Set IR LED | `0x40` | `0x12` | `0` or `1` | `0` | OUT, no payload |
| Reset transport | `0x40` | `0x14` | `0` | `0` | OUT, no payload |
| Check live finger | `0xc0` | `0x16` | `0` | `0` | IN, 4 bytes |
| Write CMOS sensor register (I²C) | `0x40` | `0x22` | `(mode << 8) \| 0x37` | reg | OUT, 1 byte |
| Read CMOS sensor register (I²C) | `0xc0` | `0x22` | `(mode << 8) \| 0x37` | reg | IN, 1 byte |
| Read serial number | `0xc0` | `0x25` | `0` | `0` | IN, 30 bytes (UTF-16LE device serial) |
| Write serial number | `0x40` | `0x25` | `0` | `0` | OUT, 30 bytes (factory serial write) |
| Get fake-detection level | `0xc0` | `0x31` | `0` | `0` | IN, 1 byte |
| Set exposure by command | `0x40` | `0x40` | `0` | `0` | OUT, 4 bytes (**only** `PID > 0x2202`, unused on HU20) |
| Set gain by command | `0x40` | `0x41` | `0` | `0` | OUT, 4 bytes (**only** `PID > 0x2202`, unused on HU20) |
| Set USB ID LED | `0x40` | `0xc0` | ID | `0` | OUT, no payload |

Internal firmware memory offsets used by `0x08` / `0x09`:
- `0x1c00`: LED data parameters (4 bytes)
- `0x1c04`: Registration / image coordinates (32 bytes)
- `0x1c24`: Integration time
- `0x2000`: Calibration / firmware tables (4096-byte blocks)
- `0x0000..0xffff`: Background image / pixel compensation data

---

## 4. Initialization Handshake and CMOS Sensor (`CSIDO020A`)

### Open and Handshake Sequence
1. Select configuration `1` (only if not already active), claim interface `0`, and select altsetting `0` (Linux).
2. Read standard USB Device Descriptor (18 bytes).
3. Issue `0xc0/0x25` (read 30-byte serial number) before any sensor I²C traffic.
4. Write the 59 initialization registers (`SENSOR_REGISTERS`) followed by the 8 frame-window registers (`0x41`..`0x48` configuring `1100 × 800` capture window at `(250, 40)`) to CMOS I²C slave address `0x37` via `bRequest = 0x22`.

### Mandatory I²C Readback Verification
Every register write (`0x40/0x22`) must be immediately followed by a readback (`0xc0/0x22`) verifying `readback == written_value`. Writing without readback verification can leave the sensor in an uninitialized state.

### Exposure and Gain Control (`PID 0x2200`)
For HU20 (`PID 0x2200 <= 0x2202`), commands `0x40` and `0x41` do not apply:
- **Exposure (16-bit integration time)** is written directly via I²C (`0x22`):
  - Register `0x30`: high byte (`exposure >> 8`)
  - Register `0x31`: low byte (`exposure & 0xff`)
- **Analog Gain (8-bit)** is written directly via I²C (`0x22`):
  - Register `0x32`: gain value (default `0x08`)

The exposure curve follows `5000 - (level * 50)`. At default brightness level `50`, exposure is `2500` (`0x09c4`), overriding the initial static table value `448` (`0x01c0`) which otherwise underexposes FTIR optical frames.

---

## 5. Native Sensor Geometry, Framing, and Image Processing

### True CMOS Matrix (`1100 × 800`)
The CMOS sensor streams **1100 × 800 pixels (880,000 bytes)** over Bulk IN `0x82`:
- Interpreting the stream directly as `300` pixels wide causes a `1100 mod 300 = 200` pixel phase shear per row, destroying macroscopic contours.
- Reading the full `880,000`-byte stream with stride `1100` (`SensorFrame`) preserves the true optical prism geometry.

### Pixel Polarity
The CMOS sensor outputs inverted grayscale values. Every pixel is inverted with bitwise NOT (`!pixel` / `255 - pixel`) so ridges are dark and valleys are light.

### Active Platen Crop, Feathering, and Resampling
- **Active optical prism window** in native `1100 × 800` coordinates: `X ∈ [140..960]` (width `820`), `Y ∈ [130..750]` (height `620`).
- **Bezel elimination (`SensorFrame::masked_platen`):** Applies an 18-pixel feathering taper to pure white (`255`) outside the active glass window to remove internal housing reflections without cropping the print.
- **Bilinear resampling (`resample_bilinear`):** Downscales/resamples the `820 × 620` active platen into standard `300 × 400` (`500 DPI`, `120,000` bytes) or `600 × 800` without scanline/moiré artifacts.

---

## 6. Smart Capture™ (Closed-Loop Auto-Exposure & Gain)

Closed-loop integration-time tuning adapts to FTIR contact dynamics:
- **Light touch / dry finger** (`mean_touch > 130` or `delta < 25`): increases exposure (up to `EXPOSURE_MAX = 8000`) and gain (up to `GAIN_MAX = 0x28`).
- **Heavy press / wet finger** (`deep_black_pct > 8%` or `mean_touch < 75`, *ridge collapse*): decreases exposure (down to `EXPOSURE_MIN = 500`) and gain (down to `GAIN_MIN = 0x04`).
- Implemented in `SecuGenDevice::auto_tune_exposure()` and `SecuGenDevice::capture_smart_frame()`, converging in `< 250 ms`.

---

## 7. Physical Validation Summary (macOS Apple Silicon)

Validated on a physical `1162:2200` HU20 scanner:

| Step / Command | Parameters | Result | Observation |
| --- | --- | :---: | --- |
| Optical LED (`0x11`) | `0x40/0x11`, `wValue=1` then `0` | **Pass** | LED illuminated cleanly and turned off on `Drop` / command. |
| Serial Query (`0x25`) | `0xc0/0x25`, `wLength=30` | **Pass** | Returned 30-byte UTF-16LE serial string. |
| CMOS I²C Read (`0x22`) | `0xc0/0x22`, `wValue=0x0037`, `wIndex=0x03` | **Pass** | Register `0x03` read returned `0x02`. |
| CMOS I²C Write + Verify | `0x40/0x22` followed by `0xc0/0x22` | **Pass** | Immediate readback matched written value. |
| Full Sensor Initialization | 59 registers + 8 window registers (`0x41`..`0x48`) | **Pass** | All registers written and verified without timeout. |
| Image Capture Stream | `0x40/0x01` or `0x03` → Bulk IN `0x82` → `0x40/0x02` | **Pass** | Acquired `120,000` bytes (`300×400`) and `880,000` bytes (`1100×800`). |
| Polarity + Smart Capture | `!pixel` + `auto_tune_exposure()` + `masked_platen()` | **Pass** | Clean biometric `300×400` and `600×800` frames with distinct ridges/valleys. |
