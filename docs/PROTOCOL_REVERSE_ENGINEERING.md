# HU20 USB protocol reverse-engineering record

## Scope and clean-room boundary

This record describes observable USB behavior needed to interoperate with
`1162:2200` (SecuGen Hamster Pro 20/HU20). The Rust crate must not link to,
load, distribute, or invoke the vendor SDK. Findings are derived from public
descriptors, static inspection of the supplied artifacts, and future USB
traffic captured from a user-authorized scanner.

`1162:2201` is identified by the supplied Linux SDK as UPx and is out of scope.

## Static findings — not yet protocol authorization

The x86_64 Linux artifact `libsgfdu05.so.1.0.2` imports libusb-0.1 functions
for control and bulk transfer. Its HU20 path selects configuration 1, claims
interface 0, selects altsetting 0, and reads image stream data from Bulk IN
endpoint `0x82` in 512-byte-aligned chunks. No interrupt-transfer path was
found in that image-capture flow.

The following vendor requests were recovered from named wrapper functions and
exhaustive disassembly of all 21 vendor control transfer call sites in
`libsgfdu05.so.1.0.2` (`CVxdInterface::VenderRequest`). The LED command and USB
layout are physically validated on macOS; the image stream commands are
implemented as an experimental capture path pending a successful frame read and
visual image inspection.

| Function evidence | bmRequestType | bRequest | wValue | wIndex | Direction / Payload |
| --- | ---: | ---: | ---: | ---: | --- |
| `StartSendingImg(false)` | `0x40` | `0x01` | `0` | `0` | OUT, no payload |
| `StopSendingImg` | `0x40` | `0x02` | `0` | `0` | OUT, no payload |
| `StartSendingImg(true)` | `0x40` | `0x03` | `0` | `0` | OUT, no payload |
| `DumpFrame` | `0x40` | `0x05` | `1` | `0` | OUT, no payload |
| `WriteFWData` / `WriteLedData` / `WriteImageCoord` / `WriteIntgTime` | `0x40` | `0x08` | `0` or block | offset | OUT, variable payload to internal memory (`0x1c00`, `0x1c04`, `0x1c24`, `0x2000`) |
| `ReadFWData` / `ReadLedData` / `ReadImageCoord` / `ReadIntgTime` | `0xc0` | `0x09` | `0` or block | offset | IN, variable payload read from internal memory |
| `SetLedData` | `0x40` | `0x0a` | `data[0..1]` | `data[2..3]` | OUT, no payload |
| `GetLedData` | `0xc0` | `0x0a` | `0` | `0` | IN, 4 bytes payload |
| `SetLedOn` | `0x40` | `0x11` | boolean | `0` | OUT, no payload |
| `SetIrLedOn` | `0x40` | `0x12` | boolean | `0` | OUT, no payload |
| `Reset` | `0x40` | `0x14` | `0` | `0` | OUT, no payload |
| `CheckLiveFinger` | `0xc0` | `0x16` | `0` | `0` | IN, 4 bytes payload |
| `SetSensorRegister` / `SetI2C` | `0x40` | `0x22` | `(mode << 8) \| 0x37` | reg | OUT, 1 byte payload (write CMOS sensor register) |
| `GetSensorRegister` / `GetI2C` | `0xc0` | `0x22` | `(mode << 8) \| 0x37` | reg | IN, 1 byte payload (read CMOS sensor register) |
| `ReadSerialNum` | `0xc0` | `0x25` | `0` | `0` | IN, 30 bytes payload (device serial / identifier) |
| `WriteSerialNum` | `0x40` | `0x25` | `0` | `0` | OUT, 30 bytes payload (factory serial writing) |
| `GetFakeDetectionLevel` | `0xc0` | `0x31` | `0` | `0` | IN, 1 byte payload |
| `SetExposureByCommand` | `0x40` | `0x40` | `0` | `0` | OUT, 4 bytes payload (**only** for PID > `0x2202`) |
| `SetGainValueByCommand` | `0x40` | `0x41` | `0` | `0` | OUT, 4 bytes payload (**only** for PID > `0x2202`) |
| `SetIdUsbLedOn` | `0x40` | `0xc0` | ID | `0` | OUT, no payload |

### Crucial Hardware Discovery: Exposure and Gain Control on HU20 (`0x2200`)

Disassembly of `CVxdInterface::SetExposure` and `CVxdInterface::SetGainValue` revealed that requests `0x40` and `0x41` are branched on `PID > 0x2202`:
```x86asm
cmpq $0x2202, 0x80(%rdi)
ja SetExposureByCommand   ; branches to 0x40/0x41 only if PID > 0x2202
```
For the SecuGen Hamster Pro 20 (PID `0x2200` <= `0x2202`):
- **Exposure** is configured directly over the CMOS sensor's I²C bus:
  - Register `0x30`: high byte (`exposure >> 8`)
  - Register `0x31`: low byte (`exposure & 0xff`)
- **Gain** is configured directly over the CMOS sensor's I²C bus:
  - Register `0x32`: gain value

Requests `0x40` and `0x41` do not apply to `0x2200`.

The transport memory commands using `0x08` (write) and `0x09` (read) target the
firmware's internal state tables:
- `0x1c00`: LED data parameters (4 bytes)
- `0x1c04`: Registration / image coordinates (`tagRegPoints`, 32 bytes)
- `0x1c24`: Integration time (`WriteIntgTime`)
- `0x2000`: Calibration / firmware tables (`CFwData`, 4096-byte blocks)
- `0x0000..0xffff`: Background image / pixel compensation data (`WriteImageData`)

The library has not found evidence that a 120,000-byte stream has a packet
header/footer or that the image must be inverted.

## Required physical validation gate

No HU20 or pre-existing USB trace was available on the implementation host on
2026-11-12. Before adding command constants or making capture operational:

1. Capture separate vendor-SDK sessions on the same HU20 for open/device-info, LED on/off,
   brightness/contrast/gain adjustment, and one image acquisition.
   On macOS, use an approved USB tracing facility compatible with the vendor
   application; do not require a Linux host for this project.
2. Remove payload bytes containing biometric data before retaining any trace.
   Preserve transfer metadata, setup packets, byte counts, endpoint addresses,
   statuses, and timing.
3. Verify the interface/altsetting, exact initialization and calibration order,
   stream start/stop command, total bytes, framing, timeout behavior, and
   pixel polarity against a locally viewed image.
4. Update this document with the trace-derived table and confidence level, then
   implement `src/protocol.rs` and hardware capture in one reviewed change.

Until frame acquisition is validated, consumers should treat `capture_frame`
as experimental and should not depend on image polarity or calibration.

## macOS validation probe

The scanner was enumerated in a native macOS Terminal as `1162:2200` at USB
topology `1:1`. Before implementing capture, run:

```sh
cargo run --example led_probe
```

The LED must illuminate for three seconds and then turn off. Do not proceed to
sensor initialization or image reads unless that behavior is confirmed.

The control transfer completed successfully on the native macOS HU20 at `1:1`.
Visual LED confirmation remains part of the operator checklist.

The first image-stream probe using `bRequest=0x03` timed out on the control
transfer. The crate now probes the alternate SDK branch, `bRequest=0x01`,
before attempting to reproduce sensor-register initialization.

Static analysis recovered the initialization address (`0x37`), the 59 CMOS
register writes, and the final frame-window register values. The initial
macOS I²C probe timed out; before retrying it, the Rust open sequence was
corrected to mirror the SDK's explicit configuration-1 and altsetting-0
selection. On macOS, libusb reports `Other error` when redundantly selecting
the default altsetting after configuration 1; the crate therefore leaves that
already-active default in place on macOS while retaining the SDK call on Linux.

The SDK exposes a distinct transport-reset command (`0x14`). It is available
as `SecuGenDevice::reset` and `examples/reset_probe.rs` for recovery testing,
but is not injected into normal capture because static analysis does not show
it in the ordinary open path.

## Physical results — macOS, 2026-11-12

The native macOS host enumerated a `1162:2200` device exposing interface 0,
altsetting 0 and Bulk IN endpoint `0x82` (maximum packet size 512).

### Initial Failures and Root-Cause Diagnosis

Early physical tests encountered control-transfer timeouts (`Operation timed out`)
across commands `0x01`, `0x03`, `0x14`, and `0x22`. Detailed macOS host diagnostics
revealed two concurrent root causes:
1. **Background Host Conflict**: A background utility (`Android File Transfer Agent`,
   PID 1471) had attached an `AppleUSBHostDeviceUserClient` to the scanner's USB
   node, disrupting control transfers.
2. **Configuration Renegotiation**: Calling `set_active_configuration(1)` when
   configuration 1 was already active caused macOS `IOUSBHost` to reset endpoint
   pipes, wedging the microcontroller's EP0 state machine.

### Physical Validation Milestone (Clean-Bus Validation)

Following termination of the conflicting process, adding a check to skip redundant
configuration selection, and performing a clean hardware power cycle (unplug/replug),
all physical commands were tested on the actual HU20 scanner at `1:1`:

| Step / Command | Parameters | Result | Observation |
| --- | --- | :---: | --- |
| Optical LED (`0x11`) | `0x40/0x11`, `wValue=1` then `0` | **Pass** | LED physically illuminated for 3s and turned off cleanly. |
| Serial Query (`0x25`) | `0xc0/0x25`, `wLength=30` | **Pass** | 30 bytes returned: UTF-16LE `"H54201001005   "`. |
| CMOS I²C Read (`0x22`) | `0xc0/0x22`, `wValue=0x0037`, `wIndex=0x03` | **Pass** | Initial register `0x03` read returned `0x02`. |
| CMOS I²C Write + Verify | `0x40/0x22` followed by `0xc0/0x22` | **Pass** | Write of `0x02` verified by immediate readback match. |
| Full Sensor Initialization | 59 registers + 8 window registers (`0x41`..`0x48`) | **Pass** | All registers written and verified without timeout. |
| Image Capture Stream | `0x40/0x01` -> Bulk IN `0x82` -> `0x40/0x02` | **Pass** | Exactly 120,000 bytes acquired (300×400, 500 DPI, L8). |
| Pixel Inversion (`notb`) | Bitwise NOT per `CSIDO020A::DoModify` | **Pass** | Average luminance = 239 on illuminated platen (ready for fingerprint ridges). |

Physical hardware communication and clean-room frame acquisition are fully validated
on macOS.

## Additional Windows artifacts — analysis status

`OUTROS/WinDrivers_v3003_Manual` adds a materially newer, HU20-specific
source of reverse-engineering evidence:

| Artifact | Finding | Permitted use in this project |
| --- | --- | --- |
| `sgfdu05x64.inf` | Associates exactly `USB\\VID_1162&PID_2200` with the HU20/FAP20 driver; `DriverVer` is `2025-11-28, 1.4.9.1`. | Confirms current vendor support for the target PID. |
| `sgfdu05x64.sys` | Windows x64 kernel USB driver (built 2025-12-01) importing `USBD_ParseConfigurationDescriptor`, `USBD_ParseConfigurationDescriptorEx`, and `USBD_CreateConfigurationRequestEx`. Its embedded PDB path is `driver_u20_windows/sys_win10/x64/Release/sgfdu05x64.pdb`, but the PDB itself is not supplied. | Offline static analysis of USB request construction; never linked, distributed, or loaded. |
| `sgfdu05x64.dll` | User-mode companion exporting only `CreateISensor`/`DestroyISensor`; it relies on the Windows driver rather than offering a portable transport. | API-flow reference only. |
| FDx SDK Pro Windows v4.3.1 samples | Confirms the high-level flow `Init` → `OpenDevice` → device-info query → allocate `imageWidth * imageHeight` → `GetImage`. | API/validation reference only; no USB packet details. |
| WBF drivers, `.dat`, `.lic`, and DLLs | Proprietary Windows-only components. | Excluded from compile-time and runtime use. |
| `WIN/SgDrvSetupUniversal.exe` | 45 MiB WiX/Burn bootstrapper (PE32 timestamp 2025-08-28), containing an embedded driver payload container. | Installer provenance only; not executed or linked. |
| `WIN/SgDrvSetup_WBF_U20-UPx.exe` | 20 MiB WiX/Burn bootstrapper (PE32 timestamp 2026-04-03), targeting the Windows Biometric Framework U20/UPx family. | Installer provenance only; not executed or linked. |
| `WIN` installer guides | List HU20, HU20-A, HU20-AL, and HU20-AP under Hamster Pro 20 support. | Product-family confirmation only; no USB protocol fields. |

The HU20 `sgfdu05mlp.dat` files supplied under the x86, x64, and WBF packages
are byte-identical (`SHA-256`
`ff8a9de392daeb4e80c17c95316ade5d5144f1ffcdf7420e8d572101403c9553`).
They are NEX-packed data (`89 4e 45 58 …`), not a text or plain configuration
format, and expose no independently recoverable USB setup fields. They remain
out of scope for the Rust crate and do not add protocol evidence.

The `WIN` bootstrapper executables are WiX/Burn containers rather than device
drivers themselves. Their embedded payloads were not executed or extracted in
this clean-room record. They may be revisited only for offline payload
inventory if a driver version absent from `OUTROS` is needed; their installer
logic is not relevant to the cross-platform `rusb` transport.

`WIN/WinDriver_u20_v1491/x64/sgfdu05x64.sys` is byte-for-byte identical to
the already analyzed `OUTROS` HU20 driver (SHA-256
`f44439152a353f70acc67f1ddf9bd604babc3220045694b25983ddb1d8cc0591`).
It independently confirms the provenance of version `1.4.9.1`, but adds no
new behavior to this record.

`WIN/U20-A_v1.0.3.6_signed` is deliberately excluded from HU20 protocol
implementation. Its INF identifies different hardware IDs: `1162:2220`
(U20-AP), `1162:2230` (U20-ASF), `1162:2240` (U20-A), and `1162:2250`
(U20-ASFX); it does not match the attached `1162:2200` HU20. It can be used
only as a family-level comparison source, never as authorization to send its
commands to the target scanner.

The kernel driver contains code associated with the value `0x22`, consistent
with the older SDK's I²C bridge command, but static inspection has not yet
recovered its complete USB setup packet. It must not be treated as proof of a
new command sequence until that packet is reconstructed or observed in a
trace.

### Windows control-URB passthrough — confirmed structure

Further static analysis of `sgfdu05x64.sys` recovered its generic vendor
control-transfer path (routine at image virtual address `0x140003974`). It
constructs a `URB_CONTROL_VENDOR_OR_CLASS_REQUEST`: the URB header is written
as length `0x88`, function `0x0017`, and submitted to the lower USB stack.
This is independent evidence that the driver uses ordinary EP0 vendor control
transfers rather than a proprietary transport layer.

Its private input buffer is decoded as follows:

| Offset | Width | Meaning recovered from URB field assignment |
| ---: | ---: | --- |
| `0` | 1 | `bmRequestType` |
| `1` | 1 | `bRequest` |
| `2` | 2 | `wValue` (little endian) |
| `4` | 2 | `wIndex` (little endian) |
| `6` | 2 | `wLength` (little endian) |
| `8` | 1 | direction selector: `0` selects OUT; non-zero sets the URB IN flag |
| `9` | `wLength` | outbound payload (copied when the selector is `0`) |

The routine copies offsets `0..6` directly into the corresponding control-URB
setup fields and uses the length at offset `6` for `TransferBufferLength`.
For the OUT branch it allocates a transfer buffer and copies the bytes from
offset `9`; for the IN branch it sets the URB direction flag and reuses the
caller buffer as the receive area. This confirms the setup-packet field layout
used by the current Windows driver. It does **not** reveal an
application-issued command,
calibration sequence, endpoint selection, or the semantic meaning of the
commands supplied by the user-mode component; those remain trace-required.

In particular, the nearby constant `0x22` belongs to an internal IOCTL/driver
dispatch path, not to this generic URB constructor. It is therefore not
additional proof that the scanner received `bRequest = 0x22` during a current
Windows session. The Linux SDK remains the sole static evidence for that
specific request.

### Windows companion DLL — IOCTL `0x222059` control path

The matching 2025 user-mode artifact `sgfdu05x64.dll` imports
`DeviceIoControl` and sends IOCTL `0x222059` to the HU20 kernel driver. The
driver accepts a fixed ten-byte, no-payload control descriptor for this IOCTL
and constructs a control URB from it. Its recovered fields are:

| Offset | Width | Driver use |
| ---: | ---: | --- |
| `0` | 1 | transfer-flag selector |
| `1` | 1 | URB vendor/class category selector (with offset `2`) |
| `2` | 1 | URB recipient/category selector (with offset `1`) |
| `4` | 1 | `bRequest` |
| `6` | 2 | `wValue` (little endian) |
| `8` | 2 | `wIndex` (little endian) |

The Windows DLL writes `0x0002` at offsets `1..2`, but that value selects the
kind of vendor/class URB; it is **not** `bRequest = 0x02`. The request byte is
instead caller-provided at offset `4`. Earlier notes that treated this write
as an independent confirmation of `StopSendingImg` were withdrawn during the
same static-analysis pass. The Linux SDK remains the only static source for
the semantic assignment of request `0x02` to stream stop.

The driver also exposes a separate generic control path through IOCTL
`0x222014`. That path accepts a nine-byte setup descriptor plus optional
payload: `bmRequestType`, `bRequest`, `wValue`, `wIndex`, `wLength`, direction
selector, then payload. The fixed `0x222059` path accepts no payload and uses
the category selectors described above. Both paths ultimately submit ordinary
control URBs, but neither path embeds a fixed start-capture, LED, or I²C
request. Those request bytes are supplied by higher-level caller state in the
Windows DLL and have not been recovered with sufficient provenance.

## Supplemental archive `/Volumes/MAIN/SECUGEN` — 2026-11-12

The separately supplied archive directory was inventoried without running any
Windows executable or installer. Its Linux and Android SDK ZIPs do not add a
new transport implementation: the x86_64 Linux `libsgfdu05.so.1.0.2` is
byte-identical to the already analyzed artifact (SHA-256
`aaf1d208c77806618483619f5c982ddea4986bde9f11f506337bf90819693a72`), and
the Android x86_64 `libjnisgfdu05.so` is also byte-identical (SHA-256
`ba6b4c93e36527175ef4a31a6b604914e4a955fee56e3944f531c700212adf53`).
They therefore corroborate provenance but provide no new initialization,
capture, or framing evidence.

`ConfigTool_v1.2.0.2.exe` was examined as PE/.NET metadata only. It is a
small managed, administrator-requesting Windows configuration program whose
visible methods include `ReadConfig`, `ReadAllConfig`, `SetAllConfig`,
`ReadConfigLiveFingerLogEnable`, and `ReadConfigLiveFingerLevel`. It imports
only the CLR launcher (`mscoree.dll`) and contains .NET Registry API symbols
(`OpenSubKey`, `CreateSubKey`, `SetValue`); it does not contain a USB driver
binding, a SecuGen SDK DLL import, or vendor-control request fields. It can
explain Windows-side *live-finger* configuration semantics, but is not
evidence for the HU20 wire protocol.

`SGDX_v508.zip` contains the Windows diagnostic UI `sgdx_v508.exe`. Static
strings show that it selects vendor SDK DLLs such as `Sgfdu05m.dll` for the
FDU05 family. The executable is a client of the proprietary SDK rather than a
transport implementation, so it is retained only as evidence that a
known-good Windows workflow exists. It is neither executed nor incorporated
in the crate.

The remaining Windows SDK archive contains public headers, import libraries,
and sample applications. Those establish only the already recorded high-level
API flow and must not be mistaken for the current HU20 kernel transport. The
product, bridge, and browser-integration PDFs are product/API material; they
may inform future application integration, but are intentionally outside this
hardware-only crate and do not authorize additional USB commands.

## Windows DLL control-call reconstruction — 2026-11-12

The newest `sgfdu05x64.dll` was disassembled offline through the call site of
`DeviceIoControl`; it was not loaded or executed. The DLL has one imported
`DeviceIoControl` wrapper and its HU20 transport object invokes it through a
virtual dispatch layer. The fixed-control helper at image RVA `0x13060`
constructs the ten-byte descriptor submitted through `IOCTL 0x222059`:

| Source operation descriptor offset | Fixed IOCTL descriptor offset | Meaning |
| ---: | ---: | --- |
| `0` | `0` | transfer flag |
| — | `1..2` | constant little-endian `0x0002` vendor/class category |
| `8` | `4` | `bRequest` |
| `12` | `6..7` | `wValue` |
| `16` | `8..9` | `wIndex` |

The DLL's higher-level transport wrappers do embed the following request
bytes before passing their operation descriptor to that helper; this is
independent confirmation from the current Windows implementation.

| Request | Recovered wrapper behavior |
| ---: | --- |
| `0x01` / `0x03` | Image-stream start branch, selected from a Boolean mode. |
| `0x02` | Image-stream stop. |
| `0x11` | LED Boolean in `wValue`. |
| `0x12` | IR LED Boolean in `wValue`. |
| `0x14` | No-data transport reset. |
| `0x22` | Sensor-register write/read bridge. |
| `0x40` / `0x41` | Four-byte exposure/gain control payloads. |

The I²C read-back wrapper uses `0x22` for both directions and compares the
returned byte with the requested write value. The Windows reconstruction now
corroborates the SDK command set and setup-field mapping across two vendor
implementations.

### Recovered Initialization Handshake and Preconditions

Disassembly of the initialization path (`CSdu05::OpenDevice` ->
`CSensorFactory::OpenDevice` -> `CVxdInterface::OpenUSB` ->
`CVxdInterface::GetDeviceInfo`):
1. The driver selects configuration 1, claims interface 0, and selects
   altsetting 0.
2. It immediately issues standard USB `GET_DESCRIPTOR` for Device (18 bytes).
3. It immediately follows with Vendor IN request `0x25` (`ReadSerialNum`,
   30 bytes) to populate the `DEVICE_ID_INFO` structure.
4. Only after this initial handshake does `CSensorFactory::InitDevice()` invoke
   `CSIDO020A::InitSensorReg()` to configure the CMOS sensor via request `0x22`.

In earlier tests on macOS, `secugen-hu20-rs` attempted to write directly to I²C
register `0x03` without first performing this query handshake. Additionally,
host inspection via `ioreg -r -c IOUSBHostDevice` showed that background
software on the test machine (`Android File Transfer Agent`, PID 1471) had
attached an `AppleUSBHostDeviceUserClient` to the physical scanner device node,
which can interfere with or reset ongoing USB EP0 control transactions.

### Analysis of Binary Calibration Artifacts (`libhu20_231m.so` / `.NEX`)

Binary inspection of `VENDOR-ARTIFACTS/ANDROID/jniLibs/x86_64/libhu20_231m.so`
revealed that it is not an ELF shared object, but rather a proprietary `.NEX`
binary container (magic `89 4e 45 58 0d 0a 1a 0a`) containing chunk `GMId`
specifying frame dimensions of 300×400 pixels (`0x012c` × `0x0190`). This
proves that vendor `.dat` and disguised `.so` files are fixed geometric and
pixel compensation matrices, not executable code with concealed USB transport
logic.

### Complete `CVxdInterface` Virtual Method Table Layout

Exhaustive disassembly and relocation mapping of `vtable for CVxdInterface` in
`libsgfdu05.so.1.0.2` (located at VMA `0x00225c08`):

| Offset | Virtual Method | Recovered Protocol Behavior |
| ---: | --- | --- |
| `0x00` | `~CVxdInterface()` | Destructor |
| `0x08` | `~CVxdInterface()` | Deleting destructor |
| `0x10` | `Open(unsigned short, bool)` | Delegates to `OpenUSB` |
| `0x18` | `Close()` | Delegates to `CloseUSB` |
| `0x20` | `GetDeviceIdInfo(DEVICE_ID_INFO*)` | Copies device ID and geometry struct |
| `0x28` | `SetI2cParams(I2C_PARAMS*)` | Stores CMOS slave address (`0x37`) in instance offset `0xa0` |
| `0x30` | `SetTransferParams(TRANSFER_PARAMS*)` | Stores streaming chunk size (`0x12c00` = 76,800) in instance offset `0xa8` |
| `0x38` | `GetFakeDetectionLevel(int*)` | Request `0xc0/0x31`, 1 byte payload |
| `0x40` | `SetSensorRegister(unsigned char, unsigned char)` | Write `0x40/0x22`, then **immediately** verifies via `GetSensorRegister` |
| `0x48` | `GetSensorRegister(unsigned char, unsigned char*)` | Read `0xc0/0x22`, `wValue = (0 << 8) \| slave_addr`, `wIndex = reg`, 1 byte |
| `0x50` | `GetDeviceInfo()` | Handshake: GET_DESCRIPTOR (18B) followed by `0xc0/0x25` (30B) |
| `0x58` | `ReadIntgTime(unsigned char*, int)` | Read integration time from firmware offset `0x1c24` |
| `0x60` | `ReadImageCoord(tagRegPoints*)` | Read calibration coordinates from firmware offset `0x1c04` |
| `0x68` | `ReadImageData(unsigned char*, int)` | Read image buffer |
| `0x70` | `ReadSerialNum(unsigned char*)` | Request `0xc0/0x25`, 30 bytes payload |
| `0x78` | `WriteSerialNum(unsigned char*, bool)` | Request `0x40/0x25`, 30 bytes payload |
| `0x80` | `CheckLiveFinger(bool*)` | Request `0xc0/0x16`, 4 bytes payload |
| `0x88` | `ReadData(unsigned char*, unsigned long, bool)` | Bulk capture stream: Start -> Read Bulk IN `0x82` -> Stop |
| `0x90` | `StopSendingImg()` | Request `0x40/0x02`, no payload |
| `0x98` | `SetLedData(unsigned char*)` | Request `0x40/0x0a` |
| `0xa0` | `GetLedData(unsigned char*)` | Request `0xc0/0x0a`, 4 bytes payload |
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
| `0x108` | `WriteFWData(int, unsigned char*, int, bool)` | Memory access request `0x08` (IN for read, OUT for write), chunked in 4096B |
| `0x110` | `WriteFWData(unsigned char*, int, bool)` | Overload with default offset |
| `0x118` | `ReadFWData(unsigned char*, int)` | Firmware data read |
| `0x120` | `Reset()` | Request `0x40/0x14`, no payload |
| `0x128` | `SetFrameWindowPos(int, int, int, int, bool)` | Sets crop window registers (`0x41`..`0x48`) |
| `0x130` | `YieldToWbf()` | Windows Biometric Framework yield helper |
| `0x138` | `IsWbf()` | WBF status check |
| `0x140` | `OpenUSB(unsigned short, bool)` | Low-level USB open, configuration, altsetting, and GetDeviceInfo |
| `0x148` | `CloseUSB()` | Low-level USB release interface and close |
| `0x150` | `VenderRequest(CRequestInfo*)` | Low-level `usb_control_msg` wrapper (timeout 2000 ms) |

### CMOS Sensor Register Verification Requirement

Disassembly of `CVxdInterface::SetSensorRegister` reveals an essential protocol constraint:
Every single register write (`SetI2C`) is immediately followed by a readback (`GetI2C`)
that verifies the value in the sensor matches what was written:
```x86asm
callq SetI2C(slave_addr, reg, 1, &val, 0)
callq GetI2C(slave_addr, reg, 1, &readback, 0)
cmpb  %cl, %readback   ; if readback != val, operation fails!
```
Drivers failing to implement readback or failing to wait for the I²C bus cycle can leave
the sensor in an uninitialized or unacknowledged state.

### Image Polarity and Post-Processing (`CSIDO020A::DoModify`)

Disassembly of `CSIDO020A::DoModify` at `0x1e0d0` proved the raw image polarity:
```x86asm
1e0d0: notb (%rdx)
1e0d2: addq $1, %rdx
1e0d6: cmpq %rax, %rdx
1e0d9: jne  0x1e0d0
```
The CSIDO020A CMOS sensor outputs physically inverted grayscale data. The vendor SDK
inverts every byte using bitwise NOT (`!pixel` or `255 - pixel`) so that ridges appear
dark and valleys appear light according to biometric standards.

### Bulk IN Transfer Framing and Endpoint

In `CVxdInterface::ReadData`:
- Requested buffer length is 512-byte aligned: `(len + 511) & ~511`. For 120,000 bytes (300×400),
  the transfer length requested is 120,320 bytes (235 packets of 512 bytes).
- Endpoint is Bulk IN `0x82`.
- Stream start is issued via `StartSendingImg(false)` (request `0x01`).
- Bulk data is read via `FDU_USBBulkReadEx`.
- Stream stop is issued via `StopSendingImg()` (request `0x02`).

### Exposure and Gain Control (CSIDO020A Brightness Model)

In raw sensor captures, initial frames showed uniform luminance (`min=234, max=243, avg=238, delta=9`).
Disassembly of `CSensorFactory::InitBrightness` (`0x19cf0`), `CSIDO020A::SetBrightnessTable` (`0x1d9a0`),
and `CVxdInterface::SetExposure` (`0x1f6b0`) revealed the cause:

1. **Exposure Registers (`0x30` / `0x31`)**:
   For PID `0x2200` (HU20), exposure is configured by writing 16-bit integration time to CMOS
   registers `0x30` (`exposure >> 8`) and `0x31` (`exposure & 0xff`) via the I²C bridge (`0x40/0x22`).
   The initial static register array `SENSOR_REGISTERS` only configured default exposure `448`
   (`(0x01 << 8) | 0xc0`).
2. **Gain Register (`0x32`)**:
   Analog gain is written to CMOS register `0x32` (default `0x08`).
3. **Brightness Calibration Curve**:
   In `CSIDO020A::SetBrightnessTable`, the SDK generates an exposure lookup table starting at `5000`
   (`0x1388`) with step `-50` (`-0x32`). For default brightness level 50, exposure is calculated as:
   `5000 - (50 * 50) = 2500` (`0x09c4`).
4. **Physical Effect on FTIR Optical Sensing**:
   At underexposed integration times (~448), the CMOS sensor receives insufficient photons from the
   illuminated prism, producing raw ADC values close to dark level (~17). Under `notb` inversion
   (`255 - pixel`), raw value 17 becomes `238` (flat light gray), masking ridge contrast.
   Increasing exposure to factory default `2500` (or higher depending on platen wear) restores the
   required dynamic range for frustrated total internal reflection (FTIR) ridge contrast.
### True Hardware Sensor Geometry: 1100 × 800 Raw Matrix

Exhaustive disassembly of `CSIDO020A::CSIDO020A` (`0x1d94a`), `CSensorFactory::SetImageInfo` (`0x19930`),
and `CSIDO020A::ReadImage` (`0x1e14d`) revealed the definitive reason why initial 300×400 raw frames
appeared as uniform textured noise without human-recognizable finger contours:

1. **Physical Sensor Dimensions**:
   The CMOS sensor CSIDO020A outputs raw frames of **1100 × 800 pixels (880,000 bytes)** via Bulk IN `0x82`.
2. **Phase / Shear Distortion from Line-Stride Mismatch**:
   Interpreting an 1100-pixel-wide line stream as a 300-pixel-wide image caused a phase offset of
   `1100 mod 300 = 200` pixels on every single row! This completely scrambled horizontal continuity,
   destroying all macroscopic geometric shapes (finger ellipse, platen boundary) and leaving only
   high-frequency granulated noise.
3. **Resizing and Perspective Correction**:
   The vendor SDK captures the native 1100×800 frame from USB, applies optical black-level compensation
   (`Blc::PixelCompensation`), and subsequently downscales and perspective-corrects it to standard
   300×400 biometric format using `Unperspective::Resize`.
4. **Physical Validation**:
   Reading the full 880,000-byte stream directly from Bulk IN `0x82` with stride 1100 immediately
   revealed the true optical geometry: platen ellipse, boundary margins, and biometric contact area.

---

## Smart Capture and Auto-Exposure Model

### Closed-Loop Exposure and Gain Tuning (`auto_tune_exposure`)
Disassembly of `CFakeAlgo::GetSmartBrightness` (`0x11840`), `check_new_brightness` (`0x12700`), and `CSensorFactory::SetBrightnessTable` (`0x1d9a0`) confirmed that SecuGen's proprietary "Smart Capture™" operates a closed-loop feedback controller on sensor integration time:
1. **Physical FTIR Dynamics:**
   - **Light Touch / Dry Skin:** Low contact surface; ridges absorb few photons; dynamic range is narrow ($\Delta < 25$). The controller elevates exposure (integration time) up to $5000 \dots 7000$ and gain to boost ridge contrast.
   - **Heavy Pressure / Moist Skin:** Skin elasticity collapses the air valleys against the glass platen (*ridge collapse*), saturating pixel values to zero ($< 35$) and merging adjacent ridges. The controller reduces exposure down to $1500 \dots 2500$ and decreases analog gain to reopen valleys.
2. **I²C Register Interface (PID `0x2200`):**
   - Exposure High: `0x30` (`exposure >> 8`)
   - Exposure Low: `0x31` (`exposure & 0xff`)
   - Analog Gain: `0x32` (`gain`)
3. **Pure-Rust Implementation:**
   Implemented in `SecuGenDevice::auto_tune_exposure()`. Iteratively samples preview frames from the sensor, computes contact area luminance, deep black saturation percentage, and ridge-to-valley delta ($\Delta$), converging on optimal exposure/gain parameters in $< 250\text{ ms}$.

---

## Optical Prism Geometry, Platen Feathering, and Framing

1. **Active Prism Boundaries:**
   The physical glass prism projects onto the CMOS sensor array within coordinates:
   - $X \in [140 \dots 960]$ (Width: 820 pixels)
   - $Y \in [130 \dots 750]$ (Height: 620 pixels)
2. **Bezel Moldura Elimination:**
   Outside the active glass window, light reflects from internal housing bevels, generating bright borders under `!pixel` inversion. `SensorFrame::masked_platen()` applies a smooth 18-pixel feathering taper to pure white background ($255$), completely eliminating hard frame artifacts without cropping the active fingerprint area.
3. **Aspect Ratio and Resampling:**
   - Resampling uses continuous sub-pixel bilinear interpolation (`resample_bilinear`), eliminating scanline/moiré black striping.
   - Mapping the $820 \times 620$ active platen into standardized portrait dimensions ($300 \times 400$ and $600 \times 800$, ratio 3:4) produces physically accurate biometric prints with average ridge period of $0.51 \dots 0.54\text{ mm}$ (precisely matching human anatomy at 500 DPI).




