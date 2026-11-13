# HU20 USB Protocol Reverse-Engineering Record

## 1. Scope and Clean-Room Boundary

This document consolidates the clean-room USB protocol specification for the **SecuGen Hamster Pro 20 (HU20)** (`VID 0x1162`, `PID 0x2200`).

- **Zero proprietary dependencies:** `secugen-hu20-rs` never links to, loads, distributes, or invokes any vendor `.so`, `.dll`, `.sys`, or `.dat`/`.NEX` binary at compile time or runtime.
- **Out-of-scope hardware IDs:** `1162:2201` (UPx) and U20-A family PIDs (`1162:2220` U20-AP, `1162:2230` U20-ASF, `1162:2240` U20-A, `1162:2250` U20-ASFX) are explicitly excluded.
- **Evidence sources:** Static disassembly of the Linux x86_64 library (`libsgfdu05.so.1.0.2`), cross-corroborated against the Windows x64 driver/DLL (`sgfdu05x64.sys` / `sgfdu05x64.dll`), and physically validated against a `1162:2200` HU20 on macOS Apple Silicon.

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

Recovered from exhaustive disassembly of all 21 `CVxdInterface::VenderRequest` call sites in `libsgfdu05.so.1.0.2` and corroborated by `sgfdu05x64.dll` (IOCTL `0x222059` / `0x222014` passthrough to `sgfdu05x64.sys`):

| Function evidence | `bmRequestType` | `bRequest` | `wValue` | `wIndex` | Direction / Payload |
| --- | ---: | ---: | ---: | ---: | --- |
| `StartSendingImg(false)` | `0x40` | `0x01` | `0` | `0` | OUT, no payload |
| `StopSendingImg` | `0x40` | `0x02` | `0` | `0` | OUT, no payload |
| `StartSendingImg(true)` | `0x40` | `0x03` | `0` | `0` | OUT, no payload (high-speed capture) |
| `DumpFrame` | `0x40` | `0x05` | `1` | `0` | OUT, no payload |
| `WriteFWData` / `WriteLedData` / `WriteImageCoord` / `WriteIntgTime` | `0x40` | `0x08` | `0` or block | offset | OUT, variable payload to internal memory |
| `ReadFWData` / `ReadLedData` / `ReadImageCoord` / `ReadIntgTime` | `0xc0` | `0x09` | `0` or block | offset | IN, variable payload from internal memory |
| `SetLedData` | `0x40` | `0x0a` | `data[0..1]` | `data[2..3]` | OUT, no payload |
| `GetLedData` | `0xc0` | `0x0a` | `0` | `0` | IN, 4 bytes |
| `SetLedOn` | `0x40` | `0x11` | `0` or `1` | `0` | OUT, no payload (optical LED) |
| `SetIrLedOn` | `0x40` | `0x12` | `0` or `1` | `0` | OUT, no payload |
| `Reset` | `0x40` | `0x14` | `0` | `0` | OUT, no payload (transport reset) |
| `CheckLiveFinger` | `0xc0` | `0x16` | `0` | `0` | IN, 4 bytes |
| `SetSensorRegister` / `SetI2C` | `0x40` | `0x22` | `(mode << 8) \| 0x37` | reg | OUT, 1 byte (write CMOS register) |
| `GetSensorRegister` / `GetI2C` | `0xc0` | `0x22` | `(mode << 8) \| 0x37` | reg | IN, 1 byte (read CMOS register) |
| `ReadSerialNum` | `0xc0` | `0x25` | `0` | `0` | IN, 30 bytes (UTF-16LE device serial) |
| `WriteSerialNum` | `0x40` | `0x25` | `0` | `0` | OUT, 30 bytes (factory serial write) |
| `GetFakeDetectionLevel` | `0xc0` | `0x31` | `0` | `0` | IN, 1 byte |
| `SetExposureByCommand` | `0x40` | `0x40` | `0` | `0` | OUT, 4 bytes (**only** `PID > 0x2202`, unused on HU20) |
| `SetGainValueByCommand` | `0x40` | `0x41` | `0` | `0` | OUT, 4 bytes (**only** `PID > 0x2202`, unused on HU20) |
| `SetIdUsbLedOn` | `0x40` | `0xc0` | ID | `0` | OUT, no payload |

Internal firmware memory offsets used by `0x08` / `0x09`:
- `0x1c00`: LED data parameters (4 bytes)
- `0x1c04`: Registration / image coordinates (`tagRegPoints`, 32 bytes)
- `0x1c24`: Integration time (`WriteIntgTime`)
- `0x2000`: Calibration / firmware tables (`CFwData`, 4096-byte blocks)
- `0x0000..0xffff`: Background image / pixel compensation data (`WriteImageData`)

---

## 4. Initialization Handshake and CMOS Sensor (`CSIDO020A`)

### Open and Handshake Sequence
Disassembly of `CSdu05::OpenDevice` → `CSensorFactory::OpenDevice` → `CVxdInterface::OpenUSB` → `CVxdInterface::GetDeviceInfo`:
1. Select configuration `1` (only if not already active), claim interface `0`, and select altsetting `0` (Linux).
2. Read standard USB Device Descriptor (18 bytes).
3. Issue `0xc0/0x25` (`ReadSerialNum`, 30 bytes) before any sensor I²C traffic.
4. Write the 59 initialization registers (`SENSOR_REGISTERS`) followed by the 8 frame-window registers (`SetFrameWindowPos(250, 40, 1100, 800, true)` on `0x41`..`0x48`) to CMOS I²C slave address `0x37` via `bRequest = 0x22`.

### Mandatory I²C Readback Verification
Every register write (`0x40/0x22`) in `CVxdInterface::SetSensorRegister` is immediately followed by a readback (`0xc0/0x22`) checking `readback == written_value`:

```x86asm
callq SetI2C(slave_addr, reg, 1, &val, 0)
callq GetI2C(slave_addr, reg, 1, &readback, 0)
cmpb  %cl, %readback   ; mismatch fails the operation
```

### Exposure and Gain Control (`PID 0x2200`)
Disassembly of `CVxdInterface::SetExposure` and `CVxdInterface::SetGainValue` shows a branch on `PID > 0x2202`:
- Requests `0x40` and `0x41` are **not** used on HU20 (`0x2200`).
- **Exposure (16-bit integration time)** is written directly via I²C (`0x22`):
  - Register `0x30`: high byte (`exposure >> 8`)
  - Register `0x31`: low byte (`exposure & 0xff`)
- **Analog Gain (8-bit)** is written directly via I²C (`0x22`):
  - Register `0x32`: gain value (default `0x08`)

In `CSIDO020A::SetBrightnessTable`, the SDK builds an exposure table `5000 - (level * 50)`. At default brightness level `50`, exposure is `2500` (`0x09c4`), overriding the initial static table value `448` (`0x01c0`) which otherwise underexposes FTIR optical frames.

---

## 5. Native Sensor Geometry, Framing, and Image Processing

### True CMOS Matrix (`1100 × 800`)
Disassembly of `CSIDO020A::CSIDO020A`, `CSensorFactory::SetImageInfo`, and `CSIDO020A::ReadImage` proved that the sensor streams **1100 × 800 pixels (880,000 bytes)** over Bulk IN `0x82`:
- Interpreting the stream directly as `300` pixels wide causes a `1100 mod 300 = 200` pixel phase shear per row, destroying macroscopic contours.
- Reading the full `880,000`-byte stream with stride `1100` (`SensorFrame`) preserves the true optical prism geometry.

### Pixel Polarity (`CSIDO020A::DoModify`)
The CMOS sensor outputs inverted grayscale values. Every pixel is inverted with bitwise NOT (`!pixel` / `255 - pixel`) so ridges are dark and valleys are light:

```x86asm
1e0d0: notb (%rdx)
1e0d2: addq $1, %rdx
1e0d6: cmpq %rax, %rdx
1e0d9: jne  0x1e0d0
```

### Active Platen Crop, Feathering, and Resampling
- **Active optical prism window** in native `1100 × 800` coordinates: `X ∈ [140..960]` (width `820`), `Y ∈ [130..750]` (height `620`).
- **Bezel elimination (`SensorFrame::masked_platen`):** Applies an 18-pixel feathering taper to pure white (`255`) outside the active glass window to remove internal housing reflections without cropping the print.
- **Bilinear resampling (`resample_bilinear`):** Downscales/resamples the `820 × 620` active platen into standard `300 × 400` (`500 DPI`, `120,000` bytes) or `600 × 800` without scanline/moiré artifacts.

> **Note on `.NEX` / `.dat` vendor files:** Files such as `sgfdu05mlp.dat` and `libhu20_231m.so` are `.NEX` binary containers (magic `89 4e 45 58 0d 0a 1a 0a`, `GMId` chunk `300×400`) holding static geometric/compensation tables, not executable USB transport code.

---

## 6. Smart Capture™ (Closed-Loop Auto-Exposure & Gain)

Disassembly of `CFakeAlgo::GetSmartBrightness` (`0x11840`) and `check_new_brightness` (`0x12700`) showed closed-loop integration-time tuning for FTIR dynamics:
- **Light touch / dry finger** (`mean_touch > 130` or `delta < 25`): increases exposure (up to `EXPOSURE_MAX = 8000`) and gain (up to `GAIN_MAX = 0x28`).
- **Heavy press / wet finger** (`deep_black_pct > 8%` or `mean_touch < 75`, *ridge collapse*): decreases exposure (down to `EXPOSURE_MIN = 500`) and gain (down to `GAIN_MIN = 0x04`).
- Implemented in `SecuGenDevice::auto_tune_exposure()` and `SecuGenDevice::capture_smart_frame()`.

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

---

## 8. Appendix: `CVxdInterface` Virtual Method Table (`libsgfdu05.so.1.0.2`)

Located at VMA `0x00225c08`:

| Offset | Virtual Method | Recovered Protocol Behavior |
| ---: | --- | --- |
| `0x00` | `~CVxdInterface()` | Destructor |
| `0x08` | `~CVxdInterface()` | Deleting destructor |
| `0x10` | `Open(unsigned short, bool)` | Delegates to `OpenUSB` |
| `0x18` | `Close()` | Delegates to `CloseUSB` |
| `0x20` | `GetDeviceIdInfo(DEVICE_ID_INFO*)` | Copies device ID and geometry struct |
| `0x28` | `SetI2cParams(I2C_PARAMS*)` | Stores CMOS slave address (`0x37`) in offset `0xa0` |
| `0x30` | `SetTransferParams(TRANSFER_PARAMS*)` | Stores streaming chunk size (`0x12c00` = 76,800) in offset `0xa8` |
| `0x38` | `GetFakeDetectionLevel(int*)` | Request `0xc0/0x31`, 1 byte |
| `0x40` | `SetSensorRegister(unsigned char, unsigned char)` | Write `0x40/0x22`, then verify via `GetSensorRegister` |
| `0x48` | `GetSensorRegister(unsigned char, unsigned char*)` | Read `0xc0/0x22`, `wValue = (0 << 8) \| slave_addr`, `wIndex = reg` |
| `0x50` | `GetDeviceInfo()` | `GET_DESCRIPTOR` (18B) followed by `0xc0/0x25` (30B) |
| `0x58` | `ReadIntgTime(unsigned char*, int)` | Read integration time from firmware offset `0x1c24` |
| `0x60` | `ReadImageCoord(tagRegPoints*)` | Read calibration coordinates from firmware offset `0x1c04` |
| `0x68` | `ReadImageData(unsigned char*, int)` | Read image buffer |
| `0x70` | `ReadSerialNum(unsigned char*)` | Request `0xc0/0x25`, 30 bytes |
| `0x78` | `WriteSerialNum(unsigned char*, bool)` | Request `0x40/0x25`, 30 bytes |
| `0x80` | `CheckLiveFinger(bool*)` | Request `0xc0/0x16`, 4 bytes |
| `0x88` | `ReadData(unsigned char*, unsigned long, bool)` | Start → Read Bulk IN `0x82` → Stop |
| `0x90` | `StopSendingImg()` | Request `0x40/0x02`, no payload |
| `0x98` | `SetLedData(unsigned char*)` | Request `0x40/0x0a` |
| `0xa0` | `GetLedData(unsigned char*)` | Request `0xc0/0x0a`, 4 bytes |
| `0xa8` | `WriteLedData(unsigned char*, bool)` | Memory request `0x08`, offset `0x1c00` (4 bytes) |
| `0xb0` | `LockDevice()` | Mutex lock |
| `0xb8` | `UnlockDevice()` | Mutex unlock |
| `0xc0` | `IsLocked()` | Mutex state query |
| `0xc8` | `DumpFrame()` | Request `0x40/0x05`, `wValue = 1`, `wIndex = 0` |
| `0xd0` | `SetIrLedOn(bool)` | Request `0x40/0x12`, `wValue = bool`, `wIndex = 0` |
| `0xd8` | `SetGainValue(unsigned long, bool)` | For PID `0x2200`: writes CMOS reg `0x32` |
| `0xe0` | `SetExposure(unsigned long, bool)` | For PID `0x2200`: writes CMOS regs `0x30` (MSB) and `0x31` (LSB) |
| `0xe8` | `SetLedOn(bool)` | Request `0x40/0x11`, `wValue = bool`, `wIndex = 0` |
| `0xf0` | `IsWaking(bool*)` | Wake state query |
| `0xf8` | `WaitForWorkingState()` | Wait helper |
| `0x100` | `SetIdUsbLedOn(int)` | Request `0x40/0xc0` |
| `0x108` | `WriteFWData(int, unsigned char*, int, bool)` | Memory access `0x08`, chunked in 4096B |
| `0x110` | `WriteFWData(unsigned char*, int, bool)` | Overload with default offset |
| `0x118` | `ReadFWData(unsigned char*, int)` | Firmware data read |
| `0x120` | `Reset()` | Request `0x40/0x14`, no payload |
| `0x128` | `SetFrameWindowPos(int, int, int, int, bool)` | Sets crop window registers (`0x41`..`0x48`) |
| `0x130` | `YieldToWbf()` | Windows Biometric Framework yield helper |
| `0x138` | `IsWbf()` | WBF status check |
| `0x140` | `OpenUSB(unsigned short, bool)` | USB open, config, altsetting, and `GetDeviceInfo` |
| `0x148` | `CloseUSB()` | USB release interface and close |
| `0x150` | `VenderRequest(CRequestInfo*)` | `usb_control_msg` wrapper (2000 ms timeout) |
